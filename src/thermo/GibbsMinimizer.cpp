#include "GibbsMinimizer.hpp"

namespace cma::application::modules {

ResultadoGibbs GibbsMinimizer::minimizar(
    const std::map<int64_t, double>& alimentacion,
    double T, double P,
    const std::map<int64_t, cma::nucleo::Especie>& db,
    const std::vector<int64_t>& especies_posibles,
    std::map<int64_t, double>* n_inout,
    bool modo_celda,
    const std::map<int64_t, double>& coef_actividad,
    const std::map<int64_t, int>& fases_especie)
{
    auto res = minimizador_.minimizar(alimentacion, T, P, db, especies_posibles, n_inout, modo_celda, coef_actividad, fases_especie);
    ResultadoGibbs r;
    r.flujo_molar = std::move(res.flujo_molar);
    r.energia_gibbs_total = res.energia_gibbs_total;
    r.iteraciones = res.iteraciones;
    r.convergio = res.convergio;
    r.mensaje = std::move(res.mensaje);
    return r;
}

} // namespace cma::application::modules
