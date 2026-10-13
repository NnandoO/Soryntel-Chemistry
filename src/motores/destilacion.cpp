#include "motores/destilacion.hpp"
#include <algorithm>
#include <cmath>

namespace cma { namespace motores {

double Destilacion::fenske(double xD, double xB, double alfa) const {
    if (!std::isfinite(xD) || !std::isfinite(xB) || !std::isfinite(alfa) || xD <= xB) return 0;
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
    if (!std::isfinite(alfa) || !std::isfinite(xF) || !std::isfinite(xD) || !std::isfinite(q) ||
        alfa <= 1 || xF <= 0 || xF >= 1 || xD <= xF || xD >= 1 || q < 0 || q > 1) return 0;
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
    if (N_min < 1 || !std::isfinite(R_min) || !std::isfinite(R_op) || R_min < 0 || R_op <= R_min) return 0;
    double X = (R_op - R_min) / (R_op + 1.0);
    // Eduljee approximation to Gilliland; Y approaches zero at total reflux.
    double Y = 1.0 - std::exp(((1 + 54.4 * X) / (11 + 117.2 * X)) * ((X - 1) / std::sqrt(X)));
    // Gilliland: Y = (N - N_min)/(N + 1)  =>  N = (N_min + Y)/(1 - Y)
    if (Y >= 1.0 || (N_min + Y) / (1 - Y) > 1000000) return 0;
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
    if (!std::isfinite(zF) || !std::isfinite(xD) || !std::isfinite(xB) ||
        !std::isfinite(alfa) || !std::isfinite(q) || !std::isfinite(R_op) ||
        !(0 < xB && xB < zF && zF < xD && xD < 1) || alfa <= 1 || q < 0 || q > 1 || R_op < 0) {
        res.mensaje = "Destilación inválida: requiere 0 < xB < zF < xD < 1, alfa > 1 y 0 <= q <= 1";
        return res;
    }
    res.R_min = underwood_min_reflujo(alfa, zF, xD, q);
    if (R_op <= res.R_min) { res.mensaje = "Reflujo de operación debe superar el mínimo"; return res; }
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
    res.ok = res.N_etapas > 0;

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
    res.R_operacion = R_op;
    if (claves_livianas.size() != 1 || claves_pesadas.size() != 1 ||
        claves_livianas[0] == claves_pesadas[0] ||
        !std::isfinite(recup_liv_dest) || !std::isfinite(recup_pes_res) ||
        recup_liv_dest <= 0 || recup_liv_dest >= 1 || recup_pes_res <= 0 || recup_pes_res >= 1 ||
        !std::isfinite(R_op) || R_op < 0 || alimentacion.flujo_total() <= 0) {
        res.mensaje = "FUG requiere una clave liviana y una pesada, recuperaciones en (0,1) y alimentación positiva";
        return res;
    }
    const auto lk = claves_livianas[0], hk = claves_pesadas[0];
    if (!db.count(lk) || !db.count(hk) || alimentacion.flujo_especie(lk) <= 0 || alimentacion.flujo_especie(hk) <= 0) {
        res.mensaje = "Claves desconocidas o ausentes en alimentación"; return res;
    }
    EquilibrioFases equilibrio;
    auto bubble = equilibrio.punto_burbuja(alimentacion.fracciones_molares(), alimentacion.presion, db);
    if (!bubble.convergio) { res.mensaje = "No se pudo estimar temperatura de burbuja para FUG"; return res; }
    const double kh = equilibrio.coeficiente_K(db.at(hk), bubble.temperatura, alimentacion.presion);
    const double kl = equilibrio.coeficiente_K(db.at(lk), bubble.temperatura, alimentacion.presion);
    if (kh <= 0 || kl <= kh) { res.mensaje = "Volatilidades de claves inválidas"; return res; }
    const double alpha_lk = kl / kh;
    const double odds_h = (1 - recup_pes_res) / recup_pes_res;
    const double odds_l = recup_liv_dest / (1 - recup_liv_dest);
    const double stages = std::log(odds_l / odds_h) / std::log(alpha_lk);
    if (!std::isfinite(stages) || stages <= 0 || stages > 1000000) {
        res.mensaje = "Recuperaciones incompatibles con separación"; return res;
    }
    std::map<int64_t, double> alpha, d, b;
    for (const auto& [sid, n] : alimentacion.flujo_molar) {
        if (!db.count(sid) || !std::isfinite(n) || n < 0) { res.mensaje = "Alimentación inválida"; return res; }
        const double a = equilibrio.coeficiente_K(db.at(sid), bubble.temperatura, alimentacion.presion) / kh;
        if (!std::isfinite(a) || a <= 0) { res.mensaje = "FUG requiere coeficientes de vapor para todas las especies"; return res; }
        alpha[sid] = a;
        const double log_odds = std::log(odds_h) + stages * std::log(a);
        const double recovery = log_odds >= 0 ? 1 / (1 + std::exp(-log_odds)) : std::exp(log_odds) / (1 + std::exp(log_odds));
        d[sid] = n * recovery; b[sid] = n - d[sid];
        res.destilado += d[sid]; res.residuo += b[sid];
    }
    if (res.destilado <= 0 || res.residuo <= 0) { res.mensaje = "Separación degenerada"; return res; }
    for (const auto& [sid, n] : d) res.x_destilado[sid] = n / res.destilado;
    for (const auto& [sid, n] : b) res.x_residuo[sid] = n / res.residuo;
    const auto z = alimentacion.fracciones_molares();
    auto equation = [&](double theta) {
        double value = 0;
        for (const auto& [sid, a] : alpha) value += a * z.at(sid) / (a - theta);
        return value;
    };
    // This shortcut supports adjacent keys and saturated liquid feed (q=1).
    for (const auto& [sid, a] : alpha) {
        if (sid != lk && sid != hk && a > 1 && a < alpha_lk) {
            res.mensaje = "FUG requiere claves adyacentes en volatilidad"; return res;
        }
    }
    double lo = 1 + (alpha_lk - 1) * 1e-9, hi = alpha_lk - (alpha_lk - 1) * 1e-9;
    for (int i = 0; i < 100; ++i) {
        const double mid = (lo + hi) / 2;
        if (equation(mid) < 0) lo = mid; else hi = mid;
    }
    const double theta = (lo + hi) / 2;
    res.R_min = -1;
    for (const auto& [sid, a] : alpha) res.R_min += a * res.x_destilado.at(sid) / (a - theta);
    res.R_min = std::max(0.0, res.R_min);
    res.N_min = static_cast<int>(std::ceil(stages));
    res.N_etapas = gilliland(res.N_min, res.R_min, R_op);
    res.ok = res.N_etapas > 0;
    res.mensaje = res.ok ? "FUG ideal, alimentación líquida saturada; volatilidades constantes" : "Reflujo insuficiente para FUG";
    return res;
}

}} // namespace
