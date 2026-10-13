#pragma once
#include "domain/ports/ThermoRepository.hpp"
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
    explicit StabilityDiagrams(const domain::ThermoRepository* repo = nullptr) : repo_(repo) {}
    std::vector<PuntoEstabilidad> generar(const std::string& metal,
                                           double T = 900.0) const;
    std::vector<PuntoEstabilidad> generar(const std::string& metal, double T,
                                           double log_pO2_min, double log_pO2_max, int steps_O2,
                                           double log_pSO2_min, double log_pSO2_max, int steps_SO2) const;
private:
    const domain::ThermoRepository* repo_ = nullptr;
};

}
