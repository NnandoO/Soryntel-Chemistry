#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de Simulación Dinámica (HSC-Sim Dynamic Dialog)
// ═══════════════════════════════════════════════════════════════════════════
PanelSimulacionDinamica::PanelSimulacionDinamica() {
    tanques_ = {
        {"Container 1", 2.0, 3.0, 0.0085, 65.0},
        {"Container 2", 2.2, 3.0, 0.0075, 35.0},
        {"Container 3", 2.5, 3.0, 0.0068, 15.0}
    };
    agenda_ = {
        {0.0,  42000.0, 100.0, 100.0, 100.0},
        {30.0, 15000.0,  80.0, 100.0, 100.0},
        {60.0, 55000.0, 100.0,  70.0, 100.0},
        {90.0, 25000.0, 100.0, 100.0,  85.0}
    };
    ejecutar_cascada_tanques();
}

void PanelSimulacionDinamica::ejecutar_cascada_tanques() {
    if (tanques_.size() < 3) return;
    double dt = std::max(0.05, paso_);
    double t0 = std::max(0.0, t_inicial_);
    double tf = std::max(t0 + dt, t_final_);

    const double g = 9.81;
    const double Cd = 0.62;
    const double rho = 1000.0; // kg/m³

    auto eval_agenda = [&](double t) -> EventoDinamicoTanques {
        EventoDinamicoTanques ev = agenda_.empty()
            ? EventoDinamicoTanques{0.0, 36000.0, 100.0, 100.0, 100.0}
            : agenda_.front();
        for (const auto& item : agenda_) {
            if (t >= item.tiempo_s) ev = item;
        }
        return ev;
    };

    auto q_out_kgh = [&](int idx, double h, double valve_pct) -> double {
        if (h <= 1e-6 || valve_pct <= 0.0) return 0.0;
        double v_frac = std::clamp(valve_pct / 100.0, 0.0, 1.0);
        double a = std::max(1e-6, tanques_[idx].area_orificio_m2);
        return 3600.0 * rho * v_frac * Cd * a * std::sqrt(2.0 * g * std::max(0.0, h));
    };

    std::map<std::string, double> y0;
    for (int i = 0; i < 3; ++i) {
        std::string key = "h" + std::to_string(i + 1);
        y0[key] = std::clamp(tanques_[i].nivel_inicial_pct / 100.0, 0.0, 1.0) * tanques_[i].altura_max_m;
    }

    auto ode_fn = [&](double t, const std::map<std::string, double>& y, std::map<std::string, double>& dydt) {
        EventoDinamicoTanques ev = eval_agenda(t);
        double h1 = std::clamp(y.at("h1"), 0.0, tanques_[0].altura_max_m);
        double h2 = std::clamp(y.at("h2"), 0.0, tanques_[1].altura_max_m);
        double h3 = std::clamp(y.at("h3"), 0.0, tanques_[2].altura_max_m);

        double q1 = q_out_kgh(0, h1, ev.valvula1_pct);
        double q2 = q_out_kgh(1, h2, ev.valvula2_pct);
        double q3 = q_out_kgh(2, h3, ev.valvula3_pct);

        double dh1 = (ev.feed_kg_h - q1) / (3600.0 * rho * std::max(0.1, tanques_[0].area_m2));
        double dh2 = (q1 - q2) / (3600.0 * rho * std::max(0.1, tanques_[1].area_m2));
        double dh3 = (q2 - q3) / (3600.0 * rho * std::max(0.1, tanques_[2].area_m2));

        if (h1 >= tanques_[0].altura_max_m && dh1 > 0) dh1 = 0.0;
        if (h2 >= tanques_[1].altura_max_m && dh2 > 0) dh2 = 0.0;
        if (h3 >= tanques_[2].altura_max_m && dh3 > 0) dh3 = 0.0;
        dydt["h1"] = dh1;
        dydt["h2"] = dh2;
        dydt["h3"] = dh3;
    };

    resultado_ = motor_.resolver(t0, tf, dt, y0, ode_fn);
    resultado_.variables["Level 1 (%)"].clear();
    resultado_.variables["Level 2 (%)"].clear();
    resultado_.variables["Level 3 (%)"].clear();
    resultado_.variables["Feed (kg/h)"].clear();
    resultado_.variables["Outflow 1 (kg/h)"].clear();
    resultado_.variables["Outflow 2 (kg/h)"].clear();
    resultado_.variables["Outflow 3 (kg/h)"].clear();

    for (size_t k = 0; k < resultado_.tiempos.size(); ++k) {
        double t = resultado_.tiempos[k];
        EventoDinamicoTanques ev = eval_agenda(t);
        double h1 = std::clamp(resultado_.variables["h1"][k], 0.0, tanques_[0].altura_max_m);
        double h2 = std::clamp(resultado_.variables["h2"][k], 0.0, tanques_[1].altura_max_m);
        double h3 = std::clamp(resultado_.variables["h3"][k], 0.0, tanques_[2].altura_max_m);

        resultado_.variables["Level 1 (%)"].push_back(100.0 * h1 / std::max(0.1, tanques_[0].altura_max_m));
        resultado_.variables["Level 2 (%)"].push_back(100.0 * h2 / std::max(0.1, tanques_[1].altura_max_m));
        resultado_.variables["Level 3 (%)"].push_back(100.0 * h3 / std::max(0.1, tanques_[2].altura_max_m));

        resultado_.variables["Feed (kg/h)"].push_back(ev.feed_kg_h);
        resultado_.variables["Outflow 1 (kg/h)"].push_back(q_out_kgh(0, h1, ev.valvula1_pct));
        resultado_.variables["Outflow 2 (kg/h)"].push_back(q_out_kgh(1, h2, ev.valvula2_pct));
        resultado_.variables["Outflow 3 (kg/h)"].push_back(q_out_kgh(2, h3, ev.valvula3_pct));
    }

    if (!resultado_.tiempos.empty()) {
        char buf[160];
        snprintf(buf, sizeof(buf),
                 "RK4 completado (%zu pasos). Niveles finales: C1=%.1f%%, C2=%.1f%%, C3=%.1f%%",
                 resultado_.tiempos.size(),
                 resultado_.variables["Level 1 (%)"].back(),
                 resultado_.variables["Level 2 (%)"].back(),
                 resultado_.variables["Level 3 (%)"].back());
        resultado_.mensaje = buf;
    }
    calculado_ = true;
}

void PanelSimulacionDinamica::renderizar() {
    // Barra superior estilo HSC-Sim Dynamic Dialog
    ImGui::TextColored(ImVec4(0.25f, 0.58f, 0.96f, 1.0f), "HSC-Sim Dynamic Dialog — Gravitational Flow & Transient ODE Solver");
    ImGui::Separator();

    if (ImGui::Button("Run", ImVec2(75, 28))) {
        if (modo_ == 0) ejecutar_cascada_tanques();
    }
    ImGui::SameLine();
    if (ImGui::Button("Reset", ImVec2(75, 28))) {
        tanques_ = {
            {"Container 1", 2.0, 3.0, 0.0085, 65.0},
            {"Container 2", 2.2, 3.0, 0.0075, 35.0},
            {"Container 3", 2.5, 3.0, 0.0068, 15.0}
        };
        ejecutar_cascada_tanques();
    }
    ImGui::SameLine();
    ImGui::SetNextItemWidth(95);
    ImGui::InputDouble("Start (s)", &t_inicial_, 0, 0, "%.1f");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(95);
    ImGui::InputDouble("End (s)", &t_final_, 0, 0, "%.1f");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(85);
    ImGui::InputDouble("Step (s)", &paso_, 0, 0, "%.2f");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(230);
    if (ImGui::Combo("Model", &modo_,
        "Gravitational Flow (3 Tanks HSC-Sim)\0Single Tank Drain (Torricelli)\0Furnace Cooling\0Batch Reactor\0")) {
        if (modo_ == 0) ejecutar_cascada_tanques();
    }

    ImGui::Separator();

    if (modo_ == 0) {
        if (ImGui::BeginTabBar("HSCDynamicTabs")) {
            if (ImGui::BeginTabItem("Schedule Sheet 1")) {
                ImGui::TextDisabled("Eventos programados de alimentación y apertura de válvulas durante la corrida dinámica:");
                if (ImGui::BeginTable("ScheduleSheetTable", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_SizingStretchProp, ImVec2(0, 125))) {
                    ImGui::TableSetupColumn("Step #", ImGuiTableColumnFlags_WidthFixed, 55.0f);
                    ImGui::TableSetupColumn("Time (s)", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Feed In (kg/h)", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Container 1 Valve (%)", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Container 2 Valve (%)", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Container 3 Valve (%)", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableHeadersRow();

                    bool changed = false;
                    for (size_t i = 0; i < agenda_.size(); ++i) {
                        ImGui::TableNextRow();
                        ImGui::PushID((int)i);
                        ImGui::TableNextColumn(); ImGui::Text("#%zu", i + 1);
                        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
                        changed |= ImGui::InputDouble("##t", &agenda_[i].tiempo_s, 0, 0, "%.1f");
                        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
                        changed |= ImGui::InputDouble("##f", &agenda_[i].feed_kg_h, 0, 0, "%.0f");
                        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
                        changed |= ImGui::InputDouble("##v1", &agenda_[i].valvula1_pct, 0, 0, "%.1f");
                        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
                        changed |= ImGui::InputDouble("##v2", &agenda_[i].valvula2_pct, 0, 0, "%.1f");
                        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
                        changed |= ImGui::InputDouble("##v3", &agenda_[i].valvula3_pct, 0, 0, "%.1f");
                        ImGui::PopID();
                    }
                    ImGui::EndTable();
                    if (changed) ejecutar_cascada_tanques();
                }
                if (ImGui::SmallButton("+ Add Schedule Event")) {
                    double next_t = agenda_.empty() ? 0.0 : agenda_.back().tiempo_s + 30.0;
                    agenda_.push_back({next_t, 36000.0, 100.0, 100.0, 100.0});
                    ejecutar_cascada_tanques();
                }
                ImGui::EndTabItem();
            }

            if (ImGui::BeginTabItem("Tanks (Container 1 -> 2 -> 3)")) {
                if (ImGui::BeginTable("TanksConfigTable", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg, ImVec2(0, 115))) {
                    ImGui::TableSetupColumn("Unit Name", ImGuiTableColumnFlags_WidthFixed, 130.0f);
                    ImGui::TableSetupColumn("Cross Area (m²)", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Max Height (m)", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Orifice Area (m²)", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Initial Level (%)", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableHeadersRow();

                    bool changed = false;
                    for (size_t i = 0; i < tanques_.size(); ++i) {
                        ImGui::TableNextRow();
                        ImGui::PushID((int)i + 500);
                        ImGui::TableNextColumn(); ImGui::TextUnformatted(tanques_[i].nombre.c_str());
                        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
                        changed |= ImGui::InputDouble("##a", &tanques_[i].area_m2, 0, 0, "%.2f");
                        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
                        changed |= ImGui::InputDouble("##h", &tanques_[i].altura_max_m, 0, 0, "%.2f");
                        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
                        changed |= ImGui::InputDouble("##o", &tanques_[i].area_orificio_m2, 0, 0, "%.4f");
                        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
                        changed |= ImGui::InputDouble("##l0", &tanques_[i].nivel_inicial_pct, 0, 0, "%.1f");
                        ImGui::PopID();
                    }
                    ImGui::EndTable();
                    if (changed) ejecutar_cascada_tanques();
                }
                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }

        // Gráficos apilados HSC-Sim: 1) Tank Level % vs Time (s) con cursor vertical, 2) Outflow (kg/h) vs Time (s)
        if (calculado_ && !resultado_.tiempos.empty()) {
            const auto& ts = resultado_.tiempos;
            const auto& l1 = resultado_.variables["Level 1 (%)"];
            const auto& l2 = resultado_.variables["Level 2 (%)"];
            const auto& l3 = resultado_.variables["Level 3 (%)"];
            const auto& qf = resultado_.variables["Feed (kg/h)"];
            const auto& q1 = resultado_.variables["Outflow 1 (kg/h)"];
            const auto& q2 = resultado_.variables["Outflow 2 (kg/h)"];
            const auto& q3 = resultado_.variables["Outflow 3 (kg/h)"];
            int npts = (int)ts.size();

            // Interpolar valores en la posición del cursor vertical
            cursor_tiempo_ = std::clamp(cursor_tiempo_, ts.front(), ts.back());
            size_t idx_c = 0;
            for (size_t i = 0; i < ts.size(); ++i) {
                if (ts[i] <= cursor_tiempo_) idx_c = i;
            }
            ImGui::TextColored(ImVec4(0.45f, 0.85f, 0.55f, 1.0f),
                "Cursor t = %.1f s  |  Levels: C1=%.1f%%, C2=%.1f%%, C3=%.1f%%  |  Outflows: C1=%.0f, C2=%.0f, C3=%.0f kg/h",
                ts[idx_c], l1[idx_c], l2[idx_c], l3[idx_c], q1[idx_c], q2[idx_c], q3[idx_c]);

            if (ImPlot::BeginPlot("Tank Level %", ImVec2(-1, 185))) {
                ImPlot::SetupAxes("Time (s)", "Tank Level (%)", ImPlotAxisFlags_AutoFit, ImPlotAxisFlags_None);
                ImPlot::SetupAxisLimits(ImAxis_Y1, 0.0, 105.0, ImGuiCond_Once);
                ImPlot::PlotLine("Container 1 Level %", ts.data(), l1.data(), npts);
                ImPlot::PlotLine("Container 2 Level %", ts.data(), l2.data(), npts);
                ImPlot::PlotLine("Container 3 Level %", ts.data(), l3.data(), npts);
                ImPlot::DragLineX(0, &cursor_tiempo_, ImVec4(0.95f, 0.80f, 0.20f, 0.95f), 1.5f);
                ImPlot::EndPlot();
            }

            if (ImPlot::BeginPlot("Outflow (kg/h)", ImVec2(-1, 185))) {
                ImPlot::SetupAxes("Time (s)", "Flowrate (kg/h)", ImPlotAxisFlags_AutoFit, ImPlotAxisFlags_AutoFit);
                ImPlot::PlotLine("Feed In", ts.data(), qf.data(), npts);
                ImPlot::PlotLine("Container 1 Outflow", ts.data(), q1.data(), npts);
                ImPlot::PlotLine("Container 2 Outflow", ts.data(), q2.data(), npts);
                ImPlot::PlotLine("Container 3 Outflow", ts.data(), q3.data(), npts);
                ImPlot::DragLineX(1, &cursor_tiempo_, ImVec4(0.95f, 0.80f, 0.20f, 0.95f), 1.5f);
                ImPlot::EndPlot();
            }
        }
        return;
    }

    if (modo_ == 1) {
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
    } else if (modo_ == 2) {
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
            ImPlot::SetupAxes("Tiempo (s)", modo_ == 2 ? "T (K)" : modo_ == 1 ? "h (m)" : "C (mol/L)");
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

void PanelReconciliacion::reset() {
    config_ = motores::ConfigBalanceMetalurgico{};
    resultado_ = motores::ResultadoBalanceMetalurgico{};
    elementos_.clear();
    fs_ = nullptr;
    modo_ = 0;
    fila_seleccionada_ = -1;
    calculado_ = false;
    cargado_inicial_ = false;
    mensaje_.clear();
}

void PanelReconciliacion::inicializar_datos(diagrama_flujo::Flowsheet* flowsheet) {
    fs_ = flowsheet;
    config_ = motores::ConfigBalanceMetalurgico{};
    elementos_ = {"Cu", "Fe", "S", "SiO2"};
    if (flowsheet != nullptr && !flowsheet->mapa_flujos().empty()) {
        for (const auto& [fid, fl] : flowsheet->mapa_flujos()) {
            if (fl.es_energia) continue;
            motores::CorrienteBalanceMetalurgico c;
            c.id = fid;
            c.nombre = fl.nombre.empty() ? ("Stream " + std::to_string(fid)) : fl.nombre;
            double sum_sol_kg_h = 0.0;
            for (const auto& [sid, v] : fl.flujo_solido) sum_sol_kg_h += v;
            c.solido_t_h = std::max(0.0, sum_sol_kg_h / 1000.0);
            c.liquido_t_h = std::max(0.0, fl.agua_kg_h / 1000.0);
            c.sd_solido_t_h = std::max(0.1, c.solido_t_h * 0.05);
            c.sd_liquido_t_h = std::max(0.1, c.liquido_t_h * 0.05);
            config_.corrientes.push_back(c);
        }
        for (const auto& n : flowsheet->nodos()) {
            if (n.ids_flujos_entrada.empty() || n.ids_flujos_salida.empty()) continue;
            motores::UnidadBalanceMetalurgico u;
            u.id = n.id;
            u.nombre = n.nombre;
            u.entradas = n.ids_flujos_entrada;
            u.salidas = n.ids_flujos_salida;
            config_.unidades.push_back(u);
        }
    }
    if (config_.corrientes.empty()) {
        motores::CorrienteBalanceMetalurgico f1{1, "Feed", 100.0, 150.0, 2.0, 5.0, true, true, false, {{"Cu", 0.012}, {"Fe", 0.045}}};
        motores::CorrienteBalanceMetalurgico f2{2, "Concentrate", 4.2, 8.0, 0.2, 0.5, true, true, false, {{"Cu", 0.265}, {"Fe", 0.280}}};
        motores::CorrienteBalanceMetalurgico f3{3, "Tailings", 95.8, 142.0, 2.0, 5.0, true, true, false, {{"Cu", 0.001}, {"Fe", 0.035}}};
        config_.corrientes = {f1, f2, f3};
        motores::UnidadBalanceMetalurgico u1{1, "Flotation Cell", {1}, {2, 3}, true};
        config_.unidades = {u1};
    }
    cargado_inicial_ = true;
}

void PanelReconciliacion::ejecutar_balance() {
    config_.modo = static_cast<motores::ModoBalanceMetalurgico>(std::clamp(modo_, 0, 2));
    motores::BalanceMetalurgico solver;
    resultado_ = solver.resolver(config_);
    calculado_ = true;
    mensaje_ = resultado_.mensaje;
}

void PanelReconciliacion::renderizar(diagrama_flujo::Flowsheet* flowsheet) {
    if (!cargado_inicial_ || fs_ != flowsheet) {
        inicializar_datos(flowsheet);
        ejecutar_balance();
    }
    const char* modos[] = {"Solids only", "Assays only", "Solids + Water"};
    ImGui::SetNextItemWidth(180.0f);
    if (ImGui::Combo("Mode", &modo_, modos, 3)) ejecutar_balance();
    ImGui::SameLine();
    if (ImGui::Button("Solve Mass Balance")) ejecutar_balance();
    ImGui::SameLine();
    if (ImGui::Button("Reload Flowsheet")) {
        inicializar_datos(flowsheet);
        ejecutar_balance();
    }
    if (!mensaje_.empty()) ImGui::TextWrapped("%s", mensaje_.c_str());
    if (ImGui::BeginTable("reconcil_tbl", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Stream");
        ImGui::TableSetupColumn("Measured Solids (t/h)");
        ImGui::TableSetupColumn("SD Solids");
        ImGui::TableSetupColumn("Reconciled Solids (t/h)");
        ImGui::TableSetupColumn("Measured Water (t/h)");
        ImGui::TableSetupColumn("Reconciled Water (t/h)");
        ImGui::TableHeadersRow();
        for (size_t i = 0; i < config_.corrientes.size(); ++i) {
            auto& c = config_.corrientes[i];
            ImGui::TableNextRow();
            ImGui::PushID(static_cast<int>(i));
            ImGui::TableNextColumn(); ImGui::Text("%s", c.nombre.c_str());
            ImGui::TableNextColumn(); ImGui::InputDouble("##ms", &c.solido_t_h, 0, 0, "%.2f");
            ImGui::TableNextColumn(); ImGui::InputDouble("##sds", &c.sd_solido_t_h, 0, 0, "%.2f");
            ImGui::TableNextColumn();
            ImGui::Text("%.2f", resultado_.solido_reconciliado_t_h.count(c.id) ? resultado_.solido_reconciliado_t_h.at(c.id) : c.solido_t_h);
            ImGui::TableNextColumn(); ImGui::InputDouble("##mw", &c.liquido_t_h, 0, 0, "%.2f");
            ImGui::TableNextColumn();
            ImGui::Text("%.2f", resultado_.liquido_reconciliado_t_h.count(c.id) ? resultado_.liquido_reconciliado_t_h.at(c.id) : c.liquido_t_h);
            ImGui::PopID();
        }
        ImGui::EndTable();
    }
}

}} // namespace
