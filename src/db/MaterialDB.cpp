#include "MaterialDB.hpp"
#include "nucleo/tabla_periodica.hpp"
#include <algorithm>
#include <cctype>
#include <cmath>
#include <fstream>
#include <iomanip>
#include <map>
#include <sstream>

namespace cma::application::modules {

namespace {

std::string trim_copy(const std::string& s) {
    size_t b = 0;
    while (b < s.size() && std::isspace(static_cast<unsigned char>(s[b]))) ++b;
    size_t e = s.size();
    while (e > b && std::isspace(static_cast<unsigned char>(s[e - 1]))) --e;
    return s.substr(b, e - b);
}

std::string lower_copy(const std::string& s) {
    std::string out = s;
    std::transform(out.begin(), out.end(), out.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return out;
}

// Extrae los símbolos elementales en orden de aparición de izquierda a derecha en la fórmula
std::vector<std::string> orden_aparicion_elementos(const std::string& formula) {
    std::vector<std::string> orden;
    const auto& tp = cma::nucleo::TablaPeriodica::instancia();
    size_t n = formula.size();
    size_t i = 0;
    while (i < n) {
        if (formula[i] == '(') {
            // Ignorar sufijos de fase como (s), (l), (g), (aq)
            auto close = formula.find(')', i + 1);
            if (close != std::string::npos) {
                std::string tag = formula.substr(i + 1, close - i - 1);
                if (tag == "s" || tag == "l" || tag == "g" || tag == "a" || tag == "aq") {
                    i = close + 1;
                    continue;
                }
            }
        }
        if (std::isupper(static_cast<unsigned char>(formula[i]))) {
            std::string sym(1, formula[i]);
            if (i + 1 < n && std::islower(static_cast<unsigned char>(formula[i + 1]))) {
                sym.push_back(formula[i + 1]);
                ++i;
            }
            if (tp.por_simbolo(sym)) {
                if (std::find(orden.begin(), orden.end(), sym) == orden.end()) {
                    orden.push_back(sym);
                }
            }
        }
        ++i;
    }
    return orden;
}

double masa_atomica_hsc(const std::string& simbolo) {
    const auto* el = cma::nucleo::TablaPeriodica::instancia().por_simbolo(simbolo);
    return el ? el->masa_atomica : 0.0;
}

} // namespace

MaterialDB::MaterialDB(const cma::domain::ThermoRepository& repository) : motor_(repository) {
    restaurar_predeterminados();
}

std::vector<const cma::nucleo::Especie*> MaterialDB::materiales() {
    auto todas = motor_.todas_especies();
    std::vector<const cma::nucleo::Especie*> out;
    for (auto* e : todas)
        if (e && e->fase_std != cma::nucleo::Fase::Gas)
            out.push_back(e);
    return out;
}

const RegistroMaterial* MaterialDB::buscar_por_nombre(const std::string& nombre) const {
    const std::string clave = lower_copy(trim_copy(nombre));
    for (const auto& m : catalogo_) {
        if (lower_copy(trim_copy(m.nombre)) == clave) {
            return &m;
        }
    }
    return nullptr;
}

RegistroMaterial* MaterialDB::buscar_por_nombre(const std::string& nombre) {
    const std::string clave = lower_copy(trim_copy(nombre));
    for (auto& m : catalogo_) {
        if (lower_copy(trim_copy(m.nombre)) == clave) {
            return &m;
        }
    }
    return nullptr;
}

void MaterialDB::agregar_o_actualizar(const RegistroMaterial& mat) {
    if (auto* existente = buscar_por_nombre(mat.nombre)) {
        *existente = mat;
    } else {
        catalogo_.push_back(mat);
    }
}

bool MaterialDB::eliminar(const std::string& nombre) {
    const std::string clave = lower_copy(trim_copy(nombre));
    auto it = std::remove_if(catalogo_.begin(), catalogo_.end(),
                             [&](const RegistroMaterial& m) {
                                 return lower_copy(trim_copy(m.nombre)) == clave;
                             });
    if (it != catalogo_.end()) {
        catalogo_.erase(it, catalogo_.end());
        return true;
    }
    return false;
}

void MaterialDB::resolver_especies_recursivo(
    const RegistroMaterial& mat,
    double factor_escala,
    std::vector<std::string>& pila_visitados,
    std::vector<EspecieResuelta>& acumuladas) const
{
    const std::string clave_actual = lower_copy(trim_copy(mat.nombre));
    if (std::find(pila_visitados.begin(), pila_visitados.end(), clave_actual) != pila_visitados.end()) {
        return; // Evitar ciclos recursivos entre sub-materiales
    }
    pila_visitados.push_back(clave_actual);

    for (const auto& comp : mat.composicion) {
        std::string nom = trim_copy(comp.nombre);
        if (nom.empty() || std::abs(comp.wt_pct) < 1e-12) continue;

        if (comp.es_submaterial()) {
            const std::string sub_nom = comp.nombre_submaterial();
            if (const auto* sub_mat = buscar_por_nombre(sub_nom)) {
                double nuevo_factor = factor_escala * (comp.wt_pct / 100.0);
                resolver_especies_recursivo(*sub_mat, nuevo_factor, pila_visitados, acumuladas);
                continue;
            }
        }

        // Especie química directa
        const double aporte_wt = factor_escala * comp.wt_pct;
        auto it = std::find_if(acumuladas.begin(), acumuladas.end(),
                               [&](const EspecieResuelta& e) { return e.formula == nom; });
        if (it != acumuladas.end()) {
            it->wt_pct += aporte_wt;
            if (it->referencia.empty() && !comp.referencia.empty()) {
                it->referencia = comp.referencia;
            }
        } else {
            acumuladas.push_back({nom, aporte_wt, comp.referencia});
        }
    }

    pila_visitados.pop_back();
}

std::vector<EspecieResuelta> MaterialDB::resolver_especies(const RegistroMaterial& mat) const {
    std::vector<EspecieResuelta> resultado;
    std::vector<std::string> visitados;
    resolver_especies_recursivo(mat, 1.0, visitados, resultado);
    return resultado;
}

std::vector<ElementoResuelto> MaterialDB::resolver_elementos(
    const std::vector<EspecieResuelta>& especies) const
{
    const auto& tp = cma::nucleo::TablaPeriodica::instancia();
    std::vector<ElementoResuelto> resultado;

    for (const auto& esp : especies) {
        if (esp.formula.empty() || std::abs(esp.wt_pct) < 1e-12) continue;
        auto atomos = tp.analizar_formula(esp.formula);
        if (atomos.empty()) continue;

        double masa_molar = 0.0;
        std::map<std::string, double> masa_por_elem;
        for (const auto& a : atomos) {
            const double m_at = masa_atomica_hsc(a.elem);
            if (m_at > 0.0 && a.cuenta > 0) {
                const double m = m_at * static_cast<double>(a.cuenta);
                masa_por_elem[a.elem] += m;
                masa_molar += m;
            }
        }
        if (masa_molar <= 1e-12) continue;

        // Respetar el orden de aparición de izquierda a derecha en la fórmula
        std::vector<std::string> orden = orden_aparicion_elementos(esp.formula);
        for (const auto& a : atomos) {
            if (std::find(orden.begin(), orden.end(), a.elem) == orden.end()) {
                orden.push_back(a.elem);
            }
        }

        for (const auto& simb : orden) {
            auto it_m = masa_por_elem.find(simb);
            if (it_m == masa_por_elem.end()) continue;
            const double wt_elem = esp.wt_pct * (it_m->second / masa_molar);
            auto it_res = std::find_if(resultado.begin(), resultado.end(),
                                       [&](const ElementoResuelto& r) { return r.simbolo == simb; });
            if (it_res != resultado.end()) {
                it_res->wt_pct += wt_elem;
            } else {
                resultado.push_back({simb, wt_elem});
            }
        }
    }

    return resultado;
}

std::vector<ElementoResuelto> MaterialDB::resolver_elementos(const RegistroMaterial& mat) const {
    return resolver_elementos(resolver_especies(mat));
}

double MaterialDB::suma_composicion(const RegistroMaterial& mat) {
    double total = 0.0;
    for (const auto& c : mat.composicion) total += c.wt_pct;
    return total;
}

double MaterialDB::suma_especies(const std::vector<EspecieResuelta>& esp) {
    double total = 0.0;
    for (const auto& e : esp) total += e.wt_pct;
    return total;
}

double MaterialDB::suma_elementos(const std::vector<ElementoResuelto>& elem) {
    double total = 0.0;
    for (const auto& e : elem) total += e.wt_pct;
    return total;
}

void MaterialDB::normalizar_composicion(RegistroMaterial& mat) {
    const double suma = suma_composicion(mat);
    if (suma <= 1e-12) return;
    const double factor = 100.0 / suma;
    for (auto& c : mat.composicion) {
        c.wt_pct *= factor;
    }
}

bool MaterialDB::exportar_csv_material(const std::string& ruta, const RegistroMaterial& mat) const {
    std::ofstream out(ruta);
    if (!out.is_open()) return false;

    const auto especies = resolver_especies(mat);
    const auto elementos = resolver_elementos(especies);

    out << "# HSC Material Database Export - CMA Chemistry\n";
    out << "# Material," << mat.nombre << ",Category," << mat.categoria << ",Code," << mat.codigo << "\n";
    out << "Composition,wt-%,Species,wt-%,Elements,wt-%,Reference\n";
    out << std::fixed << std::setprecision(3);
    out << "Total," << suma_composicion(mat)
        << ",," << suma_especies(especies)
        << ",," << suma_elementos(elementos) << ",\n";

    const size_t max_rows = std::max({mat.composicion.size(), especies.size(), elementos.size()});
    for (size_t i = 0; i < max_rows; ++i) {
        if (i < mat.composicion.size()) {
            out << "\"" << mat.composicion[i].nombre << "\"," << mat.composicion[i].wt_pct << ",";
        } else {
            out << ",,";
        }
        if (i < especies.size()) {
            out << "\"" << especies[i].formula << "\"," << especies[i].wt_pct << ",";
        } else {
            out << ",,";
        }
        if (i < elementos.size()) {
            out << "\"" << elementos[i].simbolo << "\"," << elementos[i].wt_pct << ",";
        } else {
            out << ",,";
        }
        if (i < mat.composicion.size()) {
            out << "\"" << mat.composicion[i].referencia << "\"";
        }
        out << "\n";
    }

    if (!mat.propiedades_fisicas.empty() || !mat.analisis.empty() || !mat.propiedades_usuario.empty()) {
        out << "\n# Material Properties & Analyses\n";
        out << "Section,Property,Value,Unit,Reference\n";
        for (const auto& p : mat.propiedades_fisicas) {
            out << "Physical,\"" << p.nombre << "\"," << p.valor << ",\"" << p.unidad << "\",\"" << p.referencia << "\"\n";
        }
        for (const auto& p : mat.analisis) {
            out << "Analysis,\"" << p.nombre << "\"," << p.valor << ",\"" << p.unidad << "\",\"" << p.referencia << "\"\n";
        }
        for (const auto& p : mat.propiedades_usuario) {
            out << "UserDefined,\"" << p.nombre << "\"," << p.valor << ",\"" << p.unidad << "\",\"" << p.referencia << "\"\n";
        }
    }
    return true;
}

bool MaterialDB::guardar_catalogo_tsv(const std::string& ruta) const {
    std::ofstream out(ruta);
    if (!out.is_open()) return false;
    out << "CMA_MATERIAL_DB_V1\n";
    out << std::fixed << std::setprecision(6);
    for (const auto& m : catalogo_) {
        out << "MATERIAL\t" << m.nombre << "\t" << m.categoria << "\t"
            << m.codigo << "\t" << m.descripcion << "\t" << m.referencia << "\n";
        for (const auto& c : m.composicion) {
            out << "COMP\t" << c.nombre << "\t" << c.wt_pct << "\t" << c.referencia << "\n";
        }
        for (const auto& p : m.propiedades_fisicas) {
            out << "PHYS\t" << p.nombre << "\t" << p.valor << "\t" << p.unidad << "\t" << p.referencia << "\n";
        }
        for (const auto& p : m.analisis) {
            out << "ANAL\t" << p.nombre << "\t" << p.valor << "\t" << p.unidad << "\t" << p.referencia << "\n";
        }
        for (const auto& p : m.propiedades_usuario) {
            out << "USER\t" << p.nombre << "\t" << p.valor << "\t" << p.unidad << "\t" << p.referencia << "\n";
        }
        out << "END_MATERIAL\n";
    }
    return true;
}

bool MaterialDB::cargar_catalogo_tsv(const std::string& ruta) {
    std::ifstream in(ruta);
    if (!in.is_open()) return false;
    std::string header;
    if (!std::getline(in, header) || header.find("CMA_MATERIAL_DB_V1") == std::string::npos) {
        return false;
    }

    std::vector<RegistroMaterial> nuevo;
    RegistroMaterial actual;
    bool en_mat = false;
    std::string line;
    auto split_tabs = [](const std::string& s) {
        std::vector<std::string> parts;
        std::stringstream ss(s);
        std::string item;
        while (std::getline(ss, item, '\t')) parts.push_back(item);
        return parts;
    };

    while (std::getline(in, line)) {
        if (!line.empty() && line.back() == '\r') line.pop_back();
        if (line.empty()) continue;
        auto cols = split_tabs(line);
        if (cols.empty()) continue;
        if (cols[0] == "MATERIAL" && cols.size() >= 3) {
            actual = RegistroMaterial{};
            actual.nombre = cols[1];
            actual.categoria = cols[2];
            if (cols.size() > 3) actual.codigo = cols[3];
            if (cols.size() > 4) actual.descripcion = cols[4];
            if (cols.size() > 5) actual.referencia = cols[5];
            en_mat = true;
        } else if (cols[0] == "END_MATERIAL" && en_mat) {
            nuevo.push_back(actual);
            en_mat = false;
        } else if (en_mat && cols[0] == "COMP" && cols.size() >= 3) {
            ComponenteMaterial c;
            c.nombre = cols[1];
            try { c.wt_pct = std::stod(cols[2]); } catch (...) { c.wt_pct = 0.0; }
            if (cols.size() > 3) c.referencia = cols[3];
            actual.composicion.push_back(c);
        } else if (en_mat && (cols[0] == "PHYS" || cols[0] == "ANAL" || cols[0] == "USER") && cols.size() >= 4) {
            PropiedadMaterial p;
            p.nombre = cols[1];
            try { p.valor = std::stod(cols[2]); } catch (...) { p.valor = 0.0; }
            p.unidad = cols[3];
            if (cols.size() > 4) p.referencia = cols[4];
            p.es_usuario = (cols[0] == "USER");
            if (cols[0] == "PHYS") actual.propiedades_fisicas.push_back(p);
            else if (cols[0] == "ANAL") actual.analisis.push_back(p);
            else actual.propiedades_usuario.push_back(p);
        }
    }

    if (!nuevo.empty()) {
        catalogo_ = std::move(nuevo);
        return true;
    }
    return false;
}

void MaterialDB::restaurar_predeterminados() {
    catalogo_.clear();

    // 1. Smelter Feed Blend (Reproduce exactamente la hoja de referencia de HSC Material Database)
    {
        RegistroMaterial m;
        m.nombre = "Smelter Feed Blend";
        m.categoria = "Blends";
        m.codigo = "MAT-BLD-001";
        m.descripcion = "Flash smelter composite feed charge ([Concentrate] + [Silica Flux] + Coke breeze + Moisture)";
        m.referencia = "HSC Sim Flash Smelting Reference Feed";
        m.composicion = {
            {"[Concentrate]", 75.000, "Sulfide Concentrate Feed"},
            {"[Silica Flux]", 20.000, "Quartz Fluxing Agent"},
            {"C",              3.000, "Metallurgical Coke Fines"},
            {"H2O",            2.000, "Residual Feed Moisture"}
        };
        m.propiedades_fisicas = {
            {"Bulk Density", 1850.0, "kg/m3", "Plant silo measurement", false},
            {"True Density", 3620.0, "kg/m3", "He pycnometer", false},
            {"Moisture Content", 2.00, "wt-%", "Dryer outlet target", false},
            {"Particle Size d80", 74.0, "um", "Laser diffraction", false},
            {"Specific Heat (25 C)", 0.68, "kJ/(kg.K)", "Calorimetry", false},
            {"Angle of Repose", 36.5, "deg", "Feed bin design", false}
        };
        m.analisis = {
            {"Cu Grade (Assay)", 13.335, "wt-%", "ICP-OES / XRF", false},
            {"Fe Grade (Assay)", 23.795, "wt-%", "ICP-OES / XRF", false},
            {"S Total (LECO)",   26.038, "wt-%", "LECO Combustion", false},
            {"SiO2 Grade",       23.805, "wt-%", "XRF Fusion Bead", false},
            {"Fe/SiO2 Ratio",     1.000, "kg/kg", "Slag control index", false}
        };
        m.propiedades_usuario = {
            {"Work Index (Bond BWi)", 13.40, "kWh/t", "Comminution lab test", true},
            {"Dust Generation Index",  4.20, "wt-%", "Flash shaft carry-over", true},
            {"Feed Cost",            845.00, "USD/t", "Contract valuation", true}
        };
        catalogo_.push_back(m);
    }

    // 2. Concentrate (Sub-material referenciado por [Concentrate] al 75.000 wt-%)
    {
        RegistroMaterial m;
        m.nombre = "Concentrate";
        m.categoria = "Concentrates";
        m.codigo = "MAT-CON-001";
        m.descripcion = "Copper-iron sulfide flotation concentrate (Chalcopyrite-Pyrite-Chalcocite-Bornite)";
        m.referencia = "Flotation Plant Final Cu Concentrate";
        m.composicion = {
            {"CuFeS2",  22.450, "Chalcopyrite (MLA/QEMSCAN)"},
            {"Cu5FeS4",  1.755, "Bornite"},
            {"Cu2S",    11.140, "Chalcocite"},
            {"FeS",     12.570, "Pyrrhotite"},
            {"FeS2",    35.000, "Pyrite"},
            {"ZnS",      1.750, "Sphalerite"},
            {"NiS",      0.879, "Millerite / Pentlandite"},
            {"SiO2",     8.660, "Quartz gangue"},
            {"MgO",      5.372, "Talc / Mg-silicate gangue"},
            {"Al2O3",    0.424, "Aluminosilicate gangue"}
        };
        m.propiedades_fisicas = {
            {"Bulk Density", 2050.0, "kg/m3", "Filter cake", false},
            {"True Density", 4180.0, "kg/m3", "Pycnometer", false},
            {"Moisture Content", 8.50, "wt-%", "Larox filter cake", false},
            {"Particle Size d80", 52.0, "um", "Cyclone overflow", false},
            {"Net Calorific Value", 6.45, "MJ/kg", "Sulfide oxidation heat", false}
        };
        m.analisis = {
            {"Cu Assay", 17.780, "wt-%", "Wet chemical titration", false},
            {"Fe Assay", 31.303, "wt-%", "XRF", false},
            {"S Assay",  34.718, "wt-%", "LECO S", false},
            {"Insolubles (SiO2+Al2O3)", 9.084, "wt-%", "Acid digestion", false}
        };
        m.propiedades_usuario = {
            {"Flotation Recovery", 91.5, "%", "Concentrator KPI", true},
            {"Au Credit", 1.85, "g/t", "Fire assay", true},
            {"Ag Credit", 42.0, "g/t", "Fire assay", true}
        };
        catalogo_.push_back(m);
    }

    // 3. Silica Flux (Sub-material referenciado por [Silica Flux] al 20.000 wt-%)
    {
        RegistroMaterial m;
        m.nombre = "Silica Flux";
        m.categoria = "Fluxes";
        m.codigo = "MAT-FLX-001";
        m.descripcion = "High-purity metallurgical quartz flux for fayalite slag formation";
        m.referencia = "Quarry Crushed Silica Flux";
        m.composicion = {
            {"SiO2",  86.550, "Alpha-Quartz"},
            {"Al2O3",  5.685, "Feldspar / Kaolinite"},
            {"Fe2O3",  2.270, "Hematite coating"},
            {"CaO",    5.500, "Calcite / Lime"}
        };
        m.propiedades_fisicas = {
            {"Bulk Density", 1520.0, "kg/m3", "Crushed flux bin", false},
            {"True Density", 2650.0, "kg/m3", "Quartz standard", false},
            {"Particle Size d80", 120.0, "um", "Ball mill product", false},
            {"Melting Point", 1670.0, "C", "Cristobalite liquidus", false}
        };
        m.analisis = {
            {"Available SiO2", 86.55, "wt-%", "XRF", false},
            {"Basicity (CaO/SiO2)", 0.064, "ratio", "Slag chemistry", false}
        };
        m.propiedades_usuario = {
            {"Bond Work Index", 14.8, "kWh/t", "Quarry sample", true}
        };
        catalogo_.push_back(m);
    }

    // 4. Bituminous Coal (Categoría: Fuels)
    {
        RegistroMaterial m;
        m.nombre = "Bituminous Coal";
        m.categoria = "Fuels";
        m.codigo = "MAT-FUL-001";
        m.descripcion = "Pulverized bituminous coal for rotary kiln and reverberatory furnace firing";
        m.referencia = "Industrial Fuel Database - ASTM D388";
        m.composicion = {
            {"C",     74.200, "Fixed + Volatile Carbon"},
            {"H2O",    5.500, "Inherent Moisture"},
            {"FeS2",   1.800, "Pyritic Sulfur"},
            {"SiO2",   9.500, "Coal Ash - Silica"},
            {"Al2O3",  5.200, "Coal Ash - Alumina"},
            {"CaO",    1.800, "Coal Ash - Calcia"},
            {"MgO",    2.000, "Coal Ash - Magnesia"}
        };
        m.propiedades_fisicas = {
            {"Higher Heating Value (HHV)", 29.85, "MJ/kg", "Bomb calorimeter", false},
            {"Lower Heating Value (LHV)",  28.40, "MJ/kg", "ISO 1928", false},
            {"Bulk Density", 820.0, "kg/m3", "Pulverized silo", false},
            {"Ash Fusion Temp (IDT)", 1280.0, "C", "Reducing atmosphere", false}
        };
        m.analisis = {
            {"Fixed Carbon (Proximate)", 56.40, "wt-%", "ASTM D3172", false},
            {"Volatile Matter",          21.60, "wt-%", "ASTM D3175", false},
            {"Ash Content",              16.50, "wt-%", "ASTM D3174", false},
            {"Moisture",                  5.50, "wt-%", "ASTM D3173", false}
        };
        m.propiedades_usuario = {
            {"Hardgrove Grindability (HGI)", 58.0, "index", "ASTM D409", true},
            {"CO2 Emission Factor", 2.72, "kg CO2/kg", "IPCC Tier 2", true}
        };
        catalogo_.push_back(m);
    }

    // 5. Refinery Anode Sludge (Categoría: Sludges)
    {
        RegistroMaterial m;
        m.nombre = "Refinery Anode Sludge";
        m.categoria = "Sludges";
        m.codigo = "MAT-SLG-001";
        m.descripcion = "Electrolytic copper tankhouse anode slime rich in precious metals, Pb and Se";
        m.referencia = "Tankhouse Decantation Underflow";
        m.composicion = {
            {"Cu2S",  24.500, "Residual Copper Sulfide"},
            {"PbSO4", 32.000, "Anglesite precipitate"},
            {"Ag",    14.500, "Metallic Silver"},
            {"Au",     0.850, "Metallic Gold"},
            {"SiO2",   8.150, "Insoluble silica"},
            {"H2O",   20.000, "Filter cake moisture"}
        };
        m.propiedades_fisicas = {
            {"Slurry Density", 1680.0, "kg/m3", "Decanter underflow", false},
            {"Solids Concentration", 80.0, "wt-%", "Pressure filter", false},
            {"Particle Size d50", 18.0, "um", "Fine precipitate", false}
        };
        m.analisis = {
            {"Ag Assay", 14.50, "wt-%", "Fire Assay / ICP", false},
            {"Au Assay",  0.85, "wt-%", "Fire Assay", false},
            {"Pb Assay", 21.86, "wt-%", "AAS", false}
        };
        m.propiedades_usuario = {
            {"Precious Metal Value", 185000.0, "USD/t", "Spot basket estimate", true}
        };
        catalogo_.push_back(m);
    }

    // 6. Stainless Steel 316L Scrap (Categoría: Steels)
    {
        RegistroMaterial m;
        m.nombre = "Stainless Steel 316L";
        m.categoria = "Steels";
        m.codigo = "MAT-STL-316L";
        m.descripcion = "Austenitic Cr-Ni-Mo stainless steel scrap for EAF / AOD charge modeling";
        m.referencia = "ASTM A240 / AISI 316L";
        m.composicion = {
            {"Fe", 68.470, "Iron Matrix"},
            {"Cr", 17.000, "Chromium Alloy"},
            {"Ni", 11.000, "Nickel Alloy"},
            {"Mo",  2.200, "Molybdenum Alloy"},
            {"Mn",  1.300, "Manganese"},
            {"C",   0.030, "Low Carbon (L grade)"}
        };
        m.propiedades_fisicas = {
            {"True Density", 7990.0, "kg/m3", "20 C", false},
            {"Melting Point (Liquidus)", 1400.0, "C", "Fe-Cr-Ni phase diagram", false},
            {"Thermal Conductivity", 16.3, "W/(m.K)", "At 100 C", false},
            {"Specific Heat", 0.50, "kJ/(kg.K)", "At 25 C", false}
        };
        m.analisis = {
            {"PREN (Pitting Resistance)", 24.26, "index", "%Cr + 3.3*%Mo + 16*%N", false}
        };
        m.propiedades_usuario = {
            {"Yield Strength (Rp0.2)", 205.0, "MPa", "Annealed plate", true},
            {"Tensile Strength (Rm)",  515.0, "MPa", "ASTM A240", true}
        };
        catalogo_.push_back(m);
    }

    // 7. Shredded PE/PP Plastic Waste (Categoría: Plastics)
    {
        RegistroMaterial m;
        m.nombre = "Shredded PE-PP Plastic";
        m.categoria = "Plastics";
        m.codigo = "MAT-PLS-001";
        m.descripcion = "Recycled polyolefin plastic granulate used as alternative reducing agent in smelting";
        m.referencia = "Secondary Raw Materials / Circular Metallurgy";
        m.composicion = {
            {"C2H4",  92.500, "Polyethylene / Polypropylene monomer eq."},
            {"CaCO3",  4.500, "Mineral filler"},
            {"TiO2",   1.500, "White pigment"},
            {"H2O",    1.500, "Surface moisture"}
        };
        m.propiedades_fisicas = {
            {"Bulk Density", 420.0, "kg/m3", "Shredded flakes", false},
            {"True Density", 935.0, "kg/m3", "ASTM D792", false},
            {"Lower Heating Value (LHV)", 41.20, "MJ/kg", "Calorimetric bomb", false},
            {"Melting Range", 135.0, "C", "DSC peak", false}
        };
        m.analisis = {
            {"Carbon Equivalent", 79.82, "wt-%", "Ultimate analysis", false},
            {"Hydrogen Equivalent", 13.41, "wt-%", "Ultimate analysis", false},
            {"Ash Residue", 6.00, "wt-%", "600 C calcination", false}
        };
        m.propiedades_usuario = {
            {"Coke Replacement Ratio", 1.25, "kg coke/kg plastic", "Blast furnace tuyere injection", true}
        };
        catalogo_.push_back(m);
    }

    // 8. E-Waste Scrap (Categoría: Plastics / E-Waste - del modelo HSC Sim E-Waste and Copper Scrap Smelting)
    {
        RegistroMaterial m;
        m.nombre = "E-Waste 25oC";
        m.categoria = "Plastics";
        m.codigo = "MAT-EW-001";
        m.descripcion = "Shredded WEEE printed circuit boards (PCBs) for Ausmelt copper scrap smelting";
        m.referencia = "HSC Sim E-Waste and Copper Scrap Smelting";
        m.composicion = {
            {"Cu",    26.500, "PCB copper tracks & pins"},
            {"SiO2",  24.000, "FR-4 fiberglass reinforcement"},
            {"Al2O3",  8.500, "Ceramic capacitors & IC packages"},
            {"Fe",     6.200, "Ferrous frames & leads"},
            {"C2H4",  28.800, "Epoxy-brominated resin matrix"},
            {"CaO",    4.500, "Glass filler"},
            {"Ag",     0.350, "Solder & contacts"},
            {"Au",     0.050, "Gold-plated connectors"},
            {"H2O",    1.100, "Ambient moisture"}
        };
        m.propiedades_fisicas = {
            {"Bulk Density", 680.0, "kg/m3", "Shredded PCB <15 mm", false},
            {"Lower Heating Value (LHV)", 11.80, "MJ/kg", "Resin combustion energy", false},
            {"Particle Size d80", 12000.0, "um", "Hammer mill product", false}
        };
        m.analisis = {
            {"Cu Assay", 26.50, "wt-%", "ICP-OES after smelting", false},
            {"Organic Fraction (LOI)", 28.80, "wt-%", "TGA 850 C", false},
            {"Au Content", 500.0, "g/t", "Fire assay", false},
            {"Ag Content", 3500.0, "g/t", "Fire assay", false}
        };
        m.propiedades_usuario = {
            {"Halogen Content (Br+Cl)", 1.85, "wt-%", "Offgas scrubber design", true},
            {"Gate Fee / Purchase Cost", 1420.0, "USD/t", "Recycling contract", true}
        };
        catalogo_.push_back(m);
    }

    // 9. Black Copper 25oC (Categoría: Steels / Secondary Alloys - modelo Ausmelt HSC Sim)
    {
        RegistroMaterial m;
        m.nombre = "Black Copper 25oC";
        m.categoria = "Steels";
        m.codigo = "MAT-BCU-001";
        m.descripcion = "Secondary black copper bullion from reducing smelting of copper scrap & slag";
        m.referencia = "Ausmelt Furnace Secondary Copper Feed";
        m.composicion = {
            {"Cu", 82.500, "Metallic Copper matrix"},
            {"Fe",  7.800, "Dissolved Iron"},
            {"Zn",  4.200, "Dissolved Zinc"},
            {"Pb",  2.900, "Dissolved Lead"},
            {"Ni",  1.600, "Dissolved Nickel"},
            {"Sn",  1.000, "Dissolved Tin (Bronze/Solder)"}
        };
        m.propiedades_fisicas = {
            {"True Density", 8650.0, "kg/m3", "Cast ingots", false},
            {"Liquidus Temperature", 1065.0, "C", "Cu-Fe-Zn-Pb alloy", false}
        };
        m.analisis = {
            {"Cu Purity", 82.50, "wt-%", "XRF / Electrogravimetry", false},
            {"Sn+Pb+Zn Impurities", 8.10, "wt-%", "Converter slagging load", false}
        };
        m.propiedades_usuario = {
            {"Refining Charge (RC)", 185.0, "USD/t", "Secondary smelter tariff", true}
        };
        catalogo_.push_back(m);
    }

    // 10. Limestone 25oC (Categoría: Fluxes - modelo Ausmelt HSC Sim)
    {
        RegistroMaterial m;
        m.nombre = "Limestone 25oC";
        m.categoria = "Fluxes";
        m.codigo = "MAT-FLX-002";
        m.descripcion = "Metallurgical limestone flux for CaO-FeO-SiO2-Al2O3 slag basicity control";
        m.referencia = "HSC Sim Ausmelt Furnace Flux";
        m.composicion = {
            {"CaCO3", 94.500, "Calcite"},
            {"MgCO3",  2.800, "Dolomite minor"},
            {"SiO2",   1.700, "Silica impurity"},
            {"H2O",    1.000, "Surface moisture"}
        };
        m.propiedades_fisicas = {
            {"Bulk Density", 1450.0, "kg/m3", "Crushed limestone", false},
            {"True Density", 2710.0, "kg/m3", "Calcite", false},
            {"Calcination Temperature", 898.0, "C", "1 atm CO2", false}
        };
        m.analisis = {
            {"Available CaO", 52.95, "wt-%", "After calcination", false},
            {"LOI (CO2 + H2O)", 44.01, "wt-%", "1000 C furnace", false}
        };
        m.propiedades_usuario = {
            {"Neutralizing Value", 96.8, "% CaCO3 eq.", "Acid plant effluent", true}
        };
        catalogo_.push_back(m);
    }

    // 11. Slag heel 1250oC (Categoría: Slags - modelo Ausmelt HSC Sim)
    {
        RegistroMaterial m;
        m.nombre = "Slag heel 1250oC";
        m.categoria = "Slags";
        m.codigo = "MAT-SLG-1250";
        m.descripcion = "Molten iron-silicate-calcium slag heel retained in Ausmelt TSL furnace at 1250 C";
        m.referencia = "Ausmelt Furnace Molten Slag Bath";
        m.composicion = {
            {"FeO",   42.000, "Wustite / Fayalite component"},
            {"Fe3O4",  6.500, "Magnetite spinels"},
            {"SiO2",  31.500, "Silicate network"},
            {"CaO",   11.000, "Dissolved lime"},
            {"Al2O3",  6.800, "Dissolved alumina from E-Waste"},
            {"Cu2O",   1.200, "Dissolved cuprous oxide"},
            {"ZnO",    1.000, "Slagged zinc oxide"}
        };
        m.propiedades_fisicas = {
            {"Melt Density (1250 C)", 3480.0, "kg/m3", "Molten bath", false},
            {"Viscosity (1250 C)", 0.42, "Pa.s", "Urbain model", false},
            {"Liquidus Temperature", 1175.0, "C", "FeO-SiO2-CaO-Al2O3", false}
        };
        m.analisis = {
            {"Fe/SiO2 Ratio", 1.24, "wt/wt", "Slag control", false},
            {"Cu Loss in Slag", 1.07, "wt-%", "Settling furnace feed", false}
        };
        m.propiedades_usuario = {
            {"Magnetite Saturation Index", 0.72, "ratio", "Lance immersion control", true}
        };
        catalogo_.push_back(m);
    }
}

} // namespace cma::application::modules
