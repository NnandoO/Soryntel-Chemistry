#include "shared.hpp"
#include "application/SimulationService.hpp"
#include "domain/ThermoSnapshot.hpp"
namespace cma { namespace interfaz {
// Panel de Equilibrio de Fases
// ═══════════════════════════════════════════════════════════════════════════

void PanelEquilibrioFases::renderizar(const std::map<int64_t, nucleo::Especie>& db) {
    if (calculation_done_) {
        if (calc_thread_.joinable()) calc_thread_.join();

        x_values_ = std::move(temp_x_values_);
        y_values_ = std::move(temp_y_values_);
        z_values_ = std::move(temp_z_values_);
        g_values_ = std::move(temp_g_values_);
        chart_surfaces_ = std::move(temp_chart_surfaces_);
        error_msg_ = temp_error_msg_;
        plot_t_steps_ = temp_plot_t_steps_;
        plot_p_steps_ = temp_plot_p_steps_;
        if (table_p_index_ >= plot_p_steps_) table_p_index_ = plot_p_steps_ > 0 ? plot_p_steps_ - 1 : 0;

        calculado_ = true;
        if (error_msg_.empty()) {
            show_chart_window_ = true;
        }

        is_calculating_ = false;
        calculation_done_ = false;
    }

    ImGui::Text(tr("Equilibrium Compositions (Gibbs Energy Minimization)"));
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("GEM: equilibrium composition and prevailing phases");
    ImGui::Separator();

    ImGui::Columns(2, "eq_calc_columns", false);
    ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.6f);

    // --- Left Pane: Species List ---
    ImGui::Text(tr("System Species"));
    if (ImGui::BeginTable("eq_species_table", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, ImGui::GetContentRegionAvail().y - 80))) {
        ImGui::TableSetupColumn("Fórmula");
        ImGui::TableSetupColumn("Nombre");
        ImGui::TableSetupColumn("Amount (kmol)");
        ImGui::TableHeadersRow();

        for (size_t i = 0; i < species_list_.size(); ++i) {
            auto& sp = species_list_[i];
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), sp.formula.c_str());
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), sp.name.c_str());
            ImGui::TableNextColumn();
            ImGui::PushID((int)i);
            ImGui::SetNextItemWidth(-FLT_MIN);
            ImGui::InputDouble("##amt", &sp.initial_amount, 0.1, 1.0, "%.5f");
            ImGui::PopID();
        }
        ImGui::EndTable();
    }

    if (ImGui::Button(tr("Examinar Base de Datos##fases"), ImVec2(-1, 0))) {
        ModalBuscadorDB::instancia().abrir("Buscar Especie (Fases)");
    }
    
    std::vector<const nucleo::Especie*> sel_fases;
    if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Fases)", sel_fases, true)) {
        for (auto* sp : sel_fases) {
            bool exists = false;
            for (auto& s : species_list_) if (s.id == sp->id) exists = true;
            if (!exists) {
                species_list_.push_back({sp->id, sp->nombre, sp->formula, std::string(cma::nucleo::nombre_fase(sp->fase_std)), 0.0});
            }
        }
    }

    // ── Selección por elementos (como HSC: "select the elements of the system") ──
    ImGui::Text(tr("Elements (e.g. Na, S, O, C):"));
    ImGui::SetNextItemWidth(-1);
    ImGui::InputTextWithHint("##elements_in", "Na, S, O, C...", elementos_buf_, sizeof(elementos_buf_));
    ImGui::Checkbox(tr("All elements must exist"), &todos_elementos_);
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Only species containing ALL the selected elements");
    ImGui::SameLine();
    ImGui::Checkbox(tr("Cell calculation"), &modo_celda_);
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Electrochemical cell: adds e- and charge balance");

    if (ImGui::Button(tr("Generate Species"), ImVec2(-1, 0))) {
        auto& tp = nucleo::TablaPeriodica::instancia();
        std::set<std::string> elementos;
        // Del campo de texto; si está vacío, deducir de la alimentación
        std::string ebuf = elementos_buf_;
        if (ebuf.find_first_not_of(" \t") != std::string::npos) {
            for (auto& tok : split_str(ebuf, ",")) {
                std::string el = trim_str(tok);
                if (!el.empty()) elementos.insert(el);
            }
        } else {
            for (auto& sp : species_list_) {
                if (sp.initial_amount > 0.0) {
                    auto atomos = tp.analizar_formula(sp.formula);
                    for (auto& a : atomos) elementos.insert(a.elem);
                }
            }
        }

        if (!elementos.empty()) {
            for (auto& [sid, db_sp] : db) {
                auto atomos = tp.analizar_formula(db_sp.formula);
                if (atomos.empty()) continue;
                std::set<std::string> elems_sp;
                for (auto& a : atomos) elems_sp.insert(a.elem);
                bool valid = true;
                for (auto& e : elems_sp) {
                    if (elementos.find(e) == elementos.end()) { valid = false; break; }
                }
                if (valid && todos_elementos_ && elems_sp != elementos) valid = false;
                if (valid) {
                    bool exists = false;
                    for (auto& s : species_list_) if (s.id == sid) exists = true;
                    if (!exists) {
                        species_list_.push_back({sid, db_sp.nombre, db_sp.formula, std::string(cma::nucleo::nombre_fase(db_sp.fase_std)), 0.0});
                    }
                }
            }
        }

        // Modo celda: incluir el electrón para permitir redox con balance de carga
        if (modo_celda_) {
            bool existe_e = false;
            for (auto& s : species_list_) if (s.formula == "e-") existe_e = true;
            if (!existe_e) {
                for (auto& [sid, db_sp] : db) {
                    if (db_sp.formula == "e-") {
                        species_list_.push_back({sid, db_sp.nombre, db_sp.formula, std::string(cma::nucleo::nombre_fase(db_sp.fase_std)), 0.0});
                        break;
                    }
                }
            }
        }
    }

    // ── Guardar / cargar composición del sistema (archivo .gem) ──
    if (ImGui::Button(tr("Save Species File"), ImVec2(ImGui::GetContentRegionAvail().x * 0.5f - 4, 0))) {
        FILE* zen = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.gem' --title='Save GEM System' 2>/dev/null");
        if (zen) {
            char buf[512] = {0};
            if (fgets(buf, sizeof(buf), zen)) {
                std::string path(buf);
                if (!path.empty() && path.back() == '\n') path.pop_back();
                if (!path.empty()) {
                    if (path.find(".gem") == std::string::npos) path += ".gem";
                    FILE* fp = fopen(path.c_str(), "w");
                    if (fp) {
                        fprintf(fp, "# GEM species file: formula;amount_kmol\n");
                        for (auto& s : species_list_)
                            fprintf(fp, "%s;%.6g\n", s.formula.c_str(), s.initial_amount);
                        fclose(fp);
                    }
                    // Guardar el nombre base como título del gráfico
                    std::string base = path;
                    size_t slash = base.find_last_of('/');
                    if (slash != std::string::npos) base = base.substr(slash + 1);
                    if (base.size() > 4 && base.substr(base.size() - 4) == ".gem") base = base.substr(0, base.size() - 4);
                    strncpy(titulo_chart_, base.c_str(), sizeof(titulo_chart_) - 1);
                    titulo_chart_[sizeof(titulo_chart_) - 1] = '\0';
                }
            }
            fclose(zen);
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Load Species File"), ImVec2(-1, 0))) {
        FILE* zen = adapters::dialog_pipe("zenity --file-selection --file-filter='*.gem' --title='Load GEM System' 2>/dev/null");
        if (zen) {
            char buf[512] = {0};
            if (fgets(buf, sizeof(buf), zen)) {
                std::string path(buf);
                if (!path.empty() && path.back() == '\n') path.pop_back();
                if (!path.empty()) {
                    FILE* fp = fopen(path.c_str(), "r");
                    if (fp) {
                        char line[256];
                        while (fgets(line, sizeof(line), fp)) {
                            std::string l = trim_str(line);
                            if (l.empty() || l[0] == '#') continue;
                            auto parts = split_str(l, ";");
                            std::string form = trim_str(parts[0]);
                            if (form.empty()) continue;
                            double amt = 0.0;
                            if (parts.size() > 1) { try { amt = std::stod(trim_str(parts[1])); } catch (...) {} }
                            const auto* esp = buscar_especie_por_nombre_o_formula(form);
                            if (!esp) continue;
                            bool exists = false;
                            for (auto& s : species_list_)
                                if (s.id == esp->id) { exists = true; s.initial_amount = amt; break; }
                            if (!exists) {
                                species_list_.push_back({esp->id, esp->nombre, esp->formula, std::string(cma::nucleo::nombre_fase(esp->fase_std)), amt});
                            }
                        }
                        fclose(fp);
                        std::string base = path;
                        size_t slash = base.find_last_of('/');
                        if (slash != std::string::npos) base = base.substr(slash + 1);
                        if (base.size() > 4 && base.substr(base.size() - 4) == ".gem") base = base.substr(0, base.size() - 4);
                        strncpy(titulo_chart_, base.c_str(), sizeof(titulo_chart_) - 1);
                        titulo_chart_[sizeof(titulo_chart_) - 1] = '\0';
                    }
                }
            }
            fclose(zen);
        }
    }

    ImGui::NextColumn();

    // --- Right Pane: Parameters ---
    ImGui::Text(tr("Operations"));
    ImGui::Separator();
    
    bool calc_disabled = is_calculating_;
    if (calc_disabled) ImGui::BeginDisabled();
    if (ImGui::Button(calc_disabled ? "Calculating..." : "Calculate", ImVec2(100, 30))) {
        x_values_.clear();
        y_values_.clear();
        z_values_.clear();
        chart_surfaces_.clear();
        calculado_ = false;
        error_msg_.clear();

        bool any_feed = false;
        for (auto& s : species_list_) if (s.initial_amount > 0.0) any_feed = true;
        if (species_list_.empty() || !any_feed || t_steps_ < 1 || p_steps_ < 1 ||
            t_steps_ > 1000 || p_steps_ > 1000 || static_cast<int64_t>(t_steps_) * p_steps_ > 100000 ||
            !std::isfinite(t_start_) || !std::isfinite(t_end_) || t_start_ <= -273.15 || t_end_ <= -273.15 ||
            !std::isfinite(p_start_) || !std::isfinite(p_end_) || p_start_ <= 0 || p_end_ <= 0) {
            error_msg_ = "Define species and set a positive input amount for at least the feed.";
        } else {
            is_calculating_ = true;
            calc_progress_ = 0.0f;
            calc_cancelled_ = false;

            if (calc_thread_.joinable()) calc_thread_.join();

            auto species_copy = species_list_;
            double ts = t_start_, te = t_end_;        // °C
            int tsteps = t_steps_;
            double ps = p_start_, pe = p_end_;        // bar
            int psteps = p_steps_;
            bool celda = modo_celda_;

            calc_thread_ = std::thread([this, species_copy, ts, te, tsteps, ps, pe, psteps, celda, db]() {
                try {
                domain::ThermoSnapshot snapshot(db);
                application::SimulationService simulation(snapshot);

                double t_step_size = (te - ts) / (std::max(tsteps - 1, 1));
                double p_step_size = (pe - ps) / (std::max(psteps - 1, 1));

                std::vector<double> local_x(tsteps * psteps);   // T (°C)
                std::vector<double> local_y(tsteps * psteps);   // P (bar)
                std::vector<double> local_g(tsteps * psteps, 0.0);
                std::map<int64_t, std::vector<double>> local_z;

                for (auto& sp : species_copy) {
                    local_z[sp.id].resize(tsteps * psteps, 0.0);
                }

                std::map<int64_t, double> warm;  // warm start entre puntos del barrido
                std::string local_error;
                int total_steps = psteps * tsteps;
                int current_step = 0;
                int no_conv = 0;

                for (int i = 0; i < psteps; ++i) {
                    double P_bar = ps + i * p_step_size;
                    double P_atm = P_bar * 0.986923; // bar -> atm
                    for (int j = 0; j < tsteps; ++j) {
                        if (calc_cancelled_) {
                            temp_error_msg_ = "Cálculo cancelado";
                            calculation_done_ = true;
                            return;
                        }
                        double T_C = ts + j * t_step_size;
                        double T_K = T_C + 273.15;
                        int idx = i * tsteps + j;

                        local_x[idx] = T_C;
                        local_y[idx] = P_bar;

                        std::map<int64_t, double> alimentacion;
                        for (auto& sp : species_copy) {
                            alimentacion[sp.id] = sp.initial_amount;
                        }

                        auto res = simulation.run_equilibrium(alimentacion, T_K, P_atm, {}, &warm, celda);
                        if (!res.convergio) {
                            if (++no_conv <= 1 && local_error.empty()) {
                                local_error = "Convergence failed at T=" + std::to_string(T_C) + " °C, P=" + std::to_string(P_bar) + " bar";
                            }
                        }
                        local_g[idx] = res.energia_gibbs_total;

                        for (auto& sp : species_copy) {
                            double amt = res.flujo_molar.count(sp.id) ? res.flujo_molar[sp.id] : 0.0;
                            local_z[sp.id][idx] = amt;
                        }

                        current_step++;
                        calc_progress_ = (float)current_step / (float)total_steps;
                    }
                }

                std::vector<ChartSurface> local_surfaces;
                std::map<std::string, int> fase_numero;
                int proxima_fase = 2; // fase 1 = gas
                for (auto& [sid, z_data] : local_z) {
                    double max_amt = 0.0;
                    for (double a : z_data) max_amt = std::max(max_amt, a);
                    if (max_amt > 1e-6) {
                        std::string legend = "Unknown";
                        std::string fase = "?";
                        for (auto& sp : species_copy) {
                            if (sp.id == sid) {
                                legend = sp.formula;
                                fase = sp.phase;
                                break;
                            }
                        }
                        int fnum;
                        if (fase == "Gas") fnum = 1;
                        else {
                            auto itf = fase_numero.find(legend);
                            if (itf == fase_numero.end()) { fase_numero[legend] = proxima_fase; fnum = proxima_fase; ++proxima_fase; }
                            else fnum = itf->second;
                        }
                        local_surfaces.push_back({sid, legend, fase, fnum, z_data, true});
                    }
                }

                temp_x_values_ = std::move(local_x);
                temp_y_values_ = std::move(local_y);
                temp_z_values_ = std::move(local_z);
                temp_g_values_ = std::move(local_g);
                temp_chart_surfaces_ = std::move(local_surfaces);
                temp_error_msg_ = local_error;
                temp_plot_t_steps_ = tsteps;
                temp_plot_p_steps_ = psteps;

                } catch (const std::exception& e) {
                    temp_error_msg_ = std::string("Error de cálculo: ") + e.what();
                } catch (...) {
                    temp_error_msg_ = "Error inesperado en cálculo de equilibrio";
                }
                calculation_done_ = true;
            });
        }
    }
    if (calc_disabled) ImGui::EndDisabled();

    if (is_calculating_) {
        ImGui::SameLine();
        ImGui::ProgressBar(calc_progress_, ImVec2(150, 30), "Running...");
        ImGui::SameLine();
        if (ImGui::Button("Cancelar cálculo")) calc_cancelled_ = true;
    }

    ImGui::SameLine();
    bool show_chart_disabled = is_calculating_ || !calculado_;
    if (show_chart_disabled) ImGui::BeginDisabled();
    if (ImGui::Button(tr("Show Chart"), ImVec2(100, 30))) {
        show_chart_window_ = true;
    }
    if (show_chart_disabled) ImGui::EndDisabled();
    
    ImGui::SameLine();
    bool clear_disabled = is_calculating_;
    if (clear_disabled) ImGui::BeginDisabled();
    if (ImGui::Button(tr("Limpiar"), ImVec2(100, 30))) {
        species_list_.clear();
        chart_surfaces_.clear();
        calculado_ = false;
        error_msg_.clear();
    }
    if (clear_disabled) ImGui::EndDisabled();

    ImGui::SameLine();
    bool table_disabled = is_calculating_ || !calculado_;
    if (table_disabled) ImGui::BeginDisabled();
    if (ImGui::Button(tr("Table"), ImVec2(100, 30))) {
        show_table_window_ = true;
    }
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Tabular numerical data (can be saved as text file)");
    if (table_disabled) ImGui::EndDisabled();

    ImGui::Spacing();
    ImGui::Text(tr("Chart Title:"));
    ImGui::SameLine();
    ImGui::SetNextItemWidth(-1);
    ImGui::InputText("##gem_title", titulo_chart_, sizeof(titulo_chart_));

    ImGui::Spacing();
    ImGui::Text(tr("System State"));
    ImGui::Separator();

    if (ImGui::BeginTable("eq_sweep_params", 4, ImGuiTableFlags_BordersInnerH | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Variable", ImGuiTableColumnFlags_WidthFixed, 130.0f);
        ImGui::TableSetupColumn("Initial State", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Final State", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Number of Steps", ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableHeadersRow();

        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Temperature (°C)"));
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Ti", &t_start_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Tf", &t_end_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputInt("##Ts", &t_steps_);
        if (t_steps_ < 2) t_steps_ = 2;

        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Pressure (bar)"));
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Pi", &p_start_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Pf", &p_end_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputInt("##Ps", &p_steps_);
        if (p_steps_ < 2) p_steps_ = 2;

        ImGui::EndTable();
    }

    if (!error_msg_.empty()) {
        ImGui::Spacing();
        ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.4f, 0.4f, 1.0f), "%s", error_msg_.c_str());
    }

    ImGui::Columns(1);
    
    if (show_chart_window_) {
        ImGui::SetNextWindowSize(ImVec2(1000, 650), ImGuiCond_FirstUseEver);
        std::string wintitle = std::string("Equilibrium Calculations - [Chart C:|CMA|") + titulo_chart_ + ".gem]###gem_chart";
        if (ImGui::Begin(wintitle.c_str(), &show_chart_window_)) {
            if (!error_msg_.empty()) {
                ImGui::Spacing();
                ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.4f, 0.4f, 1.0f), "Warning: %s", error_msg_.c_str());
                ImGui::Spacing();
            }
            if (ImGui::BeginTable("ChartLayout", 2, ImGuiTableFlags_None, ImVec2(-1, -1))) {
                ImGui::TableSetupColumn("PlotCol", ImGuiTableColumnFlags_WidthStretch, 0.72f);
                ImGui::TableSetupColumn("LegendCol", ImGuiTableColumnFlags_WidthStretch, 0.28f);
                ImGui::TableNextRow();
                ImGui::TableNextColumn();

                if (calculado_ && x_values_.size() > 0) {
                    if (ImPlot3D::BeginPlot(titulo_chart_[0] ? titulo_chart_ : "Equilibrium", ImVec2(-1, -1))) {
                        ImPlot3D::SetupAxes("Temperature, °C", "Pressure, bar", "Equilibrium Amount, kmol");

                        for (auto& surf : chart_surfaces_) {
                            if (surf.visible) {
                                ImPlot3D::PlotSurface(surf.legend.c_str(), x_values_.data(), y_values_.data(), surf.z_data.data(), plot_t_steps_, plot_p_steps_);
                            }
                        }
                        ImPlot3D::EndPlot();
                    }
                } else {
                    ImGui::Text(tr("No data to display. Please calculate first."));
                }

                ImGui::TableNextColumn();

                ImGui::Text(tr("Chart parameters"));
                ImGui::Separator();
                // Leyenda estilo HSC: Description | Phase | visible
                ImGui::Text(tr("Legend"));
                if (ImGui::BeginTable("##gem_legend", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, -60))) {
                    ImGui::TableSetupColumn("", ImGuiTableColumnFlags_WidthFixed, 24.0f);
                    ImGui::TableSetupColumn("Description", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Phase", ImGuiTableColumnFlags_WidthFixed, 45.0f);
                    ImGui::TableHeadersRow();
                    for (auto& surf : chart_surfaces_) {
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn();
                        ImGui::PushID((int)(surf.id) + 9200);
                        ImGui::Checkbox("##vis", &surf.visible);
                        ImGui::PopID();
                        ImGui::TableNextColumn();
                        ImGui::Text("%s (%s)", surf.legend.c_str(), surf.fase.c_str());
                        ImGui::TableNextColumn();
                        ImGui::Text("%d", surf.fase_num);
                    }
                    ImGui::EndTable();
                }

                if (ImGui::Button(tr("Table"), ImVec2(-1, 0)) && calculado_) {
                    show_table_window_ = true;
                }

                ImGui::EndTable();
            }
        }
        ImGui::End();
    }

    // ── Ventana de tabla numérica (botón "Table" de HSC) ──
    if (show_table_window_ && calculado_ && !x_values_.empty()) {
        ImGui::SetNextWindowSize(ImVec2(950, 520), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Equilibrium Data Table"), &show_table_window_)) {
            // Selector de presión (la tabla muestra un corte T vs especies a P fija)
            if (plot_p_steps_ > 1) {
                ImGui::SliderInt("Pressure row", &table_p_index_, 0, plot_p_steps_ - 1);
                ImGui::SameLine();
            }
            int p_idx = std::clamp(table_p_index_, 0, plot_p_steps_ > 0 ? plot_p_steps_ - 1 : 0);
            double P_sel = y_values_[p_idx * plot_t_steps_];
            ImGui::Text("P = %.4g bar", P_sel);

            // Especies con cantidad significativa en este corte
            std::vector<int64_t> ids_tabla;
            for (auto& surf : chart_surfaces_) {
                bool significativa = false;
                for (int j = 0; j < plot_t_steps_; ++j)
                    if (surf.z_data[p_idx * plot_t_steps_ + j] > 1e-8) significativa = true;
                if (significativa) ids_tabla.push_back(surf.id);
            }

            if (ImGui::BeginTable("##gem_table", 2 + (int)ids_tabla.size(),
                ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_NoHostExtendX,
                ImVec2(0, ImGui::GetContentRegionAvail().y - 45)))
            {
                ImGui::TableSetupScrollFreeze(0, 1);
                ImGui::TableSetupColumn("T (°C)", ImGuiTableColumnFlags_WidthFixed, 80);
                ImGui::TableSetupColumn("G (kJ)", ImGuiTableColumnFlags_WidthFixed, 105);
                for (int64_t sid : ids_tabla) {
                    std::string hdr = "?";
                    for (auto& surf : chart_surfaces_) if (surf.id == sid) hdr = surf.legend;
                    ImGui::TableSetupColumn(hdr.c_str(), ImGuiTableColumnFlags_WidthFixed, 95);
                }
                ImGui::TableHeadersRow();

                for (int j = 0; j < plot_t_steps_; ++j) {
                    int idx = p_idx * plot_t_steps_ + j;
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("%.1f", x_values_[idx]);
                    ImGui::TableNextColumn(); ImGui::Text("%.4e", g_values_.size() > (size_t)idx ? g_values_[idx] : 0.0);
                    for (int64_t sid : ids_tabla) {
                        ImGui::TableNextColumn();
                        auto itz = z_values_.find(sid);
                        double v = (itz != z_values_.end()) ? itz->second[idx] : 0.0;
                        if (std::abs(v) < 1e-6) ImGui::TextDisabled("0"); else ImGui::Text("%.4f", v);
                    }
                }
                ImGui::EndTable();
            }

            // Guardar tabla completa (todos los cortes T x P) como texto (HSC Table button)
            if (ImGui::Button(tr("Save Table (.txt)"), ImVec2(150, 26))) {
                FILE* zen = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.txt' --title='Save Equilibrium Table' 2>/dev/null");
                if (zen) {
                    char buf[512] = {0};
                    if (fgets(buf, sizeof(buf), zen)) {
                        std::string path(buf);
                        if (!path.empty() && path.back() == '\n') path.pop_back();
                        if (!path.empty()) {
                            if (path.find(".txt") == std::string::npos) path += ".txt";
                            FILE* f = fopen(path.c_str(), "w");
                            if (f) {
                                fprintf(f, "# Equilibrium table: %s\n", titulo_chart_);
                                fprintf(f, "# columns: T(°C), P(bar), G_total(kJ), amounts kmol per species\n");
                                fprintf(f, "T(°C)\tP(bar)\tG(kJ)");
                                for (auto& surf : chart_surfaces_) fprintf(f, "\t%s(%s)", surf.legend.c_str(), surf.fase.c_str());
                                fprintf(f, "\n");
                                for (size_t idx = 0; idx < x_values_.size(); ++idx) {
                                    fprintf(f, "%.2f\t%.4f\t%.6e", x_values_[idx], y_values_[idx],
                                        g_values_.size() > idx ? g_values_[idx] : 0.0);
                                    for (auto& surf : chart_surfaces_)
                                        fprintf(f, "\t%.6e", surf.z_data[idx]);
                                    fprintf(f, "\n");
                                }
                                fclose(f);
                            }
                        }
                    }
                    fclose(zen);
                }
            }
            ImGui::SameLine();
            if (ImGui::Button(tr("Copiar al Portapapeles"), ImVec2(150, 26))) {
                std::string clip = "T(°C)\tP(bar)\tG(kJ)";
                for (auto& surf : chart_surfaces_) { clip += "\t" + surf.legend + "(" + surf.fase + ")"; }
                clip += "\n";
                char buf[128];
                for (size_t idx = 0; idx < x_values_.size(); ++idx) {
                    snprintf(buf, sizeof(buf), "%.2f\t%.4f\t%.6e", x_values_[idx], y_values_[idx],
                        g_values_.size() > idx ? g_values_[idx] : 0.0);
                    clip += buf;
                    for (auto& surf : chart_surfaces_) {
                        snprintf(buf, sizeof(buf), "\t%.6e", surf.z_data[idx]);
                        clip += buf;
                    }
                    clip += "\n";
                }
                ImGui::SetClipboardText(clip.c_str());
            }
        }
        ImGui::End();
    }
}

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
