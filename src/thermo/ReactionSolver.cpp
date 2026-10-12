#include "ReactionSolver.hpp"
#include "base_datos/gestor_db.hpp"

namespace hsc_clone {

ResultadoReaccion ReactionSolver::resolver(
    const std::vector<cma::nucleo::Flujo>& entradas,
    const cma::nucleo::Reaccion& rxn,
    double conversion, double T, double P)
{
    ResultadoReaccion r;
    try {
        r.flujo_salida = balance_masa_.reactor(entradas, rxn, conversion, T, P);
        auto db = cma::base_datos::GestorDB::instancia().mapa_especies();
        r.calor_reaccion = balance_energia_.calor_reaccion(rxn, T, db);
        r.ok = true;
    } catch (const std::exception& e) {
        r.ok = false;
        r.mensaje = e.what();
    }
    return r;
}

} // namespace hsc_clone
