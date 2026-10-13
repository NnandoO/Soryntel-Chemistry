#ifndef CMA_INTERFAZ_APLICACION_HPP
#define CMA_INTERFAZ_APLICACION_HPP

#include <GLFW/glfw3.h>
#include "interfaz/lienzo.hpp"
#include "interfaz/paneles.hpp"
#include "interfaz/panels/scientific.hpp"

#include "diagrama_flujo/flowsheet.hpp"
#include "base_datos/gestor_db.hpp"
#include "motores/balance_masa.hpp"
#include "motores/balance_energia.hpp"
#include "motores/minimizador_gibbs.hpp"
#include "motores/equilibrio_fases.hpp"
#include "motores/electroquimica.hpp"
#include "motores/cinetica.hpp"
#include "motores/destilacion.hpp"
#include "sim/SteadyStateEngine.hpp"
#include "sim/DynamicEngine.hpp"
#include "sim/MassReconciliation.hpp"
#include "thermo/ReactionSolver.hpp"
#include "thermo/GibbsMinimizer.hpp"
#include "thermo/HeatMaterialBalance.hpp"
#include "thermo/ExergyCalculator.hpp"
#include "diagrams/PropertyDiagrams.hpp"

#include "diagrams/StabilityDiagrams.hpp"
#include "aqueous/SteamTables.hpp"
#include "aqueous/ElectrolyteSolver.hpp"
#include "db/ThermoDB.hpp"
#include "db/MineralDB.hpp"
#include "db/MaterialDB.hpp"
#include "estimation/StatisticalEstimator.hpp"
#include "estimation/BensonEstimator.hpp"
#include "tools/HeatLossCalc.hpp"
#include "tools/SpeciesConverter.hpp"
#include "tools/PeriodicTable.hpp"
#include "tools/UnitConverter.hpp"
#include "tools/LCAEvaluator.hpp"
#include "tools/DataFitter.hpp"
#include "tools/ExternalAPI.hpp"

#include <string>
#include <memory>
#include <optional>
#include <vector>
#include "application/ports/FileDialog.hpp"

namespace cma { namespace interfaz {

class Aplicacion {
public:
    int ancho_ = 1280;
    int alto_ = 800;

    Aplicacion(base_datos::GestorDB& repository, application::FileDialog& file_dialog,
               const domain::AqueousDatabase& aqueous_database, const domain::AqueousEngine& aqueous_engine);
    ~Aplicacion();

    bool iniciar(int argc, char** argv);
    void ejecutar();

private:
    base_datos::GestorDB& repository_;
    application::FileDialog& file_dialog_;
    NativeAqueousPanel panel_native_aqueous_;
    AdvancedAqueousPanel panel_advanced_aqueous_;
    MineralProcessingPanel panel_mineral_processing_;
    CatalogAuditPanel panel_catalog_audit_;
    bool mostrar_mineral_processing_ = false, mostrar_catalog_audit_ = false;
    std::string review_panel_;
    bool iniciar_ventana();
    bool iniciar_imgui();
    void cerrar();

    void frame_glfw();
    void renderizar();
    void renderizar_modulos();

    void menu_principal();
    void menu_archivo();
    void menu_editar();
    void menu_simulacion();
    void menu_herramientas();
    void menu_modulos();
    void menu_ayuda();

    void nueva_simulacion();
    void abrir_archivo();
    void guardar_archivo();
    void ejecutar_simulacion();
    void reiniciar_simulacion();
    void ejecutar_convergencia();
    void ejecutar_gibbs();
    void exportar_resultados_csv();
    void cargar_ejemplo(int ejemplo = 1);
    void cargar_ejemplo_1();
    void cargar_ejemplo_2();
    void cargar_ejemplo_3();
    void cargar_ejemplo_4();
    void agregar_anotacion();
    void agregar_tabla_flujo();
    void guardar_proyecto();
    void guardar_proyecto_como();
    void cargar_proyecto();

    void configurar_estilo_oscuro();
    void configurar_estilo_claro();
    void alternar_modo();

    static void error_glfw_cb(int error, const char* desc);
    static void redimensionar_cb(GLFWwindow* w, int ancho, int alto);
    static void scroll_cb(GLFWwindow* w, double dx, double dy);

    // Permite que el callback global de cierre acceda a hay_cambios_sin_guardar_
    friend void glfw_cerrar_ventana_cb(GLFWwindow* w);

    GLFWwindow* ventana_ = nullptr;
    bool ejecutando_ = false;
    bool smoke_test_ = false;

    std::unique_ptr<diagrama_flujo::Flowsheet> flowsheet_;

    Lienzo lienzo_;

    PanelHerramientas panel_herramientas_;
    PanelPropiedades panel_propiedades_;
    PanelEspecies panel_especies_;
    PanelResultados panel_resultados_;
    PanelCalculadora panel_calculadora_;
    PanelTablaPeriodica panel_tabla_;
    PanelElectroquimica panel_electroquimica_;
    PanelCinetica panel_cinetica_;
    PanelEquilibrioFases panel_equilibrio_;
    PanelSimulacionDinamica panel_sim_dinamica_;
    PanelReconciliacion panel_reconciliacion_;
    PanelBalanceHoja panel_balance_hoja_;
    PanelExergia panel_exergia_;
    PanelDiagramasTermo panel_diagramas_termo_;

    PanelEstabilidad panel_estabilidad_;
    PanelDiagramasLpp panel_lpp_;
    PanelAgua panel_agua_;
    PanelSolucionesAcuosas panel_soluciones_;
    PanelPerdidaCalor panel_perdida_calor_;
    PanelConvertidorEspecies panel_convertidor_;
    PanelUnidades panel_unidades_;
    PanelEstimacionProp panel_estimacion_;
    PanelBenson panel_benson_;
    PanelLCA panel_lca_;
    PanelDataFit panel_fit_;
    PanelStatisticalAnalysis panel_stat_;
    PanelDatosANN panel_datos_ann_;
    PanelGrafico3D panel_grafico3d_;
    PanelReactionSolver panel_reaction_solver_;
    PanelEquilibriumCalc panel_equilibrium_calc_;
    PanelExternalAPI panel_api_externa_;
    PanelAddIn panel_addin_;
    PanelGeoMineralogical panel_geo_;
    PanelMapGPS panel_map_gps_;
    PanelPourbaix panel_pourbaix_;

    // Portapapeles y deshacer
    struct NodoClipboard {
        nucleo::TipoNodo tipo;
        std::string nombre;
        std::map<std::string, double> params;
    };
    struct AnotacionClipboard {
        diagrama_flujo::Anotacion::Tipo tipo;
        float w, h;
        std::string texto;
        uint32_t color_fondo;
        uint32_t color_borde;
        float grosor_borde;
    };
    struct UndoAction {
        enum class Tipo { Nodo, Anotacion } tipo;
        int64_t id;
    };
    std::optional<NodoClipboard> portapapeles_;
    std::optional<AnotacionClipboard> portapapeles_anotacion_;
    std::vector<UndoAction> pila_undo_acciones_;

    void manejar_atajos_globales();
    void copiar_seleccion();
    void pegar_seleccion();
    void duplicar_seleccion();
    void deshacer_ultimo();

    bool mostrar_tabla_ = false;
    bool mostrar_calculadora_ = false;
    bool mostrar_electroquimica_ = false;
    bool mostrar_cinetica_ = false;
    bool mostrar_equilibrio_ = false;
    bool mostrar_sim_dinamica_ = false;
    bool mostrar_reconciliacion_ = false;
    bool mostrar_balance_hoja_ = false;
    bool mostrar_exergia_ = false;
    bool mostrar_diagramas_termo_ = false;

    bool mostrar_estabilidad_ = false;
    bool mostrar_lpp_ = false;
    bool mostrar_agua_ = false;
    bool mostrar_soluciones_ = false;
    bool mostrar_perdida_calor_ = false;
    bool mostrar_convertidor_ = false;
    bool mostrar_unidades_ = false;
    bool mostrar_estimacion_ = false;
    bool mostrar_benson_ = false;
    bool mostrar_lca_ = false;
    bool mostrar_datos_ann_ = false;
    bool mostrar_grafico3d_ = false;
    bool mostrar_reaction_solver_ = false;
    bool mostrar_equilibrium_calc_ = false;
    bool mostrar_api_externa_ = false;

    // Nuevos módulos HSC Chemistry
    bool mostrar_lpp_diagrams_ = false;
    bool mostrar_add_in_functions_ = false;
    bool mostrar_geo_mineralogical_ = false;
    bool mostrar_map_gps_ = false;
    bool mostrar_pourbaix_ = false;
    bool mostrar_data_fit_ = false;
    bool mostrar_statistical_analysis_ = false;
    bool mostrar_process_simulation_status_ = false;
    bool persist_tool_ = false;
    bool modo_oscuro_ = true;

    // Estado del panel Sim
    bool sim_resuelto_ = false;
    diagrama_flujo::Flowsheet::ResultadoConvergencia sim_ultimo_resultado_;

    // Cambios sin guardar
    bool hay_cambios_sin_guardar_ = false;
    bool mostrar_confirmar_salir_ = false;

    std::string ruta_archivo_actual_;
    int contador_nodos_ = 0;

    // Layout: dimensiones de paneles (persisten entre frames, no son static locales)
    float layout_der_w_ = 0.0f;
    float layout_inf_h_ = 0.0f;
    float layout_esp_ratio_ = 0.55f;
    bool  layout_drag_b_ = false;
    bool  layout_drag_r_ = false;
    bool  layout_drag_rv_ = false;
    bool  layout_inicializado_ = false;
};

}} // namespace
#endif
