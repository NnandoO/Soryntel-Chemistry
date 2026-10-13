#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de Exergía
// ═══════════════════════════════════════════════════════════════════════════
void PanelExergia::renderizar() {
    ImGui::Text(tr("Análisis Exergético"));
    ImGui::Separator();
    ImGui::InputDouble("H entrada (kJ/mol)", &h_ent_);
    ImGui::InputDouble("H salida (kJ/mol)", &h_sal_);
    ImGui::InputDouble("H₀ ref. (kJ/mol)", &h0_);
    ImGui::InputDouble("T entrada (K)", &T_ent_);
    ImGui::InputDouble("T salida (K)", &T_sal_);
    ImGui::InputDouble("T₀ ambiente (K)", &T0_);
    ImGui::InputDouble("S entrada (kJ/(mol·K))", &s_ent_);
    ImGui::InputDouble("S salida (kJ/(mol·K))", &s_sal_);
    ImGui::InputDouble("S₀ ref. (kJ/(mol·K))", &s0_);
    ImGui::InputDouble("Trabajo útil (kJ/mol)", &W_util_);
    ImGui::InputDouble("Calor aportado (kJ/mol)", &Q_aporte_);
    if (ImGui::Button(tr("Calcular exergía"), ImVec2(-1,30))) {
        resultado_ = motor_.analizar_proceso(h_ent_, h_sal_, h0_, T_ent_, T_sal_, T0_, s_ent_, s_sal_, s0_, W_util_, Q_aporte_);
        calculado_ = true;
    }
    if (calculado_) {
        ImGui::Separator();
        ImGui::Text(tr("Exergía física: %.4f kJ/mol"), resultado_.exergia_fisica);
        ImGui::Text(tr("Exergía química: %.4f kJ/mol"), resultado_.exergia_quimica);
        ImGui::Text(tr("Exergía destruida: %.4f kJ/mol"), resultado_.exergia_destruida);
        ImGui::Text(tr("Exergía total: %.4f kJ/mol"), resultado_.exergia_total);
        ImGui::Text(tr("Eficiencia exergética: %.2f%%"), resultado_.eficiencia_exergetica);

        double vals[3] = {resultado_.exergia_fisica, resultado_.exergia_quimica, resultado_.exergia_destruida};
        const char* labels[3] = {"Física", "Química", "Destruida"};
        if (ImPlot::BeginPlot("Desglose Exergético", ImVec2(-1, 200), ImPlotFlags_NoTitle)) {
            ImPlot::SetupAxes("Componente", "Exergía (kJ/mol)");
            double max_val = 1.0;
            for (int i = 0; i < 3; i++) if (vals[i] > max_val) max_val = vals[i];
            ImPlot::SetupAxesLimits(-0.5, 2.5, 0, max_val * 1.2);
            for (int i = 0; i < 3; i++) {
                ImPlot::PlotBars(labels[i], &vals[i], 1, 0.5, i - 0.25);
            }
            ImPlot::EndPlot();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Diagramas H, S, Cp, G vs T
// ═══════════════════════════════════════════════════════════════════════════
void PanelDiagramasTermo::recalcular() {
    datos_calculados_.clear();
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    
    for (auto& par : especies_seleccionadas_) {
        if (!par.second) continue;
        const nucleo::Especie* esp = nullptr;
        for (const auto* e : db_especies) {
            if (e->id == par.first) { esp = e; break; }
        }
        if (!esp) continue;
        
        CurvaEspecie ce;
        ce.nombre = esp->formula;
        
        // Calculate curve points
        // Assuming PropertyDiagrams generates standard curve logic
        double cp_a = esp->cp_a, cp_b = esp->cp_b, cp_c = esp->cp_c, cp_d = esp->cp_d;
        double h_ref = esp->h_formacion;
        double s_ref = esp->s_formacion;
        
        ce.datos = motor_.generarCurvas(h_ref, s_ref, cp_a, cp_b, cp_c, cp_d, T_min_, T_max_);
        for (const auto& p : ce.datos) {
            ce.Ts_cache.push_back(p.T);
            double y = 0;
            switch (curva_activa_) {
                case 0: y = p.H; break;
                case 1: y = p.S; break;
                case 2: y = p.Cp; break;
                case 3: y = p.G; break;
                case 4: y = p.G; break; // Delta G approximated as G in this MVP
            }
            ce.Ys_cache.push_back(y);
        }
        datos_calculados_.push_back(ce);
    }
    calculado_ = !datos_calculados_.empty();
}

void PanelDiagramasTermo::renderizar() {
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    
    // UI layout: Left panel (Species selection), Right panel (Controls & Plot)
    ImGui::Columns(2, "DiagramsLayout", true);
    ImGui::SetColumnWidth(0, 300.0f);
    
    // -- Left Column: Species Spreadsheet --
    ImGui::Text(tr("Especies Activas"));
    if (ImGui::Button(tr("Examinar Base de Datos"), ImVec2(-1, 0))) {
        ModalBuscadorDB::instancia().abrir("Buscar Especie (Termo)");
    }
    
    ImGui::BeginChild("SpeciesList", ImVec2(0, -1), true);
    if (ImGui::BeginTable("SpeciesTable", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Especie", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("TMin (K)", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("TMax (K)", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableHeadersRow();
        
        for (auto& par : especies_seleccionadas_) {
            if (!par.second) continue;
            const nucleo::Especie* esp = nullptr;
            for (const auto* e : db_especies) {
                if (e->id == par.first) { esp = e; break; }
            }
            if (!esp) continue;
            
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::Text(tr("%s"), esp->formula.c_str());
            ImGui::TableSetColumnIndex(1);
            ImGui::Text(tr("%.2f"), T_min_);
            ImGui::TableSetColumnIndex(2);
            ImGui::Text(tr("%.2f"), T_max_);
        }
        ImGui::EndTable();
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();
    // -- Right Column: Controls and Plot --
    
    ImGui::Text(tr("Tipo de Diagrama"));
    ImGui::SetNextItemWidth(-1);
    if (ImGui::Combo("##Curva", &curva_activa_, "H (Enthalpy)\0S (Entropy)\0Cp (Heat Capacity)\0G (Gibbs Free Energy)\0Delta G (Ellingham)\0")) {
        recalcular();
    }
    
    ImGui::Spacing();
    ImGui::Text(tr("X-axis Range (Temperature K)"));
    ImGui::PushItemWidth(100);
    bool range_changed = false;
    range_changed |= ImGui::InputDouble("MIN##X", &T_min_, 0, 0, "%.0f");
    ImGui::SameLine();
    range_changed |= ImGui::InputDouble("MAX##X", &T_max_, 0, 0, "%.0f");
    ImGui::PopItemWidth();
    
    if (range_changed) recalcular();
    
    if (ImGui::Button(tr("Update Diagram"), ImVec2(-1,30))) {
        recalcular();
    }
    
    if (calculado_ && !datos_calculados_.empty()) {
        ImGui::Separator();
        static const char* nombres[] = {"H (kJ/mol)", "S (J/(mol·K))", "Cp (J/(mol·K))", "G (kJ/mol)", "Delta G (kJ/mol)"};
        
        if (ImPlot::BeginPlot("Thermochemical Data Diagrams", ImVec2(-1, -1))) {
            ImPlot::SetupAxes("Temperature (K)", nombres[curva_activa_]);
            ImPlot::SetupAxisLimits(ImAxis_X1, T_min_, T_max_);
            
            for (const auto& ce : datos_calculados_) {
                ImPlot::PlotLine(ce.nombre.c_str(), ce.Ts_cache.data(), ce.Ys_cache.data(), (int)ce.Ts_cache.size());
            }
            ImPlot::EndPlot();
        }
    } else {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.5f, 0.5f, 1.0f), "Add species from database and press Update Diagram to render.");
    }
    
    ImGui::Columns(1);
    
    // -- Modals --
    if (show_select_species_modal_) {
        ImGui::OpenPopup("Browse Database");
    }
    
    ImGui::SetNextWindowSize(ImVec2(600, 500), ImGuiCond_FirstUseEver);
    if (ImGui::BeginPopupModal("Browse Database", &show_select_species_modal_)) {
        ImGui::InputText(tr("Search"), search_buffer_, IM_ARRAYSIZE(search_buffer_));
        std::string search_str = search_buffer_;
        std::transform(search_str.begin(), search_str.end(), search_str.begin(), ::tolower);
        
        if (ImGui::BeginTable("DBTable", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, -40))) {
            ImGui::TableSetupScrollFreeze(0, 1);
            ImGui::TableSetupColumn("Selección", ImGuiTableColumnFlags_WidthFixed, 50.0f);
            ImGui::TableSetupColumn("ID", ImGuiTableColumnFlags_WidthFixed, 50.0f);
            ImGui::TableSetupColumn("Nombre", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("Fórmula", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableHeadersRow();
            
            for (const auto* esp : db_especies) {
                std::string nm = esp->nombre;
                std::string fm = esp->formula;
                std::string nml = nm, fml = fm;
                std::transform(nml.begin(), nml.end(), nml.begin(), ::tolower);
                std::transform(fml.begin(), fml.end(), fml.begin(), ::tolower);
                
                if (search_str.empty() || nml.find(search_str) != std::string::npos || fml.find(search_str) != std::string::npos) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0);
                    bool selected = especies_seleccionadas_[esp->id];
                    if (ImGui::Checkbox((std::string("##sel") + std::to_string(esp->id)).c_str(), &selected)) {
                        especies_seleccionadas_[esp->id] = selected;
                    }
                    ImGui::TableSetColumnIndex(1);
                    ImGui::Text(tr("%ld"), esp->id);
                    ImGui::TableSetColumnIndex(2);
                    ImGui::Text(tr("%s"), nm.c_str());
                    ImGui::TableSetColumnIndex(3);
                    ImGui::Text(tr("%s"), fm.c_str());
                }
            }
            ImGui::EndTable();
        }
        
        if (ImGui::Button(tr("Cerrar"), ImVec2(120, 0))) {
            ImGui::CloseCurrentPopup();
            show_select_species_modal_ = false;
            recalcular();
        }
        ImGui::EndPopup();
    }
}

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
