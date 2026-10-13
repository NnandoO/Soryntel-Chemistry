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
void Aplicacion::manejar_atajos_globales() {
    auto& io = ImGui::GetIO();
    if (io.WantTextInput) return;

    bool ctrl = io.KeyCtrl;
    bool shift = io.KeyShift;

    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_N)) nueva_simulacion();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_E)) cargar_ejemplo();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_O)) cargar_proyecto();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_S)) guardar_proyecto();
    if (ctrl && shift && ImGui::IsKeyPressed(ImGuiKey_S)) guardar_proyecto_como();

    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_C)) copiar_seleccion();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_V)) pegar_seleccion();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_Z)) lienzo_.deshacer();
    if ((ctrl && shift && ImGui::IsKeyPressed(ImGuiKey_Z)) ||
        (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_Y))) lienzo_.rehacer();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_D)) duplicar_seleccion();

    if (ImGui::IsKeyPressed(ImGuiKey_F5)) ejecutar_simulacion();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_R)) ejecutar_convergencia();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_G)) ejecutar_gibbs();

    if (ImGui::IsKeyPressed(ImGuiKey_Delete) || ImGui::IsKeyPressed(ImGuiKey_KeypadDecimal)) {
        if (lienzo_.id_nodo_seleccionado > 0) {
            lienzo_.guardar_estado();
            flowsheet_->eliminar_nodo(lienzo_.id_nodo_seleccionado);
            lienzo_.id_nodo_seleccionado = -1;
            hay_cambios_sin_guardar_ = true;
        } else if (lienzo_.id_flujo_seleccionado > 0) {
            lienzo_.guardar_estado();
            flowsheet_->eliminar_flujo(lienzo_.id_flujo_seleccionado);
            lienzo_.id_flujo_seleccionado = -1;
            hay_cambios_sin_guardar_ = true;
        } else if (lienzo_.id_anotacion_seleccionada > 0) {
            lienzo_.guardar_estado();
            flowsheet_->eliminar_anotacion(lienzo_.id_anotacion_seleccionada);
            lienzo_.id_anotacion_seleccionada = -1;
            hay_cambios_sin_guardar_ = true;
        }
    }
}

void Aplicacion::copiar_seleccion() {
    if (lienzo_.id_nodo_seleccionado > 0) {
        auto* n = flowsheet_->nodo_por_id(lienzo_.id_nodo_seleccionado);
        if (n) {
            portapapeles_ = NodoClipboard{n->tipo, n->nombre, n->params};
            portapapeles_anotacion_ = std::nullopt;
        }
    } else if (lienzo_.id_anotacion_seleccionada > 0) {
        diagrama_flujo::Anotacion* found = nullptr;
        for (auto& a : flowsheet_->anotaciones()) {
            if (a.id == lienzo_.id_anotacion_seleccionada) {
                found = &a;
                break;
            }
        }
        if (found) {
            portapapeles_anotacion_ = AnotacionClipboard{found->tipo, found->w, found->h, found->texto, found->color_fondo, found->color_borde, found->grosor_borde};
            portapapeles_ = std::nullopt;
        }
    }
}

void Aplicacion::pegar_seleccion() {
    if (portapapeles_) {
        lienzo_.guardar_estado();
        auto& clip = *portapapeles_;
        auto id = flowsheet_->agregar_nodo(clip.tipo);
        auto* n = flowsheet_->nodo_por_id(id);
        if (!n) return;
        n->nombre = clip.nombre + " (copia)";
        n->params = clip.params;
        n->canvas_x = std::min(n->canvas_x + 30.0f, 1200.0f);
        n->canvas_y = std::min(n->canvas_y + 30.0f, 800.0f);
        n->pagina = lienzo_.pagina_actual;
        pila_undo_acciones_.push_back({UndoAction::Tipo::Nodo, id});
        panel_propiedades_.seleccionar_nodo(id);
        lienzo_.id_nodo_seleccionado = id;
        hay_cambios_sin_guardar_ = true;
    } else if (portapapeles_anotacion_) {
        lienzo_.guardar_estado();
        auto& clip = *portapapeles_anotacion_;
        int64_t aid = flowsheet_->agregar_anotacion(clip.texto);
        for (auto& a : flowsheet_->anotaciones()) {
            if (a.id == aid) {
                a.tipo = clip.tipo;
                a.w = clip.w;
                a.h = clip.h;
                a.color_fondo = clip.color_fondo;
                a.color_borde = clip.color_borde;
                a.grosor_borde = clip.grosor_borde;
                a.pagina = lienzo_.pagina_actual;
                // Offset slightly
                a.x = std::min(a.x + 30.0f, 1200.0f);
                a.y = std::min(a.y + 30.0f, 800.0f);
                break;
            }
        }
        pila_undo_acciones_.push_back({UndoAction::Tipo::Anotacion, aid});
        panel_propiedades_.seleccionar_anotacion(aid);
        lienzo_.id_anotacion_seleccionada = aid;
        hay_cambios_sin_guardar_ = true;
    }
}

void Aplicacion::duplicar_seleccion() {
    if (lienzo_.id_nodo_seleccionado > 0) {
        auto* n = flowsheet_->nodo_por_id(lienzo_.id_nodo_seleccionado);
        if (!n) return;
        copiar_seleccion();
        pegar_seleccion();
    } else if (lienzo_.id_anotacion_seleccionada > 0) {
        copiar_seleccion();
        pegar_seleccion();
    }
}

void Aplicacion::deshacer_ultimo() {
    if (pila_undo_acciones_.empty()) return;
    auto act = pila_undo_acciones_.back();
    pila_undo_acciones_.pop_back();
    if (act.tipo == UndoAction::Tipo::Nodo) {
        flowsheet_->eliminar_nodo(act.id);
        lienzo_.id_nodo_seleccionado = -1;
    } else {
        flowsheet_->eliminar_anotacion(act.id);
        lienzo_.id_anotacion_seleccionada = -1;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Cierre
// ═══════════════════════════════════════════════════════════════════════════


}} // namespace
