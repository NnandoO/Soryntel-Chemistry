#pragma once
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace cma::domain {

enum class StabilityAxisVar : int { TemperatureK = 0, LogPressureGas1 = 1, LogPressureGas2 = 2 };

struct StabilitySpeciesSelection {
    int64_t id = -1;
    std::string formula;
    std::string name;
    std::string phase_label;
    double n_main = 0.0;
    double n_elem1 = 0.0;
    double n_elem2 = 0.0;
    double nu_gas1 = 0.0; // Coeficiente estequiométrico de Gas 1 por fórmula
    double nu_gas2 = 0.0; // Coeficiente estequiométrico de Gas 2 por fórmula
    double activity = 1.0;
    bool selected = true;
    bool is_gas = false;
};

struct BalancedPhasePlane {
    int64_t species_id = -1;
    std::string formula;
    double n_main = 1.0;
    double nu_gas1_per_metal = 0.0;
    double nu_gas2_per_metal = 0.0;
    double activity = 1.0;
    double g_std_kj_mol = 0.0;
    // Potencial químico normalizado por mol de metal:
    // G_norm(x, y) = Z + A * log10(p_gas1) + B * log10(p_gas2)
    double Z = 0.0;
    double A = 0.0;
    double B = 0.0;
    std::string formation_reaction;
};

struct PhasePolygonLpp {
    int64_t species_id = -1;
    std::string formula;
    std::string formation_reaction;
    double activity = 1.0;
    std::vector<std::pair<double, double>> vertices;
    double centroid_x = 0.0;
    double centroid_y = 0.0;
    double area_log2 = 0.0;
};

struct BoundaryLineLpp {
    std::string phase_a;
    std::string phase_b;
    std::string reaction_equation;
    double x1 = 0.0;
    double y1 = 0.0;
    double x2 = 0.0;
    double y2 = 0.0;
    double slope_dy_dx = 0.0;
    double log_k = 0.0;
};

struct TriplePointLpp {
    std::string phase_a;
    std::string phase_b;
    std::string phase_c;
    double log_p_gas1 = 0.0;
    double log_p_gas2 = 0.0;
};

struct LppDiagramRequest {
    std::string main_element = "Cu";
    std::string gas1_formula = "O2";
    std::string gas2_formula = "SO2";
    double temperature_k = 973.15; // 700 °C
    double x_min = -20.0;
    double x_max = 0.0;
    double y_min = -15.0;
    double y_max = 2.0;
    double probe_log_p1 = -10.0;
    double probe_log_p2 = -2.0;
    std::vector<StabilitySpeciesSelection> candidates;
};

struct LppDiagramResult {
    bool ok = false;
    std::string message;
    std::string system_title;
    double temperature_k = 973.15;
    std::vector<BalancedPhasePlane> planes;
    std::vector<PhasePolygonLpp> polygons;
    std::vector<BoundaryLineLpp> boundaries;
    std::vector<TriplePointLpp> triple_points;
    std::string probe_predominant_phase;
    double probe_min_potential_kj = 0.0;
};

struct TppLegendEntry {
    int64_t species_id = -1;
    int color_index = 0;
    std::string formula;
    double area_fraction_pct = 0.0;
};

struct TppContourSegment {
    double x1 = 0.0;
    double y1 = 0.0;
    double x2 = 0.0;
    double y2 = 0.0;
    std::string phase_a;
    std::string phase_b;
};

struct TppDiagramRequest {
    std::string main_element = "Cu";
    std::string gas1_formula = "O2";
    std::string gas2_formula = "SO2";
    StabilityAxisVar x_var = StabilityAxisVar::LogPressureGas1;
    StabilityAxisVar y_var = StabilityAxisVar::LogPressureGas2;
    double fixed_value = 973.15; // K si X/Y son presiones, o log10(p) si uno de los ejes es T
    double x_min = -20.0;
    double x_max = 0.0;
    double y_min = -15.0;
    double y_max = 2.0;
    int rows = 140;
    int cols = 140;
    double probe_x = -10.0;
    double probe_y = -2.0;
    std::vector<StabilitySpeciesSelection> candidates;
};

struct TppDiagramResult {
    bool ok = false;
    std::string message;
    std::string system_title;
    int rows = 0;
    int cols = 0;
    std::vector<int> phase_index_grid; // Índice local en legend (0..K-1) para cada celda [r * cols + c]
    std::vector<TppLegendEntry> legend;
    std::vector<TppContourSegment> contours;
    std::string probe_predominant_phase;
    double probe_min_potential_kj = 0.0;
};

} // namespace cma::domain
