#pragma once
#include "application/modules.hpp"
#include <thread>
#include <atomic>
#include "motores/balance_exergia.hpp"
#include "nucleo/tipos.hpp"
#include "nucleo/tabla_periodica.hpp"
#include "diagrama_flujo/flowsheet.hpp"
#include "base_datos/gestor_db.hpp"
#include "motores/balance_masa.hpp"
#include "motores/balance_energia.hpp"
#include "motores/balance_hoja.hpp"
#include "motores/equilibrio_fases.hpp"
#include "motores/electroquimica.hpp"
#include "motores/cinetica.hpp"
#include "motores/destilacion.hpp"
#include "motores/minimizador_gibbs.hpp"
#include "motores/simulacion_dinamica.hpp"
#include "motores/exergia.hpp"
#include "../diagrams/PropertyDiagrams.hpp"
#include "../diagrams/StabilityDiagrams.hpp"

#include "motores/agua.hpp"
#include "motores/soluciones_acuosas.hpp"
#include "motores/perdida_calor.hpp"
#include "motores/estimador_propiedades.hpp"
#include "motores/lca.hpp"
#include "motores/red_neuronal.hpp"
#include "thermo/ReactionSolver.hpp"
#include "tools/SpeciesConverter.hpp"
#include "tools/DataFitter.hpp"
#include "tools/ExternalAPI.hpp"
#include "motores/pourbaix.hpp"
#include "panels/panel_balance_hoja.hpp"
#include "panels/panel_lca.hpp"
#include "panels/panel_balance_metalurgico.hpp"
#include "panels/panel_material_db.hpp"
#include "panels/panel_perdida_calor.hpp"
#include "panels/panel_estabilidad.hpp"
#include "panels/panel_pourbaix.hpp"
#include "panels/panel_equilibrio.hpp"
#include "panels/panel_geologia.hpp"
#include "panels/panel_estimacion.hpp"
#include "panels/panel_reaccion.hpp"
#include "panels/panel_acuoso.hpp"
#include "db/MineralDB.hpp"
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
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void establecer_gestor_db(base_datos::GestorDB* db) { db_ = db; repo_ = db; }

private:
    ModalBuscadorDB() = default;
    const domain::ThermoRepository* repo_ = nullptr;
    base_datos::GestorDB* db_ = nullptr;
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
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void establecer_gestor_db(base_datos::GestorDB* db) { repo_ = db; }
    void seleccionar_nodo(int64_t id);
    void seleccionar_flujo(int64_t id);
    void seleccionar_anotacion(int64_t id);
    void abrir_editor_unidad(int64_t id);
    void renderizar();
    void renderizar_editor_unidad();
    std::function<void()> al_cambio_flowsheet;
    std::function<void()> al_antes_de_modificar;  // hook para guardar estado antes de undo
    std::function<void()> al_ejecutar_simulacion;

private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    const domain::ThermoRepository* repo_ = nullptr;
    int64_t id_nodo_   = -1;
    int64_t id_flujo_  = -1;
    int64_t id_anotacion_ = -1;
    int64_t id_nodo_editor_ = -1;
    bool mostrar_editor_unidad_ = false;
    bool forzar_seleccion_proceso_ = false;
    bool forzar_seleccion_dibujos_ = false;

    void renderizar_contenido_proceso();
    void renderizar_stream_viewer();
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
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void establecer_gestor_db(base_datos::GestorDB* db) { repo_ = db; }
    void renderizar();
    void refrescar();

    std::function<void(const nucleo::Especie&)> al_seleccionar_especie;
    const nucleo::Especie* especie_seleccionada() const { return seleccionada_; }

private:
    const domain::ThermoRepository* repo_ = nullptr;
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
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void establecer_gestor_db(base_datos::GestorDB* db) { db_ = db; repo_ = db; }
    void renderizar(const std::map<int64_t, nucleo::Especie>& db);

private:
    const domain::ThermoRepository* repo_ = nullptr;
    base_datos::GestorDB* db_ = nullptr;
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

}} // namespace cma::interfaz
