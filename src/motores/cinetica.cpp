#include "motores/cinetica.hpp"
#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace cma { namespace motores {

double Cinetica::arrhenius(double A, double Ea_kJ, double T_K) const {
    if (T_K <= 0) return 0;
    return A * std::exp(-Ea_kJ * 1000.0 / (nucleo::R_GAS * T_K));
}

double Cinetica::velocidad_reaccion(double k, const std::vector<double>& concs,
                                     const std::vector<int>& ordenes) const {
    if (concs.size() != ordenes.size()) return 0;
    double r = k;
    for (size_t i = 0; i < concs.size(); ++i)
        r *= std::pow(concs[i], ordenes[i]);
    return r;
}

ResultadoCinetica Cinetica::reactor_batch(double C0, int orden, double k,
                                           double tiempo, double Cb0) const {
    ResultadoCinetica res;
    res.tiempo = tiempo;

    if (k <= 0) { res.mensaje = "k debe ser > 0"; return res; }

    double Cf = C0;

    switch (orden) {
        case 0:
            Cf = C0 - k * tiempo;
            res.velocidad = k;
            break;
        case 1:
            Cf = C0 * std::exp(-k * tiempo);
            res.velocidad = k * Cf;
            break;
        case 2: {
            if (Cb0 > 0 && std::abs(Cb0 - C0) > 1e-10) {
                // Second order A + B → P, different initial concentrations
                // [A](t) = C0*(M-1) / (M*exp(C0*(M-1)*k*t) - 1), M = Cb0/C0
                double M = Cb0 / C0;
                double exp_arg = C0 * (M - 1.0) * k * tiempo;
                double denom = M * std::exp(exp_arg) - 1.0;
                if (std::abs(denom) < 1e-15) Cf = 0;
                else Cf = C0 * (M - 1.0) / denom;
            } else {
                Cf = C0 / (1.0 + k * tiempo * C0);
            }
            res.velocidad = k * Cf * Cf;
            break;
        }
        default:
            res.mensaje = "Orden no soportado: " + std::to_string(orden);
            return res;
    }

    if (Cf < 0) Cf = 0;
    res.concentracion_final[0] = Cf;
    res.conversion = (C0 > 0) ? (C0 - Cf) / C0 : 0;
    res.mensaje = "Batch: C_f = " + std::to_string(Cf) + " mol/L, X = " + std::to_string(res.conversion);
    return res;
}

ResultadoCinetica Cinetica::reactor_cstr(double C0, int orden, double k, double tau, double Cb0) const {
    ResultadoCinetica res;
    res.tiempo = tau;

    if (k <= 0 || tau <= 0) { res.mensaje = "k y tau deben ser > 0"; return res; }

    double Cf = C0;

    switch (orden) {
        case 0:
            Cf = C0 - k * tau;
            break;
        case 1:
            Cf = C0 / (1.0 + k * tau);
            break;
        case 2: {
            // Cf = (-1 + sqrt(1 + 4*k*tau*C0)) / (2*k*tau)
            double disc = 1.0 + 4.0 * k * tau * C0;
            if (disc < 0) { res.mensaje = "Discriminante negativo"; return res; }
            Cf = (-1.0 + std::sqrt(disc)) / (2.0 * k * tau);
            break;
        }
        default:
            res.mensaje = "Orden no soportado";
            return res;
    }

    if (Cf < 0) Cf = 0;
    res.concentracion_final[0] = Cf;
    res.conversion = (C0 > 0) ? (C0 - Cf) / C0 : 0;
    res.mensaje = "CSTR: C_f = " + std::to_string(Cf) + " mol/L, X = " + std::to_string(res.conversion);
    return res;
}

ResultadoCinetica Cinetica::reactor_pfr(double C0, int orden, double k, double tau, double Cb0) const {
    // PFR tiene la misma ecuación que Batch (misma ecuación de diseño)
    return reactor_batch(C0, orden, k, tau, Cb0);
}

double Cinetica::vida_media(double k, int orden, double C0) const {
    if (k <= 0) return 0;
    switch (orden) {
        case 0: return C0 / (2.0 * k);
        case 1: return std::log(2.0) / k;
        case 2: return 1.0 / (k * C0);
        default: return 0;
    }
}

double Cinetica::conversion_desde_C(double C0, double C) const {
    if (C0 <= 0) return 0;
    return (C0 - C) / C0;
}

}} // namespace
