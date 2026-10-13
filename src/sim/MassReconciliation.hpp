#pragma once
#include "application/modules.hpp"
#include "../motores/reconciliacion_datos.hpp"
#include <vector>
#include <string>
#include <map>

namespace cma::application::modules {

using Medicion = cma::motores::Medicion;

struct ResultadoReconciliacion {
    std::map<std::string, double> valores_reconciliados;
    std::map<std::string, double> residuos;
    double chi_cuadrado = 0;
    double q_ajuste = 0;
    std::string mensaje;
    bool ok = false;
};

class MassReconciliation {
public:
    ResultadoReconciliacion reconciliar(const std::vector<Medicion>& entradas,
                                        const std::vector<Medicion>& salidas);
private:
    cma::motores::ReconciliacionDatos reconciliacion_;
};

} // namespace cma::application::modules
