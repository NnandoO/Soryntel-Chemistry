#pragma once
#include "application/modules.hpp"
#include <string>
#include "../motores/estimador_propiedades.hpp"

namespace cma::application::modules {

struct PropiedadesEstimadas {
    double h_formacion = 0.0;
    double s_formacion = 0.0;
    double masa_molar  = 0.0;
    bool ok = false;
    std::string mensaje;
};

class StatisticalEstimator {
public:
    PropiedadesEstimadas estimarJoback(const std::string& formula);

private:
    cma::motores::EstimadorPropiedades motor_;
};

} // namespace cma::application::modules
