#pragma once
#include "../motores/minimizador_gibbs.hpp"
#include <map>
#include <vector>
#include <string>

namespace hsc_clone {

struct ResultadoGibbs {
    std::map<int64_t, double> flujo_molar;
    double energia_gibbs_total = 0.0;
    int iteraciones = 0;
    bool convergio = false;
    std::string mensaje;
};

class GibbsMinimizer {
public:
    ResultadoGibbs minimizar(const std::map<int64_t, double>& alimentacion,
                             double T, double P,
                             const std::map<int64_t, cma::nucleo::Especie>& db,
                             const std::vector<int64_t>& especies_posibles = {},
                             std::map<int64_t, double>* n_inout = nullptr,
                             bool modo_celda = false);

private:
    cma::motores::MinimizadorGibbs minimizador_;
};

} // namespace hsc_clone
