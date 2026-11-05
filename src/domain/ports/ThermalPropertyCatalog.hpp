#pragma once
#include "domain/HeatLossTypes.hpp"

#include <string>
#include <vector>

namespace cma::domain {

// Read/write port for the Conduction, Convection and Surface Radiation databases
// used by the Heat Loss / Heat Transfer module.
class ThermalPropertyCatalog {
  public:
    virtual ~ThermalPropertyCatalog() = default;

    virtual const std::vector<ConductionMaterialEntry>& conduction_materials() const = 0;
    virtual const std::vector<ConvectionFluidEntry>& convection_fluids() const = 0;
    virtual const std::vector<SurfaceRadiationEntry>& surface_materials() const = 0;

    virtual const ConductionMaterialEntry* find_conduction(const std::string& name) const = 0;
    virtual const ConvectionFluidEntry* find_convection(const std::string& name) const = 0;
    virtual const SurfaceRadiationEntry* find_surface(const std::string& name) const = 0;
};

} // namespace cma::domain
