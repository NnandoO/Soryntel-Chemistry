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
void Aplicacion::renderizar() {
    manejar_atajos_globales();
    menu_principal();

    ImVec2 tam_vp = ImGui::GetMainViewport()->Size;
    float menu_h = ImGui::GetFrameHeight();
    float stat_h = ImGui::GetFrameHeight();

    // Layout con splitters manuales (sin dependencia de ventana ImGui)
    const float izq_w = 46.0f;
    if (!layout_inicializado_ || tam_vp.x <= 0 || tam_vp.y <= 0) {
        layout_der_w_ = tam_vp.x * 0.32f;
        layout_inf_h_ = tam_vp.y * 0.28f;
        layout_esp_ratio_ = 0.55f;
        layout_inicializado_ = true;
    }
    float& der_w = layout_der_w_;
    float& inf_h = layout_inf_h_;
    float& esp_ratio = layout_esp_ratio_;

    float min_panel = 80.0f;
    float disp_h = tam_vp.y - menu_h - inf_h - stat_h;

    // ── Splitters manuales mediante mouse tracking directo ──
    auto& io = ImGui::GetIO();
    ImVec2 mp = io.MousePos;

    auto punto_en_rect = [](ImVec2 p, float x1, float y1, float x2, float y2) {
        return p.x >= x1 && p.x < x2 && p.y >= y1 && p.y < y2;
    };

    // ── Lienzo (Flowsheet Area) ──
    ImGui::SetNextWindowPos(ImVec2(izq_w, menu_h));
    ImGui::SetNextWindowSize(ImVec2(tam_vp.x - izq_w - der_w, disp_h));
    ImGui::Begin(tr("Lienzo"), nullptr, ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoBringToFrontOnFocus);
    ImVec2 lienzo_tam = ImGui::GetContentRegionAvail();
    lienzo_.persistir_herramienta = persist_tool_;
    lienzo_.renderizar(lienzo_tam);
    ImGui::End();

    // ── Panel izquierdo: Barra de Herramientas CAD ──
    ImGui::SetNextWindowPos(ImVec2(0, menu_h));
    ImGui::SetNextWindowSize(ImVec2(izq_w, disp_h));
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(4, 8));
    ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(4, 6));
    ImGui::Begin(tr("CAD Tools"), nullptr, ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    {
        auto render_tool_button = [&](const char* label_id, const char* tooltip, Lienzo::ModoDibujo target_mode, std::function<void(ImDrawList*, ImVec2)> draw_icon) {
            bool active = (lienzo_.modo_dibujo == target_mode);
            if (active) {
                ImGui::PushStyleColor(ImGuiCol_Button, ImGui::GetStyle().Colors[ImGuiCol_ButtonActive]);
            } else {
                ImGui::PushStyleColor(ImGuiCol_Button, ImGui::GetStyle().Colors[ImGuiCol_Button]);
            }
            ImVec2 pos = ImGui::GetCursorScreenPos();
            if (ImGui::Button(label_id, ImVec2(38, 38))) {
                lienzo_.modo_dibujo = target_mode;
            }
            ImGui::PopStyleColor();
            
            ImDrawList* draw = ImGui::GetWindowDrawList();
            ImVec2 center(pos.x + 19.0f, pos.y + 19.0f);
            draw_icon(draw, center);

            if (ImGui::IsItemHovered()) {
                ImGui::SetTooltip("%s", tooltip);
            }
            ImGui::Spacing();
        };

        // Pointer Tool
        render_tool_button("##ptr", "Pointer Tool (Seleccionar y Mover)", Lienzo::ModoDibujo::Seleccion, [](ImDrawList* dl, ImVec2 center) {
            ImVec2 p0(center.x - 7, center.y - 10);
            ImVec2 p1(center.x - 7, center.y + 8);
            ImVec2 p2(center.x - 2, center.y + 3);
            ImVec2 p3(center.x + 3, center.y + 8);
            ImVec2 p4(center.x + 5, center.y + 6);
            ImVec2 p5(center.x, center.y + 1);
            ImVec2 p6(center.x + 5, center.y - 1);
            
            dl->AddTriangleFilled(p0, p1, p2, IM_COL32(30, 30, 35, 255));
            dl->AddTriangleFilled(p0, p2, p6, IM_COL32(30, 30, 35, 255));
            dl->AddLine(p2, p3, IM_COL32(30, 30, 35, 255), 2.5f);
            dl->AddTriangle(p0, p1, p6, IM_COL32(255, 255, 255, 240), 1.0f);
        });

        // Stream Tool
        render_tool_button("##stream", "Stream Tool (Conectar Nodos/Corrientes)", Lienzo::ModoDibujo::DibujarFlujo, [](ImDrawList* dl, ImVec2 center) {
            ImVec2 p_start(center.x - 10, center.y + 8);
            ImVec2 cp1(center.x - 4, center.y - 8);
            ImVec2 cp2(center.x + 2, center.y + 8);
            ImVec2 p_end(center.x + 10, center.y - 6);
            
            dl->AddBezierCubic(p_start, cp1, cp2, p_end, IM_COL32(0, 150, 255, 255), 2.5f);
            
            ImVec2 dir(1.0f, -0.6f);
            float len = sqrt(dir.x*dir.x + dir.y*dir.y);
            dir.x /= len; dir.y /= len;
            ImVec2 perp(-dir.y, dir.x);
            dl->AddTriangleFilled(p_end,
                ImVec2(p_end.x - dir.x * 6 + perp.x * 3.5f, p_end.y - dir.y * 6 + perp.y * 3.5f),
                ImVec2(p_end.x - dir.x * 6 - perp.x * 3.5f, p_end.y - dir.y * 6 - perp.y * 3.5f),
                IM_COL32(0, 150, 255, 255));
        });

        // Rectangle Tool
        render_tool_button("##rect", "Rectangle Tool (Dibujar Caja de Fondo)", Lienzo::ModoDibujo::Rectangulo, [](ImDrawList* dl, ImVec2 center) {
            ImVec2 p0(center.x - 10, center.y - 8);
            ImVec2 p1(center.x + 10, center.y + 8);
            dl->AddRectFilled(p0, p1, IM_COL32(80, 120, 220, 50), 2.0f);
            dl->AddRect(p0, p1, IM_COL32(80, 120, 220, 255), 2.0f, 0, 1.5f);
        });

        // Ellipse Tool
        render_tool_button("##ellipse", "Ellipse Tool (Dibujar Círculo/Elipse)", Lienzo::ModoDibujo::Elipse, [](ImDrawList* dl, ImVec2 center) {
            dl->AddEllipseFilled(center, ImVec2(11, 8), IM_COL32(180, 80, 220, 50));
            dl->AddEllipse(center, ImVec2(11, 8), IM_COL32(180, 80, 220, 255), 0, 0, 1.5f);
        });

        // Text Tool
        render_tool_button("##text", "Text Tool (Dibujar Caja de Texto)", Lienzo::ModoDibujo::Texto, [](ImDrawList* dl, ImVec2 center) {
            dl->AddLine(ImVec2(center.x - 9, center.y - 8), ImVec2(center.x + 9, center.y - 8), IM_COL32(230, 80, 50, 255), 2.5f);
            dl->AddLine(ImVec2(center.x, center.y - 8), ImVec2(center.x, center.y + 9), IM_COL32(230, 80, 50, 255), 2.5f);
            dl->AddLine(ImVec2(center.x - 4, center.y + 9), ImVec2(center.x + 4, center.y + 9), IM_COL32(230, 80, 50, 255), 1.5f);
        });
        
        ImGui::Separator();
        ImGui::Spacing();

        // Fit View
        ImVec2 pos_f = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##fit", ImVec2(38, 38))) {
            lienzo_.encajar_todo();
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_f.x + 19.0f, pos_f.y + 19.0f);
            dl->AddCircle(center, 5.5f, IM_COL32(100, 180, 120, 255), 0, 1.5f);
            float d = 9.0f;
            dl->AddLine(ImVec2(center.x - d, center.y - d), ImVec2(center.x - 4, center.y - 4), IM_COL32(100, 180, 120, 255), 1.5f);
            dl->AddLine(ImVec2(center.x + d, center.y - d), ImVec2(center.x + 4, center.y - 4), IM_COL32(100, 180, 120, 255), 1.5f);
            dl->AddLine(ImVec2(center.x - d, center.y + d), ImVec2(center.x - 4, center.y + 4), IM_COL32(100, 180, 120, 255), 1.5f);
            dl->AddLine(ImVec2(center.x + d, center.y + d), ImVec2(center.x + 4, center.y + 4), IM_COL32(100, 180, 120, 255), 1.5f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Encajar todo (Fit Flowsheet)");

        ImGui::Spacing();

        // Zoom In
        ImVec2 pos_z_in = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##z_in", ImVec2(38, 38))) {
            lienzo_.zoom = std::min(lienzo_.zoom * 1.2f, 5.0f);
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_z_in.x + 19.0f, pos_z_in.y + 19.0f);
            dl->AddLine(ImVec2(center.x - 8, center.y), ImVec2(center.x + 8, center.y), IM_COL32(80, 200, 120, 255), 2.5f);
            dl->AddLine(ImVec2(center.x, center.y - 8), ImVec2(center.x, center.y + 8), IM_COL32(80, 200, 120, 255), 2.5f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Acercar Zoom");

        ImGui::Spacing();

        // Zoom Out
        ImVec2 pos_z_out = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##z_out", ImVec2(38, 38))) {
            lienzo_.zoom = std::max(lienzo_.zoom / 1.2f, 0.1f);
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_z_out.x + 19.0f, pos_z_out.y + 19.0f);
            dl->AddLine(ImVec2(center.x - 8, center.y), ImVec2(center.x + 8, center.y), IM_COL32(220, 80, 80, 255), 2.5f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Alejar Zoom");

        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();

        // Undo Action
        ImVec2 pos_u = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##undo_btn", ImVec2(38, 38))) {
            deshacer_ultimo();
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_u.x + 19.0f, pos_u.y + 19.0f);
            dl->PathArcTo(ImVec2(center.x + 2, center.y + 2), 6.0f, 3.14159f * 1.0f, 3.14159f * 1.8f);
            dl->PathStroke(IM_COL32(200, 180, 80, 255), false, 2.0f);
            dl->AddTriangleFilled(
                ImVec2(center.x - 4, center.y + 2),
                ImVec2(center.x - 1, center.y - 1),
                ImVec2(center.x - 1, center.y + 5),
                IM_COL32(200, 180, 80, 255));
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Deshacer última acción (Ctrl+Z)");

        ImGui::Spacing();

        // Delete Selected
        ImVec2 pos_d = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##del_btn", ImVec2(38, 38))) {
            if (lienzo_.id_nodo_seleccionado > 0) {
                flowsheet_->eliminar_nodo(lienzo_.id_nodo_seleccionado);
                lienzo_.id_nodo_seleccionado = -1;
            } else if (lienzo_.id_flujo_seleccionado > 0) {
                flowsheet_->eliminar_flujo(lienzo_.id_flujo_seleccionado);
                lienzo_.id_flujo_seleccionado = -1;
            } else if (lienzo_.id_anotacion_seleccionada > 0) {
                flowsheet_->eliminar_anotacion(lienzo_.id_anotacion_seleccionada);
                lienzo_.id_anotacion_seleccionada = -1;
            }
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_d.x + 19.0f, pos_d.y + 19.0f);
            dl->AddLine(ImVec2(center.x - 7, center.y - 7), ImVec2(center.x + 7, center.y + 7), IM_COL32(230, 70, 70, 255), 2.5f);
            dl->AddLine(ImVec2(center.x + 7, center.y - 7), ImVec2(center.x - 7, center.y + 7), IM_COL32(230, 70, 70, 255), 2.5f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Eliminar seleccionado (Delete)");

        ImGui::Spacing();

        // Clear Canvas
        ImVec2 pos_c = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##clear_btn", ImVec2(38, 38))) {
            ImGui::OpenPopup("Confirmar Nueva");
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_c.x + 19.0f, pos_c.y + 19.0f);
            dl->AddRect(ImVec2(center.x - 6, center.y - 8), ImVec2(center.x + 6, center.y + 8), IM_COL32(200, 200, 220, 255), 1.5f);
            dl->AddLine(ImVec2(center.x - 3, center.y - 3), ImVec2(center.x + 3, center.y - 3), IM_COL32(200, 200, 220, 255), 1.0f);
            dl->AddLine(ImVec2(center.x - 3, center.y + 1), ImVec2(center.x + 3, center.y + 1), IM_COL32(200, 200, 220, 255), 1.0f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Limpiar lienzo completo (Nuevo)");

        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();

        // ── Controls ──
        ImGui::TextColored(ImVec4(0.5f, 0.85f, 0.6f, 1.0f), "RUN");
        ImGui::Spacing();

        // Run Simulation
        ImVec2 pos_run = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##run_btn", ImVec2(38, 38))) {
            ejecutar_simulacion();
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 c(pos_run.x + 19.0f, pos_run.y + 19.0f);
            dl->AddTriangleFilled(ImVec2(c.x-6, c.y-8), ImVec2(c.x-6, c.y+8), ImVec2(c.x+8, c.y), IM_COL32(80, 220, 100, 255));
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Ejecutar Simulación (Run)"));

        ImGui::Spacing();

        // Reset Simulation
        ImVec2 pos_rst = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##rst_btn", ImVec2(38, 38))) {
            reiniciar_simulacion();
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 c(pos_rst.x + 19.0f, pos_rst.y + 19.0f);
            dl->AddRectFilled(ImVec2(c.x-6, c.y-6), ImVec2(c.x+6, c.y+6), IM_COL32(220, 80, 80, 255));
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Reiniciar Simulación (Reset)"));
    }
    ImGui::End();
    ImGui::PopStyleVar(2);

    // ── Panel inferior ──
    float inf_y = tam_vp.y - inf_h - stat_h;
    ImGui::SetNextWindowPos(ImVec2(0, inf_y));
    ImGui::SetNextWindowSize(ImVec2(tam_vp.x, inf_h));
    ImGui::Begin(tr("Resultados"), nullptr, ImGuiWindowFlags_NoCollapse);
    panel_resultados_.renderizar();
    ImGui::End();

    // Splitter inferior (resultados / lienzo)
    {   bool& drag_b = layout_drag_b_;
        float r_x1=0, r_y1=inf_y, r_x2=tam_vp.x, r_y2=inf_y+8;
        bool hover_b = punto_en_rect(mp, r_x1, r_y1, r_x2, r_y2);
        if (hover_b && io.MouseClicked[0]) drag_b = true;
        if (drag_b) { inf_h -= io.MouseDelta.y; if (!io.MouseDown[0]) drag_b = false; }
        inf_h = std::max(60.0f, std::min(inf_h, tam_vp.y * 0.6f));
        if (hover_b || drag_b) ImGui::SetMouseCursor(ImGuiMouseCursor_ResizeNS);
        auto* dl = ImGui::GetForegroundDrawList();
        ImU32 col = drag_b ? IM_COL32(26, 115, 232, 200) : hover_b ? IM_COL32(150, 150, 150, 150) : IM_COL32(0, 0, 0, 0);
        dl->AddRectFilled(ImVec2(0,inf_y), ImVec2(tam_vp.x,inf_y+8), col);
        if (hover_b || drag_b) {
            dl->AddRectFilled(ImVec2(tam_vp.x/2 - 20, inf_y+3), ImVec2(tam_vp.x/2 + 20, inf_y+5), IM_COL32(255, 255, 255, 180));
        }
    }

    disp_h = tam_vp.y - menu_h - inf_h - stat_h;
    float esp_h = disp_h * esp_ratio;

    // ── Panel derecho superior: Propiedades ──
    ImGui::SetNextWindowPos(ImVec2(tam_vp.x - der_w, menu_h));
    ImGui::SetNextWindowSize(ImVec2(der_w, esp_h));
    ImGui::Begin(tr("Propiedades"), nullptr, ImGuiWindowFlags_NoCollapse);
    panel_propiedades_.renderizar();
    ImGui::End();

    // Splitter derecho (propiedades / caja de herramientas)
    {   bool& drag_r = layout_drag_r_;
        float r_x1=tam_vp.x-der_w, r_y1=menu_h+esp_h-4, r_x2=tam_vp.x, r_y2=menu_h+esp_h+4;
        bool hover_r = punto_en_rect(mp, r_x1, r_y1, r_x2, r_y2);
        if (hover_r && io.MouseClicked[0]) drag_r = true;
        if (drag_r) { esp_ratio += io.MouseDelta.y / disp_h; if (!io.MouseDown[0]) drag_r = false; }
        esp_ratio = std::max(0.15f, std::min(esp_ratio, 0.85f));
        esp_h = disp_h * esp_ratio;
        if (hover_r || drag_r) ImGui::SetMouseCursor(ImGuiMouseCursor_ResizeNS);
        auto* dl = ImGui::GetForegroundDrawList();
        ImU32 col = drag_r ? IM_COL32(26, 115, 232, 200) : hover_r ? IM_COL32(150, 150, 150, 150) : IM_COL32(0, 0, 0, 0);
        dl->AddRectFilled(ImVec2(tam_vp.x-der_w,menu_h+esp_h-4), ImVec2(tam_vp.x,menu_h+esp_h+4), col);
        if (hover_r || drag_r) {
            dl->AddRectFilled(ImVec2(tam_vp.x-der_w/2 - 20, menu_h+esp_h-1), ImVec2(tam_vp.x-der_w/2 + 20, menu_h+esp_h+1), IM_COL32(255, 255, 255, 180));
        }
    }

    // ── Panel derecho inferior: Caja de Herramientas y Paleta de Equipos (Unit Icons) ──
    float prop_y = menu_h + esp_h;
    ImGui::SetNextWindowPos(ImVec2(tam_vp.x - der_w, prop_y));
    ImGui::SetNextWindowSize(ImVec2(der_w, disp_h - esp_h));
    ImGui::Begin(tr("Caja de Herramientas"), nullptr, ImGuiWindowFlags_NoCollapse);
    {
        if (ImGui::BeginTabBar("HSCRightBottomTabs")) {
            if (ImGui::BeginTabItem(tr("Unit Icons"))) {
                ImGui::BeginChild("UnitIconsScroll", ImVec2(0, 0), false, ImGuiWindowFlags_HorizontalScrollbar);
                
                std::vector<nucleo::TipoNodo> types = {
                    nucleo::TipoNodo::Fuente,
                    nucleo::TipoNodo::Sumidero,
                    nucleo::TipoNodo::Mezclador,
                    nucleo::TipoNodo::Divisor,
                    nucleo::TipoNodo::Reactor,
                    nucleo::TipoNodo::Espesador,
                    nucleo::TipoNodo::Filtro,
                    nucleo::TipoNodo::CeldaFlotacion,
                    nucleo::TipoNodo::MolinoSAG,
                    nucleo::TipoNodo::MolinoBolas,
                    nucleo::TipoNodo::Hidrociclon,
                    nucleo::TipoNodo::TrituradoraGiratoria,
                    nucleo::TipoNodo::Zaranda,
                    nucleo::TipoNodo::IntercambiadorCalor,
                    nucleo::TipoNodo::ColumnaDestilacion,
                    nucleo::TipoNodo::Compresor,
                    nucleo::TipoNodo::Válvula,
                    nucleo::TipoNodo::Separador
                };

                float col_width = 75.0f;
                int cols = std::max(1, (int)(ImGui::GetContentRegionAvail().x / col_width));
                if (ImGui::BeginTable("unit_icons_grid", cols)) {
                    for (size_t i = 0; i < types.size(); ++i) {
                        ImGui::TableNextColumn();
                        
                        nucleo::TipoNodo t = types[i];
                        ImGui::PushID((int)t);
                        
                        ImVec2 p = ImGui::GetCursorScreenPos();
                        ImVec2 btn_size(60, 60);

                        auto* draw_list = ImGui::GetWindowDrawList();
                        bool is_hover = ImGui::IsMouseHoveringRect(p, ImVec2(p.x + btn_size.x, p.y + btn_size.y));
                        ImU32 col_bg = is_hover ? IM_COL32(180, 200, 240, 60) : IM_COL32(230, 230, 235, 180);
                        draw_list->AddRectFilled(p, ImVec2(p.x + btn_size.x, p.y + btn_size.y), col_bg, 4.0f);
                        draw_list->AddRect(p, ImVec2(p.x + btn_size.x, p.y + btn_size.y), IM_COL32(180, 180, 190, 200), 4.0f);

                        // Dibujar vista previa del equipo en miniatura vectorial
                        Lienzo::dibujar_modelo(draw_list, ImVec2(p.x + 30, p.y + 30), 30.0f, t);

                        if (ImGui::InvisibleButton("##btn", btn_size)) {
                            lienzo_.guardar_estado();
                            auto id = flowsheet_->agregar_nodo(t);
                            auto* n = flowsheet_->nodo_por_id(id);
                            if (n) {
                                n->nombre = std::string(nucleo::nombre_tipo_nodo(t)) + " #" + std::to_string(id);
                                n->canvas_x = 50.0f + (contador_nodos_ % 5) * 160.0f;
                                n->canvas_y = 50.0f + (contador_nodos_ / 5) * 90.0f;
                                n->pagina = lienzo_.pagina_actual;
                                contador_nodos_++;
                            }
                            panel_propiedades_.seleccionar_nodo(id);
                            lienzo_.id_nodo_seleccionado = id;
                            lienzo_.id_flujo_seleccionado = -1;
                            lienzo_.id_anotacion_seleccionada = -1;
                            hay_cambios_sin_guardar_ = true;
                        }

                        if (ImGui::BeginDragDropSource(ImGuiDragDropFlags_SourceNoDisableHover)) {
                            ImGui::SetDragDropPayload("tipo_nodo", &t, sizeof(t));
                            ImGui::Text(tr("%s"), nucleo::nombre_tipo_nodo(t));
                            ImGui::EndDragDropSource();
                        }

                        ImGui::TextWrapped(tr("%s"), nucleo::nombre_tipo_nodo(t));
                        
                        ImGui::PopID();
                    }
                    ImGui::EndTable();
                }
                ImGui::EndChild();
                ImGui::EndTabItem();
            }

            if (ImGui::BeginTabItem(tr("Especie"))) {
                panel_especies_.renderizar();
                ImGui::EndTabItem();
            }

            if (ImGui::BeginTabItem(tr("Árbol de proceso"))) {
                ImGui::Text(tr("Elementos del Proceso:"));
                ImGui::Separator();
                if (ImGui::TreeNode(tr("Unidades (Nodos)"))) {
                    for (auto& n : flowsheet_->nodos()) {
                        bool sel = (lienzo_.id_nodo_seleccionado == n.id);
                        if (ImGui::Selectable((n.nombre + "##node").c_str(), sel)) {
                            panel_propiedades_.seleccionar_nodo(n.id);
                            lienzo_.id_nodo_seleccionado = n.id;
                            lienzo_.id_flujo_seleccionado = -1;
                        }
                    }
                    ImGui::TreePop();
                }
                if (ImGui::TreeNode(tr("Corrientes (Flujos)"))) {
                    for (auto& [fid, f] : flowsheet_->mapa_flujos()) {
                        bool sel = (lienzo_.id_flujo_seleccionado == fid);
                        std::string name = f.nombre.empty() ? ("Flujo " + std::to_string(fid)) : f.nombre;
                        if (ImGui::Selectable((name + "##stream").c_str(), sel)) {
                            panel_propiedades_.seleccionar_flujo(fid);
                            lienzo_.id_flujo_seleccionado = fid;
                            lienzo_.id_nodo_seleccionado = -1;
                        }
                    }
                    ImGui::TreePop();
                }
                ImGui::EndTabItem();
            }

            if (ImGui::BeginTabItem(tr("Log Viewer"))) {
                ImGui::Text(tr("Registro de Simulación:"));
                ImGui::Separator();
                ImGui::TextDisabled(tr("Simulación lista."));
                ImGui::TextDisabled(tr("Haga clic en 'Play' en la barra superior."));
                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }
    }
    ImGui::End();

    // Splitter derecho vertical (derecha / lienzo)
    {   bool& drag_rv = layout_drag_rv_;
        float r_x1=tam_vp.x-der_w-8, r_y1=menu_h, r_x2=tam_vp.x-der_w, r_y2=menu_h+disp_h;
        bool hover_rv = punto_en_rect(mp, r_x1, r_y1, r_x2, r_y2);
        if (hover_rv && io.MouseClicked[0]) drag_rv = true;
        if (drag_rv) { der_w -= io.MouseDelta.x; if (!io.MouseDown[0]) drag_rv = false; }
        der_w = std::max(120.0f, std::min(der_w, tam_vp.x - izq_w - min_panel));
        if (hover_rv || drag_rv) ImGui::SetMouseCursor(ImGuiMouseCursor_ResizeEW);
        auto* dl = ImGui::GetForegroundDrawList();
        ImU32 col = drag_rv ? IM_COL32(26, 115, 232, 200) : hover_rv ? IM_COL32(150, 150, 150, 150) : IM_COL32(0, 0, 0, 0);
        dl->AddRectFilled(ImVec2(tam_vp.x-der_w-8,menu_h), ImVec2(tam_vp.x-der_w,menu_h+disp_h), col);
        if (hover_rv || drag_rv) {
            dl->AddRectFilled(ImVec2(tam_vp.x-der_w-5, menu_h+disp_h/2 - 20), ImVec2(tam_vp.x-der_w-3, menu_h+disp_h/2 + 20), IM_COL32(255, 255, 255, 180));
        }
    }

    // ── Lienzo central ──
    // ── Barra de estado ──
    ImGui::SetNextWindowPos(ImVec2(0, tam_vp.y - stat_h));
    ImGui::SetNextWindowSize(ImVec2(tam_vp.x, stat_h));
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(6, 2));
    ImGui::Begin("###statusbar", nullptr,
        ImGuiWindowFlags_NoDecoration | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoScrollWithMouse);
    {
        // Pestañas de página al estilo Excel/HSC Sim
        for (int p = 0; p < 3; p++) {
            char tab_label[16];
            snprintf(tab_label, sizeof(tab_label), "Page %d", p + 1);
            bool is_active = (lienzo_.pagina_actual == p);
            if (is_active) {
                ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.18f, 0.28f, 0.45f, 1.0f));
                ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
            } else {
                ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.12f, 0.12f, 0.15f, 0.5f));
                ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.6f, 0.6f, 0.7f, 1.0f));
            }
            if (ImGui::Button(tab_label, ImVec2(65, 20))) {
                lienzo_.pagina_actual = p;
            }
            ImGui::PopStyleColor(2);
            ImGui::SameLine();
        }

        if (ImGui::SmallButton("Catálogo sin verificar")) mostrar_catalog_audit_=true;
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("CMA Chemistry es independiente de HSC/Metso. Consulte fuentes y rangos antes de usar resultados del catálogo heredado.");
        
        ImGui::SameLine();
        if (hay_cambios_sin_guardar_) {
            ImGui::TextColored(ImVec4(1.0f, 0.8f, 0.2f, 1), " ●");
            if (ImGui::IsItemHovered()) ImGui::SetTooltip("Cambios sin guardar");
        }

        ImGui::SameLine();
        ImGui::SetCursorPosX(420.0f);
        ImGui::TextColored(ImVec4(0.4f,0.7f,0.4f,1), "| Nodos: %zu  |  Flujos: %zu", flowsheet_->conteo_nodos(), flowsheet_->conteo_flujos());

        ImGui::SameLine();
        if (sim_resuelto_) {
            ImGui::TextColored(ImVec4(0.3f, 0.9f, 0.3f, 1), "  | Sim: %s", sim_ultimo_resultado_.convergio ? "OK" : "Error");
            if (ImGui::IsItemHovered()) ImGui::SetTooltip("%s", sim_ultimo_resultado_.mensaje.c_str());
        }

        ImGui::SameLine();
        ImGui::SetCursorPosX(tam_vp.x - 520);
        ImGui::TextColored(ImVec4(0.8f,0.8f,0.8f,1), "Mouse: %d, %d", (int)lienzo_.coord_raton.x, (int)lienzo_.coord_raton.y);

        ImGui::SameLine();
        ImGui::SetCursorPosX(tam_vp.x - 390);
        ImGui::Checkbox(tr("Persist Tool"), &persist_tool_);

        ImGui::SameLine();
        ImGui::SetCursorPosX(tam_vp.x - 280);
        ImGui::Checkbox(tr("Ajustar a grilla"), &lienzo_.ajustar_grilla);

        ImGui::SameLine();
        ImGui::SetCursorPosX(tam_vp.x - 140);
        ImGui::SetNextItemWidth(70);
        float z_pct = lienzo_.zoom * 100.0f;
        if (ImGui::SliderFloat("##zoom_slider", &z_pct, 10.0f, 200.0f, "%.0f%%")) {
            lienzo_.zoom = z_pct / 100.0f;
        }
    }
    ImGui::End();
    ImGui::PopStyleVar();

    renderizar_modulos();
}



}} // namespace
