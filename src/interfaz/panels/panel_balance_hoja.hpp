#pragma once
#include "diagrama_flujo/flowsheet.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "motores/balance_hoja.hpp"
#include <string>
#include <vector>

namespace cma { namespace interfaz {

// ── Panel de Balance de Calor y Materiales (hoja tipo HSC) ──────────────
class PanelBalanceHoja {
public:
    PanelBalanceHoja();
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void renderizar(diagrama_flujo::Flowsheet* flowsheet = nullptr);
    void inicializar_valores_defecto();

private:
    const domain::ThermoRepository* repo_ = nullptr;
    struct FilaUI {
        char formula[64] = "";
        double temper = 25.0;   // en la unidad de temperatura seleccionada
        double presion = 1.0;   // bar
        double kmol = 0.0;
    };

    struct CorrienteUI {
        char nombre[64] = "";
        std::vector<FilaUI> filas;
        double calor = 0.0;     // unidad de energía; > 0 = pérdida al entorno
        bool temperatura_fija = false; // mantener T fija durante Temp. Balance
    };

    struct AreaUI {
        std::string nombre = "Balance Area 1";
        std::vector<CorrienteUI> entradas;
        std::vector<CorrienteUI> salidas;
    };

    enum class Vista { Balance, IN_Global, OUT_Global, Entrada, Salida, Target, InPct, OutPct };

    enum class AccionPendiente {
        Ninguna, BorrarFila, DuplicarFila, MoverArriba, MoverAbajo,
        InsertarDebajo, BorrarCorriente, DuplicarCorriente
    };

    // Datos
    std::vector<CorrienteUI> entradas_;
    std::vector<CorrienteUI> salidas_;
    std::vector<AreaUI> areas_ui_;
    int area_activa_ = 0;

    // Estado de interfaz
    Vista vista_ = Vista::OUT_Global;
    bool lado_salida_sel_ = true;
    int corriente_sel_ = 0;
    int fila_sel_ = 0;
    int col_sel_ = 1;        // 0:Formula, 1:Temp, 2:P, 3:kmol, 4:kg, 5:Nm3
    int temp_unit_ = 0;      // 0: °C, 1: K
    int energy_unit_ = 2;    // 0: Mcal, 1: MJ, 2: kWh, 3: kJ, 4: kcal
    int preset_idx_ = 0;
    bool enable_predictive_typing_ = true;
    bool mantener_offsets_ = false;
    bool show_diagram_ = false;
    bool elem_bal_kg_ = false;
    AccionPendiente accion_pendiente_ = AccionPendiente::Ninguna;

    // Modal buscador de especies
    int modal_corriente_ = -1;
    int modal_fila_ = -1;
    bool modal_es_salida_ = false;

    // Diagrama
    int diag_entrada_ = 0, diag_fila_ = 0;
    int diag_unidad_x_ = 2;  // 0: kmol, 1: kg, 2: Nm³
    bool diag_acoplar_esteq_ = true;
    double diag_min_ = 1.0, diag_max_ = 12.0;
    int diag_puntos_ = 36;
    bool diag_dirty_ = true;
    std::vector<double> diag_x_, diag_y_;

    // Pestaña objetivo / solver / reacción / escalado
    double target_temp_ = 1000.0;
    double calor_requerido_ = 0.0;
    bool calor_calculado_ = false;
    int preheat_stream_idx_ = 0;
    motores::BalanceHoja::ResultadoTemperatura ultimo_preheat_;
    int rxn_sel_idx_ = 5;     // Combustión de metano por defecto
    double rxn_kmol_base_ = 1.0;
    double rxn_conversion_pct_ = 100.0;
    double rxn_exceso_o2_pct_ = 15.0;
    bool rxn_incluir_aire_n2_ = true;
    double scale_factor_ = 10.0;
    double scale_target_kg_ = 1000.0;
    double factor_escala_reactiva_ = 1.0; // Factor de acoplamiento estequiométrico (12.0 en ref HSC)
    double gem_temp_ = 1000.0;
    double gem_pres_bar_ = 1.0;
    std::string gem_mensaje_;
    bool gem_ok_ = false;

    motores::BalanceHoja motor_;
    motores::AreaBalance area_;
    motores::ResultadoBalance resultado_;
    std::vector<motores::BalanceElemento> elem_balance_;
    motores::BalanceHoja::ResultadoTemperatura ultimo_solver_;

    // ── Helpers ──
    motores::AreaBalance area_escalada_operacion() const;
    void calcular_equilibrio_gibbs();
    std::vector<CorrienteUI>& lista(bool salida) { return salida ? salidas_ : entradas_; }
    const std::vector<CorrienteUI>& lista(bool salida) const { return salida ? salidas_ : entradas_; }
    motores::UnidadEnergia unidad_energia() const {
        return static_cast<motores::UnidadEnergia>(energy_unit_);
    }
    motores::UnidadTemperatura unidad_temperatura() const {
        return (temp_unit_ == 0) ? motores::UnidadTemperatura::Celsius
                                 : motores::UnidadTemperatura::Kelvin;
    }
    const char* etiqueta_energia() const;
    const char* etiqueta_temperatura() const;

    void cargar_preset(int idx);
    void cambiar_area_activa(int nueva_area);
    void insertar_area_balance();
    void eliminar_area_balance();
    void sincronizar_motor(bool resolver);
    void recalcular(bool resolver = true);
    void copiar_temperaturas_desde_motor();
    void temp_balance();
    void ajustar_calor();
    void aplicar_accion_pendiente();
    void agregar_corriente(bool salida);
    void cargar_flujos(diagrama_flujo::Flowsheet* fs);
    void abrir_txt();
    void guardar_txt();
    void exportar_csv();
    void copiar_informe();
    std::string generar_informe() const;
    void actualizar_diagrama();

    void dibujar_barra_formula();
    void dibujar_pestana_balance();
    void dibujar_pestana_global_hoja(bool es_salida);
    void dibujar_pestana_porcentaje(bool es_salida);
    void dibujar_pestana_corriente(int idx, bool es_salida);
    void dibujar_pestana_target();
    void dibujar_tabla_corriente(std::vector<CorrienteUI>& grupos, int idx, bool es_salida);
    void dibujar_tabla_elementos();
    void dibujar_resumen_total();
    void dibujar_herramientas();
    void dibujar_diagrama();
};

}} // namespace cma::interfaz
