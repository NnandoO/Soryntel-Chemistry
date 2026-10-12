#include "ExergyCalculator.hpp"

namespace hsc_clone {

ResultadoExergia ExergyCalculator::analizar(
    double h_in, double h_out,
    double T_in, double T_out,
    double s_in, double s_out)
{
    auto res = exergia_.analizar_proceso(h_in, h_out, 0.0,
                                         T_in, T_out, 298.15,
                                         s_in, s_out, 0.0,
                                         0.0, 0.0);
    ResultadoExergia r;
    r.exergia_fisica = res.exergia_fisica;
    r.exergia_quimica = res.exergia_quimica;
    r.exergia_total = res.exergia_total;
    r.eficiencia_exergetica = res.eficiencia_exergetica;
    r.exergia_destruida = res.exergia_destruida;
    r.mensaje = std::move(res.mensaje);
    r.ok = res.ok;
    return r;
}

} // namespace hsc_clone
