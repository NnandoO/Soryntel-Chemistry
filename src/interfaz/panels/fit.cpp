#include "shared.hpp"
#include <random>
using namespace cma::interfaz;
cma::interfaz::PanelDataFit::PanelDataFit() {
    refrescar_datos();
}

void cma::interfaz::PanelDataFit::refrescar_datos() {
    data_.clear();
    if (!flowsheet_) return;
    
    // Obtener pares (Temperatura, Entalpía) de los flujos para demostrar el ajuste
    for (const auto& [fid, f] : flowsheet_->mapa_flujos()) {
        float x = (float)f.temperatura;
        float y = (float)f.entalpia;
        data_.push_back({x, y});
    }
}

void cma::interfaz::PanelDataFit::fit_curve() {
    if (data_.empty()) return;
    std::vector<double> x, y;
    for (const auto& point : data_) { x.push_back(point.x); y.push_back(point.y); }
    cma::application::modules::DataFitter fitter;
    // Linear/quadratic use the same coefficient order as evaluate_func.
    const auto fit = fitter.polinomial(x, y, func_type_ == 0 ? 1 : 2);
    if (!fit.ok) { error_ = 0; r2_ = 0; return; }
    std::fill(std::begin(params_), std::end(params_), 0.0f);
    if (func_type_ == 0) { params_[0] = static_cast<float>(fit.coeficientes[1]); params_[1] = static_cast<float>(fit.coeficientes[0]); }
    else { params_[0] = static_cast<float>(fit.coeficientes[2]); params_[1] = static_cast<float>(fit.coeficientes[1]); params_[2] = static_cast<float>(fit.coeficientes[0]); }
    ssr_ = 0;
    for (const auto& point : data_) { const double residual = evaluate_func(point.x) - point.y; ssr_ += static_cast<float>(residual * residual); }
    error_ = std::sqrt(ssr_ / static_cast<float>(data_.size()));
    r2_ = static_cast<float>(fit.r_cuadrado);
}


float cma::interfaz::PanelDataFit::evaluate_func(float x) const {
    if (func_type_ == 0) {
        return params_[0] * x + params_[1];
    } else if (func_type_ == 1) {
        return params_[0] * x * x + params_[1] * x + params_[2];
    } else {
        return (params_[0] + params_[1] * x + params_[2] * x * x) / (1.0f + params_[3] * x + params_[4] * x * x);
    }
}

void cma::interfaz::PanelDataFit::renderizar() {
    ImGui::Text(tr("HSC-Sim Data Fit"));
    ImGui::Separator();
    
    // --- Top Panel: Specifications ---
    ImGui::BeginChild("FitSpecs", ImVec2(0, 100), true);
    ImGui::Columns(2, "spec_cols");
    
    ImGui::TextDisabled(tr("Opciones de Ajuste"));
    ImGui::PushItemWidth(100);
    ImGui::TextWrapped("Mínimos cuadrados con QR; requiere puntos distintos.");
    ImGui::PopItemWidth();
    
    ImGui::NextColumn();
    
    ImGui::TextDisabled(tr("Function Formula"));
    const char* func_items[] = {
        "Y = A*x + B", 
        "Y = A*x^2 + B*x + C"
    };
    if (ImGui::Combo("Formula", &func_type_, func_items, IM_ARRAYSIZE(func_items))) {
        // Reset params
        for(int i=0; i<7; ++i) params_[i] = (i==0) ? 80.0f : 0.1f;
    }
    ImGui::TextDisabled(tr("Points: %d"), (int)data_.size());
    ImGui::Columns(1);
    ImGui::EndChild();
    
    // --- Split Left (Table) and Right (Plot) ---
    float left_width = 300.0f;
    ImGui::BeginChild("FitLeft", ImVec2(left_width, 0), true);
    
    // Parameters Table
    if (ImGui::BeginTable("param_table", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Nombre");
        ImGui::TableSetupColumn("Valor");
        ImGui::TableHeadersRow();
        
        int num_params = (func_type_ == 0) ? 2 : ((func_type_ == 1) ? 4 : 7);
        const char* pnames[] = {"A", "B", "C", "D", "E", "F", "G"};
        
        for (int i = 0; i < num_params; ++i) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text(tr("%s"), pnames[i]);
            ImGui::TableSetColumnIndex(1); 
            ImGui::PushID(i);
            ImGui::PushItemWidth(-FLT_MIN);
            ImGui::InputFloat("##pval", &params_[i], 0.0f, 0.0f, "%.4f");
            ImGui::PopItemWidth();
            ImGui::PopID();
        }
        ImGui::EndTable();
    }
    
    ImGui::Spacing();
    
    // Stats Table
    if (ImGui::BeginTable("stats_table", 2, ImGuiTableFlags_Borders)) {
        ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Error")); ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.2f"), error_);
        ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("R2")); ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.4f"), r2_);
        ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("SSR")); ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.2f"), ssr_);
        ImGui::EndTable();
    }
    
    ImGui::Spacing();
    if (ImGui::Button(tr("Ajustar Curva"), ImVec2(-1, 30))) {
        fit_curve();
    }
    
    if (ImGui::Button(tr("Exportar CSV (Ajuste)"), ImVec2(-1, 25))) {
        std::string fname = "export_fit.csv";
        FILE* f = fopen(fname.c_str(), "w");
        if (f) {
            fprintf(f, "X,Y_data,Y_fit\n");
            for (const auto& pt : data_) {
                fprintf(f, "%.4f,%.4f,%.4f\n", pt.x, pt.y, evaluate_func(pt.x));
            }
            fclose(f);
        }
    }

    if (ImGui::Button(tr("Refrescar del Flowsheet"), ImVec2(-1, 25))) {
        refrescar_datos();
    }
    
    ImGui::EndChild();
    
    ImGui::SameLine();
    
    // --- Plot Area ---
    ImGui::BeginChild("FitPlot", ImVec2(0, 0), true);
    
    if (ImPlot::BeginPlot("Electron Mobility Fit", ImVec2(-1, -1))) {
        ImPlot::SetupAxes("Density", "Y");
        
        // Prepare Scatter Data
        std::vector<float> sx, sy;
        for (const auto& pt : data_) { sx.push_back(pt.x); sy.push_back(pt.y); }
        
        // Prepare Line Data (Fitted Curve)
        std::vector<float> lx, ly;
        for (float x = -3.5f; x <= 2.5f; x += 0.05f) {
            lx.push_back(x);
            ly.push_back(evaluate_func(x));
        }
        
        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 4, ImVec4(1,0,0,1), -1, ImVec4(1,0,0,1));
        ImPlot::PlotScatter("Exp. Data", sx.data(), sy.data(), (int)sx.size());
        
        ImPlot::SetNextLineStyle(ImVec4(0,0,0,1), 2.0f); // Black curve
        ImPlot::PlotLine("Fitted Function", lx.data(), ly.data(), (int)lx.size());
        
        ImPlot::EndPlot();
    }
    
    ImGui::EndChild();
}

#include <cmath>
#include <random>
#include <algorithm>
