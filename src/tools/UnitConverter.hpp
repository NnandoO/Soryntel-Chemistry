#pragma once
#include "application/modules.hpp"
#include <string>

namespace cma::application::modules {

class UnitConverter {
public:
    double convertir(double valor, const std::string& de, const std::string& a);
};

} // namespace cma::application::modules
