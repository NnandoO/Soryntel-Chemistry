#pragma once
#include "nucleo/tipos.hpp"
#include "diagrama_flujo/flowsheet.hpp"
#include <imgui.h>
#include <functional>
#include <implot.h>

namespace cma { namespace interfaz {

class Lienzo {
public:
    Lienzo();

    static void dibujar_modelo(ImDrawList* draw, ImVec2 c, float s, nucleo::TipoNodo tipo);
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs);
    void renderizar(const ImVec2& region);
    void centrar_en(float x, float y);
    void encajar_todo();

    // Undo/Redo (snapshots JSON del flowsheet)
    void guardar_estado();
    void deshacer();
    void rehacer();
    void limpiar_historial();
    bool hay_deshacer() const { return !undo_stack_.empty(); }
    bool hay_rehacer() const { return !redo_stack_.empty(); }
    void establecer_callback_cambio(std::function<void()> cb) { al_cambio_flowsheet_ = std::move(cb); }

    // Callbacks
    std::function<void(int64_t)> al_seleccionar_nodo;
    std::function<void(int64_t)> al_seleccionar_flujo;
    std::function<void(int64_t)> al_seleccionar_anotacion;
    std::function<void(int64_t)> al_crear_anotacion;
    std::function<void(nucleo::TipoNodo)> al_solicitar_agregar_nodo;

    // Estado público
    int64_t id_nodo_seleccionado   = -1;
    int64_t id_flujo_seleccionado  = -1;
    int64_t id_anotacion_seleccionada = -1;
    ImVec2  desplazamiento         = ImVec2(0, 0);
    float   zoom                   = 1.0f;
    bool    mostrar_minimapa       = true;
    bool    ajustar_grilla         = true;
    int     pagina_actual          = 0;
    bool    persistir_herramienta  = false;
    bool    activo_dibujo_flujo    = false;
    bool    activo_crear_texto     = false;
    ImVec2  coord_raton            = ImVec2(0, 0);
    bool    activo_crear_rect      = false;
    bool    activo_crear_elipse    = false;

    enum class ModoDibujo { Seleccion, DibujarFlujo, Texto, Rectangulo, Elipse };
    ModoDibujo modo_dibujo = ModoDibujo::Seleccion;

private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;

    void renderizar_nodo(nucleo::Nodo& nodo, const ImVec2& pos_lienzo);
    void renderizar_flujos(const ImVec2& pos_lienzo);
    void renderizar_flujo_energia(const nucleo::Flujo& flujo, const ImVec2& inicio, const ImVec2& fin, int64_t fid);
    void renderizar_conexion_arrastrada(const ImVec2& pos_lienzo);
    void renderizar_menu_contextual(const ImVec2& pos_lienzo);
    void renderizar_minimapa(const ImVec2& pos, const ImVec2& tam);
    void renderizar_overlay_zoom(const ImVec2& pos, const ImVec2& tam);
    void renderizar_anotaciones(const ImVec2& pos);
    void renderizar_tablas_flujo(const ImVec2& pos);

    std::map<int64_t, std::vector<ImVec2>> puertos_entrada_;
    std::map<int64_t, std::vector<ImVec2>> puertos_salida_;
    void calcular_posicion_puertos(const nucleo::Nodo& nodo, const ImVec2& npos, const ImVec2& nsz);

    bool    arrastrando_conexion_ = false;
    int64_t nodo_origen_ = -1;
    ImVec2  inicio_arrastre_;
    ImVec2  actual_arrastre_;
    int64_t nodo_arrastrando_ = -1;     // nodo siendo arrastrado (drag)
    int64_t anotacion_arrastrando_ = -1; // anotación siendo arrastrada (drag)
    bool    creando_anotacion_ = false;
    int64_t id_anotacion_creando_ = -1;
    ImVec2  inicio_creacion_anotacion_;

    bool    arrastrando_lienzo_ = false;
    ImVec2  vel_desplazamiento_ = ImVec2(0, 0);
    ImVec2  ultimo_raton_lienzo_;

    std::string error_conexion_;
    float       tiempo_error_conexion_ = 0.0f;
    void        mostrar_error_conexion(const std::string& msg);

    ImVec2  des_destino_ = ImVec2(0, 0);
    bool    animando_ = false;
    bool    clic_detectado_en_objeto_ = false;
    bool    hover_lienzo_ = false;      // true si el mouse está sobre el lienzo
    bool    clic_en_lienzo_ = false;    // true si se hizo clic en el lienzo este frame
    std::vector<int64_t> nodos_a_eliminar_;
    std::function<void()> al_cambio_flowsheet_; // notifica cambios al Aplicacion
    
    // Historial y Portapapeles
    std::vector<std::string> undo_stack_;
    std::vector<std::string> redo_stack_;
    std::string portapapeles_json_; // Almacenaremos el nodo como JSON si es posible, o usaremos otra técnica
    nucleo::TipoNodo portapapeles_tipo_ = nucleo::TipoNodo::Fuente;
    bool tiene_portapapeles_ = false;

    ImVec2 ultima_region_ = ImVec2(800, 600);

    void copiar();
    void cortar();
    void pegar();
};

}} // namespace
