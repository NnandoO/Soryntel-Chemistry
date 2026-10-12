#pragma once
#include <thread>
#include <atomic>
#include "motores/balance_exergia.hpp"
#include "nucleo/tipos.hpp"
#include "nucleo/tabla_periodica.hpp"
#include "diagrama_flujo/flowsheet.hpp"
#include "base_datos/gestor_db.hpp"
#include "motores/balance_masa.hpp"
#include "motores/balance_energia.hpp"
#include "motores/equilibrio_fases.hpp"
#include "motores/electroquimica.hpp"
#include "motores/cinetica.hpp"
#include "motores/destilacion.hpp"
#include "motores/minimizador_gibbs.hpp"
#include "motores/simulacion_dinamica.hpp"
#include "motores/reconciliacion_datos.hpp"
#include "motores/exergia.hpp"
#include "../diagrams/PropertyDiagrams.hpp"
#include "../diagrams/StabilityDiagrams.hpp"

#include "motores/agua.hpp"
#include "motores/soluciones_acuosas.hpp"
#include "motores/perdida_calor.hpp"
#include "motores/estimador_propiedades.hpp"
#include "motores/lca.hpp"
#include "thermo/ReactionSolver.hpp"
#include "tools/SpeciesConverter.hpp"
#include "tools/DataFitter.hpp"
#include "tools/ExternalAPI.hpp"
#include "motores/pourbaix.hpp"
#include <imgui.h>
#include <functional>

namespace cma { namespace interfaz {

// ── Modal Buscador Universal de la Base de Datos ────────────────────────────
class ModalBuscadorDB {
public:
    static ModalBuscadorDB& instancia() {
        static ModalBuscadorDB inst;
        return inst;
    }
    
    void abrir(const char* titulo_popup);
    bool dibujar(const char* titulo_popup, std::vector<const nucleo::Especie*>& seleccionados, bool seleccion_multiple = true);

private:
    ModalBuscadorDB() = default;
    char search_buffer_[256] = "";
    std::map<int64_t, bool> checks_;
};

// ── Panel de Caja de Herramientas ───────────────────────────────────────────
class PanelHerramientas {
public:
    void renderizar();
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }

    std::function<void(nucleo::TipoNodo)> al_agregar_nodo;
    std::function<void()> al_abrir_tabla;
    std::function<void()> al_abrir_calculadora;
    std::function<void()> al_abrir_electroquimica;
    std::function<void()> al_abrir_cinetica;
    std::function<void()> al_abrir_equilibrio;
    std::function<void(int64_t)> al_seleccionar_nodo;
    std::function<void()> al_abrir_anotacion;
    std::function<void()> al_agregar_tabla_flujo;
    std::function<void()> al_ejecutar_simulacion;
    std::function<void()> al_ejecutar_convergencia;

    // Modo de dibujo
    enum class ModoDibujo { Seleccion, DibujarFlujo, Texto, Rectangulo, Elipse };
    ModoDibujo modo_dibujo = ModoDibujo::Seleccion;

private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    int pestaña_activa_ = 0; // 0=Explorador, 1=Paleta, 2=Herramientas

    void renderizar_explorador();
    void renderizar_paleta();
    void renderizar_herramientas_dibujo();
    void renderizar_categoria(const char* nombre, const nucleo::TipoNodo* tipos, int count);
};

// ── Panel de Propiedades ────────────────────────────────────────────────────
class PanelPropiedades {
public:
    PanelPropiedades();

    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }
    void seleccionar_nodo(int64_t id);
    void seleccionar_flujo(int64_t id);
    void seleccionar_anotacion(int64_t id);
    void renderizar();
    std::function<void()> al_cambio_flowsheet;
    std::function<void()> al_antes_de_modificar;  // hook para guardar estado antes de undo

private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    int64_t id_nodo_   = -1;
    int64_t id_flujo_  = -1;
    int64_t id_anotacion_ = -1;
    bool forzar_seleccion_proceso_ = false;
    bool forzar_seleccion_dibujos_ = false;

    void renderizar_contenido_proceso();
    void renderizar_contenido_dibujos();
    void propiedades_nodo(nucleo::Nodo& nodo);
    void propiedades_flujo(nucleo::Flujo& flujo);
    void propiedades_anotacion(diagrama_flujo::Anotacion& anotacion);
    void propiedades_nodo_especificas(nucleo::Nodo& nodo);
    void propiedades_flujo_especificas(nucleo::Flujo& flujo);

    // Selector de especies para unidades de separación (parámetro "split_<id>"
    // = fracción de la especie hacia la SALIDA 1). Usa el buscador de BD.
    void dibujar_selector_splits(nucleo::Nodo& nodo, const char* titulo);
};

// ── Panel de Navegador de Especies ──────────────────────────────────────────
class PanelEspecies {
public:
    PanelEspecies();
    void renderizar();
    void refrescar();

    std::function<void(const nucleo::Especie&)> al_seleccionar_especie;
    const nucleo::Especie* especie_seleccionada() const { return seleccionada_; }

private:
    std::vector<const nucleo::Especie*> lista_;
    const nucleo::Especie* seleccionada_ = nullptr;
    char busqueda_[64] = {};
    bool necesita_refresh_ = true;
};

// ── Panel de Resultados ─────────────────────────────────────────────────────

class PanelResultados {
public:
    void renderizar();

    void poner_resultado_masa(const std::string& msg, bool ok);
    void poner_resultado_energia(const std::string& msg, bool ok);
    void poner_resultado_gibbs(const std::string& msg, bool ok,
                               const std::map<int64_t, double>& comp,
                               const std::map<int64_t, nucleo::Especie>& db);
    void poner_resultado_exergia(const motores::ResultadoBalanceExergia& res, bool ok, const std::string& msg);

    void limpiar();

private:
    struct {
        std::string mensaje; bool ok = false;
    } masa_, energia_, gibbs_;
    struct {
        std::map<int64_t, double> composicion;
        std::map<int64_t, nucleo::Especie> db;
    } datos_gibbs_;
    struct {
        motores::ResultadoBalanceExergia res;
        bool ok = false;
        std::string mensaje;
    } exergia_;
    char reporte_[32768] = {};

    void generar_reporte();
};

// ── Panel de Calculadora Termodinámica ──────────────────────────────────────
class PanelCalculadora {
public:
    void renderizar(const std::map<int64_t, nucleo::Especie>& db);

private:
    int64_t id_especie_ = -1;
    double temperatura_ = 1000.0;
    double presion_ = 1.0;
    double T_min_ = 298.0, T_max_ = 2000.0;
    char busqueda_[64] = {};
    int modo_ = 0; // 0=simple, 1=gráfico
};

// ── Panel de Tabla Periódica ────────────────────────────────────────────────
class PanelTablaPeriodica {
public:
    PanelTablaPeriodica();
    void renderizar();
    std::function<void(const nucleo::Elemento&)> al_seleccionar_elemento;

private:
    const nucleo::Elemento* seleccionado_ = nullptr;
    float tam_celda_ = 54.0f;
    bool mostrar_leyenda_ = true;
    char busqueda_[64] = "";
    bool buscar_ = false;
    
    // Nuevas variables
    int vista_actual_ = 0; // 0 = Chart, 1 = Database
    char prop_seleccionada_[128] = "Electronegativity";
    bool db_inicializada_ = false;
    char filename_[256] = "elementos_db.json";
    
    void renderizar_elemento(int Z, const ImVec2& pos, const ImVec2& tam);
    void renderizar_leyenda(const ImVec2& pos);
};

// ── Panel de Electroquímica ─────────────────────────────────────────────────
class PanelElectroquimica {
public:
    void renderizar();

private:
    char anodo_[128] = "Zn2+ + 2e- -> Zn";
    char catodo_[128] = "Cu2+ + 2e- -> Cu";
    double T_ = 298.15;
    double E_celda_ = 0.0;
    double delta_G_ = 0.0;
    double K_eq_ = 0.0;
    bool calculado_ = false;
};

// ── Panel de Cinética ──────────────────────────────────────────────────────
class PanelCinetica {
public:
    void renderizar();

private:
    double C0_ = 1.0;
    double k_ = 0.1;
    double tiempo_ = 10.0;
    int orden_ = 1;
    int reactor_ = 0; // 0=batch, 1=CSTR, 2=PFR
    double Cb0_ = 0.0;
    double Cf_ = 0.0;
    double conversion_ = 0.0;
    bool calculado_ = false;
};

// ── Panel de Equilibrio de Fases ────────────────────────────────────────────
class PanelEquilibrioFases {
public:
    ~PanelEquilibrioFases() {
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
    };
    std::vector<EqSpecies> species_list_;
    char add_species_buf_[64] = "";

    // Selección por elementos (HSC) y modo celda
    char elementos_buf_[128] = "";          // p.ej. "Na, S, O, C"
    bool todos_elementos_ = false;          // "All elements must exist"
    bool modo_celda_ = false;               // Cell equilibrium (carga neta 0)
    char titulo_chart_[128] = "GEM System"; // título del gráfico (como el nombre de archivo en HSC)

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
    };
    std::vector<ChartSurface> chart_surfaces_;

    bool calculado_ = false;
    bool show_chart_window_ = false;
    bool show_table_window_ = false;
    int  table_p_index_ = 0;   // fila de presión mostrada en la tabla 2D
    std::string error_msg_;

    // Energía de Gibbs total por punto del grid (para la tabla)
    std::vector<double> g_values_;
    std::vector<double> temp_g_values_;
    
    std::atomic<bool> is_calculating_{false};
    std::atomic<float> calc_progress_{0.0f};
    std::atomic<bool> calculation_done_{false};
    
    // Valid dimensions of the current plot data
    int plot_t_steps_ = 0;
    int plot_p_steps_ = 0;
    int temp_plot_t_steps_ = 0;
    int temp_plot_p_steps_ = 0;
    
    // Temp storage for async thread results
    std::vector<double> temp_x_values_;
    std::vector<double> temp_y_values_;
    std::map<int64_t, std::vector<double>> temp_z_values_;
    std::vector<ChartSurface> temp_chart_surfaces_;
    std::string temp_error_msg_;
    std::thread calc_thread_;
};

// ── Panel de Simulación Dinámica ────────────────────────────────────────
class PanelSimulacionDinamica {
public:
    void renderizar();
private:
    double t_inicial_ = 0, t_final_ = 100, paso_ = 0.1;
    double A_tanque_ = 1.0, A_orificio_ = 0.01, h0_ = 2.0;
    double T0_horno_ = 300, T_amb_ = 298, masa_ = 100, cp_ = 500, hA_ = 50;
    double C0_ = 1.0, k0_ = 0.1, Ea_ = 50000, T_react_ = 500;
    int modo_ = 0; // 0=vaciado, 1=horno, 2=batch
    bool calculado_ = false;
    motores::ResultadoODEs resultado_;
    motores::SimulacionDinamica motor_;
};

// ── Panel de Reconciliación de Datos (Mass Balance) ──────────────────────
struct ReconciliacionUnit {
    std::string name;
    bool selected = true;
    bool psd_changing = false;
};

struct ReconciliacionStream {
    int64_t id = -1;
    std::string name;
    bool selected = true;
    bool assayed = true;
    double measured_flow = 100.0;
    double sd = 5.0;
    double reconciled_flow = 0.0;
    double reconciled_error = 0.0;
};

class PanelReconciliacion {
public:
    void renderizar(diagrama_flujo::Flowsheet* flowsheet = nullptr);
    void reset() { cargado_inicial_ = false; unidades_.clear(); corrientes_.clear(); calculado_ = false; }
private:
    enum class SeccionNavegacion {
        Units, Streams, Variables, SizeFractions, DataSets, MeasurementData,
        StreamView, VariableView, SDStream, SDVariable, Balancing, Reporting
    };
    
    SeccionNavegacion seccion_ = SeccionNavegacion::Units;
    std::vector<ReconciliacionUnit> unidades_;
    std::vector<ReconciliacionStream> corrientes_;
    int fila_seleccionada_ = -1;
    int tipo_problema_ = 0; // 0: Solids, 1: Assays Only, 2: Solids & Water Independently
    bool calculado_ = false;
    bool cargado_inicial_ = false;
    
    motores::ReconciliacionDatos motor_;
    motores::ResultadoReconciliacion resultado_;

    void inicializar_datos(diagrama_flujo::Flowsheet* fs);
    void ejecutar_balance();
};

// ── Panel de Balance Hoja de Cálculo ─────────────────────────────────────
class PanelBalanceHoja {
public:
    PanelBalanceHoja();
    void renderizar(diagrama_flujo::Flowsheet* flowsheet = nullptr);
    void inicializar_valores_defecto();

private:
    struct SpeciesRow {
        char formula[64] = "";
        double temper = 25.0;     // in selected temperature unit
        double pressure = 1.0;    // bar
        double amount_kmol = 0.0;
        double amount_kg = 0.0;
        double amount_nm3 = 0.0;
        double heat_content = 0.0; // selected energy unit / kmol
        double total_h = 0.0;      // selected energy unit
    };

    struct StreamGroup {
        char nombre[64] = "";
        std::vector<SpeciesRow> filas;
        double heat_loss = 0.0;   // in selected energy unit
    };

    std::vector<StreamGroup> entradas_;
    std::vector<StreamGroup> salidas_;

    int tab_actual_ = 1; // 0: BALANCE, 1: IN1, 2: OUT1, 3: Target
    int temp_unit_ = 0;  // 0: °C, 1: K
    int energy_unit_ = 2; // 0: Mcal, 1: MJ, 2: kWh
    bool enable_predictive_typing_ = true;
    
    int modal_target_stream_ = -1;
    int modal_target_row_ = -1;
    bool show_diagram_ = false;

    // Active sheet selection
    int selected_stream_idx_ = 0;
    int selected_row_idx_ = 0;

    void reciclar_calculos();
    double calcular_entalpia_corrientes(const std::vector<StreamGroup>& grupos, double t_out_forced = -9999.0);
    void Temp_Balance();

    struct ElementBal {
        std::string element;
        double in_val = 0.0;
        double out_val = 0.0;
        double balance = 0.0;
    };
    std::vector<ElementBal> elem_balance_;
    void calcular_balance_elemental();
};

// ── Panel de Exergía ────────────────────────────────────────────────────
class PanelExergia {
public:
    void renderizar();
private:
    double h_ent_ = 100, h_sal_ = 80, h0_ = 50;
    double T_ent_ = 600, T_sal_ = 400, T0_ = 298;
    double s_ent_ = 0.5, s_sal_ = 0.4, s0_ = 0.3;
    double W_util_ = 10, Q_aporte_ = 50;
    motores::Exergia motor_;
    motores::ResultadoExergia resultado_;
    bool calculado_ = false;
};

// ── Panel de Diagramas (H, S, Cp, G vs T) ───────────────────────────────
class PanelDiagramasTermo {
public:
    void renderizar();
private:
    double T_min_ = 298, T_max_ = 2000;
    int curva_activa_ = 0; // 0=H, 1=S, 2=Cp, 3=G, 4=Ellingham
    hsc_clone::PropertyDiagrams motor_;
    
    // UI state for multiple species
    bool show_select_species_modal_ = false;
    char search_buffer_[256] = "";
    std::map<int64_t, bool> especies_seleccionadas_;
    
    struct CurvaEspecie {
        std::string nombre;
        std::vector<hsc_clone::PuntoPropiedad> datos;
        std::vector<double> Ts_cache;
        std::vector<double> Ys_cache;
    };
    std::vector<CurvaEspecie> datos_calculados_;
    
    bool calculado_ = false;
    bool localizar_ = false;
    
    void recalcular();
};

// ── Panel de Diagrama de Estabilidad (predominancia) ────────────────────
class PanelEstabilidad {
public:
    void renderizar();
private:
    char main_element_[16] = "V";
    char gas1_element_[16] = "O";
    char gas2_element_[16] = "Cl";
    
    int xaxis_var_ = 1; // 0=T, 1=log p(Gas1), 2=log p(Gas2)
    int yaxis_var_ = 2; // 0=T, 1=log p(Gas1), 2=log p(Gas2)
    
    double fixed_val_ = 500.0; // T if X/Y are pressures. log p if one is T.
    
    double x_min_ = -40.0, x_max_ = 0.0;
    double y_min_ = -40.0, y_max_ = 0.0;
    
    std::map<int64_t, bool> especies_seleccionadas_;
    
    struct HeatmapData {
        int rows = 200;
        int cols = 200;
        std::vector<int> grid; // Contains the local species index that is stable at this pixel
        std::map<int, std::string> labels;
    };
    HeatmapData heatmap_;
    bool calculado_ = false;
    
    void recalcular();
};

// ── Panel de Diagrama de Lpp (Vectorial / Kellogg) ────────────────────
class PanelDiagramasLpp {
public:
    void renderizar();
private:
    char main_element_[16] = "Ni";
    char gas1_element_[16] = "O2";
    char gas2_element_[16] = "S2";
    
    double T_ = 300.0;
    
    double x_min_ = -80.0, x_max_ = -50.0;
    double y_min_ = -60.0, y_max_ = -10.0;
    
    std::map<int64_t, bool> especies_seleccionadas_;
    
    struct PoligonoFase {
        int64_t id;
        std::string formula;
        ImU32 color;
        std::vector<ImVec2> vertices;
        ImVec2 centroide;
    };
    std::vector<PoligonoFase> poligonos_;
    bool calculado_ = false;
    
    void recalcular();
};

// ── Panel de Agua (Tablas de Vapor) ─────────────────────────────────────
class PanelAgua {
public:
    void renderizar();
private:
    double T_ = 373.15, P_ = 0.101325;
    double T_min_sat_ = 298.15, T_max_sat_ = 647.0, T_step_sat_ = 10.0;
    motores::Agua motor_;
    motores::PropiedadesAgua resultado_;
    std::vector<motores::PropiedadesAgua> tabla_sat_;
    bool calculado_ = false;
    int modo_ = 0; // 0=propiedades, 1=saturación
};

// ── Panel de Soluciones Acuosas ─────────────────────────────────────────
class PanelSolucionesAcuosas {
public:
    void renderizar();
private:
    struct FilaAqua {
        char especie[128] = {};
        double temp = 40.0;
        double amount_kmol = 0.0;
        double amount_mol_pct = 0.0;
        double amount_g = 0.0;
        double h_ideal = 0.0;
        double h_estimate = 0.0;
        double cp_ideal = 0.0;
        double cp_estimate = 0.0;
        double ac_molar = 1.0;
        double ac_molal = 1.0;
        int carga = 0;
        bool calculo_ok = false;
    };
    
    std::vector<FilaAqua> filas_;
    bool usar_davies_ = false;
    bool usar_pitzer_ = false;
    bool usar_harvie_ = true;
    bool db_inicializada_ = false;

    motores::SolucionesAcuosas motor_;
    void calcular_hoja();
};

// ── Panel Geo-Mineralogical Calculations ─────────────────────────────────
class PanelGeoMineralogical {
public:
    void renderizar();
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }
private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    int tab_activo_ = 0; // 0 = Calculation, 1 = Error Estimation, 2 = Database
    
    // UI State for Modal Calculations
    char data_source_[128] = "Bruvann.xls - XRF";
    int current_method_ = 0; // 0=LS, 1=WLS, 2=NNLS, 3=WNNLS
    
    // UI State for Periodic Table
    bool element_selected_[118] = {false};
    
    // UI State for Database
    char filter_query_[256] = "[Cu %] Is greater than 10 AND [Specific Gravity] Is between 2.2 and 4.4";

    // Popups funcionales
    bool mostrar_datos_ = false;
    bool mostrar_wizard_ = false;
    bool mostrar_resultado_ = false;
    std::string resultado_geo_;

    // Dynamic Database
    struct GeoMineral {
        std::string name;
        std::string symbol;
        std::string formula;
        std::string location;
        std::string ref;
    };
    std::vector<GeoMineral> db_;
    
    void refrescar_datos();
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

// ── Panel de Pérdida de Calor ───────────────────────────────────────────
class PanelPerdidaCalor {
public:
    PanelPerdidaCalor();
    void renderizar();
    void inicializar_valores_defecto();

private:
    struct LayerColumn {
        char name[64] = "";
        int type = 1; // 0: Surface, 1: Solid Layer, 2: Contact Layer, 3: Enclosure
        char material[64] = "";
        char surface_material[64] = "";
        double temp_left = 0.0;
        double temp_right = 0.0;
        double thickness = 0.0; // cm
        int grid_points = 10;
        double conductivity = 1.0; // W/(mK)
        double convection_hc = 5.0; // W/(m²K)
        double radiation_hr = 0.0; // W/(m²K)
        double resistance = 0.0; // °C/kW
        double heat_flow = 0.0; // kW
        double heat_flux = 0.0; // kW/m²
        
        // Fixed value indicators
        bool fix_temp_left = false;
        bool fix_temp_right = false;
        bool fix_convection = false;
        bool fix_radiation = false;
        bool fix_resistance = false;
        bool fix_conductivity = false;
    };

    std::vector<LayerColumn> columnas_;
    int active_sheet_ = 0; // 0: Wall, 1: Roof, 2: Bottom
    int geometry_type_ = 2; // 0: Wall, 1: Cube, 2: Cylinder, 3: Sphere
    double geom_height_ = 1.0; // m
    double geom_diameter_ = 0.5; // m
    double total_heat_flow_ = 0.0; // kW
    int calc_type_ = 1; // 0: Temperature profile, 1: Heat flow
    bool show_diagram_ = false;
    bool calculated_ = false;

    void calcular_perfil_temperatura();
    void calcular_flujo_calor();
    void reciclar_calculos();
};

// ── Panel de Convertidor de Especies (análisis químico a minerales) ─────
class PanelConvertidorEspecies {
public:
    PanelConvertidorEspecies();
    void renderizar();
private:
    struct InputItem { 
        char element[32]; 
        double wt_pct; 
        double calc_elem; 
    };
    struct OutputItem { 
        char species[32]; 
        double calc_wt; 
        double target_wt; 
        double weight_coeff; 
        char type[16]; 
    };

    std::vector<InputItem> inputs_;
    std::vector<OutputItem> outputs_;

    double input_total_wt_ = 0.0;
    double output_total_wt_ = 0.0;

    bool mostrar_grafico_ = false;

    hsc_clone::SpeciesConverter motor_;
    
    char save_filename_[128] = "conversion.json";
    char open_filename_[128] = "conversion.json";
    
    void calcular();
    void guardar_json(const char* path);
    void cargar_json(const char* path);
};

// ── Panel de Unidades (conversor) ───────────────────────────────────────
class PanelUnidades {
public:
    void renderizar();
private:
    bool db_inicializada_ = false;
    int categoria_ = 0;
    int unidad_origen_ = 0;
    int unidad_destino_ = 1;
    
    struct FilaUnidades {
        double d1 = 0.0;
        double d2 = 0.0;
        std::string cat = "";
        int u1 = 0;
        int u2 = 1;
        std::string nom_u1 = "";
        std::string nom_u2 = "";
        bool activa = false;
    };
    std::vector<FilaUnidades> filas_;
};

// ── Panel de Estimación de Propiedades ──────────────────────────────────
class PanelEstimacionProp {
public:
    void renderizar();
private:
    char formula_[64] = "CuSO4";
    double temperatura_ = 25.0;
    bool es_celsius_ = true;
    int tipo_especie_ = 1; // 0=Not Specified, 1=Inorganic, 2=Organic
    bool mostrar_grafico_ = false;
    bool grafico_calculado_ = false;
    std::vector<double> exp_h_;
    std::vector<double> est_h_;
    std::vector<std::string> db_formulas_;
    double mean_error_ = 0.0;
    double median_error_ = 0.0;
    int species_count_ = 0;
    motores::EstimadorPropiedades motor_;
    std::vector<motores::PropiedadesEstimadasHSC> resultados_;
};

// ── Panel de Método Benson ──────────────────────────────────────────────
struct Vec3 { 
    float x,y,z;
    Vec3 operator+(const Vec3& o) const { return {x+o.x, y+o.y, z+o.z}; }
    Vec3 operator-(const Vec3& o) const { return {x-o.x, y-o.y, z-o.z}; }
    Vec3 operator*(float s) const { return {x*s, y*s, z*s}; }
    Vec3 cross(const Vec3& o) const { return {y*o.z - z*o.y, z*o.x - x*o.z, x*o.y - y*o.x}; }
    float dot(const Vec3& o) const { return x*o.x + y*o.y + z*o.z; }
    Vec3 normalize() const {
        float l = std::sqrt(x*x+y*y+z*z);
        if (l < 1e-6f) return {1.0f,0.0f,0.0f};
        return {x/l, y/l, z/l};
    }
};

struct Atomo3D {
    float x, y, z;
    std::string elemento;
    std::vector<Vec3> valencias; // vectores direccionales disponibles para enlaces
};
struct Enlace3D {
    int atomo1, atomo2;
};

class PanelBenson {
public:
    void renderizar();
private:
    char formula_[128] = "";
    double mw_ = 0.0;
    double h_est_ = 0.0;
    double s_est_ = 0.0;
    double cp_est_ = 0.0;
    
    std::vector<Atomo3D> atomos_;
    std::vector<Enlace3D> enlaces_;
    
    float atom_size_ = 1.0f;
    float opacity_ = 1.0f;
    
    void agregar_grupo(const std::string& nombre, const std::string& elemento_central);
    void deshacer();
    void recalcular();
};

// ── Panel de LCA ────────────────────────────────────────────────────────
struct MapeoLcaFlujo {
    int64_t stream_id = -1;
    std::string unit_name;
    std::string stream_name;
    double flow_value = 0.0;
    std::string unit = "kg";
    std::string lca_equivalent = "No Mapping";
    std::string lca_group = "Not defined";
    bool es_entrada = true;
};

class PanelLCA {
public:
    void renderizar(diagrama_flujo::Flowsheet* flowsheet = nullptr);
    void reset() { cargado_inicial_ = false; mapeos_.clear(); normalizado_ = false; factor_normalizacion_ = 1.0; fila_seleccionada_ = -1; }
private:
    std::vector<MapeoLcaFlujo> mapeos_;
    int fila_seleccionada_ = -1;
    std::string nodo_arbol_seleccionado_;
    
    // Metadatos de la categoría seleccionada
    std::string cat_nombre_ = "None";
    std::string cat_path_ = "None";
    std::string cat_unit_ = "kg";
    std::string cat_ref_q_ = "1 kg";
    double cat_conv_f_ = 1.0;

    double factor_normalizacion_ = 1.0;
    bool normalizado_ = false;

    // Consumos manuales auxiliares
    double manual_electricidad_ = 120.0; // kWh
    double manual_combustible_ = 45.0;   // kg
    double manual_agua_ = 15.0;          // m3

    // Resultados calculados
    double resultado_huella_ = 0.0;
    double resultado_energia_ = 0.0;
    double resultado_residuos_ = 0.0;
    bool calculado_ = false;
    bool cargado_inicial_ = false;

    void inicializar_flujos(diagrama_flujo::Flowsheet* flowsheet);
    void calcular_impacto();
};

// ── Panel de Datos / DataFitter ─────────────────────────────────────────
class PanelDatosANN {
public:
    void renderizar();
private:
    char entrada_1_[64] = "1,2,3,4,5";
    char entrada_2_[64] = "2.1,4.3,6.5,8.7,10.9";
    bool entrenado_ = false;
    hsc_clone::ResultadoFit res_fit_;
    std::vector<double> datos_x_, datos_y_;
};

// ── Panel de ReactionSolver ─────────────────────────────────────────────
class PanelReactionSolver {
public:
    void renderizar();
private:
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

    hsc_clone::ReactionSolver motor_;

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
    hsc_clone::ExternalAPI motor_;
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

// ── Panel de Equilibrium Calculations (HSC Equilibrium) ──────────────────
class PanelEquilibriumCalc {
public:
    void renderizar();
    void reset();
private:
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

// ── Panel de HSC Add-In Functions ──────────────────────────────────────────
class PanelAddIn {
public:
    void renderizar();
private:
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

// ── Panel FIT – NUMERICAL DATA FIT ─────────────────────────────────────────
class PanelDataFit {
public:
    PanelDataFit();
    void renderizar();
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }
private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    struct DataPoint { float x; float y; };
    std::vector<DataPoint> data_;
    
    int func_type_ = 0; // 0=Linear, 1=Polynomial, 2=Rational
    
    // Regression parameters (A, B, C, D, E, F, G)
    float params_[7] = {1.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f};
    
    // Results
    float error_ = 0.0f;
    float r2_ = 0.0f;
    float ssr_ = 0.0f;
    
    // Gradient Descent Settings
    int iterations_max_ = 1000;
    float learning_rate_ = 0.0001f;
    
    // Input
    void refrescar_datos();
    void fit_curve();
    float evaluate_func(float x) const;
};

// ── Panel POURBAIX (Eh-pH) ──────────────────────────────────────────────────
class PanelPourbaix {
public:
    PanelPourbaix();
    void renderizar();

private:
    std::string elemento_actual_ = "Fe";
    double temperatura_actual_ = 298.15;
    double concentracion_actual_ = 1e-6;
    cma::motores::CalculadorPourbaix calc_;
    bool calculado_ = false;
};

class PanelStatisticalAnalysis {
public:
    PanelStatisticalAnalysis();
    void renderizar();
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }
private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    struct StatDataPoint {
        int id;
        float al2o3;
        float cr2o3;
        float zn_rec;
        float cu_rec;
        int group; // 0 or 1 for coloring
    };
    std::vector<StatDataPoint> db_;
    
    // UI state
    int x_axis_col_ = 0; // 0=Al2O3, 1=Cr2O3, 2=Zn, 3=Cu
    int y_axis_col_ = 1;
    
    // Variance data for PCA mock
    std::vector<float> pca_variance_;
    std::vector<std::string> pca_labels_;

    void refrescar_datos();
};

}} // namespace
