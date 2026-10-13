#pragma once
#include "application/modules.hpp"
#include "motores/soluciones_acuosas.hpp"

namespace cma::application::modules {

class ElectrolyteSolver {
public:
    double calcularpH(double concentracion_H, double T = 298.15);
    double coeficienteActividad(double carga, double fuerza_ionica, double T = 298.15);
private:
    cma::motores::SolucionesAcuosas motor_;
};

}
