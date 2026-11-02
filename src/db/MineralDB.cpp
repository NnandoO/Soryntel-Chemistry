#include "MineralDB.hpp"
#include "nucleo/tabla_periodica.hpp"
#include <algorithm>
#include <cctype>
#include <cmath>
#include <iomanip>
#include <sstream>

namespace cma::application::modules {

namespace {

std::string lower_str(const std::string& s) {
    std::string out = s;
    std::transform(out.begin(), out.end(), out.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return out;
}

std::string trim_str_copy(const std::string& s) {
    size_t b = 0;
    while (b < s.size() && std::isspace(static_cast<unsigned char>(s[b]))) ++b;
    size_t e = s.size();
    while (e > b && std::isspace(static_cast<unsigned char>(s[e - 1]))) --e;
    return s.substr(b, e - b);
}

// Resuelve un sistema lineal pequeño (n <= 32) mediante eliminación de Gauss-Jordan con pivoteo parcial
std::vector<double> solve_linear_system(std::vector<std::vector<double>> A, std::vector<double> b) {
    const size_t n = b.size();
    for (size_t i = 0; i < n; ++i) {
        size_t pivot = i;
        double max_v = std::abs(A[i][i]);
        for (size_t r = i + 1; r < n; ++r) {
            if (std::abs(A[r][i]) > max_v) {
                max_v = std::abs(A[r][i]);
                pivot = r;
            }
        }
        if (max_v < 1e-12) continue;
        if (pivot != i) {
            std::swap(A[i], A[pivot]);
            std::swap(b[i], b[pivot]);
        }
        const double diag = A[i][i];
        for (size_t c = i; c < n; ++c) A[i][c] /= diag;
        b[i] /= diag;
        for (size_t r = 0; r < n; ++r) {
            if (r == i) continue;
            const double factor = A[r][i];
            if (std::abs(factor) < 1e-15) continue;
            for (size_t c = i; c < n; ++c) A[r][c] -= factor * A[i][c];
            b[r] -= factor * b[i];
        }
    }
    return b;
}

} // namespace

MineralDB::MineralDB() {
    cargar_catalogo_literatura();
}

MineralDB::MineralDB(const cma::domain::ThermoRepository& repository)
    : motor_(&repository)
{
    cargar_catalogo_literatura();
}

std::vector<const cma::nucleo::Especie*> MineralDB::minerales() {
    std::vector<const cma::nucleo::Especie*> out;
    if (!motor_) return out;
    auto todas = motor_->todas_especies();
    for (auto* e : todas) {
        if (e && e->fase_std == cma::nucleo::Fase::Solido) {
            out.push_back(e);
        }
    }
    return out;
}

std::map<std::string, double> MineralDB::calcular_wt_elemental(const std::string& formula, double* out_mw) {
    std::map<std::string, double> wt;
    auto& tp = cma::nucleo::TablaPeriodica::instancia();
    auto atomos = tp.analizar_formula(formula);
    double mw = 0.0;
    std::map<std::string, double> masa_elem;
    for (const auto& at : atomos) {
        if (at.cuenta <= 0) continue;
        const auto* el = tp.por_simbolo(at.elem);
        const double aw = el ? el->masa_atomica : 0.0;
        if (aw > 0.0) {
            masa_elem[at.elem] += static_cast<double>(at.cuenta) * aw;
            mw += static_cast<double>(at.cuenta) * aw;
        }
    }
    if (out_mw) *out_mw = mw;
    if (mw > 1e-9) {
        for (const auto& [el, m] : masa_elem) {
            wt[el] = (m / mw) * 100.0;
        }
    }
    return wt;
}

void MineralDB::cargar_catalogo_literatura() {
    catalogo_.clear();

    auto add = [this](const char* nombre, const char* corto, const char* ima, const char* formula,
                      const char* clase, const char* sistema, const char* loc, const char* ref,
                      double sg, double mohs, const char* col) {
        FichaMineralGeo f;
        f.nombre = nombre;
        f.nombre_corto = corto;
        f.simbolo_ima = ima;
        f.formula = formula;
        f.clase_mineral = clase;
        f.sistema_cristalino = sistema;
        f.localidad = loc;
        f.referencia = ref;
        f.gravedad_especifica = sg;
        f.dureza_mohs = mohs;
        f.color = col;
        f.elem_wt_pct = calcular_wt_elemental(formula, &f.masa_molar);
        catalogo_.push_back(std::move(f));
    };

    // ── 1. Los 6 minerales principales de la imagen de referencia HSC Geo 8.0 ──
    add("Chalcocite (Arizona, USA)", "Chalcocite", "Cc", "Cu2S",
        "Sulfides", "Orthorhombic", "Morenci, Arizona, USA", "Anthony et al. (1990) Hdbk Mineralogy",
        5.65, 2.5, "Lead-grey");
    add("Chalcopyrite (Erora, Portugal)", "Chalcopyrite", "Ccp", "CuFeS2",
        "Sulfides", "Tetragonal", "Erora, Alentejo, Portugal", "Cabri (1973) Econ. Geol. 68",
        4.19, 3.5, "Brass-yellow");
    add("Pyrite (The Wilhelmine mine, Spessart,Bavaria)", "Pyrite", "Py", "FeS2",
        "Sulfides", "Isometric", "Wilhelmine Mine, Spessart, Bavaria", "Deer, Howie & Zussman (1992)",
        5.01, 6.5, "Pale brass-yellow");
    add("Galena (Kansan, USA)", "Galena", "Gn", "PbS",
        "Sulfides", "Isometric", "Tri-State District, Kansas, USA", "Klein & Hurlbut (1999)",
        7.58, 2.5, "Silver-lead grey");
    add("Quartz (Kondyor PGE Placer Deposit, Russia)", "Quartz", "Qtz", "SiO2",
        "Silicates", "Trigonal", "Kondyor Massif, Khabarovsk, Russia", "Frondel (1962) Dana's System",
        2.65, 7.0, "Colorless / White");
    add("Olivine (Western Bushveld complex, South Africa)", "Olivine", "Ol", "Mg2SiO4",
        "Silicates", "Orthorhombic", "Western Bushveld Complex, South Africa", "Cawthorn (2015) Bushveld Rev.",
        3.32, 6.5, "Olive-green");

    // ── 2. Sulfuros, Sulfosales y Arseniuros adicionales de Literatura ──
    add("Pentlandite (Bruvann, Rana, Norway)", "Pentlandite", "Pn", "Fe5Ni4S8",
        "Sulfides", "Isometric", "Bruvann Ni-Cu-Co Deposit, Norway", "Boyd & Mathiesen (1979) NGU",
        4.80, 3.5, "Light bronze-yellow");
    add("Pyrrhotite (Sudbury, Ontario, Canada)", "Pyrrhotite", "Po", "Fe7S8",
        "Sulfides", "Monoclinic", "Sudbury Igneous Complex, Canada", "Arnold (1967) Econ. Geol.",
        4.61, 4.0, "Bronze-brown");
    add("Sphalerite (Santander, Spain)", "Sphalerite", "Sp", "ZnS",
        "Sulfides", "Isometric", "Picos de Europa, Santander, Spain", "Barton & Toulmin (1966)",
        4.05, 3.5, "Amber-brown");
    add("Bornite (Butte, Montana, USA)", "Bornite", "Bn", "Cu5FeS4",
        "Sulfides", "Orthorhombic", "Butte District, Montana, USA", "Morimoto & Kullerud (1961)",
        5.08, 3.0, "Copper-red / Tarnished purple");
    add("Covellite (Bor, Serbia)", "Covellite", "Cv", "CuS",
        "Sulfides", "Hexagonal", "Bor Porphyry Deposit, Serbia", "Evans & Konnert (1976)",
        4.68, 1.5, "Indigo-blue");
    add("Molybdenite (Climax, Colorado, USA)", "Molybdenite", "Mol", "MoS2",
        "Sulfides", "Hexagonal", "Climax Mine, Colorado, USA", "Dickinson & Pauling (1923)",
        4.73, 1.5, "Bluish lead-grey");
    add("Arsenopyrite (Boliden, Sweden)", "Arsenopyrite", "Apy", "FeAsS",
        "Sulfides", "Monoclinic", "Boliden Au-Cu-As Deposit, Sweden", "Kretschmar & Scott (1976)",
        6.07, 5.5, "Silver-white");
    add("Cobaltite (Tunaberg, Sweden)", "Cobaltite", "Cbt", "CoAsS",
        "Sulfides", "Orthorhombic", "Tunaberg, Sodermanland, Sweden", "Fleet & Burns (1990)",
        6.33, 5.5, "Silver-white pinkish");
    add("Enargite (Chuquicamata, Chile)", "Enargite", "Eng", "Cu3AsS4",
        "Sulfides", "Orthorhombic", "Chuquicamata, Antofagasta, Chile", "Maske & Skinner (1971)",
        4.45, 3.0, "Greyish-black");
    add("Stibnite (Xikuangshan, Hunan, China)", "Stibnite", "Stb", "Sb2S3",
        "Sulfides", "Orthorhombic", "Xikuangshan, Hunan, China", "Kyono & Kimata (2004)",
        4.63, 2.0, "Lead-grey metallic");

    // ── 3. Silicatos Formadores de Roca (Ígneos y Metamórficos) ──
    add("Fayalite (Skaergaard Intrusion, Greenland)", "Fayalite", "Fa", "Fe2SiO4",
        "Silicates", "Orthorhombic", "Skaergaard, East Greenland", "Wager & Brown (1968)",
        4.39, 6.5, "Greenish-yellow to brown");
    add("Enstatite (Great Dyke, Zimbabwe)", "Enstatite", "En", "MgSiO3",
        "Silicates", "Orthorhombic", "Darwendale, Great Dyke, Zimbabwe", "Wilson (1982) J. Petrol.",
        3.21, 5.5, "Greyish-green");
    add("Diopside (Outokumpu, Finland)", "Diopside", "Di", "CaMgSi2O6",
        "Silicates", "Monoclinic", "Outokumpu Belt, Eastern Finland", "Treloar (1987) J. Metamorph.",
        3.28, 6.0, "Light green");
    add("Anorthite (Stillwater Complex, Montana, USA)", "Anorthite", "An", "CaAl2Si2O8",
        "Silicates", "Triclinic", "Stillwater Complex, Montana, USA", "McCallum (1996)",
        2.76, 6.0, "White / Grey");
    add("Albite (Amelia Courthouse, Virginia, USA)", "Albite", "Ab", "NaAlSi3O8",
        "Silicates", "Triclinic", "Amelia County, Virginia, USA", "Smith & Brown (1988) Feldspars",
        2.62, 6.0, "White");
    add("Orthoclase (St. Gotthard, Switzerland)", "Orthoclase", "Or", "KAlSi3O8",
        "Silicates", "Monoclinic", "St. Gotthard Massif, Switzerland", "Deer et al. (2001) Framework Silicates",
        2.56, 6.0, "Flesh-pink / White");
    add("Biotite (Bancroft, Ontario, Canada)", "Biotite", "Bt", "KMg3AlSi3O10F2",
        "Silicates", "Monoclinic", "Bancroft District, Ontario, Canada", "Rieder et al. (1998) IMA Mica",
        3.09, 2.5, "Dark brown / Black");
    add("Muscovite (Minas Gerais, Brazil)", "Muscovite", "Ms", "KAl3Si3O12H2",
        "Silicates", "Monoclinic", "Governador Valadares, Brazil", "Bailey (1984) Micas",
        2.82, 2.5, "Silvery-white");
    add("Talc (Trimouns, Pyrenees, France)", "Talc", "Tlc", "Mg3Si4O12H2",
        "Silicates", "Monoclinic", "Luzenac, Ariège, France", "Evans & Guggenheim (1988)",
        2.75, 1.0, "Apple-green / White");
    add("Zircon (Mud Tank, Northern Territory, Australia)", "Zircon", "Zrn", "ZrSiO4",
        "Silicates", "Tetragonal", "Mud Tank Carbonatite, Australia", "Black & Gulson (1978)",
        4.65, 7.5, "Reddish-brown");

    // ── 4. Óxidos, Carbonatos, Fosfatos y Sulfatos ──
    add("Magnetite (Kiruna, Norrbotten, Sweden)", "Magnetite", "Mag", "Fe3O4",
        "Oxides", "Isometric", "Kiirunavaara, Lapland, Sweden", "Frietsch (1978) Econ. Geol.",
        5.18, 6.0, "Iron-black");
    add("Hematite (Carajas, Para, Brazil)", "Hematite", "Hem", "Fe2O3",
        "Oxides", "Trigonal", "Serra Norte, Carajás, Brazil", "Klein (2005) Econ. Geol.",
        5.26, 6.0, "Steel-grey to red");
    add("Ilmenite (Tellnes, Rogaland, Norway)", "Ilmenite", "Ilm", "FeTiO3",
        "Oxides", "Trigonal", "Tellnes Ti-Fe Deposit, Norway", "Duchesne (1999)",
        4.72, 5.5, "Iron-black");
    add("Chromite (Bushveld UG2 Reef, South Africa)", "Chromite", "Chr", "FeCr2O4",
        "Oxides", "Isometric", "UG2 Chromitite Layer, South Africa", "Mondal & Mathez (2007) J. Petrol.",
        4.79, 5.5, "Brownish-black");
    add("Cassiterite (Cornwall, England, UK)", "Cassiterite", "Cst", "SnO2",
        "Oxides", "Tetragonal", "Camborne-Redruth, Cornwall, UK", "Haapala (1997)",
        6.99, 6.5, "Dark brown / Adamantine");
    add("Calcite (Sweetwater Mine, Missouri, USA)", "Calcite", "Cal", "CaCO3",
        "Carbonates", "Trigonal", "Viburnum Trend, Missouri, USA", "Reeder (1983) Carbonates",
        2.71, 3.0, "White / Honey-yellow");
    add("Dolomite (Binn Valley, Valais, Switzerland)", "Dolomite", "Dol", "CaMgC2O6",
        "Carbonates", "Trigonal", "Lengenbach Quarry, Valais, Switzerland", "Gregg et al. (2015)",
        2.85, 3.5, "Pearly white");
    add("Fluorapatite (Khibiny Massif, Kola, Russia)", "Fluorapatite", "Ap", "Ca5P3O12F",
        "Phosphates", "Hexagonal", "Kukisvumchorr, Kola Peninsula, Russia", "Hughes & Rakovan (2002)",
        3.18, 5.0, "Sea-green");
    add("Barite (Silvermine, Cumbria, England)", "Barite", "Brt", "BaSO4",
        "Sulfates", "Orthorhombic", "Northern Pennines, Cumbria, UK", "Hanor (2000)",
        4.48, 3.0, "White / Pale blue");
    add("Uraninite (Athabasca Basin, Canada)", "Uraninite", "Urn", "UO2",
        "Oxides", "Isometric", "Cigar Lake, Saskatchewan, Canada", "Janeczek & Ewing (1992)",
        10.60, 5.5, "Pitch-black");

    // ── Muestras predefinidas para Cálculos Modales (Bruvann.xls - XRF y más) ──
    muestras_.clear();
    {
        // Muestra 1: Bruvann.xls - XRF (mezcla exacta de Calcopirita 25%, Pirita 20%, Galena 5%, Cuarzo 30%, Olivino 20%)
        MuestraGeoModal m1;
        m1.nombre_fuente = "Bruvann.xls - XRF";
        m1.descripcion = "Bruvann Polymetallic Sulfide-Silicate Sample (Rana Intrusion, Norway)";
        m1.minerales_candidatos = {
            "Chalcocite (Arizona, USA)",
            "Chalcopyrite (Erora, Portugal)",
            "Pyrite (The Wilhelmine mine, Spessart,Bavaria)",
            "Galena (Kansan, USA)",
            "Quartz (Kondyor PGE Placer Deposit, Russia)",
            "Olivine (Western Bushveld complex, South Africa)"
        };
        // Composición sintética consistente con 25% Ccp, 20% Py, 5% Gn, 30% Qtz, 20% Ol
        m1.ensayo_elem_wt = {
            {"Cu", 8.657},
            {"Fe", 16.917},
            {"S",  20.095},
            {"Pb", 4.330},
            {"Si", 18.019},
            {"Mg", 6.912},
            {"O",  25.070}
        };
        muestras_.push_back(m1);
    }
    {
        // Muestra 2: Bushveld Merensky Reef Cu-Ni-PGE Concentrate
        MuestraGeoModal m2;
        m2.nombre_fuente = "Bushveld_Merensky.xls - XRF";
        m2.descripcion = "Western Bushveld Complex Cu-Ni Sulfide-Silicate Reef";
        m2.minerales_candidatos = {
            "Chalcopyrite (Erora, Portugal)",
            "Pentlandite (Bruvann, Rana, Norway)",
            "Pyrrhotite (Sudbury, Ontario, Canada)",
            "Enstatite (Great Dyke, Zimbabwe)",
            "Anorthite (Stillwater Complex, Montana, USA)",
            "Chromite (Bushveld UG2 Reef, South Africa)"
        };
        m2.ensayo_elem_wt = {
            {"Cu", 6.23},
            {"Ni", 5.48},
            {"Fe", 24.85},
            {"S",  18.90},
            {"Mg", 6.10},
            {"Ca", 3.60},
            {"Al", 4.85},
            {"Cr", 2.32},
            {"Si", 9.80},
            {"O",  17.87}
        };
        muestras_.push_back(m2);
    }
    {
        // Muestra 3: Kiruna Magnetite-Apatite Iron Ore
        MuestraGeoModal m3;
        m3.nombre_fuente = "Kiruna_LKAB.xls - XRF";
        m3.descripcion = "Kiirunavaara High-Phosphorus Magnetite-Apatite Ore";
        m3.minerales_candidatos = {
            "Magnetite (Kiruna, Norrbotten, Sweden)",
            "Hematite (Carajas, Para, Brazil)",
            "Fluorapatite (Khibiny Massif, Kola, Russia)",
            "Quartz (Kondyor PGE Placer Deposit, Russia)",
            "Calcite (Sweetwater Mine, Missouri, USA)"
        };
        m3.ensayo_elem_wt = {
            {"Fe", 61.50},
            {"P",  1.45},
            {"Ca", 4.20},
            {"Si", 2.80},
            {"C",  0.30},
            {"F",  0.28},
            {"O",  29.47}
        };
        muestras_.push_back(m3);
    }
}

const FichaMineralGeo* MineralDB::buscar_por_nombre(const std::string& nombre) const {
    const std::string nl = lower_str(trim_str_copy(nombre));
    for (const auto& m : catalogo_) {
        if (lower_str(m.nombre) == nl || lower_str(m.nombre_corto) == nl || lower_str(m.simbolo_ima) == nl) {
            return &m;
        }
    }
    return nullptr;
}

bool MineralDB::cumple_consulta_hsc(const FichaMineralGeo& m, const std::string& query) const {
    const std::string q = trim_str_copy(query);
    if (q.empty()) return true;
    const std::string ql = lower_str(q);

    // Evaluar cláusulas tipo "[Cu %] Is greater than 10" y "[Specific Gravity] Is between 2.2 and 4.4"
    // Si es una consulta simple de texto sin corchetes, buscar subcadena en nombre/fórmula/localidad/símbolo
    if (q.find('[') == std::string::npos) {
        return lower_str(m.nombre).find(ql) != std::string::npos ||
               lower_str(m.formula).find(ql) != std::string::npos ||
               lower_str(m.simbolo_ima).find(ql) != std::string::npos ||
               lower_str(m.localidad).find(ql) != std::string::npos ||
               lower_str(m.clase_mineral).find(ql) != std::string::npos;
    }

    // Parsear bloques [Campo] separados por AND
    size_t pos = 0;
    while (pos < q.size()) {
        size_t b1 = q.find('[', pos);
        if (b1 == std::string::npos) break;
        size_t b2 = q.find(']', b1 + 1);
        if (b2 == std::string::npos) break;

        std::string campo = trim_str_copy(q.substr(b1 + 1, b2 - b1 - 1));
        size_t next_b = q.find('[', b2 + 1);
        std::string expr = (next_b == std::string::npos) ? q.substr(b2 + 1) : q.substr(b2 + 1, next_b - b2 - 1);
        std::string expr_l = lower_str(expr);

        double valor_campo = 0.0;
        std::string campo_l = lower_str(campo);
        if (campo_l.find("specific gravity") != std::string::npos || campo_l == "sg" || campo_l == "density") {
            valor_campo = m.gravedad_especifica;
        } else if (campo_l.find("hardness") != std::string::npos || campo_l == "mohs") {
            valor_campo = m.dureza_mohs;
        } else if (campo_l.find("mw") != std::string::npos || campo_l.find("molar") != std::string::npos) {
            valor_campo = m.masa_molar;
        } else {
            // Símbolo elemental ej. "Cu %" o "Fe"
            std::string elem;
            for (char c : campo) {
                if (std::isalpha(static_cast<unsigned char>(c))) elem.push_back(c);
            }
            valor_campo = m.porcentaje_elemento(elem);
        }

        // Extraer números en expr
        std::vector<double> nums;
        std::stringstream ss(expr);
        std::string tok;
        while (ss >> tok) {
            try {
                size_t idx = 0;
                double v = std::stod(tok, &idx);
                if (idx > 0) nums.push_back(v);
            } catch (...) {}
        }

        if (expr_l.find("between") != std::string::npos && nums.size() >= 2) {
            double lo = std::min(nums[0], nums[1]);
            double hi = std::max(nums[0], nums[1]);
            if (valor_campo < lo || valor_campo > hi) return false;
        } else if ((expr_l.find("greater") != std::string::npos || expr_l.find(">") != std::string::npos) && !nums.empty()) {
            if (!(valor_campo > nums[0])) return false;
        } else if ((expr_l.find("less") != std::string::npos || expr_l.find("<") != std::string::npos) && !nums.empty()) {
            if (!(valor_campo < nums[0])) return false;
        } else if (!nums.empty()) {
            if (std::abs(valor_campo - nums[0]) > 1.0) return false;
        }

        pos = (next_b == std::string::npos) ? q.size() : next_b;
    }

    return true;
}

std::vector<const FichaMineralGeo*> MineralDB::filtrar_minerales(
    const std::string& texto_libre,
    const std::string& clase_filtro,
    const std::vector<std::string>& elem_requeridos,
    const std::vector<std::string>& elem_excluidos,
    double sg_min, double sg_max,
    double mohs_min, double mohs_max) const
{
    std::vector<const FichaMineralGeo*> out;
    for (const auto& m : catalogo_) {
        if (!clase_filtro.empty() && clase_filtro != "All" && m.clase_mineral != clase_filtro) continue;
        if (m.gravedad_especifica < sg_min || m.gravedad_especifica > sg_max) continue;
        if (m.dureza_mohs < mohs_min || m.dureza_mohs > mohs_max) continue;

        bool req_ok = true;
        for (const auto& el : elem_requeridos) {
            if (m.porcentaje_elemento(el) <= 1e-6) {
                req_ok = false;
                break;
            }
        }
        if (!req_ok) continue;

        bool excl_ok = true;
        for (const auto& el : elem_excluidos) {
            if (m.porcentaje_elemento(el) > 1e-6) {
                excl_ok = false;
                break;
            }
        }
        if (!excl_ok) continue;

        if (!cumple_consulta_hsc(m, texto_libre)) continue;
        out.push_back(&m);
    }
    return out;
}

ResultadoCalculoModal MineralDB::resolver_modal_rondas(
    const std::map<std::string, double>& ensayo_elem_wt,
    const std::vector<RondaModal>& rondas,
    MetodoModal metodo) const
{
    ResultadoCalculoModal res;
    std::map<std::string, double> rem = ensayo_elem_wt;

    for (size_t r_idx = 0; r_idx < rondas.size(); ++r_idx) {
        const auto& ronda = rondas[r_idx];
        if (ronda.fases.empty() || ronda.elementos.empty()) continue;

        std::vector<const FichaMineralGeo*> fichas;
        for (const auto& fname : ronda.fases) {
            if (const auto* f = buscar_por_nombre(fname)) {
                fichas.push_back(f);
            }
        }
        if (fichas.empty()) continue;

        const size_t m_rows = ronda.elementos.size();
        const size_t n_cols = fichas.size();
        res.rondas_ejecutadas++;

        // Construir matriz estequiométrica A (m_rows x n_cols) donde A[i][j] = fracción másica del elemento i en el mineral j
        std::vector<std::vector<double>> A(m_rows, std::vector<double>(n_cols, 0.0));
        std::vector<double> b(m_rows, 0.0);
        std::vector<double> w(m_rows, 1.0);

        for (size_t i = 0; i < m_rows; ++i) {
            const std::string& el = ronda.elementos[i];
            auto it = rem.find(el);
            b[i] = (it != rem.end()) ? std::max(0.0, it->second) : 0.0;
            for (size_t j = 0; j < n_cols; ++j) {
                A[i][j] = fichas[j]->porcentaje_elemento(el) / 100.0;
            }
            if (metodo == MetodoModal::WLS || metodo == MetodoModal::WNNLS) {
                const double sigma = std::max(0.04 * b[i], 0.05);
                w[i] = 1.0 / (sigma * sigma);
            }
        }

        // Registrar ecuaciones simbólicas de la ronda para el botón "Equations"
        for (size_t i = 0; i < m_rows; ++i) {
            std::ostringstream eq;
            eq << "[" << ronda.nombre << "] " << ronda.elementos[i] << " (" << std::fixed << std::setprecision(2) << b[i] << "%): ";
            bool primero = true;
            for (size_t j = 0; j < n_cols; ++j) {
                if (A[i][j] > 1e-6) {
                    if (!primero) eq << " + ";
                    eq << std::fixed << std::setprecision(4) << A[i][j] << " * [" << fichas[j]->simbolo_ima << "]";
                    primero = false;
                }
            }
            eq << " = " << std::fixed << std::setprecision(3) << b[i];
            res.ecuaciones_rondas.push_back(eq.str());
        }

        // Armar ecuaciones normales (A^T W A + reg*I) x = A^T W b
        std::vector<std::vector<double>> AtWA(n_cols, std::vector<double>(n_cols, 0.0));
        std::vector<double> AtWb(n_cols, 0.0);
        for (size_t j = 0; j < n_cols; ++j) {
            for (size_t k = 0; k < n_cols; ++k) {
                double s = 0.0;
                for (size_t i = 0; i < m_rows; ++i) s += A[i][j] * w[i] * A[i][k];
                AtWA[j][k] = s;
            }
            AtWA[j][j] += 1e-9; // Regularización de Tikhonov suave
            double sb = 0.0;
            for (size_t i = 0; i < m_rows; ++i) sb += A[i][j] * w[i] * b[i];
            AtWb[j] = sb;
        }

        std::vector<double> x = solve_linear_system(AtWA, AtWb);

        // Si el método es NNLS o WNNLS, aplicar iteraciones de Gauss-Seidel proyectado sobre el cono x >= 0
        if (metodo == MetodoModal::NNLS || metodo == MetodoModal::WNNLS) {
            for (double& xj : x) xj = std::max(0.0, xj);
            for (int iter = 0; iter < 160; ++iter) {
                double max_diff = 0.0;
                for (size_t j = 0; j < n_cols; ++j) {
                    if (AtWA[j][j] <= 1e-12) continue;
                    double r_j = AtWb[j];
                    for (size_t k = 0; k < n_cols; ++k) {
                        if (k != j) r_j -= AtWA[j][k] * x[k];
                    }
                    double x_new = std::max(0.0, r_j / AtWA[j][j]);
                    max_diff = std::max(max_diff, std::abs(x_new - x[j]));
                    x[j] = x_new;
                }
                if (max_diff < 1e-9) break;
            }
        }

        // Guardar resultados de la ronda y descontar masa elemental consumida para la siguiente ronda
        for (size_t j = 0; j < n_cols; ++j) {
            ResultadoFaseModal rf;
            rf.nombre_mineral = fichas[j]->nombre;
            rf.formula = fichas[j]->formula;
            rf.ronda_resuelta = static_cast<int>(r_idx) + 1;
            rf.wt_pct = x[j];
            rf.sigma_error = std::max(0.05, 0.025 * std::abs(x[j]));
            if (std::abs(x[j]) < 1e-6) res.fases_no_determinadas++;
            res.fases.push_back(rf);
            res.total_mineral_wt += x[j];

            // Descontar todos los elementos que contiene este mineral del ensayo remanente
            const double x_pos = std::max(0.0, x[j]);
            for (const auto& [el_m, wt_m] : fichas[j]->elem_wt_pct) {
                if (rem.count(el_m)) {
                    rem[el_m] = std::max(0.0, rem[el_m] - x_pos * (wt_m / 100.0));
                }
            }
        }
    }

    // Calcular porcentajes en volumen (vol-%) a partir de la gravedad específica de cada mineral
    double vol_tot = 0.0;
    for (const auto& rf : res.fases) {
        const auto* f = buscar_por_nombre(rf.nombre_mineral);
        const double sg = (f && f->gravedad_especifica > 0.1) ? f->gravedad_especifica : 2.7;
        if (rf.wt_pct > 0.0) vol_tot += rf.wt_pct / sg;
    }
    if (vol_tot > 1e-9) {
        for (auto& rf : res.fases) {
            const auto* f = buscar_por_nombre(rf.nombre_mineral);
            const double sg = (f && f->gravedad_especifica > 0.1) ? f->gravedad_especifica : 2.7;
            rf.vol_pct = (rf.wt_pct > 0.0) ? ((rf.wt_pct / sg) / vol_tot * 100.0) : 0.0;
        }
    }

    // Comparación elemental completa (medido vs. retro-calculado desde los minerales resueltos)
    double sum_sq_err = 0.0;
    double chi_sq = 0.0;
    int n_el = 0;
    for (const auto& [el, medido] : ensayo_elem_wt) {
        double calc = 0.0;
        for (const auto& rf : res.fases) {
            if (const auto* f = buscar_por_nombre(rf.nombre_mineral)) {
                calc += rf.wt_pct * (f->porcentaje_elemento(el) / 100.0);
            }
        }
        ComparacionElementoModal cmp;
        cmp.elemento = el;
        cmp.medido_wt = medido;
        cmp.calculado_wt = calc;
        cmp.residual_wt = medido - calc;
        cmp.error_rel_pct = (std::abs(medido) > 1e-6) ? ((calc - medido) / medido * 100.0) : 0.0;
        res.balance_elemental.push_back(cmp);

        sum_sq_err += cmp.residual_wt * cmp.residual_wt;
        const double sigma = std::max(0.04 * medido, 0.05);
        chi_sq += (cmp.residual_wt * cmp.residual_wt) / (sigma * sigma);
        n_el++;
    }
    res.rmse_elemental = (n_el > 0) ? std::sqrt(sum_sq_err / n_el) : 0.0;
    res.chi_cuadrado = chi_sq;
    res.no_identificado_wt = std::max(0.0, 100.0 - res.total_mineral_wt);
    return res;
}

ResultadoFormulaMineral MineralDB::calcular_formula_mineral(
    const std::vector<EntradaAnalisisMineral>& analisis,
    const std::string& elemento_base_normalizacion,
    double num_atomos_base) const
{
    ResultadoFormulaMineral out;
    auto& tp = cma::nucleo::TablaPeriodica::instancia();

    std::map<std::string, double> moles_elem;
    std::map<std::string, double> gramos_elem;
    double total_gramos = 0.0;

    for (const auto& item : analisis) {
        if (item.wt_pct <= 0.0 || item.componente.empty()) continue;
        out.suma_analisis_wt += item.wt_pct;

        auto atomos = tp.analizar_formula(item.componente);
        double mw_comp = 0.0;
        for (const auto& at : atomos) {
            const auto* el = tp.por_simbolo(at.elem);
            if (el) mw_comp += static_cast<double>(at.cuenta) * el->masa_atomica;
        }
        if (mw_comp <= 1e-9) continue;

        const double moles_comp = item.wt_pct / mw_comp;
        for (const auto& at : atomos) {
            const auto* el = tp.por_simbolo(at.elem);
            if (!el) continue;
            const double mol_e = moles_comp * static_cast<double>(at.cuenta);
            moles_elem[at.elem] += mol_e;
            gramos_elem[at.elem] += mol_e * el->masa_atomica;
            total_gramos += mol_e * el->masa_atomica;
        }
    }

    if (total_gramos > 1e-9) {
        for (const auto& [el, g] : gramos_elem) {
            out.elem_wt_normalizado[el] = (g / total_gramos) * 100.0;
        }
    }

    double moles_base = 0.0;
    if (!elemento_base_normalizacion.empty() && moles_elem.count(elemento_base_normalizacion)) {
        moles_base = moles_elem[elemento_base_normalizacion];
    } else {
        for (const auto& [el, m] : moles_elem) moles_base += m;
    }

    const double factor = (moles_base > 1e-9 && num_atomos_base > 0.0) ? (num_atomos_base / moles_base) : 1.0;
    std::ostringstream f_str;
    // Ordenar cationes primero y aniones (S, As, O, F, Cl) al final
    auto es_anion = [](const std::string& e) {
        return e == "O" || e == "S" || e == "F" || e == "Cl" || e == "As" || e == "Se" || e == "Te";
    };

    for (int pass = 0; pass < 2; ++pass) {
        for (const auto& [el, mol] : moles_elem) {
            if ((pass == 0 && es_anion(el)) || (pass == 1 && !es_anion(el))) continue;
            const double apfu = mol * factor;
            if (apfu < 0.01) continue;
            out.apfu[el] = apfu;
            f_str << el;
            if (std::abs(apfu - std::round(apfu)) < 0.04) {
                int n_int = static_cast<int>(std::round(apfu));
                if (n_int > 1) f_str << n_int;
            } else {
                f_str << std::fixed << std::setprecision(2) << apfu;
            }
        }
    }
    out.formula_empirica = f_str.str();

    // Buscar el mineral más cercano en el catálogo por distancia euclidiana en wt-% elemental
    double min_dist = 1e9;
    for (const auto& m : catalogo_) {
        double d2 = 0.0;
        for (const auto& [el, wt] : out.elem_wt_normalizado) {
            double diff = wt - m.porcentaje_elemento(el);
            d2 += diff * diff;
        }
        for (const auto& [el, wt_m] : m.elem_wt_pct) {
            if (!out.elem_wt_normalizado.count(el)) {
                d2 += wt_m * wt_m;
            }
        }
        double dist = std::sqrt(d2);
        if (dist < min_dist) {
            min_dist = dist;
            out.mineral_mas_cercano = m.nombre;
            out.formula_ideal = m.formula;
            out.similitud_pct = std::max(0.0, 100.0 - dist);
        }
    }

    return out;
}

ResultadoPetrologiaCIPW MineralDB::calcular_norma_cipw(const AnalisisRocaOxidos& ox) const {
    ResultadoPetrologiaCIPW res;

    // Normalizar óxidos a 100% libre de volátiles
    const double sum_ox = ox.SiO2 + ox.TiO2 + ox.Al2O3 + ox.Fe2O3 + ox.FeO +
                          ox.MnO + ox.MgO + ox.CaO + ox.Na2O + ox.K2O + ox.P2O5;
    const double f = (sum_ox > 1e-6) ? (100.0 / sum_ox) : 1.0;

    const double sio2  = ox.SiO2 * f;
    const double tio2  = ox.TiO2 * f;
    const double al2o3 = ox.Al2O3 * f;
    const double fe2o3 = ox.Fe2O3 * f;
    const double feo   = (ox.FeO + ox.MnO) * f;
    const double mgo   = ox.MgO * f;
    const double cao   = ox.CaO * f;
    const double na2o  = ox.Na2O * f;
    const double k2o   = ox.K2O * f;
    const double p2o5  = ox.P2O5 * f;

    // Proporciones molares de óxidos
    double m_SiO2  = sio2  / 60.084;
    double m_TiO2  = tio2  / 79.866;
    double m_Al2O3 = al2o3 / 101.961;
    double m_Fe2O3 = fe2o3 / 159.688;
    double m_FeO   = feo   / 71.844;
    double m_MgO   = mgo   / 40.304;
    double m_CaO   = cao   / 56.077;
    double m_Na2O  = na2o  / 61.979;
    double m_K2O   = k2o   / 94.196;
    double m_P2O5  = p2o5  / 141.945;

    // Índices petrológicos geoquímicos
    res.mg_number = (m_MgO + m_FeO > 1e-9) ? (100.0 * m_MgO / (m_MgO + m_FeO)) : 0.0;
    res.indice_acnk = (m_CaO + m_Na2O + m_K2O > 1e-9) ? (m_Al2O3 / (m_CaO + m_Na2O + m_K2O)) : 0.0;
    if (res.indice_acnk > 1.05) res.saturacion_alumina = "Peraluminous (A/CNK > 1.0)";
    else if (m_Al2O3 < m_Na2O + m_K2O) res.saturacion_alumina = "Peralkaline (A/NK < 1.0)";
    else res.saturacion_alumina = "Metaluminous (A/CNK <= 1.0)";

    // Secuencia estándar CIPW Norm (Cross, Iddings, Pirsson & Washington, 1902)
    // 1. Apatite: 3.33 CaO · P2O5
    double m_Ap = m_P2O5;
    m_CaO = std::max(0.0, m_CaO - 3.333 * m_Ap);

    // 2. Ilmenite: FeO · TiO2
    double m_Ilm = std::min(m_FeO, m_TiO2);
    m_FeO -= m_Ilm;

    // 3. Magnetite: FeO · Fe2O3
    double m_Mag = std::min(m_FeO, m_Fe2O3);
    m_FeO -= m_Mag;

    // 4. Orthoclase: K2O · Al2O3 · 6 SiO2
    double m_Or = std::min(m_K2O, m_Al2O3);
    m_Al2O3 -= m_Or;
    double si_usado = 6.0 * m_Or;

    // 5. Albite: Na2O · Al2O3 · 6 SiO2
    double m_Ab = std::min(m_Na2O, m_Al2O3);
    m_Al2O3 -= m_Ab;
    si_usado += 6.0 * m_Ab;

    // 6. Anorthite: CaO · Al2O3 · 2 SiO2 vs Corundum (Al2O3 residual)
    double m_An = std::min(m_CaO, m_Al2O3);
    m_CaO -= m_An;
    m_Al2O3 -= m_An;
    double m_Cor = m_Al2O3;
    si_usado += 2.0 * m_An;

    // 7. Diopside: CaO · (Mg,Fe)O · 2 SiO2
    double m_FM = m_MgO + m_FeO;
    double m_Di = std::min(m_CaO, m_FM);
    m_FM -= m_Di;
    si_usado += 2.0 * m_Di;

    // 8. Hypersthene ((Mg,Fe)SiO3) vs Olivine ((Mg,Fe)2SiO4) + Quartz (SiO2)
    double si_disp = m_SiO2 - si_usado;
    double m_Hy = 0.0;
    double m_Ol = 0.0;
    double m_Q  = 0.0;

    if (si_disp >= m_FM) {
        m_Hy = m_FM;
        m_Q  = si_disp - m_FM;
    } else if (si_disp >= 0.5 * m_FM) {
        m_Ol = m_FM - si_disp;
        m_Hy = m_FM - 2.0 * m_Ol;
    } else {
        m_Ol = 0.5 * m_FM;
    }

    // Conversión de moles normativos a wt-% mineralógico
    res.norma_wt["Quartz (SiO2)"]        = m_Q   * 60.084;
    res.norma_wt["Orthoclase (KAlSi3O8)"]= m_Or  * 2.0 * 278.33;
    res.norma_wt["Albite (NaAlSi3O8)"]   = m_Ab  * 2.0 * 262.22;
    res.norma_wt["Anorthite (CaAl2Si2O8)"] = m_An * 278.21;
    res.norma_wt["Corundum (Al2O3)"]     = m_Cor * 101.96;
    res.norma_wt["Diopside (CaMgSi2O6)"] = m_Di  * 216.55;
    res.norma_wt["Hypersthene (Mg,Fe)SiO3"] = m_Hy * 108.0;
    res.norma_wt["Olivine (Mg,Fe)2SiO4"] = m_Ol  * 0.5 * 155.0;
    res.norma_wt["Magnetite (Fe3O4)"]    = m_Mag * 231.53;
    res.norma_wt["Ilmenite (FeTiO3)"]    = m_Ilm * 151.71;
    res.norma_wt["Apatite (Ca5(PO4)3F)"] = m_Ap  * 336.0;

    // Normalizar suma normativa a 100%
    double sum_norm = 0.0;
    for (const auto& [k, v] : res.norma_wt) sum_norm += v;
    if (sum_norm > 1e-6) {
        for (auto& [k, v] : res.norma_wt) v = (v / sum_norm) * 100.0;
    }

    res.indice_diferenciacion = res.norma_wt["Quartz (SiO2)"] +
                                res.norma_wt["Orthoclase (KAlSi3O8)"] +
                                res.norma_wt["Albite (NaAlSi3O8)"];

    // Clasificación TAS (Total Alkali-Silica, Le Bas et al. 1986)
    const double alk = na2o + k2o;
    if (sio2 < 45.0) {
        res.clasificacion_tas = (alk < 3.0) ? "Picrobasalt / Peridotite (Ultramafic)" : "Tephrite / Basanite";
    } else if (sio2 < 52.0) {
        res.clasificacion_tas = (alk < 5.0) ? "Basalt / Gabbro-Norite (Mafic)" : "Trachybasalt / Monzogabbro";
    } else if (sio2 < 57.0) {
        res.clasificacion_tas = (alk < 5.7) ? "Basaltic Andesite / Gabbrodiorite" : "Basaltic Trachyandesite";
    } else if (sio2 < 63.0) {
        res.clasificacion_tas = (alk < 7.0) ? "Andesite / Diorite (Intermediate)" : "Trachyandesite / Monzonite";
    } else if (sio2 < 69.0) {
        res.clasificacion_tas = (alk < 8.0) ? "Dacite / Granodiorite" : "Trachyte / Quartz Syenite";
    } else {
        res.clasificacion_tas = "Rhyolite / Granite (Felsic)";
    }

    // Clasificación QAPF (Streckeisen 1976)
    const double Q = res.norma_wt["Quartz (SiO2)"];
    const double A = res.norma_wt["Orthoclase (KAlSi3O8)"];
    const double P = res.norma_wt["Albite (NaAlSi3O8)"] + res.norma_wt["Anorthite (CaAl2Si2O8)"];
    const double qap = Q + A + P;
    if (qap > 1e-6) {
        const double q_norm = (Q / qap) * 100.0;
        const double p_ratio = (A + P > 1e-6) ? (P / (A + P) * 100.0) : 100.0;
        if (q_norm > 20.0) {
            if (p_ratio > 65.0) res.clasificacion_qapf = "Granodiorite / Tonalite";
            else res.clasificacion_qapf = "Monzogranite / Syenogranite";
        } else if (q_norm > 5.0) {
            res.clasificacion_qapf = (p_ratio > 65.0) ? "Quartz Gabbro-Norite / Quartz Diorite" : "Quartz Monzonite";
        } else {
            res.clasificacion_qapf = (sio2 < 52.0) ? "Gabbro / Norite / Anorthosite" : "Diorite / Monzodiorite";
        }
    } else {
        res.clasificacion_qapf = "Ultramafic Cumulate (Dunite / Pyroxenite)";
    }

    return res;
}

} // namespace cma::application::modules
