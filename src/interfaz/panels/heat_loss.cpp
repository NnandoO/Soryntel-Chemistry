#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de Pérdida de Calor
// ═══════════════════════════════════════════════════════════════════════════
PanelPerdidaCalor::PanelPerdidaCalor() {
    inicializar_valores_defecto();
}

void PanelPerdidaCalor::inicializar_valores_defecto() {
    columnas_.clear();
    
    // Column 1: Inner Surface
    LayerColumn c1;
    c1.type = 0; // Surface
    strncpy(c1.material, "Molten metal (p = 1 bar)", sizeof(c1.material)-1);
    strncpy(c1.surface_material, "Ideal black surface", sizeof(c1.surface_material)-1);
    c1.temp_left = 1100.0;
    c1.temp_right = 1087.0;
    c1.thickness = 0.0;
    c1.convection_hc = 100.0;
    c1.fix_convection = true;
    c1.radiation_hr = 579.216;
    c1.resistance = 0.937;
    columnas_.push_back(c1);

    // Column 2: Solid Layer (Forsterite)
    LayerColumn c2;
    c2.type = 1; // Solid Layer
    strncpy(c2.material, "Forsterite (Mg2SiO4)", sizeof(c2.material)-1);
    c2.temp_left = 1087.0;
    c2.temp_right = 444.0;
    c2.thickness = 50.0; // cm
    c2.grid_points = 10;
    c2.conductivity = 3.849;
    c2.resistance = 47.729;
    columnas_.push_back(c2);

    // Column 3: Contact Layer
    LayerColumn c3;
    c3.type = 2; // Contact Layer
    strncpy(c3.material, "Contact layer N/A", sizeof(c3.material)-1);
    c3.temp_left = 444.0;
    c3.temp_right = 175.0;
    c3.resistance = 20.0;
    c3.fix_resistance = true;
    columnas_.push_back(c3);

    // Column 4: Solid Layer (Stainless Steel)
    LayerColumn c4;
    c4.type = 1; // Solid Layer
    strncpy(c4.material, "Stainless steel (AISI 347)", sizeof(c4.material)-1);
    c4.temp_left = 175.0;
    c4.temp_right = 162.0;
    c4.thickness = 8.0;
    c4.grid_points = 10;
    c4.conductivity = 16.444;
    c4.resistance = 0.981;
    columnas_.push_back(c4);

    // Column 5: Contact Layer
    LayerColumn c5;
    c5.type = 2; // Contact Layer
    strncpy(c5.material, "Contact layer N/A", sizeof(c5.material)-1);
    c5.temp_left = 162.0;
    c5.temp_right = 95.0;
    c5.resistance = 5.0;
    c5.fix_resistance = true;
    columnas_.push_back(c5);

    // Column 6: Solid Layer (Carbon Steel)
    LayerColumn c6;
    c6.type = 1; // Solid Layer
    strncpy(c6.material, "Carbon steel AISI 1010", sizeof(c6.material)-1);
    c6.temp_left = 95.0;
    c6.temp_right = 87.0;
    c6.thickness = 20.0;
    c6.grid_points = 10;
    c6.conductivity = 60.588;
    c6.resistance = 0.567;
    columnas_.push_back(c6);

    // Column 7: Outer Surface
    LayerColumn c7;
    c7.type = 0; // Surface
    strncpy(c7.material, "Air (gas) (p = 1 bar)", sizeof(c7.material)-1);
    strncpy(c7.surface_material, "Ideal black surface", sizeof(c7.surface_material)-1);
    c7.temp_left = 87.0;
    c7.temp_right = 25.0;
    c7.thickness = 0.0;
    c7.convection_hc = 4.987;
    c7.radiation_hr = 8.153;
    c7.resistance = 4.593;
    columnas_.push_back(c7);

    geometry_type_ = 2; // Cylinder
    geom_height_ = 1.0;
    geom_diameter_ = 0.5;
    total_heat_flow_ = 13.471;
    calculated_ = true;
}

void PanelPerdidaCalor::reciclar_calculos() {
    if (calc_type_ == 0) {
        calcular_perfil_temperatura();
    } else {
        calcular_flujo_calor();
    }
}

void PanelPerdidaCalor::calcular_perfil_temperatura() {
    if (columnas_.empty()) return;
    
    double Q = total_heat_flow_; // kW
    
    for (int iter = 0; iter < 15; ++iter) {
        double r = geom_diameter_ / 2.0;
        
        for (size_t i = 0; i < columnas_.size(); ++i) {
            auto& col = columnas_[i];
            double Area = 1.0;
            if (geometry_type_ == 0) {
                Area = geom_height_ * geom_diameter_;
            } else if (geometry_type_ == 1) {
                Area = 6.0 * r * r;
            } else if (geometry_type_ == 2) {
                Area = 2.0 * M_PI * r * geom_height_;
            } else if (geometry_type_ == 3) {
                Area = 4.0 * M_PI * r * r;
            }
            
            double thickness_m = col.thickness / 100.0;
            
            if (col.type == 0) { // Surface
                if (!col.fix_radiation) {
                    double T_L_K = col.temp_left + 273.15;
                    double T_R_K = col.temp_right + 273.15;
                    col.radiation_hr = 5.670374e-8 * (T_L_K * T_L_K + T_R_K * T_R_K) * (T_L_K + T_R_K);
                }
                col.resistance = 1.0 / ((col.convection_hc + col.radiation_hr) * Area) * 1000.0;
            } else if (col.type == 1) { // Solid Layer
                if (geometry_type_ == 0) {
                    col.resistance = thickness_m / (col.conductivity * Area) * 1000.0;
                } else if (geometry_type_ == 2) {
                    double r_next = r + thickness_m;
                    col.resistance = log(r_next / r) / (2.0 * M_PI * geom_height_ * col.conductivity) * 1000.0;
                    r = r_next;
                } else if (geometry_type_ == 3) {
                    double r_next = r + thickness_m;
                    col.resistance = (1.0/r - 1.0/r_next) / (4.0 * M_PI * col.conductivity) * 1000.0;
                    r = r_next;
                } else {
                    double r_next = r + thickness_m;
                    col.resistance = thickness_m / (col.conductivity * (6.0 * r * r)) * 1000.0;
                    r = r_next;
                }
            } else if (col.type == 2) { // Contact Layer
                // resistance is fixed
            } else if (col.type == 3) { // Enclosure
                if (geometry_type_ == 0) {
                    col.resistance = thickness_m / (0.024 * Area) * 1000.0;
                } else {
                    double r_next = r + thickness_m;
                    col.resistance = log(r_next / r) / (2.0 * M_PI * geom_height_ * 0.024) * 1000.0;
                    r = r_next;
                }
            }
            
            if (i == 0) {
                if (!col.fix_temp_left) col.temp_left = 1100.0;
            } else {
                col.temp_left = columnas_[i-1].temp_right;
            }
            col.temp_right = col.temp_left - Q * (col.resistance / 1000.0);
            
            col.heat_flow = Q;
            col.heat_flux = Q / Area;
        }
    }
    calculated_ = true;
}

void PanelPerdidaCalor::calcular_flujo_calor() {
    if (columnas_.empty()) return;
    
    double t_left = columnas_[0].temp_left;
    double t_right = columnas_.back().temp_right;
    
    size_t n = columnas_.size();
    
    for (int iter = 0; iter < 20; ++iter) {
        double r = geom_diameter_ / 2.0;
        double R_total = 0.0;
        
        for (size_t i = 0; i < n; ++i) {
            auto& col = columnas_[i];
            double Area = 1.0;
            if (geometry_type_ == 0) Area = geom_height_ * geom_diameter_;
            else if (geometry_type_ == 1) Area = 6.0 * r * r;
            else if (geometry_type_ == 2) Area = 2.0 * M_PI * r * geom_height_;
            else if (geometry_type_ == 3) Area = 4.0 * M_PI * r * r;
            
            double thickness_m = col.thickness / 100.0;
            
            if (col.type == 0) { // Surface
                if (!col.fix_radiation) {
                    double T_L_K = col.temp_left + 273.15;
                    double T_R_K = col.temp_right + 273.15;
                    col.radiation_hr = 5.670374e-8 * (T_L_K * T_L_K + T_R_K * T_R_K) * (T_L_K + T_R_K);
                }
                col.resistance = 1.0 / ((col.convection_hc + col.radiation_hr) * Area) * 1000.0;
            } else if (col.type == 1) { // Solid Layer
                if (geometry_type_ == 0) {
                    col.resistance = thickness_m / (col.conductivity * Area) * 1000.0;
                } else if (geometry_type_ == 2) {
                    double r_next = r + thickness_m;
                    col.resistance = log(r_next / r) / (2.0 * M_PI * geom_height_ * col.conductivity) * 1000.0;
                    r = r_next;
                } else if (geometry_type_ == 3) {
                    double r_next = r + thickness_m;
                    col.resistance = (1.0/r - 1.0/r_next) / (4.0 * M_PI * col.conductivity) * 1000.0;
                    r = r_next;
                } else {
                    double r_next = r + thickness_m;
                    col.resistance = thickness_m / (col.conductivity * (6.0 * r * r)) * 1000.0;
                    r = r_next;
                }
            } else if (col.type == 2) { // Contact Layer
                // resistance set by user
            } else if (col.type == 3) { // Enclosure
                if (geometry_type_ == 0) {
                    col.resistance = thickness_m / (0.024 * Area) * 1000.0;
                } else {
                    double r_next = r + thickness_m;
                    col.resistance = log(r_next / r) / (2.0 * M_PI * geom_height_ * 0.024) * 1000.0;
                    r = r_next;
                }
            }
            R_total += col.resistance / 1000.0;
        }
        
        if (R_total < 1e-9) R_total = 1e-9;
        double Q = (t_left - t_right) / R_total / 1000.0; // kW
        total_heat_flow_ = Q;
        
        double temp = t_left;
        r = geom_diameter_ / 2.0;
        for (size_t i = 0; i < n; ++i) {
            auto& col = columnas_[i];
            col.temp_left = temp;
            col.temp_right = temp - Q * (col.resistance / 1000.0);
            temp = col.temp_right;
            col.heat_flow = Q;
            
            double Area = 1.0;
            if (geometry_type_ == 0) Area = geom_height_ * geom_diameter_;
            else if (geometry_type_ == 1) Area = 6.0 * r * r;
            else if (geometry_type_ == 2) Area = 2.0 * M_PI * r * geom_height_;
            else if (geometry_type_ == 3) Area = 4.0 * M_PI * r * r;
            col.heat_flux = Q / Area;
            
            double thickness_m = col.thickness / 100.0;
            if (col.type == 1 || col.type == 3) {
                r += thickness_m;
            }
        }
    }
    calculated_ = true;
}

void PanelPerdidaCalor::renderizar() {
    ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.65f, 0.0f, 1.0f), "Heat Loss / Heat Transfer Calculator");
    ImGui::Separator();
    ImGui::Spacing();
    
    // Tab sheets
    if (ImGui::Button(tr("Muro"))) { active_sheet_ = 0; } ImGui::SameLine();
    if (ImGui::Button(tr("Techo / Techo"))) { active_sheet_ = 1; } ImGui::SameLine();
    if (ImGui::Button(tr("Fondo"))) { active_sheet_ = 2; }
    
    ImGui::Separator();
    ImGui::Spacing();
    
    // ── Spreadsheet Grid Table ──
    int n_cols = (int)columnas_.size();
    if (ImGui::BeginTable("heat_loss_grid", n_cols + 1, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollX, ImVec2(0, 360))) {
        ImGui::TableSetupColumn("Propiedad", ImGuiTableColumnFlags_WidthFixed, 180.0f);
        for (int i = 0; i < n_cols; ++i) {
            char header_name[32];
            snprintf(header_name, sizeof(header_name), "%d", i + 1);
            ImGui::TableSetupColumn(header_name, ImGuiTableColumnFlags_WidthFixed, 140.0f);
        }
        ImGui::TableHeadersRow();
        
        // Row 1: Layer Type
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Tipo de Capa"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 1);
            ImGui::SetNextItemWidth(-1.0f);
            const char* type_labels[] = { "Surface", "Solid Layer", "Contact Layer", "Enclosure" };
            if (ImGui::Combo("##type", &columnas_[i].type, type_labels, 4)) {
                reciclar_calculos();
            }
            ImGui::PopID();
        }
        
        // Row 2: Layer Material
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Material de Capa"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 2);
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputText("##material", columnas_[i].material, sizeof(columnas_[i].material))) {
                // simple metadata update
            }
            ImGui::PopID();
        }
        
        // Row 3: Surface Material
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Material de Superficie"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 0) {
                ImGui::PushID(i * 100 + 3);
                ImGui::SetNextItemWidth(-1.0f);
                ImGui::InputText("##surf_material", columnas_[i].surface_material, sizeof(columnas_[i].surface_material));
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 4: Temperature left edge (°C)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Temperature (left edge) °C"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 4);
            ImGui::SetNextItemWidth(-1.0f);
            if (columnas_[i].fix_temp_left) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
            if (ImGui::InputDouble("##temp_left", &columnas_[i].temp_left, 0, 0, "%.1f")) {
                reciclar_calculos();
            }
            if (columnas_[i].fix_temp_left) ImGui::PopStyleColor();
            ImGui::PopID();
        }
        
        // Row 5: Temperature right edge (°C)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Temperature (right edge) °C"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 5);
            ImGui::SetNextItemWidth(-1.0f);
            if (columnas_[i].fix_temp_right) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
            if (ImGui::InputDouble("##temp_right", &columnas_[i].temp_right, 0, 0, "%.1f")) {
                reciclar_calculos();
            }
            if (columnas_[i].fix_temp_right) ImGui::PopStyleColor();
            ImGui::PopID();
        }
        
        // Row 6: Thickness (cm)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Thickness cm"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 1 || columnas_[i].type == 3) {
                ImGui::PushID(i * 100 + 6);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputDouble("##thick", &columnas_[i].thickness, 0, 0, "%.3f")) {
                    reciclar_calculos();
                }
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("0.0"));
            }
        }
        
        // Row 7: Calculation Grid
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Cuadrícula de Cálculo"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 1) {
                ImGui::PushID(i * 100 + 7);
                ImGui::SetNextItemWidth(-1.0f);
                ImGui::InputInt("##grid", &columnas_[i].grid_points, 0, 0);
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 8: Average conductivity (k) W/(mK)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Average conductivity (k)"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 1) {
                ImGui::PushID(i * 100 + 8);
                ImGui::SetNextItemWidth(-1.0f);
                if (columnas_[i].fix_conductivity) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
                if (ImGui::InputDouble("##conductivity", &columnas_[i].conductivity, 0, 0, "%.3f")) {
                    reciclar_calculos();
                }
                if (columnas_[i].fix_conductivity) ImGui::PopStyleColor();
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 9: Convection coefficient (hc) W/(m²K)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Convection coeff. (hc)"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 0) {
                ImGui::PushID(i * 100 + 9);
                ImGui::SetNextItemWidth(-1.0f);
                if (columnas_[i].fix_convection) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
                if (ImGui::InputDouble("##convection", &columnas_[i].convection_hc, 0, 0, "%.3f")) {
                    reciclar_calculos();
                }
                if (columnas_[i].fix_convection) ImGui::PopStyleColor();
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 10: Radiation coefficient (hr) W/(m²K)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Radiation coeff. (hr)"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 0) {
                ImGui::PushID(i * 100 + 10);
                ImGui::SetNextItemWidth(-1.0f);
                if (columnas_[i].fix_radiation) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
                if (ImGui::InputDouble("##radiation", &columnas_[i].radiation_hr, 0, 0, "%.3f")) {
                    reciclar_calculos();
                }
                if (columnas_[i].fix_radiation) ImGui::PopStyleColor();
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 11: Thermal Resistance °C/kW
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Thermal Resistance °C/kW"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 11);
            ImGui::SetNextItemWidth(-1.0f);
            if (columnas_[i].fix_resistance) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
            if (ImGui::InputDouble("##resistance", &columnas_[i].resistance, 0, 0, "%.3f")) {
                reciclar_calculos();
            }
            if (columnas_[i].fix_resistance) ImGui::PopStyleColor();
            ImGui::PopID();
        }
        
        // Row 12: Heat Flow kW
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Flujo de Calor kW"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::Text(tr("%.3f"), columnas_[i].heat_flow);
        }
        
        // Row 13: Heat Flux kW/m²
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Flujo de Calor kW/m²"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::Text(tr("%.3f"), columnas_[i].heat_flux);
        }
        
        ImGui::EndTable();
    }
    
    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Spacing();
    
    // ── Bottom Dashboard Columns ──
    ImGui::Columns(3, "heat_loss_dashboard", true);
    
    // Column 1: Total Heat Flow & Shapes
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f), "Total Heat Flow");
    ImGui::SetNextItemWidth(-1.0f);
    ImGui::InputDouble("##total_h_flow", &total_heat_flow_, 0, 0, "%.3f");
    
    ImGui::Spacing();
    ImGui::Text(tr("Shape & Dimensions:"));
    ImGui::RadioButton("Muro", &geometry_type_, 0); ImGui::SameLine();
    if (ImGui::RadioButton("Cylinder", &geometry_type_, 2)) {
        reciclar_calculos();
    }
    ImGui::RadioButton("Cube", &geometry_type_, 1); ImGui::SameLine();
    if (ImGui::RadioButton("Sphere", &geometry_type_, 3)) {
        reciclar_calculos();
    }
    
    ImGui::SetNextItemWidth(100.0f);
    if (ImGui::InputDouble("Height (m)", &geom_height_)) reciclar_calculos();
    ImGui::SameLine();
    ImGui::SetNextItemWidth(100.0f);
    if (ImGui::InputDouble("Diameter (m)", &geom_diameter_)) reciclar_calculos();
    
    ImGui::NextColumn();
    
    // Column 2: Options / Actions
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f), "Options");
    if (ImGui::Button(tr("Fix Selected Value"), ImVec2(-1, 24))) {
        // Toggle fix flags as a demonstration
        for (auto& col : columnas_) {
            col.fix_convection = !col.fix_convection;
        }
    }
    if (ImGui::Button(tr("Tabla del Diagrama"), ImVec2(-1, 24))) {
        // Option to display numeric table profile
    }
    if (ImGui::Button(tr("Dibujar Perfil de Temperatura"), ImVec2(-1, 24))) {
        show_diagram_ = !show_diagram_;
    }
    if (ImGui::Button(tr("Reset to Defaults"), ImVec2(-1, 24))) {
        inicializar_valores_defecto();
    }
    
    ImGui::NextColumn();
    
    // Column 3: Calculate Solvers
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f), "Calculate");
    
    if (ImGui::Button(tr("Temperature Profile Solver"), ImVec2(-1, 28))) {
        calc_type_ = 0;
        calcular_perfil_temperatura();
    }
    if (ImGui::Button(tr("Resolvedor de Flujo de Calor"), ImVec2(-1, 28))) {
        calc_type_ = 1;
        calcular_flujo_calor();
    }
    
    ImGui::Spacing();
    // Hot to cold color bar indicator
    ImGui::Text(tr("HOT")); ImGui::SameLine();
    ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 10);
    ImVec2 cursor = ImGui::GetCursorScreenPos();
    ImGui::GetWindowDrawList()->AddRectFilled(cursor, ImVec2(cursor.x + 100, cursor.y + 16), IM_COL32(255, 0, 0, 255), 2.0f);
    ImGui::GetWindowDrawList()->AddRectFilledMultiColor(cursor, ImVec2(cursor.x + 100, cursor.y + 16), IM_COL32(255, 0, 0, 255), IM_COL32(0, 0, 255, 255), IM_COL32(0, 0, 255, 255), IM_COL32(255, 0, 0, 255));
    ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 110);
    ImGui::Text(tr("COLD"));
    
    ImGui::Columns(1);
    
    // ── ImPlot Diagram Profile popup ──
    if (show_diagram_) {
        ImGui::SetNextWindowSize(ImVec2(600, 400), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Temperature Profile Diagram"), &show_diagram_)) {
            std::vector<double> xs;
            std::vector<double> ys;
            
            double current_x = 0.0;
            xs.push_back(current_x);
            ys.push_back(columnas_[0].temp_left);
            
            for (size_t i = 0; i < columnas_.size(); ++i) {
                auto& col = columnas_[i];
                if (col.type == 1 || col.type == 3) { // Solid Layer / Enclosure
                    double dx = col.thickness * 10.0; // convert cm to mm
                    // Generate interior grid points for a continuous curve
                    int steps = std::max(2, col.grid_points);
                    for (int s = 1; s <= steps; ++s) {
                        double fraction = (double)s / steps;
                        xs.push_back(current_x + dx * fraction);
                        
                        // Linear interpolation or logarithmic (we use linear approximation for display)
                        double t_interp = col.temp_left + fraction * (col.temp_right - col.temp_left);
                        ys.push_back(t_interp);
                    }
                    current_x += dx;
                } else { // Surface or contact layer
                    // sudden temperature drop at interfaces
                    xs.push_back(current_x);
                    ys.push_back(col.temp_right);
                }
            }
            
            char title_lbl[128];
            snprintf(title_lbl, sizeof(title_lbl), "Temperature Profile Wall (Total Heat flow: %.3f kW)", total_heat_flow_);
            if (ImPlot::BeginPlot(title_lbl, ImVec2(-1, -1))) {
                ImPlot::SetupAxes("Distance mm", "Temperature °C");
                ImPlot::PlotLine("Wall Temp", xs.data(), ys.data(), (int)xs.size());
                ImPlot::EndPlot();
            }
            ImGui::End();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
