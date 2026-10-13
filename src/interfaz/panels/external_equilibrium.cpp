#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de ExternalAPI
// ═══════════════════════════════════════════════════════════════════════════

void PanelExternalAPI::renderizar() {
    ImGui::Text(tr("Importación de datos externos"));
    ImGui::Separator();

    ImGui::InputText(tr("Ruta CSV"), comando_, sizeof(comando_));
    if (ImGui::Button(tr("Importar CSV"), ImVec2(-1, 30))) {
        std::vector<std::vector<double>> datos;
        if (motor_.importarCSV(comando_, datos)) {
            std::string res;
            for (auto& fila : datos) {
                for (auto& v : fila)
                    res += std::to_string(v) + " ";
                res += "\n";
            }
            strncpy(resultado_, res.c_str(), sizeof(resultado_) - 1);
            resultado_[sizeof(resultado_) - 1] = '\0';
        } else {
            snprintf(resultado_, sizeof(resultado_), "Error al importar CSV");
        }
        ejecutado_ = true;
    }

    if (ejecutado_) {
        ImGui::Separator();
        ImGui::Text(tr("Resultado:"));
        ImGui::InputTextMultiline("##res", resultado_, sizeof(resultado_),
            ImVec2(-1, ImGui::GetContentRegionAvail().y - 10),
            ImGuiInputTextFlags_ReadOnly);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Equilibrium Calculations (HSC Equilibrium)
// ═══════════════════════════════════════════════════════════════════════════

void PanelEquilibriumCalc::reset() {
    especies_.clear();
    resultados_.clear();
    nom_especies_.clear();
    calculado_ = false;
    calcular_ = false;
    convergio_ = false;
    mostrar_selector_ = false;
    t_inicio_ = 500.0;
    t_fin_ = 1500.0;
    num_pasos_ = 20;
    presion_ = 1.0;
    modo_celda_ = false;
    escala_log_ = false;
    auto_escala_ = true;
}

void PanelEquilibriumCalc::agregar_especie_desde_bd(const std::string& nombre, double cant) {
    EspecieInput ei;
    ei.id = 0;
    strncpy(ei.nombre, nombre.c_str(), sizeof(ei.nombre) - 1);
    ei.cantidad = cant;
    ei.activo = true;
    especies_.push_back(ei);
}

void PanelEquilibriumCalc::renderizar_selector_especies(
    const std::map<int64_t, cma::nucleo::Especie>&)
{
    ImGui::Text(tr("Agregar especies al sistema (Feed / Posibles Productos):"));
    if (ImGui::Button(tr("Examinar Base de Datos##eq"), ImVec2(-1, 0))) {
        ModalBuscadorDB::instancia().abrir("Buscar Especie (Equilibrio)");
    }
    
    std::vector<const nucleo::Especie*> sel_eq;
    if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Equilibrio)", sel_eq, true)) {
        for (auto* esp : sel_eq) {
            bool found = false;
            for (auto& s : especies_) {
                if (s.id == esp->id) { found = true; break; }
            }
            if (!found) {
                agregar_especie_desde_bd(esp->nombre, 0.0);
                especies_.back().id = esp->id;
                nom_especies_[esp->id] = esp->nombre;
            }
        }
    }

    ImGui::Separator();
    ImGui::Text(tr("Especies cargadas:"));
    if (ImGui::BeginTable("##tab_esp", 4,
        ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_Resizable))
    {
        ImGui::TableSetupColumn("Act");
        ImGui::TableSetupColumn("Especie");
        ImGui::TableSetupColumn("kmol");
        ImGui::TableSetupColumn("X");
        ImGui::TableHeadersRow();
        for (size_t i = 0; i < especies_.size(); ++i) {
            auto& sp = especies_[i];
            ImGui::TableNextRow();
            ImGui::TableNextColumn();
            ImGui::PushID((int)i);
            ImGui::Checkbox("##act", &sp.activo);
            ImGui::TableNextColumn();
            ImGui::Text(tr("%s"), sp.nombre);
            ImGui::TableNextColumn();
            ImGui::PushItemWidth(-1);
            ImGui::InputDouble("##cant", &sp.cantidad, 0, 0, "%.4f");
            ImGui::PopItemWidth();
            ImGui::TableNextColumn();
            if (ImGui::Button(tr("x##del"))) {
                especies_.erase(especies_.begin() + i);
                ImGui::PopID();
                break;
            }
            ImGui::PopID();
        }
        ImGui::EndTable();
    }
}

void PanelEquilibriumCalc::ejecutar_calculo(
    const std::map<int64_t, cma::nucleo::Especie>& db)
{
    resultados_.clear();
    calculado_ = false;
    convergio_ = false;
    energia_total_ = 0.0;

    // Build feed from active species
    std::map<int64_t, double> alimentacion;
    for (auto& sp : especies_) {
        if (sp.activo && sp.id > 0)
            alimentacion[sp.id] = sp.cantidad;
    }
    if (alimentacion.empty()) return;

    double step = (t_fin_ - t_inicio_) / std::max(1, num_pasos_ - 1);
    cma::application::modules::GibbsMinimizer gibbs;
    int exitos = 0;

    for (int i = 0; i < num_pasos_; ++i) {
        double T = t_inicio_ + i * step;
        auto res = gibbs.minimizar(alimentacion, T, presion_, db);
        if (res.convergio) {
            exitos++;
            PuntoEquilibrio pt;
            pt.temperatura = T;
            pt.composicion = res.flujo_molar;
            pt.G_total = res.energia_gibbs_total;
            resultados_.push_back(std::move(pt));
        }
    }

    calculado_ = true;
    convergio_ = (exitos == num_pasos_);
    energia_total_ = resultados_.empty() ? 0.0 : resultados_.back().G_total;

    // Collect species names from results
    for (auto& pt : resultados_) {
        for (auto& [sid, _] : pt.composicion) {
            if (nom_especies_.count(sid) == 0) {
                for (auto& [db_id, e] : db) {
                    if (db_id == sid) {
                        nom_especies_[sid] = e.nombre;
                        break;
                    }
                }
            }
        }
        // Fix names for IDs that were resolved later
        for (auto& [sid, _] : pt.composicion) {
            auto it = nom_especies_.find(sid);
            if (it == nom_especies_.end()) {
                for (auto& [db_id, e] : db) {
                    if (db_id == sid) {
                        nom_especies_[sid] = e.nombre;
                        break;
                    }
                }
            }
        }
    }
}

void PanelEquilibriumCalc::renderizar() {
    ImGui::Text(tr("Cálculos de Equilibrio"));
    ImGui::Separator();
    ImGui::TextWrapped(tr("Calculate equilibrium composition and phase amounts using Gibbs energy minimization."));

    auto& gestor = cma::base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();

    // ── Input section ──
    if (ImGui::CollapsingHeader("Feed & Species", ImGuiTreeNodeFlags_DefaultOpen)) {
        renderizar_selector_especies(db);
    }

    if (ImGui::CollapsingHeader("Rango de Temperatura", ImGuiTreeNodeFlags_DefaultOpen)) {
        ImGui::InputDouble("T inicio (K)", &t_inicio_, 10, 100, "%.0f");
        ImGui::InputDouble("T fin (K)", &t_fin_, 10, 100, "%.0f");
        ImGui::SliderInt("Pasos", &num_pasos_, 2, 200);
        ImGui::InputDouble("Presión (atm)", &presion_, 0.1, 1, "%.3f");
    }

    if (ImGui::CollapsingHeader("Opciones", ImGuiTreeNodeFlags_DefaultOpen)) {
        ImGui::Checkbox(tr("Modo Celda Electroquímica"), &modo_celda_);
        if (modo_celda_) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8,0.8,0.2,1),
                "Cell mode: calculates charge and phase composition in electrochemical cells.");
        }
        ImGui::Checkbox(tr("Escala logarítmica Y"), &escala_log_);
        ImGui::Checkbox(tr("Auto escala"), &auto_escala_);
        if (!auto_escala_) {
            ImGui::InputDouble("Y min", &eje_y_min_, 0, 0, "%.2e");
            ImGui::InputDouble("Y max", &eje_y_max_, 0, 0, "%.2e");
        }
    }

    // ── Calculate ──
    if (ImGui::Button(tr("Calcular"), ImVec2(-1, 36))) {
        ejecutar_calculo(db);
    }

    // ── Results ──
    if (calculado_) {
        ImGui::Separator();
        ImGui::TextColored(convergio_ ? cma::interfaz::AdaptiveColor(0,1,0,1) : cma::interfaz::AdaptiveColor(1,0.5,0,1),
            "%s  |  %zu temperaturas calculadas  |  G_total = %.2e kJ",
            convergio_ ? "[OK] Converged" : "[!] Partial convergence",
            resultados_.size(), energia_total_);

        if (!resultados_.empty()) {
            // ── Plot ──
            if (ImPlot::BeginPlot("##equilibrium_plot", ImVec2(-1, 350),
                ImPlotFlags_NoTitle | ImPlotFlags_NoInputs))
            {
                ImPlot::SetupAxis(ImAxis_X1, "Temperature (K)");
                ImPlot::SetupAxis(escala_log_ ? ImAxis_Y1 : ImAxis_Y1, "kmol");

                if (!auto_escala_ && eje_y_max_ > eje_y_min_) {
                    ImPlot::SetupAxisLimits(ImAxis_Y1, eje_y_min_, eje_y_max_);
                }
                if (escala_log_)
                    ImPlot::SetupAxisScale(ImAxis_Y1, ImPlotScale_Log10);

                // Collect all species IDs across all points
                std::set<int64_t> todas_sp;
                for (auto& pt : resultados_)
                    for (auto& [sid, _] : pt.composicion)
                        todas_sp.insert(sid);

                // Plot each species as a line
                for (int64_t sid : todas_sp) {
                    std::vector<double> xs, ys;
                    for (auto& pt : resultados_) {
                        xs.push_back(pt.temperatura);
                        auto it = pt.composicion.find(sid);
                        ys.push_back(it != pt.composicion.end() ? it->second : 0.0);
                    }
                    auto it = nom_especies_.find(sid);
                    const char* label = (it != nom_especies_.end()) ? it->second.c_str() : "?";
                    ImPlot::PlotLine(label, xs.data(), ys.data(), (int)xs.size());
                }

                ImPlot::EndPlot();
            }

            // ── Table ──
            if (ImGui::CollapsingHeader("Tabla de Resultados", ImGuiTreeNodeFlags_DefaultOpen)) {
                // Collect all species
                std::vector<int64_t> todas_ids;
                std::set<int64_t> ids_set;
                for (auto& pt : resultados_)
                    for (auto& [sid, _] : pt.composicion)
                        if (ids_set.insert(sid).second)
                            todas_ids.push_back(sid);

                if (ImGui::BeginTable("##eq_table", 2 + (int)todas_ids.size(),
                    ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY |
                    ImGuiTableFlags_SizingFixedFit | ImGuiTableFlags_NoHostExtendX))
                {
                    ImGui::TableSetupScrollFreeze(0, 1);
                    ImGui::TableSetupColumn("T (K)", ImGuiTableColumnFlags_WidthFixed, 80);
                    ImGui::TableSetupColumn("G (kJ)", ImGuiTableColumnFlags_WidthFixed, 100);
                    for (int64_t sid : todas_ids) {
                        auto it = nom_especies_.find(sid);
                        ImGui::TableSetupColumn(it != nom_especies_.end() ? it->second.c_str() : "?",
                            ImGuiTableColumnFlags_WidthFixed, 90);
                    }
                    ImGui::TableHeadersRow();

                    // Limit rows for display
                    int max_rows = std::min((int)resultados_.size(), 500);
                    for (int r = 0; r < max_rows; ++r) {
                        auto& pt = resultados_[r];
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.1f"), pt.temperatura);
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.3f"), pt.G_total);
                        for (int64_t sid : todas_ids) {
                            ImGui::TableNextColumn();
                            auto it = pt.composicion.find(sid);
                            double v = (it != pt.composicion.end()) ? it->second : 0.0;
                            if (std::abs(v) < 1e-4)
                                ImGui::Text(tr("0"));
                            else
                                ImGui::Text(tr("%.4f"), v);
                        }
                    }
                    ImGui::EndTable();
                }

                // ── Save to file ──
                if (ImGui::Button(tr("Save Table (.txt)"), ImVec2(160, 28))) {
                    std::string fname = "equilibrium_results.txt";
                    FILE* f = fopen(fname.c_str(), "w");
                    if (f) {
                        // Header
                        fprintf(f, "T (K)\tG (kJ)");
                        for (int64_t sid : todas_ids) {
                            auto it = nom_especies_.find(sid);
                            fprintf(f, "\t%s", it != nom_especies_.end() ? it->second.c_str() : "?");
                        }
                        fprintf(f, "\n");
                        // Data
                        for (auto& pt : resultados_) {
                            fprintf(f, "%.0f\t%.2e", pt.temperatura, pt.G_total);
                            for (int64_t sid : todas_ids) {
                                auto it = pt.composicion.find(sid);
                                double v = (it != pt.composicion.end()) ? it->second : 0.0;
                                fprintf(f, "\t%.6e", v);
                            }
                            fprintf(f, "\n");
                        }
                        fclose(f);
                    }
                }
                ImGui::SameLine();
                if (ImGui::Button(tr("Copiar al Portapapeles"), ImVec2(160, 28))) {
                    std::string clip;
                    clip += "T (K)\tG (kJ)";
                    for (int64_t sid : todas_ids) {
                        auto it = nom_especies_.find(sid);
                        clip += "\t";
                        clip += (it != nom_especies_.end()) ? it->second : "?";
                    }
                    clip += "\n";
                    char buf[64];
                    for (auto& pt : resultados_) {
                        snprintf(buf, sizeof(buf), "%.1f", pt.temperatura);
                        clip += buf; clip += "\t";
                        snprintf(buf, sizeof(buf), "%.3f", pt.G_total);
                        clip += buf;
                        for (int64_t sid : todas_ids) {
                            auto it = pt.composicion.find(sid);
                            double v = (it != pt.composicion.end()) ? it->second : 0.0;
                            clip += "\t";
                            if (std::abs(v) < 1e-4) clip += "0";
                            else {
                                snprintf(buf, sizeof(buf), "%.4f", v);
                                clip += buf;
                            }
                        }
                        clip += "\n";
                    }
                    ImGui::SetClipboardText(clip.c_str());
                }
            }
        }
    }
}

}} // namespace
