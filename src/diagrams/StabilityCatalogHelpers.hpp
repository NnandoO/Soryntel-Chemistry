#pragma once

#include "StabilityDiagrams.hpp"
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

namespace cma::application::modules::detail {

constexpr double kR_kJ = 0.008314462618;
constexpr double kLn10 = 2.302585092994046;

struct Vec2d {
    double x = 0.0;
    double y = 0.0;
};

// Recorte exacto de Sutherland-Hodgman contra el semiplano A*x + B*y + C <= 0
inline std::vector<Vec2d> clip_polygon_halfplane(const std::vector<Vec2d>& poly,
                                                 double A, double B, double C_const) {
    if (poly.empty()) return poly;
    const double norm = std::hypot(A, B);
    if (norm < 1e-13) {
        return (C_const <= 1e-11) ? poly : std::vector<Vec2d>{};
    }
    const double inv = 1.0 / norm;
    A *= inv;
    B *= inv;
    C_const *= inv;

    std::vector<Vec2d> res;
    res.reserve(poly.size() + 2);
    auto dist = [=](const Vec2d& pt) { return A * pt.x + B * pt.y + C_const; };
    const double eps = 1e-10;

    for (size_t i = 0; i < poly.size(); ++i) {
        const Vec2d cur = poly[i];
        const Vec2d prev = poly[(i == 0) ? (poly.size() - 1) : (i - 1)];
        const double d_cur = dist(cur);
        const double d_prev = dist(prev);
        if (d_cur <= eps) {
            if (d_prev > eps) {
                const double t = d_prev / (d_prev - d_cur);
                res.push_back({prev.x + t * (cur.x - prev.x), prev.y + t * (cur.y - prev.y)});
            }
            res.push_back(cur);
        } else if (d_prev <= eps) {
            const double t = d_prev / (d_prev - d_cur);
            res.push_back({prev.x + t * (cur.x - prev.x), prev.y + t * (cur.y - prev.y)});
        }
    }
    return res;
}

inline std::string strip_phase_or_polymorph_tag(const std::string& raw, std::string* out_tag = nullptr) {
    std::string s = raw;
    while (!s.empty() && std::isspace(static_cast<unsigned char>(s.front()))) s.erase(0, 1);
    while (!s.empty() && std::isspace(static_cast<unsigned char>(s.back()))) s.pop_back();
    if (out_tag) out_tag->clear();
    if (!s.empty() && s.back() == ')') {
        const size_t open = s.rfind('(');
        if (open != std::string::npos && open > 0) {
            const std::string tag = s.substr(open + 1, s.size() - open - 2);
            const auto& tp = nucleo::TablaPeriodica::instancia();
            bool es_grupo_quimico = false;
            if (tag.size() >= 2 && (tag == "OH" || tag == "SO4" || tag == "CO3" || tag == "NO3" ||
                                    tag == "PO4" || tag == "SiO3" || tag == "BO3" || tag == "CrO4" ||
                                    tag == "AsO4" || tag == "CN" || tag == "SCN" || tag == "ClO4" ||
                                    tag == "ClO3" || tag == "ClO2" || tag == "ClO" || tag == "SO3")) {
                es_grupo_quimico = true;
            }
            if (!es_grupo_quimico && !tp.por_simbolo(tag) &&
                (tag == "g" || tag == "l" || tag == "s" || tag == "cr" ||
                 tag == "A" || tag == "B" || tag == "FCC" || tag == "T" ||
                 tag == "M" || tag == "G" || tag == "R" || tag == "a" || tag == "aq")) {
                if (out_tag) *out_tag = tag;
                return s.substr(0, open);
            }
            if (tag == "B" || tag == "C" || tag == "F") {
                if (out_tag) *out_tag = tag;
                return s.substr(0, open);
            }
        }
    }
    return s;
}

inline std::map<std::string, double> parsear_composicion_fraccionaria(const std::string& raw) {
    std::map<std::string, double> total;
    if (raw.empty() || raw.find("+a") != std::string::npos || raw.find("-a") != std::string::npos ||
        raw.find("2a)") != std::string::npos || raw.find("3a)") != std::string::npos) {
        return total;
    }

    std::string tag;
    std::string clean = strip_phase_or_polymorph_tag(raw, &tag);
    if (tag == "a" || tag == "aq") return total;

    std::vector<std::string> parts;
    std::string cur;
    for (size_t i = 0; i < clean.size();) {
        if (clean[i] == '*') {
            parts.push_back(cur);
            cur.clear();
            ++i;
        } else if (i + 1 < clean.size() &&
                   static_cast<unsigned char>(clean[i]) == 0xC2 &&
                   static_cast<unsigned char>(clean[i + 1]) == 0xB7) {
            parts.push_back(cur);
            cur.clear();
            i += 2;
        } else {
            cur.push_back(clean[i++]);
        }
    }
    parts.push_back(cur);

    const auto& tp = nucleo::TablaPeriodica::instancia();

    for (std::string part : parts) {
        while (!part.empty() && std::isspace(static_cast<unsigned char>(part.front()))) part.erase(0, 1);
        while (!part.empty() && std::isspace(static_cast<unsigned char>(part.back()))) part.pop_back();
        if (part.empty()) return {};

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
            try {
                return std::stod(part.substr(start, pos - start));
            } catch (...) {
                return def_val;
            }
        };

        double mult = 1.0;
        if (pos < part.size() && std::isdigit(static_cast<unsigned char>(part[pos]))) {
            mult = read_number(1.0);
        }

        bool valid = true;
        auto parse_group = [&](auto& self, int depth) -> std::map<std::string, double> {
            std::map<std::string, double> group;
            if (depth > 16) {
                valid = false;
                return group;
            }
            while (valid && pos < part.size() && part[pos] != ')') {
                std::map<std::string, double> sub;
                if (part[pos] == '(') {
                    ++pos;
                    sub = self(self, depth + 1);
                    if (pos >= part.size() || part[pos] != ')') {
                        valid = false;
                        break;
                    }
                    ++pos;
                } else if (std::isupper(static_cast<unsigned char>(part[pos]))) {
                    std::string sym(1, part[pos++]);
                    if (pos < part.size() && std::islower(static_cast<unsigned char>(part[pos]))) {
                        sym.push_back(part[pos++]);
                    }
                    if (!tp.por_simbolo(sym)) {
                        valid = false;
                        break;
                    }
                    sub[sym] = 1.0;
                } else {
                    valid = false;
                    break;
                }
                const double factor = read_number(1.0);
                for (const auto& [el, cnt] : sub) {
                    group[el] += cnt * factor;
                }
            }
            return group;
        };

        auto g = parse_group(parse_group, 0);
        if (!valid || pos != part.size()) return {};
        for (const auto& [el, cnt] : g) {
            total[el] += cnt * mult;
        }
    }
    return total;
}

inline double calcular_masa_molar(const std::map<std::string, double>& comp) {
    const auto& tp = nucleo::TablaPeriodica::instancia();
    double m = 0.0;
    for (const auto& [el, cnt] : comp) {
        if (const auto* info = tp.por_simbolo(el)) {
            m += info->masa_atomica * cnt;
        }
    }
    return m;
}

inline const std::vector<nucleo::Especie>& catalogo_estabilidad_hsc() {
    static std::vector<nucleo::Especie> cat;
    if (!cat.empty()) return cat;

    struct Row {
        const char* formula;
        const char* nombre;
        nucleo::Fase fase;
        double hf, sf, cpa, cpb, cpc, cpd;
    };

    const Row rows[] = {
        {"O2(g)",  "Oxygen Gas",              nucleo::Fase::Gas,      0.000, 205.147,  24.6979, 0.0167754, -8.1510e-6,  1.4986e-9},
        {"Cl2(g)", "Chlorine Gas",            nucleo::Fase::Gas,      0.000, 223.081,  33.0510, 0.0012229,  1.4466e-6, -1.4987e-9},
        {"S2(g)",  "Sulfur Diatomic Gas",     nucleo::Fase::Gas,    128.600, 228.170,  32.4700, 0.0050000,  0.0,        0.0},
        {"SO2(g)", "Sulfur Dioxide",          nucleo::Fase::Gas,   -296.842, 248.223,  38.8310, 0.0016215,  2.0518e-6, -2.3557e-9},
        {"SO3(g)", "Sulfur Trioxide Gas",     nucleo::Fase::Gas,   -395.765, 256.769,  48.5180, 0.0072450,  5.8552e-6, -9.2519e-9},
        {"H2(g)",  "Hydrogen Gas",            nucleo::Fase::Gas,      0.000, 130.680,  27.1415, 0.0092744, -1.3015e-6,  7.6313e-9},
        {"H2O(g)", "Water Vapor",             nucleo::Fase::Gas,   -241.826, 188.835,  31.9500, 0.0028273,  1.0000e-5, -3.3741e-9},
        {"H2S(g)", "Hydrogen Sulfide",        nucleo::Fase::Gas,    -20.500, 205.800,  34.5350, 0.0012540,  2.9077e-6, -1.5287e-9},
        {"HCl(g)", "Hydrogen Chloride",       nucleo::Fase::Gas,    -92.311, 186.900,  29.1330,-0.0000996,  1.9500e-6, -1.0394e-9},
        {"CO(g)",  "Carbon Monoxide",         nucleo::Fase::Gas,   -110.527, 197.660,  27.3579, 0.0037843,  3.7410e-6, -1.8158e-9},
        {"CO2(g)", "Carbon Dioxide",          nucleo::Fase::Gas,   -393.522, 213.795,  23.1089, 0.0577123, -3.3541e-5,  7.1345e-9},
        {"CH4(g)", "Methane",                 nucleo::Fase::Gas,    -74.870, 186.250,  19.2500, 0.0521300,  1.1970e-5, -1.1320e-8},
        {"N2(g)",  "Nitrogen Gas",            nucleo::Fase::Gas,      0.000, 191.609,  27.3069, 0.0061500, -9.5020e-7,  0.0},
        {"NH3(g)", "Ammonia",                 nucleo::Fase::Gas,    -45.898, 192.770,  33.5721, 0.0026289,  2.1060e-7, -1.8939e-9},
        {"NO(g)",  "Nitric Oxide",            nucleo::Fase::Gas,     90.290, 210.760,  29.6870,-0.0019454,  9.7150e-6, -5.8681e-9},
        {"NO2(g)", "Nitrogen Dioxide",        nucleo::Fase::Gas,     33.100, 240.040,  30.7140, 0.0049134,  1.1688e-6, -2.3656e-9},
        {"F2(g)",  "Fluorine Gas",            nucleo::Fase::Gas,      0.000, 202.790,  31.3000, 0.0041800, -1.2000e-6,  1.2000e-9},
        {"HF(g)",  "Hydrogen Fluoride",       nucleo::Fase::Gas,   -273.300, 173.780,  28.2000, 0.0012000,  2.5000e-6, -1.5000e-9},
        {"COS(g)", "Carbonyl Sulfide",        nucleo::Fase::Gas,   -142.000, 231.500,  41.5000, 0.0120000,  0.0,        0.0},
        {"CS2(g)", "Carbon Disulfide Gas",    nucleo::Fase::Gas,    116.900, 237.800,  45.6000, 0.0100000,  0.0,        0.0},
        {"ClO2(g)","Chlorine Dioxide",        nucleo::Fase::Gas,    102.500, 256.800,  42.0000, 0.0110000,  0.0,        0.0},

        // Sistema V-Cl-O (24 especies exactas HSC Tpp Fig. 1 / Imagen 1)
        {"V",       "Vanadium",               nucleo::Fase::Solido,     0.000,  28.900,  24.900, 0.00640, 0.0, 0.0},
        {"VCl2",    "Vanadium(II) Chloride",  nucleo::Fase::Solido,  -535.135,  97.100,  72.200, 0.01600, 0.0, 0.0},
        {"VCl3",    "Vanadium(III) Chloride", nucleo::Fase::Solido,  -721.343, 131.000,  93.000, 0.02200, 0.0, 0.0},
        {"VCl4(l)", "Vanadium(IV) Chloride",  nucleo::Fase::Liquido, -689.526, 255.000, 161.700, 0.00000, 0.0, 0.0},
        {"VO",      "Vanadium(II) Oxide",     nucleo::Fase::Solido,  -464.559,  39.000,  45.500, 0.01200, 0.0, 0.0},
        {"VO1.24",  "Vanadium Oxide VO1.24",  nucleo::Fase::Solido,  -564.097,  46.000,  51.000, 0.01400, 0.0, 0.0},
        {"VO2",     "Vanadium(IV) Oxide",     nucleo::Fase::Solido,  -823.217,  51.500,  62.500, 0.01800, 0.0, 0.0},
        {"V2O3",    "Vanadium(III) Oxide",    nucleo::Fase::Solido, -1353.011,  98.100, 108.000, 0.02500, 0.0, 0.0},
        {"V2O4",    "Divanadium Tetroxide",   nucleo::Fase::Solido, -1645.161, 103.500, 125.000, 0.03600, 0.0, 0.0},
        {"V2O5",    "Vanadium(V) Oxide",      nucleo::Fase::Solido, -1820.758, 131.000, 130.000, 0.03000, 0.0, 0.0},
        {"V3O5",    "Trivanadium Pentoxide",  nucleo::Fase::Solido, -2189.502, 149.000, 168.000, 0.04000, 0.0, 0.0},
        {"V3O7",    "Trivanadium Heptoxide",  nucleo::Fase::Solido, -2662.865, 182.000, 195.000, 0.04500, 0.0, 0.0},
        {"V4O7",    "Vanadium Magneli V4O7",  nucleo::Fase::Solido, -3011.486, 201.000, 230.000, 0.05500, 0.0, 0.0},
        {"V5O9",    "Vanadium Magneli V5O9",  nucleo::Fase::Solido, -3835.815, 252.000, 292.000, 0.07000, 0.0, 0.0},
        {"V6O11",   "Vanadium Magneli V6O11", nucleo::Fase::Solido, -4662.174, 304.000, 355.000, 0.08500, 0.0, 0.0},
        {"V6O13",   "Hexavanadium Tridecoxide",nucleo::Fase::Solido,-5147.058, 335.000, 380.000, 0.09000, 0.0, 0.0},
        {"V7O13",   "Vanadium Magneli V7O13", nucleo::Fase::Solido, -5482.454, 355.000, 417.000, 0.10000, 0.0, 0.0},
        {"V8O15",   "Vanadium Magneli V8O15", nucleo::Fase::Solido, -6304.463, 407.000, 480.000, 0.11500, 0.0, 0.0},
        {"V9O17",   "Vanadium Magneli V9O17", nucleo::Fase::Solido, -7128.442, 458.000, 542.000, 0.13000, 0.0, 0.0},
        {"V10O19",  "Vanadium Magneli V10O19",nucleo::Fase::Solido, -7950.252, 510.000, 605.000, 0.14500, 0.0, 0.0},
        {"VOCl",    "Vanadium Oxychloride",   nucleo::Fase::Solido,  -694.119,  75.000,  70.000, 0.01600, 0.0, 0.0},
        {"VOCl2",   "Vanadyl Dichloride",     nucleo::Fase::Solido,  -851.095, 115.000,  92.000, 0.02000, 0.0, 0.0},
        {"VOCl3",   "Vanadyl Trichloride",    nucleo::Fase::Solido,  -910.341, 210.000, 150.000, 0.01000, 0.0, 0.0},
        {"VO2Cl",   "Dioxovanadium Chloride", nucleo::Fase::Solido,  -916.212,  98.000,  88.000, 0.01800, 0.0, 0.0},

        // Sistema S-Cu-O (23 especies exactas HSC Tpp Fig. 2 / Imagen 3)
        {"Cu",        "Copper",               nucleo::Fase::Solido,     0.000,  33.150,  24.440, 0.00640, 0.0, 0.0},
        {"Cu(l)",     "Copper Liquid",        nucleo::Fase::Liquido,   11.800,  41.800,  31.380, 0.00000, 0.0, 0.0},
        {"Cu(FCC)",   "Copper FCC",           nucleo::Fase::Solido,     0.150,  33.150,  24.440, 0.00640, 0.0, 0.0},
        {"CuO",       "Tenorite (CuO)",       nucleo::Fase::Solido,  -157.300,  42.630,  43.830, 0.01072, 0.0,-7.3e-9},
        {"CuO(T)",    "Copper(II) Oxide (T)", nucleo::Fase::Solido,  -156.900,  42.630,  43.830, 0.01072, 0.0,-7.3e-9},
        {"Cu2O",      "Cuprite (Cu2O)",       nucleo::Fase::Solido,  -168.600,  93.100,  65.000, 0.01500, 0.0, 0.0},
        {"Cu2O(l)",   "Copper(I) Oxide (l)",  nucleo::Fase::Liquido, -112.000, 129.000,  90.000, 0.00000, 0.0, 0.0},
        {"CuO*CuSO4", "Dolerophanite",        nucleo::Fase::Solido,  -930.800, 154.200, 142.630, 0.04272, 0.0,-7.3e-9},
        {"CuS",       "Covellite (CuS)",      nucleo::Fase::Solido,   -53.100,  66.500,  47.800, 0.01100, 0.0, 0.0},
        {"Cu1.75S",   "Anilite (Cu1.75S)",    nucleo::Fase::Solido,   -74.500, 108.000,  68.500, 0.01700, 0.0, 0.0},
        {"Cu1.8S",    "Digenite (Cu1.8S)",    nucleo::Fase::Solido,   -75.800, 111.000,  70.000, 0.01800, 0.0, 0.0},
        {"Cu1.95S",   "Djurleite (Cu1.95S)",  nucleo::Fase::Solido,   -78.500, 118.000,  73.500, 0.01900, 0.0, 0.0},
        {"Cu2S",      "Chalcocite (Cu2S)",    nucleo::Fase::Solido,   -79.500, 120.900,  75.000, 0.02000, 0.0, 0.0},
        {"Cu2S(l)",   "Chalcocite Liquid",    nucleo::Fase::Liquido,  -68.000, 129.000,  85.000, 0.00000, 0.0, 0.0},
        {"CuSO4",     "Chalcoalumite/CuSO4",  nucleo::Fase::Solido,  -771.360, 109.200,  98.800, 0.03200, 0.0, 0.0},
        {"Cu2SO4",    "Copper(I) Sulfate",    nucleo::Fase::Solido,  -751.400, 182.400, 125.000, 0.03500, 0.0, 0.0},
        {"S",         "Sulfur Orthorhombic",  nucleo::Fase::Solido,     0.000,  32.054,  22.600, 0.01800, 0.0, 0.0},
        {"S(l)",      "Sulfur Liquid",        nucleo::Fase::Liquido,    1.720,  36.500,  31.000, 0.00000, 0.0, 0.0},
        {"S(M)",      "Sulfur Monoclinic",    nucleo::Fase::Solido,     0.330,  32.900,  23.600, 0.01800, 0.0, 0.0},
        {"SO3",       "Sulfur Trioxide (s)",  nucleo::Fase::Solido,  -454.500,  70.700,  80.000, 0.02000, 0.0, 0.0},
        {"SO3(l)",    "Sulfur Trioxide (l)",  nucleo::Fase::Liquido, -441.000, 113.800,  95.000, 0.00000, 0.0, 0.0},
        {"SO3(B)",    "Sulfur Trioxide (B)",  nucleo::Fase::Solido,  -454.000,  71.000,  80.000, 0.02000, 0.0, 0.0},
        {"SO3(G)",    "Sulfur Trioxide (G)",  nucleo::Fase::Solido,  -453.500,  72.000,  80.000, 0.02000, 0.0, 0.0},

        // Sistema Ti-Cl-O (24 especies exactas HSC Lpp Fig. 3 / Imagen 4)
        {"Ti",        "Titanium",             nucleo::Fase::Solido,     0.000,  30.600,  25.000, 0.00550, 0.0, 0.0},
        {"Ti(A)",     "Titanium Alpha",       nucleo::Fase::Solido,     0.200,  30.600,  25.000, 0.00550, 0.0, 0.0},
        {"Ti(B)",     "Titanium Beta",        nucleo::Fase::Solido,     2.970,  31.500,  25.500, 0.00600, 0.0, 0.0},
        {"TiCl2",     "Titanium(II) Chloride",nucleo::Fase::Solido,  -514.634,  87.400,  69.800, 0.01500, 0.0, 0.0},
        {"TiCl3",     "Titanium(III) Chloride",nucleo::Fase::Solido, -665.896, 139.700,  97.100, 0.01200, 0.0, 0.0},
        {"TiCl4",     "Titanium(IV) Chloride",nucleo::Fase::Solido,  -771.214, 252.400, 145.200, 0.00200, 0.0, 0.0},
        {"TiClO",     "Titanium Oxychloride", nucleo::Fase::Solido,  -759.988,  73.200,  68.500, 0.01400, 0.0, 0.0},
        {"TiO",       "Titanium(II) Oxide",   nucleo::Fase::Solido,  -523.093,  34.800,  44.300, 0.01400, 0.0, 0.0},
        {"TiO(B)",    "Titanium(II) Oxide B", nucleo::Fase::Solido,  -521.794,  35.200,  44.500, 0.01400, 0.0, 0.0},
        {"TiO1.01",   "Titanium Oxide 1.01",  nucleo::Fase::Solido,  -528.198,  35.000,  44.600, 0.01400, 0.0, 0.0},
        {"TiO2",      "Rutile (TiO2)",        nucleo::Fase::Solido,  -941.412,  50.620,  62.860, 0.01136, 0.0, 0.0},
        {"TiO2(A)",   "Anatase (TiO2)",       nucleo::Fase::Solido,  -941.099,  49.900,  62.500, 0.01150, 0.0, 0.0},
        {"Ti2O3",     "Titanium(III) Oxide",  nucleo::Fase::Solido, -1510.764,  77.200, 105.000, 0.02800, 0.0, 0.0},
        {"Ti3O2",     "Trititanium Dioxide",  nucleo::Fase::Solido, -1060.888, 102.000, 118.000, 0.02500, 0.0, 0.0},
        {"Ti3O5",     "Trititanium Pentoxide",nucleo::Fase::Solido, -2458.808, 129.400, 162.000, 0.03800, 0.0, 0.0},
        {"Ti3O5(B)",  "Trititanium Pentoxide B",nucleo::Fase::Solido,-2453.776,131.000, 163.000, 0.03800, 0.0, 0.0},
        {"Ti4O7",     "Titanium Magneli Ti4O7",nucleo::Fase::Solido,-3390.789, 198.700, 224.000, 0.05000, 0.0, 0.0},
        {"Ti5O9",     "Titanium Magneli Ti5O9",nucleo::Fase::Solido,-4332.295, 250.000, 286.000, 0.06200, 0.0, 0.0},
        {"Ti6O11",    "Titanium Magneli Ti6O11",nucleo::Fase::Solido,-5272.845,301.000, 348.000, 0.07400, 0.0, 0.0},
        {"Ti7O13",    "Titanium Magneli Ti7O13",nucleo::Fase::Solido,-6214.441,351.000, 410.000, 0.08600, 0.0, 0.0},
        {"Ti8O15",    "Titanium Magneli Ti8O15",nucleo::Fase::Solido,-7155.791,402.000, 472.000, 0.09800, 0.0, 0.0},
        {"Ti9O17",    "Titanium Magneli Ti9O17",nucleo::Fase::Solido,-8097.041,453.000, 534.000, 0.11000, 0.0, 0.0},
        {"Ti10O19",   "Titanium Magneli Ti10O19",nucleo::Fase::Solido,-9038.191,504.000,596.000, 0.12200, 0.0, 0.0},
        {"Ti20O39",   "Titanium Magneli Ti20O39",nucleo::Fase::Solido,-18452.127,1010.000,1215.000,0.24000,0.0,0.0},

        // Sistemas Fe-S-O, Fe-Cl-O, Ni-S-O, Pb-S-O, Zn-S-O
        {"Fe",        "Iron",                 nucleo::Fase::Solido,     0.000,  27.280,  23.900, 0.00870, 0.0, 0.0},
        {"FeO",       "Wuestite (FeO)",       nucleo::Fase::Solido,  -272.000,  60.750,  48.990, 0.00837, 0.0, 0.0},
        {"Fe3O4",     "Magnetite (Fe3O4)",    nucleo::Fase::Solido, -1118.400, 146.400, 143.429, 0.02099,-6.218e-6, 0.0},
        {"Fe2O3",     "Hematite (Fe2O3)",     nucleo::Fase::Solido,  -824.200,  87.400,  97.736, 0.00729,-1.507e-6, 1.494e-9},
        {"FeS",       "Troilite (FeS)",       nucleo::Fase::Solido,  -100.000,  60.290,  51.600, 0.02300, 0.0, 0.0},
        {"FeS2",      "Pyrite (FeS2)",        nucleo::Fase::Solido,  -171.500,  52.900,  62.000, 0.03000, 0.0, 0.0},
        {"Fe7S8",     "Pyrrhotite (Fe7S8)",   nucleo::Fase::Solido,  -736.000, 485.000, 380.000, 0.14000, 0.0, 0.0},
        {"FeSO4",     "Iron(II) Sulfate",     nucleo::Fase::Solido,  -928.400, 107.500, 100.500, 0.03200, 0.0, 0.0},
        {"Fe2(SO4)3", "Iron(III) Sulfate",    nucleo::Fase::Solido, -2583.000, 282.800, 265.000, 0.08000, 0.0, 0.0},
        {"FeCl2",     "Iron(II) Chloride",    nucleo::Fase::Solido,  -341.790, 117.950,  75.200, 0.01200, 0.0, 0.0},
        {"FeCl3",     "Iron(III) Chloride",   nucleo::Fase::Solido,  -399.500, 142.000,  96.600, 0.02000, 0.0, 0.0},
        {"FeOCl",     "Iron Oxychloride",     nucleo::Fase::Solido,  -410.000,  81.000,  70.000, 0.01500, 0.0, 0.0},
        {"Ni",        "Nickel",               nucleo::Fase::Solido,     0.000,  29.870,  25.800, 0.00620, 0.0, 0.0},
        {"NiO",       "Bunsenite (NiO)",      nucleo::Fase::Solido,  -239.700,  38.000,  47.000, 0.00950, 0.0, 0.0},
        {"Ni2O3",     "Nickel(III) Oxide",    nucleo::Fase::Solido,  -489.500, 104.600,  98.000, 0.02200, 0.0, 0.0},
        {"NiS",       "Millerite (NiS)",      nucleo::Fase::Solido,   -84.500,  53.000,  47.200, 0.01500, 0.0, 0.0},
        {"NiS2",      "Vaesite (NiS2)",       nucleo::Fase::Solido,  -131.400,  72.000,  64.000, 0.01800, 0.0, 0.0},
        {"Ni3S2",     "Heazlewoodite (Ni3S2)",nucleo::Fase::Solido,  -216.000, 133.900, 118.000, 0.03600, 0.0, 0.0},
        {"NiSO4",     "Nickel(II) Sulfate",   nucleo::Fase::Solido,  -873.300,  92.000,  98.000, 0.03400, 0.0, 0.0},
        {"NiCl2",     "Nickel(II) Chloride",  nucleo::Fase::Solido,  -305.300,  97.700,  71.700, 0.01500, 0.0, 0.0},
        {"Pb",        "Lead",                 nucleo::Fase::Solido,     0.000,  64.800,  26.400, 0.00420, 0.0, 0.0},
        {"PbO",       "Litharge (PbO)",       nucleo::Fase::Solido,  -219.000,  66.500,  44.680, 0.01785, 0.0,-8.2e-9},
        {"Pb3O4",     "Minium (Pb3O4)",       nucleo::Fase::Solido,  -718.400, 211.300, 146.900, 0.04200, 0.0, 0.0},
        {"PbO2",      "Plattnerite (PbO2)",   nucleo::Fase::Solido,  -277.400,  68.600,  61.100, 0.01600, 0.0, 0.0},
        {"PbS",       "Galena (PbS)",         nucleo::Fase::Solido,  -100.400,  91.200,  48.000, 0.01200, 0.0, 0.0},
        {"PbSO4",     "Anglesite (PbSO4)",    nucleo::Fase::Solido,  -919.900, 148.500, 103.200, 0.03200, 0.0, 0.0},
        {"PbO*PbSO4", "Lanarkite",            nucleo::Fase::Solido, -1158.900, 215.000, 148.000, 0.04900, 0.0, 0.0},
        {"2PbO*PbSO4","Lead Oxysulfate 2:1",  nucleo::Fase::Solido, -1397.900, 281.500, 193.000, 0.06600, 0.0, 0.0},
        {"PbCl2",     "Cotunnite (PbCl2)",    nucleo::Fase::Solido,  -359.410, 136.000,  72.800, 0.01800, 0.0, 0.0},
        {"Zn",        "Zinc",                 nucleo::Fase::Solido,     0.000,  41.630,  25.400, 0.00840, 0.0, 0.0},
        {"ZnO",       "Zincite (ZnO)",        nucleo::Fase::Solido,  -350.500,  43.650,  40.230, 0.01006, 0.0,-4.6e-9},
        {"ZnS",       "Sphalerite (ZnS)",     nucleo::Fase::Solido,  -206.000,  57.700,  48.000, 0.01500, 0.0, 0.0},
        {"ZnSO4",     "Zinkosite (ZnSO4)",    nucleo::Fase::Solido,  -982.800, 110.500,  99.000, 0.03200, 0.0, 0.0},
        {"ZnO*2ZnSO4","Zinc Oxysulfate 1:2",  nucleo::Fase::Solido, -2318.000, 268.000, 238.000, 0.07400, 0.0, 0.0},
        {"ZnCl2",     "Zinc Chloride",        nucleo::Fase::Solido,  -415.050, 111.460,  68.500, 0.01600, 0.0, 0.0},
    };

    int64_t next_id = 900001;
    for (const auto& r : rows) {
        nucleo::Especie e;
        e.id = next_id++;
        e.formula = r.formula;
        e.nombre = r.nombre;
        e.fase_std = r.fase;
        e.h_formacion = r.hf;
        e.s_formacion = r.sf;
        e.cp_a = r.cpa;
        e.cp_b = r.cpb;
        e.cp_c = r.cpc;
        e.cp_d = r.cpd;
        e.modelo_cp = nucleo::ModeloCp::POLINOMIAL_ESTANDAR;
        e.T_min = 250.0;
        e.T_max = 2500.0;
        const auto comp = parsear_composicion_fraccionaria(e.formula);
        e.masa_molar = calcular_masa_molar(comp);
        cat.push_back(e);
    }
    return cat;
}

inline int prioridad_elemento_principal(const std::string& sym) {
    static const std::map<std::string, int> gas_nonmetals = {
        {"O", 1}, {"Cl", 2}, {"F", 3}, {"H", 4}, {"N", 5},
        {"Br", 6}, {"I", 7}, {"S", 8}, {"C", 9}, {"Se", 10}
    };
    auto it = gas_nonmetals.find(sym);
    if (it != gas_nonmetals.end()) return it->second;
    const auto* el = nucleo::TablaPeriodica::instancia().por_simbolo(sym);
    if (!el) return 50;
    if (el->grupo >= 3 && el->grupo <= 12) return 100 + el->numero_atomico;
    if (el->grupo >= 1 && el->grupo <= 2) return 90 + el->numero_atomico;
    return 80 + el->numero_atomico;
}

inline std::string format_gas_formula(const std::string& raw) {
    std::string tag;
    std::string clean = strip_phase_or_polymorph_tag(raw, &tag);
    return clean + "(g)";
}

inline std::string format_clean_plot_label(const std::string& raw) {
    std::string tag;
    std::string clean = strip_phase_or_polymorph_tag(raw, &tag);
    if (tag == "l" || tag == "s" || tag == "g") return clean;
    return raw;
}

inline std::vector<nucleo::Especie> recolectar_catalogo_combinado(const domain::ThermoRepository* repo) {
    std::vector<nucleo::Especie> resultado;
    std::set<std::string> vistas;

    for (const auto& sp : catalogo_estabilidad_hsc()) {
        resultado.push_back(sp);
        vistas.insert(sp.formula);
        std::string clean = strip_phase_or_polymorph_tag(sp.formula);
        if (sp.fase_std == nucleo::Fase::Gas) vistas.insert(clean);
    }

    if (repo) {
        const auto& mapa = repo->mapa_especies();
        const bool repo_pequeno = mapa.size() < 50;
        for (const auto& [id, sp] : mapa) {
            if (!repo_pequeno && !sp.es_curada()) continue;
            if (sp.fase_std == nucleo::Fase::Mixto) continue;
            std::string tag;
            std::string clean = strip_phase_or_polymorph_tag(sp.formula, &tag);
            if (tag == "a" || tag == "aq") continue;
            std::string key = (sp.fase_std == nucleo::Fase::Gas) ? format_gas_formula(sp.formula) : sp.formula;
            if (vistas.count(key) || vistas.count(sp.formula) || vistas.count(clean)) continue;
            auto comp = parsear_composicion_fraccionaria(sp.formula);
            if (comp.empty()) continue;
            nucleo::Especie copy = sp;
            if (copy.fase_std == nucleo::Fase::Gas) copy.formula = key;
            resultado.push_back(copy);
            vistas.insert(copy.formula);
            vistas.insert(sp.formula);
            vistas.insert(clean);
        }
    }
    return resultado;
}

inline const nucleo::Especie* buscar_especie_catalogo(const std::vector<nucleo::Especie>& catalogo,
                                                      const std::string& formula,
                                                      bool preferir_gas = false) {
    const std::string clean_target = strip_phase_or_polymorph_tag(formula);
    const std::string gas_target = clean_target + "(g)";

    if (preferir_gas) {
        for (const auto& sp : catalogo) {
            if (sp.fase_std == nucleo::Fase::Gas &&
                (sp.formula == formula || sp.formula == gas_target ||
                 strip_phase_or_polymorph_tag(sp.formula) == clean_target)) {
                return &sp;
            }
        }
    }
    for (const auto& sp : catalogo) {
        if (sp.formula == formula) return &sp;
    }
    for (const auto& sp : catalogo) {
        if (strip_phase_or_polymorph_tag(sp.formula) == clean_target) return &sp;
    }
    return nullptr;
}

inline bool resolver_estequiometria_gases(const std::string& aux1, const std::string& aux2,
                                          const std::map<std::string, double>& comp_g1,
                                          const std::map<std::string, double>& comp_g2,
                                          const std::map<std::string, double>& comp_sp,
                                          double& out_nu1, double& out_nu2) {
    auto get_c = [](const std::map<std::string, double>& m, const std::string& el) {
        auto it = m.find(el);
        return (it != m.end()) ? it->second : 0.0;
    };
    const double a11 = get_c(comp_g1, aux1);
    const double a12 = get_c(comp_g1, aux2);
    const double a21 = get_c(comp_g2, aux1);
    const double a22 = get_c(comp_g2, aux2);
    const double b1 = get_c(comp_sp, aux1);
    const double b2 = get_c(comp_sp, aux2);

    const double det = a11 * a22 - a12 * a21;
    if (std::abs(det) > 1e-9) {
        out_nu1 = (b1 * a22 - b2 * a21) / det;
        out_nu2 = (a11 * b2 - a12 * b1) / det;
        return true;
    }
    return false;
}

inline void calcular_centroide_y_area(domain::PoligonoPredominancia& pf) {
    if (pf.vertices.empty()) return;
    double cx = 0.0, cy = 0.0, area2 = 0.0;
    const size_t n = pf.vertices.size();
    for (size_t k = 0; k < n; ++k) {
        const auto& p0 = pf.vertices[k];
        const auto& p1 = pf.vertices[(k + 1) % n];
        const double cross = p0.x * p1.y - p1.x * p0.y;
        area2 += cross;
        cx += (p0.x + p1.x) * cross;
        cy += (p0.y + p1.y) * cross;
    }
    pf.area = 0.5 * std::abs(area2);
    if (std::abs(area2) > 1e-9) {
        pf.centroide = {cx / (3.0 * area2), cy / (3.0 * area2)};
    } else {
        double sx = 0.0, sy = 0.0;
        for (const auto& v : pf.vertices) {
            sx += v.x;
            sy += v.y;
        }
        pf.centroide = {sx / static_cast<double>(n), sy / static_cast<double>(n)};
    }
}

inline std::string formatear_coef(double val) {
    if (std::abs(val - std::round(val)) < 1e-4) {
        int iv = static_cast<int>(std::round(val));
        return (iv == 1) ? "" : (std::to_string(iv) + " ");
    }
    std::ostringstream os;
    os << std::fixed << std::setprecision(2) << val << " ";
    return os.str();
}

inline std::string construir_ecuacion_frontera(const std::string& f_a, double n_ma, double nu_xa, double nu_ya,
                                               const std::string& f_b, double n_mb, double nu_xb, double nu_yb,
                                               const std::string& gas_x, const std::string& gas_y) {
    const double dx = n_ma * nu_xb - n_mb * nu_xa;
    const double dy = n_ma * nu_yb - n_mb * nu_ya;

    std::ostringstream lhs, rhs;
    lhs << formatear_coef(n_mb) << f_a;
    if (dx > 1e-4) lhs << " + " << formatear_coef(dx) << gas_x;
    if (dy > 1e-4) lhs << " + " << formatear_coef(dy) << gas_y;

    rhs << formatear_coef(n_ma) << f_b;
    if (dx < -1e-4) rhs << " + " << formatear_coef(-dx) << gas_x;
    if (dy < -1e-4) rhs << " + " << formatear_coef(-dy) << gas_y;

    return lhs.str() + " = " + rhs.str();
}

} // namespace cma::application::modules::detail
