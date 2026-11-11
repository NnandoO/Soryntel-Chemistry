#include "paneles.hpp"
#include <implot.h>
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <sstream>

namespace cma { namespace interfaz {

namespace {

static const char* kGridLpp22[22][4] = {
    {"Ac", "Dy", "Mn", "Ru"},
    {"Ag", "Er", "Mo", "S"},
    {"Al", "Es", "N",  "Sb"},
    {"Am", "Eu", "Na", "Sc"},
    {"Ar", "F",  "Nb", "Se"},
    {"As", "Fe", "Nd", "Si"},
    {"At", "Fm", "Ne", "Sm"},
    {"Au", "Fr", "Ni", "Sn"},
    {"B",  "Ga", "Np", "Sr"},
    {"Ba", "Gd", "O",  "Ta"},
    {"Be", "Ge", "Os", "Tb"},
    {"Bi", "H",  "P",  "Tc"},
    {"Bk", "He", "Pa", "Te"},
    {"Br", "Hf", "Pb", "Th"},
    {"C",  "Hg", "Pd", "Ti"},
    {"Ca", "Ho", "Pm", "Tl"},
    {"Cd", "I",  "Po", "Tm"},
    {"Ce", "In", "Pr", "U"},
    {"Cf", "Ir", "Pt", "V"},
    {"Cl", "K",  "Pu", "W"},
    {"Cm", "Kr", "Ra", "Xe"},
    {"Co", "La", "Rb", "Y"}
};
static const char* kGridLppExtra[3][4] = {
    {"Cr", "Li", "Re", "Yb"},
    {"Cs", "Lu", "Rh", "Zn"},
    {"Cu", "Mg", "Rn", "Zr"}
};

ImU32 color_lpp_relleno(size_t idx) {
    static const ImU32 kColores[] = {
        IM_COL32(38, 128, 255, 195),
        IM_COL32(0, 179, 158, 195),
        IM_COL32(114, 184, 62, 195),
        IM_COL32(236, 146, 0, 195),
        IM_COL32(6, 146, 207, 195),
        IM_COL32(154, 150, 76, 195),
        IM_COL32(0, 153, 88, 195),
        IM_COL32(101, 138, 14, 195),
        IM_COL32(180, 95, 60, 195),
        IM_COL32(125, 95, 170, 195)
    };
    return kColores[idx % (sizeof(kColores) / sizeof(kColores[0]))];
}

} // namespace

PanelDiagramasLpp::PanelDiagramasLpp()
    : motor_(&base_datos::GestorDB::instancia()) {
    cargar_preset_ti_cl_o();
    ejecutar_calculo(true);
}

void PanelDiagramasLpp::actualizar_sistema(bool reset_ejes) {
    sistema_ = motor_.analizar_sistema_ternario(elementos_sel_, incluir_liquidos_, incluir_organicos_, "", false);
    especies_sel_.clear();
    for (const auto& esp : sistema_.especies_sistema) {
        especies_sel_[esp.formula] = esp.seleccionada_por_defecto && esp.contiene_elemento_principal;
    }

    if (!sistema_.especies_gas_ejes.empty() && reset_ejes) {
        std::vector<std::string> gases_validos;
        for (const auto& g : sistema_.especies_gas_ejes) {
            if (g.contiene_elemento_principal) continue;
            gases_validos.push_back(g.formula);
        }
        if (gases_validos.size() >= 2) {
            bool tiene_cl2 = std::find(gases_validos.begin(), gases_validos.end(), "Cl2(g)") != gases_validos.end();
            bool tiene_o2  = std::find(gases_validos.begin(), gases_validos.end(), "O2(g)")  != gases_validos.end();
            bool tiene_so2 = std::find(gases_validos.begin(), gases_validos.end(), "SO2(g)") != gases_validos.end();
            if (tiene_cl2 && tiene_o2) {
                x_gas_ = "Cl2(g)";
                y_gas_ = "O2(g)";
            } else if (tiene_so2 && tiene_o2) {
                x_gas_ = "SO2(g)";
                y_gas_ = "O2(g)";
            } else {
                x_gas_ = gases_validos[0];
                y_gas_ = gases_validos[1];
            }
        }
    }
}

void PanelDiagramasLpp::cargar_preset_ti_cl_o() {
    elementos_sel_ = {"Cl", "O", "Ti"};
    incluir_liquidos_ = false;
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_gas_ = "Cl2(g)";
    y_gas_ = "O2(g)";
    auto_scale_ = true;
    usar_kelvin_ = false;
    temperatura_ = 300.0;
}

void PanelDiagramasLpp::cargar_preset_v_cl_o() {
    elementos_sel_ = {"Cl", "O", "V"};
    incluir_liquidos_ = false;
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_gas_ = "Cl2(g)";
    y_gas_ = "O2(g)";
    auto_scale_ = false;
    x_min_ = -40.0;
    x_max_ = 0.0;
    y_min_ = -40.0;
    y_max_ = 0.0;
    usar_kelvin_ = false;
    temperatura_ = 500.0;
}

void PanelDiagramasLpp::cargar_preset_cu_s_o() {
    elementos_sel_ = {"Cu", "O", "S"};
    incluir_liquidos_ = false;
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_gas_ = "SO2(g)";
    y_gas_ = "O2(g)";
    auto_scale_ = true;
    usar_kelvin_ = false;
    temperatura_ = 700.0;
}

void PanelDiagramasLpp::cargar_preset_fe_s_o() {
    elementos_sel_ = {"Fe", "O", "S"};
    incluir_liquidos_ = false;
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_gas_ = "SO2(g)";
    y_gas_ = "O2(g)";
    auto_scale_ = true;
    usar_kelvin_ = false;
    temperatura_ = 727.0;
}

void PanelDiagramasLpp::cargar_preset_ni_s_o() {
    elementos_sel_ = {"Ni", "O", "S"};
    incluir_liquidos_ = false;
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_gas_ = "SO2(g)";
    y_gas_ = "O2(g)";
    auto_scale_ = true;
    usar_kelvin_ = false;
    temperatura_ = 700.0;
}

void PanelDiagramasLpp::ejecutar_calculo(bool forzar_en_actual) {
    cma::domain::ParametrosCalculoLpp params;
    params.elementos = elementos_sel_;
    params.elemento_principal = sistema_.elemento_principal;
    params.gas_x = x_gas_;
    params.gas_y = y_gas_;
    params.T_K = usar_kelvin_ ? temperatura_ : (temperatura_ + 273.15);
    params.auto_escala = auto_scale_;
    params.x_min = x_min_;
    params.x_max = x_max_;
    params.y_min = y_min_;
    params.y_max = y_max_;
    params.incluir_liquidos = incluir_liquidos_;
    params.incluir_organicos = incluir_organicos_;

    for (const auto& kv : especies_sel_) {
        if (kv.second) params.formulas_seleccionadas.push_back(kv.first);
    }

    auto res = motor_.calcular_lpp(params);
    if (!res.ok) {
        estado_msg_ = res.mensaje_error;
        return;
    }
    estado_msg_.clear();

    if (auto_scale_) {
        x_min_ = res.x_min_usado;
        x_max_ = res.x_max_usado;
        y_min_ = res.y_min_usado;
        y_max_ = res.y_max_usado;
    }

    if (pestanas_.empty() || (create_new_page_ && !forzar_en_actual)) {
        PestanaLpp nueva;
        ++contador_pestanas_;
        nueva.nombre_pestana = "Chart" + std::to_string(contador_pestanas_);
        nueva.calculado = true;
        nueva.ajustar_ejes = true;
        nueva.resultado = std::move(res);
        pestanas_.push_back(std::move(nueva));
        pestana_activa_ = static_cast<int>(pestanas_.size()) - 1;
    } else {
        int idx = std::clamp(pestana_activa_, 0, static_cast<int>(pestanas_.size()) - 1);
        pestanas_[idx].calculado = true;
        pestanas_[idx].ajustar_ejes = true;
        pestanas_[idx].resultado = std::move(res);
    }
}

void PanelDiagramasLpp::renderizar() {
    // ── Barra de Menú Superior estilo HSC Lpp (File | Help) ──────────────────
    if (ImGui::BeginMenuBar()) {
        if (ImGui::BeginMenu("File")) {
            if (ImGui::MenuItem("New Empty Chart Tab")) {
                PestanaLpp vacia;
                ++contador_pestanas_;
                vacia.nombre_pestana = "Chart" + std::to_string(contador_pestanas_);
                vacia.calculado = false;
                pestanas_.push_back(vacia);
                pestana_activa_ = static_cast<int>(pestanas_.size()) - 1;
            }
            ImGui::Separator();
            if (ImGui::BeginMenu("Load HSC Kellogg Examples")) {
                if (ImGui::MenuItem("Ti-Cl-O System at 300 °C (HSC Manual Fig. 3 & 4)")) {
                    cargar_preset_ti_cl_o();
                    ejecutar_calculo(false);
                }
                if (ImGui::MenuItem("V-Cl-O System at 500 °C")) {
                    cargar_preset_v_cl_o();
                    ejecutar_calculo(false);
                }
                if (ImGui::MenuItem("Cu-S-O System at 700 °C")) {
                    cargar_preset_cu_s_o();
                    ejecutar_calculo(false);
                }
                if (ImGui::MenuItem("Fe-S-O System at 727 °C")) {
                    cargar_preset_fe_s_o();
                    ejecutar_calculo(false);
                }
                if (ImGui::MenuItem("Ni-S-O System at 700 °C")) {
                    cargar_preset_ni_s_o();
                    ejecutar_calculo(false);
                }
                ImGui::EndMenu();
            }
            ImGui::Separator();
            if (ImGui::MenuItem("Open LPP File (*.lpp8, *.lpp)...")) {
                modo_guardar_ = false;
                modal_archivo_ = true;
            }
            if (ImGui::MenuItem("Save LPP File (*.lpp8, *.lpp)...")) {
                modo_guardar_ = true;
                modal_archivo_ = true;
            }
            if (ImGui::MenuItem("Copy Equilibrium Boundary Table (CSV)")) {
                if (!pestanas_.empty() && pestanas_[pestana_activa_].calculado) {
                    std::string csv = motor_.exportar_tabla_lpp_csv(pestanas_[pestana_activa_].resultado);
                    ImGui::SetClipboardText(csv.c_str());
                }
            }
            ImGui::EndMenu();
        }
        if (ImGui::BeginMenu("Help")) {
            ImGui::TextColored(ImVec4(0.15f, 0.45f, 0.85f, 1.0f), "HSC Lpp Phase Stability Diagram (Kellogg Vector Graphics)");
            ImGui::Separator();
            ImGui::BulletText("1. Select three elements with the mouse (e.g., Ti, Cl, and O).");
            ImGui::BulletText("2. Select at least three substances containing the main element (or press Select All).");
            ImGui::BulletText("   Double-click any substance in the list to inspect its thermodynamic data.");
            ImGui::BulletText("3. Select one gas species for the X-axis (e.g. Cl2(g)) and Y-axis (e.g. O2(g)).");
            ImGui::BulletText("4. Uncheck Auto scale if you want to customize X and Y min/max ranges.");
            ImGui::BulletText("5. Press Draw diagram, or press Show table to inspect exact boundary coordinates.");
            ImGui::EndMenu();
        }
        ImGui::EndMenuBar();
    }

    // ── Columna Izquierda: Diagram parameters (idéntico a Image 4) ───────────
    ImGui::BeginChild("##LppLeftPanel", ImVec2(355.0f, 0.0f), true);
    {
        ImGui::TextColored(ImVec4(0.28f, 0.34f, 0.44f, 1.0f), "Diagram parameters");
        ImGui::Separator();

        if (ImGui::BeginTable("##LppElemSpeciesTable", 2, ImGuiTableFlags_SizingStretchSame)) {
            ImGui::TableSetupColumn("Select 3 Elements", ImGuiTableColumnFlags_WidthFixed, 152.0f);
            ImGui::TableSetupColumn("Select Species", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableNextRow();

            // Select 3 Elements
            ImGui::TableSetColumnIndex(0);
            ImGui::TextUnformatted("Select 3 Elements");
            ImGui::BeginChild("##LppElemGrid", ImVec2(150.0f, 305.0f), true);
            if (ImGui::BeginTable("##LppGrid88", 4, ImGuiTableFlags_SizingFixedFit | ImGuiTableFlags_NoPadInnerX)) {
                auto render_elem = [&](const char* sym) {
                    ImGui::TableNextColumn();
                    bool sel = std::find(elementos_sel_.begin(), elementos_sel_.end(), sym) != elementos_sel_.end();
                    if (sel) {
                        ImGui::PushStyleColor(ImGuiCol_Header, ImVec4(0.75f, 0.84f, 0.98f, 1.0f));
                        ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.05f, 0.15f, 0.45f, 1.0f));
                    }
                    if (ImGui::Selectable(sym, sel, ImGuiSelectableFlags_None, ImVec2(29.0f, 14.0f))) {
                        auto it = std::find(elementos_sel_.begin(), elementos_sel_.end(), sym);
                        if (it != elementos_sel_.end()) {
                            elementos_sel_.erase(it);
                        } else {
                            if (elementos_sel_.size() >= 3) elementos_sel_.erase(elementos_sel_.begin());
                            elementos_sel_.push_back(sym);
                        }
                        if (elementos_sel_.size() == 3) {
                            actualizar_sistema(true);
                        } else {
                            sistema_ = cma::domain::SistemaTernarioEstabilidad{};
                            especies_sel_.clear();
                        }
                    }
                    if (sel) ImGui::PopStyleColor(2);
                };

                for (int r = 0; r < 22; ++r) {
                    ImGui::TableNextRow();
                    for (int c = 0; c < 4; ++c) render_elem(kGridLpp22[r][c]);
                }
                for (int r = 0; r < 3; ++r) {
                    ImGui::TableNextRow();
                    for (int c = 0; c < 4; ++c) render_elem(kGridLppExtra[r][c]);
                }
                ImGui::EndTable();
            }
            ImGui::EndChild();

            if (ImGui::Checkbox("Liquids", &incluir_liquidos_)) {
                if (elementos_sel_.size() == 3) actualizar_sistema(false);
            }
            ImGui::SameLine();
            if (ImGui::Checkbox("Organics", &incluir_organicos_)) {
                if (elementos_sel_.size() == 3) actualizar_sistema(false);
            }

            // Select Species
            ImGui::TableSetColumnIndex(1);
            ImGui::TextUnformatted("Select Species");
            ImGui::BeginChild("##LppSpeciesList", ImVec2(0.0f, 305.0f), true);
            for (const auto& esp : sistema_.especies_sistema) {
                if (!esp.contiene_elemento_principal) continue;
                bool marcada = especies_sel_[esp.formula];
                if (marcada) {
                    ImGui::PushStyleColor(ImGuiCol_Header, ImVec4(0.82f, 0.89f, 0.98f, 1.0f));
                }
                if (ImGui::Selectable(esp.formula.c_str(), marcada)) {
                    especies_sel_[esp.formula] = !marcada;
                }
                if (marcada) ImGui::PopStyleColor();

                if (ImGui::IsItemHovered() && ImGui::IsMouseDoubleClicked(0)) {
                    especie_inspeccionada_ = esp.formula;
                    modal_especie_ = true;
                }
            }
            ImGui::EndChild();

            if (ImGui::Button("Select All", ImVec2(-1.0f, 24.0f))) {
                bool todas = true;
                for (const auto& esp : sistema_.especies_sistema) {
                    if (!esp.contiene_elemento_principal) continue;
                    if (!especies_sel_[esp.formula]) {
                        todas = false;
                        break;
                    }
                }
                for (const auto& esp : sistema_.especies_sistema) {
                    if (!esp.contiene_elemento_principal) continue;
                    especies_sel_[esp.formula] = !todas;
                }
            }

            ImGui::EndTable();
        }

        ImGui::Spacing();
        ImGui::Separator();

        // X-axis variable & Y-axis variable (excluyendo gases que contengan el elemento principal)
        if (ImGui::BeginTable("##LppAxesTable", 2, ImGuiTableFlags_SizingStretchSame)) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::TextUnformatted("X-axis variable");
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::BeginCombo("##LppXGas", x_gas_.c_str())) {
                for (const auto& g : sistema_.especies_gas_ejes) {
                    if (g.contiene_elemento_principal) continue;
                    bool sel = (x_gas_ == g.formula);
                    if (ImGui::Selectable(g.formula.c_str(), sel)) {
                        x_gas_ = g.formula;
                    }
                }
                ImGui::EndCombo();
            }

            ImGui::TableSetColumnIndex(1);
            ImGui::TextUnformatted("Y-axis variable");
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::BeginCombo("##LppYGas", y_gas_.c_str())) {
                for (const auto& g : sistema_.especies_gas_ejes) {
                    if (g.contiene_elemento_principal) continue;
                    bool sel = (y_gas_ == g.formula);
                    if (ImGui::Selectable(g.formula.c_str(), sel)) {
                        y_gas_ = g.formula;
                    }
                }
                ImGui::EndCombo();
            }

            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::Text("      Log p%s", x_gas_.c_str());
            ImGui::TableSetColumnIndex(1);
            ImGui::Text("      Log p%s", y_gas_.c_str());

            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::BeginDisabled(auto_scale_);
            ImGui::AlignTextToFramePadding();
            ImGui::TextUnformatted("Min");
            ImGui::SameLine(36.0f);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##LppXMin", &x_min_, 0.0, 0.0, "%.2f");
            ImGui::AlignTextToFramePadding();
            ImGui::TextUnformatted("Max");
            ImGui::SameLine(36.0f);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##LppXMax", &x_max_, 0.0, 0.0, "%.2f");
            ImGui::EndDisabled();

            ImGui::TableSetColumnIndex(1);
            ImGui::BeginDisabled(auto_scale_);
            ImGui::AlignTextToFramePadding();
            ImGui::TextUnformatted("Min");
            ImGui::SameLine(36.0f);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##LppYMin", &y_min_, 0.0, 0.0, "%.2f");
            ImGui::AlignTextToFramePadding();
            ImGui::TextUnformatted("Max");
            ImGui::SameLine(36.0f);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##LppYMax", &y_max_, 0.0, 0.0, "%.2f");
            ImGui::EndDisabled();

            ImGui::EndTable();
        }

        ImGui::Checkbox("Auto scale", &auto_scale_);
        ImGui::Spacing();
        ImGui::Separator();

        ImGui::AlignTextToFramePadding();
        ImGui::TextUnformatted("Temperature:");
        ImGui::SameLine(95.0f);
        ImGui::SetNextItemWidth(105.0f);
        ImGui::InputDouble("##LppTemp", &temperatura_, 0.0, 0.0, "%.2f");
        ImGui::SameLine();
        if (ImGui::Button(usar_kelvin_ ? "K##LppUnit" : "°C##LppUnit", ImVec2(72.0f, 0.0f))) {
            if (!usar_kelvin_) {
                usar_kelvin_ = true;
                temperatura_ += 273.15;
            } else {
                usar_kelvin_ = false;
                temperatura_ -= 273.15;
            }
        }

        ImGui::Spacing();
        ImGui::Checkbox("Create new page", &create_new_page_);
        ImGui::Checkbox("Show colors", &show_colors_);

        ImGui::Spacing();
        if (ImGui::Button("Show table", ImVec2(148.0f, 28.0f))) {
            show_table_ = !show_table_;
        }
        ImGui::SameLine();
        if (ImGui::Button("Draw diagram", ImVec2(158.0f, 28.0f))) {
            ejecutar_calculo(false);
        }

        if (!estado_msg_.empty()) {
            ImGui::TextWrapped("%s", estado_msg_.c_str());
        }
    }
    ImGui::EndChild();

    ImGui::SameLine();

    // ── Columna Derecha: Pestañas de Diagramas Lpp + Tabla de Coordenadas ────
    ImGui::BeginChild("##LppRightPanel", ImVec2(0.0f, 0.0f), true);
    {
        if (ImGui::BeginTabBar("##LppTabs", ImGuiTabBarFlags_Reorderable | ImGuiTabBarFlags_AutoSelectNewTabs)) {
            for (int i = 0; i < static_cast<int>(pestanas_.size()); ++i) {
                bool abierta = true;
                if (ImGui::BeginTabItem(pestanas_[i].nombre_pestana.c_str(), &abierta)) {
                    pestana_activa_ = i;
                    auto& tab = pestanas_[i];

                    std::string titulo_def = tab.calculado
                                                 ? ("Phase Stability Diagram for " + tab.resultado.titulo_sistema + " System")
                                                 : "Phase Stability Diagram";
                    std::string lx_def = tab.calculado ? ("log p" + tab.resultado.gas_x) : "X axis title";
                    std::string ly_def = tab.calculado ? ("log p" + tab.resultado.gas_y) : "Y axis title";

                    if (ImGui::SmallButton("Redraw (Reset Zoom)")) {
                        tab.ajustar_ejes = true;
                    }
                    ImGui::SameLine();
                    if (ImGui::SmallButton("Edit Titles / Scales")) {
                        std::snprintf(edit_titulo_, sizeof(edit_titulo_), "%s", titulo_def.c_str());
                        std::snprintf(edit_x_, sizeof(edit_x_), "%s", lx_def.c_str());
                        std::snprintf(edit_y_, sizeof(edit_y_), "%s", ly_def.c_str());
                        modal_editar_ejes_ = true;
                    }
                    ImGui::SameLine();
                    if (ImGui::SmallButton("Clone Tab")) {
                        PestanaLpp clon = tab;
                        ++contador_pestanas_;
                        clon.nombre_pestana = "Chart" + std::to_string(contador_pestanas_);
                        pestanas_.push_back(std::move(clon));
                    }
                    ImGui::SameLine();
                    if (ImGui::SmallButton(show_table_ ? "Hide Boundary Table" : "Show Boundary Table")) {
                        show_table_ = !show_table_;
                    }

                    const char* titulo = titulo_def.c_str();
                    float w_avail = ImGui::GetContentRegionAvail().x;
                    float w_txt = ImGui::CalcTextSize(titulo).x;
                    ImGui::SetCursorPosX(std::max(10.0f, (w_avail - w_txt) * 0.5f));
                    ImGui::TextColored(ImVec4(0.05f, 0.05f, 0.08f, 1.0f), "%s", titulo);

                    float alto_tabla = show_table_ ? 210.0f : 0.0f;
                    ImVec2 plot_sz(ImGui::GetContentRegionAvail().x,
                                   std::max(220.0f, ImGui::GetContentRegionAvail().y - alto_tabla - 28.0f));

                    if (ImPlot::BeginPlot("##LppVectorPlot", plot_sz, ImPlotFlags_NoLegend | ImPlotFlags_NoTitle)) {
                        ImPlot::SetupAxes(lx_def.c_str(), ly_def.c_str());

                        double rx0 = tab.calculado ? tab.resultado.x_min_usado : 0.0;
                        double rx1 = tab.calculado ? tab.resultado.x_max_usado : 10.0;
                        double ry0 = tab.calculado ? tab.resultado.y_min_usado : 0.0;
                        double ry1 = tab.calculado ? tab.resultado.y_max_usado : 10.0;

                        ImPlotCond cond = tab.ajustar_ejes ? ImPlotCond_Always : ImPlotCond_Once;
                        ImPlot::SetupAxisLimits(ImAxis_X1, rx0, rx1, cond);
                        ImPlot::SetupAxisLimits(ImAxis_Y1, ry0, ry1, cond);
                        tab.ajustar_ejes = false;

                        if (tab.calculado) {
                            ImDrawList* dl = ImPlot::GetPlotDrawList();
                            ImPlot::PushPlotClipRect();

                            if (show_colors_) {
                                for (size_t p_idx = 0; p_idx < tab.resultado.poligonos.size(); ++p_idx) {
                                    const auto& pol = tab.resultado.poligonos[p_idx];
                                    if (pol.vertices.size() < 3) continue;
                                    std::vector<ImVec2> pts;
                                    pts.reserve(pol.vertices.size());
                                    for (const auto& v : pol.vertices) {
                                        pts.push_back(ImPlot::PlotToPixels(ImPlotPoint(v.x, v.y)));
                                    }
                                    dl->AddConvexPolyFilled(pts.data(), static_cast<int>(pts.size()),
                                                            color_lpp_relleno(p_idx));
                                }
                            }

                            for (const auto& fr : tab.resultado.fronteras) {
                                ImVec2 p1 = ImPlot::PlotToPixels(ImPlotPoint(fr.p1.x, fr.p1.y));
                                ImVec2 p2 = ImPlot::PlotToPixels(ImPlotPoint(fr.p2.x, fr.p2.y));
                                dl->AddLine(p1, p2, IM_COL32(15, 15, 15, 255), 1.85f);
                            }

                            for (const auto& pt : tab.resultado.puntos_triples) {
                                ImVec2 px = ImPlot::PlotToPixels(ImPlotPoint(pt.x, pt.y));
                                dl->AddCircleFilled(px, 3.5f, IM_COL32(195, 35, 35, 255));
                            }

                            for (const auto& pol : tab.resultado.poligonos) {
                                const std::string& lbl = pol.etiqueta_grafico.empty() ? pol.formula : pol.etiqueta_grafico;
                                ImVec2 pos = ImPlot::PlotToPixels(ImPlotPoint(pol.centroide.x, pol.centroide.y));
                                ImVec2 tsz = ImGui::CalcTextSize(lbl.c_str());
                                ImVec2 tp(pos.x - tsz.x * 0.5f, pos.y - tsz.y * 0.5f);
                                dl->AddText(ImVec2(tp.x + 1.0f, tp.y + 1.0f),
                                            IM_COL32(255, 255, 255, 190), lbl.c_str());
                                dl->AddText(tp, IM_COL32(10, 10, 10, 255), lbl.c_str());
                            }

                            ImPlot::PopPlotClipRect();

                            if (ImPlot::IsPlotHovered()) {
                                ImPlotPoint mp = ImPlot::GetPlotMousePos();
                                for (const auto& pt : tab.resultado.puntos_triples) {
                                    double dx = mp.x - pt.x;
                                    double dy = mp.y - pt.y;
                                    if (std::hypot(dx, dy) < 1.2) {
                                        ImGui::BeginTooltip();
                                        ImGui::TextColored(ImVec4(0.95f, 0.35f, 0.25f, 1.0f),
                                                           "Invariant Triple Point: %s + %s + %s",
                                                           pt.fase_1.c_str(), pt.fase_2.c_str(), pt.fase_3.c_str());
                                        ImGui::Text("%s = %.3f", lx_def.c_str(), pt.x);
                                        ImGui::Text("%s = %.3f", ly_def.c_str(), pt.y);
                                        ImGui::EndTooltip();
                                        break;
                                    }
                                }
                            }
                        }

                        ImPlot::EndPlot();
                    }

                    if (tab.calculado) {
                        ImGui::Text("Constant value: Temperature = %.1f °C (%.2f K)   |   Active domains: %zu   |   Equilibrium boundaries: %zu   |   Triple points: %zu",
                                    tab.resultado.T_K - 273.15,
                                    tab.resultado.T_K,
                                    tab.resultado.poligonos.size(),
                                    tab.resultado.fronteras.size(),
                                    tab.resultado.puntos_triples.size());
                    }

                    // ── Tabla Vectorial de Líneas de Equilibrio y Puntos Triples (Show table) ──
                    if (show_table_ && tab.calculado) {
                        ImGui::BeginChild("##LppTableDrawer", ImVec2(0.0f, 0.0f), true);
                        ImGui::TextColored(ImVec4(0.15f, 0.45f, 0.85f, 1.0f),
                                           "Lpp Exact Phase Boundary Coordinates & Equilibrium Reactions");
                        ImGui::SameLine(ImGui::GetWindowWidth() - 135.0f);
                        if (ImGui::SmallButton("Copy CSV Table")) {
                            std::string csv = motor_.exportar_tabla_lpp_csv(tab.resultado);
                            ImGui::SetClipboardText(csv.c_str());
                        }

                        if (ImGui::BeginTable("##LppBoundariesGrid", 7,
                                              ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                              ImGuiTableFlags_ScrollY | ImGuiTableFlags_SizingStretchProp)) {
                            ImGui::TableSetupScrollFreeze(0, 1);
                            ImGui::TableSetupColumn("Phase A", ImGuiTableColumnFlags_WidthFixed, 75.0f);
                            ImGui::TableSetupColumn("Phase B", ImGuiTableColumnFlags_WidthFixed, 75.0f);
                            ImGui::TableSetupColumn("Equilibrium Reaction", ImGuiTableColumnFlags_WidthStretch);
                            ImGui::TableSetupColumn("ΔG° (kJ)", ImGuiTableColumnFlags_WidthFixed, 82.0f);
                            ImGui::TableSetupColumn("log K", ImGuiTableColumnFlags_WidthFixed, 68.0f);
                            ImGui::TableSetupColumn("(X1, Y1)", ImGuiTableColumnFlags_WidthFixed, 115.0f);
                            ImGui::TableSetupColumn("(X2, Y2)", ImGuiTableColumnFlags_WidthFixed, 115.0f);
                            ImGui::TableHeadersRow();

                            for (const auto& fr : tab.resultado.fronteras) {
                                ImGui::TableNextRow();
                                ImGui::TableSetColumnIndex(0);
                                ImGui::TextUnformatted(fr.fase_a.c_str());
                                ImGui::TableSetColumnIndex(1);
                                ImGui::TextUnformatted(fr.fase_b.c_str());
                                ImGui::TableSetColumnIndex(2);
                                ImGui::TextUnformatted(fr.ecuacion_reaccion.c_str());
                                ImGui::TableSetColumnIndex(3);
                                ImGui::Text("%.2f", fr.delta_G_rxn_kJ);
                                ImGui::TableSetColumnIndex(4);
                                ImGui::Text("%.2f", fr.log10_K);
                                ImGui::TableSetColumnIndex(5);
                                ImGui::Text("(%.2f, %.2f)", fr.p1.x, fr.p1.y);
                                ImGui::TableSetColumnIndex(6);
                                ImGui::Text("(%.2f, %.2f)", fr.p2.x, fr.p2.y);
                            }
                            ImGui::EndTable();
                        }
                        ImGui::EndChild();
                    }

                    ImGui::EndTabItem();
                }
                if (!abierta && pestanas_.size() > 1) {
                    pestanas_.erase(pestanas_.begin() + i);
                    if (pestana_activa_ >= static_cast<int>(pestanas_.size())) {
                        pestana_activa_ = static_cast<int>(pestanas_.size()) - 1;
                    }
                    --i;
                }
            }
            ImGui::EndTabBar();
        }
    }
    ImGui::EndChild();

    // ── Modal de Inspección Termodinámica (Doble Clic en Especie) ────────────
    if (modal_especie_) {
        ImGui::OpenPopup("Species Thermodynamic Properties##LppModal");
        modal_especie_ = false;
    }
    if (ImGui::BeginPopupModal("Species Thermodynamic Properties##LppModal", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        for (const auto& esp : sistema_.especies_sistema) {
            if (esp.formula == especie_inspeccionada_) {
                ImGui::TextColored(ImVec4(0.12f, 0.42f, 0.82f, 1.0f), "Species: %s (%s)",
                                   esp.formula.c_str(), esp.nombre.c_str());
                ImGui::Separator();
                ImGui::Text("ΔHf°(298.15 K): %.3f kJ/mol", esp.h_formacion_298);
                ImGui::Text("S°(298.15 K):   %.3f J/(mol·K)", esp.s_298);
                ImGui::Text("Cp(298.15 K):   %.3f J/(mol·K)", esp.cp_298);
                break;
            }
        }
        if (ImGui::Button("Close", ImVec2(110.0f, 0.0f))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }

    // ── Modal para editar títulos de ejes y diagrama ─────────────────────────
    if (modal_editar_ejes_) {
        ImGui::OpenPopup("Edit Lpp Chart Titles##LppAxesEdit");
        modal_editar_ejes_ = false;
    }
    if (ImGui::BeginPopupModal("Edit Lpp Chart Titles##LppAxesEdit", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::InputText("Chart Title", edit_titulo_, sizeof(edit_titulo_));
        ImGui::InputText("X-Axis Title", edit_x_, sizeof(edit_x_));
        ImGui::InputText("Y-Axis Title", edit_y_, sizeof(edit_y_));
        if (ImGui::Button("Apply", ImVec2(110.0f, 0.0f))) {
            if (!pestanas_.empty()) {
                auto& tab = pestanas_[std::clamp(pestana_activa_, 0, static_cast<int>(pestanas_.size()) - 1)];
                tab.resultado.titulo_sistema = edit_titulo_;
            }
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button("Cancel", ImVec2(110.0f, 0.0f))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }

    // ── Modal para Abrir / Guardar archivo .lpp8 ─────────────────────────────
    if (modal_archivo_) {
        ImGui::OpenPopup("LPP8 File Manager##LppFileModal");
        modal_archivo_ = false;
    }
    if (ImGui::BeginPopupModal("LPP8 File Manager##LppFileModal", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::TextUnformatted(modo_guardar_ ? "Save LPP Specification (*.lpp8)" : "Open LPP Specification (*.lpp8)");
        ImGui::InputText("File path", ruta_archivo_, sizeof(ruta_archivo_));
        if (ImGui::Button(modo_guardar_ ? "Save" : "Open", ImVec2(110.0f, 0.0f))) {
            if (modo_guardar_) {
                cma::domain::ParametrosCalculoLpp p;
                p.elementos = elementos_sel_;
                p.elemento_principal = sistema_.elemento_principal;
                p.gas_x = x_gas_;
                p.gas_y = y_gas_;
                p.T_K = usar_kelvin_ ? temperatura_ : (temperatura_ + 273.15);
                p.auto_escala = auto_scale_;
                p.x_min = x_min_;
                p.x_max = x_max_;
                p.y_min = y_min_;
                p.y_max = y_max_;
                p.incluir_liquidos = incluir_liquidos_;
                p.incluir_organicos = incluir_organicos_;
                std::ofstream ofs(ruta_archivo_);
                if (ofs) ofs << motor_.serializar_lpp(p);
            } else {
                std::ifstream ifs(ruta_archivo_);
                if (ifs) {
                    std::ostringstream ss;
                    ss << ifs.rdbuf();
                    cma::domain::ParametrosCalculoLpp p;
                    if (motor_.deserializar_lpp(ss.str(), p) && p.elementos.size() == 3) {
                        elementos_sel_ = p.elementos;
                        incluir_liquidos_ = p.incluir_liquidos;
                        incluir_organicos_ = p.incluir_organicos;
                        actualizar_sistema(false);
                        x_gas_ = p.gas_x;
                        y_gas_ = p.gas_y;
                        auto_scale_ = p.auto_escala;
                        x_min_ = p.x_min;
                        x_max_ = p.x_max;
                        y_min_ = p.y_min;
                        y_max_ = p.y_max;
                        temperatura_ = usar_kelvin_ ? p.T_K : (p.T_K - 273.15);
                        ejecutar_calculo(false);
                    }
                }
            }
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button("Cancel", ImVec2(110.0f, 0.0f))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }
}

}} // namespace cma::interfaz
