#include "motores/destilacion.hpp"
#include <algorithm>
#include <cmath>

namespace cma { namespace motores {

double Destilacion::fenske(double xD, double xB, double alfa) const {
    if (alfa <= 1.0) return 0;
    // Validar que las fracciones estén en rango válido
    if (xD <= 0.0 || xD >= 1.0) return 0;
    if (xB <= 0.0 || xB >= 1.0) return 0;
    // N_min = log[(xD/(1-xD)) / (xB/(1-xB))] / log(alfa)
    double ratio = (xD / (1.0 - xD)) / (xB / (1.0 - xB));
    if (ratio <= 0) return 0;
    return std::log(ratio) / std::log(alfa);
}

double Destilacion::underwood_min_reflujo(double alfa, double xF, double xD, double q) const {
    // Para q=1 (alimentación líquida saturada): R_min = [xD/xF - alfa*(1-xD)/(1-xF)] / (alfa - 1)
    if (std::abs(q - 1.0) < 1e-10) {
        double num = xD / std::max(xF, 1e-15) - alfa * (1.0 - xD) / std::max(1.0 - xF, 1e-15);
        double den = alfa - 1.0;
        if (std::abs(den) < 1e-15) return 0;
        return std::max(0.0, num / den);
    }
    // Para q=0 (alimentación vapor saturado): R_min = (alfa*xD/xF - (1-xD)/(1-xF)) / (alfa-1) - 1
    if (std::abs(q) < 1e-10) {
        double num = alfa * xD / std::max(xF, 1e-15) - (1.0 - xD) / std::max(1.0 - xF, 1e-15);
        double den = alfa - 1.0;
        if (std::abs(den) < 1e-15) return 0;
        return std::max(0.0, num / den - 1.0);
    }
    // Caso general binario: resolver Σ α_i*z_i/(α_i - θ) = 1 - q
    // Binario: alfa*xF/(alfa-θ) + (1-xF)/(1-θ) = 1-q
    // Resolver por bisección en θ ∈ (1, alfa) si alfa > 1
    if (alfa <= 1.0) return 0;
    double theta_lo = 1.0 + 1e-6;
    double theta_hi = alfa - 1e-6;
    auto f_theta = [&](double theta) {
        return alfa * xF / (alfa - theta) + (1.0 - xF) / (1.0 - theta) - (1.0 - q);
    };
    double f_lo = f_theta(theta_lo);
    double f_hi = f_theta(theta_hi);
    // Verificar que hay cambio de signo
    if (f_lo * f_hi > 0) return 0;
    for (int iter = 0; iter < 100; ++iter) {
        double theta_mid = (theta_lo + theta_hi) / 2.0;
        double f_mid = f_theta(theta_mid);
        if (std::abs(f_mid) < 1e-10) break;
        if (f_lo * f_mid < 0) { theta_hi = theta_mid; f_hi = f_mid; }
        else { theta_lo = theta_mid; f_lo = f_mid; }
    }
    double theta = (theta_lo + theta_hi) / 2.0;
    // R_min = α*xD/(α-θ) + (1-xD)/(1-θ) - 1
    double R_min = alfa * xD / (alfa - theta) + (1.0 - xD) / (1.0 - theta) - 1.0;
    return std::max(0.0, R_min);
}

int Destilacion::gilliland(int N_min, double R_min, double R_op) const {
    if (R_op <= R_min) return N_min * 2;
    double X = (R_op - R_min) / (R_op + 1.0);
    // Correlación de Molokanov (1972): Y = 1 - exp[-(1+49.5X^0.98+0.5X)/(1+19.5X^0.98+0.5X)]
    double Xp = std::pow(X, 0.98);
    double Y = 1.0 - std::exp(-(1.0 + 49.5 * Xp + 0.5 * X) / (1.0 + 19.5 * Xp + 0.5 * X));
    // Gilliland: Y = (N - N_min)/(N + 1)  =>  N = (N_min + Y)/(1 - Y)
    if (Y >= 1.0) return N_min * 2;
    int N = (int)std::ceil((N_min + Y) / (1.0 - Y));
    return std::max(N_min + 1, N);
}

double Destilacion::volatilidad_relativa(double K1, double K2) const {
    if (K2 <= 0) return 1;
    return K1 / K2;
}

ResultadoDestilacion Destilacion::mccabe_thiele(
    double zF, double xD, double xB, double alfa, double q, double R_op) const
{
    ResultadoDestilacion res;

    // Línea de alimentación: y = [q/(q-1)]*x - zF/(q-1)
    double denom_q = q - 1.0;
    if (std::abs(denom_q) < 1e-15) denom_q = (q >= 1.0) ? 1e-15 : -1e-15;
    double pend_q = q / denom_q;
    double intercept_q = -zF / denom_q;

    // Línea de rectificación: y = [R/(R+1)]*x + xD/(R+1)
    double pend_r = R_op / (R_op + 1.0);
    double intercept_r = xD / (R_op + 1.0);

    // Línea de agotamiento: y = [(R*zF + xD - (R+1)*xB)/(R+1)] * x / (zF - xB) ...
    // Simplificada: pasa por (xB, xB) con pendiente relacionada
    double pend_s = (R_op * zF + xD - (R_op + 1.0) * xB) / ((R_op + 1.0) * (zF - xB));

    // Reflujo mínimo (intersección de líneas de alimentación y rectificación)
    double x_int = (intercept_r - intercept_q) / (pend_q - pend_r);
    double y_int = pend_r * x_int + intercept_r;
    res.R_min = (xD - y_int) / (y_int - x_int);
    if (res.R_min < 0) res.R_min = 0;
    res.R_operacion = R_op;

    // Fenske: N_min
    res.N_min = (int)std::ceil(fenske(xD, xB, alfa));

    // Gilliland: etapas reales
    res.N_etapas = gilliland(res.N_min, res.R_min, R_op);

    // Flujos (balances globales)
    double F_total = 1.0; // base de cálculo
    res.destilado = (zF - xB) / (xD - xB) * F_total;
    if (res.destilado < 0) res.destilado = 0;
    res.residuo = F_total - res.destilado;

    res.x_destilado[0] = xD;
    res.x_residuo[0] = xB;

    res.mensaje = "McCabe-Thiele: N_min=" + std::to_string(res.N_min) +
                  ", R_min=" + std::to_string(res.R_min) +
                  ", N_real=" + std::to_string(res.N_etapas);
    return res;
}

ResultadoDestilacion Destilacion::fenske_underwood_gilliland(
    const nucleo::Flujo& alimentacion,
    const std::vector<int64_t>& claves_livianas,
    const std::vector<int64_t>& claves_pesadas,
    double recup_liv_dest,
    double recup_pes_res,
    double R_op,
    const std::map<int64_t, nucleo::Especie>& db) const
{
    ResultadoDestilacion res;
    // Placeholder para implementación multicomponente completa
    res.mensaje = "FUG: método shortcut multicomponente";
    res.R_operacion = R_op;
    return res;
}

}} // namespace
