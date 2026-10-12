#pragma once
#include "nucleo/tipos.hpp"
#include <Eigen/Dense>
#include <map>
#include <vector>

namespace cma { namespace motores {

struct ResultadoGibbs {
    std::map<int64_t, double> flujo_molar;
    double energia_gibbs_total = 0.0;
    int    iteraciones = 0;
    bool   convergio   = false;
    std::string mensaje;
};

class MinimizadorGibbs {
public:
    MinimizadorGibbs() = default;

    // n_inout: semilla/resultado de composición (warm start entre puntos del barrido)
    // cargado:  añade el electrón e- y la restricción de electroneutralidad
    //           (equilibrio de celda / fases acuosas con iones)
    ResultadoGibbs minimizar(
        const std::map<int64_t, double>& alimentacion,
        double T_K,
        double P_atm,
        const std::map<int64_t, nucleo::Especie>& db,
        const std::vector<int64_t>& especies_posibles = {},
        std::map<int64_t, double>* n_inout = nullptr,
        bool modo_celda = false
    );

private:
    static constexpr int    MAX_ITER = 300;
    static constexpr double TOL      = 1e-10;
    static constexpr double DAMPING  = 0.5;
};

}} // namespace
