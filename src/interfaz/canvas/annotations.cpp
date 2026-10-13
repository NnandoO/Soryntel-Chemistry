#include "shared.hpp"
namespace cma { namespace interfaz {
void Lienzo::renderizar_anotaciones(const ImVec2& pos) {
    if (!flowsheet_) return;
    ImDrawList* draw = ImGui::GetWindowDrawList();

    for (auto& a : flowsheet_->anotaciones()) {
        if (a.pagina != pagina_actual) continue;
        float ax = pos.x + a.x * zoom + desplazamiento.x;
        float ay = pos.y + a.y * zoom + desplazamiento.y;
        float aw = a.w * zoom;
        float ah = a.h * zoom;

        bool sel = (id_anotacion_seleccionada == a.id);
        ImU32 col_b = sel ? IM_COL32(255, 210, 60, 255) : a.color_borde;
        float bw = sel ? (a.grosor_borde + 1.5f) * zoom : a.grosor_borde * zoom;

        switch (a.tipo) {
            case diagrama_flujo::Anotacion::Tipo::Texto:
                draw->AddRectFilled(ImVec2(ax,ay), ImVec2(ax+aw,ay+ah), a.color_fondo, 4 * zoom);
                draw->AddRect(ImVec2(ax,ay), ImVec2(ax+aw,ay+ah), col_b, 4 * zoom, 0, bw);
                draw->AddText(ImVec2(ax + 6 * zoom, ay + 6 * zoom), IM_COL32(220,220,240,255), a.texto.c_str());
                break;
            case diagrama_flujo::Anotacion::Tipo::Rectangulo:
                draw->AddRectFilled(ImVec2(ax,ay), ImVec2(ax+aw,ay+ah), a.color_fondo, 2 * zoom);
                draw->AddRect(ImVec2(ax,ay), ImVec2(ax+aw,ay+ah), col_b, 2 * zoom, 0, bw);
                draw->AddText(ImVec2(ax + 6 * zoom, ay + 6 * zoom), IM_COL32(200,200,220,180), a.texto.c_str());
                break;
            case diagrama_flujo::Anotacion::Tipo::Elipse:
                draw->AddEllipseFilled(ImVec2(ax+aw*0.5f,ay+ah*0.5f), ImVec2(aw*0.5f,ah*0.5f), a.color_fondo);
                draw->AddEllipse(ImVec2(ax+aw*0.5f,ay+ah*0.5f), ImVec2(aw*0.5f,ah*0.5f), col_b, 0, 0, bw);
                draw->AddText(ImVec2(ax + 6 * zoom, ay + ah * 0.5f - 8 * zoom), IM_COL32(200,200,220,180), a.texto.c_str());
                break;
        }

        // Interacción: Selección y Arrastre de Anotaciones/Dibujos
        ImVec2 mpos = ImGui::GetIO().MousePos;
        bool hover_anot = hover_rect(mpos, ImVec2(ax, ay), ImVec2(ax + aw, ay + ah));
        
        if (hover_anot && clic_en_lienzo_) {
            id_anotacion_seleccionada = a.id;
            id_nodo_seleccionado = -1;
            id_flujo_seleccionado = -1;
            if (al_seleccionar_anotacion) al_seleccionar_anotacion(a.id);
            anotacion_arrastrando_ = a.id;
            clic_detectado_en_objeto_ = true;
        }

        if (anotacion_arrastrando_ == a.id) {
            if (ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
                if (ImGui::IsMouseDragging(ImGuiMouseButton_Left)) {
                    float dx = ImGui::GetIO().MouseDelta.x / zoom;
                    float dy = ImGui::GetIO().MouseDelta.y / zoom;
                    a.x += dx;
                    a.y += dy;
                }
            } else {
                if (ajustar_grilla) {
                    a.x = round(a.x / GRILLA) * GRILLA;
                    a.y = round(a.y / GRILLA) * GRILLA;
                }
                anotacion_arrastrando_ = -1;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tablas de flujo en el lienzo
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_tablas_flujo(const ImVec2& pos) {
    if (!flowsheet_) return;
    ImDrawList* draw = ImGui::GetWindowDrawList();

    for (auto& tabla : flowsheet_->tablas_flujo()) {
        if (tabla.pagina != pagina_actual) continue;
        if (!tabla.visible) continue;

        float tx = pos.x + tabla.x + desplazamiento.x;
        float ty = pos.y + tabla.y + desplazamiento.y;

        draw->AddRectFilled(ImVec2(tx,ty), ImVec2(tx+tabla.w,ty+tabla.h), IM_COL32(20,20,30,240), 4);
        draw->AddRect(ImVec2(tx,ty), ImVec2(tx+tabla.w,ty+tabla.h), IM_COL32(80,80,120,200), 4);

        draw->AddText(ImVec2(tx+6,ty+4), IM_COL32(160,160,200,255), "Stream Table");

        float col_w[4] = {80, 80, 80, tabla.w - 280};
        const char* headers[] = {"Corriente", "Flujo (kmol/s)", "T (K)", "P (atm)"};
        float yh = ty + 24;

        for (int c = 0; c < 4; c++) {
            float cx = tx + 4;
            for (int i = 0; i < c; i++) cx += col_w[i];
            draw->AddText(ImVec2(cx+2, yh), IM_COL32(120,120,160,255), headers[c]);
        }

        int row = 0;
        for (auto& [fid, f] : flowsheet_->mapa_flujos()) {
            if (f.es_energia) continue;
            if (!tabla.ids_flujos_mostrados.empty() &&
                std::find(tabla.ids_flujos_mostrados.begin(), tabla.ids_flujos_mostrados.end(), fid)
                == tabla.ids_flujos_mostrados.end()) continue;

            float ry = yh + 18 + row * 18;
            if (ry + 18 > ty + tabla.h - 4) break;

            char buf[64];
            ImU32 col_r = (row % 2) ? IM_COL32(255,255,255,10) : IM_COL32(255,255,255,4);
            draw->AddRectFilled(ImVec2(tx+2, ry), ImVec2(tx+tabla.w-2, ry+16), col_r, 2);

            float cx = tx + 6;
            draw->AddText(ImVec2(cx, ry+1), IM_COL32(200,200,220,220), f.nombre.c_str());
            cx += col_w[0];

            snprintf(buf, sizeof(buf), "%.4f", f.flujo_total());
            draw->AddText(ImVec2(cx, ry+1), IM_COL32(180,200,180,220), buf);
            cx += col_w[1];

            snprintf(buf, sizeof(buf), "%.1f", f.temperatura);
            draw->AddText(ImVec2(cx, ry+1), IM_COL32(200,180,180,220), buf);
            cx += col_w[2];

            snprintf(buf, sizeof(buf), "%.2f", f.presion);
            draw->AddText(ImVec2(cx, ry+1), IM_COL32(180,180,200,220), buf);

            row++;
        }
    }
}


}} // namespace
