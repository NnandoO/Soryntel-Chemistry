#include "shared.hpp"
#include "panel_lca.hpp"
#include "interfaz/ui_css_style.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <map>
#include <set>
#include <sstream>

namespace cma { namespace interfaz {

namespace {

const char* kNoMapeado = "no-mapeado";

ImVec4 color_ok()   { return AdaptiveColor(0.30f, 0.85f, 0.40f, 1.0f); }
ImVec4 color_malo() { return AdaptiveColor(0.90f, 0.40f, 0.30f, 1.0f); }
ImVec4 color_aviso(){ return AdaptiveColor(0.95f, 0.75f, 0.25f, 1.0f); }

std::string minusculas(std::string s) {
    for (char& c : s) {
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c - 'A' + 'a');
    }
    return s;
}

bool coincide(const std::string& texto, const std::string& consulta_min) {
    if (consulta_min.empty()) return true;
    return minusculas(texto).find(consulta_min) != std::string::npos;
}

const char* nombre_equivalente(const motores::LCA& motor, const std::string& id) {
    const auto* e = motor.buscar_equivalente(id);
    return e != nullptr ? e->nombre.c_str() : "Sin mapear";
}

const char* grupo_equivalente(const motores::LCA& motor, const std::string& id) {
    const auto* e = motor.buscar_equivalente(id);
    return e != nullptr ? e->grupo.c_str() : "Not defined";
}

} // namespace

// ─────────────────────────────────────────────────────────────────────────────
void PanelLCA::reset() {
    filas_.clear();
    fila_sel_ = -1;
    equivalente_sel_.clear();
    grupo_sel_.clear();
    unidad_funcional_kg_ = 0.0;
    resultado_ = motores::ResultadoLCA{};
    fs_actual_ = nullptr;
    calculado_ = false;
    cargado_ = false;
    mostrar_validacion_ = false;
    mostrar_ayuda_ = false;
    export_pendiente_ = 0;
    mensaje_estado_.clear();
    busqueda_[0] = '\0';
    busqueda_catalogo_[0] = '\0';
}

// ─────────────────────────────────────────────────────────────────────────────
// Extracción del dataset de frontera (una fila por especie)
// ─────────────────────────────────────────────────────────────────────────────
void PanelLCA::cargar_flujos(diagrama_flujo::Flowsheet* flowsheet) {
    filas_.clear();
    fila_sel_ = -1;
    if (flowsheet == nullptr) {
        fs_actual_ = nullptr;
        cargado_ = false;
        calcular();
        return;
    }

    auto& gestor = base_datos::GestorDB::instancia();

    for (auto& [fid, flujo] : flowsheet->mapa_flujos()) {
        if (flujo.es_energia) continue;  // la energía se declara en Manuales

        bool tiene_origen = false;
        bool origen_fuente = false;
        bool tiene_destino = false;
        bool destino_sumidero = false;
        std::string nombre_origen;
        std::string nombre_destino;

        for (const auto& nodo : flowsheet->nodos()) {
            for (int64_t sid : nodo.ids_flujos_salida) {
                if (sid != fid) continue;
                tiene_origen = true;
                nombre_origen = nodo.nombre;
                if (nodo.tipo == nucleo::TipoNodo::Fuente) origen_fuente = true;
            }
            for (int64_t sid : nodo.ids_flujos_entrada) {
                if (sid != fid) continue;
                tiene_destino = true;
                nombre_destino = nodo.nombre;
                if (nodo.tipo == nucleo::TipoNodo::Sumidero) destino_sumidero = true;
            }
        }

        // Frontera del sistema: solo entradas desde fuentes/sin origen y
        // salidas hacia sumideros/sin destino. Las corrientes internas
        // (proceso→proceso) no forman parte del inventario.
        const bool entrada = !tiene_origen || origen_fuente;
        const bool salida = (!tiene_destino || destino_sumidero) && !entrada;
        if (!entrada && !salida) continue;

        const std::string equipo = entrada
            ? (tiene_destino ? nombre_destino : nombre_origen)
            : (tiene_origen ? nombre_origen : nombre_destino);
        const std::string corriente = flujo.nombre.empty()
            ? ("Corriente " + std::to_string(fid))
            : flujo.nombre;

        struct Componente {
            std::string etiqueta;
            double kg_h = 0.0;
        };
        std::vector<Componente> componentes;
        double total_kg_h = 0.0;

        for (const auto& [sid, n_moles] : flujo.flujo_molar) {
            if (!std::isfinite(n_moles) || n_moles <= 0.0) continue;
            const auto* esp = gestor.especie_por_id(sid);
            const std::string etiqueta =
                esp != nullptr ? esp->formula : ("Especie #" + std::to_string(sid));
            const double masa_molar =
                (esp != nullptr && esp->masa_molar > 0.0) ? esp->masa_molar : 18.015;
            const double kg_h = n_moles * 3600.0 * masa_molar;
            componentes.push_back({etiqueta, kg_h});
            total_kg_h += kg_h;
        }
        if (flujo.agua_kg_h > 0.0) {
            componentes.push_back({"H2O(l)", flujo.agua_kg_h});
            total_kg_h += flujo.agua_kg_h;
        }
        for (const auto& p : flujo.particulas) {
            if (!std::isfinite(p.mass_kg_h) || p.mass_kg_h <= 0.0) continue;
            componentes.push_back({p.mineral.empty() ? "Sólido" : p.mineral, p.mass_kg_h});
            total_kg_h += p.mass_kg_h;
        }
        if (componentes.empty() || total_kg_h <= 0.0) continue;

        for (const auto& comp : componentes) {
            FilaUI fila;
            fila.stream_id = fid;
            fila.equipo = equipo;
            fila.corriente = corriente;
            fila.especie = comp.etiqueta;
            fila.kg_h = comp.kg_h;
            fila.fraccion = comp.kg_h / total_kg_h * 100.0;
            fila.entrada = entrada;

            const auto it = flujo.lca_especies.find(comp.etiqueta);
            if (it != flujo.lca_especies.end() && it->second == kNoMapeado) {
                fila.equivalente = kNoMapeado;
            } else if (it != flujo.lca_especies.end() &&
                       motor_.buscar_equivalente(it->second) != nullptr) {
                fila.equivalente = it->second;
            } else if (componentes.size() == 1 && !flujo.lca_equivalent.empty() &&
                       flujo.lca_equivalent != "No Mapping" &&
                       motor_.buscar_equivalente(flujo.lca_equivalent) != nullptr) {
                // Proyectos antiguos: mapeo por corriente sobre una sola especie.
                fila.equivalente = flujo.lca_equivalent;
            } else {
                fila.equivalente = motor_.sugerir_equivalente(comp.etiqueta, corriente, entrada);
            }
            filas_.push_back(std::move(fila));
        }
    }

    fs_actual_ = flowsheet;
    cargado_ = true;
    calcular();
}

void PanelLCA::calcular() {
    std::vector<motores::FilaLCA> entradas;
    entradas.reserve(filas_.size());
    for (const auto& f : filas_) {
        motores::FilaLCA m;
        m.equipo = f.equipo;
        m.corriente = f.corriente;
        m.especie = f.especie;
        m.cantidad = f.kg_h;
        m.entrada = f.entrada;
        m.equivalente = f.equivalente;
        entradas.push_back(std::move(m));
    }
    resultado_ = motor_.evaluar(entradas, manual_electricidad_, manual_gas_,
                                manual_combustible_, manual_agua_, manual_residuo_,
                                unidad_funcional_kg_);
    calculado_ = true;
}

void PanelLCA::auto_mapear() {
    for (auto& f : filas_) {
        if (f.mapeada()) continue;
        f.equivalente = motor_.sugerir_equivalente(f.especie, f.corriente, f.entrada);
    }
    calcular();
    mensaje_estado_ = "Auto-mapeo aplicado.";
}

void PanelLCA::aplicar_mapeo() {
    if (fila_sel_ < 0 || fila_sel_ >= static_cast<int>(filas_.size())) return;
    if (equivalente_sel_.empty() || motor_.buscar_equivalente(equivalente_sel_) == nullptr) return;

    auto& fila = filas_[static_cast<size_t>(fila_sel_)];
    fila.equivalente = equivalente_sel_;

    if (aplicar_a_especie_) {
        for (auto& f : filas_) {
            if (f.especie == fila.especie) f.equivalente = equivalente_sel_;
        }
    }

    calcular();
    mensaje_estado_ = "Mapeo aplicado. Usa Guardar mapeos para persistirlo.";
}

void PanelLCA::guardar_mapeos(diagrama_flujo::Flowsheet* flowsheet) {
    if (flowsheet == nullptr) return;

    std::map<int64_t, std::map<std::string, std::string>> por_flujo;
    for (const auto& f : filas_) {
        if (f.stream_id >= 0) por_flujo[f.stream_id][f.especie] = f.equivalente;
    }

    for (const auto& [fid, mapa] : por_flujo) {
        auto* flujo = flowsheet->flujo_por_id(fid);
        if (flujo == nullptr) continue;
        flujo->lca_especies = mapa;

        std::set<std::string> distintos;
        for (const auto& [especie, eq] : mapa) {
            (void)especie;
            distintos.insert(eq);
        }
        if (distintos.size() == 1) {
            const auto* eq = motor_.buscar_equivalente(*distintos.begin());
            flujo->lca_equivalent = eq != nullptr ? eq->nombre : "No Mapping";
            flujo->lca_group = eq != nullptr ? eq->grupo : "Not defined";
        } else {
            flujo->lca_equivalent = "Mezcla de especies";
            flujo->lca_group = "Not defined";
        }
    }
    mensaje_estado_ = "Mapeos guardados en el flowsheet.";
}

// ─────────────────────────────────────────────────────────────────────────────
// Interfaz
// ─────────────────────────────────────────────────────────────────────────────
void PanelLCA::dibujar_toolbar(diagrama_flujo::Flowsheet* flowsheet) {
    auto solicitar_export = [&](int tipo) {
        if (resultado_.filas_sin_mapear > 0 || !resultado_.balance_masa_ok) {
            export_pendiente_ = tipo;
            mostrar_validacion_ = true;
        } else if (tipo == 1) {
            exportar_gabi(false);
        } else if (tipo == 2) {
            exportar_gabi(true);
        } else {
            exportar_json();
        }
    };

    css::begin_ribbon_bar("##LCARibbonToolbar", 78.0f);
    const float bar_bottom_y = ImGui::GetWindowPos().y + ImGui::GetWindowHeight();

    // ── GRUPO 1: Model ──
    float g1_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("lca_save", "Save", "", css::RibbonIcon::Save, 52.0f, 54.0f)) {
        guardar_mapeos(flowsheet);
    }
    float g1_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g1_start, g1_end, bar_bottom_y, "Model");

    ImGui::SameLine(0.0f, 10.0f);

    // ── GRUPO 2: Streams ──
    float g2_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("lca_in", "Input", "", css::RibbonIcon::InputArrow, 50.0f, 54.0f)) {
        cargar_flujos(flowsheet);
        pestana_forzada_ = 0;
        mensaje_estado_ = "Input streams reloaded.";
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("lca_out", "Output", "", css::RibbonIcon::OutputArrow, 50.0f, 54.0f)) {
        cargar_flujos(flowsheet);
        pestana_forzada_ = 1;
        mensaje_estado_ = "Output streams reloaded.";
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("lca_min", "Manual", "Input", css::RibbonIcon::ManualInput, 54.0f, 54.0f)) {
        pestana_forzada_ = 2;
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("lca_mout", "Manual", "Output", css::RibbonIcon::ManualOutput, 54.0f, 54.0f)) {
        pestana_forzada_ = 3;
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("lca_ind", "Indicator", "", css::RibbonIcon::Indicator, 58.0f, 54.0f)) {
        pestana_forzada_ = 4;
    }
    float g2_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g2_start, g2_end, bar_bottom_y, "Streams");

    ImGui::SameLine(0.0f, 10.0f);

    // ── GRUPO 3: Tools ──
    float g3_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("lca_map", "Mapping", "", css::RibbonIcon::Mapping, 58.0f, 54.0f)) {
        auto_mapear();
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("lca_norm", "Normalize", "", css::RibbonIcon::Normalize, 62.0f, 54.0f)) {
        if (fila_sel_ >= 0 && fila_sel_ < static_cast<int>(filas_.size()) &&
            filas_[static_cast<size_t>(fila_sel_)].kg_h > 0.0) {
            unidad_funcional_kg_ = filas_[static_cast<size_t>(fila_sel_)].kg_h;
        } else if (unidad_funcional_kg_ <= 0.0) {
            unidad_funcional_kg_ = 1000.0;
        }
        calcular();
        mensaje_estado_ = "Normalized to Functional Unit.";
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("lca_help", "Help", "", css::RibbonIcon::Help, 48.0f, 54.0f)) {
        mostrar_ayuda_ = !mostrar_ayuda_;
    }
    float g3_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g3_start, g3_end, bar_bottom_y, "Tools");

    ImGui::SameLine(0.0f, 10.0f);

    // ── GRUPO 4: Export ──
    float g4_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("lca_gabi", "To GaBi", "", css::RibbonIcon::ToGaBi, 56.0f, 54.0f)) {
        solicitar_export(1);
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("lca_excel", "To Excel", "", css::RibbonIcon::ToExcel, 56.0f, 54.0f)) {
        solicitar_export(2);
    }
    float g4_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g4_start, g4_end, bar_bottom_y, "Export");

    ImGui::SameLine(0.0f, 14.0f);
    ImGui::BeginGroup();
    if (resultado_.balance_masa_ok)
        ImGui::TextColored(ImVec4(0.16f, 0.54f, 0.22f, 1.0f), "Mass Balance: OK");
    else
        ImGui::TextColored(ImVec4(0.76f, 0.22f, 0.18f, 1.0f), "Mass Balance: dM = %.4g kg/h",
                           resultado_.masa_entrada_kg - resultado_.masa_salida_kg);
    ImGui::TextDisabled("%d rows | %d mapped | %d unmapped",
                        resultado_.filas_totales, resultado_.filas_mapeadas, resultado_.filas_sin_mapear);
    if (!mensaje_estado_.empty()) {
        ImGui::TextColored(ImVec4(0.18f, 0.45f, 0.72f, 1.0f), "%s", mensaje_estado_.c_str());
    }
    ImGui::EndGroup();

    css::end_ribbon_bar();
}

void PanelLCA::dibujar_tabla(bool entradas) {
    const std::string consulta = minusculas(busqueda_);

    // Franja superior de título estilo HSC LCA Evaluation ("Input Streams" / "Output Streams")
    const auto tk = css::tokens();
    ImGui::PushStyleColor(ImGuiCol_ChildBg, tk.header_bg);
    ImGui::BeginChild(entradas ? "##HdrInStreams" : "##HdrOutStreams", ImVec2(0, 24), true, ImGuiWindowFlags_NoScrollbar);
    const char* titulo_banda = entradas ? "Input Streams" : "Output Streams";
    float avail_w = ImGui::GetContentRegionAvail().x;
    float tw = ImGui::CalcTextSize(titulo_banda).x;
    if (avail_w > tw) ImGui::SetCursorPosX(ImGui::GetCursorPosX() + (avail_w - tw) * 0.5f);
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.text_primary), "%s", titulo_banda);
    ImGui::EndChild();
    ImGui::PopStyleColor();

    if (ImGui::BeginTable(entradas ? "lca_tabla_in" : "lca_tabla_out", 7,
                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                          ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable,
                          ImVec2(0, -56.0f))) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Unit Name", ImGuiTableColumnFlags_WidthFixed, 115.0f);
        ImGui::TableSetupColumn("Stream Name", ImGuiTableColumnFlags_WidthFixed, 125.0f);
        ImGui::TableSetupColumn("Species", ImGuiTableColumnFlags_WidthFixed, 82.0f);
        ImGui::TableSetupColumn("Amount", ImGuiTableColumnFlags_WidthFixed, 76.0f);
        ImGui::TableSetupColumn("Unit", ImGuiTableColumnFlags_WidthFixed, 42.0f);
        ImGui::TableSetupColumn("LCA Equivalent", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("LCA Group", ImGuiTableColumnFlags_WidthFixed, 130.0f);
        ImGui::TableHeadersRow();

        double total_kg_h = 0.0;
        double mapeado_kg_h = 0.0;
        for (int idx = 0; idx < static_cast<int>(filas_.size()); ++idx) {
            const FilaUI& f = filas_[static_cast<size_t>(idx)];
            if (f.entrada != entradas) continue;
            if (!consulta.empty() &&
                !coincide(f.equipo, consulta) && !coincide(f.corriente, consulta) &&
                !coincide(f.especie, consulta) && !coincide(f.equivalente, consulta)) {
                continue;
            }

            total_kg_h += f.kg_h;
            if (f.mapeada()) mapeado_kg_h += f.kg_h;

            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::PushID(idx);
            const bool seleccionada = (fila_sel_ == idx);
            std::string unit_lbl = (seleccionada ? "* " : "  ") + f.equipo;
            if (ImGui::Selectable(unit_lbl.c_str(), seleccionada,
                                  ImGuiSelectableFlags_SpanAllColumns |
                                  ImGuiSelectableFlags_AllowOverlap)) {
                fila_sel_ = idx;
            }
            ImGui::PopID();

            ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(f.corriente.c_str());
            ImGui::TableSetColumnIndex(2); ImGui::TextUnformatted(f.especie.c_str());
            ImGui::TableSetColumnIndex(3); ImGui::Text("%.4g", f.kg_h);
            ImGui::TableSetColumnIndex(4); ImGui::TextUnformatted("kg");
            ImGui::TableSetColumnIndex(5);
            if (f.mapeada()) {
                ImGui::TextColored(ImVec4(0.14f, 0.46f, 0.22f, 1.0f), "%s", nombre_equivalente(motor_, f.equivalente));
            } else {
                ImGui::TextColored(ImVec4(0.45f, 0.48f, 0.52f, 1.0f), "No Mapping");
            }
            ImGui::TableSetColumnIndex(6);
            ImGui::TextUnformatted(grupo_equivalente(motor_, f.equivalente));
        }
        ImGui::EndTable();

        ImGui::TextDisabled("Total: %.5g kg/h · Mapped: %.5g kg/h · Unmapped: %.5g kg/h",
                            total_kg_h, mapeado_kg_h, total_kg_h - mapeado_kg_h);
    }

    const bool puede_aplicar =
        fila_sel_ >= 0 && fila_sel_ < static_cast<int>(filas_.size()) &&
        !equivalente_sel_.empty();
    ImGui::BeginDisabled(!puede_aplicar);
    if (ImGui::Button("Apply Mapping", ImVec2(-1.0f, 26.0f))) {
        aplicar_mapeo();
    }
    ImGui::EndDisabled();
}

void PanelLCA::dibujar_manuales() {
    ImGui::TextColored(AdaptiveColor(0.6f, 0.75f, 0.95f, 1.0f),
                       tr("Consumos auxiliares y unidad funcional"));
    ImGui::TextWrapped(tr("Los consumos manuales están en base horaria y se normalizan con la misma unidad funcional que los flujos."));
    ImGui::Separator();

    ImGui::SetNextItemWidth(240.0f);
    bool cambio = false;
    cambio |= ImGui::InputDouble(tr("Electricidad (kWh/h)"), &manual_electricidad_, 1.0, 10.0, "%.4f");
    if (manual_electricidad_ < 0.0) manual_electricidad_ = 0.0;
    ImGui::SetNextItemWidth(240.0f);
    cambio |= ImGui::InputDouble(tr("Gas natural (Nm³/h)"), &manual_gas_, 1.0, 10.0, "%.4f");
    if (manual_gas_ < 0.0) manual_gas_ = 0.0;
    ImGui::SetNextItemWidth(240.0f);
    cambio |= ImGui::InputDouble(tr("Combustible diésel (kg/h)"), &manual_combustible_, 1.0, 10.0, "%.4f");
    if (manual_combustible_ < 0.0) manual_combustible_ = 0.0;
    ImGui::SetNextItemWidth(240.0f);
    cambio |= ImGui::InputDouble(tr("Agua de proceso (m³/h)"), &manual_agua_, 0.5, 5.0, "%.4f");
    if (manual_agua_ < 0.0) manual_agua_ = 0.0;
    ImGui::SetNextItemWidth(240.0f);
    cambio |= ImGui::InputDouble(tr("Residuo declarado (kg/h)"), &manual_residuo_, 0.5, 5.0, "%.4f");
    if (manual_residuo_ < 0.0) manual_residuo_ = 0.0;

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::TextColored(AdaptiveColor(0.6f, 0.75f, 0.95f, 1.0f), tr("Unidad funcional"));
    ImGui::SetNextItemWidth(240.0f);
    cambio |= ImGui::InputDouble(tr("Producto (kg/h)"), &unidad_funcional_kg_, 1.0, 10.0, "%.4f");
    if (unidad_funcional_kg_ < 0.0) unidad_funcional_kg_ = 0.0;

    if (ImGui::Button(tr("Usar corriente seleccionada")) && fila_sel_ >= 0 &&
        fila_sel_ < static_cast<int>(filas_.size())) {
        const double kg_h = filas_[static_cast<size_t>(fila_sel_)].kg_h;
        if (kg_h > 0.0) {
            unidad_funcional_kg_ = kg_h;
            cambio = true;
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Quitar UF"))) {
        unidad_funcional_kg_ = 0.0;
        cambio = true;
    }

    if (cambio) calcular();
    if (unidad_funcional_kg_ > 0.0) {
        ImGui::TextDisabled(tr("Indicadores expresados por 1 kg de producto (base %.5g kg/h)."),
                            unidad_funcional_kg_);
    } else {
        ImGui::TextColored(color_aviso(), tr("Sin unidad funcional: indicadores por hora."));
    }
}

void PanelLCA::dibujar_indicadores() {
    if (!calculado_) calcular();

    const char* sufijo = resultado_.factor_unidad_funcional != 1.0 ? " / kg producto" : "";
    ImGui::TextColored(AdaptiveColor(0.2f, 0.8f, 0.6f, 1.0f),
                       tr("GWP100: %.4g kg CO2-eq%s"), resultado_.gwp_kg_co2eq, sufijo);
    ImGui::TextColored(AdaptiveColor(0.9f, 0.6f, 0.2f, 1.0f),
                       tr("Acidificación: %.4g kg SO2-eq%s"), resultado_.ap_kg_so2eq, sufijo);
    ImGui::TextColored(AdaptiveColor(0.8f, 0.5f, 0.8f, 1.0f),
                       tr("Eutrofización: %.4g kg PO4-eq%s"), resultado_.ep_kg_po4eq, sufijo);
    ImGui::TextColored(AdaptiveColor(0.4f, 0.7f, 1.0f, 1.0f),
                       tr("Agua: %.4g m³%s"), resultado_.agua_m3, sufijo);
    ImGui::TextColored(AdaptiveColor(0.9f, 0.8f, 0.3f, 1.0f),
                       tr("Energía primaria: %.4g MJ%s"), resultado_.energia_mj, sufijo);
    ImGui::TextColored(AdaptiveColor(0.7f, 0.7f, 0.7f, 1.0f),
                       tr("Residuos: %.4g kg%s"), resultado_.residuos_kg, sufijo);

    ImGui::Spacing();
    ImGui::Separator();
    if (resultado_.contribuciones.empty()) {
        ImGui::TextDisabled(tr("Sin contribuciones: mapea flujos para calcular indicadores."));
        return;
    }

    std::vector<double> valores;
    std::vector<double> posiciones;
    std::vector<const char*> etiquetas;
    valores.reserve(resultado_.contribuciones.size());
    posiciones.reserve(resultado_.contribuciones.size());
    etiquetas.reserve(resultado_.contribuciones.size());

    const size_t maximo = std::min<size_t>(resultado_.contribuciones.size(), 8);
    for (size_t i = 0; i < maximo; ++i) {
        valores.push_back(resultado_.contribuciones[i].second);
        posiciones.push_back(static_cast<double>(i));
        etiquetas.push_back(resultado_.contribuciones[i].first.c_str());
    }

    if (ImPlot::BeginPlot("##lca_gwp", ImVec2(-1, 220), ImPlotFlags_NoLegend)) {
        ImPlot::SetupAxes(tr("Equivalente"), tr("kg CO2-eq"),
                          ImPlotAxisFlags_AutoFit, ImPlotAxisFlags_AutoFit);
        ImPlot::SetupAxisTicks(ImAxis_X1, posiciones.data(),
                               static_cast<int>(posiciones.size()),
                               etiquetas.data(), false);
        ImPlot::PlotBars("GWP", posiciones.data(), valores.data(),
                         static_cast<int>(valores.size()), 0.6);
        ImPlot::EndPlot();
    }

    if (ImGui::BeginTable("lca_contribuciones", 3,
                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn(tr("Equivalente"), ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn(tr("kg CO2-eq"), ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableSetupColumn(tr("%"), ImGuiTableColumnFlags_WidthFixed, 70.0f);
        ImGui::TableHeadersRow();
        const double total = resultado_.gwp_kg_co2eq > 0.0 ? resultado_.gwp_kg_co2eq : 1.0;
        for (const auto& [nombre, valor] : resultado_.contribuciones) {
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::TextUnformatted(nombre.c_str());
            ImGui::TableNextColumn(); ImGui::Text("%.4g", valor);
            ImGui::TableNextColumn(); ImGui::Text("%.1f", valor / total * 100.0);
        }
        ImGui::EndTable();
    }
}

void PanelLCA::dibujar_validacion() {
    if (!calculado_) calcular();

    const bool masa_ok = resultado_.balance_masa_ok;
    ImGui::TextColored(masa_ok ? color_ok() : color_malo(),
                       masa_ok ? tr("Balance de masa: OK")
                               : tr("Balance de masa: Δ %.4g kg/h"),
                       resultado_.masa_entrada_kg - resultado_.masa_salida_kg);
    ImGui::Text("Entradas: %.5g kg/h · Salidas: %.5g kg/h",
                resultado_.masa_entrada_kg, resultado_.masa_salida_kg);

    if (unidad_funcional_kg_ > 0.0)
        ImGui::Text("Unidad funcional: %.5g kg/h (factor %.5g 1/kg)",
                    unidad_funcional_kg_, resultado_.factor_unidad_funcional);
    else
        ImGui::TextColored(color_aviso(), tr("Unidad funcional sin definir."));

    ImGui::Spacing();
    if (resultado_.filas_sin_mapear > 0) {
        ImGui::TextColored(color_aviso(), tr("%d fila(s) sin mapear (%.5g kg/h):"),
                           resultado_.filas_sin_mapear, resultado_.masa_sin_mapear_kg);
        ImGui::BeginChild("lca_sin_mapear", ImVec2(0, 110), true);
        int mostradas = 0;
        for (const auto& f : filas_) {
            if (f.mapeada()) continue;
            ImGui::BulletText("%s · %s · %.4g kg/h", f.corriente.c_str(),
                              f.especie.c_str(), f.kg_h);
            if (++mostradas >= 12) {
                ImGui::TextDisabled("…");
                break;
            }
        }
        ImGui::EndChild();
    } else {
        ImGui::TextColored(color_ok(), tr("Todas las filas están mapeadas."));
    }

    if (!resultado_.avisos.empty()) {
        ImGui::Spacing();
        ImGui::TextColored(color_aviso(), tr("Avisos:"));
        ImGui::BeginChild("lca_avisos", ImVec2(0, 110), true);
        for (const auto& a : resultado_.avisos) ImGui::BulletText("%s", a.c_str());
        ImGui::EndChild();
    }
}

void PanelLCA::dibujar_ayuda() {
    ImGui::TextColored(AdaptiveColor(0.6f, 0.8f, 1.0f, 1.0f), tr("Metodología del módulo LCA"));
    ImGui::Separator();
    ImGui::TextWrapped(
        "1. Frontera: el dataset incluye únicamente corrientes que cruzan la frontera "
        "del sistema (fuente→proceso y proceso→sumidero). Las corrientes internas "
        "proceso→proceso se excluyen para evitar doble conteo.");
    ImGui::TextWrapped(
        "2. Desagregación: cada corriente se separa por especie; la masa de cada "
        "especie se mapea a un equivalente del catálogo (CO2, CH4, SO2, NOx, agua, "
        "escoria, menas, etc.).");
    ImGui::TextWrapped(
        "3. Factores: GWP100 del IPCC AR6; acidificación y eutrofización CML 2016; "
        "agua consuntiva y energía primaria con factores genéricos de screening. "
        "Los consumos manuales (electricidad, gas, diésel, agua y residuo) usan "
        "factores documentados en el código del motor.");
    ImGui::TextWrapped(
        "4. Unidad funcional: si se define una cantidad de producto (kg/h), todos "
        "los indicadores y masas se expresan por kg de producto. La UF también se "
        "aplica a los consumos manuales y a las exportaciones.");
    ImGui::TextWrapped(
        "5. Exportación: 'To GaBi (CSV)' genera un mapeo de intercambio por especie "
        "(no un plan nativo de GaBi). 'To Excel (CSV)' añade BOM UTF-8 para Excel y "
        "'JSON' produce un dataset neutro para otras herramientas LCA.");
    ImGui::Spacing();
    ImGui::TextDisabled(tr("Los flujos de energía del flowsheet se ignoran; decláralos en Manuales."));
}

void PanelLCA::dibujar_catalogo() {
    // Encabezado doble estilo HSC LCA Evaluation: "Flows" | "Keyword Search"
    ImGui::Columns(2, "##LCAFlowSearchSplit", true);
    ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.68f);

    ImGui::TextColored(ImVec4(0.18f, 0.24f, 0.34f, 1.0f), "Flows");
    ImGui::Separator();

    const std::string consulta = minusculas(busqueda_catalogo_);
    std::map<std::string, std::vector<const motores::EquivalenteLCA*>> grupos;
    for (const auto& e : motores::LCA::catalogo()) {
        if (!consulta.empty() && !coincide(e.nombre, consulta) &&
            !coincide(e.id, consulta) && !coincide(e.grupo, consulta)) {
            continue;
        }
        grupos[e.grupo].push_back(&e);
    }

    ImGui::BeginChild("lca_catalogo_arbol", ImVec2(0, -145.0f), false);

    // Árbol jerárquico GaBi / ecoinvent idéntico a la captura de HSC LCA Evaluation
    if (ImGui::TreeNodeEx("Deposited goods", ImGuiTreeNodeFlags_SpanAvailWidth)) {
        for (const auto& [grupo, entradas] : grupos) {
            if (grupo.find("Residuo") == std::string::npos && grupo.find("Slag") == std::string::npos) continue;
            for (const auto* e : entradas) {
                if (ImGui::Selectable(e->nombre.c_str(), equivalente_sel_ == e->id)) {
                    equivalente_sel_ = e->id;
                    grupo_sel_ = e->grupo;
                }
            }
        }
        ImGui::TreePop();
    }
    if (ImGui::TreeNodeEx("Emissions to agricultural soil", ImGuiTreeNodeFlags_SpanAvailWidth)) {
        ImGui::TextDisabled("Heavy metals to agricultural soil");
        ImGui::TreePop();
    }
    if (ImGui::TreeNodeEx("Emissions to air", ImGuiTreeNodeFlags_DefaultOpen | ImGuiTreeNodeFlags_SpanAvailWidth)) {
        const char* sub_air[] = {
            "ecoinvent long-term to air",
            "Heavy metals to air",
            "Inorganic emissions to air",
            "Organic emissions to air (group VOC)",
            "Other emissions to air",
            "Particles to air",
            "Radioactive emissions to air"
        };
        for (int s = 0; s < 7; ++s) {
            if (ImGui::TreeNodeEx(sub_air[s], (s == 2) ? ImGuiTreeNodeFlags_DefaultOpen : 0)) {
                for (const auto& e : motores::LCA::catalogo()) {
                    if (!consulta.empty() && !coincide(e.nombre, consulta) && !coincide(e.id, consulta)) continue;
                    if (ImGui::Selectable(e.nombre.c_str(), equivalente_sel_ == e.id)) {
                        equivalente_sel_ = e.id;
                        grupo_sel_ = e.grupo;
                    }
                }
                ImGui::TreePop();
            }
        }
        ImGui::TreePop();
    }
    if (ImGui::TreeNodeEx("Emissions to fresh water", ImGuiTreeNodeFlags_SpanAvailWidth)) {
        for (const auto& e : motores::LCA::catalogo()) {
            if (ImGui::Selectable(e.nombre.c_str(), equivalente_sel_ == e.id)) {
                equivalente_sel_ = e.id;
                grupo_sel_ = e.grupo;
            }
        }
        ImGui::TreePop();
    }
    if (ImGui::TreeNodeEx("Emissions to industrial soil", ImGuiTreeNodeFlags_SpanAvailWidth)) {
        ImGui::TreePop();
    }
    if (ImGui::TreeNodeEx("Emissions to sea water", ImGuiTreeNodeFlags_SpanAvailWidth)) {
        ImGui::TreePop();
    }
    if (ImGui::TreeNodeEx("Others", ImGuiTreeNodeFlags_DefaultOpen | ImGuiTreeNodeFlags_SpanAvailWidth)) {
        for (const auto& [grupo, entradas] : grupos) {
            if (!ImGui::TreeNodeEx(grupo.c_str(), ImGuiTreeNodeFlags_DefaultOpen)) continue;
            for (const auto* e : entradas) {
                const bool seleccionado = equivalente_sel_ == e->id;
                if (ImGui::Selectable(e->nombre.c_str(), seleccionado)) {
                    equivalente_sel_ = e->id;
                    grupo_sel_ = e->grupo;
                }
            }
            ImGui::TreePop();
        }
        ImGui::TreePop();
    }

    ImGui::EndChild();

    ImGui::NextColumn();
    const auto tk_cat = css::tokens();
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk_cat.text_primary), "Keyword Search");
    ImGui::Separator();
    ImGui::SetNextItemWidth(-1.0f);
    ImGui::InputTextWithHint("##buscar_catalogo", "Keyword...", busqueda_catalogo_, sizeof(busqueda_catalogo_));
    ImGui::Spacing();
    ImGui::Checkbox("Apply to species", &aplicar_a_especie_);
    if (ImGui::Button("Clear row mapping", ImVec2(-1, 0)) &&
        fila_sel_ >= 0 && fila_sel_ < static_cast<int>(filas_.size())) {
        filas_[static_cast<size_t>(fila_sel_)].equivalente = kNoMapeado;
        calcular();
    }
    ImGui::Columns(1);

    // Cuadro inferior de metadatos idéntico a la esquina inferior derecha de HSC LCA Evaluation
    ImGui::Separator();
    ImGui::PushStyleColor(ImGuiCol_ChildBg, tk_cat.card_bg);
    ImGui::BeginChild("##LCAMetadataBox", ImVec2(0, 0), true);
    if (const auto* e = motor_.buscar_equivalente(equivalente_sel_)) {
        ImGui::Text("Name: %s (%s)", e->nombre.c_str(), e->id.c_str());
        ImGui::TextWrapped("Category: \\Flows\\%s\\%s", e->grupo.c_str(), e->nombre.c_str());
        ImGui::Text("Unit: %s", e->unidad.c_str());
        ImGui::Text("Reference Quantity: Mass / Standard Flow (%s)", e->fuente.c_str());
        ImGui::Text("Energy Quantity: %.4g MJ/%s", e->energia, e->unidad.c_str());
        ImGui::Text("Conversion Factor: GWP100 = %.4g kg CO2-eq | AP = %.4g kg SO2-eq | EP = %.4g kg PO4-eq",
                    e->gwp, e->ap, e->ep);
    } else {
        ImGui::Text("Name: name");
        ImGui::TextWrapped("Category: \\Flow hierarchy level 1\\Flow hierarchy level 2\\Flow hierarchy level 3\\Flow hierarchy level 4\\Flow hierarchy level 5");
        ImGui::Text("Unit: unit");
        ImGui::Text("Reference Quantity: reference quantity");
        ImGui::Text("Energy Quantity: energy quantity");
        ImGui::Text("Conversion Factor: conversion factor");
    }
    ImGui::EndChild();
    ImGui::PopStyleColor();
}

// ─────────────────────────────────────────────────────────────────────────────
void PanelLCA::renderizar(diagrama_flujo::Flowsheet* flowsheet) {
    css::ScopedHSCTheme hsc_theme;

    if (flowsheet == nullptr) {
        ImGui::Text(tr("Cargue una simulación para evaluar LCA."));
        return;
    }
    if (!cargado_ || fs_actual_ != flowsheet) {
        cargar_flujos(flowsheet);
    }

    dibujar_toolbar(flowsheet);

    const float ancho_split = ImGui::GetContentRegionAvail().x;
    ImGui::Columns(2, "lca_split", true);
    static bool lca_columnas_inicializadas = false;
    if (!lca_columnas_inicializadas && ancho_split > 100.0f) {
        ImGui::SetColumnWidth(0, ancho_split * 0.62f);
        ImGui::SetColumnWidth(1, ancho_split * 0.38f);
        lca_columnas_inicializadas = true;
    }

    ImGui::BeginChild("lca_izquierda", ImVec2(0, 0), true);
    if (ImGui::BeginTabBar("lca_tabs")) {
        auto tab_flags = [&](int idx) -> ImGuiTabItemFlags {
            return (pestana_forzada_ == idx) ? ImGuiTabItemFlags_SetSelected : 0;
        };

        if (ImGui::BeginTabItem("Inputs", nullptr, tab_flags(0))) {
            dibujar_tabla(true);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Outputs", nullptr, tab_flags(1))) {
            dibujar_tabla(false);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Manual Inputs", nullptr, tab_flags(2))) {
            dibujar_manuales();
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Manual Outputs", nullptr, tab_flags(3))) {
            dibujar_manuales();
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Indicators", nullptr, tab_flags(4))) {
            dibujar_indicadores();
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Diagnostics")) {
            dibujar_validacion();
            ImGui::EndTabItem();
        }
        pestana_forzada_ = -1;
        ImGui::EndTabBar();
    }
    ImGui::EndChild();

    ImGui::NextColumn();
    ImGui::BeginChild("lca_derecha", ImVec2(0, 0), true);
    dibujar_catalogo();
    ImGui::EndChild();
    ImGui::Columns(1);

    if (mostrar_ayuda_) {
        ImGui::SetNextWindowSize(ImVec2(560, 420), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Ayuda - Evaluación LCA"), &mostrar_ayuda_)) {
            dibujar_ayuda();
        }
        ImGui::End();
    }

    if (mostrar_validacion_) {
        ImGui::OpenPopup(tr("Validación del dataset LCA"));
        mostrar_validacion_ = false;
    }
    if (ImGui::BeginPopupModal(tr("Validación del dataset LCA"), nullptr,
                               ImGuiWindowFlags_AlwaysAutoResize)) {
        dibujar_validacion();
        ImGui::Spacing();
        if (export_pendiente_ != 0) {
            ImGui::TextColored(color_aviso(),
                               tr("Hay filas sin mapear o el balance de masa no cierra."));
            if (ImGui::Button(tr("Exportar de todos modos"))) {
                const int tipo = export_pendiente_;
                export_pendiente_ = 0;
                ImGui::CloseCurrentPopup();
                if (tipo == 1) exportar_gabi(false);
                else if (tipo == 2) exportar_gabi(true);
                else exportar_json();
            }
            ImGui::SameLine();
            if (ImGui::Button(tr("Cancelar"))) {
                export_pendiente_ = 0;
                ImGui::CloseCurrentPopup();
            }
        } else {
            if (ImGui::Button(tr("Cerrar"))) ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }
}

}} // namespace cma::interfaz
