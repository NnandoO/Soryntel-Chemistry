#include "StatisticalEstimator.hpp"

namespace cma::application::modules {

PropiedadesEstimadas StatisticalEstimator::estimarJoback(const std::string& formula) {
    auto motor_res = motor_.estimar_joback(formula);
    PropiedadesEstimadas res;
    res.h_formacion = motor_res.h_formacion;
    res.s_formacion = motor_res.s_formacion;
    res.masa_molar  = motor_res.masa_molar;
    res.ok          = motor_res.ok;
    res.mensaje     = motor_res.mensaje;
    return res;
}

} // namespace cma::application::modules
