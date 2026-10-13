#include "interfaz/aplicacion.hpp"
#include "utils/i18n.h"

#include <GLFW/glfw3.h>
#include <imgui.h>
#include <imgui_impl_glfw.h>
#include <imgui_impl_opengl3.h>
#include <implot.h>
#include <implot3d.h>

#include <cstdio>
#include <ctime>
#include <fstream>
#include <sstream>
#include <iomanip>
#include <filesystem>
#include <set>
#include <algorithm>
#include "adapters/desktop/process.hpp"
#include "application/SimulationService.hpp"

namespace cma { namespace interfaz {
void Aplicacion::nueva_simulacion() {
    flowsheet_ = std::make_unique<diagrama_flujo::Flowsheet>();
    lienzo_.establecer_flowsheet(flowsheet_.get());
    panel_herramientas_.establecer_flowsheet(flowsheet_.get());
    panel_resultados_.limpiar();
    panel_propiedades_ = PanelPropiedades();
    panel_propiedades_.establecer_flowsheet(flowsheet_.get());
    panel_propiedades_.al_cambio_flowsheet = [this]() { hay_cambios_sin_guardar_ = true; };
    panel_propiedades_.al_antes_de_modificar = [this]() { lienzo_.guardar_estado(); };
    panel_geo_.establecer_flowsheet(flowsheet_.get());
    panel_map_gps_.establecer_flowsheet(flowsheet_.get());
    panel_fit_.establecer_flowsheet(flowsheet_.get());
    panel_stat_.establecer_flowsheet(flowsheet_.get());
    panel_lca_.reset();
    panel_reconciliacion_.reset();
    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.id_flujo_seleccionado = -1;
    lienzo_.id_anotacion_seleccionada = -1;
    lienzo_.zoom = 1.0f;
    lienzo_.desplazamiento = ImVec2(0.0f, 0.0f);
    lienzo_.pagina_actual = 0;
    lienzo_.limpiar_historial();
    pila_undo_acciones_.clear();
    ruta_archivo_actual_.clear();
    hay_cambios_sin_guardar_ = false;
    panel_resultados_.poner_resultado_masa("Nuevo lienzo creado.", true);
}

void Aplicacion::guardar_proyecto() {
    std::string ruta = ruta_archivo_actual_;
    if (ruta.empty()) {
        guardar_proyecto_como();
        return;
    }
    std::string json = flowsheet_->a_json();
    std::ofstream f(ruta, std::ios::binary);
    if (f.is_open()) {
        f.write(json.c_str(), (std::streamsize)json.size());
        f.close();
        if (f.good()) {
            ruta_archivo_actual_ = ruta;
            hay_cambios_sin_guardar_ = false;
            panel_resultados_.poner_resultado_masa("Proyecto guardado: " + ruta, true);
        } else {
            panel_resultados_.poner_resultado_masa("Error al escribir: " + ruta, false);
        }
    } else {
        panel_resultados_.poner_resultado_masa("Error al guardar: " + ruta, false);
    }
}

void Aplicacion::guardar_proyecto_como() {
    std::string filepath = file_dialog_.select("Guardar Proyecto CMA Como", "Proyectos CMA | *.cma", true);
    if (!filepath.empty() && filepath.back() == '\n') filepath.pop_back();
    
    if (!filepath.empty()) {
        if (filepath.size() < 4 || filepath.substr(filepath.size() - 4) != ".cma") {
            filepath += ".cma";
        }
        std::string json = flowsheet_->a_json();
        std::ofstream f(filepath, std::ios::binary);
        if (f.is_open()) {
            f.write(json.c_str(), (std::streamsize)json.size());
            f.close();
            if (f.good()) {
                ruta_archivo_actual_ = filepath;
                hay_cambios_sin_guardar_ = false;
                panel_resultados_.poner_resultado_masa("Proyecto guardado: " + filepath, true);
            } else {
                panel_resultados_.poner_resultado_masa("Error al escribir: " + filepath, false);
            }
        } else {
            panel_resultados_.poner_resultado_masa("Error al guardar: " + filepath, false);
        }
    }
}

void Aplicacion::cargar_proyecto() {
    std::string filepath = file_dialog_.select("Abrir Proyecto CMA", "Proyectos CMA | *.cma", false);
    if (!filepath.empty() && filepath.back() == '\n') filepath.pop_back();
    
    if (!filepath.empty()) {
        std::ifstream fp(filepath, std::ios::binary);
        if (fp.is_open()) {
            std::string json_content((std::istreambuf_iterator<char>(fp)),
                                     std::istreambuf_iterator<char>());
            fp.close();
            
            std::string err;
            if (flowsheet_->desde_json(json_content, err)) {
                ruta_archivo_actual_ = filepath;
                hay_cambios_sin_guardar_ = false;
                panel_resultados_.poner_resultado_masa("Proyecto cargado: " + filepath, true);
                
                lienzo_.id_nodo_seleccionado = -1;
                lienzo_.id_flujo_seleccionado = -1;
                lienzo_.id_anotacion_seleccionada = -1;
                panel_propiedades_ = PanelPropiedades();
                panel_propiedades_.establecer_flowsheet(flowsheet_.get());
                panel_propiedades_.al_cambio_flowsheet = [this]() { hay_cambios_sin_guardar_ = true; };
                panel_propiedades_.al_antes_de_modificar = [this]() { lienzo_.guardar_estado(); };
                panel_geo_.establecer_flowsheet(flowsheet_.get());
                panel_map_gps_.establecer_flowsheet(flowsheet_.get());
                panel_fit_.establecer_flowsheet(flowsheet_.get());
                panel_stat_.establecer_flowsheet(flowsheet_.get());
                panel_lca_.reset();
                lienzo_.encajar_todo();
                lienzo_.limpiar_historial();
                pila_undo_acciones_.clear();
            } else {
                panel_resultados_.poner_resultado_masa("Error al cargar proyecto: " + err, false);
            }
        } else {
            panel_resultados_.poner_resultado_masa("Error al abrir archivo.", false);
        }
    }
}

void Aplicacion::reiniciar_simulacion() {
    panel_resultados_.limpiar();
    
    // Clear computed flows and properties from streams
    for (auto& [id, st] : flowsheet_->mapa_flujos()) {
        st.flujo_molar.clear();
        st.flujo_solido.clear();
        st.flujo_liquido.clear();
        st.flujo_gas.clear();
        st.fraccion_solido = 0.0;
        st.fraccion_liquido = 0.0;
        st.fraccion_gas = 0.0;
        st.entalpia = 0.0;
        st.energia = 0.0;
    }
    
    // Clear calculated node params
    for (auto& n : flowsheet_->nodos()) {
        n.params.erase("Q_calculated");
        n.params.erase("W_calculated");
        n.params.erase("T_out_calculated");
        n.params.erase("P_out_calculated");
        n.params.erase("Ex_loss");
    }
}

void Aplicacion::ejecutar_convergencia() {
    auto res = flowsheet_->resolver_convergencia(repository_, 500, 1e-6, nullptr);
    panel_resultados_.poner_resultado_masa(res.mensaje, res.convergio);
    if (res.convergio) {
        char buf[128];
        snprintf(buf, sizeof(buf), "Recirculaciones resueltas. Iteraciones: %d, Error final: %.2e",
                 res.iteraciones, res.error_final);
        panel_resultados_.poner_resultado_energia(buf, true);
        // Muestra el reporte completo (masa + energía) con los valores convergidos
        ejecutar_simulacion();
    }
}

void Aplicacion::agregar_anotacion() {
    lienzo_.guardar_estado();
    int64_t aid = flowsheet_->agregar_anotacion("Nueva anotación");
    auto* a = flowsheet_->anotacion_por_id(aid);
    if (a) a->pagina = lienzo_.pagina_actual;
    pila_undo_acciones_.push_back({UndoAction::Tipo::Anotacion, aid});
    panel_propiedades_.seleccionar_anotacion(aid);
    lienzo_.id_anotacion_seleccionada = aid;
    hay_cambios_sin_guardar_ = true;
}

void Aplicacion::agregar_tabla_flujo() {
    lienzo_.guardar_estado();
    flowsheet_->agregar_tabla_flujo();
    hay_cambios_sin_guardar_ = true;
}

void Aplicacion::abrir_archivo() { cargar_proyecto(); }
void Aplicacion::guardar_archivo() { guardar_proyecto(); }

/// Helper: find origin node name for a flow ID

}} // namespace
