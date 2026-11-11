#pragma once
#include "diagrama_flujo/flowsheet.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "motores/lca.hpp"
#include <string>
#include <vector>

namespace cma { namespace interfaz {

// ── Panel de Evaluación LCA (dataset de frontera + mapeo tipo GaBi) ─────────
class PanelLCA {
public:
    explicit PanelLCA(const domain::ThermoRepository* repo = nullptr) : repo_(repo) {}
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void renderizar(diagrama_flujo::Flowsheet* flowsheet = nullptr);
    void reset();

private:
    const domain::ThermoRepository* repo_ = nullptr;
    struct FilaUI {
        int64_t stream_id = -1;
        std::string equipo;
        std::string corriente;
        std::string especie;
        double kg_h = 0.0;
        double fraccion = 0.0;   // % de la masa de la corriente
        bool entrada = true;
        bool es_energia = false;
        std::string equivalente = "no-mapeado";
        bool mapeada() const { return equivalente != "no-mapeado"; }
    };

    // Dataset de frontera (una fila por especie de cada corriente)
    std::vector<FilaUI> filas_;
    int fila_sel_ = -1;
    char busqueda_[96] = "";
    char busqueda_catalogo_[64] = "";
    std::string equivalente_sel_;
    std::string grupo_sel_;
    bool aplicar_a_especie_ = false;

    // Consumos manuales auxiliares (base horaria)
    double manual_electricidad_ = 0.0; // kWh/h
    double manual_gas_ = 0.0;          // Nm³/h
    double manual_combustible_ = 0.0;  // kg/h
    double manual_agua_ = 0.0;         // m³/h
    double manual_residuo_ = 0.0;      // kg/h

    // Unidad funcional: kg/h de producto; 0 = sin normalizar
    double unidad_funcional_kg_ = 0.0;

    // Estado
    motores::LCA motor_;
    motores::ResultadoLCA resultado_;
    diagrama_flujo::Flowsheet* fs_actual_ = nullptr;
    bool calculado_ = false;
    bool cargado_ = false;
    bool mostrar_validacion_ = false;
    bool mostrar_ayuda_ = false;
    int export_pendiente_ = 0;   // 1 = GaBi, 2 = Excel, 3 = JSON
    int pestana_forzada_ = -1;   // 0=Inputs, 1=Outputs, 2=Manual Inputs, 3=Manual Outputs, 4=Indicators
    std::string mensaje_estado_;

    void cargar_flujos(diagrama_flujo::Flowsheet* flowsheet);
    void calcular();
    void auto_mapear();
    void aplicar_mapeo();
    void guardar_mapeos(diagrama_flujo::Flowsheet* flowsheet);
    void dibujar_toolbar(diagrama_flujo::Flowsheet* flowsheet);
    void dibujar_tabla(bool entradas);
    void dibujar_manuales();
    void dibujar_indicadores();
    void dibujar_validacion();
    void dibujar_ayuda();
    void dibujar_catalogo();
    void importar_mapeos(diagrama_flujo::Flowsheet* flowsheet);
    void exportar_gabi(bool excel);
    void exportar_json();
};

}} // namespace cma::interfaz
