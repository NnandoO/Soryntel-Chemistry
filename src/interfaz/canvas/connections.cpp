#include "shared.hpp"
#include "base_datos/gestor_db.hpp"
namespace cma { namespace interfaz {
static double valor_visualizacion_flujo(const nucleo::Flujo& flujo, int modo,
                                        const std::map<int64_t, nucleo::Especie>& db,
                                        const char*& unidad_out) {
    switch (modo) {
        case 1: unidad_out = "t/h";    return flujo.masa_total_t_h(db);
        case 2: unidad_out = "kmol/s"; return flujo.flujo_molar_total();
        case 3: unidad_out = "kg/s";   return flujo.masa_total(db);
        case 4: unidad_out = "°C";     return flujo.temperatura - 273.15;
        case 5: unidad_out = "bar";    return flujo.presion * 1.01325;
        case 6: unidad_out = "kW";     return flujo.entalpia;
        default: unidad_out = "";      return 0.0;
    }
}

void Lienzo::renderizar_flujos(const ImVec2&) {
    if (!flowsheet_) return;
    ImDrawList* draw = ImGui::GetWindowDrawList();
    const auto& db = repo_ ? repo_->mapa_especies() : base_datos::GestorDB::instancia().mapa_especies();

    // Pre-cálculo del valor máximo absoluto para escalar el diagrama de Sankey
    double max_abs_val = 0.0;
    if (sankey_activo && modo_visualizacion_flujos > 0) {
        const char* u_dummy = "";
        for (const auto& [fid, f] : flowsheet_->mapa_flujos()) {
            if (f.pagina != pagina_actual || f.es_energia) continue;
            double v = std::abs(valor_visualizacion_flujo(f, modo_visualizacion_flujos, db, u_dummy));
            if (v > max_abs_val) max_abs_val = v;
        }
    }

    for (auto& [fid, flujo] : flowsheet_->mapa_flujos()) {
        if (flujo.pagina != pagina_actual) continue;
        ImVec2 inicio(0,0), fin(0,0);
        bool encontrado = false;

        for (auto& nodo : flowsheet_->nodos()) {
            for (int64_t sid : nodo.ids_flujos_salida)
                if (sid == fid) {
                    auto it = puertos_salida_.find(nodo.id);
                    if (it != puertos_salida_.end() && !it->second.empty()) {
                        size_t idx = 0;
                        for(size_t k=0; k<nodo.ids_flujos_salida.size(); ++k) if(nodo.ids_flujos_salida[k] == fid) idx = k;
                        if (idx < it->second.size()) inicio = it->second[idx];
                        else inicio = it->second.back();
                    }
                }
            for (int64_t sid : nodo.ids_flujos_entrada)
                if (sid == fid) {
                    auto it = puertos_entrada_.find(nodo.id);
                    if (it != puertos_entrada_.end() && !it->second.empty()) {
                        size_t idx = 0;
                        for(size_t k=0; k<nodo.ids_flujos_entrada.size(); ++k) if(nodo.ids_flujos_entrada[k] == fid) idx = k;
                        if (idx < it->second.size()) fin = it->second[idx];
                        else fin = it->second.back();
                        encontrado = true;
                    }
                }
        }

        if (!encontrado) continue;

        // ── Energy stream (rojo punteado) ──
        if (flujo.es_energia) {
            renderizar_flujo_energia(flujo, inicio, fin, fid);
            continue;
        }

        float dx = (fin.x-inicio.x)*0.4f;
        if (std::abs(dx) < 35.0f * zoom) dx = (fin.x >= inicio.x ? 35.0f : -35.0f) * zoom;
        ImVec2 cp1(inicio.x+dx, inicio.y), cp2(fin.x-dx, fin.y);

        const char* unidad_str = "";
        double val_vis = valor_visualizacion_flujo(flujo, modo_visualizacion_flujos, db, unidad_str);
        float flujo_total = static_cast<float>(flujo.flujo_total());

        // Cálculo de grosor y color (Sankey vs normal)
        float base_ancho = 2.2f;
        if (sankey_activo && modo_visualizacion_flujos > 0) {
            float max_w = sankey_usar_max_ancho ? std::clamp(sankey_max_ancho_px, 2.0f, 36.0f) : 16.0f;
            float ratio = (max_abs_val > 1e-9) ? static_cast<float>(std::abs(val_vis) / max_abs_val) : 0.0f;
            ratio = std::clamp(ratio, 0.0f, 1.0f);
            base_ancho = 1.8f + ratio * (max_w - 1.8f);
        } else {
            base_ancho = 1.8f + std::min(flujo_total * 0.4f, 8.0f);
        }
        float ancho = (id_flujo_seleccionado == fid) ? (base_ancho + 2.5f) * zoom : base_ancho * zoom;

        ImU32 col_c;
        if (id_flujo_seleccionado == fid) {
            col_c = IM_COL32(255, 210, 60, 245);
        } else if (sankey_activo && sankey_usar_color && modo_visualizacion_flujos > 0) {
            const float* c = (val_vis >= 0.0) ? sankey_color_pos : sankey_color_neg;
            col_c = IM_COL32((int)(c[0]*255), (int)(c[1]*255), (int)(c[2]*255), 215);
        } else {
            float factor = std::min(flujo_total / 20.0f, 1.0f);
            int r = (int)(130 - factor * 80);
            int g = (int)(145 + factor * 70);
            int b = (int)(175 + factor * 80);
            col_c = IM_COL32(r, g, b, 200);
        }

        ImVec2 mid;
        if (ruteo_ortogonal) {
            float mid_x = 0.5f * (inicio.x + fin.x);
            ImVec2 pts[4] = { inicio, ImVec2(mid_x, inicio.y), ImVec2(mid_x, fin.y), fin };
            draw->AddPolyline(pts, 4, col_c, ImDrawFlags_None, ancho);
            mid = ImVec2(mid_x, 0.5f * (inicio.y + fin.y));

            ImVec2 dir(fin.x - pts[2].x, fin.y - pts[2].y);
            float len = std::sqrt(dir.x*dir.x + dir.y*dir.y);
            if (len > 0) { dir.x /= len; dir.y /= len; } else { dir = ImVec2(1, 0); }
            ImVec2 perp(-dir.y, dir.x);
            float fs = std::max(8.0f * zoom, ancho * 1.1f);
            draw->AddTriangleFilled(fin,
                ImVec2(fin.x-dir.x*fs+perp.x*fs*0.5f, fin.y-dir.y*fs+perp.y*fs*0.5f),
                ImVec2(fin.x-dir.x*fs-perp.x*fs*0.5f, fin.y-dir.y*fs-perp.y*fs*0.5f),
                col_c);
        } else {
            draw->AddBezierCubic(inicio, cp1, cp2, fin, col_c, ancho);
            float tf = 0.95f;
            ImVec2 fp((1-tf)*(1-tf)*(1-tf)*inicio.x + 3*(1-tf)*(1-tf)*tf*cp1.x + 3*(1-tf)*tf*tf*cp2.x + tf*tf*tf*fin.x,
                      (1-tf)*(1-tf)*(1-tf)*inicio.y + 3*(1-tf)*(1-tf)*tf*cp1.y + 3*(1-tf)*tf*tf*cp2.y + tf*tf*tf*fin.y);
            ImVec2 dir(fin.x-fp.x, fin.y-fp.y);
            float len = std::sqrt(dir.x*dir.x+dir.y*dir.y);
            if (len > 0) { dir.x/=len; dir.y/=len; }
            ImVec2 perp(-dir.y, dir.x);
            float fs = std::max(8.0f * zoom, ancho * 1.05f);
            draw->AddTriangleFilled(fin,
                ImVec2(fin.x-dir.x*fs+perp.x*fs*0.5f, fin.y-dir.y*fs+perp.y*fs*0.5f),
                ImVec2(fin.x-dir.x*fs-perp.x*fs*0.5f, fin.y-dir.y*fs-perp.y*fs*0.5f),
                col_c);
            float t = 0.5f;
            mid = ImVec2((1-t)*(1-t)*(1-t)*inicio.x + 3*(1-t)*(1-t)*t*cp1.x + 3*(1-t)*t*t*cp2.x + t*t*t*fin.x,
                         (1-t)*(1-t)*(1-t)*inicio.y + 3*(1-t)*(1-t)*t*cp1.y + 3*(1-t)*t*t*cp2.y + t*t*t*fin.y);
        }

        // Partículas animadas a lo largo del flujo
        if (flujo_total > 0.001f && !ruteo_ortogonal) {
            float tiempo = ImGui::GetTime();
            float vel = 0.3f + std::min(flujo_total * 0.08f, 1.2f);
            int n_particulas = 3 + std::min((int)(flujo_total * 2), 9);
            for (int p = 0; p < n_particulas; p++) {
                float t = fmod(tiempo * vel + (float)p / (float)n_particulas, 1.0f);
                ImVec2 pp((1-t)*(1-t)*(1-t)*inicio.x + 3*(1-t)*(1-t)*t*cp1.x + 3*(1-t)*t*t*cp2.x + t*t*t*fin.x,
                          (1-t)*(1-t)*(1-t)*inicio.y + 3*(1-t)*(1-t)*t*cp1.y + 3*(1-t)*t*t*cp2.y + t*t*t*fin.y);
                float alfa = (t < 0.1f) ? t * 10 : (t > 0.9f) ? (1 - t) * 10 : 1.0f;
                draw->AddCircleFilled(pp, 3*zoom, IM_COL32(230,250,255,(int)(110*alfa)));
            }
        }

        // Etiqueta de Corriente (HSC-Sim Style: Nombre + Valor/Unidad)
        {
            char etiq[128];
            std::string name = flujo.nombre.empty() ? ("S" + std::to_string(fid)) : flujo.nombre;
            if (modo_visualizacion_flujos == 0) {
                snprintf(etiq, sizeof(etiq), "%s", name.c_str());
            } else if (flujo_total > 1e-6f || modo_visualizacion_flujos == 4 || modo_visualizacion_flujos == 5) {
                if (sankey_activo) {
                    snprintf(etiq, sizeof(etiq), "%.2f %s", val_vis, unidad_str);
                } else {
                    snprintf(etiq, sizeof(etiq), "%s [%.2f %s]", name.c_str(), val_vis, unidad_str);
                }
            } else {
                snprintf(etiq, sizeof(etiq), "%s [0.00 %s]", name.c_str(), unidad_str);
            }

            ImVec2 etsz = ImGui::CalcTextSize(etiq);
            ImU32 bg_col = sankey_activo ? IM_COL32(255, 255, 225, 240) : IM_COL32(255, 255, 255, 240);
            ImU32 bd_col = (id_flujo_seleccionado == fid) ? IM_COL32(230, 150, 0, 255) : IM_COL32(150, 155, 165, 210);
            draw->AddRectFilled(ImVec2(mid.x-etsz.x*0.5f-5*zoom, mid.y-etsz.y*0.5f-2*zoom),
                                ImVec2(mid.x+etsz.x*0.5f+5*zoom, mid.y+etsz.y*0.5f+2*zoom),
                                bg_col, 3 * zoom);
            draw->AddRect(ImVec2(mid.x-etsz.x*0.5f-5*zoom, mid.y-etsz.y*0.5f-2*zoom),
                          ImVec2(mid.x+etsz.x*0.5f+5*zoom, mid.y+etsz.y*0.5f+2*zoom),
                          bd_col, 3 * zoom, 0, 1.0f * zoom);
            draw->AddText(ImVec2(mid.x-etsz.x*0.5f, mid.y-etsz.y*0.5f),
                          IM_COL32(25, 30, 40, 255), etiq);
        }

        // Hit test (sobre la curva o sobre el punto medio de etiqueta)
        bool hit = false;
        if (std::hypot(ImGui::GetIO().MousePos.x - mid.x, ImGui::GetIO().MousePos.y - mid.y) < 18.0f * zoom) {
            hit = true;
        } else {
            for (float tt = 0; tt <= 1.0f; tt += 0.025f) {
                ImVec2 pt((1-tt)*(1-tt)*(1-tt)*inicio.x + 3*(1-tt)*(1-tt)*tt*cp1.x + 3*(1-tt)*tt*tt*cp2.x + tt*tt*tt*fin.x,
                          (1-tt)*(1-tt)*(1-tt)*inicio.y + 3*(1-tt)*(1-tt)*tt*cp1.y + 3*(1-tt)*tt*tt*cp2.y + tt*tt*tt*fin.y);
                float d = std::hypot(ImGui::GetIO().MousePos.x - pt.x, ImGui::GetIO().MousePos.y - pt.y);
                if (d < std::max(7.0f, ancho * 0.6f)) { hit = true; break; }
            }
        }
        if (hit && clic_en_lienzo_) {
            id_flujo_seleccionado = fid;
            id_nodo_seleccionado = -1;
            clic_detectado_en_objeto_ = true;
            if (al_seleccionar_flujo) al_seleccionar_flujo(fid);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Conexión arrastrada
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_conexion_arrastrada(const ImVec2& pos) {
    ImDrawList* draw = ImGui::GetWindowDrawList();
    bool sobre = false;
    int64_t dest = -1;
    // Primero buscar si pasa sobre un puerto de entrada
    for (auto& nodo : flowsheet_->nodos()) {
        if (nodo.id == nodo_origen_) continue;
        auto it = puertos_entrada_.find(nodo.id);
        if (it == puertos_entrada_.end()) continue;
        for (auto& p : it->second) {
            float d = std::sqrt(std::pow(ImGui::GetIO().MousePos.x-p.x,2)+
                                std::pow(ImGui::GetIO().MousePos.y-p.y,2));
            if (d < 15*zoom) { sobre=true; actual_arrastre_=p; dest=nodo.id; break; }
        }
        if (sobre) break;
    }
    // Si no sobre un puerto, ver si pasa sobre el cuerpo de algún nodo (más amigable)
    if (!sobre) {
        for (auto& nodo : flowsheet_->nodos()) {
            if (nodo.id == nodo_origen_) continue;
            float z = zoom;
            ImVec2 npos(pos.x + nodo.canvas_x + desplazamiento.x,
                        pos.y + nodo.canvas_y + desplazamiento.y);
            ImVec2 nsz(NODO_ANCHO * z, NODO_ALTO * z);
            ImVec2 m = ImGui::GetIO().MousePos;
            if (hover_rect(m, npos, ImVec2(npos.x + nsz.x, npos.y + nsz.y))) {
                auto it = puertos_entrada_.find(nodo.id);
                if (it != puertos_entrada_.end() && !it->second.empty()) {
                    sobre = true;
                    actual_arrastre_ = it->second[0]; // Snap al primer puerto de entrada
                    dest = nodo.id;
                    break;
                }
            }
        }
    }
    if (!sobre) actual_arrastre_ = ImGui::GetIO().MousePos;

    float dx = (actual_arrastre_.x-inicio_arrastre_.x)*0.4f;
    ImVec2 cp1(inicio_arrastre_.x+dx, inicio_arrastre_.y);
    ImVec2 cp2(actual_arrastre_.x-dx, actual_arrastre_.y);

    ImU32 col = sobre ? IM_COL32(100,255,100,220) : IM_COL32(255,255,80,200);
    draw->AddBezierCubic(inicio_arrastre_, cp1, cp2, actual_arrastre_, col, 2.5f*zoom);

    float ta = fmod(ImGui::GetTime()*2.5f,1.0f);
    ImVec2 pulso((1-ta)*(1-ta)*(1-ta)*inicio_arrastre_.x + 3*(1-ta)*(1-ta)*ta*cp1.x + 3*(1-ta)*ta*ta*cp2.x + ta*ta*ta*actual_arrastre_.x,
                 (1-ta)*(1-ta)*(1-ta)*inicio_arrastre_.y + 3*(1-ta)*(1-ta)*ta*cp1.y + 3*(1-ta)*ta*ta*cp2.y + ta*ta*ta*actual_arrastre_.y);
    draw->AddCircleFilled(pulso, 4*zoom, IM_COL32(255,255,200,220));

    if (!ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
        if (sobre && dest > 0) {
            std::string motivo;
            if (flowsheet_->conexion_valida(nodo_origen_, dest, &motivo)) {
                guardar_estado();
                flowsheet_->conectar(nodo_origen_, dest);
            } else {
                mostrar_error_conexion(motivo);
            }
        }
        arrastrando_conexion_ = false;
        nodo_origen_ = -1;
        if (!persistir_herramienta) modo_dibujo = ModoDibujo::Seleccion;
    }
}

void Lienzo::mostrar_error_conexion(const std::string& msg) {
    error_conexion_ = msg;
    tiempo_error_conexion_ = 5.0f;
}

// ═══════════════════════════════════════════════════════════════════════════
// Minimapa
// ═══════════════════════════════════════════════════════════════════════════


}} // namespace
