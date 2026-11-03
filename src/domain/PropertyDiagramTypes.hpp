#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace cma::domain {

enum class PropertyDiagramMode : int {
    SpeciesProperties = 0, // Curvas de especies puras / polimorfos (H, H-H298, S, Cp, G, dGf, dHf)
    EllinghamReactions = 1 // Diagramas de Reaccion y Ellingham (dG, dH, dS, dCp, log10K)
};

enum class PropertyDiagramType : int {
    Enthalpy_H = 0,       // H(T) o Delta H(T) [kJ/mol o kcal/mol]
    HeatContent_dH298 = 1,// H(T) - H(298.15 K) [kJ/mol o kcal/mol]
    Entropy_S = 2,        // S(T) o Delta S(T) [J/(mol*K) o cal/(mol*K)]
    HeatCapacity_Cp = 3,  // Cp(T) o Delta Cp(T) [J/(mol*K) o cal/(mol*K)]
    Gibbs_G = 4,          // G(T) = H(T) - T*S(T) [kJ/mol o kcal/mol]
    DeltaG_Ellingham = 5, // Delta G°(T) de formacion o normalizado por mol de O2/S2/Cl2 [kJ o kcal]
    Log10K = 6            // log10 K(T) = -Delta G°(T) / (R * T * ln(10))
};

enum class EllinghamNormalizationBasis : int {
    PerMoleO2 = 0,      // Normalizado por 1 mol de O2(g) (Diagrama de Ellingham de Oxidos)
    PerMoleS2 = 1,      // Normalizado por 1 mol de S2(g) (Diagrama de Ellingham de Sulfuros)
    PerMoleCl2 = 2,     // Normalizado por 1 mol de Cl2(g) (Diagrama de Ellingham de Cloruros)
    PerMoleCO2 = 3,     // Normalizado por 1 mol de CO2(g) (Carbonatacion / Calcinacion)
    PerMoleSO3 = 4,     // Normalizado por 1 mol de SO3(g) (Sulfatacion / Descomposicion)
    PerMoleH2O = 5,     // Normalizado por 1 mol de H2O(g) (Hidratacion / Deshidratacion)
    PerMoleReaction = 6 // Sin reescalar (por 1 mol de ecuacion tal como esta escrita)
};

enum class EllinghamPresetFamily : int {
    MetallurgicalOxides = 0,    // Oxidos metalurgicos + C/CO/CO2 + H2/H2O (por 1 mol O2)
    SulfideRoasting = 1,        // Sulfuros metalurgicos + H2S + SO2 (por 1 mol S2)
    ChlorinationMetallurgy = 2, // Cloruros metalurgicos + HCl (por 1 mol Cl2)
    CarbonateCalcination = 3,   // Carbonatos -> Oxido + CO2 (por 1 mol CO2)
    SulfateDecomposition = 4    // Sulfatos -> Oxido + SO3 (por 1 mol SO3)
};

struct PuntoPropiedadTermo {
    double T = 298.15;         // Temperatura [K]
    double H = 0.0;            // H°(T) o Delta H°(T) [kJ/mol]
    double dH298 = 0.0;        // H°(T) - H°(298.15 K) [kJ/mol]
    double S = 0.0;            // S°(T) o Delta S°(T) [J/(mol*K)]
    double Cp = 0.0;           // Cp°(T) o Delta Cp°(T) [J/(mol*K)]
    double G = 0.0;            // G°(T) = H°(T) - T*S°(T) [kJ/mol]
    double dGf = 0.0;          // Delta G°f(T) desde elementos o Delta G°_rxn(T) normalizado [kJ/mol]
    double dHf = 0.0;          // Delta H°f(T) desde elementos o Delta H°_rxn(T) normalizado [kJ/mol]
    double dSf = 0.0;          // Delta S°f(T) desde elementos o Delta S°_rxn(T) normalizado [J/(mol*K)]
    double log10K = 0.0;       // log10 K(T)
    std::string active_phase;  // Formula/nombre del polimorfo activo a esta T
    bool en_rango = true;      // Pertinencia al dominio termico validado
};

struct TransicionFasePunto {
    double T_K = 0.0;
    double delta_H_kJ = 0.0;
    double delta_S_J_K = 0.0;
    std::string from_phase;
    std::string to_phase;
};

struct CurvaDiagramaTermo {
    std::string label;                  // Etiqueta corta (ej. "2 Fe + O2 = 2 FeO" o "Fe2O3")
    std::string equation_normalized;    // Ecuacion quimica normalizada
    double norm_factor = 1.0;           // Factor estequiometrico aplicado
    bool visible = true;
    std::vector<PuntoPropiedadTermo> points;
    std::vector<TransicionFasePunto> phase_transitions;
    std::vector<std::string> warnings;  // Advertencias de extrapolacion termica

    double evaluar_propiedad(PropertyDiagramType tipo, size_t idx, bool en_kcal = false) const {
        if (idx >= points.size()) {
            return 0.0;
        }
        const auto& p = points[idx];
        const double k_faktor = en_kcal ? (1.0 / 4.184) : 1.0;
        switch (tipo) {
        case PropertyDiagramType::Enthalpy_H:
            return p.H * k_faktor;
        case PropertyDiagramType::HeatContent_dH298:
            return p.dH298 * k_faktor;
        case PropertyDiagramType::Entropy_S:
            return p.S * k_faktor;
        case PropertyDiagramType::HeatCapacity_Cp:
            return p.Cp * k_faktor;
        case PropertyDiagramType::Gibbs_G:
            return p.G * k_faktor;
        case PropertyDiagramType::DeltaG_Ellingham:
            return p.dGf * k_faktor;
        case PropertyDiagramType::Log10K:
            return p.log10K;
        }
        return p.dGf * k_faktor;
    }
};

struct InterseccionEllingham {
    std::string curve_a;
    std::string curve_b;
    double T_K = 0.0;
    double T_C = 0.0;
    double value_Y = 0.0;       // Valor en el eje Y (p.ej. Delta G° en kJ/mol O2)
    double log10_pO2 = 0.0;
    double log10_CO_CO2 = 0.0;
    double log10_H2_H2O = 0.0;
    std::string interpretation; // Interpretacion termodinamica (ej. inversion de reduccion)
};

struct EscalaNomograficaPunto {
    double T_K = 1000.0;
    double delta_G_kJ_per_O2 = -400.0;
    double log10_pO2 = 0.0;
    double pO2_atm = 1.0;
    double log10_CO_CO2 = 0.0;
    double ratio_CO_CO2 = 1.0;
    double log10_H2_H2O = 0.0;
    double ratio_H2_H2O = 1.0;
};

struct LineaIsoPotencial {
    std::string label;
    double parameter_value = 0.0;
    std::vector<double> T_K;
    std::vector<double> delta_G_kJ;
};

struct ResultadoDiagramaPropiedades {
    PropertyDiagramMode mode = PropertyDiagramMode::EllinghamReactions;
    PropertyDiagramType diagram_type = PropertyDiagramType::DeltaG_Ellingham;
    EllinghamNormalizationBasis basis = EllinghamNormalizationBasis::PerMoleO2;
    double T_min_K = 298.15;
    double T_max_K = 2000.0;
    std::vector<CurvaDiagramaTermo> curves;
    std::vector<InterseccionEllingham> intersections;
    std::vector<LineaIsoPotencial> iso_pO2_lines;
    std::vector<LineaIsoPotencial> iso_CO_CO2_lines;
    std::vector<LineaIsoPotencial> iso_H2_H2O_lines;
    std::string summary_log;
};

} // namespace cma::domain
