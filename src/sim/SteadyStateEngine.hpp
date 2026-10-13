#pragma once
#include "application/modules.hpp"
#include "../motores/balance_masa.hpp"
#include <vector>
#include <string>

namespace cma::application::modules {

struct ResultadoSteadyState {
    bool convergio = false;
    std::string mensaje;
    std::vector<cma::nucleo::Flujo> flujos_salida;
};

class SteadyStateEngine {
public:
    explicit SteadyStateEngine(const cma::domain::ThermoRepository& repository) : balance_masa_(repository) {}
    ResultadoSteadyState ejecutar(const cma::diagrama_flujo::Flowsheet& fs);

private:
    cma::motores::BalanceMasa balance_masa_;
};

} // namespace cma::application::modules
