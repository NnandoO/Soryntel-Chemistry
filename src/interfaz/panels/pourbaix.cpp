#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"
#pragma GCC diagnostic ignored "-Wformat-security"

#include "shared.hpp"
#include "motores/pourbaix.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <fstream>
#include <iomanip>
#include <set>
#include <sstream>

using namespace cma::interfaz;

namespace {

// Cuadrícula de elementos en 5 columnas exactas de la ventana HSC EpH (Imagen 1: Ac..Co, Cr..Ir, K..Pm, Po..Te, Th..Zr)
constexpr const char* kElementosEpHCols[21][5] = {
    {"Ac", "Cr", "K",  "Po", "Th"},
    {"Ag", "Cs", "Kr", "Pr", "Ti"},
    {"Al", "Cu", "La", "Pt", "Tl"},
    {"Am", "Dy", "Li", "Pu", "Tm"},
    {"Ar", "Er", "Lu", "Ra", "U" },
    {"As", "Es", "Mg", "Rb", "V" },
    {"At", "Eu", "Mn", "Re", "W" },
    {"Au", "F",  "Mo", "Rh", "Xe"},
    {"B",  "Fe", "N",  "Rn", "Y" },
    {"Ba", "Fm", "Na", "Ru", "Yb"},
    {"Be", "Fr", "Nb", "S",  "Zn"},
    {"Bi", "Ga", "Nd", "Sb", "Zr"},
    {"Bk", "Gd", "Ne", "Sc", ""  },
    {"Br", "Ge", "Ni", "Se", ""  },
    {"C",  "H",  "Np", "Si", ""  },
    {"Ca", "He", "O",  "Sm", ""  },
    {"Cd", "Hf", "Os", "Sn", ""  },
    {"Ce", "Hg", "P",  "Sr", ""  },
    {"Cf", "Ho", "Pa", "Ta", ""  },
    {"Cl", "I",  "Pb", "Tb", ""  },
    {"Co", "Ir", "Pm", "Te", ""  }
};

struct ColorAreaEpH {
    ImU32 fill;
    ImVec4 vec;
};

const ColorAreaEpH kPaletaEpH[] = {
    {IM_COL32( 95, 168, 235, 130), ImVec4(0.37f, 0.66f, 0.92f, 0.52f)},
    {IM_COL32(115, 205, 140, 130), ImVec4(0.45f, 0.80f, 0.55f, 0.52f)},
    {IM_COL32(240, 185,  95, 130), ImVec4(0.94f, 0.73f, 0.37f, 0.52f)},
    {IM_COL32(215, 135, 195, 130), ImVec4(0.84f, 0.53f, 0.76f, 0.52f)},
    {IM_COL32(150, 155, 165, 130), ImVec4(0.59f, 0.61f, 0.65f, 0.52f)},
    {IM_COL32( 85, 200, 195, 130), ImVec4(0.33f, 0.78f, 0.76f, 0.52f)},
    {IM_COL32(235, 130, 115, 130), ImVec4(0.92f, 0.51f, 0.45f, 0.52f)},
    {IM_COL32(180, 210, 100, 130), ImVec4(0.71f, 0.82f, 0.39f, 0.52f)},
    {IM_COL32(140, 145, 225, 130), ImVec4(0.55f, 0.57f, 0.88f, 0.52f)},
    {IM_COL32(225, 205, 120, 130), ImVec4(0.88f, 0.80f, 0.47f, 0.52f)}
};
constexpr size_t kNumColoresEpH = sizeof(kPaletaEpH) / sizeof(kPaletaEpH[0]);

// Opciones de estilo para las líneas de estabilidad del agua (Format H2O Stability Lines)
struct EstiloLineasAgua {
    bool mostrar_superior = true;
    bool mostrar_inferior = true;
    float grosor = 1.6f;
    ImVec4 color = ImVec4(0.12f, 0.12f, 0.12f, 0.92f);
};

EstiloLineasAgua& estilo_agua_global() {
    static EstiloLineasAgua st;
    return st;
}

void dibujar_segmento_punteado(ImDrawList* dl, ImVec2 p1, ImVec2 p2, ImU32 col, float thickness, float dash_len = 4.0f, float gap_len = 4.0f) {
    const float dx = p2.x - p1.x;
    const float dy = p2.y - p1.y;
    const float len = std::hypot(dx, dy);
    if (len < 1.0f) return;
    const float ux = dx / len;
    const float uy = dy / len;
    float t = 0.0f;
    while (t < len) {
        const float t_end = std::min(t + dash_len, len);
        dl->AddLine(ImVec2(p1.x + ux * t, p1.y + uy * t),
                    ImVec2(p1.x + ux * t_end, p1.y + uy * t_end),
                    col, thickness);
        t += dash_len + gap_len;
    }
}

} // namespace

cma::interfaz::PanelPourbaix::PanelPourbaix(const domain::ThermoRepository& repository)
    : calc_(repository) {
    // Inicializar por defecto con el sistema de referencia CUS25.eph8 (Cu - S a 25.00 °C, 1.00E+00 mol/kg, Imagen 1-4)
    cargar_preset(2);
}

void cma::interfaz::PanelPourbaix::cargar_preset(int idx) {
    calc_.cargar_preset_hsc(idx);
    const auto& res = calc_.ultimo_resultado_eph();
    params_.elementos = res.elementos_usados;
    params_.elemento_principal = res.elemento_principal;
    params_.T_K = res.T_K;
    params_.temp_en_celsius = true;
    params_.pH_min = res.pH_min;
    params_.pH_max = res.pH_max;
    params_.Eh_min = res.Eh_min;
    params_.Eh_max = res.Eh_max;
    params_.escala_potencial = res.escala_potencial;
    params_.modos_busqueda.condensed = true;
    params_.modos_busqueda.aqueous_ions = true;
    params_.modos_busqueda.aqueous_neutrals = (idx != 2 && idx != 3);
    params_.formulas_seleccionadas.clear();

    idx_elem_principal_combo_ = 0;
    for (size_t i = 0; i < params_.elementos.size(); ++i) {
        if (params_.elementos[i].symbol == params_.elemento_principal) {
            idx_elem_principal_combo_ = static_cast<int>(i);
            break;
        }
    }

    actualizar_especies_sistema(false);
    ejecutar_calculo(false);
    datasets_combiner_ = calc_.calcular_combiner_eph(params_, params_.temperaturas_combiner_C);
}

void cma::interfaz::PanelPourbaix::actualizar_especies_sistema(bool mantener_seleccion) {
    std::set<std::string> prev_sel;
    if (mantener_seleccion) {
        for (size_t i = 0; i < especies_candidatas_.size() && i < especies_seleccionadas_.size(); ++i) {
            if (especies_seleccionadas_[i]) prev_sel.insert(especies_candidatas_[i].formula);
        }
    }

    especies_candidatas_ = calc_.recolectar_especies_sistema(params_);
    especies_seleccionadas_.assign(especies_candidatas_.size(), true);

    for (size_t i = 0; i < especies_candidatas_.size(); ++i) {
        if (mantener_seleccion && !prev_sel.empty()) {
            especies_seleccionadas_[i] = prev_sel.count(especies_candidatas_[i].formula) > 0;
        } else {
            especies_seleccionadas_[i] = especies_candidatas_[i].seleccionada_por_defecto;
        }
    }
}

void cma::interfaz::PanelPourbaix::ejecutar_calculo(bool forzar_nueva_pagina) {
    params_.formulas_seleccionadas.clear();
    for (size_t i = 0; i < especies_candidatas_.size() && i < especies_seleccionadas_.size(); ++i) {
        if (especies_seleccionadas_[i]) {
            params_.formulas_seleccionadas.push_back(especies_candidatas_[i].formula);
        }
    }

    auto res = calc_.calcular_diagrama_eph(params_);
    if (!res.ok) {
        estado_msg_ = res.mensaje_error;
        return;
    }

    if (pestanas_.empty() || forzar_nueva_pagina || create_new_page_) {
        PestanaEpH nueva;
        nueva.nombre = "Chart" + std::to_string(pestanas_.size() + 1);
        nueva.params = params_;
        nueva.resultado = res;
        nueva.calculado = true;
        nueva.reset_limites = true;
        pestanas_.push_back(nueva);
        idx_pestana_activa_ = static_cast<int>(pestanas_.size()) - 1;
    } else {
        idx_pestana_activa_ = std::clamp(idx_pestana_activa_, 0, static_cast<int>(pestanas_.size()) - 1);
        auto& tab = pestanas_[idx_pestana_activa_];
        tab.params = params_;
        tab.resultado = res;
        tab.calculado = true;
        tab.reset_limites = true;
    }

    std::ostringstream os;
    os << res.titulo_diagrama << " | " << res.poligonos.size() << " áreas, "
       << res.fronteras.size() << " líneas.";
    estado_msg_ = os.str();
}

void cma::interfaz::PanelPourbaix::renderizar() {
    ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(8.0f, 6.0f));
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(6.0f, 4.0f));

    // ── Barra de Herramientas y Menús Superior estilo HSC EpH (Siempre visible y espaciada) ──
    ImGui::BeginChild("##EpHTopHeaderBar", ImVec2(0.0f, 36.0f), true, ImGuiWindowFlags_NoScrollbar);
    {
        if (ImGui::Button("File  v", ImVec2(68.0f, 24.0f))) ImGui::OpenPopup("##EpHFileMenuPopup");
        if (ImGui::BeginPopup("##EpHFileMenuPopup")) {
            ImGui::TextDisabled("Reference Systems (HSC EpH)");
            ImGui::Separator();
            if (ImGui::MenuItem("Load CUS25.eph8 (Cu - S - H2O at 25 °C — Fig. 1, 2, 3)")) cargar_preset(2);
            if (ImGui::MenuItem("Load S - Cu - H2O at 25 °C (Sulfur Main Element — Fig. 4)")) cargar_preset(3);
            if (ImGui::MenuItem("Load Mn - H2O at 25 °C (Section 17.1 Manual Example)")) cargar_preset(5);
            if (ImGui::MenuItem("Load Fe - H2O at 25 °C (Pourbaix Corrosion Reference)")) cargar_preset(0);
            if (ImGui::MenuItem("Load Au - Cl - H2O at 25 °C (Chloride Leaching)")) cargar_preset(4);
            if (ImGui::MenuItem("Load Zn - S - H2O at 25 °C (Sphalerite Leaching)")) cargar_preset(6);
            ImGui::Separator();
            if (ImGui::MenuItem("Save *.eph8 Specification...")) {
                if (std::ofstream ofs(ruta_eph8_); ofs) { ofs << calc_.serializar_eph8(params_); estado_msg_ = "Guardado en " + std::string(ruta_eph8_); }
            }
            if (ImGui::MenuItem("Open *.eph8 Specification...")) {
                if (std::ifstream ifs(ruta_eph8_); ifs) {
                    std::string content((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());
                    if (calc_.deserializar_eph8(content, params_)) { actualizar_especies_sistema(false); ejecutar_calculo(false); }
                }
            }
            if (ImGui::MenuItem("Export Table & Boundaries CSV...") && !pestanas_.empty()) {
                if (std::ofstream ofs("eph_diagram_table.csv"); ofs) ofs << calc_.exportar_tabla_eph_csv(pestanas_[idx_pestana_activa_].resultado);
            }
            ImGui::Separator();
            if (ImGui::MenuItem("Print / Preview...")) mostrar_modal_print_ = true;
            ImGui::EndPopup();
        }

        ImGui::SameLine();
        if (ImGui::Button("Edit  v", ImVec2(68.0f, 24.0f))) ImGui::OpenPopup("##EpHEditMenuPopup");
        if (ImGui::BeginPopup("##EpHEditMenuPopup")) {
            if (ImGui::MenuItem("Copy Summary & Boundaries to Clipboard") && !pestanas_.empty()) {
                const std::string csv = calc_.exportar_tabla_eph_csv(pestanas_[idx_pestana_activa_].resultado);
                ImGui::SetClipboardText(csv.c_str());
            }
            if (ImGui::MenuItem("Select All Species")) {
                std::fill(especies_seleccionadas_.begin(), especies_seleccionadas_.end(), true);
                ejecutar_calculo(false);
            }
            ImGui::EndPopup();
        }

        ImGui::SameLine();
        if (ImGui::Button("Format  v", ImVec2(78.0f, 24.0f))) ImGui::OpenPopup("##EpHFormatMenuPopup");
        if (ImGui::BeginPopup("##EpHFormatMenuPopup")) {
            if (ImGui::MenuItem("H2O Stability Lines...")) mostrar_modal_h2o_ = true;
            if (ImGui::MenuItem("Toggle Area Colors", nullptr, &params_.mostrar_colores)) ejecutar_calculo(false);
            if (ImGui::MenuItem("Show Predominance Areas of Ions", nullptr, &params_.mostrar_areas_iones)) ejecutar_calculo(false);
            ImGui::EndPopup();
        }

        ImGui::SameLine();
        if (ImGui::Button("Help  v", ImVec2(68.0f, 24.0f))) ImGui::OpenPopup("##EpHHelpMenuPopup");
        if (ImGui::BeginPopup("##EpHHelpMenuPopup")) {
            ImGui::TextColored(ImVec4(0.20f, 0.48f, 0.85f, 1.0f), "HSC EpH — Pourbaix Diagrams Module (STABCAL)");
            ImGui::Separator();
            ImGui::BulletText("Select 1 to 7 elements (H and O are automatically included).");
            ImGui::BulletText("Use the element selector next to Draw to switch the Main Element (e.g. Cu vs S).");
            ImGui::BulletText("Double-click any species in Select Species to inspect its thermodynamic data.");
            ImGui::BulletText("Drag or double-click any region label inside the diagram to move or edit it.");
            ImGui::EndPopup();
        }

        ImGui::SameLine(0.0f, 18.0f);
        ImGui::TextDisabled("Quick Presets:");
        ImGui::SameLine(); if (ImGui::SmallButton("Cu-S (Cu)")) cargar_preset(2);
        ImGui::SameLine(); if (ImGui::SmallButton("S-Cu (S)")) cargar_preset(3);
        ImGui::SameLine(); if (ImGui::SmallButton("Fe-H2O")) cargar_preset(0);
        ImGui::SameLine(); if (ImGui::SmallButton("Mn-H2O")) cargar_preset(5);
        ImGui::SameLine(); if (ImGui::SmallButton("Au-Cl")) cargar_preset(4);
        ImGui::SameLine(); if (ImGui::SmallButton("Zn-S")) cargar_preset(6);
    }
    ImGui::EndChild();

    ImGui::Dummy(ImVec2(0.0f, 2.0f));

    const float panel_izq_w = 435.0f;

    // ═══════════════════════════════════════════════════════════════════════════
    // COLUMNA IZQUIERDA: "Diagram Parameters" (Diseño espaciado y sin solapamientos)
    // ═══════════════════════════════════════════════════════════════════════════
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(10.0f, 10.0f));
    ImGui::BeginChild("##EpHLeftPanel", ImVec2(panel_izq_w, 0.0f), true);
    {
        ImGui::TextColored(ImVec4(0.22f, 0.45f, 0.72f, 1.0f), "Diagram Parameters");
        ImGui::Separator();
        ImGui::Dummy(ImVec2(0.0f, 2.0f));

        const float col_el_w = 205.0f;
        const float col_sp_w = panel_izq_w - col_el_w - 30.0f;
        const float top_split_h = 392.0f;

        // Cabeceras: Select Elements | Select Species
        ImGui::TextUnformatted("Select Elements");
        ImGui::SameLine(col_el_w + 12.0f);
        ImGui::TextUnformatted("Select Species");

        // Subcolumna izquierda: Select Elements + Search Mode + Organics
        ImGui::BeginChild("##EpHElemSearchSubCol", ImVec2(col_el_w, top_split_h), false, ImGuiWindowFlags_NoScrollbar);
        {
            ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(4.0f, 3.0f));
            ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(2.0f, 2.0f));
            ImGui::BeginChild("##EpHElementsGrid", ImVec2(col_el_w, 218.0f), true);
            for (int r = 0; r < 21; ++r) {
                for (int c = 0; c < 5; ++c) {
                    const char* sym = kElementosEpHCols[r][c];
                    if (!sym || sym[0] == '\0') continue;
                    if (c > 0) ImGui::SameLine(8.0f + c * 37.0f);

                    const bool es_ho = (std::string(sym) == "H" || std::string(sym) == "O");
                    bool sel = false;
                    for (const auto& el : params_.elementos) {
                        if (el.symbol == sym) { sel = true; break; }
                    }

                    if (sel) {
                        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.22f, 0.50f, 0.86f, 0.95f));
                        ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
                    } else if (es_ho) {
                        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.45f, 0.62f, 0.78f, 0.35f));
                    }

                    std::string btn_id = std::string(sym) + "##eph_el";
                    if (ImGui::Button(btn_id.c_str(), ImVec2(33.5f, 20.0f)) && !es_ho) {
                        auto it = std::find_if(params_.elementos.begin(), params_.elementos.end(),
                                               [&](const domain::EpHElementConfig& e) { return e.symbol == sym; });
                        if (it != params_.elementos.end()) {
                            if (params_.elementos.size() > 1) params_.elementos.erase(it);
                        } else if (params_.elementos.size() < 7) {
                            params_.elementos.push_back({sym, 1.0, 1.0});
                        }
                        if (!params_.elementos.empty()) {
                            bool main_valid = false;
                            for (size_t k = 0; k < params_.elementos.size(); ++k) {
                                if (params_.elementos[k].symbol == params_.elemento_principal) {
                                    main_valid = true;
                                    idx_elem_principal_combo_ = static_cast<int>(k);
                                    break;
                                }
                            }
                            if (!main_valid) {
                                params_.elemento_principal = params_.elementos.front().symbol;
                                idx_elem_principal_combo_ = 0;
                            }
                        }
                        actualizar_especies_sistema(false);
                        ejecutar_calculo(false);
                    }

                    if (sel) ImGui::PopStyleColor(2);
                    else if (es_ho) ImGui::PopStyleColor(1);
                }
            }
            ImGui::EndChild();
            ImGui::PopStyleVar(2);

            ImGui::Dummy(ImVec2(0.0f, 2.0f));
            ImGui::TextUnformatted("Search Mode");
            ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(6.0f, 3.0f));
            ImGui::BeginChild("##EpHSearchModeBox", ImVec2(col_el_w, 118.0f), true);
            {
                bool chg = false;
                chg |= ImGui::Selectable("Gases", &params_.modos_busqueda.gases);
                chg |= ImGui::Selectable("Gas Ions", &params_.modos_busqueda.gas_ions);
                chg |= ImGui::Selectable("Liquids", &params_.modos_busqueda.liquids);
                chg |= ImGui::Selectable("Condensed", &params_.modos_busqueda.condensed);
                chg |= ImGui::Selectable("Aqueous Neutrals", &params_.modos_busqueda.aqueous_neutrals);
                chg |= ImGui::Selectable("Aqueous Ions", &params_.modos_busqueda.aqueous_ions);
                if (chg) {
                    actualizar_especies_sistema(true);
                    ejecutar_calculo(false);
                }
            }
            ImGui::EndChild();
            ImGui::PopStyleVar();

            ImGui::Dummy(ImVec2(0.0f, 2.0f));
            if (ImGui::Checkbox("Organics", &params_.modos_busqueda.organics)) {
                actualizar_especies_sistema(true);
                ejecutar_calculo(false);
            }
        }
        ImGui::EndChild();

        ImGui::SameLine(col_el_w + 12.0f);

        // Subcolumna derecha: Select Species + Select All
        ImGui::BeginChild("##EpHSpeciesSubCol", ImVec2(col_sp_w, top_split_h), false, ImGuiWindowFlags_NoScrollbar);
        {
            ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(6.0f, 4.0f));
            ImGui::BeginChild("##EpHSpeciesList", ImVec2(col_sp_w, top_split_h - 36.0f), true);
            for (size_t i = 0; i < especies_candidatas_.size(); ++i) {
                const auto& sp = especies_candidatas_[i];
                bool sel = especies_seleccionadas_[i];
                if (ImGui::Selectable(sp.formula.c_str(), sel, ImGuiSelectableFlags_AllowDoubleClick)) {
                    if (ImGui::IsMouseDoubleClicked(0)) {
                        especie_modal_ = sp;
                        mostrar_modal_especie_ = true;
                    } else {
                        especies_seleccionadas_[i] = !sel;
                        ejecutar_calculo(false);
                    }
                }
                if (ImGui::IsItemHovered()) {
                    ImGui::SetTooltip("%s\nΔG°(T) = %.3f kcal/mol (%.2f kJ/mol)\nDouble-click for thermodynamic details",
                                      sp.nombre.c_str(), sp.dG0_T_kcal, sp.dG0_T_kJ);
                }
            }
            ImGui::EndChild();
            ImGui::PopStyleVar();

            ImGui::Dummy(ImVec2(0.0f, 2.0f));
            if (ImGui::Button("Select All", ImVec2(col_sp_w, 28.0f))) {
                const bool todas_activas = std::all_of(especies_seleccionadas_.begin(), especies_seleccionadas_.end(),
                                                       [](bool v) { return v; });
                for (size_t i = 0; i < especies_candidatas_.size(); ++i) {
                    especies_seleccionadas_[i] = todas_activas ? especies_candidatas_[i].seleccionada_por_defecto : true;
                }
                ejecutar_calculo(false);
            }
        }
        ImGui::EndChild();

        ImGui::Dummy(ImVec2(0.0f, 4.0f));

        // ── Sección Desplegable "Parameters and Axes" (Con tabla alineada de 2 columnas) ──
        if (ImGui::CollapsingHeader("Parameters and Axes")) {
            ImGui::PushStyleVar(ImGuiStyleVar_CellPadding, ImVec2(6.0f, 4.0f));
            const auto& cur_res = (!pestanas_.empty()) ? pestanas_[idx_pestana_activa_].resultado : calc_.ultimo_resultado_eph();

            if (ImGui::BeginTable("##EpHParamsGrid", 2, ImGuiTableFlags_SizingStretchProp)) {
                ImGui::TableSetupColumn("Label", ImGuiTableColumnFlags_WidthFixed, 175.0f);
                ImGui::TableSetupColumn("Input", ImGuiTableColumnFlags_WidthStretch);

                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::AlignTextToFramePadding(); ImGui::TextUnformatted("Dielectric Constant");
                ImGui::TableSetColumnIndex(1);
                float eps_val = static_cast<float>(params_.override_constante_dielectrica
                                                       ? params_.constante_dielectrica_manual
                                                       : cur_res.constante_dielectrica);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputFloat("##eps_in", &eps_val, 0.0f, 0.0f, "%.3f")) {
                    params_.override_constante_dielectrica = true;
                    params_.constante_dielectrica_manual = eps_val;
                }

                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::AlignTextToFramePadding(); ImGui::TextUnformatted("ΔG of H2O (kcal/mol)");
                ImGui::TableSetColumnIndex(1);
                float dg_h2o_val = static_cast<float>(params_.override_dG_agua
                                                          ? params_.dG_agua_manual_kcal
                                                          : cur_res.dG_H2O_kcal);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputFloat("##dgh2o_in", &dg_h2o_val, 0.0f, 0.0f, "%.3f")) {
                    params_.override_dG_agua = true;
                    params_.dG_agua_manual_kcal = dg_h2o_val;
                }
                ImGui::EndTable();
            }

            if (ImGui::Checkbox("Use Davies Method for Activity", &params_.usar_davies)) {
                actualizar_especies_sistema(true);
                ejecutar_calculo(false);
            }
            ImGui::BeginDisabled(!params_.usar_davies);
            if (ImGui::BeginTable("##EpHDaviesGrid", 2, ImGuiTableFlags_SizingStretchProp)) {
                ImGui::TableSetupColumn("Label", ImGuiTableColumnFlags_WidthFixed, 175.0f);
                ImGui::TableSetupColumn("Input", ImGuiTableColumnFlags_WidthStretch);

                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::AlignTextToFramePadding(); ImGui::TextUnformatted("Ion Strength (mol/kg)");
                ImGui::TableSetColumnIndex(1);
                float i_str = static_cast<float>(params_.fuerza_ionica);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputFloat("##istr_in", &i_str, 0.0f, 0.0f, "%.3f")) {
                    params_.fuerza_ionica = std::max(0.0, static_cast<double>(i_str));
                    actualizar_especies_sistema(true);
                }

                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::AlignTextToFramePadding(); ImGui::TextUnformatted("Correction Factor");
                ImGui::TableSetColumnIndex(1);
                float c_fac = static_cast<float>(cur_res.factor_correccion_davies);
                ImGui::SetNextItemWidth(-1.0f);
                ImGui::InputFloat("##cfac_in", &c_fac, 0.0f, 0.0f, "%.3f", ImGuiInputTextFlags_ReadOnly);
                ImGui::EndTable();
            }
            ImGui::EndDisabled();

            if (ImGui::Checkbox("Criss-Cobble Extrapolation", &params_.usar_criss_cobble)) {
                actualizar_especies_sistema(true);
                ejecutar_calculo(false);
            }

            ImGui::Separator();
            ImGui::TextDisabled("Diagram Axes");
            if (ImGui::BeginTable("##EpHAxesGrid", 4, ImGuiTableFlags_SizingStretchProp)) {
                float eh_min = static_cast<float>(params_.Eh_min);
                float eh_max = static_cast<float>(params_.Eh_max);
                float ph_min = static_cast<float>(params_.pH_min);
                float ph_max = static_cast<float>(params_.pH_max);

                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::AlignTextToFramePadding(); ImGui::TextUnformatted("Min Eh (V)");
                ImGui::TableSetColumnIndex(1); ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputFloat("##ehmin", &eh_min, 0.0f, 0.0f, "%.2f") && (eh_max - eh_min >= 0.2f)) params_.Eh_min = eh_min;
                ImGui::TableSetColumnIndex(2); ImGui::AlignTextToFramePadding(); ImGui::TextUnformatted("Max Eh (V)");
                ImGui::TableSetColumnIndex(3); ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputFloat("##ehmax", &eh_max, 0.0f, 0.0f, "%.2f") && (eh_max - eh_min >= 0.2f)) params_.Eh_max = eh_max;

                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::AlignTextToFramePadding(); ImGui::TextUnformatted("Min pH");
                ImGui::TableSetColumnIndex(1); ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputFloat("##phmin", &ph_min, 0.0f, 0.0f, "%.2f") && (ph_max - ph_min >= 0.2f)) params_.pH_min = ph_min;
                ImGui::TableSetColumnIndex(2); ImGui::AlignTextToFramePadding(); ImGui::TextUnformatted("Max pH");
                ImGui::TableSetColumnIndex(3); ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputFloat("##phmax", &ph_max, 0.0f, 0.0f, "%.2f") && (ph_max - ph_min >= 0.2f)) params_.pH_max = ph_max;
                ImGui::EndTable();
            }

            ImGui::TextDisabled("Potential Scale");
            int sc = static_cast<int>(params_.escala_potencial);
            bool sc_chg = false;
            sc_chg |= ImGui::RadioButton("Hydrogen", &sc, 0); ImGui::SameLine(0.0f, 14.0f);
            sc_chg |= ImGui::RadioButton("Calomel", &sc, 1);  ImGui::SameLine(0.0f, 14.0f);
            sc_chg |= ImGui::RadioButton("Ag / AgCl", &sc, 2);
            if (sc_chg) {
                params_.escala_potencial = static_cast<domain::PourbaixPotentialScale>(sc);
                ejecutar_calculo(false);
            }
            ImGui::PopStyleVar();
            ImGui::Separator();
        }

        // ── Temperatura y Unidad (°C / K) ──
        ImGui::Dummy(ImVec2(0.0f, 2.0f));
        ImGui::AlignTextToFramePadding();
        ImGui::TextUnformatted("Temperature:");
        ImGui::SameLine(115.0f);
        float t_disp = static_cast<float>(params_.temp_en_celsius ? (params_.T_K - 273.15) : params_.T_K);
        ImGui::SetNextItemWidth(175.0f);
        if (ImGui::InputFloat("##EpHTempInput", &t_disp, 0.0f, 0.0f, "%.2f")) {
            params_.T_K = params_.temp_en_celsius ? (t_disp + 273.15) : t_disp;
            params_.T_K = std::clamp(params_.T_K, 273.15, 647.0);
            actualizar_especies_sistema(true);
        }
        ImGui::SameLine(0.0f, 8.0f);
        if (ImGui::Button(params_.temp_en_celsius ? "°C##eph_u" : "K##eph_u", ImVec2(110.0f, 0.0f))) {
            params_.temp_en_celsius = !params_.temp_en_celsius;
        }

        ImGui::Dummy(ImVec2(0.0f, 2.0f));

        // ── Tabla Element | Molality (mol/kg H2O) | Pressure (bar) ──
        ImGui::PushStyleVar(ImGuiStyleVar_CellPadding, ImVec2(8.0f, 5.0f));
        if (ImGui::BeginTable("##EpHElemMolalityTable", 3,
                              ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                              ImVec2(0.0f, 104.0f))) {
            ImGui::TableSetupColumn("Element", ImGuiTableColumnFlags_WidthFixed, 82.0f);
            ImGui::TableSetupColumn("Molality (mol/kg H2O)", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("Pressure (bar)", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableHeadersRow();

            for (size_t i = 0; i < params_.elementos.size(); ++i) {
                auto& el = params_.elementos[i];
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                ImGui::AlignTextToFramePadding();
                ImGui::TextUnformatted(el.symbol.c_str());

                ImGui::TableSetColumnIndex(1);
                ImGui::PushID(static_cast<int>(i * 10 + 1));
                float mol_f = static_cast<float>(el.molality);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputFloat("##mol", &mol_f, 0.0f, 0.0f, "%.2E")) {
                    el.molality = std::max(1e-18, static_cast<double>(mol_f));
                    actualizar_especies_sistema(true);
                }
                ImGui::PopID();

                ImGui::TableSetColumnIndex(2);
                ImGui::PushID(static_cast<int>(i * 10 + 2));
                float p_f = static_cast<float>(el.pressure_bar);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputFloat("##pres", &p_f, 0.0f, 0.0f, "%.2E")) {
                    el.pressure_bar = std::max(0.01, static_cast<double>(p_f));
                    actualizar_especies_sistema(true);
                }
                ImGui::PopID();
            }
            ImGui::EndTable();
        }
        ImGui::PopStyleVar();

        ImGui::Dummy(ImVec2(0.0f, 3.0f));

        // ── Checkboxes inferiores y Botones: Table | Combiner | Draw [Main Element v] ──
        if (ImGui::Checkbox("Show predominance areas of ions", &params_.mostrar_areas_iones)) {
            ejecutar_calculo(false);
        }
        ImGui::Checkbox("Create new page", &create_new_page_);
        ImGui::SameLine(220.0f);
        if (ImGui::Checkbox("Show colors", &params_.mostrar_colores)) {
            ejecutar_calculo(false);
        }

        ImGui::Dummy(ImVec2(0.0f, 4.0f));
        if (ImGui::Button("Table", ImVec2(102.0f, 30.0f))) {
            mostrar_tabla_settings_ = true;
            ejecutar_calculo(false);
        }
        ImGui::SameLine(0.0f, 8.0f);
        if (ImGui::Button("Combiner", ImVec2(106.0f, 30.0f))) {
            mostrar_combiner_ = true;
            datasets_combiner_ = calc_.calcular_combiner_eph(params_, params_.temperaturas_combiner_C);
        }
        ImGui::SameLine(0.0f, 8.0f);
        if (ImGui::Button("Draw", ImVec2(92.0f, 30.0f))) {
            ejecutar_calculo(false);
        }
        ImGui::SameLine(0.0f, 6.0f);
        ImGui::SetNextItemWidth(85.0f);
        if (!params_.elementos.empty()) {
            idx_elem_principal_combo_ = std::clamp(idx_elem_principal_combo_, 0, static_cast<int>(params_.elementos.size()) - 1);
            const char* cur_main = params_.elementos[idx_elem_principal_combo_].symbol.c_str();
            if (ImGui::BeginCombo("##EpHMainElemCombo", cur_main)) {
                for (int i = 0; i < static_cast<int>(params_.elementos.size()); ++i) {
                    const bool is_sel = (idx_elem_principal_combo_ == i);
                    if (ImGui::Selectable(params_.elementos[i].symbol.c_str(), is_sel)) {
                        idx_elem_principal_combo_ = i;
                        params_.elemento_principal = params_.elementos[i].symbol;
                        ejecutar_calculo(false);
                        if (mostrar_combiner_) {
                            datasets_combiner_ = calc_.calcular_combiner_eph(params_, params_.temperaturas_combiner_C);
                        }
                    }
                    if (is_sel) ImGui::SetItemDefaultFocus();
                }
                ImGui::EndCombo();
            }
        }

        if (!estado_msg_.empty()) {
            ImGui::Dummy(ImVec2(0.0f, 2.0f));
            ImGui::TextDisabled("%s", estado_msg_.c_str());
        }
    }
    ImGui::EndChild();
    ImGui::PopStyleVar(); // WindowPadding

    ImGui::SameLine(0.0f, 10.0f);

    // ═══════════════════════════════════════════════════════════════════════════
    // COLUMNA DERECHA: Pestañas de Diagramas (Chart1, Table Basic Settings, Combiner)
    // ═══════════════════════════════════════════════════════════════════════════
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(12.0f, 10.0f));
    ImGui::BeginChild("##EpHRightPanel", ImVec2(0.0f, 0.0f), true);
    {
        if (ImGui::BeginTabBar("##EpHTabBar", ImGuiTabBarFlags_Reorderable | ImGuiTabBarFlags_AutoSelectNewTabs)) {
            for (int t = 0; t < static_cast<int>(pestanas_.size()); ++t) {
                auto& tab = pestanas_[t];
                bool open = true;
                std::string tab_lbl = tab.nombre + "##eph_tab_" + std::to_string(t);
                if (ImGui::BeginTabItem(tab_lbl.c_str(), (pestanas_.size() > 1) ? &open : nullptr)) {
                    idx_pestana_activa_ = t;
                    const auto& res = tab.resultado;

                    ImGui::Dummy(ImVec2(0.0f, 2.0f));

                    // Barra vertical izquierda de iconos rápidos con espaciado generoso
                    ImGui::BeginChild("##EpHQuickToolbar", ImVec2(42.0f, -86.0f), false);
                    {
                        ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(4.0f, 8.0f));
                        if (ImGui::Button("S##sv", ImVec2(34.0f, 30.0f))) {
                            std::ofstream ofs(ruta_eph8_);
                            if (ofs) ofs << calc_.serializar_eph8(tab.params);
                        }
                        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Save .eph8 Specification");

                        if (ImGui::Button("C##cp", ImVec2(34.0f, 30.0f))) {
                            const std::string csv = calc_.exportar_tabla_eph_csv(res);
                            ImGui::SetClipboardText(csv.c_str());
                        }
                        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Copy Diagram Data to Clipboard");

                        if (ImGui::Button("P##pr", ImVec2(34.0f, 30.0f))) {
                            mostrar_modal_print_ = true;
                        }
                        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Print Settings / Preview (Fig. 6)");

                        if (ImGui::Button("F##cl", ImVec2(34.0f, 30.0f))) {
                            params_.mostrar_colores = !params_.mostrar_colores;
                            tab.params.mostrar_colores = params_.mostrar_colores;
                        }
                        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Toggle Area Fill Colors");

                        if (ImGui::Button("R##rs", ImVec2(34.0f, 30.0f))) {
                            tab.reset_limites = true;
                        }
                        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Reset Axis Zoom");

                        if (ImGui::Button("A##lb", ImVec2(34.0f, 30.0f))) {
                            mostrar_modal_h2o_ = true;
                        }
                        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Format H2O Stability Lines & Labels");
                        ImGui::PopStyleVar();
                    }
                    ImGui::EndChild();

                    ImGui::SameLine(0.0f, 8.0f);

                    // Lienzo principal del gráfico Eh-pH
                    ImGui::BeginChild("##EpHPlotArea", ImVec2(0.0f, -86.0f), false);
                    {
                        const char* y_label = (res.escala_potencial == domain::PourbaixPotentialScale::SCE)
                                                  ? "E vs SCE (Volts)"
                                              : (res.escala_potencial == domain::PourbaixPotentialScale::AgAgCl)
                                                  ? "E vs Ag/AgCl (Volts)"
                                                  : "Eh (Volts)";

                        ImPlot::PushStyleColor(ImPlotCol_PlotBg, ImVec4(0.985f, 0.988f, 0.992f, 1.0f));
                        ImPlot::PushStyleColor(ImPlotCol_AxisGrid, ImVec4(0.75f, 0.78f, 0.82f, 0.55f));

                        std::string plot_title = res.titulo_diagrama + "##plot_" + std::to_string(t);
                        if (ImPlot::BeginPlot(plot_title.c_str(), ImVec2(-1.0f, -1.0f), ImPlotFlags_NoLegend)) {
                            ImPlot::SetupAxes("pH", y_label, ImPlotAxisFlags_None, ImPlotAxisFlags_None);
                            if (tab.reset_limites) {
                                ImPlot::SetupAxesLimits(res.pH_min, res.pH_max, res.Eh_min, res.Eh_max, ImGuiCond_Always);
                                tab.reset_limites = false;
                            } else {
                                ImPlot::SetupAxesLimits(res.pH_min, res.pH_max, res.Eh_min, res.Eh_max, ImGuiCond_Once);
                            }

                            ImDrawList* dl = ImPlot::GetPlotDrawList();
                            ImPlot::PushPlotClipRect();

                            // 1. Pintar áreas de predominancia si Show colors está activo
                            if (tab.params.mostrar_colores) {
                                for (size_t k = 0; k < res.poligonos.size(); ++k) {
                                    const auto& pol = res.poligonos[k];
                                    if (pol.vertices.size() < 3) continue;
                                    const ImU32 fill_col = kPaletaEpH[k % kNumColoresEpH].fill;
                                    const ImVec2 p0 = ImPlot::PlotToPixels(pol.vertices[0].pH, pol.vertices[0].Eh);
                                    for (size_t i = 1; i + 1 < pol.vertices.size(); ++i) {
                                        const ImVec2 p1 = ImPlot::PlotToPixels(pol.vertices[i].pH, pol.vertices[i].Eh);
                                        const ImVec2 p2 = ImPlot::PlotToPixels(pol.vertices[i + 1].pH, pol.vertices[i + 1].Eh);
                                        dl->AddTriangleFilled(p0, p1, p2, fill_col);
                                    }
                                }
                            }

                            // 2. Líneas sólidas negras de equilibrio de fases (Imagen 3 e Imagen 4)
                            for (const auto& f : res.fronteras) {
                                const ImVec2 p1 = ImPlot::PlotToPixels(f.p1.pH, f.p1.Eh);
                                const ImVec2 p2 = ImPlot::PlotToPixels(f.p2.pH, f.p2.Eh);
                                dl->AddLine(p1, p2, IM_COL32(20, 22, 26, 255), 1.45f);
                            }

                            // 3. Líneas punteadas negras de estabilidad del agua (O2/H2O superior y H+/H2 inferior)
                            const auto& st_h2o = estilo_agua_global();
                            const ImU32 col_h2o = ImGui::ColorConvertFloat4ToU32(st_h2o.color);
                            if (st_h2o.mostrar_superior) {
                                const ImVec2 w1 = ImPlot::PlotToPixels(res.pH_min, res.eh_agua_sup_pHmin);
                                const ImVec2 w2 = ImPlot::PlotToPixels(res.pH_max, res.eh_agua_sup_pHmax);
                                dibujar_segmento_punteado(dl, w1, w2, col_h2o, st_h2o.grosor, 4.0f, 4.0f);
                            }
                            if (st_h2o.mostrar_inferior) {
                                const ImVec2 w1 = ImPlot::PlotToPixels(res.pH_min, res.eh_agua_inf_pHmin);
                                const ImVec2 w2 = ImPlot::PlotToPixels(res.pH_max, res.eh_agua_inf_pHmax);
                                dibujar_segmento_punteado(dl, w1, w2, col_h2o, st_h2o.grosor, 4.0f, 4.0f);
                            }

                            // 4. Superposición azul punteada de "Show predominance areas of ions" (Sección 17.3.7)
                            if (tab.params.mostrar_areas_iones) {
                                for (const auto& fi : res.fronteras_iones) {
                                    const ImVec2 p1 = ImPlot::PlotToPixels(fi.p1.pH, fi.p1.Eh);
                                    const ImVec2 p2 = ImPlot::PlotToPixels(fi.p2.pH, fi.p2.Eh);
                                    dibujar_segmento_punteado(dl, p1, p2, IM_COL32(25, 95, 215, 240), 1.5f, 3.5f, 3.5f);
                                }
                            }

                            // 5. Etiquetas de áreas de estabilidad ubicadas en el punto más ancho (arrastrables y editables)
                            for (const auto& pol : res.poligonos) {
                                std::string txt = pol.etiqueta;
                                if (tab.etiquetas_custom.count(pol.formula)) {
                                    txt = tab.etiquetas_custom.at(pol.formula);
                                }
                                if (txt.empty()) continue; // Etiqueta ocultada por el usuario

                                ImVec2 coord(static_cast<float>(pol.centroide.pH), static_cast<float>(pol.centroide.Eh));
                                if (tab.pos_etiquetas.count(pol.formula)) {
                                    coord = tab.pos_etiquetas.at(pol.formula);
                                }

                                const ImVec2 px = ImPlot::PlotToPixels(coord.x, coord.y);
                                const ImVec2 tsz = ImGui::CalcTextSize(txt.c_str());
                                const ImVec2 pos_txt(px.x - 0.5f * tsz.x, px.y - 0.5f * tsz.y);

                                dl->AddText(ImVec2(pos_txt.x + 1.0f, pos_txt.y + 1.0f), IM_COL32(255, 255, 255, 220), txt.c_str());
                                dl->AddText(pos_txt, IM_COL32(15, 18, 22, 255), txt.c_str());

                                // Interacción con el ratón sobre la etiqueta (doble clic para editar, arrastrar para mover)
                                const ImVec2 mouse = ImGui::GetMousePos();
                                if (mouse.x >= pos_txt.x - 4.0f && mouse.x <= pos_txt.x + tsz.x + 4.0f &&
                                    mouse.y >= pos_txt.y - 3.0f && mouse.y <= pos_txt.y + tsz.y + 3.0f) {
                                    if (ImGui::IsMouseDoubleClicked(0)) {
                                        label_edit_key_ = pol.formula;
                                        std::snprintf(buf_label_edit_, sizeof(buf_label_edit_), "%s", txt.c_str());
                                        mostrar_modal_label_ = true;
                                    } else if (ImGui::IsMouseDragging(0)) {
                                        const ImPlotPoint mp = ImPlot::GetPlotMousePos();
                                        tab.pos_etiquetas[pol.formula] = ImVec2(static_cast<float>(mp.x), static_cast<float>(mp.y));
                                    }
                                }
                            }

                            ImPlot::PopPlotClipRect();

                            // Sonda electroquímica en tiempo real al pasar el cursor
                            if (ImPlot::IsPlotHovered()) {
                                const ImPlotPoint mp = ImPlot::GetPlotMousePos();
                                auto ev = calc_.evaluar_punto(mp.x, mp.y);
                                if (ev.valid) {
                                    const char* st_str = (ev.corrosion_state == domain::PourbaixCorrosionState::Immunity)
                                                             ? "Immunity"
                                                         : (ev.corrosion_state == domain::PourbaixCorrosionState::Corrosion)
                                                             ? "Active Corrosion (Aqueous)"
                                                             : "Passivation (Condensed)";
                                    ImGui::BeginTooltip();
                                    ImGui::Text("pH = %.2f | Eh = %+.3f V", mp.x, mp.y);
                                    ImGui::Text("Predominant Phase: %s (%s)", ev.predominant_formula.c_str(), st_str);
                                    ImGui::Text("Water Window: %s", ev.water_stable ? "Stable H2O(l)"
                                                                   : (ev.o2_evolution ? "O2(g) Evolution" : "H2(g) Evolution"));
                                    ImGui::EndTooltip();
                                }
                            }

                            ImPlot::EndPlot();
                        }
                        ImPlot::PopStyleColor(2);
                    }
                    ImGui::EndChild();

                    // ── Caja resumen inferior centrada bajo el eje pH: Element | Molality | Pressure ──
                    ImGui::Dummy(ImVec2(0.0f, 4.0f));
                    const float avail_w = ImGui::GetContentRegionAvail().x;
                    const float box_w = 340.0f;
                    if (avail_w > box_w) ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 0.5f * (avail_w - box_w));
                    ImGui::PushStyleVar(ImGuiStyleVar_CellPadding, ImVec2(10.0f, 5.0f));
                    if (ImGui::BeginTable("##EpHBottomSummaryBox", 3,
                                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_SizingFixedFit,
                                          ImVec2(box_w, 68.0f))) {
                        ImGui::TableSetupColumn("Element", ImGuiTableColumnFlags_WidthFixed, 82.0f);
                        ImGui::TableSetupColumn("Molality (mol/kg)", ImGuiTableColumnFlags_WidthFixed, 125.0f);
                        ImGui::TableSetupColumn("Pressure (bar)", ImGuiTableColumnFlags_WidthFixed, 125.0f);
                        ImGui::TableHeadersRow();
                        for (const auto& el : res.elementos_usados) {
                            ImGui::TableNextRow();
                            ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(el.symbol.c_str());
                            ImGui::TableSetColumnIndex(1); ImGui::Text("%.2E", el.molality);
                            ImGui::TableSetColumnIndex(2); ImGui::Text("%.2E", el.pressure_bar);
                        }
                        ImGui::EndTable();
                    }
                    ImGui::PopStyleVar();

                    ImGui::EndTabItem();
                }
                if (!open && pestanas_.size() > 1) {
                    pestanas_.erase(pestanas_.begin() + t);
                    idx_pestana_activa_ = std::clamp(idx_pestana_activa_, 0, static_cast<int>(pestanas_.size()) - 1);
                    break;
                }
            }

            // ── Pestaña "E-pH Diagram Basic Settings" (Botón Table — Columnas amplias y legibles) ──
            if (mostrar_tabla_settings_) {
                if (ImGui::BeginTabItem("E-pH Diagram Basic Settings (Table)", &mostrar_tabla_settings_)) {
                    const auto& res = (!pestanas_.empty()) ? pestanas_[idx_pestana_activa_].resultado : calc_.ultimo_resultado_eph();

                    ImGui::Dummy(ImVec2(0.0f, 2.0f));
                    ImGui::AlignTextToFramePadding();
                    ImGui::TextColored(ImVec4(0.15f, 0.38f, 0.68f, 1.0f), "%s", res.titulo_diagrama.c_str());
                    ImGui::SameLine(ImGui::GetContentRegionAvail().x - 190.0f);
                    if (ImGui::Button("Copy All to Clipboard", ImVec2(180.0f, 26.0f))) {
                        const std::string csv = calc_.exportar_tabla_eph_csv(res);
                        ImGui::SetClipboardText(csv.c_str());
                    }
                    ImGui::Dummy(ImVec2(0.0f, 4.0f));

                    ImGui::PushStyleVar(ImGuiStyleVar_CellPadding, ImVec2(10.0f, 6.0f));
                    if (ImGui::BeginTable("##EpHBasicSettingsSheet", 8,
                                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                          ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable,
                                          ImVec2(0.0f, 0.0f))) {
                        ImGui::TableSetupColumn("#", ImGuiTableColumnFlags_WidthFixed, 34.0f);
                        ImGui::TableSetupColumn("A (Species / Parameter)", ImGuiTableColumnFlags_WidthFixed, 175.0f);
                        ImGui::TableSetupColumn("B (DeltaG kcal/mol)", ImGuiTableColumnFlags_WidthFixed, 155.0f);
                        ImGui::TableSetupColumn("C (Area no / P)", ImGuiTableColumnFlags_WidthFixed, 105.0f);
                        ImGui::TableSetupColumn("D (X1 pH)", ImGuiTableColumnFlags_WidthFixed, 85.0f);
                        ImGui::TableSetupColumn("E (Y1 V / Setting)", ImGuiTableColumnFlags_WidthFixed, 165.0f);
                        ImGui::TableSetupColumn("F (X2 pH / Value)", ImGuiTableColumnFlags_WidthFixed, 110.0f);
                        ImGui::TableSetupColumn("G (Y2 V / Unit)", ImGuiTableColumnFlags_WidthStretch);
                        ImGui::TableHeadersRow();

                        auto add_sheet_row = [&](int row_num, const char* colA, const char* colB, const char* colC,
                                                 const char* colD, const char* colE, const char* colF, const char* colG) {
                            ImGui::TableNextRow();
                            ImGui::TableSetColumnIndex(0); ImGui::TextDisabled("%d", row_num);
                            ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(colA);
                            ImGui::TableSetColumnIndex(2); ImGui::TextUnformatted(colB);
                            ImGui::TableSetColumnIndex(3); ImGui::TextUnformatted(colC);
                            ImGui::TableSetColumnIndex(4); ImGui::TextUnformatted(colD);
                            ImGui::TableSetColumnIndex(5); ImGui::TextUnformatted(colE);
                            ImGui::TableSetColumnIndex(6); ImGui::TextUnformatted(colF);
                            ImGui::TableSetColumnIndex(7); ImGui::TextUnformatted(colG);
                        };

                        char b_temp[32], b_eps[32], b_dgw[32], b_is[32], b_cf[32];
                        char b_ehmin[32], b_ehmax[32], b_phmin[32], b_phmax[32];
                        std::snprintf(b_temp, sizeof(b_temp), "%.3f", res.T_C);
                        std::snprintf(b_eps, sizeof(b_eps), "%.3f", res.constante_dielectrica);
                        std::snprintf(b_dgw, sizeof(b_dgw), "%.3f", res.dG_H2O_kcal);
                        std::snprintf(b_is, sizeof(b_is), "%.3f", res.fuerza_ionica);
                        std::snprintf(b_cf, sizeof(b_cf), "%.3f", res.factor_correccion_davies);
                        std::snprintf(b_ehmin, sizeof(b_ehmin), "%.3f", res.Eh_min);
                        std::snprintf(b_ehmax, sizeof(b_ehmax), "%.3f", res.Eh_max);
                        std::snprintf(b_phmin, sizeof(b_phmin), "%.3f", res.pH_min);
                        std::snprintf(b_phmax, sizeof(b_phmax), "%.3f", res.pH_max);

                        add_sheet_row(1, res.titulo_diagrama.c_str(), "", "", "", "", "", "");
                        add_sheet_row(2, "Elements", "Molality", "Pressure", "", "Temperature:", b_temp, "°C");

                        auto get_el_str = [&](size_t idx, char* m_buf, char* p_buf) -> const char* {
                            if (idx >= res.elementos_usados.size()) { m_buf[0] = '\0'; p_buf[0] = '\0'; return ""; }
                            std::snprintf(m_buf, 32, "%.2E", res.elementos_usados[idx].molality);
                            std::snprintf(p_buf, 32, "%.2E", res.elementos_usados[idx].pressure_bar);
                            return res.elementos_usados[idx].symbol.c_str();
                        };
                        char m0[32], p0[32], m1[32], p1[32];
                        const char* s0 = get_el_str(0, m0, p0);
                        const char* s1 = get_el_str(1, m1, p1);

                        add_sheet_row(3, s0, m0, p0, "", "Show Colors:", params_.mostrar_colores ? "TRUE" : "FALSE", "");
                        add_sheet_row(4, s1, m1, p1, "", "Dielectric Constant:", b_eps, "");
                        add_sheet_row(5, "", "", "", "", "DeltaG of H2O:", b_dgw, "kcal/mol");
                        add_sheet_row(6, "", "", "", "", "Davies Method:", params_.usar_davies ? "TRUE" : "FALSE", "");
                        add_sheet_row(7, "", "", "", "", "Ion Strength:", b_is, "mol/kg H2O");
                        add_sheet_row(8, "", "", "", "", "Correction Factor:", b_cf, "");
                        add_sheet_row(9, "", "", "", "", "Criss Cobble:", params_.usar_criss_cobble ? "TRUE" : "FALSE", "");
                        add_sheet_row(10, "", "", "", "", "Min Eh:", b_ehmin, "V");
                        add_sheet_row(11, "", "", "", "", "Max Eh:", b_ehmax, "V");
                        add_sheet_row(12, "", "", "", "", "Min pH:", b_phmin, "");
                        add_sheet_row(13, "Potential Scale:", "1.000", "", "", "Max pH:", b_phmax, "");
                        add_sheet_row(14, "Search modes:", "Condensed;Aqueous Ions", "", "",
                                      "Show Predominance Ions:", params_.mostrar_areas_iones ? "TRUE" : "FALSE", "");
                        add_sheet_row(16, "Species", "DeltaG kcal/mol", "Area no", "X1 pH", "Y1 V", "X2 pH", "Y2 V");

                        int row_idx = 17;
                        for (size_t i = 0; i < res.especies_tabla.size(); ++i) {
                            const auto& sp = res.especies_tabla[i];
                            char dg_buf[32], area_buf[32] = "", x1_b[32] = "", y1_b[32] = "", x2_b[32] = "", y2_b[32] = "";
                            std::snprintf(dg_buf, sizeof(dg_buf), "%.3f", sp.dG0_T_kcal);
                            if (i < res.fronteras.size()) {
                                const auto& fr = res.fronteras[i];
                                std::snprintf(area_buf, sizeof(area_buf), "%d", fr.area_no);
                                std::snprintf(x1_b, sizeof(x1_b), "%.2f", fr.p1.pH);
                                std::snprintf(y1_b, sizeof(y1_b), "%.3f", fr.p1.Eh);
                                std::snprintf(x2_b, sizeof(x2_b), "%.2f", fr.p2.pH);
                                std::snprintf(y2_b, sizeof(y2_b), "%.3f", fr.p2.Eh);
                            }
                            add_sheet_row(row_idx++, sp.formula.c_str(), dg_buf, area_buf, x1_b, y1_b, x2_b, y2_b);
                        }

                        ImGui::EndTable();
                    }
                    ImGui::PopStyleVar();
                    ImGui::EndTabItem();
                }
            }

            // ── Pestaña "Combined E-pH Diagrams (Combiner)" (Sección 17.5 / Fig. 7) ──
            if (mostrar_combiner_) {
                if (ImGui::BeginTabItem("Combined E-pH Diagram (Combiner)", &mostrar_combiner_)) {
                    ImGui::Dummy(ImVec2(0.0f, 3.0f));
                    ImGui::AlignTextToFramePadding();
                    ImGui::TextUnformatted("Isotherm Datasets (°C):");
                    for (size_t d = 0; d < params_.temperaturas_combiner_C.size(); ++d) {
                        ImGui::SameLine(0.0f, 10.0f);
                        ImGui::PushID(static_cast<int>(d + 500));
                        float tc = static_cast<float>(params_.temperaturas_combiner_C[d]);
                        ImGui::SetNextItemWidth(82.0f);
                        if (ImGui::InputFloat("##tc", &tc, 0.0f, 0.0f, "%.0f °C")) {
                            params_.temperaturas_combiner_C[d] = std::clamp(static_cast<double>(tc), 0.0, 350.0);
                        }
                        ImGui::PopID();
                    }
                    ImGui::SameLine(0.0f, 14.0f);
                    if (ImGui::Button("Draw Combiner", ImVec2(130.0f, 28.0f))) {
                        datasets_combiner_ = calc_.calcular_combiner_eph(params_, params_.temperaturas_combiner_C);
                    }
                    ImGui::Dummy(ImVec2(0.0f, 4.0f));

                    const ImVec4 comb_cols[] = {
                        ImVec4(0.10f, 0.12f, 0.15f, 1.0f), // 25 °C (Negro)
                        ImVec4(0.15f, 0.45f, 0.85f, 1.0f), // 50 °C (Azul)
                        ImVec4(0.85f, 0.22f, 0.18f, 1.0f), // 75 °C (Rojo)
                        ImVec4(0.15f, 0.65f, 0.32f, 1.0f)  // 100 °C (Verde)
                    };

                    if (ImPlot::BeginPlot("Combined E-pH Diagram (Isotherm Overlay: 25, 50, 75, 100 °C)##comb", ImVec2(-1.0f, -1.0f))) {
                        ImPlot::SetupAxes("pH", "Eh (Volts)");
                        ImPlot::SetupAxesLimits(params_.pH_min, params_.pH_max, params_.Eh_min, params_.Eh_max, ImGuiCond_Once);

                        for (size_t d = 0; d < datasets_combiner_.size(); ++d) {
                            const auto& ds = datasets_combiner_[d];
                            const ImVec4 col = comb_cols[d % 4];
                            for (size_t f = 0; f < ds.resultado.fronteras.size(); ++f) {
                                const auto& fr = ds.resultado.fronteras[f];
                                double xs[2] = {fr.p1.pH, fr.p2.pH};
                                double ys[2] = {fr.p1.Eh, fr.p2.Eh};
                                ImPlot::SetNextLineStyle(col, 1.5f);
                                std::string lbl = (f == 0) ? ds.etiqueta : ("##comb_" + std::to_string(d) + "_" + std::to_string(f));
                                ImPlot::PlotLine(lbl.c_str(), xs, ys, 2);
                            }
                            if (d == 0) {
                                for (const auto& pol : ds.resultado.poligonos) {
                                    ImPlot::PlotText(pol.formula.c_str(), pol.centroide.pH, pol.centroide.Eh);
                                }
                            }
                        }
                        ImPlot::EndPlot();
                    }
                    ImGui::EndTabItem();
                }
            }

            ImGui::EndTabBar();
        }
    }
    ImGui::EndChild();
    ImGui::PopStyleVar(); // WindowPadding de EpHRightPanel

    // ═══════════════════════════════════════════════════════════════════════════
    // MODALES: Información de Especie, Formato Líneas H2O, Etiquetas e Impresión
    // ═══════════════════════════════════════════════════════════════════════════
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(16.0f, 14.0f));
    ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(10.0f, 8.0f));

    if (mostrar_modal_especie_) {
        ImGui::OpenPopup("EpH Species Thermodynamic Data");
        mostrar_modal_especie_ = false;
    }
    if (ImGui::BeginPopupModal("EpH Species Thermodynamic Data", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::TextColored(ImVec4(0.20f, 0.48f, 0.85f, 1.0f), "%s — %s",
                           especie_modal_.formula.c_str(), especie_modal_.nombre.c_str());
        ImGui::Separator();
        ImGui::Text("Ionic Charge (z):      %+d", especie_modal_.carga);
        ImGui::Text("ΔGf°(T):               %.3f kcal/mol  (%.3f kJ/mol)", especie_modal_.dG0_T_kcal, especie_modal_.dG0_T_kJ);
        ImGui::Text("ΔHf°(298.15 K):        %.3f kJ/mol", especie_modal_.h_298_kJ);
        ImGui::Text("S°(298.15 K):          %.3f J/(mol·K)", especie_modal_.s_298_J_K);
        ImGui::Text("Cp°(298.15 K):         %.3f J/(mol·K)", especie_modal_.cp_298_J_K);
        ImGui::Separator();
        if (ImGui::Button("Close", ImVec2(130.0f, 28.0f))) ImGui::CloseCurrentPopup();
        ImGui::EndPopup();
    }

    if (mostrar_modal_h2o_) {
        ImGui::OpenPopup("Format H2O Stability Lines");
        mostrar_modal_h2o_ = false;
    }
    if (ImGui::BeginPopupModal("Format H2O Stability Lines", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        auto& st = estilo_agua_global();
        ImGui::Checkbox("Show Upper O2(g)/H2O Stability Line", &st.mostrar_superior);
        ImGui::Checkbox("Show Lower H(+a)/H2(g) Stability Line", &st.mostrar_inferior);
        ImGui::SliderFloat("Line Width", &st.grosor, 1.0f, 3.5f, "%.1f px");
        ImGui::ColorEdit4("Line Color", &st.color.x);
        ImGui::Separator();
        if (ImGui::Button("OK", ImVec2(130.0f, 28.0f))) ImGui::CloseCurrentPopup();
        ImGui::EndPopup();
    }

    if (mostrar_modal_label_) {
        ImGui::OpenPopup("Label Format");
        mostrar_modal_label_ = false;
    }
    if (ImGui::BeginPopupModal("Label Format", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::Text("Species Area: %s", label_edit_key_.c_str());
        ImGui::SetNextItemWidth(240.0f);
        ImGui::InputText("Label Text (clear to hide)", buf_label_edit_, sizeof(buf_label_edit_));
        ImGui::Separator();
        if (ImGui::Button("Apply", ImVec2(110.0f, 28.0f))) {
            if (!pestanas_.empty()) {
                pestanas_[idx_pestana_activa_].etiquetas_custom[label_edit_key_] = buf_label_edit_;
            }
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button("Reset Position", ImVec2(130.0f, 28.0f))) {
            if (!pestanas_.empty()) {
                pestanas_[idx_pestana_activa_].pos_etiquetas.erase(label_edit_key_);
                pestanas_[idx_pestana_activa_].etiquetas_custom.erase(label_edit_key_);
            }
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button("Cancel", ImVec2(100.0f, 28.0f))) ImGui::CloseCurrentPopup();
        ImGui::EndPopup();
    }

    // Modal "Print Settings/Preview" (Réplica fiel y espaciada de Fig. 6 en Imagen 5)
    if (mostrar_modal_print_) {
        ImGui::OpenPopup("Print Settings/Preview");
        mostrar_modal_print_ = false;
    }
    if (ImGui::BeginPopupModal("Print Settings/Preview", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        static int orientacion = 0; // 0 = Portrait, 1 = Landscape
        static int margenes[4] = {25, 25, 25, 25};
        static bool keep_aspect = true, bg_fill = true, graph_fill = true;

        ImGui::BeginChild("##PrintLeftControls", ImVec2(245.0f, 360.0f), true);
        {
            ImGui::TextColored(ImVec4(0.20f, 0.48f, 0.85f, 1.0f), "Orientation");
            ImGui::RadioButton("Portrait", &orientacion, 0);
            ImGui::RadioButton("Landscape", &orientacion, 1);
            ImGui::Separator();
            ImGui::TextColored(ImVec4(0.20f, 0.48f, 0.85f, 1.0f), "Margins (mm)");
            ImGui::SetNextItemWidth(90.0f); ImGui::InputInt("Top", &margenes[0]);
            ImGui::SetNextItemWidth(90.0f); ImGui::InputInt("Bottom", &margenes[1]);
            ImGui::SetNextItemWidth(90.0f); ImGui::InputInt("Left", &margenes[2]);
            ImGui::SetNextItemWidth(90.0f); ImGui::InputInt("Right", &margenes[3]);
            ImGui::Separator();
            ImGui::TextColored(ImVec4(0.20f, 0.48f, 0.85f, 1.0f), "Content Options");
            ImGui::Checkbox("Keep aspect ratio", &keep_aspect);
            ImGui::Checkbox("Use background fill", &bg_fill);
            ImGui::Checkbox("Use graph fill", &graph_fill);
        }
        ImGui::EndChild();

        ImGui::SameLine(0.0f, 14.0f);

        // Vista previa vectorial de la hoja impresa (Fig. 6)
        ImGui::BeginChild("##PrintPagePreview", ImVec2(310.0f, 360.0f), true);
        {
            ImDrawList* pdl = ImGui::GetWindowDrawList();
            const ImVec2 p0 = ImGui::GetCursorScreenPos();
            const ImVec2 sz(280.0f, 332.0f);
            pdl->AddRectFilled(p0, ImVec2(p0.x + sz.x, p0.y + sz.y), IM_COL32(252, 252, 252, 255));
            pdl->AddRect(p0, ImVec2(p0.x + sz.x, p0.y + sz.y), IM_COL32(80, 80, 80, 255));

            const ImVec2 g0(p0.x + 32.0f, p0.y + 42.0f);
            const ImVec2 g1(p0.x + sz.x - 28.0f, p0.y + sz.y - 56.0f);
            pdl->AddRect(g0, g1, IM_COL32(20, 20, 20, 255));

            if (!pestanas_.empty()) {
                const auto& r = pestanas_[idx_pestana_activa_].resultado;
                pdl->AddText(ImVec2(g0.x, p0.y + 18.0f), IM_COL32(10, 10, 10, 255), r.titulo_diagrama.c_str());
                for (const auto& f : r.fronteras) {
                    const float x1 = g0.x + static_cast<float>((f.p1.pH - r.pH_min) / (r.pH_max - r.pH_min)) * (g1.x - g0.x);
                    const float y1 = g1.y - static_cast<float>((f.p1.Eh - r.Eh_min) / (r.Eh_max - r.Eh_min)) * (g1.y - g0.y);
                    const float x2 = g0.x + static_cast<float>((f.p2.pH - r.pH_min) / (r.pH_max - r.pH_min)) * (g1.x - g0.x);
                    const float y2 = g1.y - static_cast<float>((f.p2.Eh - r.Eh_min) / (r.Eh_max - r.Eh_min)) * (g1.y - g0.y);
                    pdl->AddLine(ImVec2(x1, y1), ImVec2(x2, y2), IM_COL32(20, 20, 20, 255), 1.0f);
                }
            }
        }
        ImGui::EndChild();

        ImGui::Dummy(ImVec2(0.0f, 4.0f));
        if (ImGui::Button("Export CSV / Print", ImVec2(160.0f, 30.0f))) {
            if (!pestanas_.empty()) {
                std::ofstream ofs("eph_print_export.csv");
                if (ofs) ofs << calc_.exportar_tabla_eph_csv(pestanas_[idx_pestana_activa_].resultado);
                estado_msg_ = "Reporte de impresión exportado a eph_print_export.csv";
            }
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine(0.0f, 12.0f);
        if (ImGui::Button("Close", ImVec2(115.0f, 30.0f))) ImGui::CloseCurrentPopup();
        ImGui::EndPopup();
    }

    ImGui::PopStyleVar(2); // Modales WindowPadding + ItemSpacing
    ImGui::PopStyleVar(2); // Top-level ItemSpacing + FramePadding
}

#pragma GCC diagnostic pop
