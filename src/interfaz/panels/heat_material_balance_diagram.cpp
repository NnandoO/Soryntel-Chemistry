#include "shared.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <string>

namespace cma { namespace interfaz {

namespace {
ImVec4 color_ok()   { return cma::interfaz::AdaptiveColor(0.30f, 0.85f, 0.40f, 1.0f); }
ImVec4 color_malo() { return cma::interfaz::AdaptiveColor(0.90f, 0.40f, 0.30f, 1.0f); }
ImVec4 color_aviso(){ return cma::interfaz::AdaptiveColor(0.95f, 0.75f, 0.25f, 1.0f); }
} // namespace

void PanelBalanceHoja::dibujar_pestana_balance() {
    const double ue_kj = motores::factor_energia_kJ(unidad_energia());
    const char* ue = etiqueta_energia();

    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f, 0.8f, 0.6f, 1.0f),
                       "Resumen del Balance Térmico y Material (%s)",
                       areas_ui_.empty() ? "Balance Area 1"
                                         : areas_ui_[static_cast<size_t>(area_activa_)].nombre.c_str());

    const double hf[2] = {resultado_.h_formacion_entrada_kJ, resultado_.h_formacion_salida_kJ};
    const double hs[2] = {resultado_.h_sensible_entrada_kJ, resultado_.h_sensible_salida_kJ};
    const double ht[2] = {resultado_.h_entrada_kJ, resultado_.h_salida_kJ};
    const double ex[2] = {resultado_.exergia_entrada_kJ, resultado_.exergia_salida_kJ};
    const double km[2] = {resultado_.kmol_entrada, resultado_.kmol_salida};
    const double kgv[2] = {resultado_.masa_entrada_kg, resultado_.masa_salida_kg};
    const double nm[2] = {resultado_.nm3_entrada, resultado_.nm3_salida};
    const char* nombres[2] = {"INPUTS (IN)", "OUTPUTS (OUT)"};

    if (ImGui::BeginTable("hmb_resumen_detallado", 8,
                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Concepto");
        ImGui::TableSetupColumn("Amount kmol");
        ImGui::TableSetupColumn("Amount kg");
        ImGui::TableSetupColumn("Amount Nm³");
        ImGui::TableSetupColumn("Latent Hf");
        ImGui::TableSetupColumn("Heat Content");
        ImGui::TableSetupColumn("Total H");
        ImGui::TableSetupColumn("Exergy");
        ImGui::TableHeadersRow();

        for (int i = 0; i < 2; ++i) {
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text("%s", nombres[i]);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", km[i]);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", kgv[i]);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", nm[i]);
            ImGui::TableNextColumn(); ImGui::Text("%.2f", hf[i] / ue_kj);
            ImGui::TableNextColumn(); ImGui::Text("%.2f", hs[i] / ue_kj);
            ImGui::TableNextColumn(); ImGui::Text("%.2f", ht[i] / ue_kj);
            ImGui::TableNextColumn(); ImGui::Text("%.2f", ex[i] / ue_kj);
        }

        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::TextColored(color_aviso(), "BALANCE (OUT - IN)");
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.kmol_salida - resultado_.kmol_entrada);
        ImGui::TableNextColumn();
        ImGui::TextColored(resultado_.balance_masa_ok ? color_ok() : color_malo(),
                           "%.3f", -resultado_.residual_masa_kg);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.nm3_salida - resultado_.nm3_entrada);
        ImGui::TableNextColumn();
        ImGui::Text("%.2f", (hf[1] - hf[0]) / ue_kj);
        ImGui::TableNextColumn();
        ImGui::Text("%.3f", (hs[1] - hs[0]) / ue_kj);
        ImGui::TableNextColumn();
        ImGui::TextColored(resultado_.balance_energia_ok ? color_ok() : color_malo(),
                           "%.3f", -resultado_.residual_energia_kJ / ue_kj);
        ImGui::TableNextColumn();
        ImGui::Text("%.3f", (ex[1] - ex[0]) / ue_kj);
        ImGui::EndTable();
    }

    ImGui::Spacing();
    ImGui::Text("Heat Loss / Calor externo neto: %.3f %s   |   ΔH reacción (298 K): %.3f %s",
                resultado_.calor_externo_kJ / ue_kj, ue,
                (hf[1] - hf[0]) / ue_kj, ue);
    if (ultimo_solver_.factible) {
        ImGui::TextColored(color_ok(), "Temp. Balance (ΔH = 0): %.3f %s  ·  iteraciones: %d",
                           motores::temperatura_desde_K(ultimo_solver_.temperatura_K,
                                                        unidad_temperatura()),
                           etiqueta_temperatura(), ultimo_solver_.iteraciones);
    }

    ImGui::Spacing();
    if (ImGui::Button("Temp. Balance (ΔH = 0)")) temp_balance();
    ImGui::SameLine();
    if (ImGui::Button("Ajustar calor al balance")) ajustar_calor();
    ImGui::SameLine();
    if (ImGui::Button("Copiar informe")) copiar_informe();
    ImGui::SameLine();
    if (ImGui::Button("Diagram")) show_diagram_ = !show_diagram_;

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.6f, 1.0f),
                       "Resumen por Corriente (Streams)");
    if (ImGui::BeginTable("hmb_por_corriente", 9,
                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                          ImGuiTableFlags_ScrollY, ImVec2(0, 0))) {
        ImGui::TableSetupColumn("Stream");
        ImGui::TableSetupColumn("Type");
        ImGui::TableSetupColumn("Temper.");
        ImGui::TableSetupColumn("Amount kmol");
        ImGui::TableSetupColumn("Amount kg");
        ImGui::TableSetupColumn("Amount Nm³");
        ImGui::TableSetupColumn("Heat Content");
        ImGui::TableSetupColumn("Total H");
        ImGui::TableSetupColumn("Exergy");
        ImGui::TableHeadersRow();

        auto fila_corriente = [&](const motores::CorrienteBalance& c, const char* tipo) {
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text("%s", c.nombre.c_str());
            ImGui::TableNextColumn(); ImGui::Text("%s", tipo);
            double t_disp = c.filas.empty()
                                ? 0.0
                                : motores::temperatura_desde_K(c.filas.front().temperatura_K,
                                                               unidad_temperatura());
            ImGui::TableNextColumn(); ImGui::Text("%.2f %s", t_disp, etiqueta_temperatura());
            double n = 0, kg = 0, nm3 = 0, hs_c = 0, ht_c = 0, ex_c = 0;
            for (const auto& f : c.filas) {
                n += f.kmol;
                kg += f.masa_kg;
                nm3 += f.volumen_nm3;
                hs_c += f.h_sensible_kJ;
                ht_c += f.h_total_kJ;
                ex_c += f.exergia_total_kJ;
            }
            ImGui::TableNextColumn(); ImGui::Text("%.3f", n);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", kg);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", nm3);
            ImGui::TableNextColumn(); ImGui::Text("%.2f", hs_c / ue_kj);
            ImGui::TableNextColumn(); ImGui::Text("%.2f", ht_c / ue_kj);
            ImGui::TableNextColumn(); ImGui::Text("%.2f", ex_c / ue_kj);
        };
        for (const auto& c : area_.entradas) fila_corriente(c, "IN");
        for (const auto& c : area_.salidas) fila_corriente(c, "OUT");
        ImGui::EndTable();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Diagrama (barrido de cantidad vs temperatura de equilibrio)
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::actualizar_diagrama() {
    diag_x_.clear();
    diag_y_.clear();
    if (area_.entradas.empty()) return;

    if (diag_entrada_ < 0 || diag_entrada_ >= static_cast<int>(area_.entradas.size()))
        diag_entrada_ = 0;
    const motores::CorrienteBalance& ce_base = area_.entradas[static_cast<size_t>(diag_entrada_)];
    if (ce_base.filas.empty()) return;
    if (diag_fila_ < 0 || diag_fila_ >= static_cast<int>(ce_base.filas.size())) diag_fila_ = 0;

    if (diag_max_ <= diag_min_) std::swap(diag_min_, diag_max_);
    const int puntos = std::clamp(diag_puntos_, 2, 200);

    const motores::FilaBalance& fila_base = ce_base.filas[static_cast<size_t>(diag_fila_)];
    const double mw = (fila_base.especie != nullptr && fila_base.especie->masa_molar > 0.0)
                          ? fila_base.especie->masa_molar
                          : 16.043;
    const double vm = (fila_base.especie != nullptr &&
                       motores::FilaBalance::volumen_molar_nm3(fila_base.especie) > 0.0)
                          ? motores::FilaBalance::volumen_molar_nm3(fila_base.especie)
                          : 22.414;

    auto x_a_kmol = [&](double x_val) {
        if (diag_unidad_x_ == 1) return x_val / mw;
        if (diag_unidad_x_ == 2) return x_val / vm;
        return x_val;
    };

    const double kmol_base = std::max(fila_base.kmol, 1e-9);
    bool hay_salida_fija = false;
    for (const auto& cs : area_.salidas) {
        if (cs.temperatura_fija) {
            hay_salida_fija = true;
            break;
        }
    }

    for (int k = 0; k < puntos; ++k) {
        const double x = diag_min_ + (diag_max_ - diag_min_) * k / (puntos - 1);
        const double kmol_nuevo = std::max(0.0, x_a_kmol(x));
        const double ratio = kmol_nuevo / kmol_base;

        motores::AreaBalance copia = area_;
        if (diag_acoplar_esteq_ && kmol_base > 1e-8) {
            // Escala las corrientes reactivas/gaseosas proporcionalmente manteniendo
            // constantes las corrientes auxiliares de enfriamiento (temperatura_fija).
            for (size_t i = 0; i < copia.entradas.size(); ++i) {
                bool es_agua_enfr = (copia.entradas[i].nombre.find("Water") != std::string::npos ||
                                     copia.entradas[i].nombre.find("Agua") != std::string::npos);
                if (hay_salida_fija && es_agua_enfr && static_cast<int>(i) != diag_entrada_)
                    continue;
                for (auto& f : copia.entradas[i].filas) f.kmol *= ratio;
            }
            for (size_t i = 0; i < copia.salidas.size(); ++i) {
                if (hay_salida_fija && copia.salidas[i].temperatura_fija) continue;
                copia.salidas[i].calor_kJ = 0.0;
                for (auto& f : copia.salidas[i].filas) f.kmol *= ratio;
            }
        } else {
            copia.entradas[static_cast<size_t>(diag_entrada_)]
                .filas[static_cast<size_t>(diag_fila_)].kmol = kmol_nuevo;
        }

        const auto res = motor_.temperatura_equilibrio(copia, mantener_offsets_);
        diag_x_.push_back(x);
        diag_y_.push_back(res.factible
                              ? motores::temperatura_desde_K(res.temperatura_K,
                                                             unidad_temperatura())
                              : std::nan(""));
    }
}

void PanelBalanceHoja::dibujar_diagrama() {
    if (area_.entradas.empty()) {
        ImGui::TextDisabled("Agrega corrientes de entrada para generar el diagrama.");
        return;
    }

    // Selector de corriente de entrada
    std::string items_entrada;
    for (size_t i = 0; i < area_.entradas.size(); ++i) {
        char etiqueta[160];
        std::snprintf(etiqueta, sizeof(etiqueta), "IN %d: %s",
                      static_cast<int>(i) + 1, area_.entradas[i].nombre.c_str());
        items_entrada += etiqueta;
        items_entrada.push_back('\0');
    }
    items_entrada.push_back('\0');
    ImGui::SetNextItemWidth(220);
    if (ImGui::Combo("Corriente", &diag_entrada_, items_entrada.c_str())) {
        diag_fila_ = 0;
        diag_dirty_ = true;
    }
    ImGui::SameLine();

    // Selector de fila (especie)
    std::string formula_sel = "Input";
    if (diag_entrada_ >= 0 && diag_entrada_ < static_cast<int>(area_.entradas.size())) {
        const auto& filas = area_.entradas[static_cast<size_t>(diag_entrada_)].filas;
        std::string items_fila;
        for (size_t i = 0; i < filas.size(); ++i) {
            char etiqueta[160];
            std::snprintf(etiqueta, sizeof(etiqueta), "%s (%.3f kmol / %.3f Nm³)",
                          filas[i].formula.c_str(), filas[i].kmol, filas[i].volumen_nm3);
            items_fila += etiqueta;
            items_fila.push_back('\0');
        }
        items_fila.push_back('\0');
        ImGui::SetNextItemWidth(220);
        if (!filas.empty()) {
            if (ImGui::Combo("Especie", &diag_fila_, items_fila.c_str()))
                diag_dirty_ = true;
            if (diag_fila_ >= 0 && diag_fila_ < static_cast<int>(filas.size()))
                formula_sel = filas[static_cast<size_t>(diag_fila_)].formula;
        }
    }

    ImGui::SetNextItemWidth(110);
    const char* unidades_x[] = {"kmol", "kg", "Nm³"};
    if (ImGui::Combo("Unidad X", &diag_unidad_x_, unidades_x, 3)) diag_dirty_ = true;
    ImGui::SameLine();
    ImGui::SetNextItemWidth(85);
    if (ImGui::InputDouble("Min", &diag_min_, 0, 0, "%.4g")) diag_dirty_ = true;
    ImGui::SameLine();
    ImGui::SetNextItemWidth(85);
    if (ImGui::InputDouble("Max", &diag_max_, 0, 0, "%.4g")) diag_dirty_ = true;
    ImGui::SameLine();
    ImGui::SetNextItemWidth(85);
    if (ImGui::SliderInt("Puntos", &diag_puntos_, 5, 100)) diag_dirty_ = true;

    if (ImGui::Checkbox("Acoplar estequiométricamente gases de combustión/reacción",
                        &diag_acoplar_esteq_)) {
        diag_dirty_ = true;
    }
    ImGui::SameLine();
    if (ImGui::Checkbox("Conservar ΔT entre salidas", &mantener_offsets_)) diag_dirty_ = true;
    ImGui::SameLine();
    if (ImGui::Button("Actualizar")) diag_dirty_ = true;

    if (diag_dirty_) {
        actualizar_diagrama();
        diag_dirty_ = false;
    }

    if (diag_x_.empty()) {
        ImGui::TextDisabled("No hay datos para graficar.");
        return;
    }

    char eje_x[128], eje_y[64];
    std::snprintf(eje_x, sizeof(eje_x), "%s Input, %s",
                  formula_sel.c_str(), unidades_x[diag_unidad_x_]);
    std::snprintf(eje_y, sizeof(eje_y), "Temperature %s",
                  etiqueta_temperatura());

    std::string serie_nombre = "Output Gas";
    for (const auto& cs : area_.salidas) {
        if (!cs.temperatura_fija) {
            serie_nombre = cs.nombre;
            break;
        }
    }

    if (ImPlot::BeginPlot("##hmb_diagrama", ImVec2(-1, -1))) {
        ImPlot::SetupAxes(eje_x, eje_y);
        ImPlot::PushStyleVar(ImPlotStyleVar_PlotPadding, ImVec2(12, 12));
        ImPlot::SetNextLineStyle(ImVec4(0.32f, 0.48f, 0.18f, 1.0f), 3.0f);
        ImPlot::PlotLine(serie_nombre.c_str(), diag_x_.data(), diag_y_.data(),
                         static_cast<int>(diag_x_.size()));
        ImPlot::PopStyleVar();
        ImPlot::EndPlot();
    }
}
// ─────────────────────────────────────────────────────────────────────────────
// Carga desde el diagrama de flujo
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::cargar_flujos(diagrama_flujo::Flowsheet* fs) {
    if (fs == nullptr) return;

    entradas_.clear();
    salidas_.clear();

    for (const auto& [fid, flujo] : fs->mapa_flujos()) {
        if (flujo.es_energia) continue;

        bool tiene_origen = false;
        bool tiene_destino = false;
        bool producido_por_fuente = false;
        bool consumido_por_sumidero = false;
        for (const auto& n : fs->nodos()) {
            for (int64_t sid : n.ids_flujos_salida) {
                if (sid == fid) {
                    tiene_origen = true;
                    if (n.tipo == nucleo::TipoNodo::Fuente) producido_por_fuente = true;
                }
            }
            for (int64_t sid : n.ids_flujos_entrada) {
                if (sid == fid) {
                    tiene_destino = true;
                    if (n.tipo == nucleo::TipoNodo::Sumidero) consumido_por_sumidero = true;
                }
            }
        }

        const bool es_entrada = !tiene_origen || producido_por_fuente;
        const bool es_salida = (!tiene_destino || consumido_por_sumidero) && !es_entrada;
        if (!es_entrada && !es_salida) continue;  // corriente interna

        CorrienteUI c;
        copiar_texto(c.nombre, sizeof(c.nombre),
                     flujo.nombre.empty() ? ("Corriente " + std::to_string(fid))
                                          : flujo.nombre);

        const double T_display = (temp_unit_ == 0)
                                     ? flujo.temperatura - 273.15
                                     : flujo.temperatura;
        const double P_bar = flujo.presion * 1.01325;

        if (!flujo.flujo_molar.empty()) {
            for (const auto& [sid, n] : flujo.flujo_molar) {
                if (std::abs(n) < 1e-14) continue;
                const auto* esp = base_datos::GestorDB::instancia().especie_por_id(sid);
                FilaUI f;
                copiar_texto(f.formula, sizeof(f.formula),
                             esp != nullptr ? esp->formula
                                            : ("Especie #" + std::to_string(sid)));
                f.temper = T_display;
                f.presion = P_bar;
                f.kmol = n * 3600.0;  // kmol/s -> kmol/h
                c.filas.push_back(f);
            }
        } else {
            if (flujo.agua_kg_h > 0.0) {
                FilaUI f;
                copiar_texto(f.formula, sizeof(f.formula), "H2O(l)");
                f.temper = T_display;
                f.presion = P_bar;
                f.kmol = flujo.agua_kg_h / 18.015;
                c.filas.push_back(f);
            }
            for (const auto& p : flujo.particulas) {
                const auto* esp = buscar_especie_por_nombre_o_formula(p.mineral);
                if (esp != nullptr && esp->masa_molar > 0.0) {
                    FilaUI f;
                    copiar_texto(f.formula, sizeof(f.formula), esp->formula);
                    f.temper = T_display;
                    f.presion = P_bar;
                    f.kmol = p.mass_kg_h / esp->masa_molar;
                    c.filas.push_back(f);
                }
            }
        }

        if (c.filas.empty()) continue;
        (es_salida ? salidas_ : entradas_).push_back(c);
    }

    corriente_sel_ = 0;
    fila_sel_ = -1;
    lado_salida_sel_ = false;
    vista_ = Vista::Balance;
    ultimo_solver_ = motores::BalanceHoja::ResultadoTemperatura{};
    recalcular(true);
}

}} // namespace cma::interfaz

