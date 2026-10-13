#include "MassReconciliation.hpp"

namespace cma::application::modules {

ResultadoReconciliacion MassReconciliation::reconciliar(
    const std::vector<Medicion>& entradas,
    const std::vector<Medicion>& salidas)
{
    auto res = reconciliacion_.reconciliar_balance_masa(entradas, salidas);
    ResultadoReconciliacion r;
    r.valores_reconciliados = std::move(res.valores_reconciliados);
    r.residuos = std::move(res.residuos);
    r.chi_cuadrado = res.chi_cuadrado;
    r.q_ajuste = res.q_ajuste;
    r.mensaje = std::move(res.mensaje);
    r.ok = res.ok;
    return r;
}

} // namespace cma::application::modules
