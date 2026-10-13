#include "ReactionSolver.hpp"

namespace cma::application::modules {

ResultadoReaccion ReactionSolver::resolver(
    const std::vector<cma::nucleo::Flujo>& entradas,
    const cma::nucleo::Reaccion& rxn,
    double conversion, double T, double P)
{
    ResultadoReaccion r;
    try {
        r.flujo_salida = balance_masa_.reactor(entradas, rxn, conversion, T, P);
        const auto& db = repository_.mapa_especies();
        r.calor_reaccion = balance_energia_.calor_reaccion(rxn, T, db);
        r.ok = true;
    } catch (const std::exception& e) {
        r.ok = false;
        r.mensaje = e.what();
    }
    return r;
}

} // namespace cma::application::modules
