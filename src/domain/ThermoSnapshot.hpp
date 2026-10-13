#pragma once
#include "domain/ports/ThermoRepository.hpp"
namespace cma::domain {
// Immutable value snapshot; safe to move to a scientific worker.
class ThermoSnapshot final : public ThermoRepository {
public:
    explicit ThermoSnapshot(std::map<int64_t,nucleo::Especie> species) : species_(std::move(species)) {}
    const nucleo::Especie* especie_por_id(int64_t id) const override {
        const auto it = species_.find(id); return it == species_.end() ? nullptr : &it->second;
    }
    const nucleo::Especie* especie_por_nombre(const std::string& name) const override {
        for (const auto& [id,species] : species_) if (species.nombre == name) return &species;
        return nullptr;
    }
    const nucleo::Reaccion* reaccion_por_id(int64_t) const override { return nullptr; }
    const std::map<int64_t,nucleo::Especie>& mapa_especies() const override { return species_; }
private:
    const std::map<int64_t,nucleo::Especie> species_;
};
}
