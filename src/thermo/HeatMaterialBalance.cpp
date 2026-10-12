#include "HeatMaterialBalance.hpp"

namespace hsc_clone {

ResultadoBalanceTermico HeatMaterialBalance::calcular(
    const cma::nucleo::Nodo& nodo,
    const std::vector<cma::nucleo::Flujo>& entradas,
    const std::vector<cma::nucleo::Flujo>& salidas,
    const std::map<int64_t, cma::nucleo::Especie>& db)
{
    auto res = balance_energia_.resolver_nodo(nodo, entradas, salidas, db);
    ResultadoBalanceTermico r;
    r.entalpia_entrada = res.entalpia_entrada;
    r.entalpia_salida = res.entalpia_salida;
    r.calor = res.calor;
    r.trabajo = res.trabajo;
    r.delta_T = res.delta_T;
    r.balanceado = res.balanceado;
    r.mensaje = std::move(res.mensaje);
    return r;
}

} // namespace hsc_clone
