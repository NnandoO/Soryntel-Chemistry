#include "shared.hpp"
#include "base_datos/gestor_db.hpp"
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
    const auto& db = base_datos::GestorDB::instancia();
    const auto& cfg = flowsheet_->config_tabla_flujos;

    for (auto& tabla : flowsheet_->tablas_flujo()) {
        if (tabla.pagina != pagina_actual || !tabla.visible) continue;

        std::vector<const nucleo::Flujo*> cols;
        for (const auto& [fid, f] : flowsheet_->mapa_flujos()) {
            if (f.es_energia) continue;
            if (!tabla.ids_flujos_mostrados.empty() &&
                std::find(tabla.ids_flujos_mostrados.begin(), tabla.ids_flujos_mostrados.end(), fid)
                == tabla.ids_flujos_mostrados.end()) continue;
            cols.push_back(&f);
        }

        const auto& vars = cfg.variables_seleccionadas;
        const float col_prop_w = (cfg.modo_vista == 0) ? 130.0f : 0.0f;
        const float col_unit_w = (cfg.modo_vista <= 1) ? 54.0f : 0.0f;
        const float col_str_w  = 92.0f;
        tabla.w = std::max(260.0f, col_prop_w + col_unit_w + col_str_w * static_cast<float>(std::max<size_t>(1, cols.size())) + 12.0f);
        tabla.h = std::max(72.0f, (cfg.mostrar_encabezado ? 44.0f : 24.0f) + 18.0f * static_cast<float>(vars.size()) + 8.0f);

        float tx = pos.x + tabla.x * zoom + desplazamiento.x;
        float ty = pos.y + tabla.y * zoom + desplazamiento.y;
        float tw = tabla.w * zoom;
        float th = tabla.h * zoom;

        draw->AddRectFilled(ImVec2(tx, ty), ImVec2(tx + tw, ty + th), IM_COL32(246, 248, 252, 245), 4.0f * zoom);
        draw->AddRect(ImVec2(tx, ty), ImVec2(tx + tw, ty + th), IM_COL32(70, 95, 135, 230), 4.0f * zoom, 0, 1.5f * zoom);
        draw->AddRectFilled(ImVec2(tx, ty), ImVec2(tx + tw, ty + 20.0f * zoom), IM_COL32(36, 88, 156, 245), 4.0f * zoom);
        draw->AddText(ImVec2(tx + 8.0f * zoom, ty + 3.0f * zoom), IM_COL32(255, 255, 255, 255), cfg.nombre_set.c_str());

        float yh = ty + 22.0f * zoom;
        if (cfg.mostrar_encabezado) {
            float cx = tx + 6.0f * zoom;
            if (col_prop_w > 0.0f) { draw->AddText(ImVec2(cx, yh), IM_COL32(30, 50, 85, 255), "Variable"); cx += col_prop_w * zoom; }
            if (col_unit_w > 0.0f) { draw->AddText(ImVec2(cx, yh), IM_COL32(80, 95, 120, 255), "Unit"); cx += col_unit_w * zoom; }
            for (const auto* f : cols) {
                draw->AddText(ImVec2(cx, yh), IM_COL32(20, 70, 140, 255), f->nombre.c_str());
                cx += col_str_w * zoom;
            }
            yh += 18.0f * zoom;
            draw->AddLine(ImVec2(tx + 2.0f * zoom, yh - 2.0f * zoom), ImVec2(tx + tw - 2.0f * zoom, yh - 2.0f * zoom), IM_COL32(160, 175, 200, 200));
        }

        auto eval_var = [&](const std::string& k, const nucleo::Flujo& f, const char*& u_out) -> double {
            if (k == "Total Solids Flow")    { u_out = "t/h";    return f.masa_solidos_t_h(db); }
            if (k == "Total Gas Flow")       { u_out = "t/h";    return f.masa_gas_t_h(db); }
            if (k == "Total Liquid Flow")    { u_out = "t/h";    return f.masa_liquido_t_h(db); }
            if (k == "Pulp Mass Flow")       { u_out = "t/h";    return f.masa_solidos_t_h(db) + f.masa_liquido_t_h(db); }
            if (k == "Pulp Volumetric Flow") { u_out = "m3/h";   return f.volumen_total_m3_h(db); }
            if (k == "Pulp SG")              { u_out = "g/cm3";  return f.densidad_pulpa_g_cm3(db); }
            if (k == "% Solids")             { u_out = "wt-%";   return f.porcentaje_solidos_wt(db); }
            if (k == "Temperature")          { u_out = "°C";     return f.temperatura - 273.15; }
            if (k == "Pressure")             { u_out = "bar";    return f.presion; }
            if (k == "Total Mass Flow")      { u_out = "t/h";    return f.flujo_masico_total(db) * 3.6; }
            if (k == "Gas Volume Flow")      { u_out = "Nm3/h";  return f.volumen_gas_Nm3_h(db); }
            if (k == "Enthalpy Flow" || k == "Thermal Energy Flow") { u_out = "kW"; return f.entalpia; }
            if (k == "Heat Content Flow")    { u_out = "kW";     return f.calor_contenido_Hc_kW(db); }
            if (k == "Exergy Flow")          { u_out = "kW";     return f.exergia_flujo_kW(db); }
            if (k == "Heat Capacity")        { u_out = "kJ/kgK"; return f.cp_especifico_kJ_kgK(db); }
            u_out = "-"; return 0.0;
        };

        for (size_t r = 0; r < vars.size(); ++r) {
            float ry = yh + static_cast<float>(r) * 18.0f * zoom;
            if (ry + 16.0f * zoom > ty + th) break;
            if (r % 2 == 1) {
                draw->AddRectFilled(ImVec2(tx + 2.0f * zoom, ry), ImVec2(tx + tw - 2.0f * zoom, ry + 17.0f * zoom), IM_COL32(228, 234, 244, 160));
            }
            const char* u_str = "";
            if (!cols.empty()) eval_var(vars[r], *cols[0], u_str);
            float cx = tx + 6.0f * zoom;
            if (col_prop_w > 0.0f) {
                draw->AddText(ImVec2(cx, ry + 1.0f * zoom), IM_COL32(25, 35, 50, 255), vars[r].c_str());
                cx += col_prop_w * zoom;
            }
            if (col_unit_w > 0.0f) {
                draw->AddText(ImVec2(cx, ry + 1.0f * zoom), IM_COL32(95, 110, 135, 255), u_str);
                cx += col_unit_w * zoom;
            }
            for (const auto* f : cols) {
                const char* u_dummy = "";
                double v = eval_var(vars[r], *f, u_dummy);
                char buf[48];
                snprintf(buf, sizeof(buf), "%.2f", v);
                draw->AddText(ImVec2(cx, ry + 1.0f * zoom), IM_COL32(15, 25, 40, 255), buf);
                cx += col_str_w * zoom;
            }
        }
    }
}


}} // namespace
