#include "MaterialDB.hpp"

namespace hsc_clone {

MaterialDB::MaterialDB() : motor_(cma::base_datos::GestorDB::instancia()) {}

std::vector<const cma::nucleo::Especie*> MaterialDB::materiales() {
    auto todas = motor_.todas_especies();
    std::vector<const cma::nucleo::Especie*> out;
    for (auto* e : todas)
        if (e && e->fase_std != cma::nucleo::Fase::Gas)
            out.push_back(e);
    return out;
}

}
