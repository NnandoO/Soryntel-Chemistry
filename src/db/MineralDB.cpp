#include "MineralDB.hpp"

namespace cma::application::modules {

MineralDB::MineralDB(const cma::domain::ThermoRepository& repository) : motor_(repository) {}

std::vector<const cma::nucleo::Especie*> MineralDB::minerales() {
    auto todas = motor_.todas_especies();
    std::vector<const cma::nucleo::Especie*> out;
    for (auto* e : todas)
        if (e && e->fase_std == cma::nucleo::Fase::Solido)
            out.push_back(e);
    return out;
}

}
