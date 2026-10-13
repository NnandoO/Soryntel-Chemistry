#pragma once
#include "domain/ports/ThermoRepository.hpp"
#include "motores/balance_masa.hpp"
#include "motores/minimizador_gibbs.hpp"
#include "motores/simulacion_dinamica.hpp"

namespace cma::application {
// Input boundary: a desktop, CLI or test can invoke these use cases with any
// repository implementing the domain's read contract.
class SimulationService {
public:
    explicit SimulationService(const domain::ThermoRepository& repository) : repository_(repository) {}
    motores::ResultadoBalanceMasa run_steady_state(const diagrama_flujo::Flowsheet& project,
        int iterations = 1000, double tolerance = 1e-9) const {
        motores::BalanceMasa engine(repository_);
        return engine.resolver(project, iterations, tolerance);
    }
    motores::ResultadoGibbs run_equilibrium(const std::map<int64_t,double>& feed, double kelvin,
        double atm, const std::vector<int64_t>& candidates = {},
        std::map<int64_t,double>* warm_start = nullptr, bool electrochemical = false) const {
        motores::MinimizadorGibbs engine;
        return engine.minimizar(feed,kelvin,atm,repository_.mapa_especies(),candidates,warm_start,electrochemical);
    }
private:
    const domain::ThermoRepository& repository_;
};
}
