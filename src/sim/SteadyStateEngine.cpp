#include "SteadyStateEngine.hpp"

namespace hsc_clone {

ResultadoSteadyState SteadyStateEngine::ejecutar(const cma::diagrama_flujo::Flowsheet& fs) {
    auto res = balance_masa_.resolver(fs);
    ResultadoSteadyState r;
    r.convergio = res.convergio;
    r.mensaje = res.mensaje;
    r.flujos_salida = std::move(res.flujos_salida);
    return r;
}

} // namespace hsc_clone
