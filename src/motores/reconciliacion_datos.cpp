#include "reconciliacion_datos.hpp"
#include <cmath>
#include <algorithm>
#include <sstream>
#include <set>

namespace cma { namespace motores {

namespace {
bool validar(const std::vector<Medicion>& a, const std::vector<Medicion>& b) {
    std::set<std::string> names;
    for (const auto* group : {&a, &b}) for (const auto& m : *group) {
        if (m.nombre.empty() || !names.insert(m.nombre).second || !std::isfinite(m.valor_medido) ||
            !std::isfinite(m.incertidumbre) || m.incertidumbre < 0) return false;
    }
    return !a.empty() && !b.empty();
}
// Regularized incomplete beta via continued fractions, used for Student's t.
double beta_fraction(double a, double b, double x) {
    const double tiny = 1e-300;
    double c = 1, d = 1 - (a + b) * x / (a + 1);
    if (std::abs(d) < tiny) d = tiny;
    d = 1 / d;
    double h = d;
    for (int m = 1; m <= 300; ++m) {
        const double mm = m;
        double aa = mm * (b - mm) * x / ((a + 2 * mm - 1) * (a + 2 * mm));
        d = 1 + aa * d; if (std::abs(d) < tiny) d = tiny;
        c = 1 + aa / c; if (std::abs(c) < tiny) c = tiny;
        d = 1 / d; h *= d * c;
        aa = -(a + mm) * (a + b + mm) * x / ((a + 2 * mm) * (a + 2 * mm + 1));
        d = 1 + aa * d; if (std::abs(d) < tiny) d = tiny;
        c = 1 + aa / c; if (std::abs(c) < tiny) c = tiny;
        d = 1 / d;
        const double delta = d * c; h *= delta;
        if (std::abs(delta - 1) < 1e-13) break;
    }
    return h;
}
double regularized_beta(double x, double a, double b) {
    if (x <= 0) return 0;
    if (x >= 1) return 1;
    const double factor = std::exp(std::lgamma(a + b) - std::lgamma(a) - std::lgamma(b) +
        a * std::log(x) + b * std::log1p(-x));
    return x < (a + 1) / (a + b + 2) ? factor * beta_fraction(a, b, x) / a
        : 1 - factor * beta_fraction(b, a, 1 - x) / b;
}
double student_quantile(double upper_tail, double degrees) {
    auto tail = [&](double t) { return 0.5 * regularized_beta(degrees / (degrees + t * t), degrees / 2, 0.5); };
    double lo = 0, hi = 1;
    while (tail(hi) > upper_tail && hi < 1e12) hi *= 2;
    for (int i = 0; i < 100; ++i) {
        const double mid = (lo + hi) / 2;
        if (tail(mid) > upper_tail) lo = mid; else hi = mid;
    }
    return (lo + hi) / 2;
}
}

double ReconciliacionDatos::media(const std::vector<double>& v) {
    if (v.empty()) return 0;
    double s = 0;
    for (auto& x : v) s += x;
    return s / v.size();
}

double ReconciliacionDatos::desviacion(const std::vector<double>& v, double m) {
    if (v.size() < 2) return 0;
    double s = 0;
    for (auto& x : v) s += (x - m) * (x - m);
    return std::sqrt(s / (v.size() - 1));
}

ResultadoReconciliacion ReconciliacionDatos::reconciliar_balance_masa(
    const std::vector<Medicion>& entradas,
    const std::vector<Medicion>& salidas,
    double tolerancia, int max_iter)
{
    ResultadoReconciliacion res;
    error_.clear();
    if (!validar(entradas, salidas) || !std::isfinite(tolerancia) || tolerancia <= 0 || max_iter < 1) {
        error_ = res.mensaje = "Mediciones o tolerancia inválidas"; return res;
    }
    double sum_ent = 0, sum_sal = 0;
    double var_ent = 0, var_sal = 0;

    for (auto& m : entradas) {
        sum_ent += m.valor_medido;
        var_ent += m.incertidumbre * m.incertidumbre;
    }
    for (auto& m : salidas) {
        sum_sal += m.valor_medido;
        var_sal += m.incertidumbre * m.incertidumbre;
    }

    double desbalance = sum_ent - sum_sal;
    double var_total = var_ent + var_sal;

    if (var_total < 1e-15) {
        error_ = "Varianza total cero";
        res.mensaje = error_;
        return res;
    }

    // Factor de corrección (mínimos cuadrados ponderados)
    double lambda = desbalance / var_total;

    for (auto& m : entradas) {
        double corr = m.valor_medido - lambda * m.incertidumbre * m.incertidumbre;
        res.valores_reconciliados[m.nombre] = corr;
        res.residuos[m.nombre] = corr - m.valor_medido;
    }
    for (auto& m : salidas) {
        double corr = m.valor_medido + lambda * m.incertidumbre * m.incertidumbre;
        res.valores_reconciliados[m.nombre] = corr;
        res.residuos[m.nombre] = corr - m.valor_medido;
    }

    res.chi_cuadrado = lambda * lambda * var_total;
    res.q_ajuste = std::erfc(std::sqrt(res.chi_cuadrado / 2.0)); // chi-square survival, 1 constraint
    double residual = 0;
    for (const auto& m : entradas) residual += res.valores_reconciliados.at(m.nombre);
    for (const auto& m : salidas) residual -= res.valores_reconciliados.at(m.nombre);
    if (!std::isfinite(residual) || std::abs(residual) > tolerancia * std::max({1.0, std::abs(sum_ent), std::abs(sum_sal)})) {
        res.mensaje = "Balance reconciliado excede tolerancia"; return res;
    }
    res.ok = true;
    res.mensaje = "Reconciliación completada. χ²=" + std::to_string(res.chi_cuadrado);

    return res;
}

ResultadoReconciliacion ReconciliacionDatos::reconciliar_balance_energia(
    const std::vector<Medicion>& corr_entrada,
    const std::vector<Medicion>& corr_salida,
    const std::vector<Medicion>& temperaturas,
    double tolerancia)
{
    ResultadoReconciliacion res;
    error_.clear();
    if (!validar(corr_entrada, corr_salida) || !std::isfinite(tolerancia) || tolerancia <= 0) {
        error_ = res.mensaje = "Mediciones o tolerancia inválidas"; return res;
    }
    std::set<std::string> names;
    for (const auto* group : {&corr_entrada, &corr_salida, &temperaturas}) for (const auto& m : *group) {
        if (!names.insert(m.nombre).second || !std::isfinite(m.valor_medido) || !std::isfinite(m.incertidumbre) || m.incertidumbre < 0) {
            res.mensaje = "Mediciones duplicadas o inválidas"; return res;
        }
    }
    double sum_ent = 0, sum_sal = 0;
    double var_ent = 0, var_sal = 0;

    for (auto& m : corr_entrada) {
        sum_ent += m.valor_medido;
        var_ent += m.incertidumbre * m.incertidumbre;
    }
    for (auto& m : corr_salida) {
        sum_sal += m.valor_medido;
        var_sal += m.incertidumbre * m.incertidumbre;
    }

    double desbalance = sum_ent - sum_sal;
    double var_total = var_ent + var_sal;

    if (var_total < 1e-15) {
        error_ = "Varianza total cero";
        res.mensaje = error_;
        return res;
    }

    double lambda = desbalance / var_total;

    for (auto& m : corr_entrada) {
        double corr = m.valor_medido - lambda * m.incertidumbre * m.incertidumbre;
        res.valores_reconciliados[m.nombre] = corr;
        res.residuos[m.nombre] = corr - m.valor_medido;
    }
    for (auto& m : corr_salida) {
        double corr = m.valor_medido + lambda * m.incertidumbre * m.incertidumbre;
        res.valores_reconciliados[m.nombre] = corr;
        res.residuos[m.nombre] = corr - m.valor_medido;
    }
    
    // Las temperaturas se dejan tal cual en este modelo simple no acoplado
    for (auto& m : temperaturas) {
        res.valores_reconciliados[m.nombre] = m.valor_medido;
        res.residuos[m.nombre] = 0.0;
    }

    res.chi_cuadrado = lambda * lambda * var_total;
    res.q_ajuste = std::erfc(std::sqrt(res.chi_cuadrado / 2.0));
    res.ok = true;
    res.mensaje = "Reconciliación de energía completada. χ²=" + std::to_string(res.chi_cuadrado);

    return res;
}

std::vector<int> ReconciliacionDatos::detectar_outliers(
    const std::vector<Medicion>& datos, double alpha)
{
    std::vector<int> outliers;
    if (datos.size() < 3 || !std::isfinite(alpha) || alpha <= 0 || alpha >= 1) return outliers;
    for (const auto& m : datos) if (!std::isfinite(m.valor_medido)) return outliers;

    std::vector<double> vals;
    for (auto& m : datos) vals.push_back(m.valor_medido);

    double med = media(vals);
    double desv = desviacion(vals, med);
    if (desv < 1e-15) return outliers;

    const double N = static_cast<double>(vals.size());
    const double t = student_quantile(alpha / (2 * N), N - 2);
    const double critical = (N - 1) / std::sqrt(N) * std::sqrt(t * t / (N - 2 + t * t));
    for (size_t i = 0; i < vals.size(); ++i) {
        double G = std::abs(vals[i] - med) / desv;
        if (G > critical) outliers.push_back(static_cast<int>(i));
    }
    return outliers;
}

}} // namespace
