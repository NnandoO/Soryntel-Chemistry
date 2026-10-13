#pragma once
#include "nucleo/tipos.hpp"
#include <map>
#include <vector>

namespace cma::domain {
// Read port: scientific code depends on this contract, never on storage.
class ThermoRepository {
public:
    virtual ~ThermoRepository() = default;
    virtual const nucleo::Especie* especie_por_id(int64_t id) const = 0;
    virtual const nucleo::Especie* especie_por_nombre(const std::string& nombre) const = 0;
    virtual const nucleo::Reaccion* reaccion_por_id(int64_t id) const = 0;
    virtual const std::map<int64_t, nucleo::Especie>& mapa_especies() const = 0;
    std::vector<const nucleo::Especie*> todas_especies() const {
        std::vector<const nucleo::Especie*> result;
        for (const auto& [id, especie] : mapa_especies()) result.push_back(&especie);
        return result;
    }
};
}
