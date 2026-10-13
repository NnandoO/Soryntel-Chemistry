#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de Simulación Dinámica
// ═══════════════════════════════════════════════════════════════════════════
void PanelSimulacionDinamica::renderizar() {
    ImGui::Text(tr("Simulación Dinámica (ODE)"));
    ImGui::Separator();
    ImGui::Combo("Modo", &modo_, "Vaciado de tanque\0Horno (enfriamiento)\0Reactor Batch\0");
    ImGui::Separator();

    ImGui::InputDouble("t inicial (s)", &t_inicial_);
    ImGui::InputDouble("t final (s)", &t_final_);
    ImGui::InputDouble("Paso (s)", &paso_);

    if (modo_ == 0) {
        ImGui::InputDouble("Área tanque (m²)", &A_tanque_);
        ImGui::InputDouble("Área orificio (m²)", &A_orificio_);
        ImGui::InputDouble("Altura inicial (m)", &h0_);
        if (ImGui::Button(tr("Calcular vaciado"), ImVec2(-1,30))) {
            double g = 9.81, Cd = 0.62;
            double t_vaciado = motor_.tiempo_vaciado_tanque(A_tanque_, A_orificio_, h0_, Cd);
            resultado_.tiempos.clear();
            resultado_.variables.clear();
            int n = (int)(t_final_ / paso_);
            double c = Cd * A_orificio_ * std::sqrt(2*g) / (2 * A_tanque_);
            double sqrt_h0 = std::sqrt(h0_);
            for (int i = 0; i <= n; i++) {
                double t = i * paso_;
                double sqrt_h = sqrt_h0 - c * t;
                double h = (sqrt_h > 0) ? sqrt_h * sqrt_h : 0;
                resultado_.tiempos.push_back(t);
                resultado_.variables["h (m)"].push_back(h);
            }
            resultado_.mensaje = "Tiempo de vaciado: " + std::to_string(t_vaciado) + " s";
            calculado_ = true;
        }
    } else if (modo_ == 1) {
        ImGui::InputDouble("T inicial (K)", &T0_horno_);
        ImGui::InputDouble("T ambiente (K)", &T_amb_);
        ImGui::InputDouble("Masa (kg)", &masa_);
        ImGui::InputDouble("Cp (J/(kg·K))", &cp_);
        ImGui::InputDouble("hA (W/K)", &hA_);
        if (ImGui::Button(tr("Calcular perfil térmico"), ImVec2(-1,30))) {
            auto perfil = motor_.perfil_temperatura_horno(T0_horno_, T_amb_, masa_, cp_, hA_, t_final_, paso_);
            resultado_.tiempos.clear();
            resultado_.variables.clear();
            int n = (int)perfil.size();
            for (int i = 0; i < n; i++) {
                resultado_.tiempos.push_back(i * paso_);
                resultado_.variables["T (K)"].push_back(perfil[i]);
            }
            resultado_.mensaje = "T final: " + std::to_string(perfil.back()) + " K";
            calculado_ = true;
        }
    } else {
        ImGui::InputDouble("C₀ (mol/L)", &C0_);
        ImGui::InputDouble("k₀ (factor pre-exp)", &k0_);
        ImGui::InputDouble("Ea (J/mol)", &Ea_);
        ImGui::InputDouble("T (K)", &T_react_);
        if (ImGui::Button(tr("Resolver batch"), ImVec2(-1,30))) {
            resultado_ = motor_.reactor_batch_dinamico(T_react_, 1.0, k0_, Ea_, {C0_}, 1, t_final_, paso_);
            calculado_ = resultado_.ok;
            if (resultado_.ok && !resultado_.variables.empty()) {
                auto it = resultado_.variables.begin();
                resultado_.mensaje = "C final: " + std::to_string(it->second.back()) + " mol/L";
            } else {
                resultado_.mensaje = "Error en la simulación";
            }
        }
    }

    if (calculado_ && !resultado_.tiempos.empty()) {
        ImGui::Separator();
        ImGui::Text(tr("Resultados (%zu puntos)"), resultado_.tiempos.size());

        if (ImPlot::BeginPlot("Evolución Temporal", ImVec2(-1, 250), ImPlotFlags_NoTitle)) {
            ImPlot::SetupAxes("Tiempo (s)", modo_ == 1 ? "T (K)" : modo_ == 0 ? "h (m)" : "C (mol/L)");
            auto it = resultado_.variables.begin();
            if (it != resultado_.variables.end()) {
                std::vector<double>& t = resultado_.tiempos;
                std::vector<double>& v = it->second;
                if (!v.empty()) {
                    ImPlot::SetupAxesLimits(0, t.back(), 0, *std::max_element(v.begin(), v.end()) * 1.1);
                }
                ImPlot::PlotLine("Variable", t.data(), v.data(), (int)std::min(t.size(), v.size()));
            }
            ImPlot::EndPlot();
        }

        auto it = resultado_.variables.begin();
        if (it != resultado_.variables.end() && !it->second.empty()) {
            ImGui::Text(tr("Valor final: %.4f"), it->second.back());
        }

        if (!resultado_.mensaje.empty()) {
            ImGui::TextWrapped(tr("%s"), resultado_.mensaje.c_str());
        }
    } else if (calculado_) {
        ImGui::Separator();
        ImGui::TextWrapped(tr("%s"), resultado_.mensaje.c_str());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Reconciliación de Datos
// ═══════════════════════════════════════════════════════════════════════════
void PanelReconciliacion::inicializar_datos(diagrama_flujo::Flowsheet* fs) {
    if (!fs || cargado_inicial_) return;
    unidades_.clear();
    corrientes_.clear();
    
    for (auto& n : fs->nodos()) {
        if (n.tipo == nucleo::TipoNodo::CajaTexto || n.tipo == nucleo::TipoNodo::Rectangulo || n.tipo == nucleo::TipoNodo::Elipse) continue;
        ReconciliacionUnit u;
        u.name = n.nombre;
        u.selected = true;
        u.psd_changing = (n.tipo == nucleo::TipoNodo::MolinoSAG || n.tipo == nucleo::TipoNodo::MolinoBolas || n.tipo == nucleo::TipoNodo::TrituradoraGiratoria);
        unidades_.push_back(u);
    }
    
    for (auto& [fid, f] : fs->mapa_flujos()) {
        ReconciliacionStream s;
        s.id = fid;
        s.name = f.nombre.empty() ? ("Stream " + std::to_string(fid)) : f.nombre;
        s.selected = f.mb_selected;
        s.assayed = f.mb_assayed;
        
        if (f.measured_flow >= 0.0) {
            s.measured_flow = f.measured_flow;
        } else {
            s.measured_flow = f.flujo_total();
            if (s.measured_flow <= 0.0) s.measured_flow = 100.0;
        }
        
        if (f.sd >= 0.0) {
            s.sd = f.sd;
        } else {
            s.sd = s.measured_flow * 0.05;
            if (s.sd <= 0.0) s.sd = 2.0;
        }
        corrientes_.push_back(s);
    }
    cargado_inicial_ = true;
}

void PanelReconciliacion::ejecutar_balance() {
    std::vector<motores::Medicion> ent;
    std::vector<motores::Medicion> sal;
    
    for (auto& s : corrientes_) {
        if (!s.selected) continue;
        motores::Medicion m = { s.name, s.measured_flow, s.sd };
        if (s.assayed) {
            ent.push_back(m);
        } else {
            sal.push_back(m);
        }
    }
    
    if (ent.empty() || sal.empty()) {
        ent.clear(); sal.clear();
        int half = (int)corrientes_.size() / 2;
        if (half < 1) half = 1;
        for (int i = 0; i < (int)corrientes_.size(); i++) {
            auto& s = corrientes_[i];
            motores::Medicion m = { s.name, s.measured_flow, s.sd };
            if (i < half) ent.push_back(m);
            else sal.push_back(m);
        }
    }
    
    resultado_ = motor_.reconciliar_balance_masa(ent, sal);
    calculado_ = resultado_.ok;
    
    if (calculado_) {
        for (auto& s : corrientes_) {
            auto it = resultado_.valores_reconciliados.find(s.name);
            if (it != resultado_.valores_reconciliados.end()) {
                s.reconciled_flow = it->second;
                s.reconciled_error = resultado_.residuos[s.name];
            } else {
                s.reconciled_flow = s.measured_flow;
                s.reconciled_error = 0.0;
            }
        }
    }
}

void PanelReconciliacion::renderizar(diagrama_flujo::Flowsheet* flowsheet) {
    if (!flowsheet) {
        ImGui::Text(tr("Cargue un flowsheet para realizar balance de masa."));
        return;
    }
    
    inicializar_datos(flowsheet);

    // ── Barra de herramientas superior ──
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(6, 4));
    if (ImGui::Button(tr("Abrir"))) {
        cargado_inicial_ = false;
        inicializar_datos(flowsheet);
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Guardar"))) {
        for (auto& s : corrientes_) {
            auto* f = flowsheet->flujo_por_id(s.id);
            if (f) {
                f->measured_flow = s.measured_flow;
                f->sd = s.sd;
                f->mb_selected = s.selected;
                f->mb_assayed = s.assayed;
            }
        }
        ImGui::OpenPopup("mb_saved");
    }
    if (ImGui::BeginPopup("mb_saved")) {
        ImGui::Text(tr("Configuration saved to flowsheet streams!"));
        if (ImGui::Button(tr("Cerrar"))) ImGui::CloseCurrentPopup();
        ImGui::EndPopup();
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Importar Datos"))) {
        FILE* zenity = adapters::dialog_pipe("zenity --file-selection --file-filter='*.csv' --title='Import Raw Mass Balance Measurements' 2>/dev/null");
        if (zenity) {
            char path[512] = {0};
            if (fgets(path, sizeof(path), zenity)) {
                std::string path_str(path);
                if (!path_str.empty() && path_str.back() == '\n') path_str.pop_back();
                if (!path_str.empty()) {
                    FILE* fp = fopen(path_str.c_str(), "r");
                    if (fp) {
                        char line[256];
                        // Skip header
                        if (fgets(line, sizeof(line), fp)) {
                            while (fgets(line, sizeof(line), fp)) {
                                std::string l(line);
                                auto parts = split_str(l, ",");
                                if (parts.size() >= 2) {
                                    std::string name = trim_str(parts[0]);
                                    try {
                                        double val = std::stod(parts[1]);
                                        double sd_val = (parts.size() >= 3) ? std::stod(parts[2]) : (val * 0.05);
                                        
                                        // Find stream by name (case-insensitive)
                                        std::string name_lower = name;
                                        std::transform(name_lower.begin(), name_lower.end(), name_lower.begin(), ::tolower);
                                        for (auto& s : corrientes_) {
                                            std::string s_lower = s.name;
                                            std::transform(s_lower.begin(), s_lower.end(), s_lower.begin(), ::tolower);
                                            if (s_lower == name_lower) {
                                                s.measured_flow = val;
                                                s.sd = sd_val;
                                                // Sync with flowsheet
                                                auto* f = flowsheet->flujo_por_id(s.id);
                                                if (f) {
                                                    f->measured_flow = val;
                                                    f->sd = sd_val;
                                                }
                                            }
                                        }
                                    } catch (...) {}
                                }
                            }
                        }
                        fclose(fp);
                    }
                }
            }
            fclose(zenity);
        }
    }
    ImGui::PopStyleVar();
    ImGui::Separator();
    ImGui::Spacing();

    // ── Layout de tres paneles (Splitters) ──
    // Izquierda: Balancing Navigator (25% ancho)
    // Centro: Data Grid (50% ancho)
    // Derecha: Properties Inspector (25% ancho)
    ImGui::Columns(3, "mass_balance_split", true);
    static bool init_w = true;
    if (init_w) {
        float w = ImGui::GetWindowWidth();
        ImGui::SetColumnWidth(0, w * 0.25f);
        ImGui::SetColumnWidth(1, w * 0.50f);
        ImGui::SetColumnWidth(2, w * 0.25f);
        init_w = false;
    }

    // ════════════════════════════════════════════════════════════════════════
    // Panel Izquierdo: Balancing Navigator
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("nav_pane");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.6f, 0.7f, 0.9f, 1.0f), "Balancing Navigator");
        ImGui::Separator();
        
        auto render_nav_item = [&](const char* label, SeccionNavegacion target) {
            bool sel = (seccion_ == target);
            if (ImGui::Selectable(label, sel)) {
                seccion_ = target;
                fila_seleccionada_ = -1;
            }
        };

        if (ImGui::CollapsingHeader("Experimental Data", ImGuiTreeNodeFlags_DefaultOpen)) {
            render_nav_item("Units", SeccionNavegacion::Units);
            render_nav_item("Streams", SeccionNavegacion::Streams);
            render_nav_item("Variables", SeccionNavegacion::Variables);
            render_nav_item("Size Fractions", SeccionNavegacion::SizeFractions);
            render_nav_item("Data Sets", SeccionNavegacion::DataSets);
            render_nav_item("Measurement Data", SeccionNavegacion::MeasurementData);
        }
        
        if (ImGui::CollapsingHeader("Estado de Datos")) {
            render_nav_item("Stream View", SeccionNavegacion::StreamView);
            render_nav_item("Variable View", SeccionNavegacion::VariableView);
        }
        
        if (ImGui::CollapsingHeader("Standard Deviations")) {
            render_nav_item("SD - Stream View", SeccionNavegacion::SDStream);
            render_nav_item("SD - Variable View", SeccionNavegacion::SDVariable);
        }
        
        render_nav_item("Balancing Solver", SeccionNavegacion::Balancing);
        render_nav_item("Reporting & Stats", SeccionNavegacion::Reporting);
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();

    // ════════════════════════════════════════════════════════════════════════
    // Panel Central: Data Grid / Central Editor
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("grid_pane");
    {
        if (seccion_ == SeccionNavegacion::Units) {
            ImGui::Text(tr("Process Units Configuration"));
            if (ImGui::BeginTable("units_grid", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                ImGui::TableSetupColumn("Selected", ImGuiTableColumnFlags_WidthFixed, 60.0f);
                ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("PSD Changing", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableHeadersRow();
                
                for (int i = 0; i < (int)unidades_.size(); i++) {
                    auto& u = unidades_[i];
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn();
                    ImGui::Checkbox(("##sel_u"+std::to_string(i)).c_str(), &u.selected);
                    
                    ImGui::TableNextColumn();
                    bool row_sel = (fila_seleccionada_ == i);
                    if (ImGui::Selectable(u.name.c_str(), row_sel, ImGuiSelectableFlags_SpanAllColumns)) {
                        fila_seleccionada_ = i;
                    }
                    
                    ImGui::TableNextColumn();
                    ImGui::Checkbox(("##psd_u"+std::to_string(i)).c_str(), &u.psd_changing);
                }
                ImGui::EndTable();
            }
        }
        else if (seccion_ == SeccionNavegacion::Streams) {
            ImGui::Text(tr("Process Streams Settings"));
            if (ImGui::BeginTable("streams_grid", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                ImGui::TableSetupColumn("Selected", ImGuiTableColumnFlags_WidthFixed, 60.0f);
                ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Input Stream", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableSetupColumn("Measured Flow", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableSetupColumn("Std Dev (SD)", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                ImGui::TableHeadersRow();
                
                for (int i = 0; i < (int)corrientes_.size(); i++) {
                    auto& s = corrientes_[i];
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn();
                    ImGui::Checkbox(("##sel_s"+std::to_string(i)).c_str(), &s.selected);
                    
                    ImGui::TableNextColumn();
                    bool row_sel = (fila_seleccionada_ == i);
                    if (ImGui::Selectable(s.name.c_str(), row_sel, ImGuiSelectableFlags_SpanAllColumns)) {
                        fila_seleccionada_ = i;
                    }
                    
                    ImGui::TableNextColumn();
                    ImGui::Checkbox(("##in_s"+std::to_string(i)).c_str(), &s.assayed);
                    
                    ImGui::TableNextColumn();
                    ImGui::SetNextItemWidth(90.0f);
                    ImGui::InputDouble(("##flo_s"+std::to_string(i)).c_str(), &s.measured_flow, 0, 0, "%.2f");
                    
                    ImGui::TableNextColumn();
                    ImGui::SetNextItemWidth(70.0f);
                    ImGui::InputDouble(("##sd_s"+std::to_string(i)).c_str(), &s.sd, 0, 0, "%.2f");
                }
                ImGui::EndTable();
            }
        }
        else if (seccion_ == SeccionNavegacion::SDStream) {
            ImGui::Text(tr("Desviaciones Est. Corrientes"));
            if (ImGui::BeginTable("sd_grid", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Measured Flow", ImGuiTableColumnFlags_WidthFixed, 120.0f);
                ImGui::TableSetupColumn("SD % of Flow", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableHeadersRow();
                
                for (int i = 0; i < (int)corrientes_.size(); i++) {
                    auto& s = corrientes_[i];
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn();
                    ImGui::Text(tr("%s"), s.name.c_str());
                    ImGui::TableNextColumn();
                    ImGui::Text(tr("%.2f"), s.measured_flow);
                    ImGui::TableNextColumn();
                    double pct = (s.measured_flow > 0.0) ? (s.sd / s.measured_flow * 100.0) : 5.0;
                    ImGui::Text(tr("%.1f %%"), pct);
                }
                ImGui::EndTable();
            }
        }
        else if (seccion_ == SeccionNavegacion::Balancing) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f,0.8f,0.6f,1.0f), "Mass Balance Solver Settings");
            ImGui::Separator();
            ImGui::Spacing();
            
            ImGui::Text(tr("Problem Type Option:"));
            ImGui::RadioButton("Solids Only (Standard metallurgical balancing)", &tipo_problema_, 0);
            ImGui::RadioButton("Assays Only (No direct flow measurements)", &tipo_problema_, 1);
            ImGui::RadioButton("Solids & Water Independently (Solve pulp flow rates)", &tipo_problema_, 2);
            
            ImGui::Spacing();
            ImGui::Separator();
            ImGui::Spacing();
            
            if (ImGui::Button(tr("Solve Mass Balance (Run Reconciliation)"), ImVec2(-1, 35))) {
                ejecutar_balance();
            }
            
            ImGui::Spacing();
            ImGui::Text(tr("Solver Log:"));
            ImGui::BeginChild("solver_log", ImVec2(0, 100), true);
            if (calculado_) {
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f, 0.9f, 0.4f, 1.0f), "Reconciliation finalized successfully!");
                ImGui::Text(tr("Degrees of freedom: %zu"), corrientes_.size() - 1);
                ImGui::Text(tr("Convergence chi-square check finalized."));
            } else {
                ImGui::Text(tr("Ready. Press solver button above to calculate."));
            }
            ImGui::EndChild();
        }
        else if (seccion_ == SeccionNavegacion::Reporting) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.3f,0.8f,0.9f,1.0f), "Data Reconciliation Report");
            ImGui::Separator();
            
            if (calculado_) {
                // ── Stats summary ──
                float chi2 = (float)resultado_.chi_cuadrado;
                float q    = (float)resultado_.q_ajuste;
                bool good  = (q > 0.05f);
                ImGui::TextColored(good ? cma::interfaz::AdaptiveColor(0.3f,0.9f,0.4f,1.0f) : cma::interfaz::AdaptiveColor(0.9f,0.5f,0.3f,1.0f),
                    "X2 = %.4f   Q = %.4f   %s", chi2, q, good ? "[OK] Acceptable fit" : "[!] Poor fit");
                
                ImGui::SameLine();
                if (ImGui::Button(tr("Exportar CSV##report"))) {
                    FILE* zen = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' 2>/dev/null");
                    if (zen) {
                        char path[512]={0};
                        if (fgets(path, sizeof(path), zen)) {
                            std::string p(path);
                            if (!p.empty() && p.back()=='\n') p.pop_back();
                            if (!p.empty()) {
                                if (p.find(".csv")==std::string::npos) p+=".csv";
                                FILE* fp = fopen(p.c_str(),"w");
                                if (fp) {
                                    fprintf(fp,"Chi2,%.6f\nQ,%.6f\n\n",resultado_.chi_cuadrado,resultado_.q_ajuste);
                                    fprintf(fp,"Stream,Measured,Reconciled,Residual,RecSD\n");
                                    for (auto& s : corrientes_) {
                                        fprintf(fp,"\"%s\",%.4f,%.4f,%.4f,%.4f\n",
                                            s.name.c_str(),s.measured_flow,s.reconciled_flow,
                                            s.measured_flow-s.reconciled_flow,s.reconciled_error);
                                    }
                                    fclose(fp);
                                }
                            }
                        }
                        fclose(zen);
                    }
                }
                
                ImGui::Spacing();

                if (ImGui::BeginTabBar("report_tabs")) {
                    if (ImGui::BeginTabItem(tr("Tabla"))) {
                        if (ImGui::BeginTable("report_grid", 5,
                                ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                                ImVec2(0, 180))) {
                            ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthStretch);
                            ImGui::TableSetupColumn("Measured", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                            ImGui::TableSetupColumn("Reconciled", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                            ImGui::TableSetupColumn("Residual", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                            ImGui::TableSetupColumn("SD", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                            ImGui::TableHeadersRow();
                            
                            for (auto& s : corrientes_) {
                                double res = s.measured_flow - s.reconciled_flow;
                                ImGui::TableNextRow();
                                ImGui::TableNextColumn(); ImGui::Text(tr("%s"), s.name.c_str());
                                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), s.measured_flow);
                                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), s.reconciled_flow);
                                ImGui::TableNextColumn();
                                if (fabs(res) < s.sd)
                                    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.3f,0.9f,0.4f,1.0f), "%.2f", res);
                                else
                                    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f,0.4f,0.3f,1.0f), "%.2f", res);
                                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), s.reconciled_error);
                            }
                            ImGui::EndTable();
                        }
                        ImGui::EndTabItem();
                    }
                    if (ImGui::BeginTabItem(tr("Residuals Chart"))) {
                        static std::vector<double> res_vals, xs_r;
                        static std::vector<const char*> lbs_r;
                        int nc = (int)corrientes_.size();
                        res_vals.resize(nc); xs_r.resize(nc); lbs_r.resize(nc);
                        for (int i=0;i<nc;i++){
                            res_vals[i] = corrientes_[i].measured_flow - corrientes_[i].reconciled_flow;
                            xs_r[i] = (double)i;
                            lbs_r[i] = corrientes_[i].name.c_str();
                        }
                        if (ImPlot::BeginPlot("Residuals (Measured - Reconciled)", ImVec2(-1, 200))) {
                            ImPlot::SetupAxes("Corriente", "Residual (kg/h)");
                            if (nc > 0)
                                ImPlot::SetupAxisTicks(ImAxis_X1, xs_r.data(), nc, lbs_r.data());
                            ImPlot::SetNextFillStyle(ImVec4(0.4f,0.7f,1.0f,0.85f));
                            ImPlot::PlotBars("Residual", res_vals.data(), nc, 0.6);
                            ImPlot::EndPlot();
                        }
                        ImGui::EndTabItem();
                    }
                    ImGui::EndTabBar();
                }
            } else {
                ImGui::TextDisabled(tr("Run mass balance calculations to generate reconciliation report."));
            }
        }

        else {
            ImGui::TextDisabled(tr("This section is configured automatically by flowsheet parameters."));
        }
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();

    // ════════════════════════════════════════════════════════════════════════
    // Panel Derecho: Properties Inspector
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("prop_pane");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.5f, 1.0f), "Properties");
        ImGui::Separator();
        
        if (seccion_ == SeccionNavegacion::Units && fila_seleccionada_ >= 0 && fila_seleccionada_ < (int)unidades_.size()) {
            auto& u = unidades_[fila_seleccionada_];
            ImGui::Text(tr("Unit settings:"));
            ImGui::TextDisabled(tr("Name: %s"), u.name.c_str());
            ImGui::Checkbox(tr("Selected"), &u.selected);
            ImGui::Checkbox(tr("PSD Changing"), &u.psd_changing);
        }
        else if (seccion_ == SeccionNavegacion::Streams && fila_seleccionada_ >= 0 && fila_seleccionada_ < (int)corrientes_.size()) {
            auto& s = corrientes_[fila_seleccionada_];
            ImGui::Text(tr("Stream settings:"));
            ImGui::TextDisabled(tr("Name: %s"), s.name.c_str());
            ImGui::Checkbox(tr("Selected"), &s.selected);
            ImGui::Checkbox(tr("Input Stream"), &s.assayed);
            ImGui::InputDouble("Measured", &s.measured_flow, 1.0, 10.0, "%.2f");
            ImGui::InputDouble("SD", &s.sd, 0.5, 5.0, "%.2f");
        }
        else {
            ImGui::TextDisabled(tr("Select an element in middle table grid to view properties."));
        }
    }
    ImGui::EndChild();
    
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
