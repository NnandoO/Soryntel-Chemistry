#pragma once
#include "domain/ports/ThermoRepository.hpp"
#include "nucleo/tabla_periodica.hpp"
#include <vector>
#include <map>
#include <cmath>
namespace cma::application {
struct CatalogIssue { int64_t id; std::string code; std::string detail; };
struct CatalogAudit {
    size_t records = 0, unverified = 0, missing_references = 0, unknown_ranges = 0;
    std::vector<CatalogIssue> issues;
};
inline CatalogAudit audit_catalog(const domain::ThermoRepository& repository) {
    CatalogAudit report;
    std::map<std::string,int64_t> keys;
    const auto& table=nucleo::TablaPeriodica::instancia();
    for (const auto& [id,species] : repository.mapa_especies()) {
        ++report.records;
        if (species.provenance.quality==domain::DataQuality::Unverified) ++report.unverified;
        if (species.provenance.reference.empty()) ++report.missing_references;
        if (species.provenance.min_kelvin<=0 || species.provenance.max_kelvin<species.provenance.min_kelvin) ++report.unknown_ranges;
        const auto atoms=table.analizar_formula(species.formula);
        const bool electron=species.formula=="e-" || species.formula=="e";
        if (atoms.empty() && !electron) report.issues.push_back({id,"formula_unsupported",species.formula});
        double mass=0;
        for (const auto& atom : atoms) mass+=table.por_simbolo(atom.elem)->masa_atomica*atom.cuenta;
        if (!electron && mass>0 && (!std::isfinite(species.masa_molar) || std::abs(species.masa_molar-mass)/mass>0.01))
            report.issues.push_back({id,"molar_mass_mismatch","formula="+std::to_string(mass)+", record="+std::to_string(species.masa_molar)});
        if (!std::isfinite(species.h_formacion) || !std::isfinite(species.s_formacion))
            report.issues.push_back({id,"nonfinite_formation","Hf/Sf"});
        for (double temperature : {298.15,600.0,1000.0}) {
            const double cp=species.cp(temperature);
            if (!std::isfinite(cp) || cp<0 || cp>10000)
                report.issues.push_back({id,"cp_review","T="+std::to_string(temperature)+" K, Cp="+std::to_string(cp)+" J/(mol K); check coefficient convention and valid range"});
        }
        const auto key=species.formula+"|"+std::to_string(static_cast<int>(species.fase_std));
        if (!keys.emplace(key,id).second)
            report.issues.push_back({id,"duplicate_formula_phase","previous id="+std::to_string(keys.at(key))});
    }
    return report;
}
}
