#include "agua.hpp"
#include <cmath>
#include <algorithm>
#include <sstream>

namespace cma { namespace motores {

double Agua::P_saturacion(double T) {
    // Ecuación de Antoine para agua: log10(P_mmHg) = 8.071 - 1730.63/(233.43 + T°C)
    double Tc = T - 273.15;
    if (Tc < 0 || Tc > 374) return -1;
    double logP = 8.07131 - 1730.63 / (233.43 + Tc);
    return std::pow(10, logP) * 0.000133322; // convertir mmHg a MPa
}

double Agua::h_liquido(double T) {
    // Cp ~ 4.18 kJ/(kg·K), h ≈ Cp*(T - 273.15)
    return 4.18 * (T - 273.15);
}

double Agua::h_vapor(double T) {
    // h_v = h_l + calor latente (~2500 kJ/kg a 0°C, disminuye con T)
    double h_l = h_liquido(T);
    double h_fg = 2500.9 - 2.365 * (T - 273.15);
    return h_l + h_fg;
}

double Agua::s_liquido(double T) {
    // s ≈ Cp*ln(T/273.15)
    return 4.18 * std::log(T / 273.15);
}

double Agua::s_vapor(double T) {
    double s_l = s_liquido(T);
    double h_fg = 2500.9 - 2.365 * (T - 273.15);
    return s_l + h_fg / T;
}

PropiedadesAgua Agua::propiedades(double T, double P) {
    PropiedadesAgua res;
    res.T = T;
    res.P = P;

    double P_sat = P_saturacion(T);
    double Tc = T - 273.15;

    if (P_sat < 0 || P > P_sat * 1.1) {
        // Líquido comprimido o supercrítico
        res.fase = (T > 647.0) ? "supercrítico" : "líquido comprimido";
        res.h = h_liquido(T);
        res.s = s_liquido(T);
        res.v = 0.001; // m³/kg aproximadamente
        res.cp = 4.18;
        res.cv = 4.18;
    } else if (P < P_sat * 0.9) {
        // Vapor sobrecalentado
        res.fase = "vapor";
        res.h = h_vapor(T);
        res.s = s_vapor(T);
        double R = 0.4615; // kJ/(kg·K) para agua
        res.v = R * T / (std::max(P, 1e-6) * 1000); // m³/kg
        res.cp = 1.87;
        res.cv = 1.41;
    } else {
        // Mezcla bifásica
        res.fase = "mezcla (líq+vap)";
        double x = (P - P_sat * 0.9) / (P_sat * 0.2); // fracción de vapor
        x = std::max(0.0, std::min(1.0, x));
        res.h = h_liquido(T) + x * (h_vapor(T) - h_liquido(T));
        res.s = s_liquido(T) + x * (s_vapor(T) - s_liquido(T));
        double Rv = 0.4615; // kJ/(kg·K) para agua
        res.v = 0.001 + x * (0.001 * Rv * T / (std::max(P, 1e-6) * 1000));
        res.cp = 4.18 + x * (1.87 - 4.18);
        res.cv = 4.18 + x * (1.41 - 4.18);
    }

    res.mu = viscosidad_liquida(T);
    res.k = conductividad_termica(T, P);

    return res;
}

std::pair<PropiedadesAgua, PropiedadesAgua> Agua::saturacion_T(double T) {
    PropiedadesAgua liq, vap;
    liq.T = vap.T = T;
    liq.P = vap.P = P_saturacion(T);
    liq.fase = "líquido saturado";
    vap.fase = "vapor saturado";
    liq.h = h_liquido(T);
    vap.h = h_vapor(T);
    liq.s = s_liquido(T);
    vap.s = s_vapor(T);
    liq.v = 0.001;
    vap.v = 0.001 * 0.4615 * T / (liq.P * 1000);
    liq.cp = 4.18;
    vap.cp = 1.87;
    liq.cv = 4.18;
    vap.cv = 1.41;
    liq.mu = vap.mu = viscosidad_liquida(T);
    liq.k = vap.k = conductividad_termica(T, liq.P);
    return {liq, vap};
}

double Agua::temperatura_saturacion(double P) {
    if (P <= 0) return 273.15;
    double P_mmHg = P / 0.000133322;
    double logP = std::log10(P_mmHg);
    if (std::abs(8.07131 - logP) < 1e-10) return 373.15;
    double Tc = 1730.63 / (8.07131 - logP) - 233.43;
    return std::clamp(Tc + 273.15, 273.15, 647.0);
}

double Agua::calor_latente(double T) {
    double h_l = h_liquido(T);
    double h_v = h_vapor(T);
    return h_v - h_l;
}

double Agua::viscosidad_liquida(double T) {
    // Correlación aproximada [Pa·s]
    double Tc = T - 273.15;
    if (Tc < 0) Tc = 0;
    if (Tc > 300) Tc = 300;
    return 1.0e-3 * std::exp(-0.02 * Tc);
}

double Agua::conductividad_termica(double T, double P) {
    // Aproximación para líquido [W/(m·K)]
    double Tc = T - 273.15;
    return 0.56 + 0.0018 * Tc;
}

}} // namespace
