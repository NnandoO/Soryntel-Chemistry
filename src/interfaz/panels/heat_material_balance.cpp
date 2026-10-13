#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de Balance Hoja de Cálculo
// ═══════════════════════════════════════════════════════════════════════════
PanelBalanceHoja::PanelBalanceHoja() {
    inicializar_valores_defecto();
}

void PanelBalanceHoja::inicializar_valores_defecto() {
    entradas_.clear();
    salidas_.clear();
    
    // Input Stream: CH4 Feed
    StreamGroup in1;
    strncpy(in1.nombre, "CH4 Input", sizeof(in1.nombre)-1);
    
    SpeciesRow ch4;
    strncpy(ch4.formula, "CH4(g)", sizeof(ch4.formula)-1);
    ch4.temper = 25.0;
    ch4.pressure = 1.0;
    ch4.amount_kmol = 3.300;
    in1.filas.push_back(ch4);
    entradas_.push_back(in1);

    // Input Stream: Combustion Air
    StreamGroup in2;
    strncpy(in2.nombre, "Combustion Air", sizeof(in2.nombre)-1);
    
    SpeciesRow o2;
    strncpy(o2.formula, "O2(g)", sizeof(o2.formula)-1);
    o2.temper = 25.0;
    o2.pressure = 1.0;
    o2.amount_kmol = 7.350;
    in2.filas.push_back(o2);
    
    SpeciesRow n2;
    strncpy(n2.formula, "N2(g)", sizeof(n2.formula)-1);
    n2.temper = 25.0;
    n2.pressure = 1.0;
    n2.amount_kmol = 27.650;
    in2.filas.push_back(n2);
    entradas_.push_back(in2);

    // Output Stream: Offgas
    StreamGroup out1;
    strncpy(out1.nombre, "Offgas", sizeof(out1.nombre)-1);
    
    SpeciesRow co2;
    strncpy(co2.formula, "CO2(g)", sizeof(co2.formula)-1);
    co2.temper = 350.0;
    co2.pressure = 1.0;
    co2.amount_kmol = 3.300;
    out1.filas.push_back(co2);
    
    SpeciesRow h2o;
    strncpy(h2o.formula, "H2O(g)", sizeof(h2o.formula)-1);
    h2o.temper = 350.0;
    h2o.pressure = 1.0;
    h2o.amount_kmol = 5.900;
    out1.filas.push_back(h2o);
    
    SpeciesRow n2_out;
    strncpy(n2_out.formula, "N2(g)", sizeof(n2_out.formula)-1);
    n2_out.temper = 350.0;
    n2_out.pressure = 1.0;
    n2_out.amount_kmol = 26.333;
    out1.filas.push_back(n2_out);

    SpeciesRow o2_out;
    strncpy(o2_out.formula, "O2(g)", sizeof(o2_out.formula)-1);
    o2_out.temper = 350.0;
    o2_out.pressure = 1.0;
    o2_out.amount_kmol = 0.750;
    out1.filas.push_back(o2_out);

    out1.heat_loss = 597.36;
    salidas_.push_back(out1);
    
    reciclar_calculos();
    calcular_balance_elemental();
}

void PanelBalanceHoja::reciclar_calculos() {
    double energy_factor = 1.0;
    if (energy_unit_ == 0) energy_factor = 1.0 / 4184.0; // kJ -> Mcal
    else if (energy_unit_ == 1) energy_factor = 1.0 / 1000.0; // kJ -> MJ
    else if (energy_unit_ == 2) energy_factor = 1.0 / 3600.0; // kJ -> kWh

    auto calc_rows = [&](std::vector<StreamGroup>& groups) {
        for (auto& g : groups) {
            for (auto& row : g.filas) {
                double T_K = (temp_unit_ == 0) ? (row.temper + 273.15) : row.temper;
                const auto* esp = buscar_especie_por_nombre_o_formula(row.formula);
                if (esp) {
                    row.amount_kg = row.amount_kmol * esp->masa_molar;
                    row.amount_nm3 = row.amount_kmol * 22.414;
                    double H_abs = esp->entalpia_abs(T_K); // kJ/mol
                    row.heat_content = H_abs * 1000.0 * energy_factor; // unit/kmol
                    row.total_h = row.amount_kmol * row.heat_content;
                } else {
                    row.amount_kg = 0.0;
                    row.amount_nm3 = 0.0;
                    row.heat_content = 0.0;
                    row.total_h = 0.0;
                }
            }
        }
    };
    calc_rows(entradas_);
    calc_rows(salidas_);
}

double PanelBalanceHoja::calcular_entalpia_corrientes(const std::vector<StreamGroup>& grupos, double t_out_forced) {
    double total_h = 0.0;
    
    double energy_factor = 1.0;
    if (energy_unit_ == 0) energy_factor = 1.0 / 4184.0;
    else if (energy_unit_ == 1) energy_factor = 1.0 / 1000.0;
    else if (energy_unit_ == 2) energy_factor = 1.0 / 3600.0;
    
    for (auto& g : grupos) {
        for (auto& row : g.filas) {
            double temp_c = (t_out_forced != -9999.0) ? t_out_forced : row.temper;
            double T_K = (temp_unit_ == 0) ? (temp_c + 273.15) : temp_c;
            const auto* esp = buscar_especie_por_nombre_o_formula(row.formula);
            if (esp) {
                double H_abs = esp->entalpia_abs(T_K);
                total_h += row.amount_kmol * H_abs * 1000.0 * energy_factor;
            }
        }
        total_h += g.heat_loss;
    }
    return total_h;
}

void PanelBalanceHoja::Temp_Balance() {
    double H_in = calcular_entalpia_corrientes(entradas_);
    
    auto f = [&](double temp) {
        return calcular_entalpia_corrientes(salidas_, temp) - H_in;
    };
    
    double t0 = (temp_unit_ == 0) ? 25.0 : 298.15;
    double t1 = (temp_unit_ == 0) ? 1500.0 : 1773.15;
    
    double f0 = f(t0);
    double f1 = f(t1);
    
    for (int iter = 0; iter < 50; ++iter) {
        if (std::abs(f1 - f0) < 1e-8) break;
        double t_next = t1 - f1 * (t1 - t0) / (f1 - f0);
        
        // Clamp to physical ranges to keep solver from diverging
        double t_min = (temp_unit_ == 0) ? -273.0 : 0.15;
        double t_max = (temp_unit_ == 0) ? 6000.0 : 6273.15;
        t_next = std::clamp(t_next, t_min, t_max);
        
        t0 = t1;
        f0 = f1;
        t1 = t_next;
        f1 = f(t1);
        if (std::abs(f1) < 1e-4) {
            break;
        }
    }
    
    for (auto& g : salidas_) {
        for (auto& r : g.filas) {
            r.temper = t1;
        }
    }
    
    reciclar_calculos();
}

void PanelBalanceHoja::calcular_balance_elemental() {
    elem_balance_.clear();
    std::map<std::string, double> in_elems;
    std::map<std::string, double> out_elems;
    
    for (auto& g : entradas_) {
        for (auto& r : g.filas) {
            std::string err;
            auto el_map = parse_formula_elements(r.formula, err);
            for (auto& [el, cnt] : el_map) {
                in_elems[el] += cnt * r.amount_kmol;
            }
        }
    }
    for (auto& g : salidas_) {
        for (auto& r : g.filas) {
            std::string err;
            auto el_map = parse_formula_elements(r.formula, err);
            for (auto& [el, cnt] : el_map) {
                out_elems[el] += cnt * r.amount_kmol;
            }
        }
    }
    
    std::set<std::string> all_elements;
    for (auto& [el, v] : in_elems) all_elements.insert(el);
    for (auto& [el, v] : out_elems) all_elements.insert(el);
    
    for (auto& el : all_elements) {
        ElementBal b;
        b.element = el;
        b.in_val = in_elems[el];
        b.out_val = out_elems[el];
        b.balance = b.in_val - b.out_val;
        elem_balance_.push_back(b);
    }
}

void PanelBalanceHoja::renderizar(diagrama_flujo::Flowsheet* flowsheet) {
    std::vector<StreamGroup>& active_groups = (tab_actual_ == 2) ? salidas_ : entradas_;
    // ── Toolbar ──
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(6, 4));
    if (ImGui::Button(tr("Open TXT"))) {
        FILE* zen = adapters::dialog_pipe("zenity --file-selection --file-filter='*.txt' --title='Open HMB Data File' 2>/dev/null");
        if (zen) {
            char buf[512] = {0};
            if (fgets(buf, sizeof(buf), zen)) {
                std::string path(buf);
                if (!path.empty() && path.back() == '\n') path.pop_back();
                FILE* fp = fopen(path.c_str(), "r");
                if (fp) {
                    entradas_.clear();
                    salidas_.clear();
                    bool parse_outputs = false;
                    char line[256];
                    while (fgets(line, sizeof(line), fp)) {
                        std::string l = trim_str(line);
                        if (l.empty() || l[0] == '#') {
                            if (l.find("Outputs") != std::string::npos) parse_outputs = true;
                            continue;
                        }
                        auto parts = split_str(l, ",");
                        if (parts[0] == "STREAM" && parts.size() >= 3) {
                            StreamGroup g;
                            strncpy(g.nombre, parts[1].c_str(), sizeof(g.nombre)-1);
                            g.nombre[sizeof(g.nombre)-1] = '\0';
                            try { g.heat_loss = std::stod(parts[2]); } catch (...) { g.heat_loss = 0.0; }
                            if (parse_outputs) salidas_.push_back(g);
                            else entradas_.push_back(g);
                        } else if (parts[0] == "ROW" && parts.size() >= 5) {
                            SpeciesRow r;
                            strncpy(r.formula, parts[1].c_str(), sizeof(r.formula)-1);
                            r.formula[sizeof(r.formula)-1] = '\0';
                            try {
                                r.temper = std::stod(parts[2]);
                                r.pressure = std::stod(parts[3]);
                                r.amount_kmol = std::stod(parts[4]);
                            } catch (...) { continue; }
                            if (parse_outputs && !salidas_.empty()) salidas_.back().filas.push_back(r);
                            else if (!parse_outputs && !entradas_.empty()) entradas_.back().filas.push_back(r);
                        }
                    }
                    fclose(fp);
                    reciclar_calculos();
                    calcular_balance_elemental();
                }
            }
            fclose(zen);
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Save TXT"))) {
        FILE* zen = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.txt' --title='Save HMB Data File' 2>/dev/null");
        if (zen) {
            char buf[512] = {0};
            if (fgets(buf, sizeof(buf), zen)) {
                std::string path(buf);
                if (!path.empty() && path.back() == '\n') path.pop_back();
                if (path.find(".txt") == std::string::npos) path += ".txt";
                FILE* fp = fopen(path.c_str(), "w");
                if (fp) {
                    fprintf(fp, "# HMB Inputs\n");
                    for (auto& g : entradas_) {
                        fprintf(fp, "STREAM,%s,%f\n", g.nombre, g.heat_loss);
                        for (auto& r : g.filas) {
                            fprintf(fp, "ROW,%s,%f,%f,%f\n", r.formula, r.temper, r.pressure, r.amount_kmol);
                        }
                    }
                    fprintf(fp, "# HMB Outputs\n");
                    for (auto& g : salidas_) {
                        fprintf(fp, "STREAM,%s,%f\n", g.nombre, g.heat_loss);
                        for (auto& r : g.filas) {
                            fprintf(fp, "ROW,%s,%f,%f,%f\n", r.formula, r.temper, r.pressure, r.amount_kmol);
                        }
                    }
                    fclose(fp);
                }
            }
            fclose(zen);
        }
    }
    ImGui::SameLine();
    if (flowsheet && ImGui::Button(tr("Load from Flowsheet"))) {
        entradas_.clear();
        salidas_.clear();
        
        StreamGroup inputs_group;
        strncpy(inputs_group.nombre, "Flowsheet Inputs", sizeof(inputs_group.nombre)-1);
        
        StreamGroup outputs_group;
        strncpy(outputs_group.nombre, "Flowsheet Outputs", sizeof(outputs_group.nombre)-1);
        
        for (auto& [fid, f] : flowsheet->mapa_flujos()) {
            bool is_input = true;
            bool is_output = true;
            
            for (auto& n : flowsheet->nodos()) {
                for (int64_t sid : n.ids_flujos_salida) {
                    if (sid == fid && n.tipo != nucleo::TipoNodo::Fuente) {
                        is_input = false;
                    }
                }
                for (int64_t sid : n.ids_flujos_entrada) {
                    if (sid == fid && n.tipo != nucleo::TipoNodo::Sumidero) {
                        is_output = false;
                    }
                }
            }
            
            SpeciesRow r;
            std::string name = f.nombre.empty() ? ("Stream_" + std::to_string(fid)) : f.nombre;
            strncpy(r.formula, name.c_str(), sizeof(r.formula)-1);
            r.temper = f.temperatura - 273.15; // default C
            r.pressure = f.presion;
            r.amount_kmol = f.flujo_total() * 3600.0;
            
            if (is_input && !is_output) {
                inputs_group.filas.push_back(r);
            } else {
                outputs_group.filas.push_back(r);
            }
        }
        if (!inputs_group.filas.empty()) entradas_.push_back(inputs_group);
        if (!outputs_group.filas.empty()) salidas_.push_back(outputs_group);
        
        reciclar_calculos();
        calcular_balance_elemental();
    }
    ImGui::PopStyleVar();
    ImGui::Separator();
    
    // ── Columns Split (Spreadsheet vs Tools) ──
    ImGui::Columns(2, "hmb_columns_layout", true);
    static bool init_w_hmb = true;
    if (init_w_hmb) {
        ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.73f);
        ImGui::SetColumnWidth(1, ImGui::GetWindowWidth() * 0.27f);
        init_w_hmb = false;
    }

    // ════════════════════════════════════════════════════════════════════════
    // Columna Izquierda: Spreadsheet Grid & Bottom Panes
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("left_hmb_pane");
    {
        // ── Tabs Selector ──
        if (ImGui::Button(tr("BALANCE"))) tab_actual_ = 0;
        ImGui::SameLine();
        if (ImGui::Button(tr("IN1"))) tab_actual_ = 1;
        ImGui::SameLine();
        if (ImGui::Button(tr("OUT1"))) tab_actual_ = 2;
        ImGui::SameLine();
        if (ImGui::Button(tr("Target"))) tab_actual_ = 3;
        
        ImGui::Separator();
        ImGui::Spacing();

        
        if (tab_actual_ == 1 || tab_actual_ == 2) {
            // Render spreadsheet grid
            const char* header_col = (tab_actual_ == 2) ? "OUTPUT SPECIES" : "INPUT SPECIES";
            const char* unit_t_str = (temp_unit_ == 0) ? "°C" : "K";
            const char* unit_e_str = (energy_unit_ == 0) ? "Mcal" : (energy_unit_ == 1) ? "MJ" : "kWh";
            
            char header_label[128];
            snprintf(header_label, sizeof(header_label), "%s (1)\nFormula", header_col);
            char temp_label[64];
            snprintf(temp_label, sizeof(temp_label), "Temper.\n%s", unit_t_str);
            char h_content_label[64];
            snprintf(h_content_label, sizeof(h_content_label), "Heat Content\n%s/kmol", unit_e_str);
            char total_h_label[64];
            snprintf(total_h_label, sizeof(total_h_label), "Total H\n%s", unit_e_str);
            
            if (ImGui::BeginTable("grid_table", 8, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable, ImVec2(0, 240))) {
                ImGui::TableSetupColumn(header_label, ImGuiTableColumnFlags_WidthFixed, 140.0f);
                ImGui::TableSetupColumn(temp_label, ImGuiTableColumnFlags_WidthFixed, 80.0f);
                ImGui::TableSetupColumn("Pressure\nbar", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                ImGui::TableSetupColumn("Amount\nkmol", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableSetupColumn("Amount\nkg", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableSetupColumn("Amount\nNm³", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableSetupColumn(h_content_label, ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableSetupColumn(total_h_label, ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableHeadersRow();
                
                int s_idx = 0;
                for (auto& g : active_groups) {
                    ImGui::PushID(s_idx);
                    
                    // Stream group header row
                    ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
                    ImGui::TableSetColumnIndex(0);
                    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.5f, 0.8f, 1.0f, 1.0f));
                    ImGui::SetNextItemWidth(-1.0f);
                    ImGui::InputText("##stream_name", g.nombre, sizeof(g.nombre));
                    ImGui::PopStyleColor();
                    
                    for (int c = 1; c < 8; ++c) {
                        ImGui::TableNextColumn();
                        ImGui::TextDisabled(tr("---"));
                    }
                    
                    int r_idx = 0;
                    for (auto& row : g.filas) {
                        ImGui::TableNextRow();
                        ImGui::PushID(r_idx);
                        
                        // Highlight active row selection
                        bool is_selected = (selected_stream_idx_ == s_idx && selected_row_idx_ == r_idx);
                        if (is_selected) {
                            ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, IM_COL32(40, 60, 100, 255));
                        }
                        
                        // Formula
                        ImGui::TableNextColumn();
                        char btn_label[128];
                        snprintf(btn_label, sizeof(btn_label), "%s##fbtn%d_%d", row.formula[0] == '\0' ? "Buscar..." : row.formula, s_idx, r_idx);
                        if (ImGui::Button(btn_label, ImVec2(-1, 0))) {
                            modal_target_stream_ = s_idx;
                            modal_target_row_ = r_idx;
                            ModalBuscadorDB::instancia().abrir("Buscar Especie (HMB)");
                            selected_stream_idx_ = s_idx;
                            selected_row_idx_ = r_idx;
                        }
                        
                        // We will evaluate the modal return at the end of the window to avoid nested UI IDs issues
                        
                        // Temperature
                        ImGui::TableNextColumn();
                        ImGui::SetNextItemWidth(-1.0f);
                        if (ImGui::InputDouble("##temp", &row.temper, 0, 0, "%.3f")) {
                            reciclar_calculos();
                        }
                        
                        // Pressure
                        ImGui::TableNextColumn();
                        ImGui::SetNextItemWidth(-1.0f);
                        ImGui::InputDouble("##pres", &row.pressure, 0, 0, "%.1f");
                        
                        // Amount kmol
                        ImGui::TableNextColumn();
                        ImGui::SetNextItemWidth(-1.0f);
                        if (ImGui::InputDouble("##kmol", &row.amount_kmol, 0, 0, "%.3f")) {
                            reciclar_calculos();
                            calcular_balance_elemental();
                        }
                        
                        // Amount kg (computed)
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.3f"), row.amount_kg);
                        
                        // Amount Nm3 (computed)
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.3f"), row.amount_nm3);
                        
                        // Heat Content (computed)
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.2f"), row.heat_content);
                        
                        // Total H (computed)
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.2f"), row.total_h);
                        
                        ImGui::PopID();
                        r_idx++;
                    }
                    
                    // Heat Loss Row
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0);
                    ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.6f, 0.6f, 1.0f), "Heat Loss:");
                    
                    ImGui::TableSetColumnIndex(7);
                    ImGui::SetNextItemWidth(-1.0f);
                    if (ImGui::InputDouble("##loss", &g.heat_loss, 0, 0, "%.2f")) {
                        reciclar_calculos();
                    }
                    
                    ImGui::PopID();
                    s_idx++;
                }
                ImGui::EndTable();
            }
        } 
        else if (tab_actual_ == 0) {
            // BALANCE tab
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f, 0.8f, 0.6f, 1.0f), "Heat & Material Overall Balance Summary");
            ImGui::Separator();
            ImGui::Spacing();
            
            double total_kmol_in = 0, total_kg_in = 0, total_nm3_in = 0, total_h_in = 0;
            for (auto& g : entradas_) {
                for (auto& r : g.filas) {
                    total_kmol_in += r.amount_kmol;
                    total_kg_in += r.amount_kg;
                    total_nm3_in += r.amount_nm3;
                    total_h_in += r.total_h;
                }
                total_h_in += g.heat_loss;
            }
            double total_kmol_out = 0, total_kg_out = 0, total_nm3_out = 0, total_h_out = 0;
            for (auto& g : salidas_) {
                for (auto& r : g.filas) {
                    total_kmol_out += r.amount_kmol;
                    total_kg_out += r.amount_kg;
                    total_nm3_out += r.amount_nm3;
                    total_h_out += r.total_h;
                }
                total_h_out += g.heat_loss;
            }
            
            const char* unit_e = (energy_unit_ == 0) ? "Mcal" : (energy_unit_ == 1) ? "MJ" : "kWh";
            if (ImGui::BeginTable("summary_table", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                ImGui::TableSetupColumn("Tipo");
                ImGui::TableSetupColumn("Amount (kmol)");
                ImGui::TableSetupColumn("Amount (kg)");
                ImGui::TableSetupColumn("Amount (Nm³)");
                char en_hdr[64]; snprintf(en_hdr, sizeof(en_hdr), "Total Enthalpy (%s)", unit_e);
                ImGui::TableSetupColumn(en_hdr);
                ImGui::TableHeadersRow();
                
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text(tr("Total ENTRADAS"));
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kmol_in);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kg_in);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_nm3_in);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), total_h_in);
                
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text(tr("Total SALIDAS"));
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kmol_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kg_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_nm3_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), total_h_out);
                
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(1,1,0,1), "Residual Difference");
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kmol_in - total_kmol_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kg_in - total_kg_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_nm3_in - total_nm3_out);
                
                double residual_h = total_h_in - total_h_out;
                if (std::abs(residual_h) < 1e-2) {
                    ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0.3f, 0.9f, 0.4f, 1.0f), "%.3f (OK)", residual_h);
                } else {
                    ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f, 0.4f, 0.3f, 1.0f), "%.3f (UNBALANCED)", residual_h);
                }
                
                ImGui::EndTable();
            }
        }
        else if (tab_actual_ == 3) {
            // Target Tab
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.7f, 0.7f, 1.0f, 1.0f), "Goal Seek / Solver Configuration");
            ImGui::Separator();
            ImGui::Spacing();
            ImGui::Text(tr("Calculate Adiabatic flame temp or zero balance temperature:"));
            ImGui::BulletText("Target Variable: Output Temperature");
            ImGui::BulletText("Target Value of Enthalpy Difference: 0.0");
            if (ImGui::Button(tr("Run Temp. Balance Solver"), ImVec2(-1, 30))) {
                Temp_Balance();
            }
        }
        
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();
        
        // ── Bottom Columns (Element Balance & Total Balance) ──
        ImGui::Columns(2, "bottom_panes_hmb", true);
        
        // Element Balance
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.6f, 1.0f), "Element Balance");
        if (ImGui::BeginTable("elem_table", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 110))) {
            ImGui::TableSetupColumn("Element");
            ImGui::TableSetupColumn("IN (kmol)");
            ImGui::TableSetupColumn("OUT (kmol)");
            ImGui::TableSetupColumn("Balance (kmol)");
            ImGui::TableHeadersRow();
            
            for (auto& b : elem_balance_) {
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text(tr("%s"), b.element.c_str());
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), b.in_val);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), b.out_val);
                if (std::abs(b.balance) < 1e-4) {
                    ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0.3f, 0.9f, 0.4f, 1.0f), "0.000");
                } else {
                    ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f, 0.4f, 0.3f, 1.0f), "%.3f", b.balance);
                }
            }
            ImGui::EndTable();
        }
        
        ImGui::NextColumn();
        
        // Total Balance
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.6f, 1.0f), "Total Balance");
        if (ImGui::BeginTable("tot_bal_areas", 7, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 110))) {
            ImGui::TableSetupColumn("BALANCE AREAS");
            ImGui::TableSetupColumn("Amount kmol");
            ImGui::TableSetupColumn("Amount kg");
            ImGui::TableSetupColumn("Amount Nm³");
            ImGui::TableSetupColumn("Heat Content");
            ImGui::TableSetupColumn("H Total");
            ImGui::TableSetupColumn("Exergy");
            ImGui::TableHeadersRow();
            
            double sum_kmol = 0, sum_kg = 0, sum_nm3 = 0, sum_hc = 0, sum_h = 0;
            for (auto& g : salidas_) {
                for (auto& r : g.filas) {
                    sum_kmol += r.amount_kmol;
                    sum_kg += r.amount_kg;
                    sum_nm3 += r.amount_nm3;
                    sum_hc += r.heat_content;
                    sum_h += r.total_h;
                }
            }
            
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("Balance (1)"));
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_kmol);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_kg);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_nm3);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sum_hc);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sum_h);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), -669.07); // Simulated Exergy
            
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("Total Balance"));
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_kmol);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_kg);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_nm3);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sum_hc);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sum_h);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), -669.07);
            
            ImGui::EndTable();
        }
        
        ImGui::Columns(1);
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();

    // ════════════════════════════════════════════════════════════════════════
    // Columna Derecha: Tools Panel
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("right_hmb_pane");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.6f, 0.8f, 1.0f), "Tools");
        ImGui::Separator();
        ImGui::Spacing();
        
        // Balance Area selection
        ImGui::Text(tr("Área de Balance"));
        static int area_sel = 0;
        ImGui::SetNextItemWidth(-1.0f);
        ImGui::Combo("##bal_area_sel", &area_sel, "Balance Area 1\0Balance Area 2\0");
        ImGui::Spacing();
        
        // Get Species from Database
        ImGui::Text(tr("Get Species from Database"));
        if (ImGui::Button(tr("Entrada"), ImVec2(70, 24))) {
            if (tab_actual_ == 1 && !entradas_.empty()) {
                SpeciesRow r;
                strncpy(r.formula, "O2(g)", sizeof(r.formula)-1);
                entradas_[0].filas.push_back(r);
                reciclar_calculos();
                calcular_balance_elemental();
            }
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Salida"), ImVec2(70, 24))) {
            if (tab_actual_ == 2 && !salidas_.empty()) {
                SpeciesRow r;
                strncpy(r.formula, "CO2(g)", sizeof(r.formula)-1);
                salidas_[0].filas.push_back(r);
                reciclar_calculos();
                calcular_balance_elemental();
            }
        }
        ImGui::Spacing();
        ImGui::Separator();
        
        // Insert
        ImGui::Text(tr("Insert"));
        if (ImGui::Button(tr("Área de Balance"), ImVec2(110, 24))) {
            StreamGroup g;
            snprintf(g.nombre, sizeof(g.nombre), "Area %zu", active_groups.size() + 1);
            active_groups.push_back(g);
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Eliminar Área"), ImVec2(110, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                active_groups.erase(active_groups.begin() + selected_stream_idx_);
                selected_stream_idx_ = std::max(0, selected_stream_idx_ - 1);
                reciclar_calculos();
                calcular_balance_elemental();
            } else if (!active_groups.empty()) {
                active_groups.pop_back();
            }
        }
        if (ImGui::Button(tr("Corriente"), ImVec2(100, 24))) {
            StreamGroup g;
            snprintf(g.nombre, sizeof(g.nombre), "Stream %zu", active_groups.size() + 1);
            active_groups.push_back(g);
        }
        ImGui::SameLine();
        ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 15);
        if (ImGui::Button(tr("Selected Stream"), ImVec2(120, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                active_groups.erase(active_groups.begin() + selected_stream_idx_);
                selected_stream_idx_ = std::max(0, selected_stream_idx_ - 1);
                reciclar_calculos();
                calcular_balance_elemental();
            }
        }
        
        if (ImGui::Button(tr("Fila"), ImVec2(100, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                SpeciesRow r;
                active_groups[selected_stream_idx_].filas.push_back(r);
            }
        }
        ImGui::SameLine();
        ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 15);
        if (ImGui::Button(tr("Selected Row"), ImVec2(120, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                auto& stream = active_groups[selected_stream_idx_];
                if (selected_row_idx_ >= 0 && selected_row_idx_ < (int)stream.filas.size()) {
                    stream.filas.erase(stream.filas.begin() + selected_row_idx_);
                    selected_row_idx_ = std::max(0, selected_row_idx_ - 1);
                    reciclar_calculos();
                    calcular_balance_elemental();
                }
            }
        }
        
        if (ImGui::Button(tr("Flujo de Calor"), ImVec2(100, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                active_groups[selected_stream_idx_].heat_loss += 50.0;
                reciclar_calculos();
            }
        }
        
        ImGui::Spacing();
        ImGui::Separator();
        
        // Measure Units
        ImGui::Text(tr("Measure Units"));
        ImGui::Text(tr("Temperature:")); ImGui::SameLine();
        if (ImGui::RadioButton("°C", &temp_unit_, 0)) reciclar_calculos();
        ImGui::SameLine();
        if (ImGui::RadioButton("K", &temp_unit_, 1)) reciclar_calculos();
        
        ImGui::Text(tr("Energy:"));
        if (ImGui::RadioButton("Mcal", &energy_unit_, 0)) reciclar_calculos();
        ImGui::SameLine();
        if (ImGui::RadioButton("MJ", &energy_unit_, 1)) reciclar_calculos();
        ImGui::SameLine();
        if (ImGui::RadioButton("kWh", &energy_unit_, 2)) reciclar_calculos();
        
        ImGui::Spacing();
        ImGui::Separator();
        
        // Functions
        ImGui::Text(tr("Functions"));
        ImGui::Checkbox(tr("Enable Predictive Typing"), &enable_predictive_typing_);
        
        if (ImGui::Button(tr("Temp. Balance"), ImVec2(110, 26))) {
            Temp_Balance();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Resumen"), ImVec2(100, 26))) {
            tab_actual_ = 0; // go to BALANCE
        }
        
        ImGui::Spacing();
        if (ImGui::Button(tr("Diagrama"), ImVec2(-1, 30))) {
            show_diagram_ = !show_diagram_;
        }
    }
    ImGui::EndChild();
    
    std::vector<const nucleo::Especie*> sel_hmb;
    if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (HMB)", sel_hmb, false)) { // Single selection
        if (!sel_hmb.empty() && modal_target_stream_ != -1 && modal_target_row_ != -1) {
            std::vector<StreamGroup>& groups = (tab_actual_ == 2) ? salidas_ : entradas_;
            if (modal_target_stream_ < (int)groups.size() && modal_target_row_ < (int)groups[modal_target_stream_].filas.size()) {
                strncpy(groups[modal_target_stream_].filas[modal_target_row_].formula, sel_hmb[0]->formula.c_str(), 63);
                reciclar_calculos();
                calcular_balance_elemental();
            }
        }
        modal_target_stream_ = -1;
        modal_target_row_ = -1;
    }
    
    ImGui::Columns(1);
    
    // ── Diagram window popup ──
    if (show_diagram_) {
        ImGui::SetNextWindowSize(ImVec2(450, 320), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Diagram (Adiabatic Flame Temp)"), &show_diagram_)) {
            if (!entradas_.empty() && !entradas_[0].filas.empty()) {
                double orig_val = entradas_[0].filas[0].amount_kmol;
                
                std::vector<double> xs;
                std::vector<double> ys;
                
                for (double x = 1.0; x <= 12.0; x += 1.0) {
                    entradas_[0].filas[0].amount_kmol = x;
                    Temp_Balance();
                    double solved_t = salidas_[0].filas[0].temper;
                    xs.push_back(x);
                    ys.push_back(solved_t);
                }
                
                entradas_[0].filas[0].amount_kmol = orig_val;
                reciclar_calculos();
                
                std::string label = "Adiabatic Flame Temp vs " + std::string(entradas_[0].filas[0].formula) + " Input";
                if (ImPlot::BeginPlot(label.c_str(), ImVec2(-1, -1))) {
                    ImPlot::SetupAxes("Reactant Input (kmol)", "Temperature (°C)");
                    ImPlot::PlotLine("Adiabatic Temp", xs.data(), ys.data(), (int)xs.size());
                    ImPlot::EndPlot();
                }
            } else {
                ImGui::Text(tr("Agrega especies de entrada para generar el diagrama."));
            }
            ImGui::End();
        }
    }
}


// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
