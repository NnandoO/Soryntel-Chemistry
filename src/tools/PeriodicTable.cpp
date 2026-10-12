#include "PeriodicTable.hpp"
#include <algorithm>

namespace hsc_clone {

Elemento PeriodicTable::fromNucleo(const cma::nucleo::Elemento& e) {
    Elemento el;
    el.numero_atomico    = e.numero_atomico;
    el.simbolo           = e.simbolo;
    el.nombre            = e.nombre_es;
    el.masa_atomica      = e.masa_atomica;
    el.electronegatividad = e.electronegatividad;
    el.periodo           = e.periodo;
    el.grupo             = e.grupo;
    return el;
}

void PeriodicTable::llenar_cache() const {
    if (cache_lleno_) return;
    const auto& todos = cma::nucleo::TablaPeriodica::instancia().todos();
    cache_.reserve(todos.size());
    for (const auto& e : todos)
        cache_.push_back(fromNucleo(e));
    cache_lleno_ = true;
}

const Elemento* PeriodicTable::elemento(int numero_atomico) const {
    llenar_cache();
    const auto* ne = cma::nucleo::TablaPeriodica::instancia().por_numero(numero_atomico);
    if (!ne) return nullptr;
    // Use the index from the cache to return a pointer
    auto it = std::find_if(cache_.begin(), cache_.end(),
        [z = numero_atomico](const Elemento& e) { return e.numero_atomico == z; });
    return it != cache_.end() ? &(*it) : nullptr;
}

std::vector<const Elemento*> PeriodicTable::todos() const {
    llenar_cache();
    std::vector<const Elemento*> ptrs;
    ptrs.reserve(cache_.size());
    for (const auto& e : cache_)
        ptrs.push_back(&e);
    return ptrs;
}

const Elemento* PeriodicTable::buscar(const std::string& simbolo_o_nombre) const {
    llenar_cache();
    const auto& tp = cma::nucleo::TablaPeriodica::instancia();

    const auto* ne = tp.por_simbolo(simbolo_o_nombre);
    if (!ne)
        ne = tp.por_nombre(simbolo_o_nombre);
    if (!ne) return nullptr;

    auto it = std::find_if(cache_.begin(), cache_.end(),
        [z = ne->numero_atomico](const Elemento& e) { return e.numero_atomico == z; });
    return it != cache_.end() ? &(*it) : nullptr;
}

} // namespace hsc_clone
