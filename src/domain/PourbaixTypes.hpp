#pragma once

#include "nucleo/tipos.hpp"

#include <cstdint>
#include <map>
#include <string>
#include <vector>

namespace cma::domain {

enum class PourbaixPotentialScale {
    SHE = 0,     // Standard Hydrogen Electrode (Eh, 0.0000 V)
    SCE = 1,     // Saturated Calomel Electrode (+0.2412 V vs SHE)
    AgAgCl = 2   // Ag / AgCl Reference Electrode (+0.1970 V vs SHE)
};

enum class PourbaixCorrosionState {
    Immunity = 0,    // Metal elemental en estado reducido (ΔG inmune)
    Corrosion = 1,   // Especie acuosa disuelta predominante (iónica o neutra)
    Passivation = 2  // Fase sólida condensada (óxido, hidróxido, sulfuro, sal)
};

struct EpHSearchModeFlags {
    bool gases = false;
    bool gas_ions = false;
    bool liquids = false;
    bool condensed = true;
    bool aqueous_neutrals = false;
    bool aqueous_ions = true;
    bool organics = false;
};

struct EpHElementConfig {
    std::string symbol;
    double molality = 1.0;      // mol/kg H2O
    double pressure_bar = 1.0;  // bar
};

struct EpHCandidateSpecies {
    int64_t id = 0;
    std::string formula;
    std::string clean_label;
    std::string nombre;
    std::string elemento_grupo;
    nucleo::Fase fase = nucleo::Fase::Solido;
    int estado = 2; // 0: Gas, 1: Liquid, 2: Condensed/Solid, 3: Aqueous Ion, 4: Aqueous Neutral, 5: Gas Ion
    int carga = 0;
    double h_298_kJ = 0.0;
    double s_298_J_K = 0.0;
    double cp_298_J_K = 0.0;
    double dG0_T_kJ = 0.0;
    double dG0_T_kcal = 0.0;
    double dG_app_base_kJ = 0.0;
    std::map<std::string, double> composicion;
    bool seleccionada_por_defecto = true;
};

struct EpHVertice2D {
    double pH = 0.0;
    double Eh = 0.0;
};

struct EpHPoligonoPredominancia {
    int64_t id = 0;
    int area_no = 0;
    std::string formula;
    std::string etiqueta;
    std::string nombre;
    bool es_acuoso = false;
    int carga = 0;
    int color_index = 0;
    PourbaixCorrosionState estado_corrosion = PourbaixCorrosionState::Passivation;
    double Z = 0.0; // Término independiente en kJ/(mol M)
    double A = 0.0; // Coeficiente de pH
    double B = 0.0; // Coeficiente de Eh
    double area = 0.0;
    EpHVertice2D centroide;
    std::vector<EpHVertice2D> vertices;
};

struct EpHSegmentoFrontera {
    int area_no = 0;
    std::string fase_a;
    std::string fase_b;
    std::string ecuacion_reaccion;
    int tipo_linea = 1; // 0 = Horizontal (solo e-), 1 = Diagonal (H+ y e-), 2 = Vertical (solo H+)
    bool es_agua = false;
    bool es_solo_iones = false;
    EpHVertice2D p1;
    EpHVertice2D p2;
    double pendiente = 0.0;      // dEh / dpH (V/pH)
    double intercepto_eh = 0.0;  // Eh a pH = 0 (V)
    double dG_rxn_kJ = 0.0;
};

struct EpHPuntoTriple {
    std::string fase_1;
    std::string fase_2;
    std::string fase_3;
    double pH = 0.0;
    double Eh = 0.0;
};

struct PourbaixPointEvaluation {
    bool valid = false;
    double pH = 7.0;
    double Eh = 0.0;
    std::string predominant_formula;
    std::string predominant_ion_formula;
    double dG_apparent_kJ = 0.0;
    PourbaixCorrosionState corrosion_state = PourbaixCorrosionState::Immunity;
    bool water_stable = true;
    bool h2_evolution = false;
    bool o2_evolution = false;
};

struct EpHSystemParameters {
    std::vector<EpHElementConfig> elementos = {
        {"Cu", 1.0, 1.0},
        {"S",  1.0, 1.0}
    };
    std::string elemento_principal = "Cu";
    EpHSearchModeFlags modos_busqueda;
    std::vector<std::string> formulas_seleccionadas;

    double T_K = 298.15;
    bool temp_en_celsius = true;
    std::vector<double> temperaturas_combiner_C = {25.0, 50.0, 75.0, 100.0};

    PourbaixPotentialScale escala_potencial = PourbaixPotentialScale::SHE;
    double pH_min = 0.0;
    double pH_max = 14.0;
    double Eh_min = -2.0;
    double Eh_max = 2.0;

    bool usar_davies = false;
    double fuerza_ionica = 0.0;
    double factor_correccion = 1.0;
    bool usar_criss_cobble = false;

    bool override_constante_dielectrica = false;
    double constante_dielectrica_manual = 78.382;
    bool override_dG_agua = false;
    double dG_agua_manual_kcal = -56.678;

    bool mostrar_areas_iones = false;
    bool mostrar_colores = false;
};

struct EpHDiagramResult {
    bool ok = false;
    std::string mensaje_error;
    std::string titulo_diagrama;
    std::string elemento_principal;
    std::vector<std::string> elementos_secundarios;
    std::vector<EpHElementConfig> elementos_usados;

    double T_K = 298.15;
    double T_C = 25.0;
    double presion_total_bar = 1.0;
    double presion_vapor_agua_bar = 0.03169;
    double constante_dielectrica = 78.382;
    double dG_H2O_kJ = -237.141;
    double dG_H2O_kcal = -56.678;
    double fuerza_ionica = 0.0;
    double factor_correccion_davies = 1.0;

    double pH_min = 0.0;
    double pH_max = 14.0;
    double Eh_min = -2.0;
    double Eh_max = 2.0;
    PourbaixPotentialScale escala_potencial = PourbaixPotentialScale::SHE;

    double eh_agua_sup_pHmin = 1.229;
    double eh_agua_sup_pHmax = 0.401;
    double eh_agua_inf_pHmin = 0.000;
    double eh_agua_inf_pHmax = -0.828;

    std::vector<EpHCandidateSpecies> especies_tabla;
    std::vector<EpHPoligonoPredominancia> poligonos;
    std::vector<EpHSegmentoFrontera> fronteras;

    // Segundo diagrama superpuesto solo con especies acuosas (Show predominance areas of ions)
    std::vector<EpHPoligonoPredominancia> poligonos_iones;
    std::vector<EpHSegmentoFrontera> fronteras_iones;

    std::vector<EpHPuntoTriple> puntos_triples;
    std::vector<std::string> advertencias;
};

struct EpHCombinerDataset {
    std::string etiqueta;
    double temperatura_C = 25.0;
    double molalidad_principal = 1.0;
    bool activo = true;
    EpHDiagramResult resultado;
};

} // namespace cma::domain
