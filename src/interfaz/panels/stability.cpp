#include "../paneles.hpp"
#include <implot.h>
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <sstream>

namespace cma { namespace interfaz {

namespace {

// Tabla exacta de 22 filas × 4 columnas de las capturas HSC Tpp/Lpp
static const char* kGridElementosHSC[22][4] = {
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
static const char* kGridElementosExtra[2][4] = {
    {"Cr", "Li", "Re", "Yb"},
    {"Cs", "Lu", "Rh", "Zn"}
};
static const char* kGridElementosUltima[1][4] = {
    {"Cu", "Mg", "Rn", "Zr"}
};

ImU32 color_hsc_especie(const std::string& formula, size_t idx) {
    // Colores exactos de Image 1 (V-Cl-O) e Image 3 (S-Cu-O)
    if (formula == "V2O5")       return IM_COL32(38, 128, 255, 255);  // Azul real superior
    if (formula == "V3O7")       return IM_COL32(0, 179, 158, 255);   // Turquesa
    if (formula == "V6O13")      return IM_COL32(0, 153, 88, 255);    // Verde esmeralda
    if (formula == "VO2" || formula == "V2O4") return IM_COL32(236, 146, 0, 255); // Naranja ámbar
    if (formula == "V3O5")       return IM_COL32(0, 158, 245, 255);   // Azul cielo brillante
    if (formula == "V2O3")       return IM_COL32(23, 76, 128, 255);   // Azul marino oscuro
    if (formula == "VO1.24" || formula == "VO") return IM_COL32(154, 150, 76, 255); // Oliva tostado inferior
    if (formula == "VCl2")       return IM_COL32(114, 184, 62, 255);  // Verde lima
    if (formula == "VCl3")       return IM_COL32(6, 143, 198, 255);   // Azul cerúleo
    if (formula == "VCl4" || formula == "VCl4(l)") return IM_COL32(100, 104, 108, 255); // Gris pizarra
    if (formula == "VOCl")       return IM_COL32(43, 114, 34, 255);   // Verde musgo oscuro
    if (formula == "VOCl3" || formula == "VOCl2") return IM_COL32(101, 138, 14, 255); // Verde oliva

    // Image 3 (S-Cu-O T-p)
    if (formula == "CuSO4")      return IM_COL32(99, 102, 104, 255);  // Gris pizarra oscuro
    if (formula == "CuO*CuSO4")  return IM_COL32(6, 146, 207, 255);   // Azul cerúleo banda central
    if (formula == "CuO" || formula == "CuO(T)") return IM_COL32(114, 184, 62, 255); // Verde hoja derecha
    if (formula == "Cu2O")       return IM_COL32(236, 146, 0, 255);
    if (formula == "Cu2S")       return IM_COL32(23, 76, 128, 255);
    if (formula == "CuS")        return IM_COL32(0, 179, 158, 255);
    if (formula == "Cu")         return IM_COL32(184, 115, 51, 255);

    // Ti-Cl-O / Fe-S-O / paleta general HSC
    static const ImU32 kPaletaHSC[] = {
        IM_COL32(38, 128, 255, 255),
        IM_COL32(0, 179, 158, 255),
        IM_COL32(114, 184, 62, 255),
        IM_COL32(236, 146, 0, 255),
        IM_COL32(6, 146, 207, 255),
        IM_COL32(23, 76, 128, 255),
        IM_COL32(154, 150, 76, 255),
        IM_COL32(100, 104, 108, 255),
        IM_COL32(0, 153, 88, 255),
        IM_COL32(101, 138, 14, 255),
        IM_COL32(180, 95, 60, 255),
        IM_COL32(125, 95, 170, 255)
    };
    return kPaletaHSC[idx % (sizeof(kPaletaHSC) / sizeof(kPaletaHSC[0]))];
}

int contar_elementos_distintos(const cma::domain::EspecieCandidataEstabilidad& esp) {
    int n = 0;
    for (const auto& kv : esp.composicion) {
        if (kv.second > 1e-9) ++n;
    }
    return n;
}

} // namespace

PanelEstabilidad::PanelEstabilidad()
    : motor_(&base_datos::GestorDB::instancia()) {
    // Pre-cargar Chart 1 (V-Cl-O a 500 °C, Image 1), Chart 2 (S-Cu-O T-p, Image 3) y Chart 3 (HSC Chart vacío, Image 2)
    cargar_preset_s_cu_o_tp();
    ejecutar_calculo(true); // Crea Chart 1 temporalmente

    PestanaTpp tab_scuo = pestanas_.back();
    tab_scuo.nombre_pestana = "Chart 2";

    pestanas_.clear();
    contador_pestanas_ = 0;

    // Ahora configuramos el estado activo como V-Cl-O (Image 1) en Chart 1
    cargar_preset_v_cl_o();
    ejecutar_calculo(true);
    if (!pestanas_.empty()) {
        pestanas_[0].nombre_pestana = "Chart 1";
    }
    pestanas_.push_back(tab_scuo);

    PestanaTpp tab_vacia;
    tab_vacia.nombre_pestana = "Chart 3";
    tab_vacia.calculado = false;
    tab_vacia.ajustar_ejes = true;
    pestanas_.push_back(tab_vacia);

    contador_pestanas_ = 3;
    pestana_activa_ = 0;
}

void PanelEstabilidad::actualizar_sistema(bool reset_ejes) {
    sistema_ = motor_.analizar_sistema_ternario(elementos_sel_, true, incluir_organicos_, "", true);
    especies_sel_.clear();
    for (const auto& esp : sistema_.especies_sistema) {
        especies_sel_[esp.formula] = esp.seleccionada_por_defecto;
    }
    if (!sistema_.especies_gas_ejes.empty() && reset_ejes) {
        bool tiene_cl2 = false, tiene_o2 = false, tiene_so2 = false;
        for (const auto& g : sistema_.especies_gas_ejes) {
            if (g.formula == "Cl2(g)") tiene_cl2 = true;
            if (g.formula == "O2(g)")  tiene_o2 = true;
            if (g.formula == "SO2(g)") tiene_so2 = true;
        }
        if (tiene_cl2 && tiene_o2) {
            x_var_ = "Cl2(g)";
            y_var_ = "O2(g)";
        } else if (tiene_so2 && tiene_o2) {
            x_var_ = "SO2(g)";
            y_var_ = "O2(g)";
        } else {
            x_var_ = sistema_.especies_gas_ejes[0].formula;
            y_var_ = (sistema_.especies_gas_ejes.size() > 1)
                         ? sistema_.especies_gas_ejes[1].formula
                         : sistema_.especies_gas_ejes[0].formula;
        }
        fixed_var_ = "Temperature";
        x_min_ = -40.0;
        x_max_ = 0.0;
        y_min_ = -40.0;
        y_max_ = 0.0;
    }
}

void PanelEstabilidad::cargar_preset_v_cl_o() {
    elementos_sel_ = {"Cl", "O", "V"};
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_var_ = "Cl2(g)";
    y_var_ = "O2(g)";
    x_min_ = -40.0;
    x_max_ = 0.0;
    y_min_ = -40.0;
    y_max_ = 0.0;
    fixed_var_ = "Temperature";
    usar_kelvin_ = false;
    fixed_val_ = 500.0;
}

void PanelEstabilidad::cargar_preset_s_cu_o_tp() {
    elementos_sel_ = {"Cu", "O", "S"};
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_var_ = "Temperature";
    y_var_ = "SO2(g)";
    usar_kelvin_ = false;
    x_min_ = 0.0;
    x_max_ = 1000.0;
    y_min_ = -5.0;
    y_max_ = 2.0;
    fixed_var_ = "O2(g)";
    fixed_val_ = 0.1; // pO2(g) = 0.1 bar (Image 3 & Manual Fig. 1-2)
}

void PanelEstabilidad::cargar_preset_cu_s_o_pp() {
    elementos_sel_ = {"Cu", "O", "S"};
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_var_ = "SO2(g)";
    y_var_ = "O2(g)";
    x_min_ = -15.0;
    x_max_ = 5.0;
    y_min_ = -25.0;
    y_max_ = 0.0;
    fixed_var_ = "Temperature";
    usar_kelvin_ = false;
    fixed_val_ = 700.0;
}

void PanelEstabilidad::cargar_preset_fe_s_o() {
    elementos_sel_ = {"Fe", "O", "S"};
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_var_ = "SO2(g)";
    y_var_ = "O2(g)";
    x_min_ = -15.0;
    x_max_ = 5.0;
    y_min_ = -30.0;
    y_max_ = 0.0;
    fixed_var_ = "Temperature";
    usar_kelvin_ = false;
    fixed_val_ = 727.0;
}

void PanelEstabilidad::cargar_preset_ti_cl_o() {
    elementos_sel_ = {"Cl", "O", "Ti"};
    incluir_organicos_ = false;
    actualizar_sistema(false);
    x_var_ = "Cl2(g)";
    y_var_ = "O2(g)";
    x_min_ = -40.0;
    x_max_ = 10.0;
    y_min_ = -65.0;
    y_max_ = 0.0;
    fixed_var_ = "Temperature";
    usar_kelvin_ = false;
    fixed_val_ = 300.0;
}

void PanelEstabilidad::ejecutar_calculo(bool forzar_en_actual) {
    cma::domain::ParametrosCalculoTpp params;
    params.elementos = elementos_sel_;
    params.elemento_principal = sistema_.elemento_principal;
    params.incluir_organicos = incluir_organicos_;
    params.temp_en_celsius = !usar_kelvin_;
    params.x_min = x_min_;
    params.x_max = x_max_;
    params.y_min = y_min_;
    params.y_max = y_max_;
    params.gas_y = y_var_;

    for (const auto& kv : especies_sel_) {
        if (kv.second) params.formulas_seleccionadas.push_back(kv.first);
    }

    if (x_var_ == "Temperature") {
        params.eje_x_es_temperatura = true;
        params.gas_fijo = fixed_var_;
        params.valor_fijo = (fixed_val_ > 0.0) ? fixed_val_ : 1e-20;
        params.presion_fija_en_bar = true;
    } else {
        params.eje_x_es_temperatura = false;
        params.gas_x = x_var_;
        params.valor_fijo = fixed_val_;
    }

    auto res = motor_.calcular_tpp(params);
    if (!res.ok) {
        estado_msg_ = res.mensaje_error;
        return;
    }
    estado_msg_.clear();

    if (pestanas_.empty() || (create_new_page_ && !forzar_en_actual)) {
        PestanaTpp nueva;
        ++contador_pestanas_;
        nueva.nombre_pestana = "Chart " + std::to_string(contador_pestanas_);
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

void PanelEstabilidad::renderizar() {
    // ── Barra de Menú Superior estilo HSC (File | Help) ──────────────────────
    if (ImGui::BeginMenuBar()) {
        if (ImGui::BeginMenu("File")) {
            if (ImGui::MenuItem("New Empty HSC Chart Tab")) {
                PestanaTpp vacia;
                ++contador_pestanas_;
                vacia.nombre_pestana = "Chart " + std::to_string(contador_pestanas_);
                vacia.calculado = false;
                pestanas_.push_back(vacia);
                pestana_activa_ = static_cast<int>(pestanas_.size()) - 1;
            }
            ImGui::Separator();
            if (ImGui::BeginMenu("Load HSC Reference Examples")) {
                if (ImGui::MenuItem("V-Cl-O System at 500 °C (p-p Diagram)")) {
                    cargar_preset_v_cl_o();
                    ejecutar_calculo(false);
                }
                if (ImGui::MenuItem("S-Cu-O System T-p Diagram (pO2 = 0.1 bar)")) {
                    cargar_preset_s_cu_o_tp();
                    ejecutar_calculo(false);
                }
                if (ImGui::MenuItem("Cu-S-O System at 700 °C (p-p Diagram)")) {
                    cargar_preset_cu_s_o_pp();
                    ejecutar_calculo(false);
                }
                if (ImGui::MenuItem("Fe-S-O System at 727 °C (p-p Diagram)")) {
                    cargar_preset_fe_s_o();
                    ejecutar_calculo(false);
                }
                if (ImGui::MenuItem("Ti-Cl-O System at 300 °C (p-p Diagram)")) {
                    cargar_preset_ti_cl_o();
                    ejecutar_calculo(false);
                }
                ImGui::EndMenu();
            }
            ImGui::Separator();
            if (ImGui::MenuItem("Open TPP File (*.tpp8, *.tpp)...")) {
                modo_guardar_ = false;
                modal_archivo_ = true;
            }
            if (ImGui::MenuItem("Save TPP File (*.tpp8, *.tpp)...")) {
                modo_guardar_ = true;
                modal_archivo_ = true;
            }
            ImGui::EndMenu();
        }
        if (ImGui::BeginMenu("Help")) {
            ImGui::TextColored(ImVec4(0.15f, 0.45f, 0.85f, 1.0f), "HSC Tpp Diagram Module (Minimum Gibbs Energy Area Graphics)");
            ImGui::Separator();
            ImGui::BulletText("1. Select three elements from the element list (e.g., V, Cl, O or Cu, S, O).");
            ImGui::BulletText("2. Select species one by one (Ctrl+click to toggle) or press Select All.");
            ImGui::BulletText("   Double-click any species to inspect its H, S, Cp thermodynamic data.");
            ImGui::BulletText("3. Select X-axis species (or Temperature for T-p diagrams) and Y-axis species.");
            ImGui::BulletText("4. Set Fixed variable (Temperature in p-p diagrams, or fixed gas pressure in T-p).");
            ImGui::BulletText("5. Press Draw Diagram.");
            ImGui::EndMenu();
        }
        ImGui::EndMenuBar();
    }

    // ── Layout Principal de 2 Columnas: Izquierda (Diagram Parameters) | Derecha (Charts) ─
    ImGui::BeginChild("##TppLeftPanel", ImVec2(355.0f, 0.0f), true);
    {
        ImGui::TextColored(ImVec4(0.28f, 0.34f, 0.44f, 1.0f), "Diagram Parameters");
        ImGui::Separator();

        if (ImGui::BeginTable("##SplitElemSpecies", 2, ImGuiTableFlags_SizingStretchSame)) {
            ImGui::TableSetupColumn("Select 3 Elements", ImGuiTableColumnFlags_WidthFixed, 152.0f);
            ImGui::TableSetupColumn("Select Species", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableNextRow();

            // Columna 1: Select 3 Elements (Grilla alfabética de 4 columnas exacta a HSC)
            ImGui::TableSetColumnIndex(0);
            ImGui::TextUnformatted("Select 3 Elements");
            ImGui::BeginChild("##ElemGridBox", ImVec2(150.0f, 305.0f), true);
            if (ImGui::BeginTable("##Grid88", 4, ImGuiTableFlags_SizingFixedFit | ImGuiTableFlags_NoPadInnerX)) {
                auto render_celda = [&](const char* sym) {
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
                            if (elementos_sel_.size() >= 3) {
                                elementos_sel_.erase(elementos_sel_.begin());
                            }
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
                    for (int c = 0; c < 4; ++c) render_celda(kGridElementosHSC[r][c]);
                }
                for (int r = 0; r < 2; ++r) {
                    ImGui::TableNextRow();
                    for (int c = 0; c < 4; ++c) render_celda(kGridElementosExtra[r][c]);
                }
                ImGui::TableNextRow();
                for (int c = 0; c < 4; ++c) render_celda(kGridElementosUltima[0][c]);

                ImGui::EndTable();
            }
            ImGui::EndChild();

            if (ImGui::Checkbox("Organics", &incluir_organicos_)) {
                if (elementos_sel_.size() == 3) actualizar_sistema(false);
            }

            // Columna 2: Select Species
            ImGui::TableSetColumnIndex(1);
            ImGui::TextUnformatted("Select Species");
            ImGui::BeginChild("##SpeciesListBox", ImVec2(0.0f, 305.0f), true);
            for (const auto& esp : sistema_.especies_sistema) {
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
                    if (!especies_sel_[esp.formula]) {
                        todas = false;
                        break;
                    }
                }
                for (const auto& esp : sistema_.especies_sistema) {
                    especies_sel_[esp.formula] = !todas;
                }
            }

            ImGui::EndTable();
        }

        ImGui::Spacing();
        ImGui::Separator();
        ImGui::TextUnformatted("Diagram parameters");

        // X-axis variable & Y-axis variable
        if (ImGui::BeginTable("##AxisVarsTable", 2, ImGuiTableFlags_SizingStretchSame)) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::TextUnformatted("X-axis variable");
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::BeginCombo("##XAxisCombo", x_var_.c_str())) {
                // Opción Temperature (modo T-p)
                {
                    bool sel = (x_var_ == "Temperature");
                    if (ImGui::Selectable("Temperature", sel)) {
                        x_var_ = "Temperature";
                        x_min_ = usar_kelvin_ ? 298.15 : 0.0;
                        x_max_ = usar_kelvin_ ? 1273.15 : 1000.0;
                        for (const auto& g : sistema_.especies_gas_ejes) {
                            if (g.formula != y_var_) {
                                fixed_var_ = g.formula;
                                break;
                            }
                        }
                        fixed_val_ = 1e-20;
                    }
                }
                for (const auto& g : sistema_.especies_gas_ejes) {
                    bool sel = (x_var_ == g.formula);
                    if (ImGui::Selectable(g.formula.c_str(), sel)) {
                        x_var_ = g.formula;
                        x_min_ = -40.0;
                        x_max_ = 0.0;
                        fixed_var_ = "Temperature";
                        fixed_val_ = usar_kelvin_ ? 298.15 : 25.0;
                    }
                }
                ImGui::EndCombo();
            }

            ImGui::TableSetColumnIndex(1);
            ImGui::TextUnformatted("Y-axis variable");
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::BeginCombo("##YAxisCombo", y_var_.c_str())) {
                for (const auto& g : sistema_.especies_gas_ejes) {
                    bool sel = (y_var_ == g.formula);
                    if (ImGui::Selectable(g.formula.c_str(), sel)) {
                        y_var_ = g.formula;
                        y_min_ = -40.0;
                        y_max_ = 0.0;
                    }
                }
                ImGui::EndCombo();
            }

            // Subtítulos dinámicos sobre Min/Max ("°C" o "log pCl2(g)")
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            if (x_var_ == "Temperature") {
                ImGui::Text("      %s", usar_kelvin_ ? "K" : "°C");
            } else if (!x_var_.empty()) {
                ImGui::Text("      log p%s", x_var_.c_str());
            }

            ImGui::TableSetColumnIndex(1);
            if (!y_var_.empty()) {
                ImGui::Text("      log p%s", y_var_.c_str());
            }

            // Inputs Min / Max
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::AlignTextToFramePadding();
            ImGui::TextUnformatted("Min");
            ImGui::SameLine(36.0f);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##XMinTpp", &x_min_, 0.0, 0.0, "%.2f");

            ImGui::AlignTextToFramePadding();
            ImGui::TextUnformatted("Max");
            ImGui::SameLine(36.0f);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##XMaxTpp", &x_max_, 0.0, 0.0, "%.2f");

            ImGui::TableSetColumnIndex(1);
            ImGui::AlignTextToFramePadding();
            ImGui::TextUnformatted("Min");
            ImGui::SameLine(36.0f);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##YMinTpp", &y_min_, 0.0, 0.0, "%.2f");

            ImGui::AlignTextToFramePadding();
            ImGui::TextUnformatted("Max");
            ImGui::SameLine(36.0f);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##YMaxTpp", &y_max_, 0.0, 0.0, "%.2f");

            ImGui::EndTable();
        }

        ImGui::Spacing();
        ImGui::TextUnformatted("Fixed variable");
        bool es_modo_tp = (x_var_ == "Temperature");
        if (!es_modo_tp) {
            fixed_var_ = "Temperature";
            ImGui::BeginDisabled();
            ImGui::SetNextItemWidth(115.0f);
            if (ImGui::BeginCombo("##FixedComboDisabled", "Temperature")) {
                ImGui::EndCombo();
            }
            ImGui::EndDisabled();
        } else {
            int elem_y = 2;
            for (const auto& g : sistema_.especies_gas_ejes) {
                if (g.formula == y_var_) {
                    elem_y = contar_elementos_distintos(g);
                    break;
                }
            }
            ImGui::SetNextItemWidth(115.0f);
            if (ImGui::BeginCombo("##FixedComboActive", fixed_var_.c_str())) {
                for (const auto& g : sistema_.especies_gas_ejes) {
                    if (g.formula == y_var_) continue;
                    if (contar_elementos_distintos(g) > elem_y) continue;
                    bool sel = (fixed_var_ == g.formula);
                    if (ImGui::Selectable(g.formula.c_str(), sel)) {
                        fixed_var_ = g.formula;
                    }
                }
                ImGui::EndCombo();
            }
        }

        ImGui::SameLine();
        ImGui::SetNextItemWidth(115.0f);
        ImGui::InputDouble("##FixedValInput", &fixed_val_, 0.0, 0.0, "%g");
        ImGui::SameLine();
        if (ImGui::Button(usar_kelvin_ ? "K##UnitBtn" : "°C##UnitBtn", ImVec2(58.0f, 0.0f))) {
            if (!usar_kelvin_) {
                usar_kelvin_ = true;
                if (!es_modo_tp) fixed_val_ += 273.15;
                else { x_min_ += 273.15; x_max_ += 273.15; }
            } else {
                usar_kelvin_ = false;
                if (!es_modo_tp) fixed_val_ -= 273.15;
                else { x_min_ -= 273.15; x_max_ -= 273.15; }
            }
        }

        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Checkbox("Show colors", &show_colors_);
        ImGui::SameLine(175.0f);
        ImGui::Checkbox("Gridlines", &show_grid_);
        ImGui::Checkbox("Create new page", &create_new_page_);

        ImGui::Spacing();
        if (ImGui::Button("Draw Diagram", ImVec2(165.0f, 28.0f))) {
            ejecutar_calculo(false);
        }

        if (!estado_msg_.empty()) {
            ImGui::TextWrapped("%s", estado_msg_.c_str());
        }
    }
    ImGui::EndChild();

    ImGui::SameLine();

    // ── Columna Derecha: Pestañas de Diagramas (Chart 1, Chart 2, ...) ───────
    ImGui::BeginChild("##TppRightPanel", ImVec2(0.0f, 0.0f), true);
    {
        if (ImGui::BeginTabBar("##TppChartsTabBar", ImGuiTabBarFlags_Reorderable | ImGuiTabBarFlags_AutoSelectNewTabs)) {
            for (int i = 0; i < static_cast<int>(pestanas_.size()); ++i) {
                bool abierta = true;
                if (ImGui::BeginTabItem(pestanas_[i].nombre_pestana.c_str(), &abierta)) {
                    pestana_activa_ = i;
                    auto& tab = pestanas_[i];

                    if (ImGui::SmallButton("Redraw (Reset Zoom)")) {
                        tab.ajustar_ejes = true;
                    }
                    ImGui::SameLine();
                    if (ImGui::SmallButton("Edit Titles / Scales")) {
                        std::snprintf(edit_titulo_, sizeof(edit_titulo_), "%s",
                                      tab.calculado ? tab.resultado.titulo_diagrama.c_str() : "HSC Chart");
                        std::snprintf(edit_x_, sizeof(edit_x_), "%s",
                                      tab.calculado ? tab.resultado.etiqueta_eje_x.c_str() : "X axis title");
                        std::snprintf(edit_y_, sizeof(edit_y_), "%s",
                                      tab.calculado ? tab.resultado.etiqueta_eje_y.c_str() : "Y axis title");
                        modal_editar_ejes_ = true;
                    }
                    ImGui::SameLine();
                    if (ImGui::SmallButton("Clone Tab")) {
                        PestanaTpp clon = tab;
                        ++contador_pestanas_;
                        clon.nombre_pestana = "Chart " + std::to_string(contador_pestanas_);
                        pestanas_.push_back(std::move(clon));
                    }
                    ImGui::SameLine();
                    if (ImGui::SmallButton("Copy Summary")) {
                        if (tab.calculado) {
                            std::ostringstream oss;
                            oss << tab.resultado.titulo_diagrama << "\n" << tab.resultado.subtitulo_constante << "\n";
                            for (const auto& pol : tab.resultado.poligonos) {
                                oss << " - " << pol.etiqueta_grafico << " (centroid: " << pol.centroide.x << ", " << pol.centroide.y << ")\n";
                            }
                            ImGui::SetClipboardText(oss.str().c_str());
                        }
                    }

                    const char* titulo_mostrar = tab.calculado ? tab.resultado.titulo_diagrama.c_str() : "HSC Chart";
                    float ancho_disp = ImGui::GetContentRegionAvail().x;
                    float ancho_txt = ImGui::CalcTextSize(titulo_mostrar).x;
                    ImGui::SetCursorPosX(std::max(10.0f, (ancho_disp - ancho_txt) * 0.5f));
                    ImGui::TextColored(ImVec4(0.05f, 0.05f, 0.08f, 1.0f), "%s", titulo_mostrar);

                    ImVec2 plot_size(ImGui::GetContentRegionAvail().x, ImGui::GetContentRegionAvail().y - 26.0f);
                    ImPlotFlags pflags = ImPlotFlags_NoLegend | ImPlotFlags_NoTitle;
                    if (ImPlot::BeginPlot("##TppPlotCanvas", plot_size, pflags)) {
                        ImPlotAxisFlags ax_flags = show_grid_ ? ImPlotAxisFlags_None : ImPlotAxisFlags_NoGridLines;
                        const char* lbl_x = tab.calculado ? tab.resultado.etiqueta_eje_x.c_str() : "X axis title";
                        const char* lbl_y = tab.calculado ? tab.resultado.etiqueta_eje_y.c_str() : "Y axis title";
                        ImPlot::SetupAxes(lbl_x, lbl_y, ax_flags, ax_flags);

                        double rx0 = tab.calculado ? tab.resultado.x_min : 0.0;
                        double rx1 = tab.calculado ? tab.resultado.x_max : 10.0;
                        double ry0 = tab.calculado ? tab.resultado.y_min : 0.0;
                        double ry1 = tab.calculado ? tab.resultado.y_max : 10.0;

                        ImPlotCond cond = tab.ajustar_ejes ? ImPlotCond_Always : ImPlotCond_Once;
                        ImPlot::SetupAxisLimits(ImAxis_X1, rx0, rx1, cond);
                        ImPlot::SetupAxisLimits(ImAxis_Y1, ry0, ry1, cond);
                        tab.ajustar_ejes = false;

                        if (tab.calculado) {
                            ImDrawList* dl = ImPlot::GetPlotDrawList();
                            ImPlot::PushPlotClipRect();

                            if (tab.resultado.modo == cma::domain::StabilityDiagramMode::Tpp_PartialPressure) {
                                // 1) Áreas coloreadas: Polígonos convexos (modo p-p)
                                for (size_t p_idx = 0; p_idx < tab.resultado.poligonos.size(); ++p_idx) {
                                    const auto& pol = tab.resultado.poligonos[p_idx];
                                    if (pol.vertices.size() < 3) continue;

                                    std::vector<ImVec2> pts;
                                    pts.reserve(pol.vertices.size());
                                    for (const auto& v : pol.vertices) {
                                        pts.push_back(ImPlot::PlotToPixels(ImPlotPoint(v.x, v.y)));
                                    }
                                    if (show_colors_) {
                                        dl->AddConvexPolyFilled(pts.data(), static_cast<int>(pts.size()),
                                                                color_hsc_especie(pol.etiqueta_grafico, p_idx));
                                    }
                                    dl->AddPolyline(pts.data(), static_cast<int>(pts.size()),
                                                    IM_COL32(20, 20, 20, 255), ImDrawFlags_Closed, 1.35f);
                                }
                            } else {
                                // 2) Áreas coloreadas: Trapecios verticales convexos (modo T-p)
                                for (size_t p_idx = 0; p_idx < tab.resultado.poligonos.size(); ++p_idx) {
                                    const auto& pol = tab.resultado.poligonos[p_idx];
                                    ImU32 col = color_hsc_especie(pol.etiqueta_grafico, p_idx);
                                    for (const auto& tr : pol.trapecios) {
                                        ImVec2 quad[4] = {
                                            ImPlot::PlotToPixels(ImPlotPoint(tr.x0, tr.y0_bot)),
                                            ImPlot::PlotToPixels(ImPlotPoint(tr.x1, tr.y1_bot)),
                                            ImPlot::PlotToPixels(ImPlotPoint(tr.x1, tr.y1_top)),
                                            ImPlot::PlotToPixels(ImPlotPoint(tr.x0, tr.y0_top))
                                        };
                                        if (show_colors_) {
                                            dl->AddConvexPolyFilled(quad, 4, col);
                                        }
                                    }
                                    if (pol.vertices.size() >= 3) {
                                        std::vector<ImVec2> pts;
                                        pts.reserve(pol.vertices.size());
                                        for (const auto& v : pol.vertices) {
                                            pts.push_back(ImPlot::PlotToPixels(ImPlotPoint(v.x, v.y)));
                                        }
                                        dl->AddPolyline(pts.data(), static_cast<int>(pts.size()),
                                                        IM_COL32(15, 15, 15, 255), ImDrawFlags_Closed, 1.45f);
                                    }
                                }
                            }

                            // 3) Etiquetas centradas de cada especie predominante
                            for (const auto& pol : tab.resultado.poligonos) {
                                const std::string& lbl = pol.etiqueta_grafico.empty() ? pol.formula : pol.etiqueta_grafico;
                                ImVec2 pos = ImPlot::PlotToPixels(ImPlotPoint(pol.centroide.x, pol.centroide.y));
                                ImVec2 tsize = ImGui::CalcTextSize(lbl.c_str());
                                ImVec2 text_pos(pos.x - tsize.x * 0.5f, pos.y - tsize.y * 0.5f);
                                dl->AddText(ImVec2(text_pos.x + 1.0f, text_pos.y + 1.0f),
                                            IM_COL32(255, 255, 255, 160), lbl.c_str());
                                dl->AddText(text_pos, IM_COL32(10, 10, 10, 255), lbl.c_str());
                            }

                            ImPlot::PopPlotClipRect();
                        }

                        ImPlot::EndPlot();
                    }

                    if (tab.calculado) {
                        ImGui::TextUnformatted(tab.resultado.subtitulo_constante.c_str());
                    } else {
                        ImGui::TextDisabled("Select 3 elements and press Draw Diagram.");
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

    // ── Modal de Inspección Termodinámica al hacer Doble Clic en una Especie ─
    if (modal_especie_) {
        ImGui::OpenPopup("Thermodynamic Data Inspector##TppModal");
        modal_especie_ = false;
    }
    if (ImGui::BeginPopupModal("Thermodynamic Data Inspector##TppModal", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        for (const auto& esp : sistema_.especies_sistema) {
            if (esp.formula == especie_inspeccionada_) {
                ImGui::TextColored(ImVec4(0.12f, 0.42f, 0.82f, 1.0f), "Species: %s (%s)",
                                   esp.formula.c_str(), esp.nombre.c_str());
                ImGui::Separator();
                ImGui::Text("State: %s", esp.es_liquido ? "Liquid (l)" : "Condensed Solid (s)");
                ImGui::Text("ΔHf°(298.15 K): %.3f kJ/mol", esp.h_formacion_298);
                ImGui::Text("S°(298.15 K):   %.3f J/(mol·K)", esp.s_298);
                ImGui::Text("Cp(298.15 K):   %.3f J/(mol·K)", esp.cp_298);
                ImGui::Separator();
                ImGui::Text("Stoichiometry:");
                for (const auto& kv : esp.composicion) {
                    ImGui::BulletText("%s : %.4g", kv.first.c_str(), kv.second);
                }
                break;
            }
        }
        if (ImGui::Button("Close", ImVec2(120.0f, 0.0f))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }

    // ── Modal para editar títulos y escalas (HSC: doble clic / botón Edit) ────
    if (modal_editar_ejes_) {
        ImGui::OpenPopup("Edit Diagram Titles & Axes##TppAxesModal");
        modal_editar_ejes_ = false;
    }
    if (ImGui::BeginPopupModal("Edit Diagram Titles & Axes##TppAxesModal", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::InputText("Chart Title", edit_titulo_, sizeof(edit_titulo_));
        ImGui::InputText("X-Axis Title", edit_x_, sizeof(edit_x_));
        ImGui::InputText("Y-Axis Title", edit_y_, sizeof(edit_y_));
        ImGui::Separator();
        if (ImGui::Button("Apply", ImVec2(110.0f, 0.0f))) {
            if (!pestanas_.empty()) {
                auto& tab = pestanas_[std::clamp(pestana_activa_, 0, static_cast<int>(pestanas_.size()) - 1)];
                tab.resultado.titulo_diagrama = edit_titulo_;
                tab.resultado.etiqueta_eje_x = edit_x_;
                tab.resultado.etiqueta_eje_y = edit_y_;
            }
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button("Cancel", ImVec2(110.0f, 0.0f))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }

    // ── Modal para Abrir / Guardar archivo .tpp8 ─────────────────────────────
    if (modal_archivo_) {
        ImGui::OpenPopup("TPP8 File Manager##TppFileModal");
        modal_archivo_ = false;
    }
    if (ImGui::BeginPopupModal("TPP8 File Manager##TppFileModal", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::TextUnformatted(modo_guardar_ ? "Save TPP Specification (*.tpp8)" : "Open TPP Specification (*.tpp8)");
        ImGui::InputText("File path", ruta_archivo_, sizeof(ruta_archivo_));
        if (ImGui::Button(modo_guardar_ ? "Save" : "Open", ImVec2(110.0f, 0.0f))) {
            if (modo_guardar_) {
                cma::domain::ParametrosCalculoTpp p;
                p.elementos = elementos_sel_;
                p.elemento_principal = sistema_.elemento_principal;
                p.eje_x_es_temperatura = (x_var_ == "Temperature");
                p.temp_en_celsius = !usar_kelvin_;
                p.gas_x = x_var_;
                p.gas_y = y_var_;
                p.gas_fijo = fixed_var_;
                p.x_min = x_min_;
                p.x_max = x_max_;
                p.y_min = y_min_;
                p.y_max = y_max_;
                p.valor_fijo = fixed_val_;
                p.incluir_organicos = incluir_organicos_;
                std::ofstream ofs(ruta_archivo_);
                if (ofs) ofs << motor_.serializar_tpp(p);
            } else {
                std::ifstream ifs(ruta_archivo_);
                if (ifs) {
                    std::ostringstream ss;
                    ss << ifs.rdbuf();
                    cma::domain::ParametrosCalculoTpp p;
                    if (motor_.deserializar_tpp(ss.str(), p) && p.elementos.size() == 3) {
                        elementos_sel_ = p.elementos;
                        incluir_organicos_ = p.incluir_organicos;
                        actualizar_sistema(false);
                        x_var_ = p.eje_x_es_temperatura ? "Temperature" : p.gas_x;
                        y_var_ = p.gas_y;
                        fixed_var_ = p.eje_x_es_temperatura ? p.gas_fijo : "Temperature";
                        usar_kelvin_ = !p.temp_en_celsius;
                        x_min_ = p.x_min;
                        x_max_ = p.x_max;
                        y_min_ = p.y_min;
                        y_max_ = p.y_max;
                        fixed_val_ = p.valor_fijo;
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
