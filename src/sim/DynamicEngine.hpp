#pragma once
#include "application/modules.hpp"
#include "../motores/simulacion_dinamica.hpp"
#include <vector>
#include <string>
#include <map>
#include <functional>

namespace cma::application::modules {

struct ResultadoDinamico {
    std::vector<double> tiempos;
    std::map<std::string, std::vector<double>> variables;
    bool ok = false;
    std::string mensaje;
};

class DynamicEngine {
public:
    ResultadoDinamico simular(
        double t0, double tf, double dt,
        const std::map<std::string, double>& condiciones_iniciales,
        std::function<void(double t, const std::map<std::string,double>& y,
                          std::map<std::string,double>& dydt)> f);

private:
    cma::motores::SimulacionDinamica simulacion_;
};

} // namespace cma::application::modules
