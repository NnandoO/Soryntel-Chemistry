#ifndef CMA_INTERFAZ_PANEL_PERDIDA_CALOR_HPP
#define CMA_INTERFAZ_PANEL_PERDIDA_CALOR_HPP

#include "diagrama_flujo/flowsheet.hpp"
#include "tools/HeatLossCalc.hpp"
#include <functional>
#include <string>

namespace cma { namespace interfaz {

class PanelPerdidaCalor {
public:
    PanelPerdidaCalor();

    void renderizar();
    void inicializar_valores_defecto();
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }

    cma::application::modules::HeatLossCalc& servicio() { return service_; }
    const cma::application::modules::HeatLossCalc& servicio() const { return service_; }

    // Callback opcional para transferir el flujo de calor total al módulo HSC Balance
    std::function<void(double heat_loss_kw, const std::string& zone_name)> al_enviar_a_balance;

private:
    cma::application::modules::HeatLossCalc service_;
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;

    // Estado de selección en la hoja de cálculo
    int selected_col_ = 1;
    int active_main_tab_ = 0; // 0: Worksheet, 1: Temperature Profile & Table, 2: Thermal Databases, 3: Calculators & Target Solver, 4: Summary & Sim Link
    int preset_idx_ = 0;
    char new_sheet_name_[64] = "Side Wall 2";

    // Ventana flotante opcional del diagrama
    bool show_diagram_popup_ = false;
    bool show_table_popup_ = false;

    // Estado del optimizador de espesor objetivo (Target Value Solver)
    cma::domain::TargetThicknessRequest target_req_;
    cma::domain::TargetThicknessResult target_res_;
    bool target_solved_ = false;

    // Estado de la calculadora de radiación de gases y partículas (Hottel)
    cma::domain::GasParticleRadiationInput gas_rad_in_;
    cma::domain::GasParticleRadiationResult gas_rad_res_;
    int gas_rad_target_col_ = 0;

    // Estado de base de datos térmica (filtros y alta de material de usuario)
    int db_subtab_ = 0; // 0: Conduction, 1: Convection, 2: Surface Radiation
    char db_search_[128] = "";
    int selected_cond_idx_ = 0;
    char user_mat_name_[64] = "Custom Magnesia Castable";
    char user_mat_cat_[32] = "Refractories";
    double user_mat_rho_ = 2750.0;
    double user_mat_tmax_ = 1700.0;
    double user_mat_k0_ = 2.80;
    double user_mat_k1_ = -0.0004;
    double user_mat_k2_ = 0.0;
    char user_mat_ref_[96] = "Plant refractory data sheet";

    // Exportación / Persistencia e integración Flowsheet
    char workbook_path_[256] = "hsc_heat_loss_workbook.tsv";
    char csv_profile_path_[256] = "heat_loss_temperature_profile.csv";
    int target_node_idx_ = 0;
    std::string status_banner_;
    bool status_ok_ = true;

    void renderizar_barra_superior();
    void renderizar_pestanas_hojas();
    void renderizar_hoja_calculo(cma::domain::ThermalSheet& sheet);
    void renderizar_seccion_transversal(const cma::domain::ThermalSheet& sheet);
    void renderizar_panel_inferior_controles(cma::domain::ThermalSheet& sheet);
    void renderizar_pestana_perfil_y_tabla(const cma::domain::ThermalSheet& sheet);
    void renderizar_pestana_bases_datos(cma::domain::ThermalSheet& sheet);
    void renderizar_pestana_calculadoras(cma::domain::ThermalSheet& sheet);
    void renderizar_pestana_resumen_reactor();
    void renderizar_popups_flotantes(const cma::domain::ThermalSheet& sheet);
};

}} // namespace cma::interfaz

#endif // CMA_INTERFAZ_PANEL_PERDIDA_CALOR_HPP
