#include "shared.hpp"
#include <random>
using namespace cma::interfaz;
// ── Panel POURBAIX (Eh-pH) ────────────────────────────────────────────────
#include "motores/pourbaix.hpp"

cma::interfaz::PanelPourbaix::PanelPourbaix(const domain::ThermoRepository& repository) : calc_(repository) {
    elemento_actual_ = "Fe";
    temperatura_actual_ = 298.15;
    concentracion_actual_ = 1e-6;
    calculado_ = false;
}

void cma::interfaz::PanelPourbaix::renderizar() {
    ImGui::Text("%s", tr("Configuración del Sistema"));
    ImGui::Separator();

    char buf[16];
    strncpy(buf, elemento_actual_.c_str(), sizeof(buf));
    buf[sizeof(buf)-1] = '\0';
    if (ImGui::InputText(tr("Elemento Principal"), buf, sizeof(buf))) {
        elemento_actual_ = buf;
    }

    float t_c = (float)(temperatura_actual_ - 273.15);
    if (ImGui::SliderFloat(tr("Temperatura (°C)"), &t_c, 0.0f, 300.0f)) {
        temperatura_actual_ = t_c + 273.15;
    }

    float conc_exp = log10f((float)concentracion_actual_);
    if (ImGui::SliderFloat(tr("Concentración (log M)"), &conc_exp, -10.0f, 2.0f)) {
        concentracion_actual_ = pow(10.0, conc_exp);
    }

    if (ImGui::Button(tr("Calcular Diagrama"))) {
        calc_.establecer_elemento(elemento_actual_);
        calc_.establecer_temperatura(temperatura_actual_);
        calc_.establecer_concentracion(concentracion_actual_);
        calculado_ = calc_.calcular();
    }

    ImGui::SameLine();
    if (ImGui::Button(tr("Recalcular"))) {
        calc_.establecer_elemento(elemento_actual_);
        calc_.establecer_temperatura(temperatura_actual_);
        calc_.establecer_concentracion(concentracion_actual_);
        calculado_ = calc_.calcular();
    }

    ImGui::Separator();

    // ── Lista de especies encontradas ──
    if (calculado_) {
        auto& especies = calc_.obtener_especies();
        if (!especies.empty()) {
            if (ImGui::CollapsingHeader(tr("Especies Encontradas"))) {
                if (ImGui::BeginTable("##pourbaix_esp", 5,
                        ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                        ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable)) {
                    ImGui::TableSetupColumn("Fórmula", ImGuiTableColumnFlags_WidthFixed, 90);
                    ImGui::TableSetupColumn("Estado", ImGuiTableColumnFlags_WidthFixed, 60);
                    ImGui::TableSetupColumn("Carga", ImGuiTableColumnFlags_WidthFixed, 50);
                    ImGui::TableSetupColumn("ΔGf° (kJ/mol)", ImGuiTableColumnFlags_WidthFixed, 110);
                    ImGui::TableSetupColumn("Nombre");
                    ImGui::TableHeadersRow();
                    for (size_t i = 0; i < especies.size(); ++i) {
                        const auto& e = especies[i];
                        ImGui::TableNextRow();
                        ImGui::TableSetColumnIndex(0);
                        ImGui::TextUnformatted(e.formula.c_str());
                        ImGui::TableSetColumnIndex(1);
                        const char* est = e.estado == 0 ? "Gas" :
                                          e.estado == 1 ? "Líq" :
                                          e.estado == 2 ? "Sól" : "Acu";
                        ImGui::TextUnformatted(est);
                        ImGui::TableSetColumnIndex(2);
                        if (e.carga != 0)
                            ImGui::Text("%+d", e.carga);
                        else
                            ImGui::TextUnformatted("0");
                        ImGui::TableSetColumnIndex(3);
                        ImGui::Text("%.2f", e.dG_f);
                        ImGui::TableSetColumnIndex(4);
                        ImGui::TextUnformatted(e.nombre.c_str());
                    }
                    ImGui::EndTable();
                }
            }
        }
    }

    // ── Diagrama Eh-pH ──
    if (ImPlot::BeginPlot(tr("Diagrama de Pourbaix"), ImVec2(-1, -1))) {
        ImPlot::SetupAxes("pH", "Eh (V)", ImPlotAxisFlags_None, ImPlotAxisFlags_None);
        ImPlot::SetupAxesLimits(0, 14, -2.0, 2.0);

        // ── Regiones de dominancia (heatmap) ──
        if (calculado_) {
            const auto& dom = calc_.obtener_dominancia();
            if (dom.n_pH > 0 && dom.n_Eh > 0 && !dom.grid.empty()) {
                // Colores por índice de especie (paleta fija en ImVec4)
                ImVec4 colors[] = {
                    ImVec4(0.91f,0.30f,0.24f,0.70f),  // rojo
                    ImVec4(0.18f,0.53f,0.76f,0.70f),  // azul
                    ImVec4(0.15f,0.68f,0.38f,0.70f),  // verde
                    ImVec4(0.95f,0.61f,0.07f,0.70f),  // naranja
                    ImVec4(0.61f,0.35f,0.71f,0.70f),  // violeta
                    ImVec4(0.95f,0.77f,0.06f,0.70f),  // amarillo
                    ImVec4(0.10f,0.74f,0.61f,0.70f),  // turquesa
                    ImVec4(0.93f,0.94f,0.95f,0.50f),  // gris claro
                    ImVec4(0.58f,0.65f,0.65f,0.70f),  // gris medio
                    ImVec4(0.20f,0.29f,0.37f,0.70f),  // gris oscuro
                    ImVec4(0.91f,0.30f,0.24f,0.40f),
                    ImVec4(0.18f,0.53f,0.76f,0.40f),
                    ImVec4(0.15f,0.68f,0.38f,0.40f),
                    ImVec4(0.95f,0.61f,0.07f,0.40f),
                    ImVec4(0.61f,0.35f,0.71f,0.40f),
                };

                auto& especies = calc_.obtener_especies();

                // Renderizar como puntos coloreados por especie dominante
                ImPlot::PushStyleVar(ImPlotStyleVar_MarkerSize, 3.0f);
                for (int ip = 0; ip < dom.n_pH; ++ip) {
                    double pH = dom.pH_min + (dom.pH_max - dom.pH_min) *
                                (double)ip / (double)(dom.n_pH - 1);
                    for (int ie = 0; ie < dom.n_Eh; ++ie) {
                        double Eh = dom.Eh_min + (dom.Eh_max - dom.Eh_min) *
                                    (double)ie / (double)(dom.n_Eh - 1);
                        int idx = dom.grid[ip * dom.n_Eh + ie];
                        if (idx < 0) continue;

                        ImVec4 c = colors[idx % 15];
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 3.0f, c, IMPLOT_AUTO, c);
                        ImPlot::PlotScatter("##dom", &pH, &Eh, 1);
                    }
                }
                ImPlot::PopStyleVar();

                // Leyenda de especies dominantes
                std::set<int> dominantes;
                for (int v : dom.grid) if (v >= 0) dominantes.insert(v);
                for (int idx : dominantes) {
                    if (idx < (int)especies.size()) {
                        double dummy = 0.0;
                        ImVec4 c = colors[idx % 15];
                        std::string label = especies[idx].formula;
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 4.0f, c, IMPLOT_AUTO, c);
                        ImPlot::PlotScatter(label.c_str(), &dummy, &dummy, 1);
                    }
                }
            }
        }

        // ── Líneas de equilibrio ──
        auto& lineas = calc_.obtener_lineas();
        for (size_t i = 0; i < lineas.size(); ++i) {
            const auto& l = lineas[i];
            std::string label = l.especie_a + " / " + l.especie_b;
            if (l.es_agua) label = "#" + label;

            if (l.vertical) {
                double xs[2] = {l.x_val, l.x_val};
                double ys[2] = {-2.0, 2.0};
                ImPlot::SetNextLineStyle(ImVec4(0.7f, 0.7f, 0.7f, 0.8f), l.es_agua ? 1.5f : 1.0f);
                ImPlot::PlotLine(label.c_str(), xs, ys, 2);
            } else {
                double xs[2] = {0.0, 14.0};
                double ys[2] = {l.b, l.b + l.m * 14.0};
                float thick = l.es_agua ? 1.5f : 1.0f;
                ImVec4 col = l.es_agua ? ImVec4(0.5f, 0.8f, 1.0f, 0.8f)
                                       : ImVec4(0.9f, 0.9f, 0.3f, 0.8f);
                ImPlot::SetNextLineStyle(col, thick);
                ImPlot::PlotLine(label.c_str(), xs, ys, 2);
            }
        }

        ImPlot::EndPlot();
    }
}
