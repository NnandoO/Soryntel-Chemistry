#include "shared.hpp"
#include <random>
#include "application/Pca.hpp"
using namespace cma::interfaz;
cma::interfaz::PanelStatisticalAnalysis::PanelStatisticalAnalysis() {
    refrescar_datos();
}

void cma::interfaz::PanelStatisticalAnalysis::refrescar_datos() {
    db_.clear();
    pca_labels_ = {"PC1", "PC2", "PC3", "PC4", "PC5"};
    pca_variance_.clear();
    
    if (!flowsheet_) return;

    int id = 1;
    std::vector<std::vector<double>> observations;
    for (const auto& [fid, f] : flowsheet_->mapa_flujos()) {
        StatDataPoint pt;
        pt.id = id++;
        
        // Asignamos variables termodinámicas reales a los campos del struct StatDataPoint
        pt.al2o3 = (float)f.temperatura; // Usamos al2o3 como Temperatura
        pt.cr2o3 = (float)f.presion;     // Usamos cr2o3 como Presión
        
        double moles_totales = 0;
        for (const auto& [esp_id, mol] : f.flujo_molar) moles_totales += mol;
        pt.zn_rec = (float)moles_totales; // Usamos zn_rec como Moles
        
        pt.cu_rec = (float)f.entalpia;    // Usamos cu_rec como Entalpía
        pt.group = (pt.al2o3 > 298.15f) ? 1 : 0; // Agrupamos por si está caliente o frío
        
        db_.push_back(pt);
        observations.push_back({f.temperatura, f.presion, f.flujo_total(), f.entalpia, f.fraccion_solido});
    }
    for (double variance : cma::application::explained_variance(observations))
        pca_variance_.push_back(static_cast<float>(variance));
}

void cma::interfaz::PanelStatisticalAnalysis::renderizar() {
    // We want a full-screen layout.
    // Left area: 4 quadrants of plots. Right area: Table viewer.
    ImGui::Text(tr("HSC Data - Statistical Analysis Dashboard"));
    ImGui::Separator();
    
    float right_panel_width = 300.0f;
    float content_width = ImGui::GetContentRegionAvail().x - right_panel_width - 8.0f;
    float content_height = ImGui::GetContentRegionAvail().y;
    
    ImGui::BeginChild("PlotsArea", ImVec2(content_width, content_height), false);
    
    // We divide into 2 rows and 2 columns
    float plot_w = content_width * 0.5f - 8.0f;
    float plot_h = content_height * 0.5f - 8.0f;
    
    // --- TOP LEFT: Histogram ---
    ImGui::BeginChild("Quad1", ImVec2(plot_w, plot_h), true);
    ImGui::TextDisabled(tr("Distribution (Histogram)"));
    if (ImPlot::BeginPlot("##hist", ImVec2(-1, -1))) {
        ImPlot::SetupAxes("Valor", "Count");
        std::vector<float> values;
        for (const auto& pt : db_) {
            values.push_back(x_axis_col_ == 0 ? pt.al2o3 : (x_axis_col_ == 1 ? pt.cr2o3 : pt.zn_rec));
        }
        ImPlot::SetNextFillStyle(ImVec4(0.6f, 0.6f, 0.6f, 0.8f));
        ImPlot::PlotHistogram("Frequency", values.data(), (int)values.size(), 30);
        ImPlot::EndPlot();
    }
    ImGui::EndChild();
    
    ImGui::SameLine();
    
    // --- TOP RIGHT: XY Scatter with Groups ---
    ImGui::BeginChild("Quad2", ImVec2(plot_w, plot_h), true);
    ImGui::TextDisabled(tr("XY Correlation (Clustered)"));
    if (ImPlot::BeginPlot("##scatter", ImVec2(-1, -1))) {
        const char* x_label = (x_axis_col_ == 0) ? "Temperatura (K)" : "Presión (atm)";
        const char* y_label = (y_axis_col_ == 0) ? "Temperatura (K)" : "Presión (atm)";
        ImPlot::SetupAxes(x_label, y_label);
        
        std::vector<float> x0, y0, x1, y1;
        for (const auto& pt : db_) {
            float vx = (x_axis_col_ == 0) ? pt.al2o3 : pt.cr2o3;
            float vy = (y_axis_col_ == 0) ? pt.al2o3 : pt.cr2o3;
            if (pt.group == 0) { x0.push_back(vx); y0.push_back(vy); }
            else { x1.push_back(vx); y1.push_back(vy); }
        }
        
        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 3, ImVec4(1,0,0,1), -1, ImVec4(1,0,0,1));
        ImPlot::PlotScatter("Group A", x0.data(), y0.data(), (int)x0.size());
        
        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 3, ImVec4(0,0,1,1), -1, ImVec4(0,0,1,1));
        ImPlot::PlotScatter("Group B", x1.data(), y1.data(), (int)x1.size());
        
        ImPlot::EndPlot();
    }
    ImGui::EndChild();
    
    // --- BOTTOM LEFT: PCA Variance Bar Chart ---
    ImGui::BeginChild("Quad3", ImVec2(plot_w, plot_h), true);
    ImGui::TextDisabled("PCA de propiedades estandarizadas");
    if (pca_variance_.empty()) ImGui::TextWrapped("Requiere al menos dos observaciones con variación.");
    if (ImPlot::BeginPlot("##pca", ImVec2(-1, -1))) {
        ImPlot::SetupAxes("Variable", "Of Variance, %");
        // X-axis ticks
        std::vector<double> ticks = {0, 1, 2, 3, 4};
        std::vector<const char*> labels;
        for (const auto& l : pca_labels_) labels.push_back(l.c_str());
        
        ImPlot::SetupAxisTicks(ImAxis_X1, ticks.data(), (int)ticks.size(), labels.data());
        ImPlot::SetNextFillStyle(ImVec4(0.9f, 0.1f, 0.1f, 0.9f));
        ImPlot::PlotBars("Variance", pca_variance_.data(), (int)pca_variance_.size(), 0.5);
        ImPlot::EndPlot();
    }
    ImGui::EndChild();
    
    ImGui::SameLine();
    
    // --- BOTTOM RIGHT: Time Series Line Graph ---
    ImGui::BeginChild("Quad4", ImVec2(plot_w, plot_h), true);
    ImGui::TextDisabled(tr("Process Evolution (Zn_Rec)"));
    if (ImPlot::BeginPlot("##lines", ImVec2(-1, -1))) {
        ImPlot::SetupAxes("No. Muestra", "Zn_Rec %");
        std::vector<float> x_time, y_zn, y_cu;
        for (const auto& pt : db_) {
            x_time.push_back((float)pt.id);
            y_zn.push_back(pt.zn_rec);
            y_cu.push_back(pt.cu_rec);
        }
        
        ImPlot::SetNextLineStyle(ImVec4(0,0,0.8f,1), 1.5f);
        ImPlot::PlotLine("Zn_Rec", x_time.data(), y_zn.data(), (int)x_time.size());
        
        ImPlot::SetNextLineStyle(ImVec4(0.8f,0,0,0.5f), 1.5f);
        ImPlot::PlotLine("Cu_Rec (Simulated)", x_time.data(), y_cu.data(), (int)x_time.size());
        
        ImPlot::EndPlot();
    }
    ImGui::EndChild();
    
    ImGui::EndChild(); // End PlotsArea
    
    ImGui::SameLine();
    
    // --- RIGHT PANEL: Database Table ---
    ImGui::BeginChild("TableArea", ImVec2(right_panel_width, content_height), true);
    ImGui::TextDisabled(tr("Navegador de BD"));
    ImGui::Separator();
    ImGui::Text(tr("Opciones de Datos:"));
    if (ImGui::Button(tr("Refrescar del Flowsheet"), ImVec2(-1, 25))) {
        refrescar_datos();
    }
    ImGui::Separator();

    
    ImGui::Text(tr("X Axis:"));
    ImGui::RadioButton("Temperatura", &x_axis_col_, 0); ImGui::SameLine();
    ImGui::RadioButton("Presión", &x_axis_col_, 1);
    
    ImGui::Text(tr("Y Axis:"));
    ImGui::RadioButton("Temperatura##y", &y_axis_col_, 0); ImGui::SameLine();
    ImGui::RadioButton("Presión##y", &y_axis_col_, 1);
    
    ImGui::Separator();
    ImGui::Text(tr("Vista de Datos (%d filas)"), (int)db_.size());
    
    if (ImGui::Button(tr("Exportar CSV (Estad.)"), ImVec2(-1, 25))) {
        std::string fname = "export_stats.csv";
        FILE* f = fopen(fname.c_str(), "w");
        if (f) {
            fprintf(f, "ID,Temperatura(K),Presion(atm),Flujo(kmol/s)\n");
            for (const auto& pt : db_) {
                fprintf(f, "%d,%.2f,%.2f,%.2f\n", pt.id, pt.al2o3, pt.cr2o3, pt.zn_rec);
            }
            fclose(f);
        }
    }

    
    if (ImGui::BeginTable("db_table", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, -1))) {
        ImGui::TableSetupColumn("ID", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        ImGui::TableSetupColumn("Temperatura(K)");
        ImGui::TableSetupColumn("Presion(atm)");
        ImGui::TableSetupColumn("Flujo(kmol/s)");
        ImGui::TableHeadersRow();
        
        for (const auto& pt : db_) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text(tr("%d"), pt.id);
            ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.2f"), pt.al2o3);
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("%.2f"), pt.cr2o3);
            ImGui::TableSetColumnIndex(3); ImGui::Text(tr("%.2f"), pt.zn_rec);
        }
        ImGui::EndTable();
    }
    
    ImGui::EndChild();
}
