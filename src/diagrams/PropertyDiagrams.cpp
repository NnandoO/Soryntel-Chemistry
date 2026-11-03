#include "PropertyDiagrams.hpp"

#include "nucleo/tabla_periodica.hpp"
#include "nucleo/tipos.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <iomanip>
#include <map>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

namespace cma::application::modules {

namespace {

constexpr double kR_J = 8.314462618;
constexpr double kR_kJ = 0.008314462618;
constexpr double kLn10 = 2.302585092994046;
constexpr double kTref = 298.15;

std::string trim_str(const std::string& s) {
    size_t a = 0;
    while (a < s.size() && std::isspace(static_cast<unsigned char>(s[a])) != 0) {
        ++a;
    }
    size_t b = s.size();
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1])) != 0) {
        --b;
    }
    return s.substr(a, b - a);
}

// Extrae sufijo explicito de fase si existe: "(s)", "(l)", "(g)", "(cr)", "(aq)", "(α)"
std::string clean_formula_str(const std::string& raw, std::string* phase_tag_out = nullptr) {
    std::string s = trim_str(raw);
    if (phase_tag_out != nullptr) {
        phase_tag_out->clear();
    }
    auto pos = s.rfind('(');
    if (pos != std::string::npos && pos > 0 && s.back() == ')') {
        std::string inside = s.substr(pos + 1, s.size() - pos - 2);
        std::string low;
        for (char c : inside) {
            low.push_back(static_cast<char>(std::tolower(static_cast<unsigned char>(c))));
        }
        if (low == "s" || low == "l" || low == "g" || low == "cr" || low == "aq" || low == "a" ||
            inside == "α" || inside == "β" || inside == "γ" || low == "alpha" || low == "beta") {
            if (phase_tag_out != nullptr) {
                *phase_tag_out = low;
            }
            return trim_str(s.substr(0, pos));
        }
    }
    return s;
}

bool es_especie_acuosa(const nucleo::Especie& esp) {
    return (esp.formula.find('+') != std::string::npos || esp.formula.find('-') != std::string::npos ||
            esp.formula.find("⁺") != std::string::npos || esp.formula.find("⁻") != std::string::npos ||
            esp.formula.find("(aq)") != std::string::npos || esp.formula.find("(a)") != std::string::npos);
}

nucleo::Especie make_sp(int64_t id, const char* nombre, const char* formula, double M, double Hf,
                        double Sf, double cpa, double cpb, double cpc, double cpd,
                        nucleo::Fase fase) {
    nucleo::Especie e;
    e.id = id; e.nombre = nombre; e.formula = formula; e.masa_molar = M;
    e.h_formacion = Hf; e.s_formacion = Sf;
    e.cp_a = cpa; e.cp_b = cpb; e.cp_c = cpc; e.cp_d = cpd; e.fase_std = fase;
    return e;
}

// Construye un polimorfo de alta T (liquido o gas) anclado termodinamicamente a la fase previa en T_tr
nucleo::Especie make_transition_polymorph(const nucleo::Especie& prev_phase, const std::string& new_formula,
                                          const std::string& new_name, nucleo::Fase new_fase,
                                          double T_tr, double dH_tr_kJ, double cp_a_new, double cp_b_new = 0.0) {
    nucleo::Especie e = prev_phase;
    e.nombre = new_name;
    e.formula = new_formula;
    e.fase_std = new_fase;
    e.cp_a = cp_a_new;
    e.cp_b = cp_b_new;
    e.cp_c = 0.0;
    e.cp_d = 0.0;
    e.h_formacion = 0.0;
    e.s_formacion = 0.0;
    const double H_prev = prev_phase.entalpia_abs(T_tr);
    const double S_prev = prev_phase.entropia_abs(T_tr);
    const double dH_ref_new = e.entalpia_ref(T_tr);
    const double dS_ref_new = e.entropia_ref(T_tr);
    e.h_formacion = H_prev + dH_tr_kJ - dH_ref_new;
    e.s_formacion = S_prev + (dH_tr_kJ * 1000.0 / T_tr) - dS_ref_new;
    return e;
}

struct TransSpec {
    const char* clean_sym;
    double T_m;
    double dH_m;
    double cp_l;
    double T_b;
    double dH_v;
    double cp_g;
};

const std::vector<TransSpec>& tabla_transiciones() {
    static const std::vector<TransSpec> tbl = {
        {"Zn", 692.68, 7.32, 31.38, 1180.0, 115.30, 20.786}, {"Mg", 923.00, 8.48, 34.30, 1363.0, 127.40, 20.786},
        {"Ca", 1115.0, 8.54, 35.00, 1757.0, 154.70, 20.786}, {"Pb", 600.61, 4.77, 30.63, 2022.0, 179.50, 20.786},
        {"Al", 933.47, 10.71, 31.75, 2792.0, 294.00, 20.786}, {"Cu", 1357.77, 13.26, 32.84, 2835.0, 300.40, 20.786},
        {"Fe", 1811.0, 13.81, 46.02, 3134.0, 340.00, 20.786}, {"Ni", 1728.0, 17.48, 43.10, 3186.0, 377.50, 20.786},
        {"Si", 1687.0, 50.21, 27.20, 3538.0, 383.00, 20.786}, {"Mn", 1519.0, 12.91, 46.00, 2334.0, 221.00, 20.786},
        {"Ti", 1941.0, 14.15, 47.20, 3560.0, 425.00, 20.786}, {"Cr", 2180.0, 21.00, 39.30, 2944.0, 339.50, 20.786},
        {"Mo", 2896.0, 37.48, 42.00, 4912.0, 598.00, 20.786}, {"Na", 370.95, 2.60, 31.50, 1156.0, 97.40, 20.786},
        {"K",  336.53, 2.33, 32.00, 1032.0, 76.90, 20.786},   {"PbO", 1159.0, 25.50, 65.00, 0.0, 0.0, 0.0},
        {"FeO", 1650.0, 24.06, 68.20, 0.0, 0.0, 0.0},         {"Cu2O", 1505.0, 64.80, 100.0, 0.0, 0.0, 0.0},
        {"FeCl2", 950.0, 43.00, 102.2, 1299.0, 126.0, 83.0},   {"ZnCl2", 591.0, 10.25, 100.8, 1005.0, 119.0, 62.0},
        {"PbCl2", 774.0, 21.90, 111.5, 1224.0, 127.0, 80.0},   {"MgCl2", 987.0, 43.10, 92.10, 1685.0, 156.0, 80.0},
        {"CaCl2", 1045.0, 28.50, 103.0, 2208.0, 235.0, 82.0},
    };
    return tbl;
}

// Catalogo termodinamico base curado de respaldo (JANAF / Barin / HSC)
const std::vector<nucleo::Especie>& catalogo_respaldo() {
    static const std::vector<nucleo::Especie> cat = {
        make_sp(1, "Oxígeno", "O2", 31.999, 0.0, 205.147, 24.6979, 0.0167754, -8.151e-6, 1.4986e-9, nucleo::Fase::Gas),
        make_sp(2, "Nitrógeno", "N2", 28.013, 0.0, 191.609, 27.3069, 0.0061500, -9.502e-7, 0.0, nucleo::Fase::Gas),
        make_sp(3, "Hidrógeno", "H2", 2.016, 0.0, 130.680, 27.1415, 0.0092744, -1.3015e-6, 7.6313e-9, nucleo::Fase::Gas),
        make_sp(4, "Cloro", "Cl2", 70.905, 0.0, 223.081, 33.0510, 0.0012229, 1.4466e-6, -1.4987e-9, nucleo::Fase::Gas),
        make_sp(5, "Flúor", "F2", 37.997, 0.0, 202.790, 31.3000, 0.0041800, -1.200e-6, 1.200e-9, nucleo::Fase::Gas),
        make_sp(6, "Azufre diatómico", "S2", 64.132, 128.600, 228.170, 32.4700, 0.0050000, 0.0, 0.0, nucleo::Fase::Gas),
        make_sp(7, "Dióxido de carbono", "CO2", 44.010, -393.522, 213.795, 23.1089, 0.0577123, -3.3541e-5, 7.1345e-9, nucleo::Fase::Gas),
        make_sp(8, "Monóxido de carbono", "CO", 28.010, -110.527, 197.660, 27.3579, 0.0037843, 3.741e-6, -1.8158e-9, nucleo::Fase::Gas),
        make_sp(9, "Agua (vapor)", "H2O(g)", 18.015, -241.826, 188.835, 31.9500, 0.0028273, 1.0e-5, -3.3741e-9, nucleo::Fase::Gas),
        make_sp(10, "Agua (líquida)", "H2O", 18.015, -285.830, 69.950, 75.1500, 0.0010800, 0.0, 0.0, nucleo::Fase::Liquido),
        make_sp(11, "Dióxido de azufre", "SO2", 64.064, -296.842, 248.223, 38.8310, 0.0016215, 2.0518e-6, -2.3557e-9, nucleo::Fase::Gas),
        make_sp(12, "Trióxido de azufre", "SO3", 80.064, -395.765, 256.769, 48.5180, 0.0072450, 5.8552e-6, -9.2519e-9, nucleo::Fase::Gas),
        make_sp(13, "Sulfuro de hidrógeno", "H2S", 34.080, -20.500, 205.800, 34.5350, 0.0012540, 2.9077e-6, -1.5287e-9, nucleo::Fase::Gas),
        make_sp(14, "Cloruro de hidrógeno", "HCl", 36.461, -92.311, 186.900, 29.1330, -0.0000996, 1.95e-6, -1.0394e-9, nucleo::Fase::Gas),
        make_sp(15, "Carbono (grafito)", "C", 12.011, 0.0, 5.740, 8.5170, 0.0102, 0.0, -4.259e-10, nucleo::Fase::Solido),
        make_sp(16, "Azufre (s)", "S", 32.065, 0.0, 32.050, 22.6000, 0.0040, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(17, "Hierro", "Fe", 55.845, 0.0, 27.280, 24.0000, 0.0085000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(18, "Cobre", "Cu", 63.546, 0.0, 33.150, 24.4400, 0.0063, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(19, "Aluminio", "Al", 26.982, 0.0, 28.330, 24.2000, 0.0080, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(20, "Zinc", "Zn", 65.380, 0.0, 41.630, 25.4000, 0.0084, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(21, "Plomo", "Pb", 207.200, 0.0, 64.800, 26.4400, 0.0075, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(22, "Níquel", "Ni", 58.693, 0.0, 29.870, 26.0700, 0.0060, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(23, "Magnesio", "Mg", 24.305, 0.0, 32.680, 24.9000, 0.0090, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(24, "Calcio", "Ca", 40.078, 0.0, 41.590, 25.9000, 0.0095, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(25, "Silicio", "Si", 28.085, 0.0, 18.810, 20.0000, 0.0045, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(26, "Titanio", "Ti", 47.867, 0.0, 30.630, 25.0600, 0.0072, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(27, "Cromo", "Cr", 51.996, 0.0, 23.770, 23.4000, 0.0060, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(28, "Manganeso", "Mn", 54.938, 0.0, 32.010, 26.3000, 0.0070, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(29, "Molibdeno", "Mo", 95.950, 0.0, 28.660, 23.8000, 0.0050, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(30, "Óxido de calcio", "CaO", 56.077, -634.920, 38.100, 42.1010, 0.0014583, -7.746e-7, 9.502e-10, nucleo::Fase::Solido),
        make_sp(31, "Óxido de magnesio", "MgO", 40.304, -601.600, 26.950, 42.2600, 0.0073800, 0.0, -6.2e-9, nucleo::Fase::Solido),
        make_sp(32, "Óxido de aluminio", "Al2O3", 101.960, -1675.700, 50.920, 108.158, 0.0014244, -1.8184e-6, 2.9068e-9, nucleo::Fase::Solido),
        make_sp(33, "Dióxido de titanio", "TiO2", 79.866, -944.000, 50.620, 62.8600, 0.0113600, 0.0, -9.7e-9, nucleo::Fase::Solido),
        make_sp(34, "Dióxido de silicio", "SiO2", 60.084, -910.700, 41.460, 60.0090, 0.0059407, -1.2207e-6, 2.417e-10, nucleo::Fase::Solido),
        make_sp(35, "Óxido de manganeso(II)", "MnO", 70.937, -385.200, 59.710, 46.4800, 0.0081200, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(36, "Óxido de cromo(III)", "Cr2O3", 151.990, -1139.700, 81.200, 119.370, 0.0092000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(37, "Óxido de zinc", "ZnO", 81.380, -350.500, 43.650, 40.2300, 0.0100600, 0.0, -4.6e-9, nucleo::Fase::Solido),
        make_sp(38, "Óxido de hierro(II)", "FeO", 71.844, -272.000, 60.750, 48.9900, 0.0083670, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(39, "Óxido de hierro(II,III)", "Fe3O4", 231.533, -1118.400, 146.400, 143.429, 0.0209900, -6.218e-6, 0.0, nucleo::Fase::Solido),
        make_sp(40, "Óxido de hierro(III)", "Fe2O3", 159.688, -824.200, 87.400, 97.7360, 0.0072887, -1.5069e-6, 1.4935e-9, nucleo::Fase::Solido),
        make_sp(41, "Óxido de níquel(II)", "NiO", 74.693, -239.700, 38.000, 47.0000, 0.0095000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(42, "Óxido de plomo(II)", "PbO", 223.200, -219.000, 66.500, 44.6800, 0.0178500, 0.0, -8.2e-9, nucleo::Fase::Solido),
        make_sp(43, "Óxido de cobre(I)", "Cu2O", 143.090, -168.600, 93.100, 65.0000, 0.0150000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(44, "Óxido de cobre(II)", "CuO", 79.545, -157.300, 42.630, 43.8300, 0.0107200, 0.0, -7.3e-9, nucleo::Fase::Solido),
        make_sp(45, "Calcosina", "Cu2S", 159.160, -79.500, 120.900, 75.0000, 0.0200000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(46, "Troilita", "FeS", 87.910, -100.000, 60.290, 51.6000, 0.0230000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(47, "Pirita", "FeS2", 119.975, -171.500, 52.900, 62.0000, 0.0300000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(48, "Esfalerita", "ZnS", 97.460, -206.000, 57.700, 48.0000, 0.0150000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(49, "Galena", "PbS", 239.270, -100.400, 91.200, 48.0000, 0.0120000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(50, "Molibdenita", "MoS2", 160.070, -275.000, 62.600, 65.0000, 0.0200000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(51, "Cloruro de calcio", "CaCl2", 110.984, -795.800, 108.400, 72.5900, 0.0119000, 0.0, -7.3e-9, nucleo::Fase::Solido),
        make_sp(52, "Cloruro de magnesio", "MgCl2", 95.211, -641.300, 89.600, 71.0900, 0.0141400, 0.0, -8.8e-9, nucleo::Fase::Solido),
        make_sp(53, "Cloruro de aluminio", "AlCl3", 133.341, -704.200, 109.290, 91.0000, 0.0120000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(54, "Tetracloruro de titanio", "TiCl4", 189.679, -763.200, 353.200, 106.200, 0.0015000, 0.0, 0.0, nucleo::Fase::Gas),
        make_sp(55, "Cloruro de zinc", "ZnCl2", 136.286, -415.050, 111.460, 68.5000, 0.0160000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(56, "Cloruro de hierro(II)", "FeCl2", 126.751, -341.790, 117.950, 75.2000, 0.0120000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(57, "Cloruro de plomo(II)", "PbCl2", 278.106, -359.410, 136.000, 72.8000, 0.0180000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(58, "Carbonato de calcio", "CaCO3", 100.086, -1207.600, 91.700, 82.3400, 0.0497500, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(59, "Carbonato de magnesio", "MgCO3", 84.314, -1095.800, 65.700, 75.5000, 0.0420000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(60, "Siderita", "FeCO3", 115.854, -740.600, 92.900, 82.0000, 0.0250000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(61, "Smithsonita", "ZnCO3", 125.389, -812.780, 82.400, 79.7000, 0.0350000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(62, "Sulfato de calcio", "CaSO4", 136.141, -1434.100, 106.700, 99.7000, 0.0350000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(63, "Sulfato de magnesio", "MgSO4", 120.366, -1284.900, 91.600, 96.0000, 0.0340000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(64, "Sulfato de zinc", "ZnSO4", 161.470, -982.800, 110.500, 99.0000, 0.0320000, 0.0, 0.0, nucleo::Fase::Solido),
        make_sp(65, "Sulfato de cobre(II)", "CuSO4", 159.609, -771.360, 109.200, 98.8000, 0.0320000, 0.0, 0.0, nucleo::Fase::Solido),
    };
    return cat;
}

std::string format_phase_label(const std::string& clean_f, nucleo::Fase f) {
    if (f == nucleo::Fase::Solido) {
        return clean_f + "(s)";
    }
    if (f == nucleo::Fase::Liquido) {
        return clean_f + "(l)";
    }
    if (f == nucleo::Fase::Gas) {
        return clean_f + "(g)";
    }
    return clean_f;
}

// Resuelve la lista de polimorfos candidatos para una especie desde el repositorio + respaldo curado
std::vector<nucleo::Especie> resolver_polimorfos(const domain::ThermoRepository& repo,
                                                 const std::string& formula_o_nombre,
                                                 bool usar_envolvente_fases) {
    std::string phase_tag;
    std::string clean = clean_formula_str(formula_o_nombre, &phase_tag);
    std::string raw_trim = trim_str(formula_o_nombre);

    const auto& mapa = repo.mapa_especies();
    const bool repo_pequeno = mapa.size() < 50;

    std::vector<nucleo::Especie> halladas;
    for (const auto& [id, esp] : mapa) {
        if (!repo_pequeno && !esp.es_curada()) {
            continue;
        }
        if (es_especie_acuosa(esp)) {
            continue;
        }
        if (!repo_pequeno && std::abs(esp.cp_a) < 1e-6 && std::abs(esp.cp_b) < 1e-9) {
            continue;
        }
        std::string esp_tag;
        std::string esp_clean = clean_formula_str(esp.formula, &esp_tag);
        if (esp.formula == raw_trim || esp.nombre == raw_trim || esp_clean == clean) {
            // Si el usuario pidio explicitamente H2O(g) o H2O(l)
            if (clean == "H2O" && phase_tag == "g" && esp.fase_std != nucleo::Fase::Gas) {
                continue;
            }
            if (clean == "H2O" && phase_tag == "l" && esp.fase_std != nucleo::Fase::Liquido) {
                continue;
            }
            if (!usar_envolvente_fases && !phase_tag.empty()) {
                if (phase_tag == "g" && esp.fase_std != nucleo::Fase::Gas) {
                    continue;
                }
                if (phase_tag == "l" && esp.fase_std != nucleo::Fase::Liquido) {
                    continue;
                }
                if ((phase_tag == "s" || phase_tag == "cr") && esp.fase_std != nucleo::Fase::Solido) {
                    continue;
                }
            }
            halladas.push_back(esp);
        }
    }

    if (halladas.empty()) {
        for (const auto& esp : catalogo_respaldo()) {
            std::string esp_tag;
            std::string esp_clean = clean_formula_str(esp.formula, &esp_tag);
            if (esp.formula == raw_trim || esp.nombre == raw_trim || esp_clean == clean) {
                if (clean == "H2O" && phase_tag == "g" && esp.fase_std != nucleo::Fase::Gas) {
                    continue;
                }
                if (clean == "H2O" && phase_tag == "l" && esp.fase_std != nucleo::Fase::Liquido) {
                    continue;
                }
                halladas.push_back(esp);
            }
        }
    }

    if (halladas.empty()) {
        return halladas;
    }

    // Si no se usa envolvente de fases, mantener solo el polimorfo mas estable a 298.15 K
    if (!usar_envolvente_fases || (clean == "H2O" && phase_tag == "g")) {
        auto best_it = std::min_element(halladas.begin(), halladas.end(), [](const auto& a, const auto& b) {
            return a.energia_gibbs(kTref) < b.energia_gibbs(kTref);
        });
        nucleo::Especie unico = *best_it;
        unico.formula = format_phase_label(clean, unico.fase_std);
        return {unico};
    }

    // Encontrar la fase solida (o base a 298.15 K) y generar polimorfos de fusion/ebullicion si aplican
    auto base_it = std::min_element(halladas.begin(), halladas.end(), [](const auto& a, const auto& b) {
        return a.energia_gibbs(kTref) < b.energia_gibbs(kTref);
    });
    nucleo::Especie base_sp = *base_it;
    base_sp.formula = format_phase_label(clean, base_sp.fase_std);

    std::vector<nucleo::Especie> envolvente;
    envolvente.push_back(base_sp);

    for (const auto& tr : tabla_transiciones()) {
        if (clean == tr.clean_sym && base_sp.fase_std == nucleo::Fase::Solido) {
            if (tr.T_m > 0.0 && tr.dH_m > 0.0) {
                nucleo::Especie liq = make_transition_polymorph(
                    base_sp, clean + "(l)", base_sp.nombre + " (l)", nucleo::Fase::Liquido,
                    tr.T_m, tr.dH_m, tr.cp_l);
                envolvente.push_back(liq);
                if (tr.T_b > tr.T_m && tr.dH_v > 0.0) {
                    nucleo::Especie gas = make_transition_polymorph(
                        liq, clean + "(g)", base_sp.nombre + " (g)", nucleo::Fase::Gas,
                        tr.T_b, tr.dH_v, tr.cp_g);
                    envolvente.push_back(gas);
                }
            }
            return envolvente;
        }
    }

    // Caso H2O sin sufijo (g): envolvente H2O(l) -> H2O(g) a 373.15 K
    if (clean == "H2O" && phase_tag != "g") {
        for (const auto& sp : halladas) {
            if (sp.fase_std == nucleo::Fase::Gas) {
                nucleo::Especie g = sp;
                g.formula = "H2O(g)";
                envolvente.push_back(g);
                break;
            }
        }
    }

    return envolvente;
}

const nucleo::Especie& seleccionar_polimorfo_activo(const std::vector<nucleo::Especie>& polimorfos,
                                                    double T_K) {
    auto it = std::min_element(polimorfos.begin(), polimorfos.end(), [T_K](const auto& a, const auto& b) {
        return a.energia_gibbs(T_K) < b.energia_gibbs(T_K);
    });
    return *it;
}

double refinar_temperatura_transicion(const nucleo::Especie& a, const nucleo::Especie& b,
                                      double T_low, double T_high) {
    double lo = T_low;
    double hi = T_high;
    double d_lo = a.energia_gibbs(lo) - b.energia_gibbs(lo);
    for (int iter = 0; iter < 32; ++iter) {
        double mid = 0.5 * (lo + hi);
        double d_mid = a.energia_gibbs(mid) - b.energia_gibbs(mid);
        if ((d_lo <= 0.0 && d_mid <= 0.0) || (d_lo >= 0.0 && d_mid >= 0.0)) {
            lo = mid;
            d_lo = d_mid;
        } else {
            hi = mid;
        }
    }
    return 0.5 * (lo + hi);
}

// Evalua H°(T), S°(T), G°(T) por 1 mol de atomos de un elemento en su estado estandar de referencia a T
void evaluar_elemento_referencia(const domain::ThermoRepository& repo, const std::string& elem_sym,
                                 double T_K, double& H_out, double& S_out, double& G_out) {
    if (elem_sym == "O" || elem_sym == "H" || elem_sym == "N" || elem_sym == "Cl" || elem_sym == "F") {
        std::string gas_f = elem_sym + "2";
        auto polys = resolver_polimorfos(repo, gas_f, false);
        if (!polys.empty()) {
            const auto& sp = polys.front();
            H_out = 0.5 * sp.entalpia_abs(T_K);
            S_out = 0.5 * sp.entropia_abs(T_K);
            G_out = 0.5 * sp.energia_gibbs(T_K);
            return;
        }
    }
    if (elem_sym == "S") {
        auto s_sol = resolver_polimorfos(repo, "S", false);
        auto s2_gas = resolver_polimorfos(repo, "S2", false);
        if (!s_sol.empty() && !s2_gas.empty()) {
            const double g_s = s_sol.front().energia_gibbs(T_K);
            const double g_s2 = 0.5 * s2_gas.front().energia_gibbs(T_K);
            if (T_K >= 717.8 || g_s2 < g_s) {
                H_out = 0.5 * s2_gas.front().entalpia_abs(T_K);
                S_out = 0.5 * s2_gas.front().entropia_abs(T_K);
                G_out = g_s2;
            } else {
                H_out = s_sol.front().entalpia_abs(T_K);
                S_out = s_sol.front().entropia_abs(T_K);
                G_out = g_s;
            }
            return;
        }
    }
    auto polys = resolver_polimorfos(repo, elem_sym, true);
    if (!polys.empty()) {
        const auto& act = seleccionar_polimorfo_activo(polys, T_K);
        H_out = act.entalpia_abs(T_K);
        S_out = act.entropia_abs(T_K);
        G_out = act.energia_gibbs(T_K);
        return;
    }
    H_out = 0.0;
    S_out = 0.0;
    G_out = 0.0;
}

bool es_elemento_referencia_puro(const std::string& clean_f) {
    static const std::vector<std::string> refs = {
        "O2", "H2", "N2", "Cl2", "F2", "S", "C", "Fe", "Cu", "Zn", "Al", "Si",
        "Mg", "Ca", "Pb", "Ni", "Ti", "Cr", "Mn", "Mo", "Co", "V", "W", "Ag",
        "Au", "Pt", "Na", "K", "Sn", "He", "Ne", "Ar"
    };
    return std::find(refs.begin(), refs.end(), clean_f) != refs.end();
}

std::string format_coef(double c) {
    double ac = std::abs(c);
    if (std::abs(ac - 1.0) < 1e-6) {
        return "";
    }
    // Fracciones metalurgicas comunes (4/3, 2/3, 3/2, 1/2, 1/3, 1/4, 3/4)
    static const std::pair<int, int> fracs[] = {
        {1, 2}, {3, 2}, {1, 3}, {2, 3}, {4, 3}, {1, 4}, {3, 4}
    };
    for (const auto& [num, den] : fracs) {
        if (std::abs(ac - static_cast<double>(num) / static_cast<double>(den)) < 1e-4) {
            return std::to_string(num) + "/" + std::to_string(den) + " ";
        }
    }
    if (std::abs(ac - std::round(ac)) < 1e-5) {
        return std::to_string(static_cast<int>(std::round(ac))) + " ";
    }
    std::ostringstream os;
    os << std::fixed << std::setprecision(3) << ac << " ";
    return os.str();
}

std::string formatear_ecuacion_normalizada(const std::vector<TerminoReaccionDiagrama>& terminos) {
    std::ostringstream lhs;
    std::ostringstream rhs;
    bool first_l = true;
    bool first_r = true;
    for (const auto& t : terminos) {
        if (t.coef < -1e-12) {
            if (!first_l) {
                lhs << " + ";
            }
            lhs << format_coef(t.coef) << t.formula;
            first_l = false;
        } else if (t.coef > 1e-12) {
            if (!first_r) {
                rhs << " + ";
            }
            rhs << format_coef(t.coef) << t.formula;
            first_r = false;
        }
    }
    return lhs.str() + " = " + rhs.str();
}

// Parsea un termino individual como "4/3 Al", "2 FeO", "O2(g)", "0.5 Fe3O4"
bool parse_termino_quimico(const std::string& raw_term, double sign, TerminoReaccionDiagrama& out) {
    std::string s = trim_str(raw_term);
    if (s.empty()) {
        return false;
    }
    size_t idx = 0;
    double coef = 1.0;
    if (std::isdigit(static_cast<unsigned char>(s[0])) != 0 || s[0] == '.') {
        size_t num_end = 0;
        double num = std::stod(s, &num_end);
        idx = num_end;
        while (idx < s.size() && s[idx] == ' ') {
            ++idx;
        }
        if (idx < s.size() && s[idx] == '/') {
            ++idx;
            while (idx < s.size() && s[idx] == ' ') {
                ++idx;
            }
            size_t den_end = 0;
            double den = std::stod(s.substr(idx), &den_end);
            if (std::abs(den) > 1e-12) {
                coef = num / den;
            }
            idx += den_end;
        } else {
            coef = num;
        }
    }
    std::string formula = trim_str(s.substr(idx));
    if (formula.empty()) {
        return false;
    }
    out.formula = formula;
    out.coef = sign * coef;
    return true;
}

std::vector<std::string> split_reaction_side(const std::string& side) {
    std::vector<std::string> parts;
    std::string current;
    int paren_depth = 0;
    for (size_t i = 0; i < side.size(); ++i) {
        char c = side[i];
        if (c == '(' || c == '[') {
            ++paren_depth;
        } else if (c == ')' || c == ']') {
            paren_depth = std::max(0, paren_depth - 1);
        }
        if (c == '+' && paren_depth == 0) {
            parts.push_back(current);
            current.clear();
        } else {
            current.push_back(c);
        }
    }
    if (!current.empty()) {
        parts.push_back(current);
    }
    return parts;
}

std::vector<TerminoReaccionDiagrama> parsear_ecuacion_quimica(const std::string& eq) {
    std::vector<TerminoReaccionDiagrama> terminos;
    std::string lhs_str;
    std::string rhs_str;

    static const char* delimiters[] = {"<=>", "<->", "->", "=", "→", "⇌"};
    size_t split_pos = std::string::npos;
    size_t delim_len = 0;
    for (const char* d : delimiters) {
        size_t p = eq.find(d);
        if (p != std::string::npos) {
            split_pos = p;
            delim_len = std::char_traits<char>::length(d);
            break;
        }
    }
    if (split_pos == std::string::npos) {
        return terminos;
    }
    lhs_str = eq.substr(0, split_pos);
    rhs_str = eq.substr(split_pos + delim_len);

    for (const auto& part : split_reaction_side(lhs_str)) {
        TerminoReaccionDiagrama t;
        if (parse_termino_quimico(part, -1.0, t)) {
            terminos.push_back(t);
        }
    }
    for (const auto& part : split_reaction_side(rhs_str)) {
        TerminoReaccionDiagrama t;
        if (parse_termino_quimico(part, +1.0, t)) {
            terminos.push_back(t);
        }
    }
    return terminos;
}

std::string gas_objetivo_para_base(domain::EllinghamNormalizationBasis basis) {
    switch (basis) {
    case domain::EllinghamNormalizationBasis::PerMoleO2:
        return "O2";
    case domain::EllinghamNormalizationBasis::PerMoleS2:
        return "S2";
    case domain::EllinghamNormalizationBasis::PerMoleCl2:
        return "Cl2";
    case domain::EllinghamNormalizationBasis::PerMoleCO2:
        return "CO2";
    case domain::EllinghamNormalizationBasis::PerMoleSO3:
        return "SO3";
    case domain::EllinghamNormalizationBasis::PerMoleH2O:
        return "H2O";
    case domain::EllinghamNormalizationBasis::PerMoleReaction:
        return "";
    }
    return "";
}

// Evalua dG°(T) de 2 CO(g) + O2(g) = 2 CO2(g) y 2 H2(g) + O2(g) = 2 H2O(g)
void evaluar_reacciones_escala_gases(const domain::ThermoRepository& repo, double T_K,
                                     double& dG_CO_CO2_kJ, double& dG_H2_H2O_kJ) {
    auto o2 = resolver_polimorfos(repo, "O2(g)", false);
    auto co = resolver_polimorfos(repo, "CO(g)", false);
    auto co2 = resolver_polimorfos(repo, "CO2(g)", false);
    auto h2 = resolver_polimorfos(repo, "H2(g)", false);
    auto h2o = resolver_polimorfos(repo, "H2O(g)", false);

    if (!o2.empty() && !co.empty() && !co2.empty()) {
        dG_CO_CO2_kJ = 2.0 * co2.front().energia_gibbs(T_K) -
                       2.0 * co.front().energia_gibbs(T_K) -
                       o2.front().energia_gibbs(T_K);
    } else {
        dG_CO_CO2_kJ = -565.0 + 0.173 * T_K;
    }

    if (!o2.empty() && !h2.empty() && !h2o.empty()) {
        dG_H2_H2O_kJ = 2.0 * h2o.front().energia_gibbs(T_K) -
                       2.0 * h2.front().energia_gibbs(T_K) -
                       o2.front().energia_gibbs(T_K);
    } else {
        dG_H2_H2O_kJ = -492.0 + 0.110 * T_K;
    }
}

} // namespace

std::vector<PuntoPropiedad> PropertyDiagrams::generarCurvas(double h_ref, double s_ref, double cp_a,
                                                            double cp_b, double cp_c, double cp_d,
                                                            double T_min, double T_max) {
    std::vector<PuntoPropiedad> out;
    const int steps = 100;
    const double dt = (T_max - T_min) / static_cast<double>(steps);

    nucleo::Especie sp;
    sp.h_formacion = h_ref;
    sp.s_formacion = s_ref;
    sp.cp_a = cp_a;
    sp.cp_b = cp_b;
    sp.cp_c = cp_c;
    sp.cp_d = cp_d;
    if (std::abs(cp_c) > 1.0) sp.modelo_cp = nucleo::ModeloCp::MAIER_KELLEY;

    out.reserve(static_cast<size_t>(steps) + 1);
    for (int i = 0; i <= steps; ++i) {
        const double T = std::max(T_min + static_cast<double>(i) * dt, 1e-10);
        const double cp = sp.cp(T);
        const double H = sp.entalpia_abs(T);
        const double S = sp.entropia_abs(T);
        const double G = sp.energia_gibbs(T);
        out.push_back({T, H, S, cp, G});
    }
    return out;
}

domain::CurvaDiagramaTermo PropertyDiagrams::calcular_curva_especie(
    const domain::ThermoRepository& repo,
    const std::string& formula_o_nombre,
    double T_min_K,
    double T_max_K,
    int steps,
    bool usar_envolvente_fases,
    double multiplicador) const {
    domain::CurvaDiagramaTermo curva;
    curva.label = trim_str(formula_o_nombre);
    curva.equation_normalized = curva.label;
    curva.norm_factor = multiplicador;

    auto polimorfos = resolver_polimorfos(repo, formula_o_nombre, usar_envolvente_fases);
    if (polimorfos.empty()) {
        return curva;
    }

    std::string phase_tag;
    const std::string clean_f = clean_formula_str(formula_o_nombre, &phase_tag);
    const auto atomos = nucleo::TablaPeriodica::instancia().analizar_formula(clean_f);
    const bool es_elem_puro = es_elemento_referencia_puro(clean_f) &&
                              (usar_envolvente_fases || phase_tag.empty() || phase_tag == "s" ||
                               (clean_f == "O2" || clean_f == "H2" || clean_f == "N2" ||
                                clean_f == "Cl2" || clean_f == "F2"));

    const int n_steps = std::max(2, steps);
    const double t_lo = std::max(50.0, std::min(T_min_K, T_max_K));
    const double t_hi = std::max(t_lo + 1.0, std::max(T_min_K, T_max_K));
    const double dt = (t_hi - t_lo) / static_cast<double>(n_steps);

    const auto& ref_298 = seleccionar_polimorfo_activo(polimorfos, kTref);
    const double H_298 = ref_298.entalpia_abs(kTref);

    curva.points.reserve(static_cast<size_t>(n_steps) + 1);
    const nucleo::Especie* prev_sp = nullptr;
    double prev_T = t_lo;

    for (int i = 0; i <= n_steps; ++i) {
        const double T = (i == 0) ? t_lo : (t_lo + static_cast<double>(i) * dt);
        const auto& act = seleccionar_polimorfo_activo(polimorfos, T);

        if (prev_sp != nullptr && prev_sp->formula != act.formula) {
            const double T_tr = refinar_temperatura_transicion(*prev_sp, act, prev_T, T);
            domain::TransicionFasePunto tp;
            tp.T_K = T_tr;
            tp.delta_H_kJ = (act.entalpia_abs(T_tr) - prev_sp->entalpia_abs(T_tr)) * multiplicador;
            tp.delta_S_J_K = (act.entropia_abs(T_tr) - prev_sp->entropia_abs(T_tr)) * multiplicador;
            tp.from_phase = prev_sp->formula;
            tp.to_phase = act.formula;
            curva.phase_transitions.push_back(tp);
        }
        prev_sp = &act;
        prev_T = T;

        domain::PuntoPropiedadTermo pt;
        pt.T = T;
        pt.H = act.entalpia_abs(T) * multiplicador;
        pt.dH298 = (act.entalpia_abs(T) - H_298) * multiplicador;
        pt.S = act.entropia_abs(T) * multiplicador;
        pt.Cp = act.cp(T) * multiplicador;
        pt.G = act.energia_gibbs(T) * multiplicador;
        pt.active_phase = act.formula;

        if (es_elem_puro) {
            pt.dHf = 0.0;
            pt.dSf = 0.0;
            pt.dGf = 0.0;
            pt.log10K = 0.0;
        } else {
            double H_elems = 0.0;
            double S_elems = 0.0;
            double G_elems = 0.0;
            for (const auto& a : atomos) {
                double h_e = 0.0;
                double s_e = 0.0;
                double g_e = 0.0;
                evaluar_elemento_referencia(repo, a.elem, T, h_e, s_e, g_e);
                H_elems += static_cast<double>(a.cuenta) * h_e;
                S_elems += static_cast<double>(a.cuenta) * s_e;
                G_elems += static_cast<double>(a.cuenta) * g_e;
            }
            pt.dHf = (act.entalpia_abs(T) - H_elems) * multiplicador;
            pt.dSf = (act.entropia_abs(T) - S_elems) * multiplicador;
            pt.dGf = (act.energia_gibbs(T) - G_elems) * multiplicador;
            if (std::abs(pt.dGf) < 1e-11) {
                pt.dGf = 0.0;
            }
            pt.log10K = -pt.dGf * 1000.0 / (kR_J * T * kLn10);
        }
        curva.points.push_back(pt);
    }

    return curva;
}

domain::CurvaDiagramaTermo PropertyDiagrams::calcular_curva_reaccion(
    const domain::ThermoRepository& repo,
    const std::string& etiqueta,
    const std::vector<TerminoReaccionDiagrama>& terminos,
    domain::EllinghamNormalizationBasis basis,
    double T_min_K,
    double T_max_K,
    int steps,
    bool usar_envolvente_fases) const {
    domain::CurvaDiagramaTermo curva;
    curva.label = etiqueta;
    if (terminos.empty()) {
        return curva;
    }

    std::vector<TerminoReaccionDiagrama> norm_terms = terminos;
    double norm_factor = 1.0;
    const std::string target_gas = gas_objetivo_para_base(basis);

    if (!target_gas.empty()) {
        double nu_gas = 0.0;
        for (const auto& t : terminos) {
            if (clean_formula_str(t.formula) == target_gas) {
                nu_gas += t.coef;
            }
        }
        if (std::abs(nu_gas) > 1e-12) {
            // Convencion Ellingham: el gas reactivo (O2, S2, Cl2, CO2, SO3, H2O) queda con nu = -1
            norm_factor = -1.0 / nu_gas;
            for (auto& t : norm_terms) {
                t.coef *= norm_factor;
            }
        }
    }

    curva.norm_factor = std::abs(norm_factor);
    curva.equation_normalized = formatear_ecuacion_normalizada(norm_terms);
    if (curva.label.empty()) {
        curva.label = curva.equation_normalized;
    }

    struct TermResolved {
        TerminoReaccionDiagrama term;
        std::vector<nucleo::Especie> polys;
        double H_298 = 0.0;
        const nucleo::Especie* prev_sp = nullptr;
    };

    std::vector<TermResolved> resolved;
    resolved.reserve(norm_terms.size());
    for (const auto& t : norm_terms) {
        auto polys = resolver_polimorfos(repo, t.formula, usar_envolvente_fases);
        if (polys.empty()) {
            return domain::CurvaDiagramaTermo{};
        }
        const auto& sp298 = seleccionar_polimorfo_activo(polys, kTref);
        resolved.push_back({t, std::move(polys), sp298.entalpia_abs(kTref), nullptr});
    }

    const int n_steps = std::max(2, steps);
    const double t_lo = std::max(50.0, std::min(T_min_K, T_max_K));
    const double t_hi = std::max(t_lo + 1.0, std::max(T_min_K, T_max_K));
    const double dt = (t_hi - t_lo) / static_cast<double>(n_steps);

    curva.points.reserve(static_cast<size_t>(n_steps) + 1);
    double prev_T = t_lo;

    for (int i = 0; i <= n_steps; ++i) {
        const double T = (i == 0) ? t_lo : (t_lo + static_cast<double>(i) * dt);
        double dH = 0.0;
        double dH298 = 0.0;
        double dS = 0.0;
        double dCp = 0.0;
        double dG = 0.0;
        std::string active_desc;

        for (auto& r : resolved) {
            const auto& act = seleccionar_polimorfo_activo(r.polys, T);
            if (r.prev_sp != nullptr && r.prev_sp->formula != act.formula) {
                const double T_tr = refinar_temperatura_transicion(*r.prev_sp, act, prev_T, T);
                domain::TransicionFasePunto tp;
                tp.T_K = T_tr;
                tp.delta_H_kJ = std::abs(r.term.coef) *
                                (act.entalpia_abs(T_tr) - r.prev_sp->entalpia_abs(T_tr));
                tp.delta_S_J_K = std::abs(r.term.coef) *
                                 (act.entropia_abs(T_tr) - r.prev_sp->entropia_abs(T_tr));
                tp.from_phase = r.prev_sp->formula;
                tp.to_phase = act.formula;
                curva.phase_transitions.push_back(tp);
            }
            r.prev_sp = &act;

            dH += r.term.coef * act.entalpia_abs(T);
            dH298 += r.term.coef * (act.entalpia_abs(T) - r.H_298);
            dS += r.term.coef * act.entropia_abs(T);
            dCp += r.term.coef * act.cp(T);
            dG += r.term.coef * act.energia_gibbs(T);
            if (!active_desc.empty()) {
                active_desc += ", ";
            }
            active_desc += act.formula;
        }
        prev_T = T;

        domain::PuntoPropiedadTermo pt;
        pt.T = T;
        pt.H = dH;
        pt.dH298 = dH298;
        pt.S = dS;
        pt.Cp = dCp;
        pt.G = dG;
        pt.dGf = dG;
        pt.dHf = dH;
        pt.dSf = dS;
        pt.log10K = -dG * 1000.0 / (kR_J * T * kLn10);
        pt.active_phase = active_desc;
        curva.points.push_back(pt);
    }

    return curva;
}

domain::CurvaDiagramaTermo PropertyDiagrams::calcular_curva_ecuacion_texto(
    const domain::ThermoRepository& repo,
    const std::string& ecuacion_texto,
    domain::EllinghamNormalizationBasis basis,
    double T_min_K,
    double T_max_K,
    int steps,
    bool usar_envolvente_fases) const {
    auto terminos = parsear_ecuacion_quimica(ecuacion_texto);
    if (terminos.empty()) {
        return domain::CurvaDiagramaTermo{};
    }
    return calcular_curva_reaccion(repo, trim_str(ecuacion_texto), terminos, basis,
                                   T_min_K, T_max_K, steps, usar_envolvente_fases);
}

domain::ResultadoDiagramaPropiedades PropertyDiagrams::generar_familia_ellingham(
    const domain::ThermoRepository& repo,
    domain::EllinghamPresetFamily familia,
    double T_min_K,
    double T_max_K,
    int steps,
    bool usar_envolvente_fases) const {
    domain::ResultadoDiagramaPropiedades res;
    res.mode = domain::PropertyDiagramMode::EllinghamReactions;
    res.diagram_type = domain::PropertyDiagramType::DeltaG_Ellingham;
    res.T_min_K = T_min_K;
    res.T_max_K = T_max_K;

    std::vector<std::string> ecuaciones;
    switch (familia) {
    case domain::EllinghamPresetFamily::MetallurgicalOxides:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleO2;
        ecuaciones = {
            "2 Ca + O2(g) = 2 CaO",
            "2 Mg + O2(g) = 2 MgO",
            "4/3 Al + O2(g) = 2/3 Al2O3",
            "Ti + O2(g) = TiO2",
            "Si + O2(g) = SiO2",
            "2 Mn + O2(g) = 2 MnO",
            "4/3 Cr + O2(g) = 2/3 Cr2O3",
            "2 Zn + O2(g) = 2 ZnO",
            "2 Fe + O2(g) = 2 FeO",
            "3/2 Fe + O2(g) = 1/2 Fe3O4",
            "2 Ni + O2(g) = 2 NiO",
            "2 Pb + O2(g) = 2 PbO",
            "4 Cu + O2(g) = 2 Cu2O",
            "2 C + O2(g) = 2 CO(g)",
            "C + O2(g) = CO2(g)",
            "2 CO(g) + O2(g) = 2 CO2(g)",
            "2 H2(g) + O2(g) = 2 H2O(g)"
        };
        break;
    case domain::EllinghamPresetFamily::SulfideRoasting:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleS2;
        ecuaciones = {
            "4 Cu + S2(g) = 2 Cu2S",
            "2 Fe + S2(g) = 2 FeS",
            "2 Zn + S2(g) = 2 ZnS",
            "2 Pb + S2(g) = 2 PbS",
            "Mo + S2(g) = MoS2",
            "2 H2(g) + S2(g) = 2 H2S(g)",
            "S2(g) + 2 O2(g) = 2 SO2(g)"
        };
        break;
    case domain::EllinghamPresetFamily::ChlorinationMetallurgy:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleCl2;
        ecuaciones = {
            "Ca + Cl2(g) = CaCl2",
            "Mg + Cl2(g) = MgCl2",
            "2/3 Al + Cl2(g) = 2/3 AlCl3",
            "1/2 Ti + Cl2(g) = 1/2 TiCl4",
            "Zn + Cl2(g) = ZnCl2",
            "Fe + Cl2(g) = FeCl2",
            "Pb + Cl2(g) = PbCl2",
            "H2(g) + Cl2(g) = 2 HCl(g)"
        };
        break;
    case domain::EllinghamPresetFamily::CarbonateCalcination:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleCO2;
        ecuaciones = {
            "CaO + CO2(g) = CaCO3",
            "MgO + CO2(g) = MgCO3",
            "FeO + CO2(g) = FeCO3",
            "ZnO + CO2(g) = ZnCO3"
        };
        break;
    case domain::EllinghamPresetFamily::SulfateDecomposition:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleSO3;
        ecuaciones = {
            "CaO + SO3(g) = CaSO4",
            "MgO + SO3(g) = MgSO4",
            "ZnO + SO3(g) = ZnSO4",
            "CuO + SO3(g) = CuSO4"
        };
        break;
    }

    for (const auto& eq : ecuaciones) {
        auto c = calcular_curva_ecuacion_texto(repo, eq, res.basis, T_min_K, T_max_K,
                                               steps, usar_envolvente_fases);
        if (!c.points.empty()) {
            res.curves.push_back(std::move(c));
        }
    }

    res.intersections = encontrar_intersecciones(res.curves, res.diagram_type);
    if (res.basis == domain::EllinghamNormalizationBasis::PerMoleO2) {
        generar_lineas_isopotenciales(repo, T_min_K, T_max_K,
                                      res.iso_pO2_lines, res.iso_CO_CO2_lines, res.iso_H2_H2O_lines);
    }

    std::ostringstream log;
    log << "Diagrama Ellingham generado: " << res.curves.size() << " curvas, "
        << res.intersections.size() << " intersecciones en [" << std::fixed << std::setprecision(1)
        << T_min_K << " K, " << T_max_K << " K].";
    res.summary_log = log.str();
    return res;
}

std::vector<domain::InterseccionEllingham> PropertyDiagrams::encontrar_intersecciones(
    const std::vector<domain::CurvaDiagramaTermo>& curvas,
    domain::PropertyDiagramType tipo) const {
    std::vector<domain::InterseccionEllingham> out;
    if (curvas.size() < 2) {
        return out;
    }

    for (size_t a = 0; a < curvas.size(); ++a) {
        if (!curvas[a].visible || curvas[a].points.size() < 2) {
            continue;
        }
        for (size_t b = a + 1; b < curvas.size(); ++b) {
            if (!curvas[b].visible || curvas[b].points.size() != curvas[a].points.size()) {
                continue;
            }
            const size_t n = curvas[a].points.size();
            for (size_t i = 1; i < n; ++i) {
                const double ya0 = curvas[a].evaluar_propiedad(tipo, i - 1, false);
                const double ya1 = curvas[a].evaluar_propiedad(tipo, i, false);
                const double yb0 = curvas[b].evaluar_propiedad(tipo, i - 1, false);
                const double yb1 = curvas[b].evaluar_propiedad(tipo, i, false);
                const double d0 = ya0 - yb0;
                const double d1 = ya1 - yb1;

                if (((d0 <= 0.0 && d1 > 0.0) || (d0 >= 0.0 && d1 < 0.0)) &&
                    std::abs(d1 - d0) > 1e-12) {
                    const double alpha = -d0 / (d1 - d0);
                    const double t0 = curvas[a].points[i - 1].T;
                    const double t1 = curvas[a].points[i].T;
                    const double T_K = t0 + alpha * (t1 - t0);
                    const double val_Y = ya0 + alpha * (ya1 - ya0);
                    const double dG_a = curvas[a].points[i - 1].dGf +
                                        alpha * (curvas[a].points[i].dGf - curvas[a].points[i - 1].dGf);

                    domain::InterseccionEllingham inter;
                    inter.curve_a = curvas[a].label;
                    inter.curve_b = curvas[b].label;
                    inter.T_K = T_K;
                    inter.T_C = T_K - 273.15;
                    inter.value_Y = val_Y;
                    inter.log10_pO2 = (dG_a * 1000.0) / (kR_J * T_K * kLn10);

                    const double dG_CO_CO2 = -565.0 + 0.173 * T_K;
                    const double dG_H2_H2O = -492.0 + 0.110 * T_K;
                    inter.log10_CO_CO2 = ((dG_CO_CO2 - dG_a) * 1000.0) / (2.0 * kR_J * T_K * kLn10);
                    inter.log10_H2_H2O = ((dG_H2_H2O - dG_a) * 1000.0) / (2.0 * kR_J * T_K * kLn10);

                    const std::string& lower_above = (d1 < 0.0) ? curvas[a].label : curvas[b].label;
                    const std::string& higher_above = (d1 < 0.0) ? curvas[b].label : curvas[a].label;
                    std::ostringstream interp;
                    interp << "Por encima de T = " << std::fixed << std::setprecision(1) << T_K
                           << " K (" << inter.T_C << " °C), la curva [" << lower_above
                           << "] reduce termodinámicamente a [" << higher_above << "].";
                    inter.interpretation = interp.str();
                    out.push_back(std::move(inter));
                }
            }
        }
    }

    std::sort(out.begin(), out.end(), [](const auto& x, const auto& y) {
        return x.T_K < y.T_K;
    });
    return out;
}

domain::EscalaNomograficaPunto PropertyDiagrams::evaluar_escala_nomografica(
    const domain::ThermoRepository& repo,
    double T_K,
    double delta_G_kJ_per_O2) const {
    domain::EscalaNomograficaPunto pt;
    const double T = std::max(50.0, T_K);
    pt.T_K = T;
    pt.delta_G_kJ_per_O2 = delta_G_kJ_per_O2;

    const double rt_ln10_kJ = kR_kJ * T * kLn10;
    pt.log10_pO2 = delta_G_kJ_per_O2 / rt_ln10_kJ;
    pt.pO2_atm = std::pow(10.0, std::clamp(pt.log10_pO2, -100.0, 100.0));

    double dG_CO_CO2 = 0.0;
    double dG_H2_H2O = 0.0;
    evaluar_reacciones_escala_gases(repo, T, dG_CO_CO2, dG_H2_H2O);

    pt.log10_CO_CO2 = (dG_CO_CO2 - delta_G_kJ_per_O2) / (2.0 * rt_ln10_kJ);
    pt.ratio_CO_CO2 = std::pow(10.0, std::clamp(pt.log10_CO_CO2, -50.0, 50.0));

    pt.log10_H2_H2O = (dG_H2_H2O - delta_G_kJ_per_O2) / (2.0 * rt_ln10_kJ);
    pt.ratio_H2_H2O = std::pow(10.0, std::clamp(pt.log10_H2_H2O, -50.0, 50.0));
    return pt;
}

void PropertyDiagrams::generar_lineas_isopotenciales(
    const domain::ThermoRepository& repo,
    double T_min_K,
    double T_max_K,
    std::vector<domain::LineaIsoPotencial>& iso_pO2,
    std::vector<domain::LineaIsoPotencial>& iso_CO_CO2,
    std::vector<domain::LineaIsoPotencial>& iso_H2_H2O) const {
    iso_pO2.clear();
    iso_CO_CO2.clear();
    iso_H2_H2O.clear();

    const double t0 = std::max(50.0, std::min(T_min_K, T_max_K));
    const double t1 = std::max(t0 + 1.0, std::max(T_min_K, T_max_K));

    static const double log_pO2_vals[] = {-2, -4, -6, -8, -10, -12, -14, -16, -20, -24, -28};
    for (double lp : log_pO2_vals) {
        domain::LineaIsoPotencial line;
        std::ostringstream lbl;
        lbl << "log10(pO2) = " << static_cast<int>(lp);
        line.label = lbl.str();
        line.parameter_value = lp;
        line.T_K = {t0, t1};
        line.delta_G_kJ = {kR_kJ * t0 * kLn10 * lp, kR_kJ * t1 * kLn10 * lp};
        iso_pO2.push_back(std::move(line));
    }

    double dG_CO_0 = 0.0;
    double dG_H2_0 = 0.0;
    double dG_CO_1 = 0.0;
    double dG_H2_1 = 0.0;
    evaluar_reacciones_escala_gases(repo, t0, dG_CO_0, dG_H2_0);
    evaluar_reacciones_escala_gases(repo, t1, dG_CO_1, dG_H2_1);

    static const double log_ratio_vals[] = {-4, -3, -2, -1, 0, 1, 2, 3, 4};
    for (double lr : log_ratio_vals) {
        domain::LineaIsoPotencial l_co;
        std::ostringstream lbl_co;
        lbl_co << "log10(CO/CO2) = " << static_cast<int>(lr);
        l_co.label = lbl_co.str();
        l_co.parameter_value = lr;
        l_co.T_K = {t0, t1};
        l_co.delta_G_kJ = {dG_CO_0 - 2.0 * kR_kJ * t0 * kLn10 * lr,
                           dG_CO_1 - 2.0 * kR_kJ * t1 * kLn10 * lr};
        iso_CO_CO2.push_back(std::move(l_co));

        domain::LineaIsoPotencial l_h2;
        std::ostringstream lbl_h2;
        lbl_h2 << "log10(H2/H2O) = " << static_cast<int>(lr);
        l_h2.label = lbl_h2.str();
        l_h2.parameter_value = lr;
        l_h2.T_K = {t0, t1};
        l_h2.delta_G_kJ = {dG_H2_0 - 2.0 * kR_kJ * t0 * kLn10 * lr,
                           dG_H2_1 - 2.0 * kR_kJ * t1 * kLn10 * lr};
        iso_H2_H2O.push_back(std::move(l_h2));
    }
}

std::string PropertyDiagrams::exportar_csv(
    const std::vector<domain::CurvaDiagramaTermo>& curvas,
    const std::vector<domain::InterseccionEllingham>& intersecciones,
    domain::PropertyDiagramType tipo,
    bool temp_en_celsius,
    bool energia_en_kcal) const {
    std::ostringstream os;
    os << std::fixed << std::setprecision(4);
    os << "[CURVES]\n";
    os << (temp_en_celsius ? "T_C" : "T_K");
    for (const auto& c : curvas) {
        if (c.visible) {
            os << ",\"" << c.label << "\"";
        }
    }
    os << "\n";

    size_t max_pts = 0;
    for (const auto& c : curvas) {
        if (c.visible) {
            max_pts = std::max(max_pts, c.points.size());
        }
    }

    for (size_t i = 0; i < max_pts; ++i) {
        double T_val = 0.0;
        for (const auto& c : curvas) {
            if (c.visible && i < c.points.size()) {
                T_val = temp_en_celsius ? (c.points[i].T - 273.15) : c.points[i].T;
                break;
            }
        }
        os << T_val;
        for (const auto& c : curvas) {
            if (!c.visible) {
                continue;
            }
            if (i < c.points.size()) {
                os << "," << c.evaluar_propiedad(tipo, i, energia_en_kcal);
            } else {
                os << ",";
            }
        }
        os << "\n";
    }

    os << "\n[INTERSECTIONS]\n";
    os << "Curve_A,Curve_B,T_K,T_C,Value_Y,log10_pO2,log10_CO_CO2,log10_H2_H2O,Interpretation\n";
    const double k_faktor = (energia_en_kcal && tipo != domain::PropertyDiagramType::Log10K)
                                ? (1.0 / 4.184)
                                : 1.0;
    for (const auto& inter : intersecciones) {
        os << "\"" << inter.curve_a << "\",\"" << inter.curve_b << "\","
           << inter.T_K << "," << inter.T_C << "," << (inter.value_Y * k_faktor) << ","
           << inter.log10_pO2 << "," << inter.log10_CO_CO2 << "," << inter.log10_H2_H2O << ",\""
           << inter.interpretation << "\"\n";
    }
    return os.str();
}

} // namespace cma::application::modules
