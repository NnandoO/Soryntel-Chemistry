#pragma once
#include "application/modules.hpp"
#include <string>
#include <vector>
#include "../motores/perdida_calor.hpp"

namespace cma::application::modules {

struct Capa {
    std::string material;
    double espesor = 0.0;
    double k = 0.0;
};

struct ResultadoPerdida {
    double flujo_calor = 0.0;
    double resistencia_total = 0.0;
    std::string mensaje;
    bool ok = false;
};

class HeatLossCalc {
public:
    ResultadoPerdida plano(double T_int, double T_ext, double area,
                           const std::vector<Capa>& capas);

private:
    cma::motores::PerdidaCalor motor_;
    static cma::motores::CapaAislante toCapaAislante(const Capa& c);
};

} // namespace cma::application::modules
