#include "motores/electroquimica.hpp"
#include <algorithm>
#include <cmath>
#include <numeric>

namespace cma { namespace motores {

double Electroquimica::nernst(double E0, double T_K, int n, double Q) const {
    if (n <= 0 || Q <= 0) return E0;
    return E0 - (nucleo::R_GAS * T_K / (n * nucleo::FARADAY)) * std::log(Q);
}

PotencialElectrodo Electroquimica::potencial_electrodo(
    double E0, int n,
    const std::map<std::string, double>& concs,
    const std::map<std::string, int>& coeffs,
    double T_K) const
{
    PotencialElectrodo res;
    res.E0 = E0;
    res.semirreaccion = "Custom";

    // Build reaction quotient Q = Π [productos]^ν / Π [reactivos]^ν
    double Q = 1.0;
    for (auto& [sp, c] : concs) {
        auto it = coeffs.find(sp);
        int nu = (it != coeffs.end()) ? it->second : 1;
        if (c > 0) Q *= std::pow(c, nu);
    }

    res.E = nernst(E0, T_K, n, Q);
    res.delta_G = gibbs_desde_potencial(res.E, n);
    res.K_eq = std::exp(-res.delta_G * 1000.0 / (nucleo::R_GAS * T_K));
    res.mensaje = "E = " + std::to_string(res.E) + " V";
    return res;
}

CeldaElectroquimica Electroquimica::potencial_celda(
    const PotencialElectrodo& anodo,
    const PotencialElectrodo& catodo) const
{
    CeldaElectroquimica res;
    res.potencial_celda = catodo.E - anodo.E;
    int n = std::lcm(anodo.n_electrones, catodo.n_electrones);
    res.delta_G = -n * nucleo::FARADAY * res.potencial_celda / 1000.0;
    // K_eq a temperatura de referencia (298.15K) — los potenciales ya están corregidos por T
    res.K_eq = std::exp(-res.delta_G * 1000.0 / (nucleo::R_GAS * nucleo::STP_TEMP));
    res.espontanea = res.potencial_celda > 0;
    res.mensaje = res.espontanea
        ? "Celda espontánea: E_celda = " + std::to_string(res.potencial_celda) + " V"
        : "Celda no espontánea: E_celda = " + std::to_string(res.potencial_celda) + " V";
    return res;
}

double Electroquimica::gibbs_desde_potencial(double E, int n) const {
    return -n * nucleo::FARADAY * E / 1000.0; // kJ
}

double Electroquimica::potencial_desde_gibbs(double dG_kJ, int n) const {
    if (n <= 0) return 0;
    return -dG_kJ * 1000.0 / (n * nucleo::FARADAY);
}

double Electroquimica::potencial_estandar(const std::string& semirreaccion) {
    // Valores comunes vs SHE
    if (semirreaccion == "Li+ + e- -> Li") return -3.04;
    if (semirreaccion == "K+ + e- -> K") return -2.93;
    if (semirreaccion == "Na+ + e- -> Na") return -2.71;
    if (semirreaccion == "Mg2+ + 2e- -> Mg") return -2.37;
    if (semirreaccion == "Al3+ + 3e- -> Al") return -1.66;
    if (semirreaccion == "Zn2+ + 2e- -> Zn") return -0.76;
    if (semirreaccion == "Fe2+ + 2e- -> Fe") return -0.44;
    if (semirreaccion == "Ni2+ + 2e- -> Ni") return -0.25;
    if (semirreaccion == "Sn2+ + 2e- -> Sn") return -0.14;
    if (semirreaccion == "Pb2+ + 2e- -> Pb") return -0.13;
    if (semirreaccion == "2H+ + 2e- -> H2") return 0.00;
    if (semirreaccion == "Cu2+ + 2e- -> Cu") return 0.34;
    if (semirreaccion == "O2 + 2H2O + 4e- -> 4OH-") return 0.40;
    if (semirreaccion == "Cu+ + e- -> Cu") return 0.52;
    if (semirreaccion == "Ag+ + e- -> Ag") return 0.80;
    if (semirreaccion == "Hg2+ + 2e- -> Hg") return 0.85;
    if (semirreaccion == "Br2 + 2e- -> 2Br-") return 1.07;
    if (semirreaccion == "O2 + 4H+ + 4e- -> 2H2O") return 1.23;
    if (semirreaccion == "Cl2 + 2e- -> 2Cl-") return 1.36;
    if (semirreaccion == "Au3+ + 3e- -> Au") return 1.50;
    if (semirreaccion == "F2 + 2e- -> 2F-") return 2.87;
    return 0.0;
}

double Electroquimica::ley_faraday(double I, double t, double M, int n) const {
    if (n <= 0) return 0;
    return (I * t * M) / (n * nucleo::FARADAY); // gramos
}

double Electroquimica::conductividad_electrolito(double conc, double u) const {
    return conc * u * nucleo::FARADAY * 1000.0; // S/m
}

double Electroquimica::ph(double conc_H) const {
    if (conc_H <= 0) return 7.0;
    return -std::log10(conc_H);
}

double Electroquimica::concentracion_h(double ph) const {
    return std::pow(10.0, -ph);
}

}} // namespace
