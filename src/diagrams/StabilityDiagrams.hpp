#pragma once
#include "application/modules.hpp"
#include <vector>
#include <string>

namespace cma::application::modules {

struct PuntoEstabilidad {
    double log_pO2;
    double log_pSO2;
    std::string fase;
};

class StabilityDiagrams {
public:
    std::vector<PuntoEstabilidad> generar(const std::string& metal,
                                           double T = 900.0);
};

}
