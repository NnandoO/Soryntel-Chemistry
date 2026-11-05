#ifndef CMA_INTERFAZ_PANEL_ESTABILIDAD_HPP
#define CMA_INTERFAZ_PANEL_ESTABILIDAD_HPP

#include "diagrams/StabilityDiagrams.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include <string>
#include <vector>

namespace cma { namespace interfaz {

// ── Panel de Diagrama de Estabilidad Tpp (p-p y T-p por Minimización de Gibbs) ──
class PanelEstabilidad {
public:
    explicit PanelEstabilidad(const domain::ThermoRepository& repository);

    void renderizar();
    void cargar_preset(int idx);
    void actualizar_especies_sistema();
    void recalcular();

private:
    const domain::ThermoRepository& repo_;
    cma::application::modules::StabilityDiagrams service_;

    int preset_idx_ = 0;
    char main_element_[16] = "Cu";
    char gas1_element_[16] = "O2";
    char gas2_element_[16] = "SO2";

    int xaxis_var_ = 1; // 0=T(K), 1=log p(Gas1), 2=log p(Gas2)
    int yaxis_var_ = 2; // 0=T(K), 1=log p(Gas1), 2=log p(Gas2)
    double fixed_val_ = 973.15;
    double x_min_ = -20.0, x_max_ = 0.0;
    double y_min_ = -15.0, y_max_ = 2.0;
    int grid_res_ = 140;
    bool show_contour_lines_ = true;
    double probe_x_ = -8.0;
    double probe_y_ = -1.0;

    std::vector<domain::StabilitySpeciesSelection> candidates_;
    domain::TppDiagramRequest last_req_;
    domain::TppDiagramResult result_;
    bool calculado_ = false;
    char csv_export_path_[256] = "hsc_tpp_stability_diagram.csv";
    std::string status_msg_;
};

// ── Panel de Diagrama de Estabilidad Lpp (Vectorial Isotérmico de Kellogg) ──
class PanelDiagramasLpp {
public:
    explicit PanelDiagramasLpp(const domain::ThermoRepository& repository);

    void renderizar();
    void cargar_preset(int idx);
    void actualizar_especies_sistema();
    void recalcular();

private:
    const domain::ThermoRepository& repo_;
    cma::application::modules::StabilityDiagrams service_;

    int preset_idx_ = 0;
    char main_element_[16] = "Cu";
    char gas1_element_[16] = "O2";
    char gas2_element_[16] = "SO2";

    bool use_celsius_ = true;
    double T_input_ = 700.0; // 700 °C = 973.15 K
    double x_min_ = -20.0, x_max_ = 0.0;
    double y_min_ = -15.0, y_max_ = 2.0;
    double probe_x_ = -8.5;
    double probe_y_ = -1.5;
    bool show_triple_points_ = true;
    bool fill_polygons_ = true;

    std::vector<domain::StabilitySpeciesSelection> candidates_;
    domain::LppDiagramResult result_;
    bool calculado_ = false;
    char csv_export_path_[256] = "hsc_lpp_kellogg_diagram.csv";
    std::string status_msg_;
};

}} // namespace cma::interfaz

#endif // CMA_INTERFAZ_PANEL_ESTABILIDAD_HPP
