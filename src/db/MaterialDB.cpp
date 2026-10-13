#include "MaterialDB.hpp"

namespace cma::application::modules {

MaterialDB::MaterialDB(const cma::domain::ThermoRepository& repository) : motor_(repository) {}

std::vector<const cma::nucleo::Especie*> MaterialDB::materiales() {
    auto todas = motor_.todas_especies();
    std::vector<const cma::nucleo::Especie*> out;
    for (auto* e : todas)
        if (e && e->fase_std != cma::nucleo::Fase::Gas)
            out.push_back(e);
    return out;
}

}
