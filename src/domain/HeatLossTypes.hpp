#pragma once
#include <algorithm>
#include <cmath>
#include <string>
#include <vector>

namespace cma::domain {

enum class ThermalColumnType : int { Surface = 0, SolidLayer = 1, ContactLayer = 2, Enclosure = 3 };

enum class ThermalGeometryType : int {
    PlaneWall = 0,
    Cube = 1,
    Cylinder = 2,
    Sphere = 3,
    HorizontalCylinder = 4
};

enum class ConvectionOrientation : int {
    VerticalSurface = 0,
    HorizontalHotUp = 1,
    HorizontalHotDown = 2,
    HorizontalCylinder = 3,
    Sphere = 4
};

enum class HeatLossSolverMode : int {
    TemperatureProfile = 0, // Fixed heat flow Q + 1 fixed temperature
    HeatFlowTwoTemps = 1    // 2 fixed temperatures -> solve heat flow Q & profile
};

// Polinomio conductividad térmica k(T) = k0 + k1*T_C + k2*T_C^2 + k_inv/(T_C + 273.15)
struct ConductionMaterialEntry {
    std::string name;
    std::string category; // Refractories, Insulating, Steels & Metals, Slags & Skulls, Carbon & SiC, Misc
    double density_kg_m3 = 2000.0;
    double max_service_temp_c = 1500.0;
    double min_temp_c = 0.0;
    double k0 = 1.5;
    double k1 = 0.0;
    double k2 = 0.0;
    double k_inv = 0.0;
    std::string reference;
    bool user_defined = false;

    double evaluate_k(double temp_c) const {
        const double t_clamped = std::clamp(temp_c, -200.0, 3500.0);
        const double t_k = std::max(50.0, t_clamped + 273.15);
        const double val = k0 + k1 * t_clamped + k2 * t_clamped * t_clamped + (k_inv / t_k);
        return std::max(0.005, val);
    }
};

struct ConvectionFluidEntry {
    std::string name;
    std::string category; // Gases, Liquids, Molten Melts
    double pressure_bar = 1.0;
    double default_hc_natural = 8.0;     // W/(m2.K)
    double default_hc_forced = 35.0;     // W/(m2.K)
    double thermal_cond_w_mk = 0.026;    // k_f a T ref
    double kinematic_visc_m2_s = 1.6e-5; // nu a T ref
    double prandtl = 0.71;               // Pr
    double expansion_beta_1_k = 0.0034;  // beta (1/K)
    bool is_gas = true;
    std::string reference;
};

struct SurfaceRadiationEntry {
    std::string name;
    std::string category; // Ideal, Refractories, Oxidized Metals, Polished Metals, Paints & Coatings
    double emissivity_0 = 0.85;
    double emissivity_slope_per_c = 0.0; // d(eps)/dT
    std::string reference;
    bool user_defined = false;

    double evaluate_emissivity(double temp_c) const {
        const double eps = emissivity_0 + emissivity_slope_per_c * std::max(0.0, temp_c);
        return std::clamp(eps, 0.01, 1.0);
    }
};

struct GasParticleRadiationInput {
    double gas_temp_c = 1200.0;
    double wall_temp_c = 950.0;
    double total_pressure_bar = 1.0;
    double mean_beam_length_m = 1.5;
    double p_h2o_vol_pct = 12.0;
    double p_co2_vol_pct = 14.0;
    double p_so2_vol_pct = 8.0;
    double dust_load_g_nm3 = 2.0;
    double wall_emissivity = 0.85;
};

struct GasParticleRadiationResult {
    double gas_emissivity = 0.0;
    double gas_absorptivity = 0.0;
    double particle_emissivity = 0.0;
    double total_emissivity = 0.0;
    double radiative_flux_kw_m2 = 0.0;
    double effective_hr_w_m2k = 0.0;
};

struct ThermalColumn {
    std::string name;
    ThermalColumnType type = ThermalColumnType::SolidLayer;
    std::string material = "Forsterite (Mg2SiO4)";
    std::string surface_material = "Refractory brick (rough)";

    double temp_left_c = 1000.0;
    double temp_right_c = 500.0;
    double thickness_cm = 20.0;
    int grid_points = 10;

    double conductivity_w_mk = 2.0;
    double convection_hc_w_m2k = 10.0;
    double radiation_hr_w_m2k = 0.0;
    double emissivity = 0.85;
    double velocity_m_s = 0.0; // 0 = convección natural, >0 = convección forzada

    double resistance_c_kw = 1.0;
    double heat_flow_kw = 0.0;
    double heat_flux_kw_m2 = 0.0;
    double area_left_m2 = 1.0;
    double area_right_m2 = 1.0;
    double mass_kg = 0.0;
    double max_service_temp_c = 1800.0;
    bool exceeds_service_limit = false;

    // Indicadores de valor fijado (estilo HSC Chemistry)
    bool fix_temp_left = false;
    bool fix_temp_right = false;
    bool fix_conductivity = false;
    bool fix_convection = false;
    bool fix_radiation = false;
    bool fix_resistance = false;
};

struct ThermalProfilePoint {
    double distance_mm = 0.0;
    double temperature_c = 0.0;
    double conductivity_w_mk = 0.0;
    double heat_flux_kw_m2 = 0.0;
    double max_limit_c = 0.0;
    int column_index = 0;
    std::string column_name;
    std::string material_name;
};

struct ThermalSheet {
    std::string name = "Wall";
    ThermalGeometryType geometry = ThermalGeometryType::Cylinder;
    ConvectionOrientation orientation = ConvectionOrientation::VerticalSurface;
    double height_or_length_m = 1.0;
    double inner_diameter_or_width_m = 0.5;
    double total_heat_flow_kw = 13.471;
    HeatLossSolverMode solver_mode = HeatLossSolverMode::HeatFlowTwoTemps;

    std::vector<ThermalColumn> columns;

    // Resultados agregados de la hoja
    double total_resistance_c_kw = 0.0;
    double overall_u_inner_w_m2k = 0.0;
    double overall_u_outer_w_m2k = 0.0;
    double inner_area_m2 = 0.0;
    double outer_area_m2 = 0.0;
    double total_thickness_cm = 0.0;
    double total_mass_kg = 0.0;
    double inner_fluid_temp_c = 0.0;
    double inner_wall_temp_c = 0.0;
    double outer_shell_temp_c = 0.0;
    double ambient_temp_c = 0.0;
    bool converged = false;
    int iterations = 0;
    std::string status_message;
    std::vector<ThermalProfilePoint> profile;
};

struct TargetThicknessRequest {
    int target_column_index = 1;
    bool target_is_outer_temp = true; // true: fijar T_shell ext (°C), false: fijar Q_total (kW)
    double target_value = 85.0;       // °C o kW
    double min_thickness_cm = 0.5;
    double max_thickness_cm = 250.0;
};

struct TargetThicknessResult {
    bool ok = false;
    double optimal_thickness_cm = 0.0;
    double achieved_outer_temp_c = 0.0;
    double achieved_heat_flow_kw = 0.0;
    std::string message;
};

} // namespace cma::domain
