#pragma once
#include "nucleo/tipos.hpp"

#include <map>
#include <string>
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
    virtual std::vector<const nucleo::Especie*> todas_especies() const {
        std::vector<const nucleo::Especie*> result;
        for (const auto& [id, especie] : mapa_especies())
            result.push_back(&especie);
        return result;
    }
    virtual std::vector<const nucleo::Especie*> buscar_especies(const std::string& query) const {
        std::vector<const nucleo::Especie*> result;
        for (const auto& [id, especie] : mapa_especies())
            if (especie.nombre.find(query) != std::string::npos || especie.formula.find(query) != std::string::npos)
                result.push_back(&especie);
        return result;
    }
    virtual std::vector<const nucleo::Reaccion*> todas_reacciones() const {
        return {};
    }
};
} // namespace cma::domain
