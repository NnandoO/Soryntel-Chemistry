#pragma once
#include "../motores/balance_masa.hpp"
#include <vector>
#include <string>

namespace hsc_clone {

struct ResultadoSteadyState {
    bool convergio = false;
    std::string mensaje;
    std::vector<cma::nucleo::Flujo> flujos_salida;
};

class SteadyStateEngine {
public:
    ResultadoSteadyState ejecutar(const cma::diagrama_flujo::Flowsheet& fs);

private:
    cma::motores::BalanceMasa balance_masa_;
};

} // namespace hsc_clone
