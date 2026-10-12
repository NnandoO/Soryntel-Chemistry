#include "reconciliacion_datos.hpp"
#include <cmath>
#include <algorithm>
#include <sstream>

namespace cma { namespace motores {

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
    res.q_ajuste = std::exp(-res.chi_cuadrado / 2.0);
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
    res.q_ajuste = std::exp(-res.chi_cuadrado / 2.0);
    res.ok = true;
    res.mensaje = "Reconciliación de energía completada. χ²=" + std::to_string(res.chi_cuadrado);

    return res;
}

std::vector<int> ReconciliacionDatos::detectar_outliers(
    const std::vector<Medicion>& datos, double alpha)
{
    std::vector<int> outliers;
    if (datos.size() < 3) return outliers;

    std::vector<double> vals;
    for (auto& m : datos) vals.push_back(m.valor_medido);

    double med = media(vals);
    double desv = desviacion(vals, med);
    if (desv < 1e-15) return outliers;

    for (size_t i = 0; i < vals.size(); ++i) {
        double G = std::abs(vals[i] - med) / desv;
        double N = vals.size();
        static const double G_crit[] = {0,0,0,1.15,1.48,1.71,1.88,2.02,2.12,2.21,2.29,2.35,2.41,2.46,2.51,2.55,2.58,2.62,2.65,2.68,2.71,2.73,2.76,2.78,2.80,2.82,2.84,2.86,2.88,2.89,2.91};
        double t_crit = (N < 31) ? G_crit[(int)N] : 3.0;
        if (G > t_crit) outliers.push_back((int)i);
    }
    return outliers;
}

}} // namespace
