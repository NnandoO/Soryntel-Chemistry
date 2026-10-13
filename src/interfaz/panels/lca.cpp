#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de LCA
// ═══════════════════════════════════════════════════════════════════════════
void PanelLCA::inicializar_flujos(diagrama_flujo::Flowsheet* flowsheet) {
    if (!flowsheet || cargado_inicial_) return;
    mapeos_.clear();

    for (auto& [fid, flujo] : flowsheet->mapa_flujos()) {
        MapeoLcaFlujo m;
        m.stream_id = fid;
        m.stream_name = flujo.nombre.empty() ? ("Stream " + std::to_string(fid)) : flujo.nombre;
        // Calcular flujo masico real: sum(ni * Mi) en lugar de flujo molar
        double flujo_masa = 0.0;
        auto& gestor_lca = base_datos::GestorDB::instancia();
        for (auto& [sp_id, n_moles] : flujo.flujo_molar) {
            auto* esp = gestor_lca.especie_por_id(sp_id);
            if (esp && esp->masa_molar > 0.0) {
                flujo_masa += n_moles * esp->masa_molar; // kmol/s * g/mol = kg/s (numéricamente)
            } else {
                flujo_masa += n_moles * 18.015; // Fallback: masa molar del agua
            }
        }
        m.flow_value = flujo_masa;
        m.unit = "kg/s";

        std::string unit_name = "System";
        bool es_entrada = true;
        
        for (auto& nodo : flowsheet->nodos()) {
            for (int64_t sid : nodo.ids_flujos_salida) {
                if (sid == fid) {
                    unit_name = nodo.nombre;
                    if (nodo.tipo == nucleo::TipoNodo::Fuente) {
                        es_entrada = true;
                    } else {
                        es_entrada = false;
                    }
                }
            }
            for (int64_t sid : nodo.ids_flujos_entrada) {
                if (sid == fid) {
                    if (nodo.tipo == nucleo::TipoNodo::Sumidero) {
                        es_entrada = false;
                    }
                }
            }
        }
        
        m.unit_name = unit_name;
        m.es_entrada = es_entrada;
        m.lca_equivalent = flujo.lca_equivalent;
        m.lca_group = flujo.lca_group;
        
        if (m.lca_equivalent == "No Mapping" || m.lca_equivalent.empty()) {
            std::string lower_name = m.stream_name;
            std::transform(lower_name.begin(), lower_name.end(), lower_name.begin(), ::tolower);
            
            if (lower_name.find("gas") != std::string::npos || lower_name.find("fuel") != std::string::npos || lower_name.find("co2") != std::string::npos) {
                m.lca_group = "Emissions to air";
                m.lca_equivalent = "CO2 (fossil) - ecoinvent";
            } else if (lower_name.find("slag") != std::string::npos || lower_name.find("relave") != std::string::npos || lower_name.find("escoria") != std::string::npos) {
                m.lca_group = "Deposited goods";
                m.lca_equivalent = "Slag landfill - GaBi";
            } else if (lower_name.find("concentrado") != std::string::npos || lower_name.find("cu") != std::string::npos || lower_name.find("mineral") != std::string::npos) {
                m.lca_group = "Resources";
                m.lca_equivalent = "Copper ore resource - GaBi";
            } else if (lower_name.find("agua") != std::string::npos || lower_name.find("water") != std::string::npos) {
                m.lca_group = "Resources";
                m.lca_equivalent = "Water intake - ecoinvent";
            } else {
                m.lca_group = "Not defined";
                m.lca_equivalent = "No Mapping";
            }
        }
        
        mapeos_.push_back(m);
    }
    cargado_inicial_ = true;
}

void PanelLCA::calcular_impacto() {
    double co2 = 0.0;
    double energia = 0.0;
    double residuos = 0.0;
    
    cma::motores::LCA lca_motor;
    co2 += lca_motor.factor_emision_electricidad(manual_electricidad_, "global");
    energia += manual_electricidad_ * 3.6;
    
    co2 += lca_motor.factor_emision_combustible(manual_combustible_, "diésel");
    energia += manual_combustible_ * 42.6;
    
    for (auto& m : mapeos_) {
        double flow_kg = m.flow_value * factor_normalizacion_;
        if (m.lca_equivalent == "No Mapping") continue;
        
        if (m.lca_group == "Emissions to air") {
            if (m.lca_equivalent.find("CO2") != std::string::npos) {
                co2 += flow_kg * 1.0;
            } else if (m.lca_equivalent.find("CH4") != std::string::npos) {
                co2 += flow_kg * 28.0;
            } else if (m.lca_equivalent.find("N2O") != std::string::npos) {
                co2 += flow_kg * 265.0;
            } else if (m.lca_equivalent.find("SO2") != std::string::npos) {
                co2 += flow_kg * 0.5;
            }
        } else if (m.lca_group == "Deposited goods") {
            residuos += flow_kg;
            co2 += flow_kg * 0.05;
        } else if (m.lca_group == "Resources") {
            if (m.lca_equivalent.find("ore") != std::string::npos) {
                energia += flow_kg * 1.2;
                co2 += flow_kg * 0.15;
            } else if (m.lca_equivalent.find("Water") != std::string::npos) {
                energia += flow_kg * 0.02;
            }
        }
    }
    
    resultado_huella_ = co2;
    resultado_energia_ = energia;
    resultado_residuos_ = residuos;
    calculado_ = true;
}

void PanelLCA::renderizar(diagrama_flujo::Flowsheet* flowsheet) {
    if (!flowsheet) {
        ImGui::Text(tr("Cargue una simulación para evaluar LCA."));
        return;
    }
    
    inicializar_flujos(flowsheet);

    // ── Barra de menú superior estilo CAD ──
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(8, 6));
    if (ImGui::Button(tr("Guardar"))) {
        for (auto& m : mapeos_) {
            auto* f = flowsheet->flujo_por_id(m.stream_id);
            if (f) {
                f->lca_equivalent = m.lca_equivalent;
                f->lca_group = m.lca_group;
            }
        }
        ImGui::OpenPopup("lca_saved");
    }
    if (ImGui::BeginPopup("lca_saved")) {
        ImGui::Text(tr("Mappings saved to flowsheet streams!"));
        if (ImGui::Button(tr("Cerrar"))) ImGui::CloseCurrentPopup();
        ImGui::EndPopup();
    }
    
    ImGui::SameLine();
    if (ImGui::Button(tr("Refrescar"))) {
        cargado_inicial_ = false;
        inicializar_flujos(flowsheet);
    }
    if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Reload streams from the flowsheet"));

    ImGui::SameLine();
    if (ImGui::Button(tr("Normalize"))) {
        if (fila_seleccionada_ >= 0 && fila_seleccionada_ < (int)mapeos_.size()) {
            double val = mapeos_[fila_seleccionada_].flow_value;
            if (val > 0.0) {
                factor_normalizacion_ = 1.0 / val;
                normalizado_ = true;
                calcular_impacto();
            }
        }
    }
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Normalizar flujos basándose en la corriente seleccionada (Functional Unit = 1.0)");
    
    ImGui::SameLine();
    if (ImGui::Button(tr("Reset Norm"))) {
        factor_normalizacion_ = 1.0;
        normalizado_ = false;
        calcular_impacto();
    }

    ImGui::SameLine();
    if (ImGui::Button(tr("To GaBi"))) {
        FILE* zenity = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.xml' --title='Exportar a GaBi Plan'");
        if (zenity) {
            char path[512] = {0};
            if (fgets(path, sizeof(path), zenity)) {
                std::string path_str(path);
                if (!path_str.empty() && path_str.back() == '\n') path_str.pop_back();
                if (path_str.find(".xml") == std::string::npos) path_str += ".xml";
                
                FILE* fp = fopen(path_str.c_str(), "w");
                if (fp) {
                    fprintf(fp, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
                    fprintf(fp, "<GaBiPlan version=\"8.7\" name=\"HSC Sim Exported Plan\">\n");
                    fprintf(fp, "  <Flows>\n");
                    for (auto& m : mapeos_) {
                        if (m.lca_equivalent != "No Mapping") {
                            fprintf(fp, "    <Flow name=\"%s\" gabiEquivalent=\"%s\" group=\"%s\" flowValue=\"%.6f\" unit=\"%s\"/>\n",
                                    m.stream_name.c_str(), m.lca_equivalent.c_str(), m.lca_group.c_str(), m.flow_value * factor_normalizacion_, m.unit.c_str());
                        }
                    }
                    fprintf(fp, "  </Flows>\n");
                    fprintf(fp, "  <ManualInputs electricity_kWh=\"%.2f\" fuel_kg=\"%.2f\" water_m3=\"%.2f\"/>\n",
                            manual_electricidad_, manual_combustible_, manual_agua_);
                    fprintf(fp, "  <Indicators carbonFootprint_kgCO2eq=\"%.2f\" primaryEnergy_MJ=\"%.2f\" solidWaste_kg=\"%.2f\"/>\n",
                            resultado_huella_, resultado_energia_, resultado_residuos_);
                    fprintf(fp, "</GaBiPlan>\n");
                    fclose(fp);
                }
            }
            fclose(zenity);
        }
    }
    
    ImGui::SameLine();
    if (ImGui::Button(tr("To Excel"))) {
        FILE* zenity = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Exportar a Excel (CSV)'");
        if (zenity) {
            char path[512] = {0};
            if (fgets(path, sizeof(path), zenity)) {
                std::string path_str(path);
                if (!path_str.empty() && path_str.back() == '\n') path_str.pop_back();
                if (path_str.find(".csv") == std::string::npos) path_str += ".csv";
                
                FILE* fp = fopen(path_str.c_str(), "w");
                if (fp) {
                    fprintf(fp, "LCA Evaluation Dataset,Normalized,Factor:;%.6f\n", factor_normalizacion_);
                    fprintf(fp, "Unit Name,Stream Name,Flow Value,Unit,Lca Equivalent,Lca Group,Direction\n");
                    for (auto& m : mapeos_) {
                        fprintf(fp, "\"%s\",\"%s\",%.6f,\"%s\",\"%s\",\"%s\",\"%s\"\n",
                                m.unit_name.c_str(), m.stream_name.c_str(), m.flow_value * factor_normalizacion_, m.unit.c_str(), m.lca_equivalent.c_str(), m.lca_group.c_str(), m.es_entrada ? "Input" : "Output");
                    }
                    fprintf(fp, "\nManual Inputs / Outputs\n");
                    fprintf(fp, "Indicator,Value,Unit\n");
                    fprintf(fp, "Electricity,%.2f,kWh\n", manual_electricidad_);
                    fprintf(fp, "Fuel,%.2f,kg\n", manual_combustible_);
                    fprintf(fp, "Water,%.2f,m3\n", manual_agua_);
                    fprintf(fp, "\nEnvironmental Impacts Summary\n");
                    fprintf(fp, "Carbon Footprint,%.2f,kg CO2-equiv\n", resultado_huella_);
                    fprintf(fp, "Solid Waste,%.2f,kg\n", resultado_residuos_);
                    fclose(fp);
                }
            }
            fclose(zenity);
        }
    }
    ImGui::PopStyleVar();
    ImGui::Separator();
    ImGui::Spacing();

    // ── Grid split ──
    ImGui::Columns(2, "lca_panel_split", true);
    
    // Ancho inicial de columnas
    static bool init_w = true;
    if (init_w) {
        ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.65f);
        init_w = false;
    }

    // ── Columna Izquierda: Tabla y Pestañas ──
    ImGui::BeginChild("left_pane_lca");
    {
        if (mapeos_.empty()) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f, 0.4f, 0.4f, 1.0f), "Warning: No streams found in active page.");
            ImGui::TextWrapped(tr("Please draw some streams in the active flowsheet page to allow mapping."));
            ImGui::Spacing();
            ImGui::Separator();
            ImGui::Spacing();
        }

        if (ImGui::BeginTabBar("lca_tabs")) {
            // Pestaña INPUTS
            if (ImGui::BeginTabItem(tr("Entradas"))) {
                if (ImGui::BeginTable("inputs_table", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 240))) {
                    ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                    ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthFixed, 120.0f);
                    ImGui::TableSetupColumn("Flujo", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                    ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthFixed, 50.0f);
                    ImGui::TableSetupColumn("Lca Equivalent", ImGuiTableColumnFlags_WidthFixed, 150.0f);
                    ImGui::TableSetupColumn("Lca Group", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                    ImGui::TableHeadersRow();
                    
                    int idx = 0;
                    for (auto& m : mapeos_) {
                        if (m.es_entrada) {
                            ImGui::TableNextRow();
                            ImGui::TableNextColumn();
                            
                            bool sel = (fila_seleccionada_ == idx);
                            if (ImGui::Selectable(m.unit_name.c_str(), sel, ImGuiSelectableFlags_SpanAllColumns)) {
                                fila_seleccionada_ = idx;
                            }
                            
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.stream_name.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.4f"), m.flow_value * factor_normalizacion_);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.unit.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.lca_equivalent.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.lca_group.c_str());
                        }
                        idx++;
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }

            // Pestaña OUTPUTS
            if (ImGui::BeginTabItem(tr("Salidas"))) {
                if (ImGui::BeginTable("outputs_table", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 240))) {
                    ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                    ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthFixed, 120.0f);
                    ImGui::TableSetupColumn("Flujo", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                    ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthFixed, 50.0f);
                    ImGui::TableSetupColumn("Lca Equivalent", ImGuiTableColumnFlags_WidthFixed, 150.0f);
                    ImGui::TableSetupColumn("Lca Group", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                    ImGui::TableHeadersRow();
                    
                    int idx = 0;
                    for (auto& m : mapeos_) {
                        if (!m.es_entrada) {
                            ImGui::TableNextRow();
                            ImGui::TableNextColumn();
                            
                            bool sel = (fila_seleccionada_ == idx);
                            if (ImGui::Selectable(m.unit_name.c_str(), sel, ImGuiSelectableFlags_SpanAllColumns)) {
                                fila_seleccionada_ = idx;
                            }
                            
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.stream_name.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.4f"), m.flow_value * factor_normalizacion_);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.unit.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.lca_equivalent.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.lca_group.c_str());
                        }
                        idx++;
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }

            // Pestaña MANUAL INPUTS
            if (ImGui::BeginTabItem(tr("Manual Inputs"))) {
                ImGui::Spacing();
                ImGui::Text(tr("Consumos auxiliares directos:"));
                ImGui::Separator();
                if (ImGui::InputDouble("Electricidad Directa (kWh)", &manual_electricidad_)) calcular_impacto();
                if (ImGui::InputDouble("Combustible Diésel (kg)", &manual_combustible_)) calcular_impacto();
                if (ImGui::InputDouble("Consumo Agua de Proceso (m³)", &manual_agua_)) calcular_impacto();
                ImGui::EndTabItem();
            }

            // Pestaña INDICATORS (Gráficos)
            if (ImGui::BeginTabItem(tr("Indicators"))) {
                calcular_impacto();
                
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.2f,0.8f,0.6f,1), "Carbon Footprint: %.2f kg CO₂-equiv", resultado_huella_);
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f,0.6f,0.2f,1), "Primary Energy Demand: %.1f MJ", resultado_energia_);
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f,0.4f,0.8f,1), "Solid Waste: %.1f kg", resultado_residuos_);
                
                double vals[3] = {resultado_huella_, resultado_energia_ * 0.1, resultado_residuos_};
                const char* labels[3] = {"CO₂eq (kg)", "Energía (MJ×0.1)", "Residuos (kg)"};
                if (ImPlot::BeginPlot("LCA Indicators", ImVec2(-1, 150), ImPlotFlags_NoTitle)) {
                    ImPlot::SetupAxes("Impacto", "Escala");
                    ImPlot::SetupAxesLimits(-0.5, 2.5, 0, std::max({vals[0], vals[1], vals[2]}) * 1.2);
                    for (int i = 0; i < 3; i++) {
                        ImPlot::PlotBars(labels[i], &vals[i], 1, 0.4, i - 0.2);
                    }
                    ImPlot::EndPlot();
                }
                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }
        
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();
        
        // Botón Apply Mapping
        if (ImGui::Button(tr("Aplicar Mapeo"), ImVec2(-1, 26))) {
            if (fila_seleccionada_ >= 0 && fila_seleccionada_ < (int)mapeos_.size() && !nodo_arbol_seleccionado_.empty()) {
                auto& m = mapeos_[fila_seleccionada_];
                m.lca_equivalent = nodo_arbol_seleccionado_;
                m.lca_group = cat_path_;
                
                auto* f = flowsheet->flujo_por_id(m.stream_id);
                if (f) {
                    f->lca_equivalent = m.lca_equivalent;
                    f->lca_group = m.lca_group;
                }
                calcular_impacto();
            }
        }
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();

    // ── Columna Derecha: Árbol de categorías GaBi ──
    ImGui::BeginChild("right_pane_lca");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.6f,0.7f,0.9f,1), "Flows Database (GaBi)");
        ImGui::Separator();
        
        auto render_leaf = [&](const char* name, const char* path, const char* unit, const char* ref_q, double conv) {
            bool is_selected = (nodo_arbol_seleccionado_ == name);
            if (ImGui::Selectable(name, is_selected)) {
                nodo_arbol_seleccionado_ = name;
                cat_nombre_ = name;
                cat_path_ = path;
                cat_unit_ = unit;
                cat_ref_q_ = ref_q;
                cat_conv_f_ = conv;
            }
        };

        if (ImGui::TreeNode(tr("Bienes depositados"))) {
            render_leaf("Slag landfill - GaBi", "Deposited goods", "kg", "1 kg", 1.0);
            render_leaf("Fly ash disposal - ecoinvent", "Deposited goods", "kg", "1 kg", 1.0);
            render_leaf("Tailings deposition - GaBi", "Deposited goods", "kg", "1 kg", 1.0);
            ImGui::TreePop();
        }

        if (ImGui::TreeNode(tr("Emisiones al aire"))) {
            if (ImGui::TreeNode(tr("Greenhouse gases"))) {
                render_leaf("CO2 (fossil) - ecoinvent", "Emissions to air", "kg", "1 kg", 1.0);
                render_leaf("CH4 (methane) - ecoinvent", "Emissions to air", "kg", "1 kg", 28.0);
                render_leaf("N2O (nitrous oxide) - ecoinvent", "Emissions to air", "kg", "1 kg", 265.0);
                ImGui::TreePop();
            }
            if (ImGui::TreeNode(tr("Acidification gases"))) {
                render_leaf("SO2 (sulfur dioxide) - GaBi", "Emissions to air", "kg", "1 kg", 1.0);
                render_leaf("NOx (nitrogen oxides) - GaBi", "Emissions to air", "kg", "1 kg", 1.0);
                ImGui::TreePop();
            }
            ImGui::TreePop();
        }

        if (ImGui::TreeNode(tr("Emisiones a agua dulce"))) {
            render_leaf("Heavy metals to water - GaBi", "Emissions to fresh water", "kg", "1 kg", 1.0);
            render_leaf("Nitrates / Phosphates - ecoinvent", "Emissions to fresh water", "kg", "1 kg", 1.0);
            ImGui::TreePop();
        }

        if (ImGui::TreeNode(tr("Resources"))) {
            render_leaf("Copper ore resource - GaBi", "Resources", "kg", "1 kg", 1.0);
            render_leaf("Iron ore resource - GaBi", "Resources", "kg", "1 kg", 1.0);
            render_leaf("Water intake - ecoinvent", "Resources", "kg", "1 kg", 1.0);
            render_leaf("Electricity (cradle-to-gate) - GaBi", "Resources", "kWh", "1 kWh", 1.0);
            ImGui::TreePop();
        }

        ImGui::Spacing();
        ImGui::Separator();
        
        // Detalles de categoría seleccionada
        ImGui::Text(tr("Category Details:"));
        ImGui::TextDisabled(tr("Name: %s"), cat_nombre_.c_str());
        ImGui::TextDisabled(tr("Path: %s"), cat_path_.c_str());
        ImGui::TextDisabled(tr("Unit: %s"), cat_unit_.c_str());
        ImGui::TextDisabled(tr("Ref Quantity: %s"), cat_ref_q_.c_str());
        ImGui::TextDisabled(tr("Conv Factor: %.1f"), cat_conv_f_);
    }
    ImGui::EndChild();
    
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Datos / DataFitter
// ═══════════════════════════════════════════════════════════════════════════
void PanelDatosANN::renderizar() {
    ImGui::Text(tr("Ajuste de Curvas (DataFitter)"));
    ImGui::Separator();

    ImGui::InputText(tr("X (comma-sep)"), entrada_1_, sizeof(entrada_1_));
    ImGui::InputText(tr("Y (comma-sep)"), entrada_2_, sizeof(entrada_2_));

    static int grado_poly = 2;
    ImGui::InputInt("Grado polinomio", &grado_poly);
    if (grado_poly < 1) grado_poly = 1;
    if (grado_poly > 10) grado_poly = 10;

    auto parsear_csv = [](const std::string& s) {
        std::vector<double> v;
        std::stringstream ss(s);
        std::string token;
        while (std::getline(ss, token, ',')) {
            if (token.empty()) continue;
            try { v.push_back(std::stod(token)); }
            catch (...) { v.push_back(0.0); }
        }
        return v;
    };

    if (ImGui::Button(tr("Ajuste Lineal"), ImVec2(-1, 25))) {
        std::vector<double> x = parsear_csv(entrada_1_), y = parsear_csv(entrada_2_);
        if (x.size() == y.size() && x.size() >= 2) {
            cma::application::modules::DataFitter fitter;
            res_fit_ = fitter.lineal(x, y);
            entrenado_ = res_fit_.ok;
            datos_x_ = x;
            datos_y_ = y;
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Ajuste Polinomial"), ImVec2(-1, 25))) {
        std::vector<double> x = parsear_csv(entrada_1_), y = parsear_csv(entrada_2_);
        if (x.size() == y.size() && grado_poly >= 0 && x.size() >= static_cast<size_t>(grado_poly + 1)) {
            cma::application::modules::DataFitter fitter;
            res_fit_ = fitter.polinomial(x, y, grado_poly);
            entrenado_ = res_fit_.ok;
            datos_x_ = x;
            datos_y_ = y;
        }
    }

    if (entrenado_) {
        ImGui::Separator();
        ImGui::Text(tr("R² = %.6f"), res_fit_.r_cuadrado);
        ImGui::Text(tr("Coeficientes:"));
        for (size_t i = 0; i < res_fit_.coeficientes.size(); i++)
            ImGui::Text(tr("  a%d = %.6f"), (int)i, res_fit_.coeficientes[i]);

        if (datos_x_.size() > 1 && !res_fit_.coeficientes.empty()) {
            if (ImPlot::BeginPlot("Ajuste", ImVec2(-1, 280), ImPlotFlags_NoTitle)) {
                ImPlot::SetupAxes("X", "Y");
                double xmin = *std::min_element(datos_x_.begin(), datos_x_.end());
                double xmax = *std::max_element(datos_x_.begin(), datos_x_.end());
                double ymin = *std::min_element(datos_y_.begin(), datos_y_.end());
                double ymax = *std::max_element(datos_y_.begin(), datos_y_.end());
                double margin = (ymax - ymin) * 0.2;
                ImPlot::SetupAxesLimits(xmin - margin*0.5, xmax + margin*0.5, ymin - margin, ymax + margin);
                ImPlot::PlotScatter("Datos", datos_x_.data(), datos_y_.data(), (int)datos_x_.size());

                int n_curve = 100;
                std::vector<double> fx(n_curve), fy(n_curve);
                for (int i = 0; i < n_curve; i++) {
                    fx[i] = xmin + (xmax - xmin) * i / (n_curve - 1);
                    fy[i] = 0;
                    double xp = 1.0;
                    for (auto& c : res_fit_.coeficientes) {
                        fy[i] += c * xp;
                        xp *= fx[i];
                    }
                }
                ImPlot::PlotLine("Ajuste", fx.data(), fy.data(), n_curve);
                ImPlot::EndPlot();
            }
        }

        if (!res_fit_.mensaje.empty())
            ImGui::TextWrapped(tr("%s"), res_fit_.mensaje.c_str());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Gráficos 3D
// ═══════════════════════════════════════════════════════════════════════════

void PanelGrafico3D::generar_datos_ejemplo() {
    data_x_.clear(); data_y_.clear(); data_z_.clear();
    // Superficie: z = sin(r)/r (sinc)
    int n = 30;
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            float x = -3.0f + 6.0f * i / (n-1);
            float y = -3.0f + 6.0f * j / (n-1);
            float r = std::sqrt(x*x + y*y);
            float z = (r > 0.01f) ? std::sin(r)/r : 1.0f;
            data_x_.push_back(x);
            data_y_.push_back(y);
            data_z_.push_back(z);
        }
    }
    datos_generados_ = true;
}

ImVec3 PanelGrafico3D::proyectar(float x, float y, float z, const ImVec2& centro, float escala) const {
    float sx = -std::sin(ang_y_);
    float sy = std::cos(ang_y_);
    float cz = std::cos(ang_x_);
    float sz = std::sin(ang_x_);

    float rx = x * cz - z * sz;
    float ry = y;
    float rz = x * sz + z * cz;

    float px = rx * sy + ry * sx;
    float py = -rx * sx + ry * sy;
    float pz = rz + distancia_;

    if (pz < 0.1f) pz = 0.1f;
    float f = escala * 2.0f / pz;
    return ImVec3(centro.x + px * f, centro.y - py * f, pz);
}

void PanelGrafico3D::renderizar() {
    if (!datos_generados_) generar_datos_ejemplo();

    ImGui::Text(tr("Gráfico 3D"));
    ImGui::Separator();

    ImGui::SliderFloat("Rotación X", &ang_x_, -3.14f, 3.14f, "%.2f");
    ImGui::SliderFloat("Rotación Y", &ang_y_, -3.14f, 3.14f, "%.2f");
    ImGui::SliderFloat("Distancia", &distancia_, 1.0f, 20.0f, "%.1f");
    ImGui::Checkbox(tr("Auto-rotar"), &auto_rotar_);
    ImGui::SameLine();
    ImGui::Combo("Tipo", &tipo_grafico_, "Superficie\0Líneas\0Puntos\0Contorno\0");

    if (auto_rotar_) {
        ang_y_ += ImGui::GetIO().DeltaTime * 0.5f;
    }

    ImVec2 area = ImGui::GetContentRegionAvail();
    float lado = std::min(area.x, area.y);
    ImVec2 centro(area.x * 0.5f, 60.0f + lado * 0.5f);
    float escala = lado * 0.25f;

    ImDrawList* draw = ImGui::GetWindowDrawList();

    // Fondo
    draw->AddRectFilled(ImVec2(0, 50), ImVec2(area.x, 50 + lado),
                        IM_COL32(15,15,22,255), 4);

    // Ejes
    ImVec3 ox = proyectar(4,0,0,centro,escala);
    ImVec3 oy = proyectar(0,4,0,centro,escala);
    ImVec3 oz = proyectar(0,0,4,centro,escala);
    ImVec3 oo = proyectar(0,0,0,centro,escala);

    draw->AddLine(ImVec2(oo.x,oo.y), ImVec2(ox.x,ox.y), IM_COL32(255,80,80,200), 2);
    draw->AddLine(ImVec2(oo.x,oo.y), ImVec2(oy.x,oy.y), IM_COL32(80,255,80,200), 2);
    draw->AddLine(ImVec2(oo.x,oo.y), ImVec2(oz.x,oz.y), IM_COL32(80,80,255,200), 2);

    if (tipo_grafico_ == 0 || tipo_grafico_ == 3) {
        // Superficie / Contorno
        int n = (int)std::sqrt((float)data_x_.size());
        if (n > 1) {
            for (int i = 0; i < n-1; i++) {
                for (int j = 0; j < n-1; j++) {
                    int idx = i * n + j;
                    ImVec3 p1 = proyectar(data_x_[idx], data_y_[idx], data_z_[idx], centro, escala);
                    ImVec3 p2 = proyectar(data_x_[idx+1], data_y_[idx+1], data_z_[idx+1], centro, escala);
                    ImVec3 p3 = proyectar(data_x_[idx+n], data_y_[idx+n], data_z_[idx+n], centro, escala);
                    ImVec3 p4 = proyectar(data_x_[idx+n+1], data_y_[idx+n+1], data_z_[idx+n+1], centro, escala);

                    float val = (data_z_[idx] + data_z_[idx+n] + data_z_[idx+n+1] + data_z_[idx]) * 0.25f;
                    int r = (int)(128 + 127 * std::max(-1.0f, std::min(1.0f, val)));
                    ImU32 col = IM_COL32(r, (int)(255-r*0.5f), (int)(255-r), 180);

                    if (tipo_grafico_ == 0) {
                        draw->AddTriangleFilled(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), ImVec2(p3.x,p3.y), col);
                        draw->AddTriangleFilled(ImVec2(p2.x,p2.y), ImVec2(p4.x,p4.y), ImVec2(p3.x,p3.y), col);
                    }
                    draw->AddTriangle(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), ImVec2(p3.x,p3.y), IM_COL32(60,60,80,80), 1);
                    draw->AddTriangle(ImVec2(p2.x,p2.y), ImVec2(p4.x,p4.y), ImVec2(p3.x,p3.y), IM_COL32(60,60,80,80), 1);
                }
            }
        }
    } else if (tipo_grafico_ == 1) {
        // Líneas (isolinhas)
        int n = (int)std::sqrt((float)data_x_.size());
        for (int i = 0; i < n; i++) {
            for (int j = 0; j < n-1; j++) {
                int idx = i * n + j;
                ImVec3 p1 = proyectar(data_x_[idx], data_y_[idx], data_z_[idx], centro, escala);
                ImVec3 p2 = proyectar(data_x_[idx+1], data_y_[idx+1], data_z_[idx+1], centro, escala);
                draw->AddLine(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), IM_COL32(100,200,255,120), 1);
            }
        }
        for (int j = 0; j < n; j++) {
            for (int i = 0; i < n-1; i++) {
                int idx = i * n + j;
                ImVec3 p1 = proyectar(data_x_[idx], data_y_[idx], data_z_[idx], centro, escala);
                ImVec3 p2 = proyectar(data_x_[idx+n], data_y_[idx+n], data_z_[idx+n], centro, escala);
                draw->AddLine(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), IM_COL32(100,200,255,120), 1);
            }
        }
    } else {
        // Puntos
        for (size_t i = 0; i < data_x_.size(); i++) {
            ImVec3 p = proyectar(data_x_[i], data_y_[i], data_z_[i], centro, escala);
            float val = data_z_[i];
            int r = (int)(128 + 127 * std::max(-1.0f, std::min(1.0f, val)));
            ImU32 col = IM_COL32(r, 200-r, 255-r, 200);
            draw->AddCircleFilled(ImVec2(p.x,p.y), 3, col);
        }
    }

    // Leyenda de ejes
    draw->AddText(ImVec2(ox.x+4,ox.y-6), IM_COL32(255,120,120,200), "X");
    draw->AddText(ImVec2(oy.x+4,oy.y-6), IM_COL32(120,255,120,200), "Y");
    draw->AddText(ImVec2(oz.x+4,oz.y-6), IM_COL32(120,120,255,200), "Z");
}

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
