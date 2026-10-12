#include "lca.hpp"
#include <cmath>
#include <sstream>

namespace cma { namespace motores {

double LCA::factor_emision_electricidad(double kWh, const std::string& pais) {
    // kg CO2-equiv por kWh (promedio global ~0.5 kg/kWh)
    double factor = 0.5;
    if (pais == "china") factor = 0.7;
    else if (pais == "usa") factor = 0.45;
    else if (pais == "alemania") factor = 0.4;
    else if (pais == "francia") factor = 0.06;  // nuclear
    else if (pais == "brasil") factor = 0.15;   // hidroeléctrica
    return kWh * factor;
}

double LCA::factor_emision_gas_natural(double m3) {
    // ~2.0 kg CO2-equiv por m³ de gas natural
    return m3 * 2.0;
}

double LCA::factor_emision_combustible(double kg, const std::string& tipo) {
    double factor = 3.0; // kg CO2-equiv / kg
    if (tipo == "diésel") factor = 2.68;
    else if (tipo == "gasolina") factor = 2.31;
    else if (tipo == "carbón") factor = 2.86;
    else if (tipo == "biomasa") factor = 0.0; // neutro en CO2 (ciclo cerrado)
    return kg * factor;
}

double LCA::huella_carbono_total(const ImpactoLCA& impacto) {
    double total = impacto.emision_CO2;
    total += impacto.emision_CH4 * 28.0;  // GWP CH4 = 28
    total += impacto.emision_N2O * 265.0; // GWP N2O = 265
    return total;
}

ImpactoLCA LCA::evaluar_proceso(
    double kWh, double gas_m3, double combustible_kg,
    double materia_kg, double producto_kg, double agua_m3)
{
    ImpactoLCA res;

    double CO2_elec = factor_emision_electricidad(kWh);
    double CO2_gas = factor_emision_gas_natural(gas_m3);
    double CO2_comb = factor_emision_combustible(combustible_kg);

    res.emision_CO2 = CO2_elec + CO2_gas + CO2_comb;
    res.emision_CH4 = gas_m3 * 0.005; // 0.005 kg CH4 fugitivo por m³
    res.emision_N2O = combustible_kg * 0.0001; // 0.0001 kg N2O por kg

    res.huella_carbono = huella_carbono_total(res);
    res.consumo_energia = kWh * 3.6 + gas_m3 * 38.0; // MJ
    res.consumo_agua = agua_m3;
    res.residuos_solidos = materia_kg * 0.1; // 10% de residuos

    res.ok = true;
    std::ostringstream os;
    os << "Huella de carbono: " << res.huella_carbono << " kg CO₂-equiv";
    res.mensaje = os.str();

    return res;
}

}} // namespace
