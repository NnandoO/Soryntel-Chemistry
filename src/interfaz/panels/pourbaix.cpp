// ══════════════════════════════════════════════════════════════════════════════
//  pourbaix.cpp — Adaptador UI Hexagonal del Módulo HSC Eh-pH (Pourbaix / EpH)
// ══════════════════════════════════════════════════════════════════════════════
#include "shared.hpp"
#include "panel_pourbaix.hpp"
#include <imgui.h>
#include <implot.h>
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <limits>
#include <string>
#include <vector>

namespace cma::interfaz {

namespace Colores {
    static const ImVec4 ACENTO_CIAN     = {0.20f, 0.78f, 0.92f, 1.0f};
    static const ImVec4 ACENTO_VERDE    = {0.24f, 0.82f, 0.48f, 1.0f};
    static const ImVec4 ACENTO_AMARILLO = {0.95f, 0.78f, 0.22f, 1.0f};
    static const ImVec4 ACENTO_ROJO     = {0.92f, 0.32f, 0.32f, 1.0f};
}

static const ImVec4 kPaletteEspecies[] = {
    {0.22f, 0.56f, 0.88f, 0.85f},
    {0.24f, 0.78f, 0.45f, 0.85f},
    {0.92f, 0.56f, 0.18f, 0.85f},
    {0.86f, 0.28f, 0.36f, 0.85f},
    {0.62f, 0.42f, 0.88f, 0.85f},
    {0.18f, 0.78f, 0.78f, 0.85f},
    {0.90f, 0.76f, 0.22f, 0.85f},
    {0.82f, 0.38f, 0.68f, 0.85f},
    {0.46f, 0.76f, 0.26f, 0.85f},
    {0.48f, 0.56f, 0.68f, 0.85f},
};
static constexpr int kNumColors = 10;

PanelPourbaix::PanelPourbaix(const domain::ThermoRepository& repository)
    : calc_(&repository)
{
    sincronizar_y_calcular();
}

void PanelPourbaix::sincronizar_y_calcular() {
    calc_.establecer_elemento(elemento_actual_);
    calc_.establecer_temperatura(temperatura_actual_);
    calc_.establecer_presion(presion_actual_);
    calc_.establecer_concentracion(concentracion_actual_);
    calc_.establecer_escala_potencial(
        static_cast<domain::PourbaixPotentialScale>(std::clamp(escala_idx_, 0, 2)));

    calc_.limpiar_ligandos();
    if (lig_S_activo_)  calc_.agregar_o_actualizar_ligando("S",  lig_S_molal_,  true);
    if (lig_Cl_activo_) calc_.agregar_o_actualizar_ligando("Cl", lig_Cl_molal_, true);
    if (lig_C_activo_)  calc_.agregar_o_actualizar_ligando("C",  lig_C_molal_,  true);
    if (lig_N_activo_)  calc_.agregar_o_actualizar_ligando("N",  lig_N_molal_,  true);

    calc_.calcular();
    calculado_ = true;
}

void PanelPourbaix::renderizar() {
    ImGui::PushStyleColor(ImGuiCol_Text, Colores::ACENTO_CIAN);
    ImGui::Text("DIAGRAMAS Eh-pH (POURBAIX / HSC EpH) — HIDROMETALURGIA Y CORROSIÓN");
    ImGui::PopStyleColor();
    ImGui::Separator();

    // ── Barra superior de Presets Industriales HSC ───────────────────────────
    ImGui::TextDisabled("Presets HSC:");
    ImGui::SameLine();
    auto apply_preset = [&](int id, const char* elem, double T, double conc,
                            bool s_on, double s_m, bool cl_on, double cl_m, bool c_on, double c_m) {
        elemento_actual_ = elem;
        temperatura_actual_ = T;
        concentracion_actual_ = conc;
        lig_S_activo_ = s_on;   lig_S_molal_ = s_m;
        lig_Cl_activo_ = cl_on; lig_Cl_molal_ = cl_m;
        lig_C_activo_ = c_on;   lig_C_molal_ = c_m;
        lig_N_activo_ = false;
        calc_.cargar_preset_hsc(id);
        calculado_ = true;
    };
    if (ImGui::SmallButton("Fe-H2O (Corrosión)")) {
        apply_preset(0, "Fe", 298.15, 1e-6, false, 1e-2, false, 1e-1, false, 1e-3);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("Cu-H2O (Lixiviación)")) {
        apply_preset(1, "Cu", 298.15, 1e-4, false, 1e-2, false, 1e-1, false, 1e-3);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("Cu-S-H2O (Calcosina/Covelina)")) {
        apply_preset(2, "Cu", 298.15, 1e-4, true, 1e-2, false, 1e-1, false, 1e-3);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("Fe-S-C-H2O (Pirita/Siderita)")) {
        apply_preset(3, "Fe", 298.15, 1e-4, true, 1e-2, false, 1e-1, true, 1e-3);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("Au-Cl-H2O (Cloruración)")) {
        apply_preset(4, "Au", 298.15, 1e-5, false, 1e-2, true, 1.0, false, 1e-3);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("Mn-H2O (Pirolusita)")) {
        apply_preset(5, "Mn", 298.15, 1e-4, false, 1e-2, false, 1e-1, false, 1e-3);
    }
    ImGui::Separator();

    const float panel_izq_w = 360.0f;
    ImGui::BeginChild("##pourbaix_left", ImVec2(panel_izq_w, 0), true);
    {
        // 1. Elemento principal y condiciones P-T-m
        ImGui::TextColored(Colores::ACENTO_VERDE, "1. Sistema Químico Principal");
        static const char* kElementos[] = {
            "Fe", "Cu", "Zn", "Mn", "Ni", "Cr", "Al", "Au", "Ag", "Pb"
        };
        static int idx_elem = 0;
        for (int i = 0; i < 10; ++i) {
            if (elemento_actual_ == kElementos[i]) { idx_elem = i; break; }
        }
        ImGui::SetNextItemWidth(130.0f);
        if (ImGui::Combo("Metal Principal (M)", &idx_elem, kElementos, 10)) {
            elemento_actual_ = kElementos[idx_elem];
            sincronizar_y_calcular();
        }

        float temp_C = static_cast<float>(temperatura_actual_ - 273.15);
        ImGui::SetNextItemWidth(130.0f);
        if (ImGui::SliderFloat("Temperatura (°C)", &temp_C, 0.0f, 300.0f, "%.1f °C")) {
            temperatura_actual_ = temp_C + 273.15;
            calc_.establecer_temperatura(temperatura_actual_);
            calc_.calcular();
        }

        static float log_conc = -6.0f;
        log_conc = static_cast<float>(std::log10(std::max(1e-12, concentracion_actual_)));
        ImGui::SetNextItemWidth(130.0f);
        if (ImGui::SliderFloat("log10([M] mol/kg)", &log_conc, -8.0f, 0.0f, "10^(%.1f)")) {
            concentracion_actual_ = std::pow(10.0, static_cast<double>(log_conc));
            calc_.establecer_concentracion(concentracion_actual_);
            calc_.calcular();
        }

        const char* kEscalas[] = {"SHE (Estándar H2, 0.000 V)", "SCE (Calomelanos, +0.241 V)", "Ag/AgCl (KCl sat., +0.197 V)"};
        ImGui::SetNextItemWidth(195.0f);
        if (ImGui::Combo("Escala Ref.", &escala_idx_, kEscalas, 3)) {
            calc_.establecer_escala_potencial(static_cast<domain::PourbaixPotentialScale>(escala_idx_));
            calc_.calcular();
        }

        // 2. Ligandos Secundarios (M - L - H2O)
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::TextColored(Colores::ACENTO_VERDE, "2. Ligandos Disueltos (M-L-H2O)");
        bool lig_changed = false;
        lig_changed |= ImGui::Checkbox("S (H2S/HS-/SO4²⁻)", &lig_S_activo_);
        if (lig_S_activo_) {
            ImGui::SameLine();
            float l_s = static_cast<float>(std::log10(std::max(1e-10, lig_S_molal_)));
            ImGui::SetNextItemWidth(95.0f);
            if (ImGui::SliderFloat("##logS", &l_s, -6.0f, 0.0f, "10^%.1f m")) {
                lig_S_molal_ = std::pow(10.0, l_s);
                lig_changed = true;
            }
        }
        lig_changed |= ImGui::Checkbox("Cl (Cl-/HClO/ClO-)", &lig_Cl_activo_);
        if (lig_Cl_activo_) {
            ImGui::SameLine();
            float l_cl = static_cast<float>(std::log10(std::max(1e-10, lig_Cl_molal_)));
            ImGui::SetNextItemWidth(95.0f);
            if (ImGui::SliderFloat("##logCl", &l_cl, -6.0f, 0.5f, "10^%.1f m")) {
                lig_Cl_molal_ = std::pow(10.0, l_cl);
                lig_changed = true;
            }
        }
        lig_changed |= ImGui::Checkbox("C (H2CO3/HCO3-/CO3²⁻)", &lig_C_activo_);
        if (lig_C_activo_) {
            ImGui::SameLine();
            float l_c = static_cast<float>(std::log10(std::max(1e-10, lig_C_molal_)));
            ImGui::SetNextItemWidth(95.0f);
            if (ImGui::SliderFloat("##logC", &l_c, -6.0f, 0.0f, "10^%.1f m")) {
                lig_C_molal_ = std::pow(10.0, l_c);
                lig_changed = true;
            }
        }
        lig_changed |= ImGui::Checkbox("N (NH4+/NH3 lixiv.)", &lig_N_activo_);
        if (lig_N_activo_) {
            ImGui::SameLine();
            float l_n = static_cast<float>(std::log10(std::max(1e-10, lig_N_molal_)));
            ImGui::SetNextItemWidth(95.0f);
            if (ImGui::SliderFloat("##logN", &l_n, -6.0f, 0.5f, "10^%.1f m")) {
                lig_N_molal_ = std::pow(10.0, l_n);
                lig_changed = true;
            }
        }
        if (lig_changed) {
            sincronizar_y_calcular();
        }

        // 3. Opciones visuales
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::TextColored(Colores::ACENTO_VERDE, "3. Visualización y Capas");
        const char* kModosColor[] = {"Regiones de Predominancia Química", "Zonas de Corrosión (Pourbaix)"};
        ImGui::SetNextItemWidth(220.0f);
        ImGui::Combo("Mapa", &modo_color_, kModosColor, 2);
        ImGui::Checkbox("Mostrar regiones coloreadas", &mostrar_mapa_color_);
        ImGui::Checkbox("Límites de estabilidad del H2O (a, b)", &mostrar_agua_);
        ImGui::Checkbox("Etiquetas de fases y puntos triples", &mostrar_etiquetas_);

        // 4. Sonda puntual de operación (pH, Eh)
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::TextColored(Colores::ACENTO_AMARILLO, "4. Sonda de Punto de Operación");
        float p_pH = static_cast<float>(probe_pH_);
        float p_Eh = static_cast<float>(probe_Eh_);
        ImGui::SetNextItemWidth(110.0f);
        if (ImGui::SliderFloat("pH op.", &p_pH, static_cast<float>(calc_.pH_min()), static_cast<float>(calc_.pH_max()), "%.2f")) {
            probe_pH_ = p_pH;
        }
        ImGui::SameLine();
        ImGui::SetNextItemWidth(110.0f);
        if (ImGui::SliderFloat("Eh op. (V)", &p_Eh, static_cast<float>(calc_.Eh_min()), static_cast<float>(calc_.Eh_max()), "%.3f")) {
            probe_Eh_ = p_Eh;
        }
        const auto eval = calc_.evaluar_punto(probe_pH_, probe_Eh_);
        if (eval.valid) {
            ImGui::Text("Fase Predominante: %s (%s)",
                        eval.predominant_clean.c_str(), eval.predominant_state.c_str());
            ImGui::Text("Diagnóstico: %s", domain::pourbaix_corrosion_label(eval.corrosion_state));
            ImGui::Text("pe = %.2f | Margen ΔG = %.2f kJ/mol",
                        eval.pe, eval.stability_margin_kJ);
            if (!eval.water_stable) {
                ImGui::TextColored(Colores::ACENTO_ROJO,
                    eval.o2_evolution ? "Fuera del dominio H2O: Evolución de O2(g)"
                                      : "Fuera del dominio H2O: Evolución de H2(g)");
            } else {
                ImGui::TextColored(Colores::ACENTO_VERDE, "Dentro de la ventana de estabilidad del H2O(l)");
            }
        }

        // 5. Tabla interactiva de especies (activación y ΔG°(T))
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::TextColored(Colores::ACENTO_CIAN, "5. Especies del Sistema (%zu)", calc_.obtener_especies().size());
        if (ImGui::BeginTable("##pourbaix_sp_tbl", 5,
            ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_SizingStretchProp,
            ImVec2(0, 175)))
        {
            ImGui::TableSetupColumn("On", ImGuiTableColumnFlags_WidthFixed, 26.0f);
            ImGui::TableSetupColumn("Especie", ImGuiTableColumnFlags_WidthStretch, 1.1f);
            ImGui::TableSetupColumn("Est.", ImGuiTableColumnFlags_WidthFixed, 30.0f);
            ImGui::TableSetupColumn("ΔGf°(T)", ImGuiTableColumnFlags_WidthFixed, 68.0f);
            ImGui::TableSetupColumn("Zona", ImGuiTableColumnFlags_WidthFixed, 62.0f);
            ImGui::TableHeadersRow();

            auto& especies_mut = calc_.obtener_especies_mut();
            bool need_recalc = false;
            for (size_t i = 0; i < especies_mut.size(); ++i) {
                auto& sp = especies_mut[i];
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                ImGui::PushID(static_cast<int>(i));
                if (ImGui::Checkbox("##act", &sp.activa)) {
                    need_recalc = true;
                }
                ImGui::PopID();

                ImGui::TableSetColumnIndex(1);
                ImVec4 col = kPaletteEspecies[i % kNumColors];
                ImGui::TextColored(col, "%s", sp.formula_limpia.c_str());
                if (ImGui::IsItemHovered()) {
                    ImGui::SetTooltip("%s (ID: %lld, a=%.2e)",
                        sp.nombre.c_str(), static_cast<long long>(sp.id), sp.molalidad);
                }

                ImGui::TableSetColumnIndex(2);
                ImGui::TextUnformatted(sp.estado.c_str());

                ImGui::TableSetColumnIndex(3);
                ImGui::Text("%.1f", sp.dG0);

                ImGui::TableSetColumnIndex(4);
                const char* z_short =
                    (sp.estado_corrosion == domain::PourbaixCorrosionState::Immunity) ? "Inmune" :
                    (sp.estado_corrosion == domain::PourbaixCorrosionState::Passivation) ? "Pasiva" : "Corros.";
                ImGui::TextUnformatted(z_short);
            }
            ImGui::EndTable();
            if (need_recalc) calc_.calcular();
        }

        // Exportación CSV
        ImGui::SetNextItemWidth(180.0f);
        ImGui::InputText("##csv_pourbaix", csv_path_, sizeof(csv_path_));
        ImGui::SameLine();
        if (ImGui::Button("Exportar CSV")) {
            if (calc_.guardar_csv(csv_path_)) {
                estado_msg_ = std::string("Exportado a ") + csv_path_;
            } else {
                estado_msg_ = "Error al guardar CSV";
            }
        }
        if (!estado_msg_.empty()) {
            ImGui::TextDisabled("%s", estado_msg_.c_str());
        }
    }
    ImGui::EndChild();

    ImGui::SameLine();

    // ── Panel derecho: Gráfico Eh-pH y Pestañas de Ecuaciones Nernst / Puntos Triples ──
    ImGui::BeginChild("##pourbaix_right", ImVec2(0, 0), false);
    {
        const float avail_h = ImGui::GetContentRegionAvail().y;
        const float plot_h  = std::max(280.0f, avail_h * 0.68f);

        char titulo[192];
        std::snprintf(titulo, sizeof(titulo),
            "Diagrama Eh-pH: %s - H2O (T = %.1f °C, [M] = %.1e m, %s)###PourbaixPlot",
            elemento_actual_.c_str(),
            temperatura_actual_ - 273.15,
            concentracion_actual_,
            domain::pourbaix_scale_label(calc_.escala_potencial()));

        if (ImPlot::BeginPlot(titulo, ImVec2(-1, plot_h))) {
            ImPlot::SetupAxes("pH", domain::pourbaix_scale_label(calc_.escala_potencial()));
            ImPlot::SetupAxesLimits(calc_.pH_min(), calc_.pH_max(),
                                    calc_.Eh_min(), calc_.Eh_max(), ImGuiCond_Once);

            // Clic en el gráfico para mover la sonda operativa
            if (ImPlot::IsPlotHovered() && ImGui::IsMouseClicked(ImGuiMouseButton_Left)) {
                ImPlotPoint pt = ImPlot::GetPlotMousePos();
                probe_pH_ = std::clamp(pt.x, calc_.pH_min(), calc_.pH_max());
                probe_Eh_ = std::clamp(pt.y, calc_.Eh_min(), calc_.Eh_max());
            }

            const auto& dom = calc_.obtener_dominancia();
            const auto& especies = calc_.obtener_especies();

            // 1. Mapa de predominancia por lotes (1 llamada por especie o zona de corrosión)
            if (mostrar_mapa_color_ && !dom.pH_grid.empty() && !dom.Eh_grid.empty()) {
                if (modo_color_ == 0) {
                    std::vector<std::vector<double>> xs_sp(especies.size());
                    std::vector<std::vector<double>> ys_sp(especies.size());
                    for (size_t ie = 0; ie < dom.Eh_grid.size(); ++ie) {
                        for (size_t ip = 0; ip < dom.pH_grid.size(); ++ip) {
                            int idx = dom.especie_dom[ie][ip];
                            if (idx >= 0 && static_cast<size_t>(idx) < especies.size()) {
                                xs_sp[static_cast<size_t>(idx)].push_back(dom.pH_grid[ip]);
                                ys_sp[static_cast<size_t>(idx)].push_back(dom.Eh_grid[ie]);
                            }
                        }
                    }
                    for (size_t k = 0; k < especies.size(); ++k) {
                        if (xs_sp[k].empty()) continue;
                        ImVec4 col = kPaletteEspecies[k % kNumColors];
                        col.w = 0.28f;
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Square, 4.2f, col, 0.0f, col);
                        std::string lbl = especies[k].formula_limpia + " (" + especies[k].estado + ")";
                        ImPlot::PlotScatter(lbl.c_str(), xs_sp[k].data(), ys_sp[k].data(),
                                            static_cast<int>(xs_sp[k].size()));
                    }
                } else {
                    std::vector<double> x_imm, y_imm, x_pas, y_pas, x_cor, y_cor;
                    for (size_t ie = 0; ie < dom.Eh_grid.size(); ++ie) {
                        for (size_t ip = 0; ip < dom.pH_grid.size(); ++ip) {
                            int cz = dom.corrosion_grid[ie][ip];
                            if (cz == 0) { x_imm.push_back(dom.pH_grid[ip]); y_imm.push_back(dom.Eh_grid[ie]); }
                            else if (cz == 1) { x_pas.push_back(dom.pH_grid[ip]); y_pas.push_back(dom.Eh_grid[ie]); }
                            else { x_cor.push_back(dom.pH_grid[ip]); y_cor.push_back(dom.Eh_grid[ie]); }
                        }
                    }
                    if (!x_imm.empty()) {
                        ImVec4 c(0.20f, 0.78f, 0.38f, 0.30f);
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Square, 4.2f, c, 0.0f, c);
                        ImPlot::PlotScatter("Inmunidad (Metal Estable)", x_imm.data(), y_imm.data(), static_cast<int>(x_imm.size()));
                    }
                    if (!x_pas.empty()) {
                        ImVec4 c(0.24f, 0.62f, 0.92f, 0.30f);
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Square, 4.2f, c, 0.0f, c);
                        ImPlot::PlotScatter("Pasivación (Capa Sólida)", x_pas.data(), y_pas.data(), static_cast<int>(x_pas.size()));
                    }
                    if (!x_cor.empty()) {
                        ImVec4 c(0.92f, 0.32f, 0.32f, 0.30f);
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Square, 4.2f, c, 0.0f, c);
                        ImPlot::PlotScatter("Corrosión Activa (Disolución)", x_cor.data(), y_cor.data(), static_cast<int>(x_cor.size()));
                    }
                }
            }

            // 2. Fronteras exactas de predominancia (con separador NaN en un solo trazo)
            const double qnan = std::numeric_limits<double>::quiet_NaN();
            std::vector<double> bx, by;
            const auto& lineas = calc_.obtener_lineas();
            for (const auto& l : lineas) {
                for (const auto& seg : l.segmentos_recortados) {
                    bx.push_back(seg.pH1); by.push_back(seg.Eh1);
                    bx.push_back(seg.pH2); by.push_back(seg.Eh2);
                    bx.push_back(qnan);    by.push_back(qnan);
                }
            }
            // Complementar con fronteras de celda cuando hay ligandos multicomponente
            if (!calc_.obtener_ligandos().empty()) {
                for (const auto& seg : calc_.obtener_segmentos_frontera()) {
                    bx.push_back(seg.pH1); by.push_back(seg.Eh1);
                    bx.push_back(seg.pH2); by.push_back(seg.Eh2);
                    bx.push_back(qnan);    by.push_back(qnan);
                }
            }
            if (!bx.empty()) {
                ImPlot::SetNextLineStyle(ImVec4(0.95f, 0.96f, 0.98f, 0.95f), 2.0f);
                ImPlot::PlotLine("Fronteras de Equilibrio", bx.data(), by.data(), static_cast<int>(bx.size()));
            }

            // 3. Límites de estabilidad termodinámica del agua (a: O2/H2O, b: H+/H2)
            if (mostrar_agua_) {
                double ph_ends[2] = {calc_.pH_min(), calc_.pH_max()};
                double eh_ox[2]   = {calc_.eh_agua_oxidacion(ph_ends[0]), calc_.eh_agua_oxidacion(ph_ends[1])};
                double eh_red[2]  = {calc_.eh_agua_reduccion(ph_ends[0]), calc_.eh_agua_reduccion(ph_ends[1])};

                ImPlot::SetNextLineStyle(ImVec4(0.35f, 0.85f, 1.0f, 0.90f), 1.8f);
                ImPlot::PlotLine("(a) O2 / H2O (1 atm)", ph_ends, eh_ox, 2);
                ImPlot::SetNextLineStyle(ImVec4(0.35f, 0.85f, 1.0f, 0.90f), 1.8f);
                ImPlot::PlotLine("(b) H+ / H2 (1 atm)", ph_ends, eh_red, 2);
            }

            // 4. Etiquetas centradas en cada polígono de predominancia y puntos triples
            if (mostrar_etiquetas_) {
                for (const auto& poly : calc_.obtener_poligonos()) {
                    std::string tag = poly.clean_formula + " (" + poly.estado + ")";
                    ImPlot::PlotText(tag.c_str(), poly.centroid_pH, poly.centroid_Eh);
                }
                const auto& tps = calc_.obtener_puntos_triples();
                if (!tps.empty()) {
                    std::vector<double> tx, ty;
                    for (const auto& tp : tps) { tx.push_back(tp.pH); ty.push_back(tp.Eh_V); }
                    ImPlot::SetNextMarkerStyle(ImPlotMarker_Diamond, 5.5f,
                        ImVec4(1.0f, 0.85f, 0.20f, 1.0f), 1.5f, ImVec4(0.1f, 0.1f, 0.1f, 1.0f));
                    ImPlot::PlotScatter("Puntos Triples Invariantes", tx.data(), ty.data(), static_cast<int>(tx.size()));
                }
            }

            // 5. Marcador de la sonda de punto de operación (pH, Eh)
            ImPlot::SetNextMarkerStyle(ImPlotMarker_Cross, 9.0f,
                ImVec4(1.0f, 0.92f, 0.20f, 1.0f), 2.5f);
            ImPlot::PlotScatter("Sonda Operativa", &probe_pH_, &probe_Eh_, 1);

            ImPlot::EndPlot();
        }

        // ── Pestañas inferiores: Ecuaciones de Nernst Recortadas y Puntos Triples ──
        if (ImGui::BeginTabBar("##pourbaix_tabs")) {
            if (ImGui::BeginTabItem("Ecuaciones de Equilibrio Nernst (Recortadas)")) {
                const auto& lineas = calc_.obtener_lineas();
                if (ImGui::BeginTable("##nernst_tbl", 5,
                    ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                    ImVec2(0, 0)))
                {
                    ImGui::TableSetupColumn("Especie 1", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                    ImGui::TableSetupColumn("Especie 2", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                    ImGui::TableSetupColumn("E° (V) / pH_eq", ImGuiTableColumnFlags_WidthFixed, 115.0f);
                    ImGui::TableSetupColumn("dEh/dpH (V/pH)", ImGuiTableColumnFlags_WidthFixed, 105.0f);
                    ImGui::TableSetupColumn("Semirreacción / Equilibrio Hidrolítico", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableHeadersRow();

                    for (const auto& l : lineas) {
                        ImGui::TableNextRow();
                        ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(l.sp_ox.c_str());
                        ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(l.sp_red.c_str());
                        ImGui::TableSetColumnIndex(2);
                        if (l.es_vertical) ImGui::Text("pH_eq = %.2f", l.pH_eq);
                        else               ImGui::Text("E° = %+.3f V", l.E0);
                        ImGui::TableSetColumnIndex(3);
                        if (l.es_vertical) ImGui::TextDisabled("Vertical");
                        else               ImGui::Text("%+.4f", l.dEdpH);
                        ImGui::TableSetColumnIndex(4);
                        ImGui::TextUnformatted(l.ecuacion.c_str());
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }

            if (ImGui::BeginTabItem("Puntos Triples Invariantes")) {
                const auto& tps = calc_.obtener_puntos_triples();
                if (ImGui::BeginTable("##tp_tbl", 5,
                    ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                    ImVec2(0, 0)))
                {
                    ImGui::TableSetupColumn("pH", ImGuiTableColumnFlags_WidthFixed, 75.0f);
                    ImGui::TableSetupColumn("Eh (V)", ImGuiTableColumnFlags_WidthFixed, 85.0f);
                    ImGui::TableSetupColumn("Fase 1", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Fase 2", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Fase 3", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableHeadersRow();

                    for (const auto& tp : tps) {
                        ImGui::TableNextRow();
                        ImGui::TableSetColumnIndex(0); ImGui::Text("%.2f", tp.pH);
                        ImGui::TableSetColumnIndex(1); ImGui::Text("%+.3f", tp.Eh_V);
                        ImGui::TableSetColumnIndex(2); ImGui::TextUnformatted(tp.sp1.c_str());
                        ImGui::TableSetColumnIndex(3); ImGui::TextUnformatted(tp.sp2.c_str());
                        ImGui::TableSetColumnIndex(4); ImGui::TextUnformatted(tp.sp3.c_str());
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }
    }
    ImGui::EndChild();
}

} // namespace cma::interfaz
