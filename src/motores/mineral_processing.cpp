#include "mineral_processing.hpp"
#include <cmath>
#include <algorithm>
#include <limits>
namespace cma::motores {
namespace {
bool valid(const ParticleStream& stream) {
    if (!std::isfinite(stream.water_kg_h) || stream.water_kg_h<0 || stream.particles.size()>100000) return false;
    for (const auto& p : stream.particles)
        if (p.mineral.empty() || !std::isfinite(p.diameter_um) || p.diameter_um<=0 || !std::isfinite(p.mass_kg_h) || p.mass_kg_h<0) return false;
    return true;
}
template<class Recovery> ParticleSplit split(const ParticleStream& feed,double water_recovery,Recovery recovery) {
    ParticleSplit result;
    if (!valid(feed) || !std::isfinite(water_recovery) || water_recovery<0 || water_recovery>1) {
        result.message="Corriente o partición de agua inválida"; return result;
    }
    for (const auto& particle : feed.particles) {
        const double fraction=recovery(particle);
        if (!std::isfinite(fraction) || fraction<0 || fraction>1) { result.message="Recuperación mineral inválida"; return result; }
        auto product=particle,residue=particle;
        product.mass_kg_h=particle.mass_kg_h*fraction;
        residue.mass_kg_h=particle.mass_kg_h-product.mass_kg_h;
        result.product.particles.push_back(product); result.residue.particles.push_back(residue);
    }
    result.product.water_kg_h=feed.water_kg_h*water_recovery;
    result.residue.water_kg_h=feed.water_kg_h-result.product.water_kg_h;
    result.mass_residual_kg_h=MineralProcessing::solids_kg_h(feed)+feed.water_kg_h-
        MineralProcessing::solids_kg_h(result.product)-result.product.water_kg_h-
        MineralProcessing::solids_kg_h(result.residue)-result.residue.water_kg_h;
    result.ok=true; result.message="Balance por mineral, tamaño y agua conservado";
    return result;
}
}
double MineralProcessing::solids_kg_h(const ParticleStream& stream) {
    double total=0; for (const auto& p : stream.particles) total+=p.mass_kg_h; return total;
}
ParticleSplit MineralProcessing::classify(const ParticleStream& feed,double d50,double sharpness,double bypass,double water_to_coarse) {
    if (!std::isfinite(d50) || d50<=0 || !std::isfinite(sharpness) || sharpness<=0 ||
        !std::isfinite(bypass) || bypass<0 || bypass>=1) {
        ParticleSplit result; result.message="Parámetros de clasificación inválidos"; return result;
    }
    return split(feed,water_to_coarse,[&](const auto& p) {
        const double exponent=sharpness*std::log(d50/p.diameter_um);
        const double logistic=exponent>=0 ? std::exp(-exponent)/(1+std::exp(-exponent)) : 1/(1+std::exp(exponent));
        return bypass+(1-bypass)*logistic;
    });
}
ParticleSplit MineralProcessing::flotation(const ParticleStream& feed,const std::map<std::string,FlotationKinetics>& kinetics,
    double residence,double water_recovery) {
    if (!std::isfinite(residence) || residence<0) { ParticleSplit r; r.message="Residencia inválida"; return r; }
    for (const auto& particle : feed.particles) {
        const auto it=kinetics.find(particle.mineral);
        if (it==kinetics.end() || !std::isfinite(it->second.rate_per_min) || it->second.rate_per_min<0 ||
            !std::isfinite(it->second.ultimate_recovery) || it->second.ultimate_recovery<0 || it->second.ultimate_recovery>1) {
            ParticleSplit r; r.message="Faltan parámetros cinéticos válidos para "+particle.mineral; return r;
        }
    }
    return split(feed,water_recovery,[&](const auto& p) {
        const auto& k=kinetics.at(p.mineral);
        return k.ultimate_recovery*(-std::expm1(-k.rate_per_min*residence));
    });
}
double MineralProcessing::bond_energy_kWh_t(double index,double feed,double product) {
    if (!std::isfinite(index) || index<=0 || !std::isfinite(feed) || !std::isfinite(product) || product<=0 || feed<product)
        return std::numeric_limits<double>::quiet_NaN();
    return 10*index*(1/std::sqrt(product)-1/std::sqrt(feed));
}
}
