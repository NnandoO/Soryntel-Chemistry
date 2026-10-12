#pragma once
#include "../motores/exergia.hpp"
#include <string>

namespace hsc_clone {

struct ResultadoExergia {
    double exergia_fisica = 0;
    double exergia_quimica = 0;
    double exergia_total = 0;
    double eficiencia_exergetica = 0;
    double exergia_destruida = 0;
    std::string mensaje;
    bool ok = false;
};

class ExergyCalculator {
public:
    ResultadoExergia analizar(double h_in, double h_out,
                              double T_in, double T_out,
                              double s_in, double s_out);

private:
    cma::motores::Exergia exergia_;
};

} // namespace hsc_clone
