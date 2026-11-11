#pragma once
#include "nucleo/tipos.hpp"
#include "diagrama_flujo/flowsheet.hpp"
#include "motores/balance_masa.hpp"
#include "sim/SimOptimization.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include <string>
#include <vector>
#include <map>

namespace cma { namespace interfaz {

// ═══════════════════════════════════════════════════════════════════════════
// PanelSimCommonTools — Módulo 40: HSC Sim - Common Tools
// Incluye:
//   1. Select Unit Models (Sec. 40.2.2, Fig. 27-28)
//   2. Stream Table Editor (Sec. 40.1.4, Fig. 9-10)
//   3. Flowsheet Reports: Unit Balance & Stream Balance (Sec. 40.2.1, Fig. 24-26)
//   4. Static Scenario Editor & Monte Carlo Simulation (Sec. 40.2.3-40.2.4, Fig. 29-34)
//   5. Sim Model Convergence Monitor, Clear Internal Streams & Calculation Order (Fig. 35-40)
// ═══════════════════════════════════════════════════════════════════════════
class PanelSimCommonTools {
public:
    explicit PanelSimCommonTools(const domain::ThermoRepository& db);

    void renderizar(diagrama_flujo::Flowsheet& fs,
                    motores::BalanceMasa& motor_bm,
                    motores::ResultadoBalanceMasa& ultimo_res);

    void abrir_pestana(int idx) {
        visible = true;
        pestana_solicitada_ = idx;
    }

    // Construye el ejemplo de referencia Hydro_example.Sim (Cooler + Leaching, Fig. 24-28)
    void cargar_ejemplo_hydro(diagrama_flujo::Flowsheet& fs,
                              motores::BalanceMasa& motor_bm,
                              motores::ResultadoBalanceMasa& ultimo_res);

    bool visible = false;

private:
    const domain::ThermoRepository& db_;
    int pestana_solicitada_ = -1;

    // ── 1. Select Unit Models (Fig. 27-28) ──
    int nodo_sel_idx_ = 0;
    int cat_lib_idx_  = 0; // 0=Species, 1=Particles, 2=Others, 3=Imported
    int modelo_lib_idx_ = 1;

    // ── 2. Stream Table Editor (Fig. 9-10) ──
    int var_sel_idx_ = 0;
    int stream_preview_idx_ = 0;
    char buf_nombre_set_[64] = "Table Set 1";
    std::map<int64_t, bool> flujos_en_tabla_;

    // ── 3. Reports (Fig. 24-26) ──
    int hoja_reporte_ = 0; // 0=Unit Balance - General, 1=Stream Balance - Hydro, 2=Streams - Properties
    bool mostrar_inputs_ = true;
    bool mostrar_outputs_ = true;
    bool mostrar_intermedios_ = true;
    bool usar_t_h_ = true;
    std::string estado_exportacion_;

    // ── 4. Scenario & Monte Carlo Editor (Fig. 29-34) ──
    struct VariableEscenario {
        int64_t id_objeto = -1; // id de nodo o flujo
        bool es_flujo = false;
        std::string nombre_objeto;
        std::string parametro;  // "temperatura", "presion", "agua_kg_h", "conversion"
        double min_val = 298.15;
        double max_val = 363.15;
        double val_inicial = 333.15;
        int distribucion = 0;   // 0=Uniform, 1=Normal, 2=Laplace, 3=Trapezoid
    };
    struct FilaResultadoEscenario {
        int caso = 1;
        std::vector<double> valores_set;
        double masa_out_t_h = 0.0;
        double entalpia_neta_kW = 0.0;
        double temp_salida_C = 0.0;
        double error_mb_pct = 0.0;
    };
    bool modo_monte_carlo_ = true;
    int num_casos_mc_ = 50;
    int semilla_mc_ = 42;
    int var_hist_idx_ = 0;
    std::vector<VariableEscenario> vars_escenario_;
    std::vector<FilaResultadoEscenario> resultados_escenario_;
    application::modules::ResultadoOptimizacion ultimo_mc_;

    // ── 5. Convergence Monitor & Diagnostics (Fig. 35-40) ──
    int max_iter_conv_ = 100;
    double tol_conv_ = 1e-6;
    std::vector<double> hist_mb_pct_;
    std::vector<double> hist_eb_pct_;
    std::string mensaje_accion_conv_;

    void renderizar_select_unit_models(diagrama_flujo::Flowsheet& fs,
                                       motores::BalanceMasa& motor_bm,
                                       motores::ResultadoBalanceMasa& ultimo_res);
    void renderizar_stream_table_editor(diagrama_flujo::Flowsheet& fs);
    void renderizar_reports(diagrama_flujo::Flowsheet& fs,
                            motores::BalanceMasa& motor_bm,
                            motores::ResultadoBalanceMasa& ultimo_res);
    void renderizar_scenario_monte_carlo(diagrama_flujo::Flowsheet& fs,
                                         motores::BalanceMasa& motor_bm,
                                         motores::ResultadoBalanceMasa& ultimo_res);
    void renderizar_convergence_diagnostics(diagrama_flujo::Flowsheet& fs,
                                            motores::BalanceMasa& motor_bm,
                                            motores::ResultadoBalanceMasa& ultimo_res);
};

}} // namespace cma::interfaz
