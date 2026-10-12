#include "LCAEvaluator.hpp"

namespace hsc_clone {

ImpactoLCA LCAEvaluator::evaluar(double kWh, double gas_m3, double comb_kg,
                                 double materia_kg, double prod_kg, double agua_m3)
{
    auto motor_res = motor_.evaluar_proceso(kWh, gas_m3, comb_kg, materia_kg, prod_kg, agua_m3);
    ImpactoLCA res;
    res.emision_CO2    = motor_res.emision_CO2;
    res.huella_carbono = motor_res.huella_carbono;
    res.consumo_energia = motor_res.consumo_energia;
    res.ok             = motor_res.ok;
    res.mensaje        = motor_res.mensaje;
    return res;
}

} // namespace hsc_clone
