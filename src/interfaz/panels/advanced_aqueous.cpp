#include "scientific.hpp"
#include <imgui.h>
namespace cma::interfaz {
void AdvancedAqueousPanel::render() {
    ImGui::TextUnformatted("Equilibrio acuoso avanzado · USGS IPhreeqc");
    ImGui::TextWrapped("Motor y bases USGS independientes del catálogo heredado. Pitzer usa parámetros de interacción de pitzer.dat; phreeqc.dat usa el modelo de asociación iónica extendido.");
    if (!engine_.available()) {
        ImGui::TextColored(ImVec4(0.9f, 0.3f, 0.2f, 1.0f), "Motor no disponible: compile con CMA_WITH_IPHREEQC=ON y compruebe data/reference/phreeqc.");
        return;
    }

    bool edited = ImGui::Combo("Modelo termodinámico", &model_, "Asociación iónica (phreeqc.dat)\0Pitzer (pitzer.dat)\0");
    input_.model = (model_ == 0) ? domain::AqueousBackend::IonAssociation : domain::AqueousBackend::Pitzer;
    edited |= ImGui::InputDouble("Temperatura [°C; 0..100]", &input_.celsius, 1.0, 5.0, "%.2f");
    edited |= ImGui::InputDouble("pH inicial [0..14]", &input_.ph, 0.1, 1.0, "%.2f");

    if (ImGui::CollapsingHeader("Totales analíticos en solución (mol/kgw)", ImGuiTreeNodeFlags_DefaultOpen)) {
        static const std::vector<std::string> pitzer_comps = {"Na", "Cl", "Ca", "Mg", "K", "C(4)", "S(6)", "Fe", "Mn", "Ba", "Sr", "Li", "Br", "B", "Si"};
        static const std::vector<std::string> phreeqc_comps = {"Na", "Cl", "Ca", "Mg", "K", "C(4)", "S(6)", "Fe", "Cu", "Zn", "Pb", "Cd", "Al", "Mn", "Ba", "Sr", "Si", "Li", "Br", "B", "F", "P"};
        const auto& active_comps = (model_ == 1) ? pitzer_comps : phreeqc_comps;

        if (ImGui::BeginTable("advanced_totals", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
            ImGui::TableSetupColumn("Componente", ImGuiTableColumnFlags_WidthFixed, 80.0f);
            ImGui::TableSetupColumn("mol/kgw", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("Componente", ImGuiTableColumnFlags_WidthFixed, 80.0f);
            ImGui::TableSetupColumn("mol/kgw", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableHeadersRow();

            for (size_t i = 0; i < active_comps.size(); i += 2) {
                ImGui::TableNextRow();
                // Col 1 & 2
                ImGui::TableNextColumn();
                ImGui::TextUnformatted(active_comps[i].c_str());
                ImGui::TableNextColumn();
                ImGui::PushID(active_comps[i].c_str());
                ImGui::SetNextItemWidth(-1);
                edited |= ImGui::InputDouble("##val", &input_.totals_molal[active_comps[i]], 0, 0, "%.6g");
                ImGui::PopID();

                // Col 3 & 4
                if (i + 1 < active_comps.size()) {
                    ImGui::TableNextColumn();
                    ImGui::TextUnformatted(active_comps[i + 1].c_str());
                    ImGui::TableNextColumn();
                    ImGui::PushID(active_comps[i + 1].c_str());
                    ImGui::SetNextItemWidth(-1);
                    edited |= ImGui::InputDouble("##val", &input_.totals_molal[active_comps[i + 1]], 0, 0, "%.6g");
                    ImGui::PopID();
                } else {
                    ImGui::TableNextColumn();
                    ImGui::TableNextColumn();
                }
            }
            ImGui::EndTable();
        }
    }

    edited |= ImGui::Checkbox("Imponer electroneutralidad ajustando Na", &input_.balance_sodium);
    if (input_.balance_sodium) {
        ImGui::TextDisabled("El total de Na se ajustará automáticamente para balancear la carga de la solución.");
    }

    if (ImGui::CollapsingHeader("Fases minerales y gaseosas en equilibrio", ImGuiTreeNodeFlags_DefaultOpen)) {
        ImGui::TextWrapped("Marque los minerales a considerar para precipitación o disolución en equilibrio (SI = 0). Moles = 0 permite precipitación si se satura; moles > 0 permite disolución hasta agotarse.");
        static const std::vector<std::string> minerales = {
            "Calcite", "Gypsum", "Halite", "Sylvite", "Dolomite", "Barite", "Anhydrite", "Siderite"
        };
        for (const auto& min : minerales) {
            bool activo = input_.equilibrium_phases.count(min) > 0 || (min == "Calcite" && input_.equilibrate_calcite);
            ImGui::PushID(min.c_str());
            if (ImGui::Checkbox(min.c_str(), &activo)) {
                edited = true;
                if (activo) {
                    input_.equilibrium_phases[min] = 10.0;
                    if (min == "Calcite") input_.equilibrate_calcite = true;
                } else {
                    input_.equilibrium_phases.erase(min);
                    if (min == "Calcite") input_.equilibrate_calcite = false;
                }
            }
            if (activo) {
                ImGui::SameLine();
                ImGui::SetNextItemWidth(120);
                double moles = input_.equilibrium_phases[min];
                if (ImGui::InputDouble("moles disponibles", &moles, 0.1, 1.0, "%.3g")) {
                    edited = true;
                    input_.equilibrium_phases[min] = std::max(0.0, moles);
                    if (min == "Calcite") input_.calcite_moles = input_.equilibrium_phases[min];
                }
            }
            ImGui::PopID();
        }

        edited |= ImGui::Checkbox("Equilibrar con CO2(g)", &input_.equilibrate_co2);
        if (input_.equilibrate_co2) {
            ImGui::SameLine();
            ImGui::SetNextItemWidth(120);
            edited |= ImGui::InputDouble("log10 PCO2 [atm]", &input_.log_co2_pressure, 0.1, 1.0, "%.2f");
        }
    }

    if (edited) calculated_ = false;

    if (ImGui::Button("Resolver equilibrio avanzado (USGS)")) {
        result_ = engine_.solve(input_);
        calculated_ = true;
    }

    if (!calculated_) {
        ImGui::TextDisabled("Configure la composición y resuelva el equilibrio.");
        return;
    }

    ImGui::Separator();
    ImGui::TextWrapped("%s", result_.message.c_str());
    if (!result_.ok) return;

    ImGui::TextWrapped("%s · Base: %s", result_.engine.c_str(), result_.database.c_str());

    // Resumen de parámetros fisicoquímicos
    if (ImGui::CollapsingHeader("Propiedades globales de la solución", ImGuiTreeNodeFlags_DefaultOpen)) {
        if (ImGui::BeginTable("summary_props", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
            ImGui::TableSetupColumn("Propiedad"); ImGui::TableSetupColumn("Valor");
            ImGui::TableSetupColumn("Propiedad"); ImGui::TableSetupColumn("Valor");
            ImGui::TableHeadersRow();

            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::TextUnformatted("pH final");
            ImGui::TableNextColumn(); ImGui::Text("%.4f", result_.values.count("pH") ? result_.values.at("pH") : 0.0);
            ImGui::TableNextColumn(); ImGui::TextUnformatted("pe (redox)");
            ImGui::TableNextColumn(); ImGui::Text("%.4f", result_.values.count("pe") ? result_.values.at("pe") : 0.0);

            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::TextUnformatted("Fuerza iónica (mu)");
            ImGui::TableNextColumn(); ImGui::Text("%.6f mol/kgw", result_.values.count("mu") ? result_.values.at("mu") : 0.0);
            ImGui::TableNextColumn(); ImGui::TextUnformatted("Error balance carga");
            ImGui::TableNextColumn(); ImGui::Text("%.4e %%", result_.values.count("pct_err") ? result_.values.at("pct_err") : 0.0);

            ImGui::EndTable();
        }
    }

    // Tabla de Índices de Saturación (SI)
    if (ImGui::CollapsingHeader("Índices de Saturación de Fases (SI)", ImGuiTreeNodeFlags_DefaultOpen)) {
        if (ImGui::BeginTable("si_table", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 180))) {
            ImGui::TableSetupColumn("Mineral / Fase", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("SI = log(IAP/Ksp)", ImGuiTableColumnFlags_WidthFixed, 140.0f);
            ImGui::TableSetupColumn("Estado de saturación", ImGuiTableColumnFlags_WidthFixed, 160.0f);
            ImGui::TableHeadersRow();

            for (const auto& [key, val] : result_.values) {
                if (key.rfind("si_", 0) == 0 && key.size() > 3) {
                    std::string min = key.substr(3);
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn();
                    ImGui::TextUnformatted(min.c_str());
                    ImGui::TableNextColumn();
                    if (val < -500.0) {
                        ImGui::TextDisabled("-Inf (ausente)");
                        ImGui::TableNextColumn();
                        ImGui::TextDisabled("No precipita");
                    } else {
                        ImGui::Text("%.4f", val);
                        ImGui::TableNextColumn();
                        if (val > 0.01) {
                            ImGui::TextColored(ImVec4(0.9f, 0.4f, 0.1f, 1.0f), "Sobresaturado");
                        } else if (val < -0.01) {
                            ImGui::TextColored(ImVec4(0.3f, 0.6f, 0.9f, 1.0f), "Subsaturado");
                        } else {
                            ImGui::TextColored(ImVec4(0.2f, 0.9f, 0.3f, 1.0f), "En equilibrio (sat)");
                        }
                    }
                }
            }
            ImGui::EndTable();
        }
    }

    // Tabla de Especies, Actividades y Coeficientes gamma_i
    if (ImGui::CollapsingHeader("Especiación y Coeficientes de Actividad (gamma)")) {
        if (ImGui::BeginTable("act_table", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 220))) {
            ImGui::TableSetupColumn("Especie acuosa");
            ImGui::TableSetupColumn("Molalidad (mol/kgw)");
            ImGui::TableSetupColumn("Actividad (a_i)");
            ImGui::TableSetupColumn("Coef. Actividad (gamma)");
            ImGui::TableHeadersRow();

            for (const auto& [key, val] : result_.values) {
                if (key.rfind("act_", 0) == 0 && key.size() > 4) {
                    std::string sp = key.substr(4);
                    std::string m_key = "m_" + sp + "(mol/kgw)";
                    std::string g_key = "gamma_" + sp;

                    double m_val = result_.values.count(m_key) ? result_.values.at(m_key) : 0.0;
                    double g_val = result_.values.count(g_key) ? result_.values.at(g_key) : 0.0;

                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::TextUnformatted(sp.c_str());
                    ImGui::TableNextColumn(); ImGui::Text("%.6e", m_val);
                    ImGui::TableNextColumn(); ImGui::Text("%.6e", val);
                    ImGui::TableNextColumn();
                    if (g_val > 0) ImGui::Text("%.4f", g_val);
                    else ImGui::TextDisabled("-");
                }
            }
            ImGui::EndTable();
        }
    }

    if (ImGui::CollapsingHeader("Salida completa de texto IPhreeqc")) {
        ImGui::BeginChild("phreeqc_output", ImVec2(0, 220), true, ImGuiWindowFlags_HorizontalScrollbar);
        ImGui::TextUnformatted(result_.full_output.c_str());
        ImGui::EndChild();
    }
}
}
