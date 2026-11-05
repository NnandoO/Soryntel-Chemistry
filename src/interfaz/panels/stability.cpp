#include "shared.hpp"
#include "panel_estabilidad.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>

namespace cma { namespace interfaz {

// Paleta de colores suaves y contrastados para dominios de estabilidad (Tpp / Lpp)
ImVec4 color_from_id(int id) {
    static const ImVec4 kPalette[] = {
        ImVec4(0.32f, 0.62f, 0.88f, 1.0f),
        ImVec4(0.88f, 0.48f, 0.30f, 1.0f),
        ImVec4(0.38f, 0.76f, 0.48f, 1.0f),
        ImVec4(0.84f, 0.72f, 0.28f, 1.0f),
        ImVec4(0.68f, 0.48f, 0.84f, 1.0f),
        ImVec4(0.28f, 0.76f, 0.76f, 1.0f),
        ImVec4(0.88f, 0.42f, 0.60f, 1.0f),
        ImVec4(0.62f, 0.74f, 0.34f, 1.0f),
        ImVec4(0.75f, 0.56f, 0.42f, 1.0f),
        ImVec4(0.48f, 0.54f, 0.78f, 1.0f)
    };
    const int idx = std::abs(id) % 10;
    return kPalette[idx];
}

// ═══════════════════════════════════════════════════════════════════════════
// PanelEstabilidad (Tpp Diagram Module: p-p & T-p Gibbs Minimization)
// ═══════════════════════════════════════════════════════════════════════════

PanelEstabilidad::PanelEstabilidad(const domain::ThermoRepository& repository)
    : repo_(repository), service_(&repository)
{
    cargar_preset(0);
}

void PanelEstabilidad::cargar_preset(int idx) {
    preset_idx_ = idx;
    if (idx == 0) {
        // Cu - O2 - SO2 (p-p a 973.15 K)
        std::strncpy(main_element_, "Cu", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "SO2", sizeof(gas2_element_) - 1);
        xaxis_var_ = 1;
        yaxis_var_ = 2;
        fixed_val_ = 973.15;
        x_min_ = -20.0; x_max_ = 0.0;
        y_min_ = -15.0; y_max_ = 2.0;
        probe_x_ = -7.5; probe_y_ = -1.0;
    } else if (idx == 1) {
        // Cu - O2 - SO2 (T-p: Temperatura K en X vs log p(O2) en Y a log p(SO2) = -1.0)
        std::strncpy(main_element_, "Cu", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "SO2", sizeof(gas2_element_) - 1);
        xaxis_var_ = 0; // Temperature (K)
        yaxis_var_ = 1; // log p(O2)
        fixed_val_ = -1.0; // log p(SO2) fijo = -1.0 (0.1 atm)
        x_min_ = 600.0; x_max_ = 1500.0;
        y_min_ = -20.0; y_max_ = 0.0;
        probe_x_ = 1000.0; probe_y_ = -6.0;
    } else if (idx == 2) {
        // Fe - O2 - SO2 (T-p: Temperatura K en X vs log p(O2) en Y a log p(SO2) = -1.0)
        std::strncpy(main_element_, "Fe", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "SO2", sizeof(gas2_element_) - 1);
        xaxis_var_ = 0;
        yaxis_var_ = 1;
        fixed_val_ = -1.0;
        x_min_ = 600.0; x_max_ = 1600.0;
        y_min_ = -25.0; y_max_ = 0.0;
        probe_x_ = 1050.0; probe_y_ = -10.0;
    } else if (idx == 3) {
        // Zn - O2 - SO2 (p-p a 1100 K)
        std::strncpy(main_element_, "Zn", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "SO2", sizeof(gas2_element_) - 1);
        xaxis_var_ = 1;
        yaxis_var_ = 2;
        fixed_val_ = 1100.0;
        x_min_ = -24.0; x_max_ = 0.0;
        y_min_ = -15.0; y_max_ = 2.0;
        probe_x_ = -10.0; probe_y_ = -1.0;
    } else {
        // Fe - O2 - Cl2 (Cloración a 1000 K)
        std::strncpy(main_element_, "Fe", sizeof(main_element_) - 1);
        std::strncpy(gas1_element_, "O2", sizeof(gas1_element_) - 1);
        std::strncpy(gas2_element_, "Cl2", sizeof(gas2_element_) - 1);
        xaxis_var_ = 1;
        yaxis_var_ = 2;
        fixed_val_ = 1000.0;
        x_min_ = -30.0; x_max_ = 0.0;
        y_min_ = -25.0; y_max_ = 0.0;
        probe_x_ = -15.0; probe_y_ = -5.0;
    }
    actualizar_especies_sistema();
    recalcular();
}

void PanelEstabilidad::actualizar_especies_sistema() {
    candidates_ = service_.filtrar_especies_sistema(main_element_, gas1_element_, gas2_element_, true);
}

void PanelEstabilidad::recalcular() {
    if (candidates_.empty()) {
        actualizar_especies_sistema();
    }
    if (xaxis_var_ == yaxis_var_) {
        yaxis_var_ = (xaxis_var_ + 1) % 3;
    }

    last_req_.main_element = main_element_;
    last_req_.gas1_formula = gas1_element_;
    last_req_.gas2_formula = gas2_element_;
    last_req_.x_var = static_cast<domain::StabilityAxisVar>(xaxis_var_);
    last_req_.y_var = static_cast<domain::StabilityAxisVar>(yaxis_var_);
    last_req_.fixed_value = fixed_val_;
    last_req_.x_min = x_min_;
    last_req_.x_max = x_max_;
    last_req_.y_min = y_min_;
    last_req_.y_max = y_max_;
    last_req_.rows = std::clamp(grid_res_, 40, 260);
    last_req_.cols = std::clamp(grid_res_, 40, 260);
    last_req_.probe_x = probe_x_;
    last_req_.probe_y = probe_y_;
    last_req_.candidates = candidates_;

    result_ = service_.calcular_tpp(last_req_);
    calculado_ = result_.ok;
    status_msg_ = result_.message;
}

void PanelEstabilidad::renderizar() {
    ImGui::Columns(2, "PredominanceLayout", true);
    ImGui::SetColumnWidth(0, 355.0f);

    // ── Columna Izquierda: Parámetros del Diagrama Tpp ──
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f),
                       tr("Parámetros del Diagrama Tpp (T-p y p-p)"));
    ImGui::Separator();

    const char* presets[] = {
        "1. Cu - O2 - SO2 (p-p Isotérmico 973 K)",
        "2. Cu - O2 - SO2 (T-p: T vs log pO2 a pSO2=0.1 atm)",
        "3. Fe - O2 - SO2 (T-p: T vs log pO2 a pSO2=0.1 atm)",
        "4. Zn - O2 - SO2 (p-p Tostación Esfalerita 1100 K)",
        "5. Fe - O2 - Cl2 (p-p Cloración Metalúrgica 1000 K)"
    };
    ImGui::SetNextItemWidth(-1.0f);
    if (ImGui::Combo("##tpp_preset", &preset_idx_, presets, 5)) {
        cargar_preset(preset_idx_);
    }

    bool sys_changed = false;
    sys_changed |= ImGui::InputText(tr("Main Element"), main_element_, sizeof(main_element_));
    sys_changed |= ImGui::InputText(tr("Gas 1 (ej. O2)"), gas1_element_, sizeof(gas1_element_));
    sys_changed |= ImGui::InputText(tr("Gas 2 (ej. SO2 / Cl2)"), gas2_element_, sizeof(gas2_element_));
    if (sys_changed || ImGui::Button(tr("Filtrar Especies del Sistema"), ImVec2(-1, 24))) {
        actualizar_especies_sistema();
        recalcular();
    }

    if (ImGui::SmallButton(tr("Select All Filtered"))) {
        for (auto& c : candidates_) c.selected = true;
        recalcular();
    }
    ImGui::SameLine();
    if (ImGui::SmallButton(tr("Deseleccionar Todas"))) {
        for (auto& c : candidates_) c.selected = false;
    }

    ImGui::BeginChild("SpeciesListTpp", ImVec2(0, 175), true);
    if (ImGui::BeginTable("TppSpeciesTable", 3,
            ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Sel", ImGuiTableColumnFlags_WidthFixed, 32.0f);
        ImGui::TableSetupColumn("Fórmula", ImGuiTableColumnFlags_WidthStretch);
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
    ImGui::Text(tr("Configurar Ejes (T-p o p-p):"));
    const char* var_names[] = {"Temperature (K)", "log10 p(Gas 1)", "log10 p(Gas 2)"};

    bool ax_changed = false;
    ax_changed |= ImGui::Combo("X Axis", &xaxis_var_, var_names, 3);
    ax_changed |= ImGui::InputDouble("X Min", &x_min_, 0, 0, "%.1f");
    ax_changed |= ImGui::InputDouble("X Max", &x_max_, 0, 0, "%.1f");

    ax_changed |= ImGui::Combo("Y Axis", &yaxis_var_, var_names, 3);
    ax_changed |= ImGui::InputDouble("Y Min", &y_min_, 0, 0, "%.1f");
    ax_changed |= ImGui::InputDouble("Y Max", &y_max_, 0, 0, "%.1f");

    int fixed_var = 0;
    if (xaxis_var_ != 0 && yaxis_var_ != 0) fixed_var = 0;
    else if (xaxis_var_ != 1 && yaxis_var_ != 1) fixed_var = 1;
    else fixed_var = 2;

    char fixed_label[64];
    std::snprintf(fixed_label, sizeof(fixed_label), "Fixed: %s", var_names[fixed_var]);
    ax_changed |= ImGui::InputDouble(fixed_label, &fixed_val_, 0, 0, "%.2f");
    ImGui::Checkbox(tr("Superponer Líneas de Contorno"), &show_contour_lines_);
    if (ax_changed) recalcular();

    if (ImGui::Button(tr("Dibujar Diagrama Tpp"), ImVec2(-1, 30))) {
        recalcular();
    }
    if (!status_msg_.empty()) {
        ImGui::TextWrapped("%s", status_msg_.c_str());
    }

    ImGui::NextColumn();

    // ── Columna Derecha: Mapa de Áreas Tpp + Contornos + Exportación CSV ──
    if (calculado_ && !result_.legend.empty()) {
        static bool cmap_added = false;
        if (!cmap_added) {
            std::vector<ImU32> palette(256);
            for (int i = 0; i < 256; ++i) {
                palette[i] = ImGui::ColorConvertFloat4ToU32(color_from_id(i));
            }
            ImPlot::AddColormap("TppPalette", palette.data(), 256, true);
            cmap_added = true;
        }

        ImGui::Text("Sonda (X=%.2f, Y=%.2f) -> Fase Estable: %s (G_norm = %.2f kJ/mol-%s)",
                    probe_x_, probe_y_, result_.probe_predominant_phase.c_str(),
                    result_.probe_min_potential_kj, main_element_);
        ImGui::SameLine();
        if (ImGui::SmallButton(tr("Exportar CSV"))) {
            if (service_.exportar_tpp_csv(csv_export_path_, last_req_, result_)) {
                status_msg_ = std::string("Diagrama Tpp exportado a ") + csv_export_path_;
            }
        }

        const int n_phases = std::max(1, static_cast<int>(result_.legend.size()));
        std::vector<double> mapped_grid(result_.phase_index_grid.size(), 0.0);
        for (size_t i = 0; i < result_.phase_index_grid.size(); ++i) {
            mapped_grid[i] = static_cast<double>(result_.phase_index_grid[i]);
        }

        ImPlot::PushColormap("TppPalette");
        if (ImPlot::BeginPlot(result_.system_title.c_str(), ImVec2(-1, -48))) {
            ImPlot::SetupAxes(var_names[xaxis_var_], var_names[yaxis_var_]);
            ImPlot::SetupAxisLimits(ImAxis_X1, x_min_, x_max_, ImGuiCond_Once);
            ImPlot::SetupAxisLimits(ImAxis_Y1, y_min_, y_max_, ImGuiCond_Once);

            if (ImPlot::IsPlotHovered() && ImGui::IsMouseClicked(ImGuiMouseButton_Left)) {
                const ImPlotPoint mp = ImPlot::GetPlotMousePos();
                probe_x_ = mp.x;
                probe_y_ = mp.y;
                recalcular();
            }

            ImPlot::PlotHeatmap("##stability", mapped_grid.data(), result_.rows, result_.cols,
                                0, 255, nullptr, ImPlotPoint(x_min_, y_min_), ImPlotPoint(x_max_, y_max_));

            if (show_contour_lines_ && !result_.contours.empty()) {
                ImDrawList* dl = ImPlot::GetPlotDrawList();
                const ImU32 line_col = IM_COL32(25, 25, 30, 210);
                for (const auto& seg : result_.contours) {
                    const ImVec2 p1 = ImPlot::PlotToPixels(seg.x1, seg.y1);
                    const ImVec2 p2 = ImPlot::PlotToPixels(seg.x2, seg.y2);
                    dl->AddLine(p1, p2, line_col, 1.5f);
                }
            }

            // Punto de operación (sonda)
            ImPlot::SetNextMarkerStyle(ImPlotMarker_Cross, 8.0f, ImVec4(1.0f, 1.0f, 1.0f, 1.0f), 2.0f);
            ImPlot::PlotScatter("Probe", &probe_x_, &probe_y_, 1);

            ImPlot::EndPlot();
        }
        ImPlot::PopColormap();

        // Leyenda con fracción de área (%)
        ImGui::BeginChild("LegendTpp", ImVec2(0, 0), false, ImGuiWindowFlags_HorizontalScrollbar);
        for (int i = 0; i < n_phases; ++i) {
            const auto& entry = result_.legend[i];
            const ImVec4 c = color_from_id(entry.color_index);
            ImGui::ColorButton(entry.formula.c_str(), c);
            ImGui::SameLine();
            ImGui::Text("%s (%.1f%%)", entry.formula.c_str(), entry.area_fraction_pct);
            ImGui::SameLine(0, 18);
        }
        ImGui::EndChild();
    } else {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.5f, 0.5f, 1.0f),
                           "Select species and click Draw Diagram.");
    }

    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════
#include "../lpp_impl.cpp"
// ═══════════════════════════════════════════════════════════════════════════

}} // namespace cma::interfaz
