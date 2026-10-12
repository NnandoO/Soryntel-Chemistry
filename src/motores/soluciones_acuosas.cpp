#include "soluciones_acuosas.hpp"
#include <cmath>
#include <sstream>

namespace cma { namespace motores {

double SolucionesAcuosas::coeficiente_osmótico(double I, double T) {
    // Modelo de Pitzer simplificado
    double A_phi = 0.392 * std::pow(298.15 / T, 1.5);
    return 1.0 - A_phi * std::sqrt(I) / (1.0 + 1.2 * std::sqrt(I));
}

double SolucionesAcuosas::coeficiente_actividad_davies(double z, double I, double T) {
    // Modelo de Davies estándar
    double A = 0.509 * std::pow(298.15 / T, 1.5);
    double log_gamma = -A * z * z * (std::sqrt(I) / (1.0 + std::sqrt(I)) - 0.3 * I);
    return std::pow(10.0, log_gamma);
}

double SolucionesAcuosas::coeficiente_actividad_pitzer(double z, double I, double T) {
    // Modelo de Pitzer (aproximación simplificada base, falta matriz b_ij real)
    double A = 0.392 * std::pow(298.15 / T, 1.5);
    double f_gamma = -A * (std::sqrt(I)/(1.0 + 1.2*std::sqrt(I)) + (2.0/1.2) * std::log(1.0 + 1.2*std::sqrt(I)));
    double log_gamma = z * z * f_gamma; // término de Debye-Huckel
    // Pitzer tiene B_mx e C_mx empíricos, simulamos una pequeña desviación iónica
    log_gamma += 0.05 * I; 
    return std::exp(log_gamma); // Pitzer reporta ln(gamma), usamos exp
}

double SolucionesAcuosas::coeficiente_actividad_harvie(double z, double I, double T) {
    // Modificación de Harvie-Møller-Weare (HMW) sobre Pitzer
    double gamma_pitzer = coeficiente_actividad_pitzer(z, I, T);
    // HMW incluye parámetros ternarios (theta, psi). Simulación de desviación de Harvie
    double result = gamma_pitzer * (1.0 - 0.02 * std::pow(I, 1.2));
    return std::max(result, 0.001);
}

double SolucionesAcuosas::fuerza_ionica(
    const std::map<std::string, double>& concentraciones,
    const std::map<std::string, int>& cargas)
{
    double I = 0;
    for (auto& [ion, conc] : concentraciones) {
        int z = 1;
        auto it = cargas.find(ion);
        if (it != cargas.end()) z = it->second;
        double c = std::max(conc, 0.0);
        I += 0.5 * c * z * z;
    }
    return std::max(I, 0.0);
}

double SolucionesAcuosas::ph(double conc_H, double T) {
    if (conc_H <= 0) return 7.0;
    double Kw = std::pow(10.0, -14.0); // simplificado, depende de T
    double I_approx = conc_H; // Simplification if full I is not provided
    double gamma_H = coeficiente_actividad_davies(1.0, I_approx, T);
    return -std::log10(std::max(gamma_H * conc_H, 1e-30));
}

double SolucionesAcuosas::densidad_solucion(
    double rho_agua, const std::vector<double>& concentraciones,
    const std::vector<double>& masas_molares)
{
    double suma = 0;
    for (size_t i = 0; i < concentraciones.size() && i < masas_molares.size(); ++i)
        suma += concentraciones[i] * masas_molares[i] / 1000.0;
    return rho_agua + suma;
}

double SolucionesAcuosas::conductividad_electrica(
    double T, const std::map<std::string, double>& iones,
    const std::map<std::string, double>& movilidades)
{
    double sigma = 0;
    double F = 96485.0; // constante de Faraday
    for (auto& [ion, conc] : iones) {
        double u = 1e-7; // movilidad por defecto (m²/(V·s))
        auto it = movilidades.find(ion);
        if (it != movilidades.end()) u = it->second;
        sigma += conc * 1000 * F * u; // S/m
    }
    return sigma;
}

}} // namespace
