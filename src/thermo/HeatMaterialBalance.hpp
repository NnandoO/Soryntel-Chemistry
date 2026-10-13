#pragma once
#include "application/modules.hpp"
#include "../motores/balance_energia.hpp"
#include <vector>
#include <string>
#include <map>

namespace cma::application::modules {

struct ResultadoBalanceTermico {
    double entalpia_entrada = 0.0;
    double entalpia_salida = 0.0;
    double calor = 0.0;
    double trabajo = 0.0;
    double delta_T = 0.0;
    bool balanceado = false;
    std::string mensaje;
};

class HeatMaterialBalance {
public:
    ResultadoBalanceTermico calcular(const cma::nucleo::Nodo& nodo,
                                     const std::vector<cma::nucleo::Flujo>& entradas,
                                     const std::vector<cma::nucleo::Flujo>& salidas,
                                     const std::map<int64_t, cma::nucleo::Especie>& db);

private:
    cma::motores::BalanceEnergia balance_energia_;
};

} // namespace cma::application::modules
