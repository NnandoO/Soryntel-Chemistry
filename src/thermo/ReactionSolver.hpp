#pragma once
#include "../motores/balance_masa.hpp"
#include "../motores/balance_energia.hpp"
#include <vector>
#include <string>
#include <map>

namespace hsc_clone {

struct ResultadoReaccion {
    cma::nucleo::Flujo flujo_salida;
    double calor_reaccion = 0.0;
    std::string mensaje;
    bool ok = false;
};

class ReactionSolver {
public:
    ResultadoReaccion resolver(const std::vector<cma::nucleo::Flujo>& entradas,
                               const cma::nucleo::Reaccion& rxn,
                               double conversion, double T, double P);

private:
    cma::motores::BalanceMasa balance_masa_;
    cma::motores::BalanceEnergia balance_energia_;
};

} // namespace hsc_clone
