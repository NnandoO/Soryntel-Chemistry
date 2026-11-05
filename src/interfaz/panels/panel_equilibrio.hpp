#pragma once
#include <atomic>
#include <cstdint>
#include <imgui.h>
#include <map>
#include <string>
#include <thread>
#include <vector>
#include "domain/ports/ThermoRepository.hpp"
#include "nucleo/tipos.hpp"

namespace cma { namespace interfaz {

// ── Panel de Equilibrio de Fases ────────────────────────────────────────────
class PanelEquilibrioFases {
public:
    ~PanelEquilibrioFases() {
        calc_cancelled_ = true;
        if (calc_thread_.joinable()) calc_thread_.join();
    }

    void renderizar(const std::map<int64_t, nucleo::Especie>& db);

private:
    struct EqSpecies {
        int64_t id;
        std::string name;
        std::string formula;
        std::string phase;
        double initial_amount = 0.0;
        int phase_num = 1;           // 1 = gas, 2..n = fases condensadas
        double activity_coeff = 1.0; // γ_i (coeficiente de actividad)
    };
    std::vector<EqSpecies> species_list_;
    char add_species_buf_[64] = "";
    double add_species_amt_ = 0.0;
    int modelo_actividad_ = 0; // 0 = Ideal (γ=1), 1 = Manual, 2 = Raoult/Davies auto

    // Selección por elementos (HSC) y modo celda
    char elementos_buf_[128] = "";          // p.ej. "Na, S, O, C, N"
    bool todos_elementos_ = false;          // "All elements must exist"
    bool modo_celda_ = false;               // Cell equilibrium (carga neta 0)
    char titulo_chart_[128] = "GEM System"; // título del gráfico (como el nombre de archivo en HSC)

    // Ventana de gráfico estilo HSC
    bool mostrar_parametros_ = true;        // panel "Chart parameters" (menú Show)
    bool mostrar_acerca_ = false;           // popup Help > About
    int  modo_vista_chart_ = 0;             // 0 = 3D Surface (T×P), 1 = 2D Curves (corte)

    // Genera las especies candidatas a partir de elementos_buf_ (botón Generate / preset)
    void generar_especies_por_elementos(const std::map<int64_t, nucleo::Especie>& db);
    // Precarga el ejemplo Na2SO4 de la imagen HSC (elementos Na,S,O,C,N + barrido T×P)
    void cargar_ejemplo_na2so4(const std::map<int64_t, nucleo::Especie>& db);

    // For 3D Plot we sweep both T and P (entradas en °C y bar, estilo HSC)
    double t_start_ = 200.0;   // °C
    double t_end_ = 1000.0;    // °C
    int t_steps_ = 20;

    double p_start_ = 1.0;     // bar
    double p_end_ = 10.0;      // bar
    int p_steps_ = 10;

    std::vector<double> x_values_; // Temperature
    std::vector<double> y_values_; // Pressure
    std::map<int64_t, std::vector<double>> z_values_; // key: species ID, value: amounts at each (T,P) grid point

    struct ChartSurface {
        int64_t id;
        std::string legend;
        std::string fase;      // nombre de fase ("Gas", "Sólido", ...)
        int fase_num = 1;      // nº de fase (1 = gas, 2..n condensadas)
        std::vector<double> z_data;
        bool visible = true;
        ImVec4 color = ImVec4(0.2f, 0.7f, 0.3f, 1.0f);
    };
    std::vector<ChartSurface> chart_surfaces_;

    bool calculado_ = false;
    bool show_chart_window_ = false;
    bool show_table_window_ = false;
    int  table_p_index_ = 0;   // fila de presión mostrada en la tabla 2D / gráfico 2D
    std::string error_msg_;

    // Energía de Gibbs total por punto del grid (para la tabla)
    std::vector<double> g_values_;
    std::vector<double> temp_g_values_;

    std::atomic<bool> is_calculating_{false};
    std::atomic<float> calc_progress_{0.0f};
    std::atomic<bool> calculation_done_{false};
    std::atomic<bool> calc_cancelled_{false};

    // Valid dimensions of the current plot data
    int plot_t_steps_ = 0;
    int plot_p_steps_ = 0;
    int temp_plot_t_steps_ = 0;
    int temp_plot_p_steps_ = 0;

    // ── Mejoras estilo HSC (imagen de referencia GEM) ──
    // Variables barridas en X e Y (por defecto: X=Temperatura, Y=Presión)
    enum class VarBarrido { Temperatura, Presion, CantidadEspecie };
    VarBarrido var_x_ = VarBarrido::Temperatura;   // eje X
    VarBarrido var_y_ = VarBarrido::Presion;       // eje Y
    int especie_barrida_ = 0;                       // índice en species_list_ cuando CantidadEspecie
    double t_fijo_ = 500.0;                         // °C si Temperatura no se barre
    double p_fijo_ = 1.0;                           // bar si Presión no se barre
    double feed_start_ = 0.1, feed_end_ = 2.0;      // kmol, si CantidadEspecie se barre
    int feed_steps_ = 10;

    // Ejes del gráfico: escala logarítmica y límites manuales
    bool eje_x_log_ = false, eje_y_log_ = false, eje_z_log_ = false;
    bool eje_x_manual_ = false, eje_y_manual_ = false, eje_z_manual_ = false;
    double x_min_user_ = 0.0, x_max_user_ = 1000.0;
    double y_min_user_ = 0.0, y_max_user_ = 10.0;
    double z_min_user_ = 0.0, z_max_user_ = 1.0;
    int filtro_fase_ = -1;                          // -1 = all phases, 0..n = fase_num (0=todas)

    // Temp storage for async thread results
    std::vector<double> temp_x_values_;
    std::vector<double> temp_y_values_;
    std::map<int64_t, std::vector<double>> temp_z_values_;
    std::vector<ChartSurface> temp_chart_surfaces_;
    std::string temp_error_msg_;
    std::thread calc_thread_;
};

// ── Panel de Equilibrium Calculations (HSC Equilibrium) ──────────────────
class PanelEquilibriumCalc {
public:
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void renderizar();
    void reset();
private:
    const domain::ThermoRepository* repo_ = nullptr;
    struct EspecieInput {
        int64_t id = 0;
        char nombre[64] = {};
        double cantidad = 0.0;
        bool activo = true;
    };
    std::vector<EspecieInput> especies_;
    char nuevo_nombre_[64] = {};
    double nueva_cantidad_ = 1.0;
    bool mostrar_selector_ = false;

    double t_inicio_ = 500.0;  // K
    double t_fin_ = 1500.0;    // K
    int num_pasos_ = 20;
    double presion_ = 1.0;     // atm

    bool modo_celda_ = false;
    bool calcular_ = false;
    bool calculado_ = false;
    bool convergio_ = false;
    double energia_total_ = 0.0;

    struct PuntoEquilibrio {
        double temperatura;
        std::map<int64_t, double> composicion; // especie_id -> kmol
        double G_total;
    };
    std::vector<PuntoEquilibrio> resultados_;
    std::map<int64_t, std::string> nom_especies_; // id -> nombre

    // Ejes
    bool escala_log_ = false;
    double eje_y_min_ = 0.0;
    double eje_y_max_ = 0.0;
    bool auto_escala_ = true;

    void agregar_especie_desde_bd(const std::string& nombre, double cant);
    void ejecutar_calculo(const std::map<int64_t, cma::nucleo::Especie>& db);
    void renderizar_selector_especies(const std::map<int64_t, cma::nucleo::Especie>& db);
};

}} // namespace cma::interfaz
