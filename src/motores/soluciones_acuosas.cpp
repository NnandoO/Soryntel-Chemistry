#include "soluciones_acuosas.hpp"
#include <cmath>
#include <sstream>
#include <limits>

namespace cma { namespace motores {

double SolucionesAcuosas::coeficiente_osmótico(double I, double T) {
    (void)I; (void)T;
    error_="Coeficiente osmótico requiere parámetros de interacción";
    return std::numeric_limits<double>::quiet_NaN();
}

double SolucionesAcuosas::coeficiente_actividad_davies(double z, double I, double T) {
    if (!std::isfinite(z) || !std::isfinite(I) || I<0 || I>.5 || !std::isfinite(T) || std::abs(T-298.15)>1e-8) {
        error_="Davies: requiere 298.15 K y 0 <= I <= 0.5 mol/kg";
        return std::numeric_limits<double>::quiet_NaN();
    }
    error_.clear();
    // Modelo de Davies estándar
    double A = 0.509 * std::pow(298.15 / T, 1.5);
    double log_gamma = -A * z * z * (std::sqrt(I) / (1.0 + std::sqrt(I)) - 0.3 * I);
    return std::pow(10.0, log_gamma);
}

double SolucionesAcuosas::coeficiente_actividad_pitzer(double z, double I, double T) {
    (void)z; (void)I; (void)T;
    return std::numeric_limits<double>::quiet_NaN(); // Missing interaction parameters.
}

double SolucionesAcuosas::coeficiente_actividad_harvie(double z, double I, double T) {
    return coeficiente_actividad_pitzer(z,I,T);
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
    double, const std::map<std::string, double>& iones,
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
