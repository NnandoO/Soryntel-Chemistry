#include "shared.hpp"
namespace cma { namespace interfaz {
void Lienzo::renderizar_nodo(nucleo::Nodo& nodo, const ImVec2& pos) {
    ImDrawList* draw = ImGui::GetWindowDrawList();
    float z = zoom;

    ImVec2 npos(pos.x + nodo.canvas_x + desplazamiento.x,
                pos.y + nodo.canvas_y + desplazamiento.y);
    ImVec2 nsz(NODO_ANCHO * z, NODO_ALTO * z);
    float cab_h = CABECERA_H * z;
    float r = RADIO * z;

    bool sel = (id_nodo_seleccionado == nodo.id);
    ImU32 col_borde = sel ? IM_COL32(255, 210, 60, 255) : ImGui::GetColorU32(ImGuiCol_Border, 0.7f);
    float bw = sel ? 3.0f * z : 1.5f * z;

    // ── Sombra, Cabecera, Cuerpo, Barra y Borde removidos a petición del usuario ──
    // Se conserva únicamente el dibujo (modelo) y el texto.
    if (sel) {
        draw->AddRect(npos, ImVec2(npos.x + nsz.x, npos.y + nsz.y), col_borde, r, 0, bw);
    }

    // ── Nombre del tipo en cabecera ──
    std::string tstr_s = sanitizar_texto(nucleo::nombre_tipo_nodo(nodo.tipo));
    const char* tstr = tstr_s.c_str();
    draw->AddText(ImVec2(npos.x + 6*z, npos.y + 4*z), ImGui::GetColorU32(ImGuiCol_Text), tstr);

    // ── ID pequeño en la cabecera ──
    char idbuf[16];
    snprintf(idbuf, sizeof(idbuf), "#%lld", (long long)nodo.id);
    ImVec2 idtsz = ImGui::CalcTextSize(idbuf);
    draw->AddText(ImVec2(npos.x + nsz.x - idtsz.x - 4*z, npos.y + 4*z),
                  ImGui::GetColorU32(ImGuiCol_Text, 0.7f), idbuf);

    // ── Modelo ──
    ImVec2 cm(npos.x + nsz.x*0.5f, npos.y + cab_h + nsz.y*0.5f);
    dibujar_modelo(draw, cm, nsz.y*0.85f, nodo.tipo);

    // ── Nombre del nodo ──
    std::string etiq = nodo.nombre.empty() ? "" : sanitizar_texto(nodo.nombre);
    if (!etiq.empty()) {
        ImU32 col_etiq = ImGui::GetColorU32(ImGuiCol_Text, 0.7f);
        ImVec2 etsz = ImGui::CalcTextSize(etiq.c_str());
        draw->AddText(ImVec2(npos.x + (nsz.x - etsz.x)*0.5f, npos.y + nsz.y - etsz.y - 3*z),
                      col_etiq, etiq.c_str());
    }

    // ── Puertos ──
    calcular_posicion_puertos(nodo, npos, nsz);
    for (auto& p : puertos_entrada_[nodo.id]) {
        draw->AddCircleFilled(p, RADIO_PTO*z, IM_COL32(80,180,255,255));
        draw->AddCircle(p, RADIO_PTO*z, IM_COL32(200,230,255,200), 0, 1.5f*z);
    }
    for (auto& p : puertos_salida_[nodo.id]) {
        draw->AddCircleFilled(p, RADIO_PTO*z, IM_COL32(255,170,80,255));
        draw->AddCircle(p, RADIO_PTO*z, IM_COL32(255,220,180,200), 0, 1.5f*z);
    }

    // ── Interacción ──
    ImVec2 mp = ImGui::GetIO().MousePos;
    bool hn = hover_rect(mp, npos, ImVec2(npos.x+nsz.x, npos.y+nsz.y));

    if (hn && clic_en_lienzo_) {
        id_nodo_seleccionado = nodo.id;
        id_flujo_seleccionado = -1;
        id_anotacion_seleccionada = -1;
        if (al_seleccionar_nodo) al_seleccionar_nodo(nodo.id);
        nodo_arrastrando_ = nodo.id;
        clic_detectado_en_objeto_ = true;
    }

    if (nodo_arrastrando_ == nodo.id) {
        if (ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
            if (ImGui::IsMouseDragging(ImGuiMouseButton_Left)) {
                extern bool g_nodo_arrastrado;
                if (!g_nodo_arrastrado) {
                    guardar_estado();
                    g_nodo_arrastrado = true;
                }
                if (modo_dibujo == ModoDibujo::DibujarFlujo) {
                    if (!arrastrando_conexion_ && !puertos_salida_[nodo.id].empty()) {
                        arrastrando_conexion_ = true;
                        nodo_origen_ = nodo.id;
                        inicio_arrastre_ = puertos_salida_[nodo.id][0];
                        actual_arrastre_ = ImGui::GetIO().MousePos;
                        nodo_arrastrando_ = -1;
                        clic_detectado_en_objeto_ = true;
                    }
                } else {
                    float dx = ImGui::GetIO().MouseDelta.x / zoom;
                    float dy = ImGui::GetIO().MouseDelta.y / zoom;
                    nodo.canvas_x += dx;
                    nodo.canvas_y += dy;
                }
            }
        } else {
            if (ajustar_grilla) {
                float g = GRILLA;
                nodo.canvas_x = round(nodo.canvas_x / g) * g;
                nodo.canvas_y = round(nodo.canvas_y / g) * g;
            }
            nodo_arrastrando_ = -1;
        }
    }

    // Conexión desde puerto
    if (hn && clic_en_lienzo_ && !arrastrando_conexion_) {
        for (auto& p : puertos_salida_[nodo.id]) {
            float dist = std::sqrt(std::pow(ImGui::GetIO().MousePos.x-p.x,2)+
                                   std::pow(ImGui::GetIO().MousePos.y-p.y,2));
            if (dist < 12*z) {
                arrastrando_conexion_ = true;
                nodo_origen_ = nodo.id;
                inicio_arrastre_ = p;
                actual_arrastre_ = ImGui::GetIO().MousePos;
                nodo_arrastrando_ = -1;
                clic_detectado_en_objeto_ = true;
                break;
            }
        }
    }

    // Botón X funcional
    if (hn) {
        ImVec2 bx(npos.x+nsz.x-18*z, npos.y+2*z);
        ImVec2 bt(16*z, 16*z);
        bool hover_x = hover_rect(mp, bx, ImVec2(bx.x+bt.x, bx.y+bt.y));
        ImU32 col_x = hover_x ? IM_COL32(255,80,80,255) : IM_COL32(200,50,50,200);
        draw->AddRectFilled(bx, ImVec2(bx.x+bt.x, bx.y+bt.y), col_x, 3*z);
        draw->AddText(ImVec2(bx.x+4*z, bx.y+1*z), IM_COL32(255,255,255,255), "x");
        if (hover_x && clic_en_lienzo_) {
            nodos_a_eliminar_.push_back(nodo.id);
            hn = false;
        }
    }

    // Indicador de estado (punto verde/amarillo)
    {
        ImVec2 centro_estado(npos.x + nsz.x - 8*z, npos.y + nsz.y - 8*z);
        bool tiene_entradas = !nodo.ids_flujos_entrada.empty();
        bool tiene_salidas = !nodo.ids_flujos_salida.empty();
        ImU32 col_estado;
        if (tiene_entradas && tiene_salidas)
            col_estado = IM_COL32(60, 200, 80, 255); // verde: conectado
        else if (tiene_entradas || tiene_salidas)
            col_estado = IM_COL32(230, 180, 40, 255); // amarillo: parcialmente conectado
        else
            col_estado = IM_COL32(200, 80, 80, 255); // rojo: sin conexiones
        draw->AddCircleFilled(centro_estado, 4*z, col_estado);
    }

    // Tooltip
    if (hn && ImGui::IsItemHovered()) {
        ImGui::BeginTooltip();
        ImGui::Text("%s", nucleo::nombre_tipo_nodo(nodo.tipo));
        ImGui::Text("ID: %lld", (long long)nodo.id);
        if (!nodo.nombre.empty())
            ImGui::Text("Nombre: %s", nodo.nombre.c_str());
        ImGui::Text("Entradas: %d  Salidas: %d", nodo.num_entradas(), nodo.num_salidas());

        // Descripción del tipo de equipo
        const char* desc = descripcion_tipo_nodo(nodo.tipo);
        if (desc) {
            ImGui::Separator();
            ImGui::TextDisabled("%s", desc);
        }
        ImGui::EndTooltip();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Puertos
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::calcular_posicion_puertos(const nucleo::Nodo& nodo, const ImVec2& npos, const ImVec2& nsz) {
    auto& in = puertos_entrada_[nodo.id];
    auto& out = puertos_salida_[nodo.id];
    in.clear(); out.clear();

    int n_in = nodo.num_entradas();
    int n_out = nodo.num_salidas();

    // Default minimum ports for empty nodes
    int min_in = 1;
    int min_out = 1;

    if (nodo.tipo == nucleo::TipoNodo::Fuente) {
        min_in = 0;
        n_in = 0;
    }
    if (nodo.tipo == nucleo::TipoNodo::Sumidero) {
        min_out = 0;
        n_out = 0;
    }
    if (nodo.tipo == nucleo::TipoNodo::Mezclador) {
        min_in = 3; // Mixer starts with at least 3 input slots
    }
    if (nodo.tipo == nucleo::TipoNodo::Divisor) {
        min_out = 2; // Splitter starts with at least 2 output slots
    }

    // Always keep one extra free port slot for new connections
    n_in = std::max(n_in + 1, min_in);
    if (nodo.tipo == nucleo::TipoNodo::Fuente) n_in = 0;

    n_out = std::max(n_out + 1, min_out);
    if (nodo.tipo == nucleo::TipoNodo::Sumidero) n_out = 0;

    float cab_h = 24.0f * zoom;
    float center_y = npos.y + cab_h + nsz.y * 0.5f;
    float s_y = nsz.y * 0.85f;
    float start_y = center_y - s_y * 0.5f;
    for (int i = 0; i < n_in; ++i) {
        float y = start_y + (s_y * (i + 1)) / (n_in + 1);
        in.push_back(ImVec2(npos.x, y));
    }
    for (int i = 0; i < n_out; ++i) {
        float y = start_y + (s_y * (i + 1)) / (n_out + 1);
        out.push_back(ImVec2(npos.x + nsz.x, y));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Flujos
// ═══════════════════════════════════════════════════════════════════════════


}} // namespace
