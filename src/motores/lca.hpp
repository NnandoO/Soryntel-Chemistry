#pragma once
#include <string>
#include <vector>
#include <map>

namespace cma { namespace motores {

struct ImpactoLCA {
    double emision_CO2;      // kg CO2-equiv
    double emision_CH4;      // kg CH4-equiv
    double emision_N2O;      // kg N2O-equiv
    double huella_carbono;   // kg CO2-equiv total
    double consumo_energia;  // MJ
    double consumo_agua;     // m³
    double residuos_solidos; // kg
    std::string mensaje;
    bool ok = false;
};

class LCA {
public:
    // Evaluación simplificada basada en factores de emisión
    ImpactoLCA evaluar_proceso(
        double consumo_electricidad_kWh,     // electricidad consumida
        double consumo_gas_natural_m3,       // gas natural
        double combustible_liquido_kg,       // diésel/gasolina
        double materia_prima_kg,             // masa de materias primas
        double producto_kg,                  // masa de producto
        double agua_consumida_m3);           // agua consumida

    // Factores de emisión por tipo de energía
    double factor_emision_electricidad(double kWh, const std::string& pais = "global");
    double factor_emision_gas_natural(double m3);
    double factor_emision_combustible(double kg, const std::string& tipo = "diésel");

    // Huella de carbono total
    double huella_carbono_total(const ImpactoLCA& impacto);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;
};

}} // namespace
