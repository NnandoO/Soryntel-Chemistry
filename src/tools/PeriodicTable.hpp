#pragma once
#include "application/modules.hpp"
#include <string>
#include <vector>
#include "../nucleo/tabla_periodica.hpp"

namespace cma::application::modules {

struct Elemento {
    int numero_atomico = 0;
    std::string simbolo;
    std::string nombre;
    double masa_atomica = 0.0;
    double electronegatividad = 0.0;
    int periodo = 0;
    int grupo = 0;
};

class PeriodicTable {
public:
    const Elemento* elemento(int numero_atomico) const;
    std::vector<const Elemento*> todos() const;
    const Elemento* buscar(const std::string& simbolo_o_nombre) const;

private:
    mutable std::vector<Elemento> cache_;
    mutable bool cache_lleno_ = false;
    void llenar_cache() const;
    static Elemento fromNucleo(const cma::nucleo::Elemento& e);
};

} // namespace cma::application::modules
