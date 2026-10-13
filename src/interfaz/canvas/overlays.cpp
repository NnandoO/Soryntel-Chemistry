#include "shared.hpp"
namespace cma { namespace interfaz {
void Lienzo::renderizar_minimapa(const ImVec2& pos, const ImVec2& tam) {
    if (!flowsheet_ || flowsheet_->nodos().empty()) return;

    ImDrawList* draw = ImGui::GetWindowDrawList();
    float mm_w = 150.0f, mm_h = 100.0f;
    float pad = 8.0f;
    ImVec2 mm0(pos.x+tam.x-mm_w-pad, pos.y+tam.y-mm_h-pad);
    ImVec2 mm1(mm0.x+mm_w, mm0.y+mm_h);

    draw->AddRectFilled(mm0, mm1, IM_COL32(15,15,20,200), 4);
    draw->AddRect(mm0, mm1, IM_COL32(60,60,70,150), 4);

    // Calcular bounds
    float xmin=1e9, ymin=1e9, xmax=-1e9, ymax=-1e9;
    for (auto& n : flowsheet_->nodos()) {
        xmin = std::min(xmin, n.canvas_x);
        ymin = std::min(ymin, n.canvas_y);
        xmax = std::max(xmax, n.canvas_x+NODO_ANCHO);
        ymax = std::max(ymax, n.canvas_y+NODO_ALTO);
    }
    float rw = xmax-xmin, rh = ymax-ymin;
    if (rw < 1) rw=1;
    if (rh < 1) rh=1;

    float esc = std::min(mm_w/rw, mm_h/rh)*0.9f;
    float cx = (mm0.x+mm1.x)*0.5f;
    float cy = (mm0.y+mm1.y)*0.5f;
    float ox = cx - (xmin+xmax)*0.5f*esc;
    float oy = cy - (ymin+ymax)*0.5f*esc;

    for (auto& n : flowsheet_->nodos()) {
        float nx = ox + n.canvas_x*esc;
        float ny = oy + n.canvas_y*esc;
        float nw = NODO_ANCHO*esc;
        float nh = NODO_ALTO*esc;
        ImU32 c = col_cab(n.tipo);
        if (n.id == id_nodo_seleccionado)
            draw->AddRectFilled(ImVec2(nx,ny), ImVec2(nx+nw,ny+nh), IM_COL32(255,210,60,200), 1);
        else
            draw->AddRectFilled(ImVec2(nx,ny), ImVec2(nx+nw,ny+nh), c, 1);
    }

    // Viewport
    float vx = ox - desplazamiento.x/zoom*esc;
    float vy = oy - desplazamiento.y/zoom*esc;
    float vw = tam.x/zoom*esc;
    float vh = tam.y/zoom*esc;
    draw->AddRect(ImVec2(vx,vy), ImVec2(vx+vw,vy+vh), IM_COL32(255,255,255,80), 1, 0, 1);

    // Cerrar botón
    ImVec2 cb(mm1.x-14, mm0.y+2);
    draw->AddText(cb, IM_COL32(180,180,200,150), "x");
    ImVec2 m = ImGui::GetIO().MousePos;
    
    // Interacción
    if (hover_rect(m, mm0, mm1)) {
        if (clic_en_lienzo_ && m.x > cb.x && m.x < cb.x+12 && m.y > cb.y && m.y < cb.y+12) {
            mostrar_minimapa = false;
        } else if (ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
            // Mover el desplazamiento para centrar la cámara en m
            float target_canvas_x = (m.x - ox) / esc;
            float target_canvas_y = (m.y - oy) / esc;
            
            desplazamiento.x = -(target_canvas_x * zoom) + tam.x * 0.5f;
            desplazamiento.y = -(target_canvas_y * zoom) + tam.y * 0.5f;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Overlay de zoom
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_overlay_zoom(const ImVec2& pos, const ImVec2&) {
    ImDrawList* draw = ImGui::GetWindowDrawList();

    // Zoom % arriba a la izquierda
    char buf[160];
    snprintf(buf, sizeof(buf), "Zoom: %.0f%% | Nodos: %zu | [F] Encajar | [M] Mapa | [G] Snap",
             zoom*100.0f, flowsheet_ ? flowsheet_->conteo_nodos() : 0);
    ImVec2 tsz = ImGui::CalcTextSize(buf);
    draw->AddRectFilled(ImVec2(pos.x+4, pos.y+4), ImVec2(pos.x+tsz.x+10, pos.y+tsz.y+8),
                        IM_COL32(15,15,20,180), 3);
    draw->AddText(ImVec2(pos.x+7, pos.y+5), IM_COL32(180,180,200,200), buf);
}

// ═══════════════════════════════════════════════════════════════════════════
// Menú contextual
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_menu_contextual(const ImVec2&) {
    if (ImGui::BeginPopup("ctx_lienzo")) {
        ImGui::Text(tr("Agregar Nodo")); ImGui::Separator();
        auto op = [&](const char* lbl, nucleo::TipoNodo t) {
            if (ImGui::MenuItem(lbl)) {
                if (al_solicitar_agregar_nodo) al_solicitar_agregar_nodo(t);
                ImGui::CloseCurrentPopup();
            }
        };
        op("Fuente             [F]", nucleo::TipoNodo::Fuente);
        op("Sumidero           [S]", nucleo::TipoNodo::Sumidero);
        op("Mezclador          [M]", nucleo::TipoNodo::Mezclador);
        op("Divisor            [D]", nucleo::TipoNodo::Divisor);
        op("Reactor            [R]", nucleo::TipoNodo::Reactor);
        op("Separador          [S]", nucleo::TipoNodo::Separador);
        op("Interc. Calor      [H]", nucleo::TipoNodo::IntercambiadorCalor);
        op("Col. Destilación   [C]", nucleo::TipoNodo::ColumnaDestilacion);
        op("Compresor          [C]", nucleo::TipoNodo::Compresor);
        ImGui::Separator();
        if (ImGui::MenuItem(tr("Eliminar Seleccionado"), "Supr")) {
            if (id_nodo_seleccionado > 0) { flowsheet_->eliminar_nodo(id_nodo_seleccionado); id_nodo_seleccionado=-1; }
            else if (id_flujo_seleccionado > 0) { flowsheet_->eliminar_flujo(id_flujo_seleccionado); id_flujo_seleccionado=-1; }
            else if (id_anotacion_seleccionada > 0) { flowsheet_->eliminar_anotacion(id_anotacion_seleccionada); id_anotacion_seleccionada=-1; }
        }
        if (ImGui::MenuItem(tr("Centrar Vista"), "F")) encajar_todo();
        ImGui::MenuItem(tr("Minimapa"), "M", &mostrar_minimapa);
        ImGui::MenuItem(tr("Ajustar a Grilla"), "G", &ajustar_grilla);
        ImGui::EndPopup();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Energy Stream (línea roja punteada)
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_flujo_energia(const nucleo::Flujo& flujo, const ImVec2& inicio,
                                      const ImVec2& fin, int64_t fid) {
    ImDrawList* draw = ImGui::GetWindowDrawList();
    float dx = (fin.x-inicio.x)*0.4f;
    ImVec2 cp1(inicio.x+dx, inicio.y), cp2(fin.x-dx, fin.y);

    ImU32 col = (id_flujo_seleccionado==fid) ? IM_COL32(255,100,60,255) : IM_COL32(220,40,40,180);

    // Línea punteada zigzagueante simulando energía
    int segmentos = 40;
    float ancho = 3.0f * zoom;
    for (int i = 0; i < segmentos; i++) {
        float t1 = (float)i / segmentos;
        float t2 = (float)(i+1) / segmentos;
        ImVec2 p1((1-t1)*(1-t1)*(1-t1)*inicio.x + 3*(1-t1)*(1-t1)*t1*cp1.x + 3*(1-t1)*t1*t1*cp2.x + t1*t1*t1*fin.x,
                  (1-t1)*(1-t1)*(1-t1)*inicio.y + 3*(1-t1)*(1-t1)*t1*cp1.y + 3*(1-t1)*t1*t1*cp2.y + t1*t1*t1*fin.y);
        ImVec2 p2((1-t2)*(1-t2)*(1-t2)*inicio.x + 3*(1-t2)*(1-t2)*t2*cp1.x + 3*(1-t2)*t2*t2*cp2.x + t2*t2*t2*fin.x,
                  (1-t2)*(1-t2)*(1-t2)*inicio.y + 3*(1-t2)*(1-t2)*t2*cp1.y + 3*(1-t2)*t2*t2*cp2.y + t2*t2*t2*fin.y);
        if (i % 3 == 0) continue; // efecto punteado
        draw->AddLine(p1, p2, col, ancho);
    }

    // Flecha
    float tf = 0.95f;
    ImVec2 fp((1-tf)*(1-tf)*(1-tf)*inicio.x + 3*(1-tf)*(1-tf)*tf*cp1.x + 3*(1-tf)*tf*tf*cp2.x + tf*tf*tf*fin.x,
              (1-tf)*(1-tf)*(1-tf)*inicio.y + 3*(1-tf)*(1-tf)*tf*cp1.y + 3*(1-tf)*tf*tf*cp2.y + tf*tf*tf*fin.y);
    ImVec2 dir(fin.x-fp.x, fin.y-fp.y);
    float len = std::sqrt(dir.x*dir.x+dir.y*dir.y);
    if (len > 0) { dir.x/=len; dir.y/=len; }
    ImVec2 perp(-dir.y, dir.x);
    float fs = 10*zoom;
    draw->AddTriangleFilled(fin,
        ImVec2(fin.x-dir.x*fs+perp.x*fs*0.5f, fin.y-dir.y*fs+perp.y*fs*0.5f),
        ImVec2(fin.x-dir.x*fs-perp.x*fs*0.5f, fin.y-dir.y*fs-perp.y*fs*0.5f),
        IM_COL32(255,60,60,200));

    // Etiqueta
    float t = 0.5f;
    ImVec2 mid((1-t)*(1-t)*(1-t)*inicio.x + 3*(1-t)*(1-t)*t*cp1.x + 3*(1-t)*t*t*cp2.x + t*t*t*fin.x,
               (1-t)*(1-t)*(1-t)*inicio.y + 3*(1-t)*(1-t)*t*cp1.y + 3*(1-t)*t*t*cp2.y + t*t*t*fin.y);
    char etiq[48];
    snprintf(etiq, sizeof(etiq), "%.1f kW", flujo.energia);
    ImVec2 etsz = ImGui::CalcTextSize(etiq);
    draw->AddRectFilled(ImVec2(mid.x-etsz.x*0.5f-2, mid.y-etsz.y*0.5f-1),
                        ImVec2(mid.x+etsz.x*0.5f+2, mid.y+etsz.y*0.5f+1),
                        IM_COL32(40,20,20,220), 2);
    draw->AddText(ImVec2(mid.x-etsz.x*0.5f, mid.y-etsz.y*0.5f),
                  IM_COL32(255,180,160,220), etiq);
}

// ═══════════════════════════════════════════════════════════════════════════
// Anotaciones (texto, rectángulos, elipses)
// ═══════════════════════════════════════════════════════════════════════════


}} // namespace
