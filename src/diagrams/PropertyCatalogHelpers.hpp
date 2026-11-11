#pragma once

#include "domain/PropertyDiagramTypes.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "diagrams/PropertyDiagrams.hpp"
#include "nucleo/tabla_periodica.hpp"
#include "nucleo/tipos.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <iomanip>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

namespace cma::application::modules::detail {

constexpr double kR_J = 8.314462618;
constexpr double kR_kJ = 0.008314462618;
constexpr double kLn10 = 2.302585092994046;
constexpr double kTref = 298.15;

inline std::string trim_str(const std::string& s) {
    size_t a = 0;
    while (a < s.size() && std::isspace(static_cast<unsigned char>(s[a])) != 0) ++a;
    size_t b = s.size();
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1])) != 0) --b;
    return s.substr(a, b - a);
}

inline std::string clean_formula_str(const std::string& raw, std::string* phase_tag_out = nullptr) {
    std::string s = trim_str(raw);
    if (phase_tag_out != nullptr) phase_tag_out->clear();
    auto pos = s.rfind('(');
    if (pos != std::string::npos && pos > 0 && s.back() == ')') {
        std::string inside = s.substr(pos + 1, s.size() - pos - 2);
        std::string low;
        for (char c : inside) low.push_back(static_cast<char>(std::tolower(static_cast<unsigned char>(c))));
        if (low == "s" || low == "l" || low == "g" || low == "cr" || low == "aq" || low == "a" ||
            inside == "α" || inside == "β" || inside == "γ" || low == "alpha" || low == "beta") {
            if (phase_tag_out != nullptr) *phase_tag_out = low;
            return trim_str(s.substr(0, pos));
        }
    }
    return s;
}

inline bool es_especie_acuosa(const nucleo::Especie& esp) {
    return (esp.formula.find('+') != std::string::npos || esp.formula.find('-') != std::string::npos ||
            esp.formula.find("⁺") != std::string::npos || esp.formula.find("⁻") != std::string::npos ||
            esp.formula.find("(aq)") != std::string::npos || esp.formula.find("(a)") != std::string::npos);
}

inline nucleo::Especie make_sp(int64_t id, const char* nombre, const char* formula, double M, double Hf,
                               double Sf, double cpa, double cpb, double cpc, double cpd,
                               nucleo::Fase fase, double tmin_K = 298.15, double tmax_K = 4000.0) {
    nucleo::Especie e;
    e.id = id; e.nombre = nombre; e.formula = formula; e.masa_molar = M;
    e.h_formacion = Hf; e.s_formacion = Sf;
    e.cp_a = cpa; e.cp_b = cpb; e.cp_c = cpc; e.cp_d = cpd; e.fase_std = fase;
    e.T_min = tmin_K; e.T_max = tmax_K;
    return e;
}

inline nucleo::Especie make_transition_polymorph(const nucleo::Especie& prev_phase, const std::string& new_formula,
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

inline const std::vector<TransSpec>& tabla_transiciones() {
    static const std::vector<TransSpec> tbl = {
        {"Zn",    692.68,  7.32,  31.38, 1180.15, 115.30, 20.786},
        {"Mg",    923.00,  8.48,  34.30, 1361.15, 127.40, 20.786},
        {"Ca",   1115.00,  8.54,  35.00, 1757.00, 154.70, 20.786},
        {"Pb",    600.61,  4.77,  30.63, 2022.00, 179.50, 20.786},
        {"Al",    933.47, 10.71,  31.75, 2792.00, 294.00, 20.786},
        {"Cu",   1357.77, 13.26,  32.84, 2835.00, 300.40, 20.786},
        {"Fe",   1811.00, 13.81,  46.02, 3134.00, 340.00, 20.786},
        {"Ni",   1728.00, 17.48,  43.10, 3186.00, 377.50, 20.786},
        {"Co",   1768.00, 16.20,  40.50, 3200.00, 375.00, 20.786},
        {"Si",   1687.00, 50.21,  27.20, 3538.00, 383.00, 20.786},
        {"Mn",   1519.00, 12.91,  46.00, 2334.00, 221.00, 20.786},
        {"Ti",   1941.00, 14.15,  47.20, 3560.00, 425.00, 20.786},
        {"Cr",   2180.00, 21.00,  39.30, 2944.00, 339.50, 20.786},
        {"Mo",   2896.00, 37.48,  42.00, 4912.00, 598.00, 20.786},
        {"Na",    370.95,  2.60,  31.50, 1156.00,  97.40, 20.786},
        {"K",     336.53,  2.33,  32.00, 1032.00,  76.90, 20.786},
        {"PbO",  1159.00, 25.50,  65.00,    0.00,   0.00,  0.000},
        {"FeO",  1650.00, 24.06,  68.20,    0.00,   0.00,  0.000},
        {"Cu2O", 1505.00, 64.80, 100.00,    0.00,   0.00,  0.000},
        {"Fe3O4", 848.00, 16.50, 200.80, 1870.15, 138.00, 213.40},
        {"FeCl2", 950.00, 43.00, 102.20, 1299.00, 126.00, 83.000},
        {"ZnCl2", 591.00, 10.25, 100.80, 1005.00, 119.00, 62.000},
        {"PbCl2", 774.00, 21.90, 111.50, 1224.00, 127.00, 80.000},
        {"MgCl2", 987.00, 43.10,  92.10, 1685.00, 156.00, 80.000},
        {"CaCl2",1045.00, 28.50, 103.00, 2208.00, 235.00, 82.000},
    };
    return tbl;
}

inline const std::vector<nucleo::Especie>& catalogo_respaldo() {
    static const std::vector<nucleo::Especie> cat = {
        make_sp(1, "Oxígeno", "O2", 31.999, 0.0, 205.147, 24.6979, 0.0167754, -8.151e-6, 1.4986e-9, nucleo::Fase::Gas, 100.0, 6000.0),
        make_sp(2, "Nitrógeno", "N2", 28.013, 0.0, 191.609, 27.3069, 0.0061500, -9.502e-7, 0.0, nucleo::Fase::Gas, 100.0, 6000.0),
        make_sp(3, "Hidrógeno", "H2", 2.016, 0.0, 130.680, 27.1415, 0.0092744, -1.3015e-6, 7.6313e-9, nucleo::Fase::Gas, 100.0, 6000.0),
        make_sp(4, "Cloro", "Cl2", 70.905, 0.0, 223.081, 33.0510, 0.0012229, 1.4466e-6, -1.4987e-9, nucleo::Fase::Gas, 100.0, 6000.0),
        make_sp(5, "Flúor", "F2", 37.997, 0.0, 202.790, 31.3000, 0.0041800, -1.200e-6, 1.200e-9, nucleo::Fase::Gas, 100.0, 6000.0),
        make_sp(6, "Azufre diatómico", "S2", 64.132, 128.600, 228.170, 32.4700, 0.0050000, 0.0, 0.0, nucleo::Fase::Gas, 298.15, 6000.0),
        make_sp(7, "Dióxido de carbono", "CO2", 44.010, -393.522, 213.795, 23.1089, 0.0577123, -3.3541e-5, 7.1345e-9, nucleo::Fase::Gas, 50.0, 10000.0),
        make_sp(8, "Monóxido de carbono", "CO", 28.010, -110.527, 197.660, 27.3579, 0.0037843, 3.741e-6, -1.8158e-9, nucleo::Fase::Gas, 200.0, 20000.0),
        make_sp(9, "Agua (vapor)", "H2O(g)", 18.015, -241.826, 188.835, 31.9500, 0.0028273, 1.0e-5, -3.3741e-9, nucleo::Fase::Gas, 298.15, 20000.0),
        make_sp(10, "Agua (líquida)", "H2O", 18.015, -285.830, 69.950, 75.1500, 0.0010800, 0.0, 0.0, nucleo::Fase::Liquido, 273.15, 500.0),
        make_sp(11, "Dióxido de azufre", "SO2", 64.064, -296.842, 248.223, 38.8310, 0.0016215, 2.0518e-6, -2.3557e-9, nucleo::Fase::Gas, 298.15, 6000.0),
        make_sp(12, "Trióxido de azufre", "SO3", 80.064, -395.765, 256.769, 48.5180, 0.0072450, 5.8552e-6, -9.2519e-9, nucleo::Fase::Gas, 298.15, 6000.0),
        make_sp(13, "Sulfuro de hidrógeno", "H2S", 34.080, -20.500, 205.800, 34.5350, 0.0012540, 2.9077e-6, -1.5287e-9, nucleo::Fase::Gas, 298.15, 6000.0),
        make_sp(14, "Cloruro de hidrógeno", "HCl", 36.461, -92.311, 186.900, 29.1330, -0.0000996, 1.95e-6, -1.0394e-9, nucleo::Fase::Gas, 298.15, 6000.0),
        make_sp(15, "Carbono (grafito)", "C", 12.011, 0.0, 5.740, 8.5170, 0.0102, 0.0, -4.259e-10, nucleo::Fase::Solido, 298.15, 6000.0),
        make_sp(16, "Azufre (s)", "S", 32.065, 0.0, 32.050, 22.6000, 0.0040, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 1000.0),
        make_sp(17, "Hierro", "Fe", 55.845, 0.0, 27.280, 24.0000, 0.0085000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 5000.0),
        make_sp(18, "Cobre", "Cu", 63.546, 0.0, 33.150, 24.4400, 0.0063, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(19, "Aluminio", "Al", 26.982, 0.0, 28.330, 24.2000, 0.0080, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(20, "Zinc", "Zn", 65.380, 0.0, 41.630, 25.4000, 0.0084, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(21, "Plomo", "Pb", 207.200, 0.0, 64.800, 26.4400, 0.0075, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(22, "Níquel", "Ni", 58.693, 0.0, 29.870, 26.0700, 0.0060, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(23, "Magnesio", "Mg", 24.305, 0.0, 32.680, 24.9000, 0.0090, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 5100.0),
        make_sp(24, "Calcio", "Ca", 40.078, 0.0, 41.590, 25.9000, 0.0095, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(25, "Silicio", "Si", 28.085, 0.0, 18.810, 20.0000, 0.0045, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(26, "Titanio", "Ti", 47.867, 0.0, 30.630, 25.0600, 0.0072, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(27, "Cromo", "Cr", 51.996, 0.0, 23.770, 23.4000, 0.0060, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4500.0),
        make_sp(28, "Manganeso", "Mn", 54.938, 0.0, 32.010, 26.3000, 0.0070, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(29, "Molibdeno", "Mo", 95.950, 0.0, 28.660, 23.8000, 0.0050, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 5000.0),
        make_sp(30, "Óxido de calcio", "CaO", 56.077, -634.920, 38.100, 42.1010, 0.0014583, -7.746e-7, 9.502e-10, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(31, "Óxido de magnesio", "MgO", 40.304, -601.600, 26.950, 42.2600, 0.0073800, 0.0, -6.2e-9, nucleo::Fase::Solido, 298.15, 5100.0),
        make_sp(32, "Óxido de aluminio", "Al2O3", 101.960, -1675.700, 50.920, 108.158, 0.0014244, -1.8184e-6, 2.9068e-9, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(33, "Dióxido de titanio", "TiO2", 79.866, -944.000, 50.620, 62.8600, 0.0113600, 0.0, -9.7e-9, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(34, "Dióxido de silicio", "SiO2", 60.084, -910.700, 41.460, 60.0090, 0.0059407, -1.2207e-6, 2.417e-10, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(35, "Óxido de manganeso(II)", "MnO", 70.937, -385.200, 59.710, 46.4800, 0.0081200, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(36, "Óxido de cromo(III)", "Cr2O3", 151.990, -1139.700, 81.200, 119.370, 0.0092000, 0.0, 0.0, nucleo::Fase::Solido, 100.0, 4500.0),
        make_sp(37, "Óxido de zinc", "ZnO", 81.380, -350.500, 43.650, 40.2300, 0.0100600, 0.0, -4.6e-9, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(38, "Óxido de hierro(II)", "FeO", 71.844, -272.000, 60.750, 48.9900, 0.0083670, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 5000.0),
        make_sp(39, "Óxido de hierro(II,III)", "Fe3O4", 231.533, -1118.400, 146.400, 143.429, 0.0209900, -6.218e-6, 0.0, nucleo::Fase::Solido, 298.15, 3000.0),
        make_sp(40, "Óxido de hierro(III)", "Fe2O3", 159.688, -824.200, 87.400, 97.7360, 0.0072887, -1.5069e-6, 1.4935e-9, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(41, "Óxido de níquel(II)", "NiO", 74.693, -239.700, 38.000, 47.0000, 0.0095000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(42, "Óxido de plomo(II)", "PbO", 223.200, -219.000, 66.500, 44.6800, 0.0178500, 0.0, -8.2e-9, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(43, "Óxido de cobre(I)", "Cu2O", 143.090, -168.600, 93.100, 65.0000, 0.0150000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(44, "Óxido de cobre(II)", "CuO", 79.545, -157.300, 42.630, 43.8300, 0.0107200, 0.0, -7.3e-9, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(45, "Calcosina", "Cu2S", 159.160, -79.500, 120.900, 75.0000, 0.0200000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(46, "Troilita", "FeS", 87.910, -100.000, 60.290, 51.6000, 0.0230000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(47, "Pirita", "FeS2", 119.975, -171.500, 52.900, 62.0000, 0.0300000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(48, "Esfalerita", "ZnS", 97.460, -206.000, 57.700, 48.0000, 0.0150000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(49, "Galena", "PbS", 239.270, -100.400, 91.200, 48.0000, 0.0120000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(50, "Molibdenita", "MoS2", 160.070, -275.000, 62.600, 65.0000, 0.0200000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(51, "Cloruro de calcio", "CaCl2", 110.984, -795.800, 108.400, 72.5900, 0.0119000, 0.0, -7.3e-9, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(52, "Cloruro de magnesio", "MgCl2", 95.211, -641.300, 89.600, 71.0900, 0.0141400, 0.0, -8.8e-9, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(53, "Cloruro de aluminio", "AlCl3", 133.341, -704.200, 109.290, 91.0000, 0.0120000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(54, "Tetracloruro de titanio", "TiCl4", 189.679, -763.200, 353.200, 106.200, 0.0015000, 0.0, 0.0, nucleo::Fase::Gas, 298.15, 4000.0),
        make_sp(55, "Cloruro de zinc", "ZnCl2", 136.286, -415.050, 111.460, 68.5000, 0.0160000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(56, "Cloruro de hierro(II)", "FeCl2", 126.751, -341.790, 117.950, 75.2000, 0.0120000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(57, "Cloruro de plomo(II)", "PbCl2", 278.106, -359.410, 136.000, 72.8000, 0.0180000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(58, "Carbonato de calcio", "CaCO3", 100.086, -1207.600, 91.700, 82.3400, 0.0497500, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(59, "Carbonato de magnesio", "MgCO3", 84.314, -1095.800, 65.700, 75.5000, 0.0420000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(60, "Siderita", "FeCO3", 115.854, -740.600, 92.900, 82.0000, 0.0250000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(61, "Smithsonita", "ZnCO3", 125.389, -812.780, 82.400, 79.7000, 0.0350000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(62, "Sulfato de calcio", "CaSO4", 136.141, -1434.100, 106.700, 99.7000, 0.0350000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(63, "Sulfato de magnesio", "MgSO4", 120.366, -1284.900, 91.600, 96.0000, 0.0340000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(64, "Sulfato de zinc", "ZnSO4", 161.470, -982.800, 110.500, 99.0000, 0.0320000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(65, "Sulfato de cobre(II)", "CuSO4", 159.609, -771.360, 109.200, 98.8000, 0.0320000, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(66, "Cobalto", "Co", 58.933, 0.0, 30.040, 24.8100, 0.0082, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
        make_sp(67, "Óxido de cobalto(II)", "CoO", 74.932, -237.940, 52.970, 48.2800, 0.0085400, 0.0, 0.0, nucleo::Fase::Solido, 298.15, 4000.0),
    };
    return cat;
}

inline std::string format_phase_label(const std::string& clean_f, nucleo::Fase f) {
    if (f == nucleo::Fase::Solido) return clean_f + "(s)";
    if (f == nucleo::Fase::Liquido) return clean_f + "(l)";
    if (f == nucleo::Fase::Gas) return clean_f + "(g)";
    return clean_f;
}

inline std::vector<nucleo::Especie> resolver_polimorfos(const domain::ThermoRepository& repo,
                                                        const std::string& formula_o_nombre,
                                                        bool usar_envolvente_fases) {
    std::string phase_tag;
    std::string clean = clean_formula_str(formula_o_nombre, &phase_tag);
    std::string raw_trim = trim_str(formula_o_nombre);

    const auto& mapa = repo.mapa_especies();
    const bool repo_pequeno = mapa.size() < 50;

    std::vector<nucleo::Especie> halladas;
    for (const auto& [id, esp] : mapa) {
        if (!repo_pequeno && !esp.es_curada()) continue;
        if (es_especie_acuosa(esp)) continue;
        if (!repo_pequeno && std::abs(esp.cp_a) < 1e-6 && std::abs(esp.cp_b) < 1e-9) continue;
        std::string esp_tag;
        std::string esp_clean = clean_formula_str(esp.formula, &esp_tag);
        if (esp.formula == raw_trim || esp.nombre == raw_trim || esp_clean == clean) {
            if (clean == "H2O" && phase_tag == "g" && esp.fase_std != nucleo::Fase::Gas) continue;
            if (clean == "H2O" && phase_tag == "l" && esp.fase_std != nucleo::Fase::Liquido) continue;
            halladas.push_back(esp);
        }
    }

    if (halladas.empty()) {
        for (const auto& esp : catalogo_respaldo()) {
            std::string esp_tag;
            std::string esp_clean = clean_formula_str(esp.formula, &esp_tag);
            if (esp.formula == raw_trim || esp.nombre == raw_trim || esp_clean == clean) {
                if (clean == "H2O" && phase_tag == "g" && esp.fase_std != nucleo::Fase::Gas) continue;
                if (clean == "H2O" && phase_tag == "l" && esp.fase_std != nucleo::Fase::Liquido) continue;
                halladas.push_back(esp);
            }
        }
    }

    if (halladas.empty()) return halladas;

    // Seleccionar primero la fase condensada estable a 298.15 K
    auto base_it = std::min_element(halladas.begin(), halladas.end(), [](const auto& a, const auto& b) {
        return a.energia_gibbs(kTref) < b.energia_gibbs(kTref);
    });
    nucleo::Especie base_sp = *base_it;
    base_sp.formula = format_phase_label(clean, base_sp.fase_std);

    // Si el usuario pidió explícitamente la fase gaseosa de un metal con transición conocida (p. ej. "Zn(g)" en Fig. 7)
    if (phase_tag == "g" && base_sp.fase_std != nucleo::Fase::Gas) {
        for (const auto& tr : tabla_transiciones()) {
            if (clean == tr.clean_sym && tr.T_m > 0.0 && tr.T_b > tr.T_m) {
                nucleo::Especie liq = make_transition_polymorph(
                    base_sp, clean + "(l)", base_sp.nombre + " (l)", nucleo::Fase::Liquido,
                    tr.T_m, tr.dH_m, tr.cp_l);
                nucleo::Especie gas = make_transition_polymorph(
                    liq, clean + "(g)", base_sp.nombre + " (g)", nucleo::Fase::Gas,
                    tr.T_b, tr.dH_v, tr.cp_g);
                return {gas};
            }
        }
    }

    if (!usar_envolvente_fases || (clean == "H2O" && phase_tag == "g")) {
        return {base_sp};
    }

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

} // namespace cma::application::modules::detail
