#include "utils/i18n.h"

PanelDiagramasLpp::PanelDiagramasLpp(const domain::ThermoRepository& repository)
    : repo_(repository), service_(&repository)
{
    cargar_preset(0);
}

void PanelDiagramasLpp::cargar_preset(int idx) {
    preset_idx_ = idx;
    use_celsius_ = true;
    if (idx == 0) {
        std::strncpy(main_element_, "Cu", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "SO2", sizeof(gas2_element_) - 1);
        T_input_ = 700.0;
        x_min_ = -20.0; x_max_ = 0.0;
        y_min_ = -15.0; y_max_ = 2.0;
        probe_x_ = -7.5; probe_y_ = -1.0;
    } else if (idx == 1) {
        std::strncpy(main_element_, "Fe", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "SO2", sizeof(gas2_element_) - 1);
        T_input_ = 700.0;
        x_min_ = -25.0; x_max_ = 0.0;
        y_min_ = -15.0; y_max_ = 2.0;
        probe_x_ = -10.0; probe_y_ = -1.0;
    } else if (idx == 2) {
        std::strncpy(main_element_, "Ni", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "SO2", sizeof(gas2_element_) - 1);
        T_input_ = 750.0;
        x_min_ = -22.0; x_max_ = 0.0;
        y_min_ = -15.0; y_max_ = 2.0;
        probe_x_ = -9.0; probe_y_ = -1.0;
    } else if (idx == 3) {
        std::strncpy(main_element_, "Zn", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "SO2", sizeof(gas2_element_) - 1);
        T_input_ = 850.0;
        x_min_ = -24.0; x_max_ = 0.0;
        y_min_ = -15.0; y_max_ = 2.0;
        probe_x_ = -8.0; probe_y_ = -1.0;
    } else {
        std::strncpy(main_element_, "Pb", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "SO2", sizeof(gas2_element_) - 1);
        T_input_ = 700.0;
        x_min_ = -24.0; x_max_ = 0.0;
        y_min_ = -15.0; y_max_ = 2.0;
        probe_x_ = -9.0; probe_y_ = -1.0;
    }
    actualizar_especies_sistema();
    recalcular();
}

void PanelDiagramasLpp::actualizar_especies_sistema() {
    candidates_ = service_.filtrar_especies_sistema(main_element_, gas1_element_, gas2_element_, true);
}

void PanelDiagramasLpp::recalcular() {
    if (candidates_.empty()) {
        actualizar_especies_sistema();
    }
    domain::LppDiagramRequest req;
    req.main_element = main_element_;
    req.gas1_formula = gas1_element_;
    req.gas2_formula = gas2_element_;
    req.temperature_k = use_celsius_ ? (T_input_ + 273.15) : T_input_;
    req.x_min = x_min_;
    req.x_max = x_max_;
    req.y_min = y_min_;
    req.y_max = y_max_;
    req.probe_log_p1 = probe_x_;
    req.probe_log_p2 = probe_y_;
    req.candidates = candidates_;

    result_ = service_.calcular_lpp(req);
    calculado_ = result_.ok;
    status_msg_ = result_.message;
}

void PanelDiagramasLpp::renderizar() {
    ImGui::Columns(2, "LppLayout", true);
    ImGui::SetColumnWidth(0, 355.0f);

    // ── Columna Izquierda: Configuración del Sistema Lpp ──
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f),
                       tr("Parámetros del Diagrama Lpp (Kellogg Vectorial)"));
    ImGui::Separator();

    const char* presets[] = {
        "1. Cu - O2 - SO2 (Tostación/Fusión Cobre 700 °C)",
        "2. Fe - O2 - SO2 (Tostación Sulfatante Hierro 700 °C)",
        "3. Ni - O2 - SO2 (Tostación Níquel 750 °C)",
        "4. Zn - O2 - SO2 (Tostación Esfalerita 850 °C)",
        "5. Pb - O2 - SO2 (Sinterización Galena 700 °C)"
    };
    ImGui::SetNextItemWidth(-1.0f);
    if (ImGui::Combo("##lpp_preset", &preset_idx_, presets, 5)) {
        cargar_preset(preset_idx_);
    }

    bool sys_changed = false;
    sys_changed |= ImGui::InputText(tr("Main Element"), main_element_, sizeof(main_element_));
    sys_changed |= ImGui::InputText(tr("Gas 1 (X axis)"), gas1_element_, sizeof(gas1_element_));
    sys_changed |= ImGui::InputText(tr("Gas 2 (Y axis)"), gas2_element_, sizeof(gas2_element_));
    if (sys_changed || ImGui::Button(tr("Actualizar Especies del Sistema"), ImVec2(-1, 24))) {
        actualizar_especies_sistema();
        recalcular();
    }

    ImGui::Spacing();
    if (ImGui::SmallButton(tr("Seleccionar Todas"))) {
        for (auto& c : candidates_) c.selected = true;
        recalcular();
    }
    ImGui::SameLine();
    if (ImGui::SmallButton(tr("Deseleccionar Todas"))) {
        for (auto& c : candidates_) c.selected = false;
    }

    ImGui::BeginChild("SpeciesListLpp", ImVec2(0, 185), true);
    if (ImGui::BeginTable("LppSpeciesTable", 3,
            ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Act", ImGuiTableColumnFlags_WidthFixed, 32.0f);
        ImGui::TableSetupColumn("Especie / Fase", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Actividad a_i", ImGuiTableColumnFlags_WidthFixed, 82.0f);
        ImGui::TableHeadersRow();

        for (size_t i = 0; i < candidates_.size(); ++i) {
            auto& c = candidates_[i];
            ImGui::PushID(static_cast<int>(i));
            ImGui::TableNextRow();
            ImGui::TableNextColumn();
            if (ImGui::Checkbox("##sel", &c.selected)) recalcular();
            ImGui::TableNextColumn();
            ImGui::Text("%s", c.formula.c_str());
            ImGui::TableNextColumn();
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputDouble("##act", &c.activity, 0, 0, "%.3f")) {
                c.activity = std::clamp(c.activity, 1e-9, 10.0);
                recalcular();
            }
            ImGui::PopID();
        }
        ImGui::EndTable();
    }
    ImGui::EndChild();

    ImGui::Spacing();
    ImGui::Text(tr("Condiciones Isotérmicas y Ejes Log10(P/atm):"));
    if (ImGui::Checkbox(tr("Temperatura en °C"), &use_celsius_)) {
        T_input_ = use_celsius_ ? (T_input_ - 273.15) : (T_input_ + 273.15);
    }
    ImGui::SameLine();
    ImGui::SetNextItemWidth(-1.0f);
    if (ImGui::InputDouble("##t_lpp", &T_input_, 0, 0, "%.1f")) recalcular();

    bool ax_changed = false;
    ax_changed |= ImGui::InputDouble("X Min", &x_min_, 0, 0, "%.1f");
    ax_changed |= ImGui::InputDouble("X Max", &x_max_, 0, 0, "%.1f");
    ax_changed |= ImGui::InputDouble("Y Min", &y_min_, 0, 0, "%.1f");
    ax_changed |= ImGui::InputDouble("Y Max", &y_max_, 0, 0, "%.1f");
    if (ax_changed) recalcular();

    ImGui::Checkbox(tr("Rellenar Polígonos"), &fill_polygons_);
    ImGui::SameLine();
    ImGui::Checkbox(tr("Puntos Triples"), &show_triple_points_);

    if (ImGui::Button(tr("Dibujar Diagrama Lpp"), ImVec2(-1, 30))) {
        recalcular();
    }
    if (!status_msg_.empty()) {
        ImGui::TextWrapped("%s", status_msg_.c_str());
    }

    ImGui::NextColumn();

    // ── Columna Derecha: Gráfico Vectorial + Tablas de Líneas y Puntos Triples ──
    if (calculado_) {
        if (ImGui::BeginTabBar("lpp_right_tabs")) {
            if (ImGui::BeginTabItem(tr("1. Diagrama de Estabilidad Lpp (Vectorial)"))) {
                ImGui::Text("Sonda: log p(%s) = %.2f, log p(%s) = %.2f  ->  Fase Predominante: %s (G/n_M = %.2f kJ/mol)",
                            gas1_element_, probe_x_, gas2_element_, probe_y_,
                            result_.probe_predominant_phase.c_str(), result_.probe_min_potential_kj);

                if (ImPlot::BeginPlot(result_.system_title.c_str(), ImVec2(-1, -1))) {
                    char xlabel[64], ylabel[64];
                    std::snprintf(xlabel, sizeof(xlabel), "log10 p(%s) [atm]", gas1_element_);
                    std::snprintf(ylabel, sizeof(ylabel), "log10 p(%s) [atm]", gas2_element_);
                    ImPlot::SetupAxes(xlabel, ylabel);
                    ImPlot::SetupAxisLimits(ImAxis_X1, x_min_, x_max_, ImGuiCond_Once);
                    ImPlot::SetupAxisLimits(ImAxis_Y1, y_min_, y_max_, ImGuiCond_Once);

                    if (ImPlot::IsPlotHovered() && ImGui::IsMouseClicked(ImGuiMouseButton_Left)) {
                        const ImPlotPoint mp = ImPlot::GetPlotMousePos();
                        probe_x_ = mp.x;
                        probe_y_ = mp.y;
                        recalcular();
                    }

                    for (size_t idx = 0; idx < result_.polygons.size(); ++idx) {
                        const auto& pf = result_.polygons[idx];
                        if (pf.vertices.empty()) continue;

                        std::vector<double> xs, ys;
                        for (const auto& v : pf.vertices) {
                            xs.push_back(v.first);
                            ys.push_back(v.second);
                        }
                        xs.push_back(xs.front());
                        ys.push_back(ys.front());

                        if (fill_polygons_) {
                            ImDrawList* dl = ImPlot::GetPlotDrawList();
                            std::vector<ImVec2> screen_pts;
                            screen_pts.reserve(pf.vertices.size());
                            for (const auto& v : pf.vertices) {
                                screen_pts.push_back(ImPlot::PlotToPixels(v.first, v.second));
                            }
                            ImVec4 col_f = color_from_id(static_cast<int>(idx) + 1);
                            col_f.w = 0.42f;
                            dl->AddConvexPolyFilled(screen_pts.data(), static_cast<int>(screen_pts.size()),
                                                    ImGui::ColorConvertFloat4ToU32(col_f));
                        }

                        ImPlot::SetNextLineStyle(ImVec4(0.10f, 0.12f, 0.16f, 1.0f), 1.8f);
                        ImPlot::PlotLine(pf.formula.c_str(), xs.data(), ys.data(), static_cast<int>(xs.size()));

                        ImPlot::PushStyleColor(ImPlotCol_InlayText, ImVec4(0.05f, 0.05f, 0.08f, 1.0f));
                        ImPlot::PlotText(pf.formula.c_str(), pf.centroid_x, pf.centroid_y);
                        ImPlot::PopStyleColor();
                    }

                    if (show_triple_points_ && !result_.triple_points.empty()) {
                        std::vector<double> tx, ty;
                        for (const auto& tp : result_.triple_points) {
                            tx.push_back(tp.log_p_gas1);
                            ty.push_back(tp.log_p_gas2);
                        }
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Diamond, 6.0f, ImVec4(0.85f, 0.15f, 0.15f, 1.0f), 1.5f);
                        ImPlot::PlotScatter("Triple Points", tx.data(), ty.data(), static_cast<int>(tx.size()));
                    }

                    // Punto de operación (sonda)
                    ImPlot::SetNextMarkerStyle(ImPlotMarker_Cross, 8.0f, ImVec4(0.95f, 0.25f, 0.10f, 1.0f), 2.0f);
                    ImPlot::PlotScatter("Operating Point", &probe_x_, &probe_y_, 1);

                    ImPlot::EndPlot();
                }
                ImGui::EndTabItem();
            }

            if (ImGui::BeginTabItem(tr("2. Fronteras de Equilibrio, Puntos Triples y Exportación"))) {
                ImGui::SetNextItemWidth(240.0f);
                ImGui::InputText("##lpp_csv", csv_export_path_, sizeof(csv_export_path_));
                ImGui::SameLine();
                if (ImGui::Button(tr("Exportar Lpp a CSV"))) {
                    if (service_.exportar_lpp_csv(csv_export_path_, result_)) {
                        status_msg_ = std::string("Diagrama Lpp exportado a ") + csv_export_path_;
                    }
                }

                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f),
                                   tr("Puntos Triples Invariantes (Coexistencia de 3 Fases Condensadas):"));
                if (ImGui::BeginTable("lpp_tp_tbl", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg, ImVec2(0, 130))) {
                    ImGui::TableSetupColumn("Fase 1");
                    ImGui::TableSetupColumn("Fase 2");
                    ImGui::TableSetupColumn("Fase 3");
                    ImGui::TableSetupColumn("log10 p(Gas 1)");
                    ImGui::TableSetupColumn("log10 p(Gas 2)");
                    ImGui::TableHeadersRow();
                    for (const auto& tp : result_.triple_points) {
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::TextUnformatted(tp.phase_a.c_str());
                        ImGui::TableNextColumn(); ImGui::TextUnformatted(tp.phase_b.c_str());
                        ImGui::TableNextColumn(); ImGui::TextUnformatted(tp.phase_c.c_str());
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", tp.log_p_gas1);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", tp.log_p_gas2);
                    }
                    ImGui::EndTable();
                }

                ImGui::Spacing();
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f),
                                   tr("Líneas de Equilibrio Bifásico y Reacciones Base:"));
                if (ImGui::BeginTable("lpp_bnd_tbl", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                    ImGui::TableSetupScrollFreeze(0, 1);
                    ImGui::TableSetupColumn("Equilibrio");
                    ImGui::TableSetupColumn("(X1, Y1)");
                    ImGui::TableSetupColumn("(X2, Y2)");
                    ImGui::TableSetupColumn("Pendiente dy/dx");
                    ImGui::TableSetupColumn("log K(T)");
                    ImGui::TableHeadersRow();
                    for (const auto& b : result_.boundaries) {
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::TextUnformatted(b.reaction_equation.c_str());
                        ImGui::TableNextColumn(); ImGui::Text("(%.2f, %.2f)", b.x1, b.y1);
                        ImGui::TableNextColumn(); ImGui::Text("(%.2f, %.2f)", b.x2, b.y2);
                        ImGui::TableNextColumn();
                        if (std::abs(b.slope_dy_dx) > 1e6) ImGui::Text("Vertical (inf)");
                        else ImGui::Text("%.3f", b.slope_dy_dx);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", b.log_k);
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }
    } else {
        ImGui::TextColored(ImVec4(0.5f, 0.5f, 0.5f, 1.0f), tr("Seleccione un sistema y pulse Dibujar Diagrama Lpp."));
    }
    ImGui::Columns(1);
}
