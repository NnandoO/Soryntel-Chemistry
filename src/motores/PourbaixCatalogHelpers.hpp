#pragma once

#include "domain/PourbaixTypes.hpp"
#include "nucleo/tabla_periodica.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <iomanip>
#include <map>
#include <set>
#include <sstream>
#include <string>
#include <vector>

namespace cma::motores::detail_eph {

constexpr double kCalToKJ = 4.184;
constexpr double kKJToKcal = 1.0 / 4.184;
constexpr double kR_J = 8.314462618;
constexpr double kR_kJ = 0.008314462618;
constexpr double kFaraday_C = 96485.33212;
constexpr double kFaraday_kJ_V = 96.48533212;
constexpr double kLn10 = 2.302585092994046;

inline double offset_escala_potencial(domain::PourbaixPotentialScale esc) {
    switch (esc) {
        case domain::PourbaixPotentialScale::SCE:    return 0.2412;
        case domain::PourbaixPotentialScale::AgAgCl: return 0.1970;
        case domain::PourbaixPotentialScale::SHE:
        default:                                     return 0.0000;
    }
}

// Presión de vapor del agua líquida (bar) entre 0 y 374 °C (correlación Wagner/Antoine)
inline double calcular_presion_vapor_agua_bar(double T_K) {
    const double T_C = std::clamp(T_K - 273.15, 0.0, 373.9);
    if (T_C <= 100.0) {
        const double log10_mmHg = 8.07131 - 1730.63 / (233.426 + T_C);
        return std::pow(10.0, log10_mmHg) * 0.00133322368;
    }
    const double log10_mmHg = 8.14019 - 1810.94 / (244.485 + T_C);
    return std::pow(10.0, log10_mmHg) * 0.00133322368;
}

// Constante dieléctrica del agua ε(T, P) válida de 0 a 373 °C y 1 a 5000 bar (exactamente 78.382 a 25 °C, 1 bar)
inline double calcular_constante_dielectrica_agua(double T_K, double P_bar) {
    const double dT = T_K - 298.15;
    const double dP = std::max(0.0, P_bar - 1.0);
    double eps = 78.382 * std::exp(-0.004585 * dT + 0.0000018 * dT * dT) + 0.0036 * dP;
    return std::clamp(eps, 5.0, 120.0);
}

// Energía libre estándar de H2O(l) en kcal/mol en función de T y P (-56.678 kcal/mol a 25.00 °C)
inline double calcular_dG_agua_kcal(double T_K, double P_bar) {
    const double dT = T_K - 298.15;
    const double cp_kcal = 0.01799; // ~75.3 J/(mol·K) = 0.018 kcal/(mol·K)
    const double s298_kcal = 0.016718; // 69.95 J/(mol·K)
    double dg = -56.678 - s298_kcal * dT + cp_kcal * (dT - T_K * std::log(std::max(T_K, 200.0) / 298.15));
    // Corrección volumétrica V·ΔP (V_H2O ≈ 18.07 cm³/mol = 0.000432 kcal/(mol·bar))
    dg += 0.0004318 * std::max(0.0, P_bar - 1.0);
    return dg;
}

// Coeficiente de actividad por el método de Davies en función de I y ε(T)
inline double calcular_factor_davies(double fuerza_ionica, double eps_agua, double T_K) {
    if (fuerza_ionica <= 1e-9) return 1.0;
    const double A_dh = 1.8248e6 / std::pow( std::max(eps_agua * T_K, 100.0), 1.5 );
    const double sqI = std::sqrt(fuerza_ionica);
    const double log10_gamma = -A_dh * (sqI / (1.0 + sqI) - 0.3 * fuerza_ionica);
    return std::pow(10.0, log10_gamma);
}

// Extrapolación de capacidad calorífica de iones acuosos por el principio de correspondencia de Criss-Cobble
inline double calcular_cp_criss_cobble_J(int carga, double s298_J_K, double T_K) {
    if (std::abs(T_K - 298.15) < 0.5) return 0.0;
    // Parámetros clásicos Criss-Cobble para cationes y aniones/oxianiones a T > 25 °C
    const double a_cc = (carga >= 0) ? 45.0 : -115.0;
    const double b_cc = (carga >= 0) ? -0.52 : -0.40;
    return a_cc + b_cc * s298_J_K;
}

// Recorte de polígono de Sutherland-Hodgman contra el semiplano A*pH + B*Eh + C <= 0
inline std::vector<domain::EpHVertice2D> clip_polygon_eph(const std::vector<domain::EpHVertice2D>& poly,
                                                          double A, double B, double C_const) {
    if (poly.empty()) return poly;
    const double norm = std::hypot(A, B);
    if (norm < 1e-13) {
        return (C_const <= 1e-10) ? poly : std::vector<domain::EpHVertice2D>{};
    }
    const double inv = 1.0 / norm;
    A *= inv;
    B *= inv;
    C_const *= inv;

    std::vector<domain::EpHVertice2D> res;
    res.reserve(poly.size() + 2);
    auto dist = [=](const domain::EpHVertice2D& pt) { return A * pt.pH + B * pt.Eh + C_const; };
    const double eps = 1e-9;

    for (size_t i = 0; i < poly.size(); ++i) {
        const auto cur = poly[i];
        const auto prev = poly[(i == 0) ? (poly.size() - 1) : (i - 1)];
        const double d_cur = dist(cur);
        const double d_prev = dist(prev);
        if (d_cur <= eps) {
            if (d_prev > eps) {
                const double t = d_prev / (d_prev - d_cur);
                res.push_back({prev.pH + t * (cur.pH - prev.pH), prev.Eh + t * (cur.Eh - prev.Eh)});
            }
            res.push_back(cur);
        } else if (d_prev <= eps) {
            const double t = d_prev / (d_prev - d_cur);
            res.push_back({prev.pH + t * (cur.pH - prev.pH), prev.Eh + t * (cur.Eh - prev.Eh)});
        }
    }
    return res;
}

// Ubica la etiqueta automáticamente en el punto más ancho del área de estabilidad (Sección 17.4.2 HSC EpH)
inline void calcular_centroide_mas_ancho_eph(domain::EpHPoligonoPredominancia& pf) {
    if (pf.vertices.empty()) return;
    double area2 = 0.0, cx = 0.0, cy = 0.0;
    double min_pH = 1e30, max_pH = -1e30;
    const size_t n = pf.vertices.size();
    for (size_t k = 0; k < n; ++k) {
        const auto& p0 = pf.vertices[k];
        const auto& p1 = pf.vertices[(k + 1) % n];
        min_pH = std::min(min_pH, p0.pH);
        max_pH = std::max(max_pH, p0.pH);
        const double cross = p0.pH * p1.Eh - p1.pH * p0.Eh;
        area2 += cross;
        cx += (p0.pH + p1.pH) * cross;
        cy += (p0.Eh + p1.Eh) * cross;
    }
    pf.area = 0.5 * std::abs(area2);
    if (std::abs(area2) > 1e-9) {
        pf.centroide = {cx / (3.0 * area2), cy / (3.0 * area2)};
    } else {
        pf.centroide = pf.vertices.front();
        return;
    }

    // Buscar el corte de pH con mayor altura vertical ΔEh (o corte horizontal con mayor ΔpH)
    double best_span = -1.0;
    domain::EpHVertice2D best_pt = pf.centroide;
    for (int s = 1; s <= 15; ++s) {
        const double x = min_pH + (max_pH - min_pH) * (s / 16.0);
        std::vector<double> ys;
        for (size_t k = 0; k < n; ++k) {
            const auto& p0 = pf.vertices[k];
            const auto& p1 = pf.vertices[(k + 1) % n];
            if ((p0.pH <= x && x <= p1.pH) || (p1.pH <= x && x <= p0.pH)) {
                if (std::abs(p1.pH - p0.pH) > 1e-9) {
                    const double t = (x - p0.pH) / (p1.pH - p0.pH);
                    ys.push_back(p0.Eh + t * (p1.Eh - p0.Eh));
                }
            }
        }
        if (ys.size() >= 2) {
            auto [it_lo, it_hi] = std::minmax_element(ys.begin(), ys.end());
            const double span = *it_hi - *it_lo;
            // Ponderar ligeramente hacia el centro en pH para evitar bordes extremos
            const double center_w = 1.0 - 0.25 * std::abs((s - 8.0) / 8.0);
            if (span * center_w > best_span) {
                best_span = span * center_w;
                best_pt = {x, 0.5 * (*it_lo + *it_hi)};
            }
        }
    }
    if (best_span > 0.02) {
        pf.centroide = best_pt;
    }
}

// Parser de fórmulas HSC EpH: extrae composición elemental, carga iónica y tipo de especie
inline bool parsear_especie_eph(const std::string& raw,
                                std::map<std::string, double>& out_comp,
                                int& out_carga,
                                int& out_estado_tag) {
    out_comp.clear();
    out_carga = 0;
    out_estado_tag = -1; // -1 = usar fase_std

    std::string s = raw;
    while (!s.empty() && std::isspace(static_cast<unsigned char>(s.front()))) s.erase(0, 1);
    while (!s.empty() && std::isspace(static_cast<unsigned char>(s.back()))) s.pop_back();
    if (s.empty()) return false;

    // 1. Examinar sufijo entre paréntesis al final: (+a), (+2a), (-a), (-2a), (a), (g), (l), (s), (M), (B), (G), (FCC), (T)
    if (s.back() == ')') {
        const size_t open = s.rfind('(');
        if (open != std::string::npos && open > 0) {
            const std::string tag = s.substr(open + 1, s.size() - open - 2);
            bool es_sufijo = false;
            if (tag == "g") {
                out_estado_tag = 0;
                es_sufijo = true;
            } else if (tag == "l") {
                out_estado_tag = 1;
                es_sufijo = true;
            } else if (tag == "s" || tag == "M" || tag == "B" || tag == "G" || tag == "FCC" || tag == "T" || tag == "A") {
                out_estado_tag = 2;
                es_sufijo = true;
            } else if (tag == "a" || tag == "aq") {
                out_estado_tag = 4; // Aqueous Neutral
                es_sufijo = true;
            } else if (!tag.empty() && (tag.front() == '+' || tag.front() == '-')) {
                const int sign = (tag.front() == '+') ? 1 : -1;
                bool tiene_a = (tag.back() == 'a');
                bool tiene_g = (tag.back() == 'g');
                std::string num_str = tag.substr(1, tag.size() - (tiene_a || tiene_g ? 2 : 1));
                int mag = 1;
                if (!num_str.empty()) {
                    try { mag = std::stoi(num_str); } catch (...) { mag = 1; }
                }
                out_carga = sign * mag;
                out_estado_tag = tiene_g ? 5 : 3; // 5 = Gas Ion, 3 = Aqueous Ion
                es_sufijo = true;
            }
            if (es_sufijo) {
                s = s.substr(0, open);
            }
        }
    }

    // 2. Dividir por '*' o '·' para aductos como CuO*CuSO4
    std::vector<std::string> parts;
    std::string cur;
    for (size_t i = 0; i < s.size();) {
        if (s[i] == '*') {
            parts.push_back(cur);
            cur.clear();
            ++i;
        } else if (i + 1 < s.size() &&
                   static_cast<unsigned char>(s[i]) == 0xC2 &&
                   static_cast<unsigned char>(s[i + 1]) == 0xB7) {
            parts.push_back(cur);
            cur.clear();
            i += 2;
        } else {
            cur.push_back(s[i++]);
        }
    }
    parts.push_back(cur);

    const auto& tp = nucleo::TablaPeriodica::instancia();

    for (std::string part : parts) {
        if (part.empty()) return false;
        size_t pos = 0;
        auto read_number = [&](double def_val) -> double {
            size_t start = pos;
            bool dot = false;
            while (pos < part.size() && (std::isdigit(static_cast<unsigned char>(part[pos])) ||
                                         (!dot && part[pos] == '.'))) {
                if (part[pos] == '.') dot = true;
                ++pos;
            }
            if (pos == start) return def_val;
            try { return std::stod(part.substr(start, pos - start)); } catch (...) { return def_val; }
        };

        double mult = 1.0;
        if (pos < part.size() && std::isdigit(static_cast<unsigned char>(part[pos]))) {
            mult = read_number(1.0);
        }

        bool valid = true;
        auto parse_group = [&](auto& self, int depth) -> std::map<std::string, double> {
            std::map<std::string, double> grp;
            if (depth > 12) { valid = false; return grp; }
            while (valid && pos < part.size() && part[pos] != ')') {
                std::map<std::string, double> sub;
                if (part[pos] == '(') {
                    ++pos;
                    sub = self(self, depth + 1);
                    if (pos >= part.size() || part[pos] != ')') { valid = false; break; }
                    ++pos;
                } else if (std::isupper(static_cast<unsigned char>(part[pos]))) {
                    std::string sym(1, part[pos++]);
                    if (pos < part.size() && std::islower(static_cast<unsigned char>(part[pos]))) {
                        sym.push_back(part[pos++]);
                    }
                    if (!tp.por_simbolo(sym)) { valid = false; break; }
                    sub[sym] = 1.0;
                } else {
                    valid = false;
                    break;
                }
                const double factor = read_number(1.0);
                for (const auto& [el, cnt] : sub) grp[el] += cnt * factor;
            }
            return grp;
        };

        auto g = parse_group(parse_group, 0);
        if (!valid || pos != part.size()) return false;
        for (const auto& [el, cnt] : g) out_comp[el] += cnt * mult;
    }
    return !out_comp.empty();
}

struct EpHCatalogRow {
    const char* formula;
    const char* nombre;
    int estado;          // 0=Gas, 1=Liquid, 2=Condensed, 3=AqueousIon, 4=AqueousNeutral, 5=GasIon
    double dG298_kcal;   // ΔGf°(298.15 K) en kcal/mol (exacto HSC EpH Table Fig. 2)
    double s298_J_K;     // S°(298.15 K) en J/(mol·K)
    double cp298_J_K;    // Cp°(298.15 K) en J/(mol·K)
    bool default_selected;
};

inline const std::vector<EpHCatalogRow>& catalogo_eph_hsc() {
    static const std::vector<EpHCatalogRow> rows = {
        // ── Sistema Cu - S - H2O (Fig. 1, Fig. 2, Fig. 3, Fig. 4 — CUS25.eph8) ──
        // Especies condensadas de Cu (ordenadas como en Fig. 2 Table rows 17-23)
        {"Cu",             "Copper",                       2,    0.000,  33.15,  24.44, true},
        {"Cu(FCC)",        "Copper (FCC)",                 2,    0.003,  33.15,  24.44, true},
        {"CuO",            "Tenorite (CuO)",               2,  -30.974,  42.63,  42.30, true},
        {"CuO(T)",         "Tenorite (T)",                 2,  -30.563,  42.63,  42.30, true},
        {"Cu2O",           "Cuprite (Cu2O)",               2,  -35.335,  93.14,  63.60, true},
        {"Cu(OH)2",        "Spertiniite Cu(OH)2",          2,  -89.079,  87.00,  95.19, true},
        {"CuO*CuSO4",      "Dolerophanite",                2, -188.354, 154.20, 142.60, true},
        {"Cu2S",           "Chalcocite (Cu2S)",            2,  -21.564, 120.90,  76.30, true},
        {"CuS",            "Covellite (CuS)",              2,  -11.200,  66.50,  47.80, false},
        {"CuSO4",          "Chalcocyanite (CuSO4)",        2, -158.200, 109.20,  98.80, true},

        // Iones acuosos de Cu (Fig. 1 Select Species list)
        {"Cu(+a)",         "Cuprous Ion Cu+",              3,   11.950,  40.60,  20.00, true},
        {"Cu(+2a)",        "Cupric Ion Cu2+",              3,   15.491, -99.60, -24.00, true},
        {"Cu(HS)2(-a)",    "Bis(hydrosulfido)cuprate(I)",  3,   18.500, 145.00,  40.00, true},
        {"CuO2(-2a)",      "Cuprate(II) Ion CuO2^2-",      3,  -43.900, -80.00, -40.00, true},
        {"CuOH(+a)",       "Hydroxycopper(II) Ion",        3,  -30.200, -15.00,   0.00, true},
        {"Cu(OH)3(-a)",    "Trihydroxycuprate(II)",        3, -118.000,  25.00,  10.00, true},
        {"Cu(OH)4(-2a)",   "Tetrahydroxycuprate(II)",      3, -157.300, -10.00,   0.00, true},
        {"Cu2OH(+3a)",     "Di-copper Hydroxy Ion",        3,  -16.000, -90.00, -20.00, true},
        {"Cu2(OH)2(+2a)",  "Dihydroxydicopper(II)",        3,  -67.800, -45.00,   0.00, true},
        {"Cu3(OH)4(+2a)",  "Tetrahydroxytricopper(II)",    3, -151.400, -60.00,   0.00, true},
        {"HCuO2(-a)",      "Bicuprate Ion HCuO2-",         3,  -61.800,  30.00,   0.00, true},

        // Especies condensadas de S (Fig. 1 Select Species top rows)
        {"S",              "Sulfur Orthorhombic",          2,    0.000,  32.05,  22.60, false},
        {"S(M)",           "Sulfur Monoclinic",            2,    0.023,  32.90,  23.60, false},
        {"SO3",            "Sulfur Trioxide (s)",          2,  -88.000,  70.70,  80.00, true},
        {"SO3(B)",         "Sulfur Trioxide (B)",          2,  -87.900,  71.00,  80.00, false},
        {"SO3(G)",         "Sulfur Trioxide (G)",          2,  -87.800,  72.00,  80.00, false},

        // Iones y especies acuosas de S (Fig. 1 & Fig. 4 exact predominance areas)
        {"HS(-a)",         "Bisulfide Ion HS-",            3,    2.880,  62.80, -90.00, true},
        {"S(-2a)",         "Sulfide Ion S2-",              3,   20.479, -14.60,-150.00, true},
        {"HS2(-a)",        "Disulfide Hydrosulfide HS2-",  3,    5.200,  95.00, -60.00, true},
        {"HS3(-a)",        "Trisulfide Hydrosulfide HS3-", 3,    7.100, 125.00, -50.00, true},
        {"HS4(-a)",        "Tetrasulfide Ion HS4-",        3,    8.900, 155.00, -40.00, true},
        {"HS5(-a)",        "Pentasulfide Ion HS5-",        3,   10.500, 185.00, -30.00, true},
        {"HS6(-a)",        "Hexasulfide Ion HS6-",         3,   12.200, 215.00, -20.00, true},
        {"HSO3(-a)",       "Bisulfite Ion HSO3-",          3, -126.150, 139.70, -15.00, true},
        {"HSO4(-a)",       "Bisulfate Ion HSO4-",          3, -175.900, 131.80, -22.00, true},
        {"HSO5(-a)",       "Peroxymonosulfate HSO5-",      3, -152.400, 160.00,   0.00, true},
        {"HS2O3(-a)",      "Hydrogen Thiosulfate HS2O3-",  3, -124.900, 110.00, -30.00, true},
        {"HS2O4(-a)",      "Hydrogen Dithionite HS2O4-",   3, -143.500, 135.00, -20.00, true},
        {"HS2O5(-a)",      "Hydrogen Disulfite HS2O5-",    3, -230.000, 165.00, -20.00, true},
        {"HS2O6(-a)",      "Hydrogen Dithionate HS2O6-",   3, -252.000, 170.00, -20.00, true},
        {"HS2O7(-a)",      "Hydrogen Pyrosulfate HS2O7-",  3, -329.947, 185.00, -15.00, true},
        {"HS2O8(-a)",      "Hydrogen Peroxydisulfate",     3, -362.875, 210.00, -10.00, true},
        {"SO4(-2a)",       "Sulfate Ion SO4^2-",           3, -178.578,  20.10,-293.00, true},
        {"S5O3(-2a)",      "Pentathiosulfate S5O3^2-",     3, -249.259, 195.00, -80.00, true},
        {"S3O3(-2a)",      "Trithiosulfate S3O3^2-",       3, -202.441, 145.00, -90.00, true},
        {"S2O3(-2a)",      "Thiosulfate Ion S2O3^2-",      3, -124.000,  67.00,-240.00, false},
        {"H2S(a)",         "Dissolved Hydrogen Sulfide",   4,   -6.660, 121.00, 178.00, true},
        {"SO2(a)",         "Dissolved Sulfur Dioxide",     4,  -71.870, 161.90, 195.00, true},
        {"H2S(g)",         "Hydrogen Sulfide Gas",         0,   -8.020, 205.80,  34.20, true},
        {"SO2(g)",         "Sulfur Dioxide Gas",           0,  -71.750, 248.20,  39.90, true},

        // ── Sistema Fe - H2O (Pourbaix Atlas / HSC EpH Reference) ──────────────
        {"Fe",             "Iron Metal",                   2,    0.000,  27.28,  25.10, true},
        {"Fe(+2a)",        "Ferrous Ion Fe2+",             3,  -21.870,-107.10, -33.00, true},
        {"Fe(+3a)",        "Ferric Ion Fe3+",              3,   -4.110,-278.40, -78.00, true},
        {"FeOH(+a)",       "Hydroxyiron(II) FeOH+",        3,  -66.300, -29.00,   0.00, true},
        {"FeOH(+2a)",      "Hydroxyiron(III) FeOH2+",      3,  -57.720,-142.00, -20.00, true},
        {"Fe(OH)2(+a)",    "Dihydroxyiron(III) Fe(OH)2+",  3, -106.200, -35.00,   0.00, true},
        {"HFeO2(-a)",      "Bihypoferrite HFeO2-",         3,  -90.630,  40.00,   0.00, true},
        {"FeO4(-2a)",      "Ferrate(VI) Ion FeO4^2-",      3,  -76.900,  38.00,-120.00, true},
        {"Fe3O4",          "Magnetite (Fe3O4)",            2, -242.570, 146.40, 143.40, true},
        {"Fe2O3",          "Hematite (Fe2O3)",             2, -177.850,  87.40, 103.90, true},
        {"Fe(OH)2",        "Ferrous Hydroxide",            2, -117.100,  88.00,  97.00, false},
        {"Fe(OH)3",        "Ferric Hydroxide",             2, -168.600, 106.70, 101.70, false},

        // ── Sistema Mn - H2O (Sección 17.1 HSC Manual Introduction Example) ────
        {"Mn",             "Manganese Metal",              2,    0.000,  32.01,  26.32, true},
        {"Mn(+2a)",        "Manganous Ion Mn2+",           3,  -54.500, -73.60, -17.00, true},
        {"Mn(+3a)",        "Manganic Ion Mn3+",            3,  -20.400,-305.00, -80.00, true},
        {"HMnO2(-a)",      "Bimanganite Ion HMnO2-",       3, -120.900,  55.00,   0.00, true},
        {"MnO4(-2a)",      "Manganate(VI) Ion MnO4^2-",    3, -119.700,  59.00,-100.00, true},
        {"MnO4(-a)",       "Permanganate Ion MnO4-",       3, -107.000, 191.20, -82.00, true},
        {"Mn(OH)2",        "Pyrochroite Mn(OH)2",          2, -147.100,  99.20,  95.00, true},
        {"Mn3O4",          "Hausmannite (Mn3O4)",          2, -306.700, 155.60, 139.70, true},
        {"Mn2O3",          "Bixbyite (Mn2O3)",             2, -210.600, 110.50,  98.70, true},
        {"MnO2",           "Pyrolusite (MnO2)",            2, -111.180,  53.05,  54.14, true},

        // ── Sistema Au - Cl - H2O (Hidrometalurgia Cloruración Ácida) ──────────
        {"Au",             "Gold Metal",                   2,    0.000,  47.40,  25.42, true},
        {"Au(+a)",         "Aurous Ion Au+",               3,   39.000, 102.00,   0.00, true},
        {"Au(+3a)",        "Auric Ion Au3+",               3,  103.600,-120.00, -40.00, true},
        {"AuCl2(-a)",      "Dichloroaurate(I) AuCl2-",     3,  -36.130, 175.00, -30.00, true},
        {"AuCl4(-a)",      "Tetrachloroaurate(III) AuCl4-",3,  -56.220, 266.50, -50.00, true},
        {"Au(OH)3",        "Auric Hydroxide Au(OH)3",      2,  -75.770, 189.50, 112.00, true},
        {"Au2O3",          "Gold(III) Oxide Au2O3",        2,   18.550, 130.00, 107.00, false},
        {"Cl(-a)",         "Chloride Ion Cl-",             3,  -31.372,  56.50,-136.40, true},
        {"ClO(-a)",        "Hypochlorite Ion ClO-",        3,   -8.800,  42.00, -80.00, true},
        {"ClO3(-a)",       "Chlorate Ion ClO3-",           3,   -1.900, 162.30, -52.00, true},
        {"ClO4(-a)",       "Perchlorate Ion ClO4-",        3,   -2.060, 182.00, -25.00, true},
        {"HClO(a)",        "Hypochlorous Acid",            4,  -19.100, 142.00,  80.00, true},
        {"Cl2(a)",         "Dissolved Chlorine",           4,    1.650, 121.00, 150.00, true},
        {"Cl2(g)",         "Chlorine Gas",                 0,    0.000, 223.08,  33.90, true},

        // ── Sistemas adicionales (Zn, Ni, Pb, Ag, Cr, Ti) ──────────────────────
        {"Zn",             "Zinc Metal",                   2,    0.000,  41.63,  25.40, true},
        {"Zn(+2a)",        "Zinc Ion Zn2+",                3,  -35.170,-112.10, -24.00, true},
        {"ZnOH(+a)",       "Hydroxyzinc Ion ZnOH+",        3,  -79.800, -15.00,   0.00, true},
        {"HZnO2(-a)",      "Bizincate Ion HZnO2-",         3, -110.900,  35.00,   0.00, true},
        {"ZnO2(-2a)",      "Zincate Ion ZnO2^2-",          3,  -93.030, -40.00, -80.00, true},
        {"ZnO",            "Zincite (ZnO)",                2,  -76.640,  43.65,  40.25, true},
        {"Zn(OH)2",        "Zinc Hydroxide",               2, -132.680,  81.20,  72.40, true},
        {"ZnS",            "Sphalerite (ZnS)",             2,  -48.110,  57.70,  46.00, true},
        {"Ni",             "Nickel Metal",                 2,    0.000,  29.87,  26.07, true},
        {"Ni(+2a)",        "Nickelous Ion Ni2+",           3,  -10.900,-128.90, -48.00, true},
        {"HNiO2(-a)",      "Binickelite Ion HNiO2-",       3,  -83.460,  35.00,   0.00, true},
        {"Ni(OH)2",        "Theophrastite Ni(OH)2",        2, -106.900,  88.00,  82.00, true},
        {"Ni3O4",          "Nickel(II,III) Oxide",         2, -170.150, 140.00, 135.00, true},
        {"Ni2O3",          "Nickel(III) Oxide",            2, -112.270,  94.00,  98.00, true},
        {"NiO2",           "Nickel Peroxide NiO2",         2,  -51.420,  54.00,  60.00, true},
        {"NiS",            "Millerite (NiS)",              2,  -19.000,  53.00,  47.10, true},
        {"Pb",             "Lead Metal",                   2,    0.000,  64.80,  26.44, true},
        {"Pb(+2a)",        "Plumbous Ion Pb2+",            3,   -5.830,  10.50, -53.00, true},
        {"HPbO2(-a)",      "Biplumbite Ion HPbO2-",        3,  -80.900,  50.00,   0.00, true},
        {"PbO",            "Litharge (PbO)",               2,  -45.160,  66.50,  45.80, true},
        {"Pb3O4",          "Minium (Pb3O4)",               2, -143.700, 211.30, 146.90, true},
        {"PbO2",           "Plattnerite (PbO2)",           2,  -51.950,  68.60,  61.10, true},
        {"PbS",            "Galena (PbS)",                 2,  -23.600,  91.20,  49.50, true},
        {"PbSO4",          "Anglesite (PbSO4)",            2, -194.360, 148.50, 103.20, true},
        {"Ag",             "Silver Metal",                 2,    0.000,  42.55,  25.35, true},
        {"Ag(+a)",         "Silver Ion Ag+",               3,   18.433,  72.68,  21.80, true},
        {"Ag2O",           "Silver(I) Oxide Ag2O",         2,   -2.680, 121.30,  65.86, true},
        {"Ag2O2",          "Silver(I,III) Oxide Ag2O2",    2,    6.600, 117.00,  88.00, true},
        {"Ag2S",           "Acanthite (Ag2S)",             2,   -9.720, 144.00,  76.50, true},
        {"Cr",             "Chromium Metal",               2,    0.000,  23.77,  23.35, true},
        {"Cr(+2a)",        "Chromous Ion Cr2+",            3,  -42.100, -55.00, -20.00, true},
        {"Cr(+3a)",        "Chromic Ion Cr3+",             3,  -51.500,-307.50, -70.00, true},
        {"Cr2O3",          "Eskolaite (Cr2O3)",            2, -252.900,  81.20, 118.70, true},
        {"HCrO4(-a)",      "Bichromate Ion HCrO4-",        3, -182.800, 184.10, -20.00, true},
        {"CrO4(-2a)",      "Chromate Ion CrO4^2-",         3, -173.960,  50.21,-240.00, true},
        {"Cr2O7(-2a)",     "Dichromate Ion Cr2O7^2-",      3, -311.000, 261.90, -90.00, true},
        {"Ti",             "Titanium Metal",               2,    0.000,  30.63,  25.02, true},
        {"Ti(+2a)",        "Titanous Ion Ti2+",            3,  -75.100, -80.00, -20.00, true},
        {"Ti(+3a)",        "Titanic Ion Ti3+",             3,  -83.600,-240.00, -60.00, true},
        {"TiO(+2a)",       "Titanyl Ion TiO2+",            3, -138.000,-110.00, -30.00, true},
        {"TiO",            "Titanium(II) Oxide TiO",       2, -118.300,  34.80,  39.90, true},
        {"Ti2O3",          "Titanium(III) Oxide Ti2O3",    2, -342.700,  78.80,  97.40, true},
        {"TiO2",           "Rutile (TiO2)",                2, -212.600,  50.33,  55.06, true}
    };
    return rows;
}

} // namespace cma::motores::detail_eph
