#pragma once
#include <cstdint>
#include <imgui.h>
#include <string>
#include <vector>
#include "domain/ports/ThermoRepository.hpp"
#include "nucleo/tipos.hpp"
#include "thermo/ReactionSolver.hpp"
#include "tools/ExternalAPI.hpp"

namespace cma { namespace interfaz {

// ── Panel de ReactionSolver ─────────────────────────────────────────────
class PanelReactionSolver {
public:
    explicit PanelReactionSolver(const domain::ThermoRepository& repository) : repo_(&repository), motor_(repository) {}
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void renderizar();
private:
    const domain::ThermoRepository* repo_ = nullptr;
    char ecuacion_[128] = "2CO(g) + O2(g) = 2CO2(g)";
    double temp_desde_ = 0.0;     // °C
    double temp_hasta_ = 1200.0;  // °C
    double temp_paso_ = 25.0;     // °C
    double smallest_coefficient_ = 1.0;

    int y_axis_mode_ = 0; // Reacción: 0:LogK 1:dH 2:dS 3:dG 4:E° | Fórmula: 0:Cp 1:H 2:S 3:G
    bool collect_data_ = true;
    bool plot_all_data_ = true;

    double conversion_ = 1.0;
    double presion_ = 1.0;
    bool calculado_ = false;
    double especie_mol_ = 1.0;      // Input amount (mol) de la tabla Species Data
    bool ventana_grafico_ = false;  // "Open in new Window"

    // ── Curvas recolectadas (Collect Data) ──
    struct Curva {
        std::string etiqueta;         // p.ej. "R1: 2CO(g) + O2(g) = 2CO2(g)"
        bool para_reaccion = true;
        bool visible = true;
        cma::nucleo::Reaccion rxn;    // si es reacción
        int64_t especie_id = -1;      // si es fórmula
        std::vector<double> xs;       // T (°C)
    };
    std::vector<Curva> curvas_;

    // ── Tabs estilo HSC: R1..R3 para reacciones, S1..S3 para fórmulas ──
    struct TabEstado {
        std::string ecuacion;
        double desde = 0.0, hasta = 1200.0, paso = 25.0;
    };
    TabEstado tabs_[3];
    int tab_actual_ = 0;
    bool tabs_inicializados_ = false;

    // ── Estado del último cálculo ──
    cma::nucleo::Reaccion rxn_actual_;
    std::string error_msg_;
    std::string balance_detalle_;
    bool error_parsing_ = false;
    bool balance_ok_ = true;

    cma::application::modules::ReactionSolver motor_;

    void balancear_ecuacion();
    void guardar_tab_actual();
    void cargar_tab(int idx);
    void dibujar_grafico(const ImVec2& tam, bool para_reaccion);
};

// ── Panel de ExternalAPI ────────────────────────────────────────────────
class PanelExternalAPI {
public:
    void renderizar();
private:
    char comando_[256] = "echo Hola desde CMA Chemistry";
    char resultado_[4096] = {};
    bool ejecutado_ = false;
    cma::application::modules::ExternalAPI motor_;
};

// ── Panel de Gráficos 3D ─────────────────────────────────────────────────
struct ImVec3 { float x, y, z; ImVec3() : x(0),y(0),z(0) {} ImVec3(float _x,float _y,float _z):x(_x),y(_y),z(_z){} };
class PanelGrafico3D {
public:
    void renderizar();
private:
    float ang_x_ = 0.52f, ang_y_ = 0.78f, distancia_ = 5.0f;
    bool auto_rotar_ = false;
    int tipo_grafico_ = 0; // 0=superficie, 1=líneas, 2=puntos, 3=contorno
    std::vector<float> data_x_, data_y_, data_z_;
    bool datos_generados_ = false;
    void generar_datos_ejemplo();
    ImVec3 proyectar(float x, float y, float z, const ImVec2& centro, float escala) const;
};

// ── Panel de HSC Add-In Functions ──────────────────────────────────────────
class PanelAddIn {
public:
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void renderizar();
private:
    const domain::ThermoRepository* repo_ = nullptr;
    struct FilaAddIn {
        int func_idx = 0;
        char arg1[128] = {};
        double arg2 = 25.0;
        double arg3 = 0.0;
        double arg4 = 0.0;
        char desc[128] = {};
        double resultado = 0.0;
        bool calculo_ok = false;
        std::string mensaje;
    };
    std::vector<FilaAddIn> filas_;
    bool db_inicializada_ = false;

    void evaluar_fila(FilaAddIn& fila);
};

}} // namespace cma::interfaz
