#include "shared.hpp"
#include "interfaz/panels/panel_perdida_calor.hpp"
#include "interfaz/ui_css_style.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>

namespace cma { namespace interfaz {

namespace {

ImU32 temperature_to_color(double temp_c, double t_min_c, double t_max_c) {
    const double span = std::max(1.0, t_max_c - t_min_c);
    const float t = static_cast<float>(std::clamp((temp_c - t_min_c) / span, 0.0, 1.0));
    // Gradiente térmico: azul frío (0) -> cian -> amarillo -> naranja -> rojo incandescente (1)
    float r = std::clamp(1.8f * t, 0.15f, 1.0f);
    float g = std::clamp(1.2f * (1.0f - std::abs(2.0f * t - 1.0f)), 0.15f, 0.90f);
    float b = std::clamp(1.5f * (1.0f - t), 0.10f, 1.0f);
    return IM_COL32(static_cast<int>(r * 255), static_cast<int>(g * 255), static_cast<int>(b * 255), 220);
}

} // namespace

PanelPerdidaCalor::PanelPerdidaCalor() {
    inicializar_valores_defecto();
}

void PanelPerdidaCalor::inicializar_valores_defecto() {
    service_.load_preset(0);
    selected_col_ = 1;
    gas_rad_res_ = service_.calculate_gas_radiation(gas_rad_in_);
    status_banner_ = service_.active_sheet().status_message;
    status_ok_ = true;
}

void PanelPerdidaCalor::renderizar() {
    renderizar_barra_superior();
    renderizar_pestanas_hojas();

    auto& sheet = service_.active_sheet();
    if (selected_col_ >= static_cast<int>(sheet.columns.size())) {
        selected_col_ = std::max(0, static_cast<int>(sheet.columns.size()) - 1);
    }

    if (ImGui::BeginTabBar("heat_loss_main_tabs", ImGuiTabBarFlags_None)) {
        if (ImGui::BeginTabItem(tr("1. Hoja de Capas y Solver (Worksheet)"))) {
            active_main_tab_ = 0;
            renderizar_hoja_calculo(sheet);
            ImGui::Spacing();
            renderizar_seccion_transversal(sheet);
            ImGui::Spacing();
            ImGui::Separator();
            ImGui::Spacing();
            renderizar_panel_inferior_controles(sheet);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem(tr("2. Perfil de Temperatura y Tabla"))) {
            active_main_tab_ = 1;
            renderizar_pestana_perfil_y_tabla(sheet);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem(tr("3. Bases de Datos (Conducción / Convección / Radiación)"))) {
            active_main_tab_ = 2;
            renderizar_pestana_bases_datos(sheet);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem(tr("4. Optimizador de Espesor y Radiación de Gases"))) {
            active_main_tab_ = 3;
            renderizar_pestana_calculadoras(sheet);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem(tr("5. Resumen del Reactor e Integración Balance/Sim"))) {
            active_main_tab_ = 4;
            renderizar_pestana_resumen_reactor();
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }

    renderizar_popups_flotantes(sheet);
}

void PanelPerdidaCalor::renderizar_barra_superior() {
    ImGui::Text(tr("Caso / Preset HSC:"));
    ImGui::SameLine();
    ImGui::SetNextItemWidth(320.0f);
    const char* presets[] = {
        "1. Horno de Fusión HSC Ref. (Muro, Techo y Fondo)",
        "2. Revestimiento Congelado Cobre Refrigerado (Freeze Lining)",
        "3. Horno Rotatorio (Mag-Cromo + IFB-26 + Coraza)",
        "4. Tubería de Vapor Aislada DN300 (Silicato Ca + Lana Mineral)"
    };
    if (ImGui::Combo("##hl_preset", &preset_idx_, presets, 4)) {
        service_.load_preset(preset_idx_);
        selected_col_ = 1;
        status_banner_ = service_.active_sheet().status_message;
        status_ok_ = true;
    }

    ImGui::SameLine();
    auto& sheet = service_.active_sheet();
    if (ImGui::Button(tr("+ Capa Sólida"))) {
        cma::domain::ThermalColumn c;
        c.name = "New Brick Layer";
        c.type = cma::domain::ThermalColumnType::SolidLayer;
        c.material = "Fireclay brick (Chamotte)";
        c.thickness_cm = 12.0;
        c.grid_points = 10;
        if (const auto* m = service_.catalog().find_conduction(c.material)) {
            c.conductivity_w_mk = m->evaluate_k(500.0);
        }
        const int ins_pos = std::clamp(selected_col_ + 1, 1, std::max(1, static_cast<int>(sheet.columns.size()) - 1));
        sheet.columns.insert(sheet.columns.begin() + ins_pos, c);
        selected_col_ = ins_pos;
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("+ Contacto"))) {
        cma::domain::ThermalColumn c;
        c.name = "Contact Gap";
        c.type = cma::domain::ThermalColumnType::ContactLayer;
        c.material = "Contact layer N/A";
        c.resistance_c_kw = 5.0;
        c.fix_resistance = true;
        const int ins_pos = std::clamp(selected_col_ + 1, 1, std::max(1, static_cast<int>(sheet.columns.size()) - 1));
        sheet.columns.insert(sheet.columns.begin() + ins_pos, c);
        selected_col_ = ins_pos;
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("+ Enclosure"))) {
        cma::domain::ThermalColumn c;
        c.name = "Air Gap";
        c.type = cma::domain::ThermalColumnType::Enclosure;
        c.material = "Air (gas) (p = 1 bar)";
        c.thickness_cm = 5.0;
        c.grid_points = 6;
        const int ins_pos = std::clamp(selected_col_ + 1, 1, std::max(1, static_cast<int>(sheet.columns.size()) - 1));
        sheet.columns.insert(sheet.columns.begin() + ins_pos, c);
        selected_col_ = ins_pos;
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    if (ImGui::Button("<-") && selected_col_ > 0 && selected_col_ < static_cast<int>(sheet.columns.size())) {
        std::swap(sheet.columns[selected_col_], sheet.columns[selected_col_ - 1]);
        --selected_col_;
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    if (ImGui::Button("->") && selected_col_ >= 0 && selected_col_ + 1 < static_cast<int>(sheet.columns.size())) {
        std::swap(sheet.columns[selected_col_], sheet.columns[selected_col_ + 1]);
        ++selected_col_;
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Eliminar Col.")) && sheet.columns.size() > 2 &&
        selected_col_ >= 0 && selected_col_ < static_cast<int>(sheet.columns.size())) {
        sheet.columns.erase(sheet.columns.begin() + selected_col_);
        selected_col_ = std::clamp(selected_col_, 0, static_cast<int>(sheet.columns.size()) - 1);
        service_.solve_active_sheet();
    }

    if (!status_banner_.empty()) {
        ImVec4 col = status_ok_ ? ImVec4(0.15f, 0.55f, 0.20f, 1.0f) : ImVec4(0.80f, 0.20f, 0.15f, 1.0f);
        ImGui::TextColored(col, "%s", status_banner_.c_str());
    }
}

void PanelPerdidaCalor::renderizar_pestanas_hojas() {
    auto& sheets = service_.sheets();
    const int active_idx = service_.active_sheet_index();

    ImGui::Text(tr("Zonas del Equipo:"));
    ImGui::SameLine();
    for (int i = 0; i < static_cast<int>(sheets.size()); ++i) {
        ImGui::PushID(i);
        const bool is_active = (i == active_idx);
        if (is_active) {
            ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.72f, 0.22f, 0.18f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
        }
        char btn_lbl[96];
        std::snprintf(btn_lbl, sizeof(btn_lbl), "%s (%.2f kW)", sheets[i].name.c_str(), sheets[i].total_heat_flow_kw);
        if (ImGui::Button(btn_lbl)) {
            service_.set_active_sheet_index(i);
        }
        if (is_active) {
            ImGui::PopStyleColor(2);
        }
        ImGui::SameLine();
        ImGui::PopID();
    }

    ImGui::SetNextItemWidth(120.0f);
    ImGui::InputText("##new_sheet_name", new_sheet_name_, sizeof(new_sheet_name_));
    ImGui::SameLine();
    if (ImGui::SmallButton(tr("+ Zona"))) {
        service_.add_sheet(new_sheet_name_);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton(tr("Duplicar"))) {
        service_.duplicate_active_sheet();
    }
    if (sheets.size() > 1) {
        ImGui::SameLine();
        if (ImGui::SmallButton(tr("Borrar Zona"))) {
            service_.remove_sheet(service_.active_sheet_index());
        }
    }
    ImGui::Separator();
}

void PanelPerdidaCalor::renderizar_hoja_calculo(cma::domain::ThermalSheet& sheet) {
    const int n_cols = static_cast<int>(sheet.columns.size());
    const auto& cond_db = service_.catalog().conduction_materials();
    const auto& conv_db = service_.catalog().convection_fluids();
    const auto& rad_db = service_.catalog().surface_materials();

    if (ImGui::BeginTable("heat_loss_grid", n_cols + 1,
            ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollX | ImGuiTableFlags_ScrollY,
            ImVec2(0, 355))) {
        ImGui::TableSetupScrollFreeze(1, 1);
        ImGui::TableSetupColumn(tr("Propiedad / Columna"), ImGuiTableColumnFlags_WidthFixed, 195.0f);
        for (int i = 0; i < n_cols; ++i) {
            char header_name[64];
            std::snprintf(header_name, sizeof(header_name), "%d: %s",
                          i + 1, sheet.columns[i].name.empty() ? "Layer" : sheet.columns[i].name.c_str());
            ImGui::TableSetupColumn(header_name, ImGuiTableColumnFlags_WidthFixed, 170.0f);
        }
        ImGui::TableHeadersRow();

        // Fila 0: Selección de Columna y Nombre
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Nombre de Columna"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 0);
            bool sel = (selected_col_ == i);
            if (ImGui::RadioButton("##sel_col", sel)) selected_col_ = i;
            ImGui::SameLine();
            char buf[64];
            std::strncpy(buf, sheet.columns[i].name.c_str(), sizeof(buf) - 1);
            buf[sizeof(buf) - 1] = '\0';
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputText("##colname", buf, sizeof(buf))) {
                sheet.columns[i].name = buf;
            }
            ImGui::PopID();
        }

        // Fila 1: Tipo de Capa
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Tipo de Capa"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 1);
            ImGui::SetNextItemWidth(-1.0f);
            const char* type_labels[] = { "Surface", "Solid Layer", "Contact Layer", "Enclosure" };
            int t_val = static_cast<int>(sheet.columns[i].type);
            if (ImGui::Combo("##type", &t_val, type_labels, 4)) {
                sheet.columns[i].type = static_cast<cma::domain::ThermalColumnType>(t_val);
                service_.solve_active_sheet();
            }
            ImGui::PopID();
        }

        // Fila 2: Material de Capa / Fluido (con selector de BD)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Material de Capa / Fluido"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 2);
            auto& col = sheet.columns[i];
            ImGui::SetNextItemWidth(-1.0f);
            if (col.type == cma::domain::ThermalColumnType::SolidLayer) {
                if (ImGui::BeginCombo("##mat_cond", col.material.c_str())) {
                    for (const auto& m : cond_db) {
                        bool is_sel = (m.name == col.material);
                        char item_lbl[160];
                        std::snprintf(item_lbl, sizeof(item_lbl), "%s [k25=%.2f, Tmax=%.0f C]",
                                      m.name.c_str(), m.evaluate_k(25.0), m.max_service_temp_c);
                        if (ImGui::Selectable(item_lbl, is_sel)) {
                            col.material = m.name;
                            const double t_avg = 0.5 * (col.temp_left_c + col.temp_right_c);
                            col.conductivity_w_mk = m.evaluate_k(t_avg);
                            col.max_service_temp_c = m.max_service_temp_c;
                            service_.solve_active_sheet();
                        }
                    }
                    ImGui::EndCombo();
                }
            } else if (col.type == cma::domain::ThermalColumnType::Surface ||
                       col.type == cma::domain::ThermalColumnType::Enclosure) {
                if (ImGui::BeginCombo("##mat_conv", col.material.c_str())) {
                    for (const auto& f : conv_db) {
                        bool is_sel = (f.name == col.material);
                        if (ImGui::Selectable(f.name.c_str(), is_sel)) {
                            col.material = f.name;
                            if (!col.fix_convection) {
                                col.convection_hc_w_m2k = f.default_hc_natural;
                            }
                            service_.solve_active_sheet();
                        }
                    }
                    ImGui::EndCombo();
                }
            } else {
                ImGui::TextDisabled("Contact layer N/A");
            }
            ImGui::PopID();
        }

        // Fila 3: Material de Superficie (Radiación)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Material de Superficie"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            auto& col = sheet.columns[i];
            if (col.type == cma::domain::ThermalColumnType::Surface) {
                ImGui::PushID(i * 100 + 3);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::BeginCombo("##surf_mat", col.surface_material.c_str())) {
                    for (const auto& r : rad_db) {
                        bool is_sel = (r.name == col.surface_material);
                        char lbl[128];
                        std::snprintf(lbl, sizeof(lbl), "%s (eps=%.2f)", r.name.c_str(), r.emissivity_0);
                        if (ImGui::Selectable(lbl, is_sel)) {
                            col.surface_material = r.name;
                            col.emissivity = r.emissivity_0;
                            service_.solve_active_sheet();
                        }
                    }
                    ImGui::EndCombo();
                }
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }

        // Fila 4: Temperature (left edge) °C + Fix
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Temperature (left edge) °C"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 4);
            auto& col = sheet.columns[i];
            if (ImGui::Checkbox("##fix_tl", &col.fix_temp_left)) {
                service_.solve_active_sheet();
            }
            if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Fijar temperatura borde izquierdo"));
            ImGui::SameLine();
            ImGui::SetNextItemWidth(-1.0f);
            if (col.fix_temp_left) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.95f, 0.22f, 0.22f, 1.0f));
            if (ImGui::InputDouble("##temp_left", &col.temp_left_c, 0, 0, "%.1f")) {
                service_.solve_active_sheet();
            }
            if (col.fix_temp_left) ImGui::PopStyleColor();
            ImGui::PopID();
        }

        // Fila 5: Temperature (right edge) °C + Fix
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Temperature (right edge) °C"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 5);
            auto& col = sheet.columns[i];
            if (ImGui::Checkbox("##fix_tr", &col.fix_temp_right)) {
                service_.solve_active_sheet();
            }
            if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Fijar temperatura borde derecho"));
            ImGui::SameLine();
            ImGui::SetNextItemWidth(-1.0f);
            if (col.fix_temp_right) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.95f, 0.22f, 0.22f, 1.0f));
            if (ImGui::InputDouble("##temp_right", &col.temp_right_c, 0, 0, "%.1f")) {
                service_.solve_active_sheet();
            }
            if (col.fix_temp_right) ImGui::PopStyleColor();
            ImGui::PopID();
        }

        // Fila 6: Thickness (cm)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Thickness cm"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            auto& col = sheet.columns[i];
            if (col.type == cma::domain::ThermalColumnType::SolidLayer ||
                col.type == cma::domain::ThermalColumnType::Enclosure) {
                ImGui::PushID(i * 100 + 6);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputDouble("##thick", &col.thickness_cm, 0, 0, "%.3f")) {
                    col.thickness_cm = std::max(0.01, col.thickness_cm);
                    service_.solve_active_sheet();
                }
                ImGui::PopID();
            } else {
                ImGui::TextDisabled("0.0");
            }
        }

        // Fila 7: Calculation Grid
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Cuadrícula de Cálculo"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            auto& col = sheet.columns[i];
            if (col.type == cma::domain::ThermalColumnType::SolidLayer) {
                ImGui::PushID(i * 100 + 7);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputInt("##grid", &col.grid_points, 0, 0)) {
                    col.grid_points = std::clamp(col.grid_points, 2, 100);
                    service_.solve_active_sheet();
                }
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }

        // Fila 8: Average conductivity (k) W/(mK) + Fix
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Average conductivity (k)"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            auto& col = sheet.columns[i];
            if (col.type == cma::domain::ThermalColumnType::SolidLayer) {
                ImGui::PushID(i * 100 + 8);
                if (ImGui::Checkbox("##fix_k", &col.fix_conductivity)) {
                    service_.solve_active_sheet();
                }
                if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Fijar k manual (desmarcar para evaluar polinomio k(T) de la BD)"));
                ImGui::SameLine();
                ImGui::SetNextItemWidth(-1.0f);
                if (col.fix_conductivity) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.95f, 0.22f, 0.22f, 1.0f));
                if (ImGui::InputDouble("##conductivity", &col.conductivity_w_mk, 0, 0, "%.3f")) {
                    col.fix_conductivity = true;
                    service_.solve_active_sheet();
                }
                if (col.fix_conductivity) ImGui::PopStyleColor();
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }

        // Fila 9: Convection coefficient (hc) W/(m²K) + Fix
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Convection coeff. (hc)"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            auto& col = sheet.columns[i];
            if (col.type == cma::domain::ThermalColumnType::Surface) {
                ImGui::PushID(i * 100 + 9);
                if (ImGui::Checkbox("##fix_hc", &col.fix_convection)) {
                    service_.solve_active_sheet();
                }
                if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Fijar hc manual (desmarcar para correlación de Nusselt)"));
                ImGui::SameLine();
                ImGui::SetNextItemWidth(-1.0f);
                if (col.fix_convection) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.95f, 0.22f, 0.22f, 1.0f));
                if (ImGui::InputDouble("##convection", &col.convection_hc_w_m2k, 0, 0, "%.3f")) {
                    col.fix_convection = true;
                    service_.solve_active_sheet();
                }
                if (col.fix_convection) ImGui::PopStyleColor();
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }

        // Fila 10: Radiation coefficient (hr) W/(m²K) + Fix
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Radiation coeff. (hr)"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            auto& col = sheet.columns[i];
            if (col.type == cma::domain::ThermalColumnType::Surface) {
                ImGui::PushID(i * 100 + 10);
                if (ImGui::Checkbox("##fix_hr", &col.fix_radiation)) {
                    service_.solve_active_sheet();
                }
                ImGui::SameLine();
                ImGui::SetNextItemWidth(-1.0f);
                if (col.fix_radiation) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.95f, 0.22f, 0.22f, 1.0f));
                if (ImGui::InputDouble("##radiation", &col.radiation_hr_w_m2k, 0, 0, "%.3f")) {
                    col.fix_radiation = true;
                    service_.solve_active_sheet();
                }
                if (col.fix_radiation) ImGui::PopStyleColor();
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }

        // Fila 11: Thermal Resistance °C/kW + Fix
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Thermal Resistance °C/kW"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 11);
            auto& col = sheet.columns[i];
            if (ImGui::Checkbox("##fix_res", &col.fix_resistance)) {
                service_.solve_active_sheet();
            }
            ImGui::SameLine();
            ImGui::SetNextItemWidth(-1.0f);
            if (col.fix_resistance) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.95f, 0.22f, 0.22f, 1.0f));
            if (ImGui::InputDouble("##resistance", &col.resistance_c_kw, 0, 0, "%.3f")) {
                col.fix_resistance = true;
                service_.solve_active_sheet();
            }
            if (col.fix_resistance) ImGui::PopStyleColor();
            ImGui::PopID();
        }

        // Fila 12: Límite Térmico / Masa de Capa
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Masa / Límite Térmico"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            const auto& col = sheet.columns[i];
            if (col.type == cma::domain::ThermalColumnType::SolidLayer) {
                if (col.exceeds_service_limit) {
                    ImGui::TextColored(ImVec4(0.95f, 0.20f, 0.15f, 1.0f),
                                       "%.0f kg [>%.0f°C!]", col.mass_kg, col.max_service_temp_c);
                } else {
                    ImGui::Text("%.1f kg (Tmax %.0f°C)", col.mass_kg, col.max_service_temp_c);
                }
            } else {
                ImGui::Text("A = %.2f m²", col.area_left_m2);
            }
        }

        // Fila 13: Heat Flow kW
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Flujo de Calor kW"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::Text("%.3f", sheet.columns[i].heat_flow_kw);
        }

        // Fila 14: Heat Flux kW/m²
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Flujo de Calor kW/m²"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::Text("%.3f", sheet.columns[i].heat_flux_kw_m2);
        }

        ImGui::EndTable();
    }
}

void PanelPerdidaCalor::renderizar_seccion_transversal(const cma::domain::ThermalSheet& sheet) {
    if (sheet.columns.empty() || sheet.total_thickness_cm <= 0.0) return;

    ImGui::Text(tr("Sección Transversal Térmica (Espesor total = %.1f cm, Masa = %.1f kg, R_tot = %.3f °C/kW):"),
                sheet.total_thickness_cm, sheet.total_mass_kg, sheet.total_resistance_c_kw);

    ImVec2 p0 = ImGui::GetCursorScreenPos();
    const float avail_w = std::max(300.0f, ImGui::GetContentRegionAvail().x);
    const float bar_h = 46.0f;
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const double t_min = std::min(sheet.inner_fluid_temp_c, sheet.ambient_temp_c);
    const double t_max = std::max(sheet.inner_fluid_temp_c, sheet.ambient_temp_c);

    float cur_x = p0.x;
    for (size_t i = 0; i < sheet.columns.size(); ++i) {
        const auto& col = sheet.columns[i];
        if (col.type != cma::domain::ThermalColumnType::SolidLayer &&
            col.type != cma::domain::ThermalColumnType::Enclosure) {
            continue;
        }
        const float frac = static_cast<float>(col.thickness_cm / sheet.total_thickness_cm);
        const float w = std::max(28.0f, avail_w * frac);
        const ImU32 c_left = temperature_to_color(col.temp_left_c, t_min, t_max);
        const ImU32 c_right = temperature_to_color(col.temp_right_c, t_min, t_max);

        dl->AddRectFilledMultiColor(
            ImVec2(cur_x, p0.y), ImVec2(cur_x + w, p0.y + bar_h),
            c_left, c_right, c_right, c_left);
        dl->AddRect(ImVec2(cur_x, p0.y), ImVec2(cur_x + w, p0.y + bar_h), IM_COL32(40, 40, 45, 255), 0.0f, 0, 1.5f);

        char lbl[96];
        std::snprintf(lbl, sizeof(lbl), "%s\n%.1f cm (%.0f->%.0f°C)",
                      col.name.empty() ? col.material.substr(0, 14).c_str() : col.name.c_str(),
                      col.thickness_cm, col.temp_left_c, col.temp_right_c);
        dl->AddText(ImVec2(cur_x + 4.0f, p0.y + 6.0f), IM_COL32(20, 20, 20, 255), lbl);
        cur_x += w;
    }
    ImGui::Dummy(ImVec2(avail_w, bar_h + 4.0f));
}

void PanelPerdidaCalor::renderizar_panel_inferior_controles(cma::domain::ThermalSheet& sheet) {
    ImGui::Columns(3, "heat_loss_dashboard", true);

    // Columna 1: Flujo de Calor Total y Geometría
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f), "Total Heat Flow (kW)");
    ImGui::SetNextItemWidth(160.0f);
    if (ImGui::InputDouble("##total_h_flow", &sheet.total_heat_flow_kw, 0, 0, "%.3f")) {
        sheet.solver_mode = cma::domain::HeatLossSolverMode::TemperatureProfile;
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    ImGui::Text("U_ext = %.2f W/m²K", sheet.overall_u_outer_w_m2k);

    ImGui::Spacing();
    ImGui::Text(tr("Shape & Dimensions:"));
    int geom = static_cast<int>(sheet.geometry);
    if (ImGui::RadioButton(tr("Muro Plano"), &geom, 0)) {
        sheet.geometry = cma::domain::ThermalGeometryType::PlaneWall;
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    if (ImGui::RadioButton("Cylinder", &geom, 2)) {
        sheet.geometry = cma::domain::ThermalGeometryType::Cylinder;
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    if (ImGui::RadioButton("Horiz. Pipe", &geom, 4)) {
        sheet.geometry = cma::domain::ThermalGeometryType::HorizontalCylinder;
        service_.solve_active_sheet();
    }
    if (ImGui::RadioButton("Cube", &geom, 1)) {
        sheet.geometry = cma::domain::ThermalGeometryType::Cube;
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    if (ImGui::RadioButton("Sphere", &geom, 3)) {
        sheet.geometry = cma::domain::ThermalGeometryType::Sphere;
        service_.solve_active_sheet();
    }

    ImGui::SetNextItemWidth(95.0f);
    if (ImGui::InputDouble("Height/Len (m)", &sheet.height_or_length_m, 0, 0, "%.2f")) {
        sheet.height_or_length_m = std::max(0.05, sheet.height_or_length_m);
        service_.solve_active_sheet();
    }
    ImGui::SameLine();
    ImGui::SetNextItemWidth(95.0f);
    if (ImGui::InputDouble("Diam/Width (m)", &sheet.inner_diameter_or_width_m, 0, 0, "%.2f")) {
        sheet.inner_diameter_or_width_m = std::max(0.05, sheet.inner_diameter_or_width_m);
        service_.solve_active_sheet();
    }

    ImGui::NextColumn();

    // Columna 2: Opciones de Cálculo y Ventanas
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f), "Options & Property Mode");
    if (ImGui::Button(tr("Usar k(T) Automático de Base de Datos"), ImVec2(-1, 24))) {
        for (auto& col : sheet.columns) {
            if (col.type == cma::domain::ThermalColumnType::SolidLayer) {
                col.fix_conductivity = false;
            }
        }
        service_.solve_active_sheet();
        status_banner_ = "Conductividad k(T) dependiente de temperatura activada en todas las capas sólidas.";
        status_ok_ = true;
    }
    if (ImGui::Button(tr("Tabla del Diagrama"), ImVec2(-1, 24))) {
        show_table_popup_ = !show_table_popup_;
    }
    if (ImGui::Button(tr("Dibujar Perfil de Temperatura"), ImVec2(-1, 24))) {
        show_diagram_popup_ = !show_diagram_popup_;
    }
    if (ImGui::Button(tr("Reset to Defaults"), ImVec2(-1, 24))) {
        inicializar_valores_defecto();
    }

    ImGui::NextColumn();

    // Columna 3: Solvers HSC
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f), "Calculate Solvers");
    if (ImGui::Button(tr("Temperature Profile Solver (Q Fijo)"), ImVec2(-1, 28))) {
        sheet.solver_mode = cma::domain::HeatLossSolverMode::TemperatureProfile;
        service_.solve_active_sheet();
        status_banner_ = sheet.status_message;
        status_ok_ = sheet.converged;
    }
    if (ImGui::Button(tr("Resolvedor de Flujo de Calor (2 Temps Fijas)"), ImVec2(-1, 28))) {
        sheet.solver_mode = cma::domain::HeatLossSolverMode::HeatFlowTwoTemps;
        service_.solve_active_sheet();
        status_banner_ = sheet.status_message;
        status_ok_ = sheet.converged;
    }

    ImGui::Spacing();
    ImGui::Text(tr("HOT")); ImGui::SameLine();
    ImVec2 cursor = ImGui::GetCursorScreenPos();
    ImGui::GetWindowDrawList()->AddRectFilledMultiColor(
        cursor, ImVec2(cursor.x + 120, cursor.y + 16),
        IM_COL32(255, 45, 30, 255), IM_COL32(30, 90, 255, 255),
        IM_COL32(30, 90, 255, 255), IM_COL32(255, 45, 30, 255));
    ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 130);
    ImGui::Text(tr("COLD (T_shell = %.1f °C)"), sheet.outer_shell_temp_c);

    ImGui::Columns(1);
}

void PanelPerdidaCalor::renderizar_pestana_perfil_y_tabla(const cma::domain::ThermalSheet& sheet) {
    ImGui::SetNextItemWidth(240.0f);
    ImGui::InputText("##csv_prof_path", csv_profile_path_, sizeof(csv_profile_path_));
    ImGui::SameLine();
    if (ImGui::Button(tr("Exportar Perfil a CSV"))) {
        status_ok_ = service_.export_active_profile_csv(csv_profile_path_);
        status_banner_ = status_ok_
            ? (std::string("Perfil exportado a ") + csv_profile_path_)
            : "Error al exportar archivo CSV.";
    }

    std::vector<double> xs, ys, tmaxs;
    xs.reserve(sheet.profile.size());
    ys.reserve(sheet.profile.size());
    tmaxs.reserve(sheet.profile.size());
    for (const auto& pt : sheet.profile) {
        xs.push_back(pt.distance_mm);
        ys.push_back(pt.temperature_c);
        tmaxs.push_back(pt.max_limit_c);
    }

    char plot_title[160];
    std::snprintf(plot_title, sizeof(plot_title),
                  "Temperature Profile - %s (Total Heat Flow: %.3f kW, Shell: %.1f °C)",
                  sheet.name.c_str(), sheet.total_heat_flow_kw, sheet.outer_shell_temp_c);
    if (!xs.empty() && ImPlot::BeginPlot(plot_title, ImVec2(-1, 310))) {
        ImPlot::SetupAxes("Distance (mm)", "Temperature (°C)");
        ImPlot::PlotLine("Temperature T(x)", xs.data(), ys.data(), static_cast<int>(xs.size()));
        ImPlot::PlotLine("Max Service Temp Limit", xs.data(), tmaxs.data(), static_cast<int>(xs.size()));
        ImPlot::EndPlot();
    }

    if (ImGui::BeginTable("profile_num_table", 7,
            ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
            ImVec2(0, 220))) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("#", ImGuiTableColumnFlags_WidthFixed, 40.0f);
        ImGui::TableSetupColumn("Distancia (mm)");
        ImGui::TableSetupColumn("Temperatura (°C)");
        ImGui::TableSetupColumn("k local W/(m·K)");
        ImGui::TableSetupColumn("Flujo q'' (kW/m²)");
        ImGui::TableSetupColumn("Columna");
        ImGui::TableSetupColumn("Material");
        ImGui::TableHeadersRow();

        for (size_t i = 0; i < sheet.profile.size(); ++i) {
            const auto& pt = sheet.profile[i];
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text("%zu", i + 1);
            ImGui::TableNextColumn(); ImGui::Text("%.2f", pt.distance_mm);
            ImGui::TableNextColumn(); ImGui::Text("%.2f", pt.temperature_c);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", pt.conductivity_w_mk);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", pt.heat_flux_kw_m2);
            ImGui::TableNextColumn(); ImGui::TextUnformatted(pt.column_name.c_str());
            ImGui::TableNextColumn(); ImGui::TextUnformatted(pt.material_name.c_str());
        }
        ImGui::EndTable();
    }
}

void PanelPerdidaCalor::renderizar_pestana_bases_datos(cma::domain::ThermalSheet& sheet) {
    ImGui::RadioButton(tr("Conduction Database (Sólidos)"), &db_subtab_, 0); ImGui::SameLine();
    ImGui::RadioButton(tr("Convection Database (Fluidos)"), &db_subtab_, 1); ImGui::SameLine();
    ImGui::RadioButton(tr("Surface Radiation Database (Emisividad)"), &db_subtab_, 2);
    ImGui::SameLine();
    ImGui::SetNextItemWidth(200.0f);
    ImGui::InputTextWithHint("##db_filter", tr("Filtrar material..."), db_search_, sizeof(db_search_));
    ImGui::Separator();

    if (db_subtab_ == 0) {
        const auto& cond = service_.catalog().conduction_materials();
        ImGui::Columns(2, "cond_db_cols", true);
        if (ImGui::BeginTable("cond_table", 6,
                ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                ImVec2(0, 320))) {
            ImGui::TableSetupScrollFreeze(0, 1);
            ImGui::TableSetupColumn("Material");
            ImGui::TableSetupColumn("Categoría");
            ImGui::TableSetupColumn("ρ (kg/m³)");
            ImGui::TableSetupColumn("Tmax (°C)");
            ImGui::TableSetupColumn("k(25°C)");
            ImGui::TableSetupColumn("k(1000°C)");
            ImGui::TableHeadersRow();

            for (int i = 0; i < static_cast<int>(cond.size()); ++i) {
                const auto& m = cond[i];
                if (db_search_[0] != '\0' && m.name.find(db_search_) == std::string::npos &&
                    m.category.find(db_search_) == std::string::npos) {
                    continue;
                }
                ImGui::TableNextRow();
                ImGui::TableNextColumn();
                if (ImGui::Selectable(m.name.c_str(), selected_cond_idx_ == i, ImGuiSelectableFlags_SpanAllColumns)) {
                    selected_cond_idx_ = i;
                }
                ImGui::TableNextColumn(); ImGui::TextUnformatted(m.category.c_str());
                ImGui::TableNextColumn(); ImGui::Text("%.0f", m.density_kg_m3);
                ImGui::TableNextColumn(); ImGui::Text("%.0f", m.max_service_temp_c);
                ImGui::TableNextColumn(); ImGui::Text("%.3f", m.evaluate_k(25.0));
                ImGui::TableNextColumn(); ImGui::Text("%.3f", m.evaluate_k(1000.0));
            }
            ImGui::EndTable();
        }

        if (selected_cond_idx_ >= 0 && selected_cond_idx_ < static_cast<int>(cond.size()) &&
            selected_col_ >= 0 && selected_col_ < static_cast<int>(sheet.columns.size())) {
            if (ImGui::Button(tr("Asignar Material a Columna Seleccionada"), ImVec2(-1, 28))) {
                auto& col = sheet.columns[selected_col_];
                col.type = cma::domain::ThermalColumnType::SolidLayer;
                col.material = cond[selected_cond_idx_].name;
                col.fix_conductivity = false;
                service_.solve_active_sheet();
            }
        }

        ImGui::NextColumn();
        // Curva k(T) del material seleccionado + Alta de material personalizado
        if (selected_cond_idx_ >= 0 && selected_cond_idx_ < static_cast<int>(cond.size())) {
            const auto& m = cond[selected_cond_idx_];
            std::vector<double> ts, ks;
            for (int t = 0; t <= static_cast<int>(m.max_service_temp_c); t += 50) {
                ts.push_back(static_cast<double>(t));
                ks.push_back(m.evaluate_k(static_cast<double>(t)));
            }
            if (ImPlot::BeginPlot("Curva k(T) [W/(m·K)] vs Temperatura (°C)", ImVec2(-1, 210))) {
                ImPlot::SetupAxes("T (°C)", "k W/(m·K)");
                ImPlot::PlotLine(m.name.c_str(), ts.data(), ks.data(), static_cast<int>(ts.size()));
                ImPlot::EndPlot();
            }
        }

        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f),
                           tr("Registrar / Actualizar Material Sólido de Usuario:"));
        ImGui::InputText(tr("Nombre##umat"), user_mat_name_, sizeof(user_mat_name_));
        ImGui::InputText(tr("Categoría##ucat"), user_mat_cat_, sizeof(user_mat_cat_));
        ImGui::InputDouble(tr("Densidad (kg/m³)"), &user_mat_rho_, 0, 0, "%.0f");
        ImGui::InputDouble(tr("T máx servicio (°C)"), &user_mat_tmax_, 0, 0, "%.0f");
        ImGui::InputDouble("k0 [W/(m·K)]", &user_mat_k0_, 0, 0, "%.4f");
        ImGui::InputDouble("k1 [d(k)/dT]", &user_mat_k1_, 0, 0, "%.6f");
        if (ImGui::Button(tr("Guardar en Conduction Database"), ImVec2(-1, 26))) {
            cma::domain::ConductionMaterialEntry e;
            e.name = user_mat_name_;
            e.category = user_mat_cat_;
            e.density_kg_m3 = user_mat_rho_;
            e.max_service_temp_c = user_mat_tmax_;
            e.k0 = user_mat_k0_;
            e.k1 = user_mat_k1_;
            e.k2 = user_mat_k2_;
            e.reference = user_mat_ref_;
            e.user_defined = true;
            service_.catalog().add_or_update_conduction(e);
            status_banner_ = std::string("Material agregado al catálogo: ") + e.name;
            status_ok_ = true;
        }
        ImGui::Columns(1);
    } else if (db_subtab_ == 1) {
        const auto& conv = service_.catalog().convection_fluids();
        if (ImGui::BeginTable("conv_table", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
            ImGui::TableSetupColumn("Fluido / Medio");
            ImGui::TableSetupColumn("Categoría");
            ImGui::TableSetupColumn("hc Natural W/(m²K)");
            ImGui::TableSetupColumn("hc Forzado W/(m²K)");
            ImGui::TableSetupColumn("k_f W/(m·K)");
            ImGui::TableSetupColumn("Prandtl");
            ImGui::TableHeadersRow();
            for (const auto& f : conv) {
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::TextUnformatted(f.name.c_str());
                ImGui::TableNextColumn(); ImGui::TextUnformatted(f.category.c_str());
                ImGui::TableNextColumn(); ImGui::Text("%.1f", f.default_hc_natural);
                ImGui::TableNextColumn(); ImGui::Text("%.1f", f.default_hc_forced);
                ImGui::TableNextColumn(); ImGui::Text("%.4f", f.thermal_cond_w_mk);
                ImGui::TableNextColumn(); ImGui::Text("%.2f", f.prandtl);
            }
            ImGui::EndTable();
        }
    } else {
        const auto& rad = service_.catalog().surface_materials();
        if (ImGui::BeginTable("rad_table", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
            ImGui::TableSetupColumn("Superficie");
            ImGui::TableSetupColumn("Categoría");
            ImGui::TableSetupColumn("ε (25 °C)");
            ImGui::TableSetupColumn("ε (800 °C)");
            ImGui::TableSetupColumn("Referencia");
            ImGui::TableHeadersRow();
            for (const auto& r : rad) {
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::TextUnformatted(r.name.c_str());
                ImGui::TableNextColumn(); ImGui::TextUnformatted(r.category.c_str());
                ImGui::TableNextColumn(); ImGui::Text("%.3f", r.evaluate_emissivity(25.0));
                ImGui::TableNextColumn(); ImGui::Text("%.3f", r.evaluate_emissivity(800.0));
                ImGui::TableNextColumn(); ImGui::TextUnformatted(r.reference.c_str());
            }
            ImGui::EndTable();
        }
    }
}

void PanelPerdidaCalor::renderizar_pestana_calculadoras(cma::domain::ThermalSheet& sheet) {
    ImGui::Columns(2, "hl_calculators_cols", true);

    // 1. Target Thickness / Insulation Optimizer
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f),
                       tr("1. Optimizador de Espesor de Capa (Target Value Solver)"));
    ImGui::TextWrapped(tr("Calcula automáticamente el espesor necesario de una capa aislante/refractaria para alcanzar una temperatura máxima de coraza exterior o un flujo de calor objetivo."));
    ImGui::Spacing();

    ImGui::InputInt(tr("Índice de Columna (1..N)"), &target_req_.target_column_index);
    target_req_.target_column_index = std::clamp(
        target_req_.target_column_index, 0, std::max(0, static_cast<int>(sheet.columns.size()) - 1));
    if (!sheet.columns.empty()) {
        ImGui::Text(tr("Capa seleccionada: Col %d (%s - %s)"),
                    target_req_.target_column_index + 1,
                    sheet.columns[target_req_.target_column_index].name.c_str(),
                    sheet.columns[target_req_.target_column_index].material.c_str());
    }

    int mode_target = target_req_.target_is_outer_temp ? 0 : 1;
    ImGui::RadioButton(tr("Fijar Temperatura Coraza Exterior (°C)"), &mode_target, 0);
    ImGui::SameLine();
    ImGui::RadioButton(tr("Fijar Pérdida de Calor Total (kW)"), &mode_target, 1);
    target_req_.target_is_outer_temp = (mode_target == 0);

    ImGui::InputDouble(target_req_.target_is_outer_temp ? tr("T_shell Objetivo (°C)") : tr("Q_total Objetivo (kW)"),
                       &target_req_.target_value, 0, 0, "%.2f");
    ImGui::InputDouble(tr("Espesor Mín (cm)"), &target_req_.min_thickness_cm, 0, 0, "%.1f");
    ImGui::InputDouble(tr("Espesor Máx (cm)"), &target_req_.max_thickness_cm, 0, 0, "%.1f");

    if (ImGui::Button(tr("Optimizar Espesor de Aislamiento"), ImVec2(-1, 30))) {
        target_res_ = service_.optimize_layer_thickness(target_req_);
        target_solved_ = true;
        status_banner_ = target_res_.message;
        status_ok_ = target_res_.ok;
    }
    if (target_solved_) {
        ImGui::TextColored(target_res_.ok ? ImVec4(0.15f, 0.65f, 0.25f, 1.0f) : ImVec4(0.85f, 0.25f, 0.20f, 1.0f),
                           "%s", target_res_.message.c_str());
    }

    ImGui::NextColumn();

    // 2. Gas & Particle Radiation Calculator (Hottel)
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f),
                       tr("2. Calculadora de Radiación de Gases y Partículas (Hottel)"));
    bool changed = false;
    changed |= ImGui::InputDouble(tr("Temperatura del Gas (°C)"), &gas_rad_in_.gas_temp_c, 0, 0, "%.1f");
    changed |= ImGui::InputDouble(tr("Temperatura de Pared (°C)"), &gas_rad_in_.wall_temp_c, 0, 0, "%.1f");
    changed |= ImGui::InputDouble(tr("Presión Total (bar)"), &gas_rad_in_.total_pressure_bar, 0, 0, "%.2f");
    changed |= ImGui::InputDouble(tr("Longitud Media de Haz Le (m)"), &gas_rad_in_.mean_beam_length_m, 0, 0, "%.2f");
    changed |= ImGui::InputDouble(tr("H2O (% vol)"), &gas_rad_in_.p_h2o_vol_pct, 0, 0, "%.1f");
    changed |= ImGui::InputDouble(tr("CO2 (% vol)"), &gas_rad_in_.p_co2_vol_pct, 0, 0, "%.1f");
    changed |= ImGui::InputDouble(tr("SO2 (% vol)"), &gas_rad_in_.p_so2_vol_pct, 0, 0, "%.1f");
    changed |= ImGui::InputDouble(tr("Carga de Polvo / Hollín (g/Nm³)"), &gas_rad_in_.dust_load_g_nm3, 0, 0, "%.2f");
    if (changed) {
        gas_rad_res_ = service_.calculate_gas_radiation(gas_rad_in_);
    }

    ImGui::Separator();
    ImGui::Text("Emisividad de Gases (H2O+CO2+SO2): %.3f", gas_rad_res_.gas_emissivity);
    ImGui::Text("Emisividad de Partículas:          %.3f", gas_rad_res_.particle_emissivity);
    ImGui::Text("Emisividad Efectiva Total:         %.3f", gas_rad_res_.total_emissivity);
    ImGui::Text("Flujo Radiativo Neto:              %.2f kW/m²", gas_rad_res_.radiative_flux_kw_m2);
    ImGui::TextColored(ImVec4(0.15f, 0.55f, 0.85f, 1.0f),
                       "Coeficiente de Radiación hr:       %.2f W/(m²·K)", gas_rad_res_.effective_hr_w_m2k);

    if (!sheet.columns.empty() && ImGui::Button(tr("Aplicar hr a Superficie Interior (Col 1)"), ImVec2(-1, 28))) {
        auto& c0 = sheet.columns.front();
        if (c0.type == cma::domain::ThermalColumnType::Surface) {
            c0.radiation_hr_w_m2k = gas_rad_res_.effective_hr_w_m2k;
            c0.fix_radiation = true;
            service_.solve_active_sheet();
        }
    }

    ImGui::Columns(1);
}

void PanelPerdidaCalor::renderizar_pestana_resumen_reactor() {
    const auto& sheets = service_.sheets();
    const double q_tot = service_.total_reactor_heat_flow_kw();
    const double m_tot = service_.total_reactor_mass_kg();
    const double a_tot = service_.total_outer_area_m2();

    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f),
                       tr("Balance Global de Pérdidas Térmicas del Reactor / Equipo"));
    ImGui::Text("Pérdida de Calor Total = %.3f kW (%.2f Mcal/h | %.2f GJ/h)  |  Masa Refractaria/Acero = %.1f kg  |  Área Ext. = %.2f m²",
                q_tot, q_tot * 0.859845, q_tot * 0.0036, m_tot, a_tot);
    ImGui::Spacing();

    if (ImGui::BeginTable("reactor_summary_tbl", 8, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Zona / Hoja");
        ImGui::TableSetupColumn("Área Ext (m²)");
        ImGui::TableSetupColumn("Espesor (cm)");
        ImGui::TableSetupColumn("Masa (kg)");
        ImGui::TableSetupColumn("T_int (°C)");
        ImGui::TableSetupColumn("T_shell (°C)");
        ImGui::TableSetupColumn("Q Pérdida (kW)");
        ImGui::TableSetupColumn("% del Total");
        ImGui::TableHeadersRow();

        for (const auto& s : sheets) {
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::TextUnformatted(s.name.c_str());
            ImGui::TableNextColumn(); ImGui::Text("%.2f", s.outer_area_m2);
            ImGui::TableNextColumn(); ImGui::Text("%.1f", s.total_thickness_cm);
            ImGui::TableNextColumn(); ImGui::Text("%.1f", s.total_mass_kg);
            ImGui::TableNextColumn(); ImGui::Text("%.1f", s.inner_fluid_temp_c);
            ImGui::TableNextColumn(); ImGui::Text("%.1f", s.outer_shell_temp_c);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", s.total_heat_flow_kw);
            ImGui::TableNextColumn();
            const double pct = (std::abs(q_tot) > 1e-9) ? (100.0 * s.total_heat_flow_kw / q_tot) : 0.0;
            ImGui::Text("%.1f %%", pct);
        }
        ImGui::EndTable();
    }

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f),
                       tr("Persistencia del Libro y Enlace con HSC Sim / HSC Balance"));

    ImGui::SetNextItemWidth(240.0f);
    ImGui::InputText("##wb_path", workbook_path_, sizeof(workbook_path_));
    ImGui::SameLine();
    if (ImGui::Button(tr("Guardar Libro (.tsv)"))) {
        status_ok_ = service_.save_workbook_tsv(workbook_path_);
        status_banner_ = status_ok_ ? "Libro Heat Loss guardado correctamente." : "Error al guardar libro.";
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Cargar Libro (.tsv)"))) {
        status_ok_ = service_.load_workbook_tsv(workbook_path_);
        status_banner_ = status_ok_ ? "Libro Heat Loss cargado correctamente." : "No se pudo cargar el archivo.";
    }

    ImGui::Spacing();
    if (flowsheet_ && !flowsheet_->nodos().empty()) {
        std::vector<std::string> node_names;
        for (const auto& n : flowsheet_->nodos()) {
            node_names.push_back(n.nombre + " (ID " + std::to_string(n.id) + ")");
        }
        target_node_idx_ = std::clamp(target_node_idx_, 0, static_cast<int>(node_names.size()) - 1);
        ImGui::SetNextItemWidth(260.0f);
        if (ImGui::BeginCombo(tr("Unidad del Flowsheet"), node_names[target_node_idx_].c_str())) {
            for (int i = 0; i < static_cast<int>(node_names.size()); ++i) {
                if (ImGui::Selectable(node_names[i].c_str(), target_node_idx_ == i)) {
                    target_node_idx_ = i;
                }
            }
            ImGui::EndCombo();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Inyectar Q_loss en Unidad del Flowsheet"))) {
            auto& n = flowsheet_->nodos()[target_node_idx_];
            n.params["heat_loss_kw"] = q_tot;
            n.params["calor_kw"] = -q_tot;
            status_banner_ = "Pérdida de calor (" + std::to_string(q_tot) + " kW) asignada a " + n.nombre;
            status_ok_ = true;
        }
    }

    if (al_enviar_a_balance) {
        if (ImGui::Button(tr("Abrir Módulo HSC Heat & Material Balance con este Q_loss"))) {
            al_enviar_a_balance(q_tot, service_.active_sheet().name);
        }
    }
}

void PanelPerdidaCalor::renderizar_popups_flotantes(const cma::domain::ThermalSheet& sheet) {
    if (show_diagram_popup_) {
        ImGui::SetNextWindowSize(ImVec2(640, 420), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Temperature Profile Diagram"), &show_diagram_popup_)) {
            std::vector<double> xs, ys;
            for (const auto& pt : sheet.profile) {
                xs.push_back(pt.distance_mm);
                ys.push_back(pt.temperature_c);
            }
            char title_lbl[128];
            std::snprintf(title_lbl, sizeof(title_lbl),
                          "Temperature Profile %s (Total Heat flow: %.3f kW)",
                          sheet.name.c_str(), sheet.total_heat_flow_kw);
            if (!xs.empty() && ImPlot::BeginPlot(title_lbl, ImVec2(-1, -1))) {
                ImPlot::SetupAxes("Distance mm", "Temperature °C");
                ImPlot::PlotLine("Wall Temp", xs.data(), ys.data(), static_cast<int>(xs.size()));
                ImPlot::EndPlot();
            }
            ImGui::End();
        }
    }

    if (show_table_popup_) {
        ImGui::SetNextWindowSize(ImVec2(580, 380), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Tabla Numérica del Perfil de Temperatura"), &show_table_popup_)) {
            if (ImGui::BeginTable("popup_prof_tbl", 5,
                    ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                ImGui::TableSetupColumn("Distancia (mm)");
                ImGui::TableSetupColumn("Temperatura (°C)");
                ImGui::TableSetupColumn("k W/(m·K)");
                ImGui::TableSetupColumn("q'' (kW/m²)");
                ImGui::TableSetupColumn("Material");
                ImGui::TableHeadersRow();
                for (const auto& pt : sheet.profile) {
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("%.2f", pt.distance_mm);
                    ImGui::TableNextColumn(); ImGui::Text("%.2f", pt.temperature_c);
                    ImGui::TableNextColumn(); ImGui::Text("%.3f", pt.conductivity_w_mk);
                    ImGui::TableNextColumn(); ImGui::Text("%.3f", pt.heat_flux_kw_m2);
                    ImGui::TableNextColumn(); ImGui::TextUnformatted(pt.material_name.c_str());
                }
                ImGui::EndTable();
            }
            ImGui::End();
        }
    }
}

}} // namespace cma::interfaz
