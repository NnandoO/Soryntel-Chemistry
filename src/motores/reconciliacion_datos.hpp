#pragma once
#include <vector>
#include <string>
#include <map>

namespace cma { namespace motores {

struct Medicion {
    std::string nombre;
    double valor_medido;
    double incertidumbre; // desviación estándar
};

struct ResultadoReconciliacion {
    std::map<std::string, double> valores_reconciliados;
    std::map<std::string, double> residuos;
    double chi_cuadrado = 0;
    double q_ajuste = 0; // bondad de ajuste
    std::string mensaje;
    bool ok = false;
};

class ReconciliacionDatos {
public:
    // Reconciliación por mínimos cuadrados ponderados con restricción lineal
    // Σ(entrada) = Σ(salida) para balances másicos
    ResultadoReconciliacion reconciliar_balance_masa(
        const std::vector<Medicion>& entradas,
        const std::vector<Medicion>& salidas,
        double tolerancia = 1e-6,
        int max_iter = 100);

    // Reconciliación con restricción de balance de energía
    ResultadoReconciliacion reconciliar_balance_energia(
        const std::vector<Medicion>& corr_entrada,
        const std::vector<Medicion>& corr_salida,
        const std::vector<Medicion>& temperaturas,
        double tolerancia = 1e-6);

    // Detección de outliers (prueba de Grubbs)
    std::vector<int> detectar_outliers(const std::vector<Medicion>& datos,
                                        double alpha = 0.05);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;

    double media(const std::vector<double>& v);
    double desviacion(const std::vector<double>& v, double med);
};

}} // namespace
