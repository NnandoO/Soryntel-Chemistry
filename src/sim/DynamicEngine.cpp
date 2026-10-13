#include "DynamicEngine.hpp"

namespace cma::application::modules {

ResultadoDinamico DynamicEngine::simular(
    double t0, double tf, double dt,
    const std::map<std::string, double>& condiciones_iniciales,
    std::function<void(double t, const std::map<std::string,double>& y,
                      std::map<std::string,double>& dydt)> f)
{
    auto res = simulacion_.resolver(t0, tf, dt, condiciones_iniciales, f);
    ResultadoDinamico r;
    r.tiempos = std::move(res.tiempos);
    r.variables = std::move(res.variables);
    r.ok = res.ok;
    r.mensaje = std::move(res.mensaje);
    return r;
}

} // namespace cma::application::modules
