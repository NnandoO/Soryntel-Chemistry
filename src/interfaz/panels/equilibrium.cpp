#include "shared.hpp"
#include "application/SimulationService.hpp"
#include "domain/ThermoSnapshot.hpp"
namespace cma { namespace interfaz {
// Panel de Equilibrio de Fases
// ═══════════════════════════════════════════════════════════════════════════

// Devuelve los símbolos de elementos presentes en una fórmula (con sufijos de fase)
static std::set<std::string> elementos_de_formula(const std::string& formula) {
    auto& tp = nucleo::TablaPeriodica::instancia();
    std::string limpia;
    for (size_t k = 0; k < formula.size(); ++k) {
        if (formula[k] == '(') {                       // quitar sufijos "(g)", "(-a)", etc.
            size_t close = formula.find(')', k);
            if (close != std::string::npos) { k = close; continue; }
        }
        limpia += formula[k];
    }
    std::set<std::string> out;
    for (auto& a : tp.analizar_formula(limpia)) out.insert(a.elem);
    return out;
}

static int fase_por_defecto(const nucleo::Especie& sp) {
    if (sp.fase_std == nucleo::Fase::Gas || sp.fase_std == nucleo::Fase::Mixto) return 1;
    // Iones acuosos o agua líquida -> Fase 3 (Acuosa), sólidos/fundidos -> Fase 2
    if (sp.formula.find("a)") != std::string::npos || sp.fase_std == nucleo::Fase::Liquido) return 3;
    return 2;
}

static ImVec4 color_hsc_especie(const std::string& formula, size_t idx) {
    // Colores característicos de la captura HSC NA2SO4.gem9
    if (formula == "N2(g)" || formula == "N2")       return ImVec4(0.38f, 0.72f, 0.16f, 0.90f); // verde lima
    if (formula == "CO(g)" || formula == "CO")       return ImVec4(0.00f, 0.65f, 0.93f, 0.90f); // azul celeste
    if (formula == "CO2(g)" || formula == "CO2")     return ImVec4(0.35f, 0.35f, 0.35f, 0.90f); // gris carbón
    if (formula == "COS(g)" || formula == "COS")     return ImVec4(0.69f, 0.65f, 0.35f, 0.90f); // oliva
    if (formula == "O2(g)" || formula == "O2")       return ImVec4(0.95f, 0.58f, 0.05f, 0.90f); // naranja
    if (formula == "SO2(g)" || formula == "SO2")     return ImVec4(0.84f, 0.84f, 0.08f, 0.90f); // amarillo vivo
    if (formula == "Na2CO3" || formula == "Na2CO3(s)") return ImVec4(0.92f, 0.38f, 0.05f, 0.90f); // naranja rojizo
    if (formula == "Na2S" || formula == "Na2S(s)")   return ImVec4(0.88f, 0.32f, 0.08f, 0.90f); // terracota
    if (formula == "Na2SO4" || formula == "Na2SO4(s)") return ImVec4(0.32f, 0.45f, 0.88f, 0.90f); // azul cobalto
    if (formula == "C" || formula == "C(s)")         return ImVec4(0.75f, 0.78f, 0.12f, 0.90f); // amarillo verdoso

    static const ImVec4 palette[] = {
        ImVec4(0.20f, 0.63f, 0.85f, 0.88f),
        ImVec4(0.90f, 0.42f, 0.15f, 0.88f),
        ImVec4(0.30f, 0.72f, 0.25f, 0.88f),
        ImVec4(0.82f, 0.20f, 0.52f, 0.88f),
        ImVec4(0.55f, 0.36f, 0.82f, 0.88f),
        ImVec4(0.85f, 0.75f, 0.12f, 0.88f),
        ImVec4(0.15f, 0.75f, 0.65f, 0.88f),
        ImVec4(0.85f, 0.25f, 0.25f, 0.88f),
        ImVec4(0.45f, 0.55f, 0.88f, 0.88f),
        ImVec4(0.65f, 0.48f, 0.25f, 0.88f),
    };
    return palette[idx % (sizeof(palette) / sizeof(palette[0]))];
}

void PanelEquilibrioFases::generar_especies_por_elementos(const std::map<int64_t, nucleo::Especie>& db) {
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
                    species_list_.push_back({sid, db_sp.nombre, db_sp.formula,
                        std::string(cma::nucleo::nombre_fase(db_sp.fase_std)), 0.0,
                        fase_por_defecto(db_sp), 1.0});
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
                    species_list_.push_back({sid, db_sp.nombre, db_sp.formula,
                        std::string(cma::nucleo::nombre_fase(db_sp.fase_std)), 0.0, 3, 1.0});
                    break;
                }
            }
        }
    }
}

void PanelEquilibrioFases::cargar_ejemplo_na2so4(const std::map<int64_t, nucleo::Especie>& db) {
    // Replica el ejemplo NA2SO4.gem9 de HSC: sistema Na-S-O-C-N, alimentación
    // Na2SO4 (1 kmol) + C (4 kmol) + N2 (1 kmol), barrido 0-1000 °C × 1-10 bar.
    species_list_.clear();
    strncpy(elementos_buf_, "Na, S, O, C, N", sizeof(elementos_buf_) - 1);
    elementos_buf_[sizeof(elementos_buf_) - 1] = '\0';
    todos_elementos_ = false;
    modo_celda_ = false;
    var_x_ = VarBarrido::Temperatura;
    var_y_ = VarBarrido::Presion;

    auto agregar_feed = [&](const char* formula, double kmol) {
        const auto* esp = buscar_especie_por_nombre_o_formula(formula);
        if (esp) species_list_.push_back({esp->id, esp->nombre, esp->formula,
            std::string(cma::nucleo::nombre_fase(esp->fase_std)), kmol,
            fase_por_defecto(*esp), 1.0});
    };
    agregar_feed("Na2SO4", 1.0);
    agregar_feed("C", 4.0);
    agregar_feed("N2", 1.0);

    // Generar el resto de especies candidatas del sistema
    generar_especies_por_elementos(db);

    // Rangos de barrido como la imagen HSC
    t_start_ = 0.0;  t_end_ = 1000.0;  t_steps_ = 21;   // 0..1000 °C cada 50
    p_start_ = 1.0;  p_end_ = 10.0;    p_steps_ = 10;   // 1..10 bar
    strncpy(titulo_chart_, "NaSO4", sizeof(titulo_chart_) - 1);
    titulo_chart_[sizeof(titulo_chart_) - 1] = '\0';
    filtro_fase_ = -1;
    error_msg_.clear();
    calculado_ = false;
}

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
    ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.58f);

    // --- Left Pane: Species List ---
    ImGui::Text(tr("System Species"));
    if (ImGui::BeginTable("eq_species_table", 6,
        ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable,
        ImVec2(0, ImGui::GetContentRegionAvail().y - 165)))
    {
        ImGui::TableSetupColumn("Phase", ImGuiTableColumnFlags_WidthFixed, 48.0f);
        ImGui::TableSetupColumn("Fórmula", ImGuiTableColumnFlags_WidthFixed, 95.0f);
        ImGui::TableSetupColumn("Nombre", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Amount (kmol)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
        ImGui::TableSetupColumn("Act. Coeff (γ)", ImGuiTableColumnFlags_WidthFixed, 88.0f);
        ImGui::TableSetupColumn("X", ImGuiTableColumnFlags_WidthFixed, 26.0f);
        ImGui::TableHeadersRow();

        for (size_t i = 0; i < species_list_.size(); ++i) {
            auto& sp = species_list_[i];
            ImGui::TableNextRow();
            ImGui::PushID((int)i);

            ImGui::TableNextColumn();
            ImGui::SetNextItemWidth(-FLT_MIN);
            if (ImGui::InputInt("##ph", &sp.phase_num, 0, 0)) {
                sp.phase_num = std::clamp(sp.phase_num, 1, 10);
            }

            ImGui::TableNextColumn();
            ImGui::TextUnformatted(sp.formula.c_str());

            ImGui::TableNextColumn();
            ImGui::TextUnformatted(sp.name.c_str());

            ImGui::TableNextColumn();
            ImGui::SetNextItemWidth(-FLT_MIN);
            if (ImGui::InputDouble("##amt", &sp.initial_amount, 0.0, 0.0, "%.4f")) {
                if (sp.initial_amount < 0.0) sp.initial_amount = 0.0;
            }

            ImGui::TableNextColumn();
            if (modelo_actividad_ == 0) ImGui::BeginDisabled(true);
            ImGui::SetNextItemWidth(-FLT_MIN);
            if (ImGui::InputDouble("##gam", &sp.activity_coeff, 0.0, 0.0, "%.4g")) {
                if (sp.activity_coeff <= 0.0) sp.activity_coeff = 1.0;
            }
            if (modelo_actividad_ == 0) ImGui::EndDisabled();

            ImGui::TableNextColumn();
            if (ImGui::SmallButton("x")) {
                species_list_.erase(species_list_.begin() + (ptrdiff_t)i);
                ImGui::PopID();
                break;
            }

            ImGui::PopID();
        }
        ImGui::EndTable();
    }

    // ── Entrada rápida por fórmula (HSC: "typing the formulae") ──
    ImGui::SetNextItemWidth(160.0f);
    bool enter_formula = ImGui::InputTextWithHint("##add_formula_gem", "Fórmula (ej. FeS2, Cl2(g))", add_species_buf_, sizeof(add_species_buf_), ImGuiInputTextFlags_EnterReturnsTrue);
    ImGui::SameLine();
    ImGui::SetNextItemWidth(90.0f);
    ImGui::InputDouble("##add_amt_gem", &add_species_amt_, 0.0, 0.0, "%.3f");
    ImGui::SameLine();
    if ((ImGui::Button(tr("+ Agregar fórmula")) || enter_formula) && add_species_buf_[0] != '\0') {
        const auto* esp = buscar_especie_por_nombre_o_formula(add_species_buf_);
        if (esp) {
            bool exists = false;
            for (auto& s : species_list_) {
                if (s.id == esp->id) { s.initial_amount = std::max(0.0, add_species_amt_); exists = true; break; }
            }
            if (!exists) {
                species_list_.push_back({esp->id, esp->nombre, esp->formula,
                    std::string(cma::nucleo::nombre_fase(esp->fase_std)),
                    std::max(0.0, add_species_amt_), fase_por_defecto(*esp), 1.0});
            }
            add_species_buf_[0] = '\0';
            error_msg_.clear();
        } else {
            error_msg_ = std::string("Especie no encontrada en la BD: ") + add_species_buf_;
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Examinar BD##fases"), ImVec2(-1, 0))) {
        ModalBuscadorDB::instancia().abrir("Buscar Especie (Fases)");
    }
    
    std::vector<const nucleo::Especie*> sel_fases;
    if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Fases)", sel_fases, true)) {
        for (auto* sp : sel_fases) {
            bool exists = false;
            for (auto& s : species_list_) if (s.id == sp->id) exists = true;
            if (!exists) {
                species_list_.push_back({sp->id, sp->nombre, sp->formula,
                    std::string(cma::nucleo::nombre_fase(sp->fase_std)), 0.0,
                    fase_por_defecto(*sp), 1.0});
            }
        }
    }

    // ── Selección por elementos (como HSC: "select the elements of the system") ──
    ImGui::Text(tr("Elements (e.g. Na, S, O, C, N):"));
    ImGui::SetNextItemWidth(ImGui::GetContentRegionAvail().x * 0.55f);
    ImGui::InputTextWithHint("##elements_in", "Na, S, O, C, N...", elementos_buf_, sizeof(elementos_buf_));
    ImGui::SameLine();
    if (ImGui::Button(tr("Generate Species"), ImVec2(-1, 0))) {
        generar_especies_por_elementos(db);
    }

    ImGui::Checkbox(tr("All elements must exist"), &todos_elementos_);
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Only species containing ALL the selected elements");
    ImGui::SameLine();
    ImGui::Checkbox(tr("Cell calculation"), &modo_celda_);
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Electrochemical cell: adds e- and charge balance");
    ImGui::SameLine();
    if (ImGui::Button(tr("Ejemplo Na2SO4 (HSC)"), ImVec2(-1, 0))) {
        cargar_ejemplo_na2so4(db);
    }
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Carga el sistema Na-S-O-C-N con alimentación Na2SO4 + 4 C + N2 y barrido T 0-1000 °C × P 1-10 bar");

    // ── Guardar / cargar composición del sistema (archivo .gem) ──
    if (ImGui::Button(tr("Save Species File"), ImVec2(ImGui::GetContentRegionAvail().x * 0.5f - 4, 0))) {
        adapters::DesktopFileDialog dlg;
        std::string path = dlg.select("Save GEM System", "*.gem", true);
        if (!path.empty()) {
            if (path.find(".gem") == std::string::npos) path += ".gem";
            FILE* fp = fopen(path.c_str(), "w");
            if (fp) {
                fprintf(fp, "# GEM species file: formula;amount_kmol;phase_num;activity_coeff\n");
                for (auto& s : species_list_)
                    fprintf(fp, "%s;%.6g;%d;%.6g\n", s.formula.c_str(), s.initial_amount, s.phase_num, s.activity_coeff);
                fclose(fp);
            }
            std::string base = std::filesystem::path(path).stem().string();
            strncpy(titulo_chart_, base.c_str(), sizeof(titulo_chart_) - 1);
            titulo_chart_[sizeof(titulo_chart_) - 1] = '\0';
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Load Species File"), ImVec2(-1, 0))) {
        adapters::DesktopFileDialog dlg;
        std::string path = dlg.select("Load GEM System", "*.gem", false);
        if (!path.empty()) {
            FILE* fp = fopen(path.c_str(), "r");
            if (fp) {
                species_list_.clear();
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
                    int ph = fase_por_defecto(*esp);
                    if (parts.size() > 2) { try { ph = std::clamp(std::stoi(trim_str(parts[2])), 1, 10); } catch (...) {} }
                    double gam = 1.0;
                    if (parts.size() > 3) { try { gam = std::max(1e-12, std::stod(trim_str(parts[3]))); } catch (...) {} }
                    bool exists = false;
                    for (auto& s : species_list_)
                        if (s.id == esp->id) { exists = true; s.initial_amount = amt; s.phase_num = ph; s.activity_coeff = gam; break; }
                    if (!exists) {
                        species_list_.push_back({esp->id, esp->nombre, esp->formula,
                            std::string(cma::nucleo::nombre_fase(esp->fase_std)), amt, ph, gam});
                    }
                }
                fclose(fp);
                std::string base = std::filesystem::path(path).stem().string();
                strncpy(titulo_chart_, base.c_str(), sizeof(titulo_chart_) - 1);
                titulo_chart_[sizeof(titulo_chart_) - 1] = '\0';
            }
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
            VarBarrido vx = var_x_, vy = var_y_;
            double ts = t_start_, te = t_end_, ps = p_start_, pe = p_end_;
            double fs = feed_start_, fe = feed_end_;
            int tsteps = t_steps_, psteps = p_steps_, fsteps = feed_steps_;
            int esp_sw = especie_barrida_;
            bool celda = modo_celda_;
            int mod_act = modelo_actividad_;

            // Conteo de puntos por eje según la variable barrida
            int nx = (vx == VarBarrido::CantidadEspecie) ? fsteps : (vx == VarBarrido::Temperatura ? tsteps : psteps);
            int ny = (vy == VarBarrido::CantidadEspecie) ? fsteps : (vy == VarBarrido::Temperatura ? tsteps : psteps);

            calc_thread_ = std::thread([this, species_copy, vx, vy, ts, te, ps, pe, fs, fe, nx, ny, esp_sw, celda, mod_act, db]() {
                try {
                domain::ThermoSnapshot snapshot(db);
                application::SimulationService simulation(snapshot);

                auto rango = [vx, vy, ts, te, ps, pe, fs, fe](VarBarrido v) {
                    if (v == VarBarrido::Temperatura) return std::make_pair(ts, te);
                    if (v == VarBarrido::Presion)     return std::make_pair(ps, pe);
                    return std::make_pair(fs, fe);  // CantidadEspecie
                };
                auto bx = rango(vx), by = rango(vy);
                double dx = (nx > 1) ? (bx.second - bx.first) / (nx - 1) : 0.0;
                double dy = (ny > 1) ? (by.second - by.first) / (ny - 1) : 0.0;

                std::vector<double> local_x(nx * ny);   // valor del eje X
                std::vector<double> local_y(nx * ny);   // valor del eje Y
                std::vector<double> local_g(nx * ny, 0.0);
                std::map<int64_t, std::vector<double>> local_z;

                std::vector<int64_t> candidates;
                std::map<int64_t, int> species_phases;
                std::map<int64_t, double> base_gammas;
                for (auto& sp : species_copy) {
                    local_z[sp.id].resize(nx * ny, 0.0);
                    candidates.push_back(sp.id);
                    species_phases[sp.id] = sp.phase_num;
                    base_gammas[sp.id] = (mod_act == 0) ? 1.0 : std::max(1e-12, sp.activity_coeff);
                }

                std::map<int64_t, double> warm;  // warm start entre puntos del barrido
                std::string local_error;
                int total_steps = ny * nx;
                int current_step = 0;
                int no_conv = 0;

                for (int i = 0; i < ny; ++i) {          // recorrido del eje Y
                    double vY = by.first + i * dy;
                    for (int j = 0; j < nx; ++j) {      // recorrido del eje X
                        if (calc_cancelled_) {
                            temp_error_msg_ = "Cálculo cancelado";
                            calculation_done_ = true;
                            return;
                        }
                        double vX = bx.first + j * dx;
                        int idx = i * nx + j;

                        // Asignar T, P y cantidad desde los ejes
                        double T_C = t_fijo_, P_bar = p_fijo_, amt_override = -1.0;
                        if (vx == VarBarrido::Temperatura) T_C = vX;
                        if (vy == VarBarrido::Temperatura) T_C = vY;
                        if (vx == VarBarrido::Presion)     P_bar = vX;
                        if (vy == VarBarrido::Presion)     P_bar = vY;
                        if (vx == VarBarrido::CantidadEspecie) amt_override = vX;
                        if (vy == VarBarrido::CantidadEspecie) amt_override = vY;

                        double T_K = std::max(T_C + 273.15, 200.0);
                        double P_atm = P_bar * 0.986923; // bar -> atm

                        local_x[idx] = vX;
                        local_y[idx] = vY;

                        std::map<int64_t, double> alimentacion;
                        for (auto& sp : species_copy) {
                            alimentacion[sp.id] = sp.initial_amount;
                        }
                        if (amt_override >= 0.0 && esp_sw >= 0 && esp_sw < (int)species_copy.size()) {
                            alimentacion[species_copy[esp_sw].id] = amt_override;
                        }

                        // Modelo predefinido de actividad dependiente de T: ln(γ(T)) = (298.15/T) * ln(γ_298)
                        std::map<int64_t, double> gammas_T = base_gammas;
                        if (mod_act == 2) {
                            for (auto& [sid, g0] : gammas_T) {
                                if (std::abs(g0 - 1.0) > 1e-6) {
                                    gammas_T[sid] = std::exp((298.15 / T_K) * std::log(g0));
                                }
                            }
                        }

                        auto res = simulation.run_equilibrium(alimentacion, T_K, P_atm, candidates, &warm, celda, gammas_T, species_phases);
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
                size_t color_idx = 0;
                for (auto& sp : species_copy) {
                    const auto& z_data = local_z[sp.id];
                    double max_amt = 0.0;
                    for (double a : z_data) max_amt = std::max(max_amt, a);
                    if (max_amt > 1e-6) {
                        int fnum = (sp.phase == "Gas") ? 1 : std::max(2, sp.phase_num);
                        ImVec4 col = color_hsc_especie(sp.formula, color_idx++);
                        local_surfaces.push_back({sp.id, sp.formula, sp.phase, fnum, z_data, true, col});
                    }
                }
                // Ordenar por fase (Fase 1 primero, luego Fase 2, etc.) igual que en HSC
                std::stable_sort(local_surfaces.begin(), local_surfaces.end(),
                    [](const ChartSurface& a, const ChartSurface& b) {
                        if (a.fase_num != b.fase_num) return a.fase_num < b.fase_num;
                        return a.legend < b.legend;
                    });

                temp_x_values_ = std::move(local_x);
                temp_y_values_ = std::move(local_y);
                temp_z_values_ = std::move(local_z);
                temp_g_values_ = std::move(local_g);
                temp_chart_surfaces_ = std::move(local_surfaces);
                temp_error_msg_ = local_error;
                temp_plot_t_steps_ = nx;
                temp_plot_p_steps_ = ny;

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
    ImGui::SetNextItemWidth(180);
    ImGui::InputText("##gem_title", titulo_chart_, sizeof(titulo_chart_));
    ImGui::SameLine();
    ImGui::Text(tr("Activity Model:"));
    ImGui::SameLine();
    ImGui::SetNextItemWidth(-1);
    const char* act_models[] = {
        "Ideal (γ = 1.0)",
        "Constant γ (Manual per species)",
        "Regular Solution γ(T) = exp((298/T)·ln γ₀)"
    };
    ImGui::Combo("##act_model_gem", &modelo_actividad_, act_models, 3);

    ImGui::Spacing();
    ImGui::Text(tr("System State"));
    ImGui::Separator();

    // ── Selectores de variable de barrido (HSC: "repeat over raw material amounts or temperatures") ──
    {
        const char* var_nombres[3] = {"Temperatura (°C)", "Presión (bar)", "Cantidad de especie (kmol)"};
        auto combo_var = [&](const char* id, VarBarrido& v) {
            int cur = (v == VarBarrido::Temperatura) ? 0 : (v == VarBarrido::Presion) ? 1 : 2;
            if (ImGui::Combo(id, &cur, var_nombres, 3))
                v = (cur == 0) ? VarBarrido::Temperatura : (cur == 1) ? VarBarrido::Presion : VarBarrido::CantidadEspecie;
        };
        ImGui::Text(tr("Eje X:")); ImGui::SameLine();
        ImGui::SetNextItemWidth(185);
        combo_var("##var_x_gem", var_x_);
        ImGui::SameLine();
        ImGui::Text(tr("Eje Y:")); ImGui::SameLine();
        ImGui::SetNextItemWidth(185);
        combo_var("##var_y_gem", var_y_);
        if (var_x_ == var_y_) var_y_ = (var_x_ == VarBarrido::Temperatura) ? VarBarrido::Presion : VarBarrido::Temperatura;

        if (var_x_ == VarBarrido::CantidadEspecie || var_y_ == VarBarrido::CantidadEspecie) {
            ImGui::Text(tr("Especie barrida:")); ImGui::SameLine();
            ImGui::SetNextItemWidth(200);
            if (especie_barrida_ >= (int)species_list_.size()) especie_barrida_ = 0;
            std::string current = species_list_.empty() ? "" : species_list_[especie_barrida_].formula;
            if (ImGui::BeginCombo("##especie_sw_gem", current.c_str())) {
                for (size_t i = 0; i < species_list_.size(); ++i) {
                    bool sel = (int)i == especie_barrida_;
                    if (ImGui::Selectable(species_list_[i].formula.c_str(), sel)) especie_barrida_ = (int)i;
                    if (sel) ImGui::SetItemDefaultFocus();
                }
                ImGui::EndCombo();
            }
        }
    }

    // La tabla muestra la fila activa (variable barrida) y grises las que se mantienen fijas.
    if (ImGui::BeginTable("eq_sweep_params", 4, ImGuiTableFlags_BordersInnerH | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Variable", ImGuiTableColumnFlags_WidthFixed, 160.0f);
        ImGui::TableSetupColumn("Initial State", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Final State", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Number of Steps", ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableHeadersRow();

        bool barre_t = (var_x_ == VarBarrido::Temperatura || var_y_ == VarBarrido::Temperatura);
        bool barre_p = (var_x_ == VarBarrido::Presion    || var_y_ == VarBarrido::Presion);
        bool barre_c = (var_x_ == VarBarrido::CantidadEspecie || var_y_ == VarBarrido::CantidadEspecie);

        // Fila Temperatura
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text("%s%s", tr("Temperature (°C)"), barre_t ? "" : " (fija)");
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Ti", &t_start_);
        if (!barre_t) { t_fijo_ = t_start_; ImGui::BeginDisabled(true); }
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Tf", &t_end_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputInt("##Ts", &t_steps_);
        if (t_steps_ < 2) t_steps_ = 2;
        if (!barre_t) ImGui::EndDisabled();

        // Fila Presión
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text("%s%s", tr("Pressure (bar)"), barre_p ? "" : " (fija)");
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Pi", &p_start_);
        if (!barre_p) { p_fijo_ = p_start_; ImGui::BeginDisabled(true); }
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Pf", &p_end_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputInt("##Ps", &p_steps_);
        if (p_steps_ < 2) p_steps_ = 2;
        if (!barre_p) ImGui::EndDisabled();

        // Fila Cantidad de especie (siempre visible; activa solo cuando se barre)
        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        ImGui::Text("%s%s", tr("Cantidad (kmol)"), barre_c ? "" : " (fija)");
        if (!barre_c) ImGui::BeginDisabled(true);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Ci", &feed_start_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Cf", &feed_end_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputInt("##Cs", &feed_steps_);
        if (feed_steps_ < 2) feed_steps_ = 2;
        if (!barre_c) ImGui::EndDisabled();

        ImGui::EndTable();
    }

    if (!error_msg_.empty()) {
        ImGui::Spacing();
        ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.4f, 0.4f, 1.0f), "%s", error_msg_.c_str());
    }

    ImGui::Columns(1);
    
    if (show_chart_window_) {
        ImGui::SetNextWindowSize(ImVec2(1050, 680), ImGuiCond_FirstUseEver);
        std::string wintitle = std::string("Equilibrium Calculations - [Chart (C:\\HSC9\\Gibbs\\") + titulo_chart_ + ".gem9)]###gem_chart";
        if (ImGui::Begin(wintitle.c_str(), &show_chart_window_, ImGuiWindowFlags_MenuBar)) {
            // ── Barra de menús estilo HSC ──
            if (ImGui::BeginMenuBar()) {
                if (ImGui::BeginMenu(tr("File"))) {
                    if (ImGui::MenuItem(tr("Exportar tabla (.txt)"))) show_table_window_ = true;
                    ImGui::Separator();
                    if (ImGui::MenuItem(tr("Cerrar"))) show_chart_window_ = false;
                    ImGui::EndMenu();
                }
                if (ImGui::BeginMenu(tr("Edit"))) {
                    if (ImGui::MenuItem(tr("Copiar tabla al portapapeles"))) show_table_window_ = true;
                    ImGui::EndMenu();
                }
                if (ImGui::BeginMenu(tr("Format"))) {
                    ImGui::MenuItem(tr("Eje X logarítmico"), nullptr, &eje_x_log_);
                    ImGui::MenuItem(tr("Eje Y logarítmico"), nullptr, &eje_y_log_);
                    ImGui::MenuItem(tr("Eje Z logarítmico"), nullptr, &eje_z_log_);
                    ImGui::Separator();
                    if (ImGui::MenuItem(tr("Restablecer límites manuales"))) {
                        eje_x_manual_ = eje_y_manual_ = eje_z_manual_ = false;
                    }
                    ImGui::EndMenu();
                }
                if (ImGui::BeginMenu(tr("Show"))) {
                    ImGui::MenuItem(tr("Chart parameters"), nullptr, &mostrar_parametros_);
                    if (ImGui::MenuItem(tr("Vista 3D (Superficies)"), nullptr, modo_vista_chart_ == 0)) modo_vista_chart_ = 0;
                    if (ImGui::MenuItem(tr("Vista 2D (Curvas de Equilibrio)"), nullptr, modo_vista_chart_ == 1)) modo_vista_chart_ = 1;
                    ImGui::Separator();
                    if (ImGui::MenuItem(tr("Tabla de datos"), nullptr, &show_table_window_)) {}
                    ImGui::EndMenu();
                }
                if (ImGui::BeginMenu(tr("Window"))) {
                    if (ImGui::MenuItem(tr("Abrir tabla"))) show_table_window_ = true;
                    if (ImGui::MenuItem(tr("Abrir gráfico"))) show_chart_window_ = true;
                    ImGui::EndMenu();
                }
                if (ImGui::BeginMenu(tr("Help"))) {
                    if (ImGui::MenuItem(tr("Acerca del módulo GEM"))) mostrar_acerca_ = true;
                    ImGui::EndMenu();
                }
                ImGui::EndMenuBar();
            }

            // ── Toolbar estilo HSC (flecha de retroceso + selector 3D / 2D + Table) ──
            ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.16f, 0.39f, 0.63f, 1.0f));
            if (ImGui::ArrowButton("##gem_back", ImGuiDir_Left)) show_chart_window_ = false;
            ImGui::PopStyleColor();
            if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Volver a datos de entrada"));
            ImGui::SameLine();
            if (ImGui::RadioButton("3D Surface (T × P)", modo_vista_chart_ == 0)) modo_vista_chart_ = 0;
            ImGui::SameLine();
            if (ImGui::RadioButton("2D Diagram", modo_vista_chart_ == 1)) modo_vista_chart_ = 1;
            ImGui::SameLine();
            if (ImGui::Button(tr("Table##toolbar_btn"))) show_table_window_ = true;

            if (!error_msg_.empty()) {
                ImGui::SameLine();
                ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.4f, 0.4f, 1.0f), "Warning: %s", error_msg_.c_str());
            }
            if (ImGui::BeginTable("ChartLayout", 2, ImGuiTableFlags_Resizable, ImVec2(-1, -1))) {
                ImGui::TableSetupColumn("PlotCol", ImGuiTableColumnFlags_WidthStretch, 0.68f);
                ImGui::TableSetupColumn("LegendCol", ImGuiTableColumnFlags_WidthFixed, 340.0f);
                ImGui::TableNextRow();
                ImGui::TableNextColumn();

                if (calculado_ && !x_values_.empty()) {
                    const char* lbl_x = (var_x_ == VarBarrido::Temperatura) ? "Temperature, °C"
                                      : (var_x_ == VarBarrido::Presion) ? "Pressure, bar"
                                      : "Feed amount, kmol";
                    const char* lbl_y = (var_y_ == VarBarrido::Temperatura) ? "Temperature, °C"
                                      : (var_y_ == VarBarrido::Presion) ? "Pressure, bar"
                                      : "Feed amount, kmol";

                    if (modo_vista_chart_ == 0) {
                        // ── Gráfico 3D (ImPlot3D) ──
                        if (ImPlot3D::BeginPlot(titulo_chart_[0] ? titulo_chart_ : "Equilibrium", ImVec2(-1, -1), ImPlot3DFlags_NoLegend)) {
                            ImPlot3D::SetupAxes(lbl_x, lbl_y, "Equilibrium Amount, kmol");

                            ImPlot3D::SetupAxisScale(ImAxis3D_X, eje_x_log_ ? ImPlot3DScale_Log10 : ImPlot3DScale_Linear);
                            ImPlot3D::SetupAxisScale(ImAxis3D_Y, eje_y_log_ ? ImPlot3DScale_Log10 : ImPlot3DScale_Linear);
                            ImPlot3D::SetupAxisScale(ImAxis3D_Z, eje_z_log_ ? ImPlot3DScale_Log10 : ImPlot3DScale_Linear);

                            if (eje_x_manual_) ImPlot3D::SetupAxisLimits(ImAxis3D_X, x_min_user_, x_max_user_, ImPlot3DCond_Always);
                            if (eje_y_manual_) ImPlot3D::SetupAxisLimits(ImAxis3D_Y, y_min_user_, y_max_user_, ImPlot3DCond_Always);
                            if (eje_z_manual_) ImPlot3D::SetupAxisLimits(ImAxis3D_Z, z_min_user_, z_max_user_, ImPlot3DCond_Always);

                            // Preparar vectores sanitizados si hay escala logarítmica para evitar log10(<=0)
                            const std::vector<double>* px = &x_values_;
                            const std::vector<double>* py = &y_values_;
                            std::vector<double> x_safe, y_safe;
                            if (eje_x_log_) {
                                x_safe = x_values_;
                                for (double& v : x_safe) v = std::max(v, 1e-3);
                                px = &x_safe;
                            }
                            if (eje_y_log_) {
                                y_safe = y_values_;
                                for (double& v : y_safe) v = std::max(v, 1e-3);
                                py = &y_safe;
                            }

                            for (auto& surf : chart_surfaces_) {
                                if (!surf.visible || (filtro_fase_ > 0 && surf.fase_num != filtro_fase_)) continue;
                                ImPlot3DSpec spec;
                                spec.FillColor = surf.color;
                                spec.LineColor = ImVec4(0.12f, 0.12f, 0.12f, 0.85f); // malla negra como HSC
                                spec.FillAlpha = 0.88f;
                                if (eje_z_log_) {
                                    std::vector<double> z_safe = surf.z_data;
                                    for (double& v : z_safe) v = std::max(v, 1e-10);
                                    ImPlot3D::PlotSurface(surf.legend.c_str(), px->data(), py->data(), z_safe.data(), plot_t_steps_, plot_p_steps_, 0.0, 0.0, spec);
                                } else {
                                    ImPlot3D::PlotSurface(surf.legend.c_str(), px->data(), py->data(), surf.z_data.data(), plot_t_steps_, plot_p_steps_, 0.0, 0.0, spec);
                                }
                            }
                            ImPlot3D::EndPlot();
                        }
                    } else {
                        // ── Gráfico 2D (ImPlot) — Corte a Y constante ──
                        if (plot_p_steps_ > 1) {
                            ImGui::SetNextItemWidth(220);
                            ImGui::SliderInt("##slice_2d", &table_p_index_, 0, plot_p_steps_ - 1);
                            ImGui::SameLine();
                            int p_idx = std::clamp(table_p_index_, 0, plot_p_steps_ - 1);
                            ImGui::Text("%s = %.4g", lbl_y, y_values_[p_idx * plot_t_steps_]);
                        }
                        int p_idx = std::clamp(table_p_index_, 0, plot_p_steps_ > 0 ? plot_p_steps_ - 1 : 0);
                        if (ImPlot::BeginPlot(titulo_chart_[0] ? titulo_chart_ : "Equilibrium 2D", ImVec2(-1, -1))) {
                            ImPlot::SetupAxes(lbl_x, "Equilibrium Amount, kmol");
                            if (eje_x_log_) ImPlot::SetupAxisScale(ImAxis_X1, ImPlotScale_Log10);
                            if (eje_z_log_) ImPlot::SetupAxisScale(ImAxis_Y1, ImPlotScale_Log10);
                            if (eje_x_manual_) ImPlot::SetupAxisLimits(ImAxis_X1, x_min_user_, x_max_user_, ImPlotCond_Always);
                            if (eje_z_manual_) ImPlot::SetupAxisLimits(ImAxis_Y1, z_min_user_, z_max_user_, ImPlotCond_Always);

                            std::vector<double> xs_slice(plot_t_steps_);
                            for (int j = 0; j < plot_t_steps_; ++j) {
                                double vx_val = x_values_[p_idx * plot_t_steps_ + j];
                                xs_slice[j] = eje_x_log_ ? std::max(vx_val, 1e-3) : vx_val;
                            }

                            for (auto& surf : chart_surfaces_) {
                                if (!surf.visible || (filtro_fase_ > 0 && surf.fase_num != filtro_fase_)) continue;
                                std::vector<double> zs_slice(plot_t_steps_);
                                for (int j = 0; j < plot_t_steps_; ++j) {
                                    double vz_val = surf.z_data[p_idx * plot_t_steps_ + j];
                                    zs_slice[j] = eje_z_log_ ? std::max(vz_val, 1e-10) : vz_val;
                                }
                                ImPlot::SetNextLineStyle(surf.color, 2.2f);
                                ImPlot::PlotLine(surf.legend.c_str(), xs_slice.data(), zs_slice.data(), plot_t_steps_);
                            }
                            ImPlot::EndPlot();
                        }
                    }
                } else {
                    ImGui::Text(tr("No data to display. Please calculate first."));
                }

                ImGui::TableNextColumn();

                // ══ Chart parameters (estilo HSC) — se oculta desde menú Show ══
                if (mostrar_parametros_) {
                ImGui::Text(tr("Chart parameters"));
                ImGui::Separator();

                // Leyenda estilo HSC: Checkbox | Color | Description | Phase | Lista de elementos a la derecha
                std::vector<std::string> elems_set;
                std::set<std::string> elems_ls;
                for (auto& surf : chart_surfaces_) {
                    for (auto& e : elementos_de_formula(surf.legend)) {
                        if (elems_ls.insert(e).second) elems_set.push_back(e);
                    }
                }

                ImGui::Text(tr("Legend"));
                if (ImGui::BeginTable("##gem_legend_split", 2, ImGuiTableFlags_BordersInnerV, ImVec2(0, 290))) {
                    ImGui::TableSetupColumn("SpList", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("ElList", ImGuiTableColumnFlags_WidthFixed, 82.0f);
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn();

                    if (ImGui::BeginTable("##gem_legend", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 280))) {
                        ImGui::TableSetupColumn("", ImGuiTableColumnFlags_WidthFixed, 24.0f);
                        ImGui::TableSetupColumn("Col", ImGuiTableColumnFlags_WidthFixed, 22.0f);
                        ImGui::TableSetupColumn("Description", ImGuiTableColumnFlags_WidthStretch);
                        ImGui::TableSetupColumn("Phase", ImGuiTableColumnFlags_WidthFixed, 42.0f);
                        ImGui::TableHeadersRow();
                        for (auto& surf : chart_surfaces_) {
                            if (filtro_fase_ > 0 && surf.fase_num != filtro_fase_) continue;
                            ImGui::TableNextRow();
                            ImGui::PushID((int)(surf.id) + 9200);
                            ImGui::TableNextColumn();
                            ImGui::Checkbox("##vis", &surf.visible);
                            ImGui::TableNextColumn();
                            ImGui::ColorEdit4("##scol", &surf.color.x, ImGuiColorEditFlags_NoInputs | ImGuiColorEditFlags_NoLabel);
                            ImGui::TableNextColumn();
                            ImGui::TextUnformatted(surf.legend.c_str());
                            ImGui::TableNextColumn();
                            ImGui::Text("%d", surf.fase_num);
                            ImGui::PopID();
                        }
                        ImGui::EndTable();
                    }

                    ImGui::TableNextColumn();
                    if (ImGui::BeginListBox("##elem_filter_box", ImVec2(-1, 280))) {
                        for (auto& el : elems_set) {
                            ImGui::Selectable(el.c_str(), false);
                        }
                        ImGui::Selectable(tr("All elements"), true);
                        ImGui::EndListBox();
                    }
                    ImGui::EndTable();
                }

                // ── "All elements must exist" (como en la imagen HSC) ──
                ImGui::Checkbox(tr("All elements must exist##chart"), &todos_elementos_);

                // ── Lista de fases (filtra las superficies visibles) ──
                if (ImGui::BeginListBox("##phase_list", ImVec2(-1, 82))) {
                    if (ImGui::Selectable(tr("All phases"), filtro_fase_ <= 0)) filtro_fase_ = -1;
                    int max_fase = 0;
                    for (auto& surf : chart_surfaces_) max_fase = std::max(max_fase, surf.fase_num);
                    max_fase = std::max(max_fase, 3);
                    for (int f = 1; f <= max_fase; ++f) {
                        char lbl[32];
                        snprintf(lbl, sizeof(lbl), "Phase #%d", f);
                        if (ImGui::Selectable(lbl, filtro_fase_ == f)) filtro_fase_ = f;
                    }
                    ImGui::EndListBox();
                }

                // ── Ajustes de ejes (escala + límites) ──
                if (ImGui::CollapsingHeader(tr("Escala y límites de ejes"))) {
                    struct EjeCtrl { const char* nombre; bool* log; bool* manual; double* mn; double* mx; };
                    EjeCtrl ejes[3] = {
                        {"X", &eje_x_log_, &eje_x_manual_, &x_min_user_, &x_max_user_},
                        {"Y", &eje_y_log_, &eje_y_manual_, &y_min_user_, &y_max_user_},
                        {"Z (Amount)", &eje_z_log_, &eje_z_manual_, &z_min_user_, &z_max_user_},
                    };
                    for (auto& ej : ejes) {
                        ImGui::PushID(ej.nombre);
                        ImGui::Text("%s", ej.nombre);
                        ImGui::SameLine();
                        ImGui::Checkbox(tr("Log"), ej.log);
                        ImGui::SameLine();
                        ImGui::Checkbox(tr("Límites"), ej.manual);
                        if (*ej.manual) {
                            ImGui::PushItemWidth(70);
                            ImGui::InputDouble("min", ej.mn, 0, 0, "%.3g");
                            ImGui::SameLine();
                            ImGui::InputDouble("max", ej.mx, 0, 0, "%.3g");
                            ImGui::PopItemWidth();
                        }
                        ImGui::PopID();
                    }
                }

                if (ImGui::Button(tr("Table"), ImVec2(-1, 0)) && calculado_) {
                    show_table_window_ = true;
                }
                } // mostrar_parametros_

                ImGui::EndTable();
            }

            // ── Popup: Acerca de (menú Help) ──
            if (mostrar_acerca_) {
                ImGui::OpenPopup(tr("Acerca del módulo GEM"));
                if (ImGui::BeginPopupModal(tr("Acerca del módulo GEM"), &mostrar_acerca_, ImGuiWindowFlags_AlwaysAutoResize)) {
                    ImGui::TextWrapped(tr("Equilibrium Compositions (GEM).\nCalcula la composición de equilibrio y las fases predominantes\nmediante minimización de energía de Gibbs.\nReplica el módulo 'Equilibrium Calculations' de HSC Chemistry."));
                    if (ImGui::Button(tr("Cerrar"))) { mostrar_acerca_ = false; ImGui::CloseCurrentPopup(); }
                    ImGui::EndPopup();
                }
            }
        }
        ImGui::End();
    }

    // ── Ventana de tabla numérica (botón "Table" de HSC) ──
    if (show_table_window_ && calculado_ && !x_values_.empty()) {
        ImGui::SetNextWindowSize(ImVec2(950, 520), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Equilibrium Data Table"), &show_table_window_)) {
            const char* hdr_x = (var_x_ == VarBarrido::Temperatura) ? "T (°C)"
                              : (var_x_ == VarBarrido::Presion) ? "P (bar)" : "Feed (kmol)";
            const char* hdr_y = (var_y_ == VarBarrido::Temperatura) ? "T (°C)"
                              : (var_y_ == VarBarrido::Presion) ? "P (bar)" : "Feed (kmol)";

            if (plot_p_steps_ > 1) {
                ImGui::SliderInt("Row slice", &table_p_index_, 0, plot_p_steps_ - 1);
                ImGui::SameLine();
            }
            int p_idx = std::clamp(table_p_index_, 0, plot_p_steps_ > 0 ? plot_p_steps_ - 1 : 0);
            double Y_sel = y_values_[p_idx * plot_t_steps_];
            ImGui::Text("%s = %.4g", hdr_y, Y_sel);

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
                ImGui::TableSetupColumn(hdr_x, ImGuiTableColumnFlags_WidthFixed, 85);
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
                    ImGui::TableNextColumn(); ImGui::Text("%.2f", x_values_[idx]);
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

            // Guardar tabla completa (todos los cortes X x Y) como texto (HSC Table button)
            if (ImGui::Button(tr("Save Table (.txt)"), ImVec2(150, 26))) {
                adapters::DesktopFileDialog dlg;
                std::string path = dlg.select("Save Equilibrium Table", "*.txt", true);
                if (!path.empty()) {
                    if (path.find(".txt") == std::string::npos) path += ".txt";
                    FILE* f = fopen(path.c_str(), "w");
                    if (f) {
                        fprintf(f, "# Equilibrium table: %s\n", titulo_chart_);
                        fprintf(f, "%s\t%s\tG(kJ)", hdr_x, hdr_y);
                        for (auto& surf : chart_surfaces_) fprintf(f, "\t%s(Phase %d)", surf.legend.c_str(), surf.fase_num);
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
            ImGui::SameLine();
            if (ImGui::Button(tr("Copiar al Portapapeles"), ImVec2(150, 26))) {
                std::string clip = std::string(hdr_x) + "\t" + hdr_y + "\tG(kJ)";
                for (auto& surf : chart_surfaces_) { clip += "\t" + surf.legend + "(Phase " + std::to_string(surf.fase_num) + ")"; }
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
