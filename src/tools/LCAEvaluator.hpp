#pragma once
#include <string>
#include "../motores/lca.hpp"

namespace hsc_clone {

struct ImpactoLCA {
    double emision_CO2 = 0.0;
    double huella_carbono = 0.0;
    double consumo_energia = 0.0;
    bool ok = false;
    std::string mensaje;
};

class LCAEvaluator {
public:
    ImpactoLCA evaluar(double kWh, double gas_m3, double comb_kg,
                       double materia_kg, double prod_kg, double agua_m3);

private:
    cma::motores::LCA motor_;
};

} // namespace hsc_clone
