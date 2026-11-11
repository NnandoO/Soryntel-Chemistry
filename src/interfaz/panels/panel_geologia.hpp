#pragma once
#include <map>
#include <string>
#include <vector>
#include "db/MineralDB.hpp"
#include "diagrama_flujo/flowsheet.hpp"
#include "domain/ports/ThermoRepository.hpp"

namespace cma { namespace interfaz {

// ── Panel Geo-Mineralogical Calculations (HSC Geo 8.0) ───────────────────
class PanelGeoMineralogical {
public:
    PanelGeoMineralogical();
    void renderizar();
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    const domain::ThermoRepository* repo_ = nullptr;
    cma::application::modules::MineralDB mineral_db_;

    // Feature activa en la columna izquierda "Features" (0..5):
    // 0 = Mineral database, 1 = Find mineral, 2 = Minerals (Formula Calculator),
    // 3 = Identify mineral (Petrology CIPW), 4 = Modal calculations (por defecto), 5 = Mineral liberation
    int feature_activa_ = 4;
    int tab_modal_ = 0; // 0 = Calculation, 1 = Error Estimation

    // Estado de Modal Calculations (imagen HSC Geo 8.0)
    int idx_data_source_ = 0;
    char data_source_[128] = "Bruvann.xls - XRF";
    int current_method_ = 0; // 0=LS, 1=WLS, 2=NNLS, 3=WNNLS
    std::string elemento_activo_pt_ = "Cu";
    bool element_selected_[118] = {false};

    // Lista de minerales candidatos y rondas de cálculo
    std::vector<std::string> minerales_candidatos_;
    int idx_mineral_cand_ = 1; // "Chalcopyrite (Erora, Portugal)"
    std::vector<cma::application::modules::RondaModal> rondas_;
    int idx_ronda_activa_ = 0;
    int idx_fase_ronda_sel_ = 0;
    int idx_elem_ronda_sel_ = 0;
    std::map<std::string, double> ensayo_actual_;
    cma::application::modules::ResultadoCalculoModal ultimo_resultado_modal_;
    bool modal_calculado_ = false;

    // Panel derecho "Identify" (Sources & Thesaurus)
    bool sources_checked_[7] = {true, true, true, true, true, true, true};
    bool use_thesaurus_ = true;
    bool unidentified_to_ = true;
    char unidentified_label_[64] = "USERX";
    bool only_unidentified_ = false;
    bool source_name_eq_method_ = true;

    // Estado para Mineral Database & Find Mineral
    char filter_query_[256] = "[Cu %] Is greater than 10 AND [Specific Gravity] Is between 2.2 and 4.4";
    int idx_clase_filtro_ = 0;
    float sg_min_ = 1.0f, sg_max_ = 12.0f;
    float mohs_min_ = 1.0f, mohs_max_ = 10.0f;
    int idx_mineral_db_sel_ = 1;

    // Estado para Cálculo de Fórmula Mineral desde Análisis (Feature 2: Minerals)
    std::vector<cma::application::modules::EntradaAnalisisMineral> entradas_formula_;
    char elem_base_norm_[16] = "S";
    double num_atomos_norm_ = 2.0;
    cma::application::modules::ResultadoFormulaMineral res_formula_;

    // Estado para Cálculos Petrológicos CIPW Norm (Feature 3: Identify / Petrology)
    cma::application::modules::AnalisisRocaOxidos roca_oxidos_;
    cma::application::modules::ResultadoPetrologiaCIPW res_cipw_;

    // Popups funcionales
    bool mostrar_datos_ = false;
    bool mostrar_wizard_ = false;
    bool mostrar_ecuaciones_ = false;
    bool mostrar_resultado_ = false;
    bool mostrar_thesaurus_ = false;
    bool mostrar_about_ = false;
    std::string resultado_geo_;

    void inicializar_rondas_bruvann();
    void cargar_fuente_datos(int idx);
    void ejecutar_calculo_modal();
    void refrescar_datos();

    void dibujar_ribbon_superior();
    void dibujar_vista_modal();
    void dibujar_vista_base_datos(bool modo_busqueda_avanzada);
    void dibujar_vista_formula_mineral();
    void dibujar_vista_petrologia_cipw();
    void dibujar_vista_liberacion();
};

// Struct for GPS Module items
struct StockItem {
    char place[64];
    char mat[64];
    float amt;
    float fv;
    float o2;
    float cu;
    float fe;
    float ni;
    float s;
    float x; // GPS X coordinate
    float y; // GPS Y coordinate
    bool selected_for_mix = false;
};

// ── Panel Map-GPS Material Stock ──────────────────────────────────────────
class PanelMapGPS {
public:
    PanelMapGPS();
    void renderizar();
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }
private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    void refrescar_datos();

    int view_mode_ = 0; // 0 = Map, 1 = Stock

    // 2D Camera state
    float zoom_ = 1.0f;
    float camera_x_ = 0.0f;
    float camera_y_ = 0.0f;

    // UI selections
    int maps_idx_ = 0;

    std::vector<StockItem> stock_db_;
    int selected_idx_ = -1; // Currently selected item for the right panel

    // Right panel form data (bound to selected item)
    char place_name_[64] = "";
    char material_[64] = "";
    int amount_tons_ = 0;

    // Map View Mode Dialog state
    bool show_dialog_ = true;
    bool mostrar_transporte_ = false;
    int data_type_ = 2; // 0=Places, 1=Materials, 2=Analysis
    int radius_min_ = 2, radius_max_ = 10;
    int values_min_ = 1, values_max_ = 5000;
};

}} // namespace cma::interfaz
