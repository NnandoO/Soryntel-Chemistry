#include "HeatLossCalc.hpp"

namespace cma::application::modules {

cma::motores::CapaAislante HeatLossCalc::toCapaAislante(const Capa& c) {
    cma::motores::CapaAislante ca;
    ca.material = c.material;
    ca.espesor  = c.espesor;
    ca.k        = c.k;
    return ca;
}

ResultadoPerdida HeatLossCalc::plano(double T_int, double T_ext, double area,
                                      const std::vector<Capa>& capas) {
    std::vector<cma::motores::CapaAislante> motor_capas;
    motor_capas.reserve(capas.size());
    for (const auto& c : capas)
        motor_capas.push_back(toCapaAislante(c));

    auto motor_res = motor_.pared_plana(T_int, T_ext, area, motor_capas);

    ResultadoPerdida res;
    res.flujo_calor       = motor_res.flujo_calor;
    res.resistencia_total = motor_res.resistencia_total;
    res.mensaje           = motor_res.mensaje;
    res.ok                = motor_res.ok;
    return res;
}

} // namespace cma::application::modules
