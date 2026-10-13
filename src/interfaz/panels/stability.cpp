#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de Diagrama de Estabilidad
// ═══════════════════════════════════════════════════════════════════════════
struct EvaluatedSpecies {
    int id;
    std::string formula;
    double n_m = 0;
    double n_g1 = 0;
    double n_g2 = 0;
    ImVec4 color;
};

// Simple pseudo-random color generator based on ID
ImVec4 color_from_id(int id) {
    float r = ((id * 37) % 255) / 255.0f;
    float g = ((id * 73) % 255) / 255.0f;
    float b = ((id * 113) % 255) / 255.0f;
    return ImVec4(r*0.6f + 0.2f, g*0.6f + 0.2f, b*0.6f + 0.2f, 1.0f);
}

void PanelEstabilidad::recalcular() {
    heatmap_.labels.clear();
    heatmap_.grid.assign(heatmap_.rows * heatmap_.cols, -1);
    
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    std::vector<EvaluatedSpecies> active_species;
    
    // Parse the gas formulas (e.g., O2 -> O, 2)
    auto parser = nucleo::TablaPeriodica::instancia();
    auto g1_atoms = parser.analizar_formula(gas1_element_);
    auto g2_atoms = parser.analizar_formula(gas2_element_);
    
    std::string g1_el = g1_atoms.empty() ? "" : g1_atoms[0].elem;
    double g1_factor = g1_atoms.empty() ? 1.0 : g1_atoms[0].cuenta;
    if (g1_factor == 0) g1_factor = 1.0;
    
    std::string g2_el = g2_atoms.empty() ? "" : g2_atoms[0].elem;
    double g2_factor = g2_atoms.empty() ? 1.0 : g2_atoms[0].cuenta;
    if (g2_factor == 0) g2_factor = 1.0;
    
    const nucleo::Especie* esp_g1 = nullptr;
    const nucleo::Especie* esp_g2 = nullptr;
    for (const auto* e : db_especies) {
        if (e->formula == gas1_element_ && e->fase_std == nucleo::Fase::Gas) esp_g1 = e;
        if (e->formula == gas2_element_ && e->fase_std == nucleo::Fase::Gas) esp_g2 = e;
    }
    if (!esp_g1) { for (const auto* e : db_especies) if (e->formula == gas1_element_) { esp_g1 = e; break; } }
    if (!esp_g2) { for (const auto* e : db_especies) if (e->formula == gas2_element_) { esp_g2 = e; break; } }
    
    for (auto& par : especies_seleccionadas_) {
        if (!par.second) continue;
        const nucleo::Especie* esp = nullptr;
        for (const auto* e : db_especies) {
            if (e->id == par.first) { esp = e; break; }
        }
        if (!esp) continue;
        
        EvaluatedSpecies ev;
        ev.id = esp->id;
        ev.formula = esp->formula;
        ev.color = color_from_id(esp->id);
        
        auto atoms = parser.analizar_formula(esp->formula);
        for (const auto& at : atoms) {
            if (at.elem == main_element_) ev.n_m += at.cuenta;
            else if (at.elem == g1_el) ev.n_g1 += at.cuenta;
            else if (at.elem == g2_el) ev.n_g2 += at.cuenta;
        }
        
        if (ev.n_m > 0) {
            active_species.push_back(ev);
            heatmap_.labels[ev.id] = ev.formula;
        }
    }
    
    if (active_species.empty()) return;
    
    double dx = (x_max_ - x_min_) / heatmap_.cols;
    double dy = (y_max_ - y_min_) / heatmap_.rows;
    
    for (int r = 0; r < heatmap_.rows; ++r) {
        double y = y_min_ + (heatmap_.rows - 1 - r) * dy; // ImPlot heatmaps are top-to-bottom by default unless ColMajor/origin adjusted, but let's compute standard Cartesian. Actually ImPlot Y axis goes bottom to top. 
        // We will just map (c, r) directly to grid[r*cols + c] assuming standard ImPlot orientation.
        y = y_min_ + r * dy;
        
        for (int c = 0; c < heatmap_.cols; ++c) {
            double x = x_min_ + c * dx;
            
            double T = 298.15;
            double logp1 = 0, logp2 = 0;
            
            int fixed_var = 0;
            if (xaxis_var_ != 0 && yaxis_var_ != 0) fixed_var = 0;
            else if (xaxis_var_ != 1 && yaxis_var_ != 1) fixed_var = 1;
            else if (xaxis_var_ != 2 && yaxis_var_ != 2) fixed_var = 2;
            
            if (fixed_var == 0) T = fixed_val_;
            else if (fixed_var == 1) logp1 = fixed_val_;
            else if (fixed_var == 2) logp2 = fixed_val_;
            
            if (xaxis_var_ == 0) T = x;
            else if (xaxis_var_ == 1) logp1 = x;
            else if (xaxis_var_ == 2) logp2 = x;
            
            if (yaxis_var_ == 0) T = y;
            else if (yaxis_var_ == 1) logp1 = y;
            else if (yaxis_var_ == 2) logp2 = y;
            
            double min_G = 1e30;
            int best_id = -1;
            
            for (const auto& ev : active_species) {
                const nucleo::Especie* esp = nullptr;
                for (const auto* e : db_especies) { if (e->id == ev.id) { esp = e; break; } }
                if (!esp) continue;
                
                double Gf = esp->energia_gibbs(T); // kJ/mol
                double R_kJ = 0.008314462618;
                
                double mu0_g1 = esp_g1 ? esp_g1->energia_gibbs(T) : 0.0;
                double mu0_g2 = esp_g2 ? esp_g2->energia_gibbs(T) : 0.0;
                
                double mu_g1 = mu0_g1 + R_kJ * T * logp1 * 2.302585093; // log10 to ln
                double mu_g2 = mu0_g2 + R_kJ * T * logp2 * 2.302585093;
                
                double G_norm = (Gf - (ev.n_g1 / g1_factor) * mu_g1 - (ev.n_g2 / g2_factor) * mu_g2) / ev.n_m;
                
                if (G_norm < min_G) {
                    min_G = G_norm;
                    best_id = ev.id;
                }
            }
            heatmap_.grid[r * heatmap_.cols + c] = best_id;
        }
    }
    calculado_ = true;
}

void PanelEstabilidad::renderizar() {
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    
    ImGui::Columns(2, "PredominanceLayout", true);
    ImGui::SetColumnWidth(0, 320.0f);
    
    // -- Left Column --
    ImGui::Text(tr("Parámetros del Diagrama"));
    ImGui::Separator();
    
    ImGui::InputText(tr("Main Element"), main_element_, sizeof(main_element_));
    ImGui::InputText(tr("Gas 1 (ej. O2)"), gas1_element_, sizeof(gas1_element_));
    ImGui::InputText(tr("Gas 2 (ej. Cl2)"), gas2_element_, sizeof(gas2_element_));
    
    ImGui::Spacing();
    ImGui::Text(tr("Filtrar Especies por Elementos"));
    ImGui::BeginChild("SpeciesListTpp", ImVec2(0, 200), true);
    if (ImGui::BeginTable("TppSpeciesTable", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Selección", ImGuiTableColumnFlags_WidthFixed, 40.0f);
        ImGui::TableSetupColumn("Fórmula", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableHeadersRow();
        
        auto parser = nucleo::TablaPeriodica::instancia();
        auto main_atoms = parser.analizar_formula(main_element_);
        auto g1_atoms = parser.analizar_formula(gas1_element_);
        auto g2_atoms = parser.analizar_formula(gas2_element_);
        
        std::set<std::string> valid_elements;
        for (const auto& at : main_atoms) valid_elements.insert(at.elem);
        for (const auto& at : g1_atoms) valid_elements.insert(at.elem);
        for (const auto& at : g2_atoms) valid_elements.insert(at.elem);
        
        // If main_element_ is just "Fe", analizador might find it. If not, fallback.
        std::string m_el = main_atoms.empty() ? main_element_ : main_atoms[0].elem;
        
        for (const auto* esp : db_especies) {
            auto atoms = parser.analizar_formula(esp->formula);
            bool has_other = false;
            for (const auto& at : atoms) {
                if (valid_elements.find(at.elem) == valid_elements.end()) has_other = true;
            }
            
            // Only show species that are composed exclusively of the valid elements
            if (!atoms.empty() && !has_other) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                bool selected = especies_seleccionadas_[esp->id];
                if (ImGui::Checkbox((std::string("##sel") + std::to_string(esp->id)).c_str(), &selected)) {
                    especies_seleccionadas_[esp->id] = selected;
                }
                ImGui::TableSetColumnIndex(1);
                ImGui::Text(tr("%s"), esp->formula.c_str());
            }
        }
        ImGui::EndTable();
    }
    ImGui::EndChild();
    if (ImGui::Button(tr("Select All Filtered"))) {
        // ... (can add logic later if needed)
    }
    
    ImGui::Spacing();
    ImGui::Text(tr("Configurar Ejes"));
    const char* var_names[] = {"Temperature (K)", "log p(Gas 1)", "log p(Gas 2)"};
    
    bool changed = false;
    changed |= ImGui::Combo("X Axis", &xaxis_var_, var_names, 3);
    changed |= ImGui::InputDouble("X Min", &x_min_);
    changed |= ImGui::InputDouble("X Max", &x_max_);
    
    changed |= ImGui::Combo("Y Axis", &yaxis_var_, var_names, 3);
    changed |= ImGui::InputDouble("Y Min", &y_min_);
    changed |= ImGui::InputDouble("Y Max", &y_max_);
    
    int fixed_var = 0;
    if (xaxis_var_ != 0 && yaxis_var_ != 0) fixed_var = 0;
    else if (xaxis_var_ != 1 && yaxis_var_ != 1) fixed_var = 1;
    else if (xaxis_var_ != 2 && yaxis_var_ != 2) fixed_var = 2;
    
    char fixed_label[64];
    snprintf(fixed_label, sizeof(fixed_label), "Fixed Value: %s", var_names[fixed_var]);
    changed |= ImGui::InputDouble(fixed_label, &fixed_val_);
    
    if (ImGui::Button(tr("Dibujar Diagrama"), ImVec2(-1,30))) {
        recalcular();
    }
    
    ImGui::NextColumn();
    // -- Right Column --
    if (calculado_) {
        static bool cmap_added = false;
        if (!cmap_added) {
            std::vector<ImU32> palette(256);
            for (int i = 0; i < 256; ++i) {
                palette[i] = ImGui::ColorConvertFloat4ToU32(color_from_id(i));
            }
            ImPlot::AddColormap("TppPalette", palette.data(), 256, true);
            cmap_added = true;
        }
        
        std::map<int, int> id_to_color_idx;
        int idx = 0;
        for (const auto& pair : heatmap_.labels) {
            id_to_color_idx[pair.first] = idx++;
        }
        
        // Map grid of IDs to grid of color indices for PlotHeatmap
        std::vector<double> mapped_grid(heatmap_.rows * heatmap_.cols, 0.0);
        for (size_t i = 0; i < heatmap_.grid.size(); ++i) {
            int id = heatmap_.grid[i];
            if (id_to_color_idx.count(id)) {
                mapped_grid[i] = id_to_color_idx[id];
            }
        }
        
        ImPlot::PushColormap("TppPalette");
        
        if (ImPlot::BeginPlot("Predominance Diagram", ImVec2(-1, -1))) {
            ImPlot::SetupAxes(var_names[xaxis_var_], var_names[yaxis_var_]);
            ImPlot::SetupAxisLimits(ImAxis_X1, x_min_, x_max_);
            ImPlot::SetupAxisLimits(ImAxis_Y1, y_min_, y_max_);
            
            // Plot Heatmap
            ImPlot::PlotHeatmap("##stability", mapped_grid.data(), heatmap_.rows, heatmap_.cols, 0, 255, nullptr, 
                                ImPlotPoint(x_min_, y_min_), ImPlotPoint(x_max_, y_max_));
                                
            ImPlot::EndPlot();
        }
        ImPlot::PopColormap();
        
        // Render legend
        ImGui::Text(tr("Legend:"));
        ImGui::BeginChild("Legend", ImVec2(0, 0), false, ImGuiWindowFlags_HorizontalScrollbar);
        for (const auto& pair : heatmap_.labels) {
            int c_idx = id_to_color_idx[pair.first];
            ImVec4 c = color_from_id(c_idx);
            ImGui::ColorButton(pair.second.c_str(), c);
            ImGui::SameLine();
            ImGui::Text(tr("%s"), pair.second.c_str());
            ImGui::SameLine(0, 20);
        }
        ImGui::EndChild();
    } else {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5,0.5,0.5,1), "Select species and click Draw Diagram.");
    }
    
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════
#include "../lpp_impl.cpp"
// ═══════════════════════════════════════════════════════════════════════════
// Panel de Agua (Tablas de Vapor)
// ═══════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
