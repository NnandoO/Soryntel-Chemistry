#include "shared.hpp"
namespace cma { namespace interfaz {
void Lienzo::renderizar(const ImVec2&) {
    if (!flowsheet_) return;

    clic_detectado_en_objeto_ = false;

    ImDrawList* draw = ImGui::GetWindowDrawList();
    ImVec2 pos = ImGui::GetCursorScreenPos();
    ImVec2 tam = ImGui::GetContentRegionAvail();
    if (tam.x < 1) tam.x = 1;
    if (tam.y < 1) tam.y = 1;

    ImGui::InvisibleButton("lienzo_bg", tam);
    ImGui::SetCursorScreenPos(pos);

    bool hover = ImGui::IsItemHovered() || ImGui::IsWindowHovered(ImGuiHoveredFlags_ChildWindows);
    bool active = ImGui::IsWindowFocused(ImGuiFocusedFlags_ChildWindows);
    hover_lienzo_ = hover;
    clic_en_lienzo_ = hover && ImGui::IsMouseClicked(ImGuiMouseButton_Left);

    ultima_region_ = tam;

    // ── Atajos de Teclado (Undo, Redo, Portapapeles) ──
    ImGuiIO& io = ImGui::GetIO();
    if (active) {
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_Z)) {
            if (io.KeyShift) rehacer(); else deshacer();
        }
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_Y)) rehacer();
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_C)) copiar();
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_X)) cortar();
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_V)) pegar();
    }

    // ── Animación suave ──
    if (animando_) {
        desplazamiento.x += (des_destino_.x - desplazamiento.x) * 0.12f;
        desplazamiento.y += (des_destino_.y - desplazamiento.y) * 0.12f;
        if (std::abs(des_destino_.x - desplazamiento.x) < 0.5f &&
            std::abs(des_destino_.y - desplazamiento.y) < 0.5f) {
            desplazamiento = des_destino_;
            animando_ = false;
        }
    }

    // Resetear el arrastre si soltamos el botón
    if (!ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
        extern bool g_nodo_arrastrado;
        g_nodo_arrastrado = false;
    }

    // ── Desplazar con botón central ──
    if (hover && ImGui::IsMouseDragging(ImGuiMouseButton_Middle)) {
        desplazamiento.x += ImGui::GetIO().MouseDelta.x;
        desplazamiento.y += ImGui::GetIO().MouseDelta.y;
        arrastrando_lienzo_ = true;
    } else {
        arrastrando_lienzo_ = false;
    }

    // ── Edge scroll (arrastrar con botón derecho cerca del borde) ──
    if (hover && active && ImGui::IsMouseDown(ImGuiMouseButton_Left) && !arrastrando_conexion_) {
        ImVec2 raton = ImGui::GetIO().MousePos;
        float edge = 20.0f;
        float vel = 8.0f;
        if (raton.x - pos.x < edge) desplazamiento.x += vel;
        if (pos.x + tam.x - raton.x < edge) desplazamiento.x -= vel;
        if (raton.y - pos.y < edge) desplazamiento.y += vel;
        if (pos.y + tam.y - raton.y < edge) desplazamiento.y -= vel;
    }

    // ── Zoom con rueda (centrado en cursor) ──
    if (hover) {
        float rueda = ImGui::GetIO().MouseWheel;
        if (rueda != 0) {
            float antiguo = zoom;
            zoom = std::clamp(zoom + rueda * 0.1f, 0.1f, 5.0f);
            ImVec2 raton = ImGui::GetIO().MousePos;
            desplazamiento.x = raton.x - (raton.x - desplazamiento.x) * (zoom / antiguo);
            desplazamiento.y = raton.y - (raton.y - desplazamiento.y) * (zoom / antiguo);
            animando_ = false;
        }
    }

    // ── Inercia ──
    if (!ImGui::IsMouseDown(ImGuiMouseButton_Middle) && !arrastrando_lienzo_) {
        vel_desplazamiento_.x *= INERCIA_DEC;
        vel_desplazamiento_.y *= INERCIA_DEC;
        if (std::abs(vel_desplazamiento_.x) > 0.1f || std::abs(vel_desplazamiento_.y) > 0.1f) {
            desplazamiento.x += vel_desplazamiento_.x;
            desplazamiento.y += vel_desplazamiento_.y;
        }
    }

    // ── Fondo ──
    draw->AddRectFilled(pos, ImVec2(pos.x + tam.x, pos.y + tam.y), ImGui::GetColorU32(ImGuiCol_WindowBg));

    // Definir el origen real del lienzo dentro del margen de la regla
    ImVec2 canvas_orig(pos.x + 25, pos.y + 25);
    bool hover_canvas = hover && (ImGui::GetIO().MousePos.x > canvas_orig.x) && (ImGui::GetIO().MousePos.y > canvas_orig.y);
    if (hover_canvas) {
        coord_raton.x = (ImGui::GetIO().MousePos.x - canvas_orig.x - desplazamiento.x) / zoom;
        coord_raton.y = (ImGui::GetIO().MousePos.y - canvas_orig.y - desplazamiento.y) / zoom;
    }

    // ── Push Clip Rect para aislar el canvas de las reglas ──
    ImGui::PushClipRect(canvas_orig, ImVec2(pos.x + tam.x, pos.y + tam.y), true);

    // ── Cuadrícula CAD (estilo HSC) ──
    float grilla = GRILLA * zoom;
    ImU32 col_g = ImGui::GetColorU32(ImGuiCol_Border, 0.3f);
    ImU32 col_gf = IM_COL32(195, 195, 205, 255);
    float ox = fmod(desplazamiento.x + 25, grilla);
    float oy = fmod(desplazamiento.y + 25, grilla);
    if (grilla > 4) {
        for (float x = ox; x < tam.x; x += grilla) {
            if (pos.x + x < canvas_orig.x) continue;
            bool f = fmod((x - ox) / grilla, 5) < 1;
            draw->AddLine(ImVec2(pos.x + x, canvas_orig.y), ImVec2(pos.x + x, pos.y + tam.y), f ? col_gf : col_g);
        }
        for (float y = oy; y < tam.y; y += grilla) {
            if (pos.y + y < canvas_orig.y) continue;
            bool f = fmod((y - oy) / grilla, 5) < 1;
            draw->AddLine(ImVec2(canvas_orig.x, pos.y + y), ImVec2(pos.x + tam.x, pos.y + y), f ? col_gf : col_g);
        }
    }

    // ── Anotaciones (debajo de los nodos) ──
    renderizar_anotaciones(canvas_orig);

    // ── Tablas de flujo ──
    renderizar_tablas_flujo(canvas_orig);

    // ── Nodos y flujos ──
    renderizar_flujos(canvas_orig);
    for (auto& nodo : flowsheet_->nodos()) {
        if (nodo.pagina != pagina_actual) continue;
        if (nodo.tipo == nucleo::TipoNodo::CajaTexto ||
            nodo.tipo == nucleo::TipoNodo::Rectangulo ||
            nodo.tipo == nucleo::TipoNodo::Elipse) continue;
        renderizar_nodo(nodo, canvas_orig);
    }

    if (arrastrando_conexion_)
        renderizar_conexion_arrastrada(canvas_orig);

    // ── Pop Clip Rect del canvas ──
    ImGui::PopClipRect();

    // ── Reglas de Medición (X e Y) al estilo HSC Sim ──
    ImU32 col_regla_bg = ImGui::GetColorU32(ImGuiCol_MenuBarBg);
    ImU32 col_regla_borde = IM_COL32(180, 180, 190, 255);
    ImU32 col_regla_marca = IM_COL32(100, 100, 110, 255);
    ImU32 col_regla_texto = IM_COL32(50, 50, 60, 255);

    // Esquina superior izquierda (intersección)
    draw->AddRectFilled(pos, canvas_orig, col_regla_bg);
    draw->AddRect(pos, canvas_orig, col_regla_borde);

    // Regla Horizontal (X)
    draw->AddRectFilled(ImVec2(canvas_orig.x, pos.y), ImVec2(pos.x + tam.x, canvas_orig.y), col_regla_bg);
    draw->AddLine(ImVec2(canvas_orig.x, canvas_orig.y), ImVec2(pos.x + tam.x, canvas_orig.y), col_regla_borde, 1.5f);

    // Regla Vertical (Y)
    draw->AddRectFilled(ImVec2(pos.x, canvas_orig.y), ImVec2(canvas_orig.x, pos.y + tam.y), col_regla_bg);
    draw->AddLine(ImVec2(canvas_orig.x, canvas_orig.y), ImVec2(canvas_orig.x, pos.y + tam.y), col_regla_borde, 1.5f);

    // Ticks regla X
    float canvas_x_min = (- desplazamiento.x) / zoom;
    float canvas_x_max = (tam.x - 25 - desplazamiento.x) / zoom;
    int start_val_x = ((int)(canvas_x_min / 10)) * 10 - 10;
    int end_val_x = ((int)(canvas_x_max / 10)) * 10 + 10;

    for (int val = start_val_x; val <= end_val_x; val += 10) {
        if (val < 0) continue;
        float sx = canvas_orig.x + (val * zoom) + desplazamiento.x;
        if (sx < canvas_orig.x || sx > pos.x + tam.x) continue;

        bool major = (val % 50 == 0);
        float h_tick = major ? 12.0f : 6.0f;
        draw->AddLine(ImVec2(sx, canvas_orig.y - h_tick), ImVec2(sx, canvas_orig.y), col_regla_marca, 1.0f);

        if (major) {
            char buf[16];
            snprintf(buf, sizeof(buf), "%d", val);
            draw->AddText(ImVec2(sx + 2, pos.y + 2), col_regla_texto, buf);
        }
    }

    // Ticks regla Y
    float canvas_y_min = (- desplazamiento.y) / zoom;
    float canvas_y_max = (tam.y - 25 - desplazamiento.y) / zoom;
    int start_val_y = ((int)(canvas_y_min / 10)) * 10 - 10;
    int end_val_y = ((int)(canvas_y_max / 10)) * 10 + 10;

    for (int val = start_val_y; val <= end_val_y; val += 10) {
        if (val < 0) continue;
        float sy = canvas_orig.y + (val * zoom) + desplazamiento.y;
        if (sy < canvas_orig.y || sy > pos.y + tam.y) continue;

        bool major = (val % 50 == 0);
        float w_tick = major ? 12.0f : 6.0f;
        draw->AddLine(ImVec2(canvas_orig.x - w_tick, sy), ImVec2(canvas_orig.x, sy), col_regla_marca, 1.0f);

        if (major) {
            char buf[16];
            snprintf(buf, sizeof(buf), "%d", val);
            draw->AddText(ImVec2(pos.x + 2, sy + 2), col_regla_texto, buf);
        }
    }

    // ── Overlays ──
    renderizar_overlay_zoom(pos, tam);
    if (mostrar_minimapa)
        renderizar_minimapa(pos, tam);

    // ── Aviso de conexión rechazada ──
    if (!error_conexion_.empty() && tiempo_error_conexion_ > 0.0f) {
        tiempo_error_conexion_ -= ImGui::GetIO().DeltaTime;
        ImDrawList* draw = ImGui::GetWindowDrawList();
        ImVec2 size = ImGui::CalcTextSize(error_conexion_.c_str());
        ImVec2 p0(pos.x + 16, pos.y + 16);
        draw->AddRectFilled(ImVec2(p0.x - 10, p0.y - 6),
                            ImVec2(p0.x + size.x + 10, p0.y + size.y + 8),
                            IM_COL32(90, 20, 20, 230), 6.0f);
        draw->AddText(p0, IM_COL32(255, 210, 210, 255), error_conexion_.c_str());
        if (tiempo_error_conexion_ <= 0.0f) error_conexion_.clear();
    }

    // ── Menú contextual ──
    if (hover_canvas && ImGui::IsMouseClicked(ImGuiMouseButton_Right))
        ImGui::OpenPopup("ctx_lienzo");
    renderizar_menu_contextual(pos);

    // ── Drop desde herramientas ──
    if (ImGui::BeginDragDropTarget()) {
        if (const ImGuiPayload* pl = ImGui::AcceptDragDropPayload("tipo_nodo")) {
            guardar_estado();
            nucleo::TipoNodo tipo;
            memcpy(&tipo, pl->Data, sizeof(tipo));
            ImVec2 raton = ImGui::GetIO().MousePos;
            float nx = (raton.x - pos.x - 25 - desplazamiento.x - NODO_ANCHO * zoom * 0.5f) / zoom;
            float ny = (raton.y - pos.y - 25 - desplazamiento.y - NODO_ALTO * zoom * 0.5f) / zoom;
            int64_t id = flowsheet_->agregar_nodo(tipo);
            auto* n = flowsheet_->nodo_por_id(id);
            if (n) {
                n->nombre = std::string(nucleo::nombre_tipo_nodo(tipo)) + " #" + std::to_string(id);
                if (ajustar_grilla) {
                    n->canvas_x = round(nx / GRILLA) * GRILLA;
                    n->canvas_y = round(ny / GRILLA) * GRILLA;
                } else {
                    n->canvas_x = nx;
                    n->canvas_y = ny;
                }
                n->pagina = pagina_actual;
            }
            id_nodo_seleccionado = id;
            id_flujo_seleccionado = -1;
            id_anotacion_seleccionada = -1;
            if (al_seleccionar_nodo) al_seleccionar_nodo(id);
        }
        ImGui::EndDragDropTarget();
    }

    // ── Creación de Dibujos (Anotaciones) ──
    if (hover_canvas && ImGui::IsMouseClicked(ImGuiMouseButton_Left) && !arrastrando_conexion_ && !creando_anotacion_ &&
        (modo_dibujo == ModoDibujo::Texto || modo_dibujo == ModoDibujo::Rectangulo || modo_dibujo == ModoDibujo::Elipse)) {
        
        ImVec2 raton = ImGui::GetIO().MousePos;
        bool sobre_nodo = false;
        for (auto& nodo : flowsheet_->nodos()) {
            if (nodo.pagina != pagina_actual) continue;
            ImVec2 np(canvas_orig.x + nodo.canvas_x + desplazamiento.x,
                      canvas_orig.y + nodo.canvas_y + desplazamiento.y);
            ImVec2 ns(NODO_ANCHO * zoom, NODO_ALTO * zoom);
            if (hover_rect(raton, np, ImVec2(np.x + ns.x, np.y + ns.y))) {
                sobre_nodo = true;
                break;
            }
        }
        
        if (!sobre_nodo) {
            float cx = (raton.x - canvas_orig.x - desplazamiento.x) / zoom;
            float cy = (raton.y - canvas_orig.y - desplazamiento.y) / zoom;
            
            guardar_estado();
            std::string default_text = (modo_dibujo == ModoDibujo::Texto) ? "Caja de Texto" :
                                       (modo_dibujo == ModoDibujo::Rectangulo) ? "Rectángulo" : "Elipse";
            int64_t aid = flowsheet_->agregar_anotacion(default_text);
            for (auto& a : flowsheet_->anotaciones()) {
                if (a.id == aid) {
                    a.x = cx;
                    a.y = cy;
                    a.w = 10.0f;
                    a.h = 10.0f;
                    a.pagina = pagina_actual;
                    a.tipo = (modo_dibujo == ModoDibujo::Texto) ? diagrama_flujo::Anotacion::Tipo::Texto :
                             (modo_dibujo == ModoDibujo::Rectangulo) ? diagrama_flujo::Anotacion::Tipo::Rectangulo :
                             diagrama_flujo::Anotacion::Tipo::Elipse;
                    
                    if (modo_dibujo == ModoDibujo::Rectangulo) {
                        a.color_fondo = 0x3C3C4A80; // Gris semi-transparente
                        a.color_borde = 0x5C9CFFC8; // Borde azul claro
                    } else if (modo_dibujo == ModoDibujo::Elipse) {
                        a.color_fondo = 0x4A3C4A80; // Púrpura semi-transparente
                        a.color_borde = 0xFF9C5FC8; // Borde púrpura
                    } else {
                        a.color_fondo = 0x1A1A24C8; // Fondo oscuro para texto
                        a.color_borde = 0x64648CC8;
                    }
                    break;
                }
            }
            
            id_anotacion_seleccionada = aid;
            id_nodo_seleccionado = -1;
            id_flujo_seleccionado = -1;
            
            creando_anotacion_ = true;
            id_anotacion_creando_ = aid;
            inicio_creacion_anotacion_ = ImVec2(cx, cy);
            
            if (al_seleccionar_anotacion) al_seleccionar_anotacion(aid);
            if (al_crear_anotacion) al_crear_anotacion(aid);
        }
    }

    // ── Actualizar redimensionamiento durante creación ──
    if (creando_anotacion_) {
        float cur_cx = (ImGui::GetIO().MousePos.x - canvas_orig.x - desplazamiento.x) / zoom;
        float cur_cy = (ImGui::GetIO().MousePos.y - canvas_orig.y - desplazamiento.y) / zoom;
        
        float x0 = std::min(inicio_creacion_anotacion_.x, cur_cx);
        float y0 = std::min(inicio_creacion_anotacion_.y, cur_cy);
        float w = std::abs(cur_cx - inicio_creacion_anotacion_.x);
        float h = std::abs(cur_cy - inicio_creacion_anotacion_.y);
        
        if (ajustar_grilla) {
            x0 = roundf(x0 / GRILLA) * GRILLA;
            y0 = roundf(y0 / GRILLA) * GRILLA;
            w = std::max(GRILLA, roundf(w / GRILLA) * GRILLA);
            h = std::max(GRILLA, roundf(h / GRILLA) * GRILLA);
        } else {
            w = std::max(10.0f, w);
            h = std::max(10.0f, h);
        }
        
        for (auto& a : flowsheet_->anotaciones()) {
            if (a.id == id_anotacion_creando_) {
                a.x = x0;
                a.y = y0;
                a.w = w;
                a.h = h;
                break;
            }
        }
        
        if (!ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
            creando_anotacion_ = false;
            id_anotacion_creando_ = -1;
            if (!persistir_herramienta) modo_dibujo = ModoDibujo::Seleccion;
        }
    }
    // ── Deseleccionar en Selección Normal ──
    else if (hover_canvas && ImGui::IsMouseClicked(ImGuiMouseButton_Left) && !arrastrando_conexion_ && !clic_detectado_en_objeto_) {
        id_nodo_seleccionado = -1;
        id_flujo_seleccionado = -1;
        id_anotacion_seleccionada = -1;
        if (al_seleccionar_nodo) al_seleccionar_nodo(-1);
        if (al_seleccionar_anotacion) al_seleccionar_anotacion(-1);
    }

    // ── Atajos ──
    if (hover) {
        if (ImGui::IsKeyPressed(ImGuiKey_F)) encajar_todo();
        if (ImGui::IsKeyPressed(ImGuiKey_M)) mostrar_minimapa = !mostrar_minimapa;
        if (ImGui::IsKeyPressed(ImGuiKey_G)) ajustar_grilla = !ajustar_grilla;
    }

    // ── Eliminación diferida de nodos ──
    for (int64_t id : nodos_a_eliminar_) {
        if (flowsheet_) flowsheet_->eliminar_nodo(id);
        puertos_entrada_.erase(id);
        puertos_salida_.erase(id);
    }
    nodos_a_eliminar_.clear();
}

// ═══════════════════════════════════════════════════════════════════════════
// Renderizar Nodo
// ═══════════════════════════════════════════════════════════════════════════


}} // namespace
