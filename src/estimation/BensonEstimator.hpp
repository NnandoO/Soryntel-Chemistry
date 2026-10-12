#pragma once
#include "StatisticalEstimator.hpp"

namespace hsc_clone {

class BensonEstimator {
public:
    PropiedadesEstimadas estimarBenson(const std::string& formula);

private:
    cma::motores::EstimadorPropiedades motor_;
};

} // namespace hsc_clone
