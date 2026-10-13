#pragma once
#include "nucleo/tipos.hpp"
#include <map>
#include <string>
#include <vector>
namespace cma::motores {
using MineralParticle = nucleo::MineralParticle;
using ParticleStream = nucleo::ParticleStream;
struct ParticleSplit {
    bool ok = false;
    std::string message;
    ParticleStream product, residue;
    double mass_residual_kg_h = 0;
};
struct FlotationKinetics { double rate_per_min = 0; double ultimate_recovery = 0; };
class MineralProcessing {
public:
    // Coarse partition = bypass + (1-bypass)/(1+(d50/d)^sharpness).
    // Parameters are supplied/calibrated by the user, not invented from icons.
    static ParticleSplit classify(const ParticleStream& feed,double d50_um,double sharpness,
        double bypass,double water_to_coarse);
    // First-order batch/plug-flow kinetic recovery; not a CSTR bank model.
    static ParticleSplit flotation(const ParticleStream& feed,
        const std::map<std::string,FlotationKinetics>& kinetics,double residence_min,double water_recovery);
    // Bond work index estimate, feed/product 80% passing diameters in micrometers.
    static double bond_energy_kWh_t(double work_index_kWh_t,double feed_f80_um,double product_p80_um);
    static double solids_kg_h(const ParticleStream& stream);
};
}
