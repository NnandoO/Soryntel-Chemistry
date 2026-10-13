#include "shared.hpp"
namespace cma { namespace interfaz {
void Lienzo::renderizar_flujos(const ImVec2&) {
    if (!flowsheet_) return;
    ImDrawList* draw = ImGui::GetWindowDrawList();

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
        ImVec2 cp1(inicio.x+dx, inicio.y), cp2(fin.x-dx, fin.y);

        // Flecha
        float tf = 0.95f;
        ImVec2 fp((1-tf)*(1-tf)*(1-tf)*inicio.x + 3*(1-tf)*(1-tf)*tf*cp1.x + 3*(1-tf)*tf*tf*cp2.x + tf*tf*tf*fin.x,
                  (1-tf)*(1-tf)*(1-tf)*inicio.y + 3*(1-tf)*(1-tf)*tf*cp1.y + 3*(1-tf)*tf*tf*cp2.y + tf*tf*tf*fin.y);
        ImVec2 dir(fin.x-fp.x, fin.y-fp.y);
        float len = std::sqrt(dir.x*dir.x+dir.y*dir.y);
        if (len > 0) { dir.x/=len; dir.y/=len; }
        ImVec2 perp(-dir.y, dir.x);
        float fs = 8*zoom;
        draw->AddTriangleFilled(fin,
            ImVec2(fin.x-dir.x*fs+perp.x*fs*0.5f, fin.y-dir.y*fs+perp.y*fs*0.5f),
            ImVec2(fin.x-dir.x*fs-perp.x*fs*0.5f, fin.y-dir.y*fs-perp.y*fs*0.5f),
            IM_COL32(170,170,190,200));

        float flujo_total = flujo.flujo_total();
        float base_ancho = 1.8f + std::min(flujo_total * 0.5f, 15.0f);
        float ancho = (id_flujo_seleccionado==fid) ? (base_ancho + 2.0f)*zoom : base_ancho*zoom;

        ImU32 col_c;
        if (id_flujo_seleccionado == fid) {
            col_c = IM_COL32(255, 210, 60, 240);
        } else {
            float factor = std::min(flujo_total / 20.0f, 1.0f);
            int r = (int)(140 - factor * 90);
            int g = (int)(145 + factor * 70);
            int b = (int)(165 + factor * 90);
            col_c = IM_COL32(r, g, b, 180);
        }
        draw->AddBezierCubic(inicio, cp1, cp2, fin, col_c, ancho);

        // Partículas animadas a lo largo del flujo
        if (flujo_total > 0.001f) {
            float tiempo = ImGui::GetTime();
            float vel = 0.3f + flujo_total * 0.1f;
            int n_particulas = 3 + (int)(flujo_total * 2);
            if (n_particulas > 12) n_particulas = 12;
            for (int p = 0; p < n_particulas; p++) {
                float t = fmod(tiempo * vel + (float)p / (float)n_particulas, 1.0f);
                ImVec2 pp((1-t)*(1-t)*(1-t)*inicio.x + 3*(1-t)*(1-t)*t*cp1.x + 3*(1-t)*t*t*cp2.x + t*t*t*fin.x,
                          (1-t)*(1-t)*(1-t)*inicio.y + 3*(1-t)*(1-t)*t*cp1.y + 3*(1-t)*t*t*cp2.y + t*t*t*fin.y);
                float alfa = (t < 0.1f) ? t * 10 : (t > 0.9f) ? (1 - t) * 10 : 1.0f;
                draw->AddCircleFilled(pp, 3*zoom, IM_COL32(100,200,255,(int)(80*alfa)));
            }
        }

        // Etiqueta de Corriente (HSC Style: Nombre [Value])
        {
            float t = 0.5f;
            ImVec2 mid((1-t)*(1-t)*(1-t)*inicio.x + 3*(1-t)*(1-t)*t*cp1.x + 3*(1-t)*t*t*cp2.x + t*t*t*fin.x,
                       (1-t)*(1-t)*(1-t)*inicio.y + 3*(1-t)*(1-t)*t*cp1.y + 3*(1-t)*t*t*cp2.y + t*t*t*fin.y);
            
            char etiq[96];
            std::string name = flujo.nombre.empty() ? ("Flujo " + std::to_string(fid)) : flujo.nombre;
            if (flujo_total > 0.0001f) {
                snprintf(etiq, sizeof(etiq), "%s [%.2f kmol/s]", name.c_str(), flujo_total);
            } else {
                snprintf(etiq, sizeof(etiq), "%s [Value]", name.c_str());
            }

            ImVec2 etsz = ImGui::CalcTextSize(etiq);
            // Draw background badge (white with grey outline, style HSC Sim)
            draw->AddRectFilled(ImVec2(mid.x-etsz.x*0.5f-5*zoom, mid.y-etsz.y*0.5f-3*zoom),
                                ImVec2(mid.x+etsz.x*0.5f+5*zoom, mid.y+etsz.y*0.5f+3*zoom),
                                IM_COL32(255, 255, 255, 240), 3 * zoom);
            draw->AddRect(ImVec2(mid.x-etsz.x*0.5f-5*zoom, mid.y-etsz.y*0.5f-3*zoom),
                          ImVec2(mid.x+etsz.x*0.5f+5*zoom, mid.y+etsz.y*0.5f+3*zoom),
                          IM_COL32(180, 180, 190, 200), 3 * zoom, 0, 1.0f * zoom);
            
            draw->AddText(ImVec2(mid.x-etsz.x*0.5f, mid.y-etsz.y*0.5f),
                          IM_COL32(40, 40, 50, 255), etiq);
        }

        // Hit test
        for (float tt = 0; tt <= 1; tt += 0.02f) {
            ImVec2 pt((1-tt)*(1-tt)*(1-tt)*inicio.x + 3*(1-tt)*(1-tt)*tt*cp1.x + 3*(1-tt)*tt*tt*cp2.x + tt*tt*tt*fin.x,
                      (1-tt)*(1-tt)*(1-tt)*inicio.y + 3*(1-tt)*(1-tt)*tt*cp1.y + 3*(1-tt)*tt*tt*cp2.y + tt*tt*tt*fin.y);
            float d = std::sqrt(std::pow(ImGui::GetIO().MousePos.x-pt.x,2)+
                                std::pow(ImGui::GetIO().MousePos.y-pt.y,2));
            if (d < 6 && clic_en_lienzo_) {
                id_flujo_seleccionado = fid;
                id_nodo_seleccionado = -1;
                clic_detectado_en_objeto_ = true;
                if (al_seleccionar_flujo) al_seleccionar_flujo(fid);
                break;
            }
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
