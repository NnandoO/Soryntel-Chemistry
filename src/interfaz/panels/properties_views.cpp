#include "shared.hpp"

namespace cma { namespace interfaz {

// ═══════════════════════════════════════════════════════════════════════════
void PanelPropiedades::renderizar_stream_viewer() {
    if (!flowsheet_ || flowsheet_->mapa_flujos().empty()) {
        ImGui::TextDisabled("No hay corrientes en el diagrama de flujo.");
        return;
    }

    auto& db = base_datos::GestorDB::instancia();

    // Determinar las corrientes a comparar: si hay nodo seleccionado, sus entradas/salidas; si no, hasta 5 corrientes del proceso
    std::vector<const nucleo::Flujo*> cols;
    if (id_nodo_ > 0) {
        if (auto* n = flowsheet_->nodo_por_id(id_nodo_)) {
            for (int64_t fid : n->ids_flujos_entrada)
                if (auto* f = flowsheet_->flujo_por_id(fid)) cols.push_back(f);
            for (int64_t fid : n->ids_flujos_salida)
                if (auto* f = flowsheet_->flujo_por_id(fid)) cols.push_back(f);
        }
    } else if (id_flujo_ > 0) {
        if (auto* f = flowsheet_->flujo_por_id(id_flujo_)) cols.push_back(f);
        for (const auto& [fid, fl] : flowsheet_->mapa_flujos()) {
            if (fid != id_flujo_ && cols.size() < 4) cols.push_back(&fl);
        }
    }
    if (cols.empty()) {
        for (const auto& [fid, fl] : flowsheet_->mapa_flujos()) {
            cols.push_back(&fl);
            if (cols.size() >= 4) break;
        }
    }

    if (ImGui::BeginTabBar("StreamViewerSubTabs")) {
        if (ImGui::BeginTabItem("Phases")) {
            int num_cols = (int)cols.size() + 2;
            if (ImGui::BeginTable("HSCStreamViewerPhases", num_cols,
                ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollX | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable,
                ImVec2(0, 0))) {
                ImGui::TableSetupColumn("Variable / Species", ImGuiTableColumnFlags_WidthFixed, 125.0f);
                ImGui::TableSetupColumn("Unit", ImGuiTableColumnFlags_WidthFixed, 68.0f);
                for (size_t i = 0; i < cols.size(); ++i) {
                    ImGui::TableSetupColumn(cols[i]->nombre.c_str(), ImGuiTableColumnFlags_WidthFixed, 85.0f);
                }
                ImGui::TableHeadersRow();

                auto add_prop_row = [&](const char* label, const char* unit, std::function<double(const nucleo::Flujo&)> fn, const char* fmt = "%.2f") {
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::TextUnformatted(label);
                    ImGui::TableNextColumn(); ImGui::TextDisabled("%s", unit);
                    for (auto* fl : cols) {
                        ImGui::TableNextColumn();
                        ImGui::Text(fmt, fn(*fl));
                    }
                };

                // Encabezado Total
                ImGui::TableNextRow();
                ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, IM_COL32(215, 228, 242, 255));
                ImGui::TableNextColumn(); ImGui::TextColored(ImVec4(0.05f, 0.2f, 0.45f, 1.0f), "Total Stream");
                for (int c = 1; c < num_cols; ++c) { ImGui::TableNextColumn(); }

                add_prop_row("Amount", "t/h", [&](const nucleo::Flujo& f) { return f.masa_total_t_h(db); }, "%.2f");
                add_prop_row("Molar Flow", "kmol/h", [&](const nucleo::Flujo& f) { return f.flujo_molar_total() * 3600.0; }, "%.2f");
                add_prop_row("Volume Flow", "m³/h", [&](const nucleo::Flujo& f) {
                    double n_g = f.fraccion_gas * f.flujo_molar_total();
                    double v_g = (n_g > 0 && f.presion > 1e-6) ? (n_g * 3600.0 * 0.082057 * f.temperatura / f.presion) : 0.0;
                    double m_sl = (f.masa_total_t_h(db));
                    return v_g + m_sl * 0.65;
                }, "%.2f");
                add_prop_row("Temperature", "°C", [&](const nucleo::Flujo& f) { return f.temperatura - 273.15; }, "%.2f");
                add_prop_row("Pressure", "bar", [&](const nucleo::Flujo& f) { return f.presion * 1.01325; }, "%.2f");
                add_prop_row("Enthalpy", "kW", [&](const nucleo::Flujo& f) { return f.entalpia != 0.0 ? f.entalpia : f.calcular_entalpia(db); }, "%.2f");
                add_prop_row("Exergy", "kW", [&](const nucleo::Flujo& f) {
                    double H = (f.entalpia != 0.0) ? f.entalpia : f.calcular_entalpia(db);
                    double T_ratio = (f.temperatura > 1.0) ? (1.0 - 298.15 / f.temperatura) : 0.0;
                    return std::abs(H) * std::max(0.05, std::abs(T_ratio));
                }, "%.2f");
                add_prop_row("Heat Capacity", "kJ/(kg·°C)", [&](const nucleo::Flujo& f) {
                    return (f.agua_kg_h > 0.0) ? 3.45 : 1.08;
                }, "%.2f");

                // Recolectar todas las especies presentes agrupadas por fase
                std::set<int64_t> gas_ids, liq_ids, sol_ids;
                for (auto* fl : cols) {
                    for (const auto& [sid, n] : fl->flujo_molar) {
                        if (n <= 1e-12) continue;
                        const auto* sp = db.especie_por_id(sid);
                        if (!sp) continue;
                        if (sp->fase_std == nucleo::Fase::Gas) gas_ids.insert(sid);
                        else if (sp->fase_std == nucleo::Fase::Liquido) liq_ids.insert(sid);
                        else sol_ids.insert(sid);
                    }
                }

                auto render_phase_group = [&](const char* phase_title, const std::set<int64_t>& ids) {
                    if (ids.empty()) return;
                    ImGui::TableNextRow();
                    ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, IM_COL32(230, 236, 245, 255));
                    ImGui::TableNextColumn(); ImGui::TextColored(ImVec4(0.1f, 0.3f, 0.55f, 1.0f), "%s", phase_title);
                    ImGui::TableNextColumn(); ImGui::TextDisabled("t/h");
                    for (auto* fl : cols) {
                        double sum_th = 0.0;
                        for (int64_t sid : ids) {
                            auto it = fl->flujo_molar.find(sid);
                            if (it != fl->flujo_molar.end()) {
                                const auto* sp = db.especie_por_id(sid);
                                double mw = sp ? sp->masa_molar : 28.0;
                                sum_th += it->second * mw * 3.6;
                            }
                        }
                        ImGui::TableNextColumn();
                        ImGui::Text("%.2f", sum_th);
                    }

                    for (int64_t sid : ids) {
                        const auto* sp = db.especie_por_id(sid);
                        if (!sp) continue;
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::Text("  %s", sp->formula.c_str());
                        ImGui::TableNextColumn(); ImGui::TextDisabled("t/h");
                        for (auto* fl : cols) {
                            double val_th = 0.0;
                            auto it = fl->flujo_molar.find(sid);
                            if (it != fl->flujo_molar.end()) {
                                val_th = it->second * sp->masa_molar * 3.6;
                            }
                            ImGui::TableNextColumn();
                            ImGui::Text("%.3f", val_th);
                        }
                    }
                };

                render_phase_group("GAS", gas_ids);
                render_phase_group("LIQUID / AQUEOUS", liq_ids);
                render_phase_group("SOLID", sol_ids);

                ImGui::EndTable();
            }
            ImGui::EndTabItem();
        }

        if (ImGui::BeginTabItem("Pyrometallurgy / Elements")) {
            int num_cols = (int)cols.size() + 2;
            if (ImGui::BeginTable("HSCStreamViewerElements", num_cols,
                ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollX | ImGuiTableFlags_Resizable)) {
                ImGui::TableSetupColumn("Element", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableSetupColumn("Unit", ImGuiTableColumnFlags_WidthFixed, 65.0f);
                for (size_t i = 0; i < cols.size(); ++i) {
                    ImGui::TableSetupColumn(cols[i]->nombre.c_str(), ImGuiTableColumnFlags_WidthFixed, 85.0f);
                }
                ImGui::TableHeadersRow();

                // Calcular flujo masico elemental (t/h) para cada corriente
                std::set<std::string> all_elems;
                std::vector<std::map<std::string, double>> stream_elems(cols.size());
                auto& tp = nucleo::TablaPeriodica::instancia();

                for (size_t c = 0; c < cols.size(); ++c) {
                    for (const auto& [sid, n] : cols[c]->flujo_molar) {
                        if (n <= 1e-12) continue;
                        const auto* sp = db.especie_por_id(sid);
                        if (!sp) continue;
                        std::map<std::string, int> comp = sp->composicion_elemental;
                        if (comp.empty()) {
                            for (const auto& ca : tp.analizar_formula(sp->formula))
                                comp[ca.elem] += ca.cuenta;
                        }
                        for (const auto& [el, count] : comp) {
                            const auto* el_info = db.elemento_por_simbolo(el);
                            double aw = el_info ? el_info->masa_atomica : 16.0;
                            double m_th = n * count * aw * 3.6;
                            stream_elems[c][el] += m_th;
                            all_elems.insert(el);
                        }
                    }
                }

                for (const auto& el : all_elems) {
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("  %s", el.c_str());
                    ImGui::TableNextColumn(); ImGui::TextDisabled("t/h");
                    for (size_t c = 0; c < cols.size(); ++c) {
                        ImGui::TableNextColumn();
                        ImGui::Text("%.3f", stream_elems[c][el]);
                    }
                }

                ImGui::EndTable();
            }
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HSC-Sim Spreadsheet Unit Editor (Excel-like Unit Model: Input | Output | Dist | Controls | Tanks)
// ═══════════════════════════════════════════════════════════════════════════
void PanelPropiedades::renderizar_editor_unidad() {
    if (!flowsheet_) return;
    auto* nodo = flowsheet_->nodo_por_id(id_nodo_editor_);
    if (!nodo) {
        mostrar_editor_unidad_ = false;
        return;
    }

    auto& db = base_datos::GestorDB::instancia();
    ImGui::SetNextWindowSize(ImVec2(760, 520), ImGuiCond_FirstUseEver);
    std::string win_title = "HSC-Sim Unit Model Editor (XLS) - " + nodo->nombre + "###UnitEditorXLS";
    if (ImGui::Begin(win_title.c_str(), &mostrar_editor_unidad_)) {
        // Barra superior tipo Excel / HSC-Sim
        ImGui::TextColored(ImVec4(0.10f, 0.35f, 0.65f, 1.0f), "Unit: %s  |  Type: %s",
            nodo->nombre.c_str(), nucleo::nombre_tipo_nodo(nodo->tipo));
        ImGui::SameLine(ImGui::GetWindowWidth() - 260.0f);
        if (ImGui::Button("Calculate (F9)", ImVec2(120, 0))) {
            if (al_ejecutar_simulacion) al_ejecutar_simulacion();
        }
        ImGui::SameLine();
        if (ImGui::Button("Export .CSV/.XLS", ImVec2(125, 0))) {
            std::ofstream ofs("unit_" + nodo->nombre + ".csv");
            if (ofs.is_open()) {
                ofs << "Unit," << nodo->nombre << "\nParameter,Value\n";
                for (const auto& [k, v] : nodo->params) ofs << k << "," << v << "\n";
            }
        }
        ImGui::Separator();

        if (ImGui::BeginTabBar("HSCUnitSpreadsheetTabs")) {
            // ── Pestaña 1: Input ──
            if (ImGui::BeginTabItem("Input")) {
                std::vector<nucleo::Flujo*> in_streams;
                for (int64_t fid : nodo->ids_flujos_entrada)
                    if (auto* f = flowsheet_->flujo_por_id(fid)) in_streams.push_back(f);

                if (in_streams.empty()) {
                    ImGui::TextDisabled("Esta unidad no tiene corrientes de entrada conectadas.");
                } else if (ImGui::BeginTable("UnitInputSheet", (int)in_streams.size() + 3,
                    ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                    ImGui::TableSetupColumn("A: Variable / Species", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("B: Unit", ImGuiTableColumnFlags_WidthFixed, 70.0f);
                    ImGui::TableSetupColumn("C: Total In", ImGuiTableColumnFlags_WidthFixed, 95.0f);
                    for (auto* f : in_streams)
                        ImGui::TableSetupColumn(f->nombre.c_str(), ImGuiTableColumnFlags_WidthFixed, 105.0f);
                    ImGui::TableHeadersRow();

                    // Temperatura (°C) editable
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("Temperature");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("°C");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("-");
                    for (auto* f : in_streams) {
                        ImGui::TableNextColumn();
                        double tc = f->temperatura - 273.15;
                        ImGui::PushID((int)f->id + 100);
                        ImGui::SetNextItemWidth(-1);
                        if (ImGui::InputDouble("##tc", &tc, 0, 0, "%.1f")) {
                            f->temperatura = tc + 273.15;
                            if (al_cambio_flowsheet) al_cambio_flowsheet();
                        }
                        ImGui::PopID();
                    }

                    // Flujo másico total (t/h)
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("Mass Flow");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("t/h");
                    double sum_in_th = 0.0;
                    for (auto* f : in_streams) sum_in_th += f->masa_total_t_h(db);
                    ImGui::TableNextColumn(); ImGui::Text("%.3f", sum_in_th);
                    for (auto* f : in_streams) {
                        ImGui::TableNextColumn();
                        ImGui::Text("%.3f", f->masa_total_t_h(db));
                    }

                    // Especies
                    std::set<int64_t> sids;
                    for (auto* f : in_streams)
                        for (const auto& [sid, _] : f->flujo_molar) sids.insert(sid);

                    for (int64_t sid : sids) {
                        const auto* sp = db.especie_por_id(sid);
                        if (!sp) continue;
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::Text("%s (%s)", sp->formula.c_str(), sp->nombre.c_str());
                        ImGui::TableNextColumn(); ImGui::TextDisabled("kmol/s");
                        double tot_sp = 0.0;
                        for (auto* f : in_streams)
                            if (f->flujo_molar.count(sid)) tot_sp += f->flujo_molar.at(sid);
                        ImGui::TableNextColumn(); ImGui::Text("%.5f", tot_sp);
                        for (auto* f : in_streams) {
                            ImGui::TableNextColumn();
                            double val = f->flujo_molar.count(sid) ? f->flujo_molar.at(sid) : 0.0;
                            ImGui::PushID((int)(f->id * 1000 + sid));
                            ImGui::SetNextItemWidth(-1);
                            if (ImGui::InputDouble("##sp_in", &val, 0, 0, "%.5f")) {
                                f->flujo_molar[sid] = std::max(0.0, val);
                                if (al_cambio_flowsheet) al_cambio_flowsheet();
                            }
                            ImGui::PopID();
                        }
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }

            // ── Pestaña 2: Output ──
            if (ImGui::BeginTabItem("Output")) {
                std::vector<nucleo::Flujo*> out_streams;
                for (int64_t fid : nodo->ids_flujos_salida)
                    if (auto* f = flowsheet_->flujo_por_id(fid)) out_streams.push_back(f);

                if (out_streams.empty()) {
                    ImGui::TextDisabled("Esta unidad no tiene corrientes de salida conectadas.");
                } else if (ImGui::BeginTable("UnitOutputSheet", (int)out_streams.size() + 3,
                    ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                    ImGui::TableSetupColumn("A: Variable / Species", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("B: Unit", ImGuiTableColumnFlags_WidthFixed, 70.0f);
                    ImGui::TableSetupColumn("C: Total Out", ImGuiTableColumnFlags_WidthFixed, 95.0f);
                    for (auto* f : out_streams)
                        ImGui::TableSetupColumn(f->nombre.c_str(), ImGuiTableColumnFlags_WidthFixed, 105.0f);
                    ImGui::TableHeadersRow();

                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("Temperature");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("°C");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("-");
                    for (auto* f : out_streams) {
                        ImGui::TableNextColumn();
                        ImGui::Text("%.1f", f->temperatura - 273.15);
                    }

                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("Mass Flow");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("t/h");
                    double sum_out_th = 0.0;
                    for (auto* f : out_streams) sum_out_th += f->masa_total_t_h(db);
                    ImGui::TableNextColumn(); ImGui::Text("%.3f", sum_out_th);
                    for (auto* f : out_streams) {
                        ImGui::TableNextColumn();
                        ImGui::Text("%.3f", f->masa_total_t_h(db));
                    }

                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("Enthalpy Flow");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("kW");
                    double sum_h = 0.0;
                    for (auto* f : out_streams) sum_h += (f->entalpia != 0.0 ? f->entalpia : f->calcular_entalpia(db));
                    ImGui::TableNextColumn(); ImGui::Text("%.2f", sum_h);
                    for (auto* f : out_streams) {
                        ImGui::TableNextColumn();
                        ImGui::Text("%.2f", f->entalpia != 0.0 ? f->entalpia : f->calcular_entalpia(db));
                    }

                    std::set<int64_t> sids;
                    for (auto* f : out_streams)
                        for (const auto& [sid, _] : f->flujo_molar) sids.insert(sid);

                    for (int64_t sid : sids) {
                        const auto* sp = db.especie_por_id(sid);
                        if (!sp) continue;
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::Text("%s (%s)", sp->formula.c_str(), sp->nombre.c_str());
                        ImGui::TableNextColumn(); ImGui::TextDisabled("t/h");
                        double tot_th = 0.0;
                        for (auto* f : out_streams)
                            if (f->flujo_molar.count(sid)) tot_th += f->flujo_molar.at(sid) * sp->masa_molar * 3.6;
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", tot_th);
                        for (auto* f : out_streams) {
                            ImGui::TableNextColumn();
                            double v_th = f->flujo_molar.count(sid) ? f->flujo_molar.at(sid) * sp->masa_molar * 3.6 : 0.0;
                            ImGui::Text("%.3f", v_th);
                        }
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }

            // ── Pestaña 3: Dist (Elemental & Phase Distribution Model) ──
            if (ImGui::BeginTabItem("Dist")) {
                ImGui::TextWrapped("Distribución por Elementos y Especies hacia Salida 1 (Stream 1 % vs Stream 2 %).");
                const char* common_elems[] = { "Fe", "Cu", "Ti", "Al", "Si", "O", "S", "C", "Cl", "Na", "H", "Ca" };
                if (ImGui::BeginTable("ElemDistTable", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                    ImGui::TableSetupColumn("Element", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                    ImGui::TableSetupColumn("To Output 1 (%)", ImGuiTableColumnFlags_WidthFixed, 160.0f);
                    ImGui::TableSetupColumn("To Output 2 (%)", ImGuiTableColumnFlags_WidthFixed, 140.0f);
                    ImGui::TableSetupColumn("Status", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableHeadersRow();

                    for (const char* el : common_elems) {
                        std::string key = std::string("dist_elem_") + el;
                        bool active = nodo->params.count(key) > 0;
                        float pct1 = active ? (float)(nodo->params[key] * 100.0) : 50.0f;

                        ImGui::TableNextRow();
                        ImGui::TableNextColumn();
                        ImGui::PushID(el);
                        if (ImGui::Checkbox(el, &active)) {
                            if (active) nodo->params[key] = pct1 / 100.0;
                            else nodo->params.erase(key);
                            if (al_cambio_flowsheet) al_cambio_flowsheet();
                        }
                        ImGui::TableNextColumn();
                        if (active) {
                            ImGui::SetNextItemWidth(-1);
                            if (ImGui::SliderFloat("##pct1", &pct1, 0.0f, 100.0f, "%.1f %%")) {
                                nodo->params[key] = pct1 / 100.0;
                                if (al_cambio_flowsheet) al_cambio_flowsheet();
                            }
                        } else {
                            ImGui::TextDisabled("Default (by phase/species)");
                        }
                        ImGui::TableNextColumn();
                        if (active) ImGui::Text("%.1f %%", 100.0f - pct1);
                        else ImGui::TextDisabled("-");
                        ImGui::TableNextColumn();
                        ImGui::TextUnformatted(active ? "Element Distribution Active" : "Inactive");
                        ImGui::PopID();
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }

            // ── Pestaña 4: Controls (Setpoints & Process Parameters) ──
            if (ImGui::BeginTabItem("Controls")) {
                ImGui::TextColored(ImVec4(0.15f, 0.35f, 0.65f, 1.0f), "Parámetros de Control y Reacción de la Unidad:");
                if (ImGui::BeginTable("UnitControlsTable", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                    ImGui::TableSetupColumn("Parameter Key", ImGuiTableColumnFlags_WidthFixed, 200.0f);
                    ImGui::TableSetupColumn("Setpoint Value", ImGuiTableColumnFlags_WidthFixed, 180.0f);
                    ImGui::TableSetupColumn("Description", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableHeadersRow();

                    for (auto& [k, v] : nodo->params) {
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::TextUnformatted(k.c_str());
                        ImGui::TableNextColumn();
                        ImGui::PushID(k.c_str());
                        ImGui::SetNextItemWidth(-1);
                        double val = v;
                        if (ImGui::InputDouble("##ctrl_v", &val, 0, 0, "%.4f")) {
                            nodo->params[k] = val;
                            if (al_cambio_flowsheet) al_cambio_flowsheet();
                        }
                        ImGui::PopID();
                        ImGui::TableNextColumn();
                        ImGui::TextDisabled("HSC-Sim Unit Cell Variable");
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }

            // ── Pestaña 5: Tanks (Dynamic Holdup & Valve Geometry) ──
            if (ImGui::BeginTabItem("Tanks")) {
                ImGui::TextWrapped("Parámetros de Holdup Dinámico y Flujo Gravitacional (Ecuación de Torricelli):");
                double vol = nodo->params.count("tank_volume_m3") ? nodo->params["tank_volume_m3"] : 25.0;
                double lvl0 = nodo->params.count("initial_level_pct") ? nodo->params["initial_level_pct"] : 50.0;
                double k_valve = nodo->params.count("valve_cv") ? nodo->params["valve_cv"] : 150.0;

                if (ImGui::InputDouble("Volumen de Tanque (m³)", &vol, 1.0, 5.0, "%.1f"))
                    nodo->params["tank_volume_m3"] = std::max(0.5, vol);
                double min_lvl = 0.0, max_lvl = 100.0;
                if (ImGui::SliderScalar("Nivel Inicial (%)", ImGuiDataType_Double, &lvl0,
                    &min_lvl, &max_lvl, "%.1f %%"))
                    nodo->params["initial_level_pct"] = lvl0;
                if (ImGui::InputDouble("Coeficiente de Descarga Cv (kg/h / √m)", &k_valve, 5.0, 25.0, "%.1f"))
                    nodo->params["valve_cv"] = std::max(0.0, k_valve);

                ImGui::EndTabItem();
            }

            ImGui::EndTabBar();
        }
    }
    ImGui::End();
}

}} // namespace cma::interfaz
