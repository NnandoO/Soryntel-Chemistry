#pragma once
#include "domain/HeatLossTypes.hpp"
#include "domain/ports/ThermalPropertyCatalog.hpp"

#include "application/modules.hpp"
#include "motores/perdida_calor.hpp"

#include <string>
#include <vector>

namespace cma::application::modules {

struct Capa {
    std::string material;
    double espesor = 0.0;
    double k = 0.0;
};

struct ResultadoPerdida {
    double flujo_calor = 0.0;
    double resistencia_total = 0.0;
    std::string mensaje;
    bool ok = false;
};

// Catálogo de propiedades térmicas por defecto (Conduction, Convection, Surface Radiation)
class DefaultThermalPropertyCatalog final : public cma::domain::ThermalPropertyCatalog {
  public:
    DefaultThermalPropertyCatalog();

    const std::vector<cma::domain::ConductionMaterialEntry>& conduction_materials() const override {
        return conduction_;
    }
    const std::vector<cma::domain::ConvectionFluidEntry>& convection_fluids() const override {
        return convection_;
    }
    const std::vector<cma::domain::SurfaceRadiationEntry>& surface_materials() const override {
        return radiation_;
    }

    const cma::domain::ConductionMaterialEntry* find_conduction(const std::string& name) const override;
    const cma::domain::ConvectionFluidEntry* find_convection(const std::string& name) const override;
    const cma::domain::SurfaceRadiationEntry* find_surface(const std::string& name) const override;

    void add_or_update_conduction(const cma::domain::ConductionMaterialEntry& entry);
    void add_or_update_surface(const cma::domain::SurfaceRadiationEntry& entry);
    void restore_defaults();

  private:
    std::vector<cma::domain::ConductionMaterialEntry> conduction_;
    std::vector<cma::domain::ConvectionFluidEntry> convection_;
    std::vector<cma::domain::SurfaceRadiationEntry> radiation_;
};

// Servicio de aplicación para el módulo HSC Heat Loss (Multi-hoja, Catálogos y Solvers)
class HeatLossCalc {
  public:
    HeatLossCalc();

    // Compatibilidad hacia atrás
    ResultadoPerdida plano(double T_int, double T_ext, double area, const std::vector<Capa>& capas);

    // Acceso al catálogo de propiedades térmicas (puerto de dominio)
    DefaultThermalPropertyCatalog& catalog() { return catalog_; }
    const DefaultThermalPropertyCatalog& catalog() const { return catalog_; }

    // Gestión de hojas (Wall, Roof, Bottom, etc.)
    std::vector<cma::domain::ThermalSheet>& sheets() { return sheets_; }
    const std::vector<cma::domain::ThermalSheet>& sheets() const { return sheets_; }

    int active_sheet_index() const { return active_sheet_; }
    void set_active_sheet_index(int idx);
    cma::domain::ThermalSheet& active_sheet();
    const cma::domain::ThermalSheet& active_sheet() const;

    void add_sheet(const std::string& name);
    void duplicate_active_sheet();
    bool remove_sheet(int idx);

    // Presets industriales de referencia HSC
    void load_preset(int preset_id);

    // Solvers sobre la hoja activa o libro completo
    bool solve_active_sheet();
    bool solve_all_sheets();

    cma::domain::TargetThicknessResult
    optimize_layer_thickness(const cma::domain::TargetThicknessRequest& req);

    cma::domain::GasParticleRadiationResult
    calculate_gas_radiation(const cma::domain::GasParticleRadiationInput& input) const;

    double evaluate_surface_convection(const std::string& fluid_name, double t_surf_c, double t_fluid_c,
                                       double length_m, double velocity_m_s,
                                       cma::domain::ConvectionOrientation orient) const;

    // Totales del reactor/equipo combinando todas las hojas
    double total_reactor_heat_flow_kw() const;
    double total_reactor_mass_kg() const;
    double total_outer_area_m2() const;

    // Persistencia y exportación
    bool export_active_profile_csv(const std::string& path) const;
    bool save_workbook_tsv(const std::string& path) const;
    bool load_workbook_tsv(const std::string& path);

  private:
    cma::motores::PerdidaCalor motor_;
    DefaultThermalPropertyCatalog catalog_;
    std::vector<cma::domain::ThermalSheet> sheets_;
    int active_sheet_ = 0;

    static cma::motores::CapaAislante toCapaAislante(const Capa& c);
};

} // namespace cma::application::modules
