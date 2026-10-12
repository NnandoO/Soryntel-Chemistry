#include "motores/equilibrio_fases.hpp"
#include <algorithm>
#include <cmath>

namespace cma { namespace motores {

double EquilibrioFases::presion_vapor(const nucleo::Especie& sp, double T_K) const {
    return sp.presion_vapor(T_K);
}

double EquilibrioFases::coeficiente_K(const nucleo::Especie& sp, double T_K, double P_atm) const {
    double Pv = presion_vapor(sp, T_K);
    if (Pv <= 0 || P_atm <= 0) return 0;
    return Pv / P_atm;
}

double EquilibrioFases::clausius_clapeyron(double P_ref, double H_vap, double T_ref, double T_K) const {
    if (T_K <= 0 || T_ref <= 0) return 0;
    return P_ref * std::exp(-(H_vap * 1000.0 / nucleo::R_GAS) * (1.0/T_K - 1.0/T_ref));
}

PuntoBurbuja EquilibrioFases::punto_burbuja(
    const std::map<int64_t, double>& x_comp,
    double P_atm,
    const std::map<int64_t, nucleo::Especie>& db) const
{
    PuntoBurbuja res;
    if (x_comp.empty()) { res.mensaje = "Composición vacía"; return res; }

    // Estimate initial temperature (average boiling points)
    double T_est = 0.0, total_x = 0.0;
    for (auto& [sid, x] : x_comp) {
        auto it = db.find(sid);
        if (it != db.end()) {
            // Crude estimate: P_vap = P_tot => Antoine backward
            double A = it->second.antoine_a, B = it->second.antoine_b, C = it->second.antoine_c;
            if (A > 0 && P_atm > 0) {
                double logP = std::log10(P_atm * 760);
                if (std::abs(A - logP) > 1e-10) {
                    double T_C = B / (A - logP) - C;
                    T_est += (T_C + nucleo::CERO_KELVIN) * x;
                }
            }
            total_x += x;
        }
    }
    if (total_x <= 0) total_x = 1;
    T_est = T_est / total_x;
    if (T_est < 200) T_est = 350;

    double T = T_est;
    for (int iter = 0; iter < MAX_ITER; ++iter) {
        double sum_Kx = 0.0;
        for (auto& [sid, x] : x_comp) {
            auto it = db.find(sid);
            if (it != db.end()) {
                double K = coeficiente_K(it->second, T, P_atm);
                sum_Kx += K * x;
            }
        }

        double f = sum_Kx - 1.0;
        if (std::abs(f) < TOL) {
            res.convergio = true;
            break;
        }

        // Numerical derivative
        double dT = 0.001 * T;
        double sum_Kx_dT = 0.0;
        for (auto& [sid, x] : x_comp) {
            auto it = db.find(sid);
            if (it != db.end()) {
                double K = coeficiente_K(it->second, T + dT, P_atm);
                sum_Kx_dT += K * x;
            }
        }
        double df = (sum_Kx_dT - sum_Kx) / dT;
        if (std::abs(df) < 1e-15) break;
        T -= (sum_Kx - 1.0) / df;
        if (T < 100) T = 100;
        if (T > 2000) T = 2000;
    }

    res.temperatura = T;
    res.presion = P_atm;
    if (res.convergio) {
        for (auto& [sid, x] : x_comp) {
            auto it = db.find(sid);
            if (it != db.end())
                res.y_composicion[sid] = coeficiente_K(it->second, T, P_atm) * x;
        }
        res.mensaje = "Punto de burbuja calculado a T=" + std::to_string(T) + " K";
    } else {
        res.mensaje = "No convergió después de " + std::to_string(MAX_ITER) + " iteraciones";
    }
    return res;
}

PuntoRocio EquilibrioFases::punto_rocio(
    const std::map<int64_t, double>& y_comp,
    double P_atm,
    const std::map<int64_t, nucleo::Especie>& db) const
{
    PuntoRocio res;
    if (y_comp.empty()) { res.mensaje = "Composición vacía"; return res; }

    double T = 0.0, total_y = 0.0;
    for (auto& [sid, y] : y_comp) {
        auto it = db.find(sid);
        if (it != db.end()) {
            double A = it->second.antoine_a, B = it->second.antoine_b, C = it->second.antoine_c;
            if (A > 0 && P_atm > 0) {
                double logP = std::log10(P_atm * 760);
                if (std::abs(A - logP) > 1e-10) {
                    T += (B / (A - logP) - C + nucleo::CERO_KELVIN) * y;
                }
            }
            total_y += y;
        }
    }
    if (total_y <= 0) total_y = 1;
    T = T / total_y;
    if (T < 50.0) T = 400.0; // Fallback
    for (int iter = 0; iter < MAX_ITER; ++iter) {
        double sum_y_K = 0.0;
        for (auto& [sid, y] : y_comp) {
            auto it = db.find(sid);
            if (it != db.end()) {
                double K = coeficiente_K(it->second, T, P_atm);
                if (K > 1e-15) sum_y_K += y / K;
            }
        }

        double f = sum_y_K - 1.0;
        if (std::abs(f) < TOL) {
            res.convergio = true;
            break;
        }

        double dT = 0.001 * T;
        double sum_y_K_dT = 0.0;
        for (auto& [sid, y] : y_comp) {
            auto it = db.find(sid);
            if (it != db.end()) {
                double K = coeficiente_K(it->second, T + dT, P_atm);
                if (K > 1e-15) sum_y_K_dT += y / K;
            }
        }
        double df = (sum_y_K_dT - sum_y_K) / dT;
        if (std::abs(df) < 1e-15) break;
        T -= (sum_y_K - 1.0) / df;
        if (T < 100) T = 100;
        if (T > 2000) T = 2000;
    }

    res.temperatura = T;
    res.presion = P_atm;
    if (res.convergio) {
        for (auto& [sid, y] : y_comp) {
            auto it = db.find(sid);
            if (it != db.end()) {
                double K = coeficiente_K(it->second, T, P_atm);
                res.x_composicion[sid] = (K > 1e-15) ? (y / K) : 0.0;
            }
        }
        res.mensaje = "Punto de rocío calculado a T=" + std::to_string(T) + " K";
    } else {
        res.mensaje = "No convergió";
    }
    return res;
}

ResultadoFlash EquilibrioFases::flash_isotermico(
    const nucleo::Flujo& alimentacion,
    double T_K, double P_atm,
    const std::map<int64_t, nucleo::Especie>& db) const
{
    ResultadoFlash res;
    res.T = T_K; res.P = P_atm;

    if (alimentacion.flujo_total() <= 0) { res.mensaje = "Flujo vacío"; return res; }

    std::vector<int64_t> species_ids;
    for (auto& [sid, _] : alimentacion.flujo_molar) species_ids.push_back(sid);

    // Rachford-Rice: solve Σ (z_i * (K_i - 1)) / (1 + ψ * (K_i - 1)) = 0
    double f0 = 0.0, f1 = 0.0;
    for (int64_t sid : species_ids) {
        auto it = db.find(sid);
        double z = alimentacion.flujo_especie(sid) / alimentacion.flujo_total();
        if (z <= 0) continue;
        double K = (it != db.end()) ? coeficiente_K(it->second, T_K, P_atm) : 1.0;
        f0 += z * (K - 1.0);
        f1 += z * (K - 1.0) / std::max(K, 1e-15);
    }
    
    double psi = 0.5; // V/F guess
    if (f0 < 0.0) { psi = 0.0; res.convergio = true; } // Líquido subenfriado
    else if (f1 > 0.0) { psi = 1.0; res.convergio = true; } // Vapor sobrecalentado
    else {
        for (int iter = 0; iter < MAX_ITER; ++iter) {
            double sum_f = 0.0, sum_df = 0.0;
            for (int64_t sid : species_ids) {
                auto it = db.find(sid);
                double z = alimentacion.flujo_especie(sid) / alimentacion.flujo_total();
                if (z <= 0) continue;
                double K = (it != db.end()) ? coeficiente_K(it->second, T_K, P_atm) : 1.0;
                double denom = 1.0 + psi * (K - 1.0);
                if (std::abs(denom) < 1e-15) denom = (denom < 0) ? -1e-15 : 1e-15;
                sum_f += z * (K - 1.0) / denom;
                sum_df -= z * (K - 1.0) * (K - 1.0) / (denom * denom);
            }

            if (std::abs(sum_f) < TOL) { res.convergio = true; break; }
            if (std::abs(sum_df) < 1e-15) break;
            psi -= sum_f / sum_df;
            psi = std::clamp(psi, 0.0, 1.0);
        }
    }

    res.fraccion_vapor = psi;
    if (res.convergio) {
        for (int64_t sid : species_ids) {
            auto it = db.find(sid);
            double z = alimentacion.flujo_especie(sid) / alimentacion.flujo_total();
            double K = (it != db.end()) ? coeficiente_K(it->second, T_K, P_atm) : 1.0;
            double denom = 1.0 + psi * (K - 1.0);
            if (std::abs(denom) < 1e-15) continue;
            res.x_liquido[sid] = z / denom;
            res.y_vapor[sid] = K * z / denom;
        }
        res.mensaje = "Flash calculado: V/F = " + std::to_string(psi);
    } else {
        res.mensaje = "Flash no convergió";
    }
    return res;
}

double EquilibrioFases::coeficiente_actividad(double x, double T_K, double A) const {
    if (A == 0 || T_K <= 0.0) return 1.0;
    // Van Laar / Margules simplificado
    return std::exp(A * (1.0 - x) * (1.0 - x) / (nucleo::R_GAS * T_K / 1000.0));
}

}} // namespace
