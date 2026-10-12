#include "ElectrolyteSolver.hpp"

namespace hsc_clone {

double ElectrolyteSolver::calcularpH(double concentracion_H, double T) {
    return motor_.ph(concentracion_H, T);
}

double ElectrolyteSolver::coeficienteActividad(double carga, double fuerza_ionica, double T) {
    return motor_.coeficiente_actividad_davies(carga, fuerza_ionica, T);
}

}
