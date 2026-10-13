#pragma once
#include "application/modules.hpp"
#include "StatisticalEstimator.hpp"

namespace cma::application::modules {

class BensonEstimator {
public:
    PropiedadesEstimadas estimarBenson(const std::string& formula);

private:
    cma::motores::EstimadorPropiedades motor_;
};

} // namespace cma::application::modules
