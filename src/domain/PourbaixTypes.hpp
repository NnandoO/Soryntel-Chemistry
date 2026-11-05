#pragma once
#include <cstdint>
#include <map>
#include <string>
#include <utility>
#include <vector>

namespace cma::domain {

enum class PourbaixPotentialScale : uint8_t {
    SHE = 0,   // Standard Hydrogen Electrode (0.000 V ref)
    SCE = 1,   // Saturated Calomel Electrode (+0.241 V vs SHE at 25°C)
    AgAgCl = 2 // Ag/AgCl (sat. KCl, +0.197 V vs SHE at 25°C)
};

enum class PourbaixCorrosionState : uint8_t {
    Immunity = 0,    // Metallic / elemental zero-valent solid (e.g. Fe, Cu, Au)
    Passivation = 1, // Condensed oxide, hydroxide, sulfide or insoluble salt (e.g. Fe2O3, Cu2O)
    Corrosion = 2,   // Dissolved aqueous ion or soluble complex (e.g. Fe2+, Cu2+, AuCl4-)
    GasPhase = 3     // Volatile species
};

struct PourbaixLigandSpec {
    std::string element;    // e.g. "S", "Cl", "C", "N"
    double molality = 1e-3; // Total dissolved ligand activity/molality [mol/kg H2O]
    bool enabled = true;
};

struct PourbaixSpeciesInfo {
    int64_t id = 0;
    std::string formula;       // Raw DB formula (e.g. "Fe(+2a)", "Fe2O3")
    std::string clean_formula; // Formatted label (e.g. "Fe²⁺(aq)", "Fe₂O₃(s)")
    std::string nombre;
    std::string estado; // "s", "l", "g", "aq"
    int carga = 0;
    int n_M = 0;
    int n_H = 0;
    int n_O = 0;
    std::map<std::string, int> n_ligands;
    double dG0_kJ_mol = 0.0; // Standard Gibbs energy at T [kJ/mol]
    double molality = 1e-6;
    bool enabled = true;
    PourbaixCorrosionState corrosion_state = PourbaixCorrosionState::Corrosion;
};

struct PourbaixSegment {
    double pH1 = 0.0;
    double Eh1 = 0.0;
    double pH2 = 0.0;
    double Eh2 = 0.0;
};

struct PourbaixReactionLine {
    int idx_sp1 = -1;
    int idx_sp2 = -1;
    std::string sp1_formula;
    std::string sp2_formula;
    std::string reaction_equation;
    double E0_V = 0.0;    // Intercept at pH = 0 [V vs SHE]
    double dEh_dpH = 0.0; // Nernst slope [V/pH]
    double pH_eq = 0.0;   // Equilibrium pH for vertical acid-base line
    int n_electrons = 0;
    int n_protons = 0;
    bool is_vertical = false;
    bool is_horizontal = false;
    std::vector<PourbaixSegment> clipped_segments;
};

struct PourbaixPolygon {
    int species_index = -1;
    std::string formula;
    std::string clean_formula;
    std::string estado;
    PourbaixCorrosionState corrosion_state = PourbaixCorrosionState::Corrosion;
    std::vector<std::pair<double, double>> vertices; // (pH, Eh) CCW convex polygon
    double centroid_pH = 7.0;
    double centroid_Eh = 0.0;
    double area = 0.0;
};

struct PourbaixTriplePoint {
    double pH = 0.0;
    double Eh_V = 0.0;
    std::string sp1;
    std::string sp2;
    std::string sp3;
};

struct PourbaixPointEvaluation {
    bool valid = false;
    double pH = 7.0;
    double Eh_SHE_V = 0.0;
    double Eh_display_V = 0.0;
    double pe = 0.0;
    int predominant_idx = -1;
    std::string predominant_formula;
    std::string predominant_clean;
    std::string predominant_state;
    PourbaixCorrosionState corrosion_state = PourbaixCorrosionState::Corrosion;
    double eh_water_upper_V = 1.229;
    double eh_water_lower_V = 0.0;
    bool water_stable = true;
    bool o2_evolution = false;
    bool h2_evolution = false;
    double stability_margin_kJ = 0.0;
    std::string runner_up_formula;
};

struct PourbaixDiagramConfig {
    std::string main_element = "Fe";
    std::vector<PourbaixLigandSpec> ligands;
    double temperature_K = 298.15;
    double pressure_bar = 1.0;
    double metal_molality = 1e-6;
    double pH_min = -1.0;
    double pH_max = 15.0;
    double Eh_min = -1.8;
    double Eh_max = 2.2;
    int grid_pH = 90;
    int grid_Eh = 90;
    double p_O2_atm = 1.0;
    double p_H2_atm = 1.0;
    PourbaixPotentialScale scale = PourbaixPotentialScale::SHE;
    bool suppress_gas_phases = true;
};

inline double pourbaix_scale_offset_V(PourbaixPotentialScale scale) {
    switch (scale) {
    case PourbaixPotentialScale::SHE:
        return 0.0;
    case PourbaixPotentialScale::SCE:
        return 0.2412; // E(SHE) = E(SCE) + 0.2412 V -> E(SCE) = E(SHE) - 0.2412 V
    case PourbaixPotentialScale::AgAgCl:
        return 0.1970; // E(Ag/AgCl) = E(SHE) - 0.1970 V
    }
    return 0.0;
}

inline const char* pourbaix_scale_label(PourbaixPotentialScale scale) {
    switch (scale) {
    case PourbaixPotentialScale::SHE:
        return "V vs. SHE";
    case PourbaixPotentialScale::SCE:
        return "V vs. SCE (+0.241 V)";
    case PourbaixPotentialScale::AgAgCl:
        return "V vs. Ag/AgCl (+0.197 V)";
    }
    return "V vs. SHE";
}

inline const char* pourbaix_corrosion_label(PourbaixCorrosionState st) {
    switch (st) {
    case PourbaixCorrosionState::Immunity:
        return "Inmunidad (Metal Estable)";
    case PourbaixCorrosionState::Passivation:
        return "Pasivación (Capa Sólida Protectora)";
    case PourbaixCorrosionState::Corrosion:
        return "Corrosión / Disolución Activa (Acuoso)";
    case PourbaixCorrosionState::GasPhase:
        return "Volatilización (Fase Gas)";
    }
    return "Desconocido";
}

} // namespace cma::domain
