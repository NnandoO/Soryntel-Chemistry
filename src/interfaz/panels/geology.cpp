#include "shared.hpp"
#include "interfaz/ui_css_style.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <sstream>

using namespace cma::interfaz;

namespace {

struct ElementData {
    const char* symbol;
    int row;
    int col;
    bool in_geo_db; // true = white box in HSC Geo (element in mineral database), false = dark grey box
};

// Disposición exacta de la Tabla Periódica de HSC Geo 8.0 (media_1793656361746_21c5f76a.png)
const ElementData PT_ELEMENTS[] = {
    {"H", 0, 0, false},  {"He", 0, 17, false},
    {"Li", 1, 0, false}, {"Be", 1, 1, false},
    {"B", 1, 12, false}, {"C", 1, 13, true},  {"N", 1, 14, false}, {"O", 1, 15, false}, {"F", 1, 16, false}, {"Ne", 1, 17, false},
    {"Na", 2, 0, true},  {"Mg", 2, 1, true},
    {"Al", 2, 12, true}, {"Si", 2, 13, true}, {"P", 2, 14, true},  {"S", 2, 15, true},  {"Cl", 2, 16, true}, {"Ar", 2, 17, false},
    {"K", 3, 0, true},   {"Ca", 3, 1, true},
    {"Sc", 3, 2, false}, {"Ti", 3, 3, true},  {"V", 3, 4, true},   {"Cr", 3, 5, true},  {"Mn", 3, 6, true},  {"Fe", 3, 7, true},
    {"Co", 3, 8, true},  {"Ni", 3, 9, true},  {"Cu", 3, 10, true}, {"Zn", 3, 11, true},
    {"Ga", 3, 12, false},{"Ge", 3, 13, false},{"As", 3, 14, true}, {"Se", 3, 15, true}, {"Br", 3, 16, false},{"Kr", 3, 17, false},
    {"Rb", 4, 0, true},  {"Sr", 4, 1, true},
    {"Y", 4, 2, true},   {"Zr", 4, 3, true},  {"Nb", 4, 4, true},  {"Mo", 4, 5, true},  {"Tc", 4, 6, false}, {"Ru", 4, 7, false},
    {"Rh", 4, 8, false}, {"Pd", 4, 9, false}, {"Ag", 4, 10, true}, {"Cd", 4, 11, false},
    {"In", 4, 12, false},{"Sn", 4, 13, true}, {"Sb", 4, 14, true}, {"Te", 4, 15, true}, {"I", 4, 16, false}, {"Xe", 4, 17, false},
    {"Cs", 5, 0, true},  {"Ba", 5, 1, true},
    {"Hf", 5, 3, false}, {"Ta", 5, 4, true},  {"W", 5, 5, true},   {"Re", 5, 6, false}, {"Os", 5, 7, false}, {"Ir", 5, 8, false},
    {"Pt", 5, 9, false}, {"Au", 5, 10, false},{"Hg", 5, 11, false},
    {"Tl", 5, 12, false},{"Pb", 5, 13, true}, {"Bi", 5, 14, true}, {"Po", 5, 15, false},{"At", 5, 16, false},{"Rn", 5, 17, false},
    {"Fr", 6, 0, false}, {"Ra", 6, 1, false},
    // Lantánidos (fila 7, cols 3..16)
    {"La", 7, 3, true},  {"Ce", 7, 4, false}, {"Pr", 7, 5, false}, {"Nd", 7, 6, false}, {"Pm", 7, 7, false},
    {"Sm", 7, 8, false}, {"Eu", 7, 9, false}, {"Gd", 7, 10, false},{"Tb", 7, 11, false},{"Dy", 7, 12, false},
    {"Ho", 7, 13, false},{"Er", 7, 14, false},{"Tm", 7, 15, false},{"Yb", 7, 16, false},{"Lu", 7, 17, false},
    // Actínidos (fila 8, cols 3..16)
    {"Ac", 8, 3, false}, {"Th", 8, 4, true},  {"Pa", 8, 5, false}, {"U", 8, 6, true},   {"Np", 8, 7, false},
    {"Pu", 8, 8, false}, {"Am", 8, 9, false}, {"Cm", 8, 10, false},{"Bk", 8, 11, false},{"Cf", 8, 12, false},
    {"Es", 8, 13, false},{"Fm", 8, 14, false},{"Md", 8, 15, false}
};

static const char* kGeoSourcesList[7] = {
    "Bruvann.xls - Header",
    "Bruvann.xls - REE",
    "Bruvann.xls - PGE",
    "Bruvann.xls - XRF",
    "Bruvann.xls - BM",
    "Bruvann.xls - LECO",
    "Bruvann.xls - LOI"
};

static const char* kMineralClasses[7] = {
    "All", "Sulfides", "Silicates", "Oxides", "Carbonates", "Phosphates", "Sulfates"
};

} // namespace

namespace cma::interfaz {

PanelGeoMineralogical::PanelGeoMineralogical() {
    cargar_fuente_datos(0);
    entradas_formula_ = {
        {"Cu", 34.50},
        {"Fe", 30.55},
        {"S",  34.95}
    };
    res_formula_ = mineral_db_.calcular_formula_mineral(entradas_formula_, elem_base_norm_, num_atomos_norm_);
    res_cipw_ = mineral_db_.calcular_norma_cipw(roca_oxidos_);
}

void PanelGeoMineralogical::cargar_fuente_datos(int idx) {
    const auto& muestras = mineral_db_.muestras_predefinidas();
    if (muestras.empty()) return;
    if (idx < 0 || idx >= static_cast<int>(muestras.size())) idx = 0;
    idx_data_source_ = idx;

    const auto& m = muestras[static_cast<size_t>(idx)];
    std::snprintf(data_source_, sizeof(data_source_), "%s", m.nombre_fuente.c_str());
    minerales_candidatos_ = m.minerales_candidatos;
    ensayo_actual_ = m.ensayo_elem_wt;
    idx_mineral_cand_ = (minerales_candidatos_.size() > 1) ? 1 : 0;

    inicializar_rondas_bruvann();
    ejecutar_calculo_modal();
}

void PanelGeoMineralogical::inicializar_rondas_bruvann() {
    rondas_.clear();
    if (idx_data_source_ == 0) {
        rondas_.push_back({"Round 1", {"Chalcopyrite (Erora, Portugal)"}, {"Cu"}});
        rondas_.push_back({"Round 2", {"Pyrite (The Wilhelmine mine, Spessart,Bavaria)", "Galena (Kansan, USA)"}, {"S", "Pb"}});
        rondas_.push_back({"Round 3", {"Quartz (Kondyor PGE Placer Deposit, Russia)", "Olivine (Western Bushveld complex, South Africa)"}, {"Si", "Mg"}});
    } else if (idx_data_source_ == 1) {
        rondas_.push_back({"Round 1", {"Chalcopyrite (Erora, Portugal)", "Pentlandite (Bruvann, Rana, Norway)", "Pyrrhotite (Sudbury, Ontario, Canada)"}, {"Cu", "Ni", "S"}});
        rondas_.push_back({"Round 2", {"Chromite (Bushveld UG2 Reef, South Africa)", "Anorthite (Stillwater Complex, Montana, USA)", "Enstatite (Great Dyke, Zimbabwe)"}, {"Cr", "Ca", "Mg"}});
    } else {
        rondas_.push_back({"Round 1", {"Fluorapatite (Khibiny Massif, Kola, Russia)", "Calcite (Sweetwater Mine, Missouri, USA)", "Quartz (Kondyor PGE Placer Deposit, Russia)"}, {"P", "C", "Si"}});
        rondas_.push_back({"Round 2", {"Magnetite (Kiruna, Norrbotten, Sweden)", "Hematite (Carajas, Para, Brazil)"}, {"Fe", "O"}});
    }
    idx_ronda_activa_ = 0;
    elemento_activo_pt_ = "Cu";
}

void PanelGeoMineralogical::ejecutar_calculo_modal() {
    auto met = static_cast<cma::application::modules::MetodoModal>(current_method_);
    ultimo_resultado_modal_ = mineral_db_.resolver_modal_rondas(ensayo_actual_, rondas_, met);
    modal_calculado_ = true;
}

void PanelGeoMineralogical::refrescar_datos() {
    if (!flowsheet_) return;
    auto& gestor = base_datos::GestorDB::instancia();
    for (const auto& [fid, fl] : flowsheet_->mapa_flujos()) {
        for (const auto& [esp_id, mol] : fl.flujo_molar) {
            if (mol <= 0.0) continue;
            const auto* esp = gestor.especie_por_id(esp_id);
            if (!esp || esp->fase_std != nucleo::Fase::Solido) continue;
            if (!mineral_db_.buscar_por_nombre(esp->nombre)) {
                cma::application::modules::FichaMineralGeo f;
                f.nombre = esp->nombre + " (" + fl.nombre + ")";
                f.nombre_corto = esp->nombre;
                f.simbolo_ima = esp->formula.substr(0, std::min<size_t>(3, esp->formula.size()));
                f.formula = esp->formula;
                f.clase_mineral = "Flowsheet Solids";
                f.sistema_cristalino = "Crystalline";
                f.localidad = fl.nombre;
                f.referencia = "HSC Sim Flowsheet";
                f.gravedad_especifica = 3.20;
                f.dureza_mohs = 4.5;
                f.color = "Metallic / Grey";
                f.elem_wt_pct = cma::application::modules::MineralDB::calcular_wt_elemental(esp->formula, &f.masa_molar);
                mineral_db_.catalogo().push_back(f);
            }
        }
    }
}

void PanelGeoMineralogical::dibujar_ribbon_superior() {
    const auto tk = css::tokens();

    // Pestaña superior "Main" estilo cinta HSC Geo 8.0
    if (ImGui::BeginTabBar("##HSCGeoMainRibbonTab")) {
        if (ImGui::BeginTabItem("Main")) {
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }

    css::begin_ribbon_bar("##HSCGeoRibbon", 68.0f);
    const float bar_bottom_y = ImGui::GetWindowPos().y + ImGui::GetWindowHeight();

    // ── Grupo Home (New | Open Data | Save DB) ──
    const float g1_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("geo_new", "New", "", css::RibbonIcon::NewDoc, 48.0f, 46.0f)) {
        inicializar_rondas_bruvann();
        ejecutar_calculo_modal();
    }
    ImGui::SameLine(0.0f, 4.0f);
    if (css::ribbon_button("geo_open", "Open", "Data", css::RibbonIcon::FolderOpen, 50.0f, 46.0f)) {
        idx_data_source_ = (idx_data_source_ + 1) % static_cast<int>(mineral_db_.muestras_predefinidas().size());
        cargar_fuente_datos(idx_data_source_);
    }
    ImGui::SameLine(0.0f, 4.0f);
    if (css::ribbon_button("geo_save", "Save", "CSV", css::RibbonIcon::Save, 48.0f, 46.0f)) {
        std::ofstream ofs("hsc_geo_modal_results.csv");
        if (ofs.is_open()) {
            ofs << "Mineral,Formula,Round,Modal_wt_pct,Modal_vol_pct,Sigma\n";
            for (const auto& f : ultimo_resultado_modal_.fases) {
                ofs << "\"" << f.nombre_mineral << "\"," << f.formula << ","
                    << f.ronda_resuelta << "," << f.wt_pct << "," << f.vol_pct << "," << f.sigma_error << "\n";
            }
            resultado_geo_ = "Resultados exportados a hsc_geo_modal_results.csv";
            mostrar_resultado_ = true;
        }
    }
    const float g1_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g1_start, g1_end, bar_bottom_y, "Home");

    // ── Grupo Window (Window | Tile Horizontal | Tile Vertical | Cascade) ──
    ImGui::SameLine(0.0f, 12.0f);
    const float g2_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("geo_win_h", "Tile", "Horizontal", css::RibbonIcon::WindowTileH, 64.0f, 46.0f)) {
        feature_activa_ = 4;
        tab_modal_ = 0;
    }
    ImGui::SameLine(0.0f, 4.0f);
    if (css::ribbon_button("geo_win_v", "Tile", "Vertical", css::RibbonIcon::WindowTileV, 58.0f, 46.0f)) {
        feature_activa_ = 0;
    }
    ImGui::SameLine(0.0f, 4.0f);
    if (css::ribbon_button("geo_win_c", "Cascade", "", css::RibbonIcon::WindowCascade, 56.0f, 46.0f)) {
        feature_activa_ = 4;
        tab_modal_ = 1;
    }
    const float g2_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g2_start, g2_end, bar_bottom_y, "Window");

    // ── Grupo Help (About) ──
    ImGui::SameLine(0.0f, 12.0f);
    const float g3_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("geo_about", "About", "", css::RibbonIcon::Help, 50.0f, 46.0f)) {
        mostrar_about_ = true;
    }
    const float g3_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g3_start, g3_end, bar_bottom_y, "Help");

    // Resumen rápido a la derecha del Ribbon
    ImGui::SameLine(0.0f, 18.0f);
    ImGui::BeginGroup();
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_blue),
                       "HSC Geo 8.0 — Mineralogy, Modal & Petrological Suite");
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.text_secondary),
                       "Catalog: %zu minerals | Source: %s | Modal Total: %.2f wt-%% (RMSE: %.3f%%)",
                       mineral_db_.catalogo().size(), data_source_,
                       ultimo_resultado_modal_.total_mineral_wt,
                       ultimo_resultado_modal_.rmse_elemental);
    ImGui::EndGroup();

    css::end_ribbon_bar();
}

void PanelGeoMineralogical::dibujar_vista_modal() {
    const auto tk = css::tokens();
    const float avail_w = ImGui::GetContentRegionAvail().x;
    const float right_identify_w = std::clamp(avail_w * 0.19f, 185.0f, 235.0f);
    const float left_setup_w = 152.0f;
    const float center_w = std::max(380.0f, avail_w - left_setup_w - right_identify_w - 14.0f);

    // ── Sub-columna 1: "Modal Calculation:" (1. Data Source, 2. Minerals, 3. Rules) ──
    ImGui::BeginChild("##GeoModalSetupCol", ImVec2(left_setup_w, -32.0f), true);
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.text_primary), "Modal Calculation:");
    ImGui::Separator();

    ImGui::Text("1. Data Source");
    ImGui::SetNextItemWidth(-1.0f);
    const auto& muestras = mineral_db_.muestras_predefinidas();
    if (ImGui::BeginCombo("##GeoSourceCombo", data_source_)) {
        for (int i = 0; i < static_cast<int>(muestras.size()); ++i) {
            bool sel = (i == idx_data_source_);
            if (ImGui::Selectable(muestras[static_cast<size_t>(i)].nombre_fuente.c_str(), sel)) {
                cargar_fuente_datos(i);
            }
        }
        ImGui::EndCombo();
    }
    if (ImGui::Button("Show Data", ImVec2(-1.0f, 24.0f))) {
        mostrar_datos_ = true;
    }

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Text("2. Minerals");
    if (ImGui::Button("Define", ImVec2(-1.0f, 24.0f))) {
        feature_activa_ = 1; // Abre el selector Find Mineral
    }

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Text("3. Rules");
    if (ImGui::Button("Export", ImVec2(-1.0f, 24.0f))) {
        std::ofstream ofs("hsc_geo_rules.txt");
        if (ofs.is_open()) {
            for (const auto& r : rondas_) {
                ofs << "[" << r.nombre << "]\nPhases=";
                for (size_t i = 0; i < r.fases.size(); ++i) ofs << (i ? ";" : "") << r.fases[i];
                ofs << "\nElements=";
                for (size_t i = 0; i < r.elementos.size(); ++i) ofs << (i ? ";" : "") << r.elementos[i];
                ofs << "\n";
            }
            resultado_geo_ = "Reglas de rondas exportadas a hsc_geo_rules.txt";
            mostrar_resultado_ = true;
        }
    }
    if (ImGui::Button("Import", ImVec2(-1.0f, 24.0f))) {
        refrescar_datos();
        resultado_geo_ = "Catálogo sincronizado con especies sólidas del Flowsheet.";
        mostrar_resultado_ = true;
    }
    if (ImGui::Button("Wizard", ImVec2(-1.0f, 24.0f))) {
        mostrar_wizard_ = true;
    }
    if (ImGui::Button("Reset", ImVec2(-1.0f, 24.0f))) {
        cargar_fuente_datos(idx_data_source_);
    }
    ImGui::EndChild();

    ImGui::SameLine(0.0f, 6.0f);

    // ── Sub-columna 2 (Centro): Pestañas "Calculation" | "Error Estimation" ──
    ImGui::BeginChild("##GeoModalCenterCol", ImVec2(center_w, -32.0f), false);
    if (ImGui::BeginTabBar("##GeoModalCalcTabs")) {
        if (ImGui::BeginTabItem("Calculation")) {
            tab_modal_ = 0;

            // Bloque superior: Tabla Periódica HSC Geo + Tabla derecha "Element + SHIFT" (Phase | Element %)
            const float top_h = 228.0f;
            ImGui::BeginChild("##PTAndElemShift", ImVec2(0.0f, top_h), true);
            const float pt_avail_w = ImGui::GetContentRegionAvail().x;
            const float elem_table_w = std::clamp(pt_avail_w * 0.30f, 165.0f, 235.0f);
            const float pt_grid_w = std::max(260.0f, pt_avail_w - elem_table_w - 8.0f);

            ImGui::BeginChild("##PTGridChild", ImVec2(pt_grid_w, 0.0f), false, ImGuiWindowFlags_NoScrollbar);
            const float btn_w = std::clamp((pt_grid_w - 12.0f) / 18.0f - 1.5f, 18.0f, 25.0f);
            const float btn_h = 20.0f;
            const float sp = 1.5f;
            ImVec2 start_pos = ImGui::GetCursorPos();

            constexpr int n_pt = static_cast<int>(sizeof(PT_ELEMENTS) / sizeof(PT_ELEMENTS[0]));
            for (int i = 0; i < n_pt; ++i) {
                const auto& el = PT_ELEMENTS[i];
                if (!el.symbol) continue;
                float px = start_pos.x + el.col * (btn_w + sp);
                float py = start_pos.y + el.row * (btn_h + sp);
                if (el.row >= 7) py += 6.0f;
                ImGui::SetCursorPos(ImVec2(px, py));

                const bool is_active_el = (elemento_activo_pt_ == el.symbol);
                const bool in_assay = (ensayo_actual_.find(el.symbol) != ensayo_actual_.end());

                // Estilo idéntico a la captura HSC Geo 8.0:
                // - Elemento activo ("Cu"): gris oscuro con borde de selección
                // - Elemento en BD/ensayo (white cell): fondo claro con texto negro
                // - Elemento no usado: fondo gris medio/oscuro
                if (is_active_el) {
                    ImGui::PushStyleColor(ImGuiCol_Button, IM_COL32(72, 78, 86, 255));
                    ImGui::PushStyleColor(ImGuiCol_ButtonHovered, IM_COL32(88, 96, 108, 255));
                    ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(255, 255, 255, 255));
                } else if (el.in_geo_db || in_assay) {
                    ImGui::PushStyleColor(ImGuiCol_Button, tk.light ? IM_COL32(252, 253, 255, 255) : IM_COL32(48, 58, 72, 255));
                    ImGui::PushStyleColor(ImGuiCol_ButtonHovered, tk.light ? IM_COL32(220, 234, 252, 255) : IM_COL32(66, 84, 110, 255));
                    ImGui::PushStyleColor(ImGuiCol_Text, tk.light ? IM_COL32(18, 22, 28, 255) : IM_COL32(238, 242, 248, 255));
                } else {
                    ImGui::PushStyleColor(ImGuiCol_Button, tk.light ? IM_COL32(116, 120, 126, 255) : IM_COL32(26, 30, 36, 255));
                    ImGui::PushStyleColor(ImGuiCol_ButtonHovered, tk.light ? IM_COL32(136, 140, 146, 255) : IM_COL32(38, 44, 52, 255));
                    ImGui::PushStyleColor(ImGuiCol_Text, tk.light ? IM_COL32(32, 34, 38, 255) : IM_COL32(115, 122, 132, 255));
                }

                ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(1.0f, 1.0f));
                if (ImGui::Button(el.symbol, ImVec2(btn_w, btn_h))) {
                    elemento_activo_pt_ = el.symbol;
                    element_selected_[i] = !element_selected_[i];
                    // Si se presiona con SHIFT o doble clic, agrega el elemento a la ronda activa
                    if (ImGui::GetIO().KeyShift && idx_ronda_activa_ >= 0 &&
                        idx_ronda_activa_ < static_cast<int>(rondas_.size())) {
                        auto& r_elems = rondas_[static_cast<size_t>(idx_ronda_activa_)].elementos;
                        if (std::find(r_elems.begin(), r_elems.end(), el.symbol) == r_elems.end()) {
                            r_elems.push_back(el.symbol);
                            ejecutar_calculo_modal();
                        }
                    }
                }
                ImGui::PopStyleVar();
                ImGui::PopStyleColor(3);
            }
            ImGui::EndChild();

            ImGui::SameLine(0.0f, 6.0f);

            // Tabla derecha: "Element + SHIFT" (Phase | Element %)
            ImGui::BeginChild("##ElemShiftPanel", ImVec2(0.0f, 0.0f), true);
            ImGui::Text("Element + SHIFT  [%s]", elemento_activo_pt_.c_str());
            if (ImGui::BeginTable("##PhaseElemPctTable", 2,
                                  ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                                  ImVec2(0.0f, -26.0f))) {
                ImGui::TableSetupColumn("Phase", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Element %", ImGuiTableColumnFlags_WidthFixed, 68.0f);
                ImGui::TableHeadersRow();

                for (const auto& mname : minerales_candidatos_) {
                    const auto* f = mineral_db_.buscar_por_nombre(mname);
                    if (!f) continue;
                    double pct = f->porcentaje_elemento(elemento_activo_pt_);
                    if (pct <= 0.001) continue;
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0);
                    ImGui::TextUnformatted(f->nombre_corto.c_str());
                    ImGui::TableSetColumnIndex(1);
                    ImGui::Text("%.2f", pct);
                }
                ImGui::EndTable();
            }
            if (ImGui::SmallButton("+ Add Element to Round")) {
                if (idx_ronda_activa_ >= 0 && idx_ronda_activa_ < static_cast<int>(rondas_.size())) {
                    auto& r_elems = rondas_[static_cast<size_t>(idx_ronda_activa_)].elementos;
                    if (std::find(r_elems.begin(), r_elems.end(), elemento_activo_pt_) == r_elems.end()) {
                        r_elems.push_back(elemento_activo_pt_);
                        ejecutar_calculo_modal();
                    }
                }
            }
            ImGui::EndChild();

            ImGui::EndChild(); // Fin PTAndElemShift

            // ── Bloque Inferior: "Minerals" (izquierda) + Botones [> < >> << - Eraser] + "Phases and Elements per Rounds" & "Method" (derecha) ──
            const float bot_avail_w = ImGui::GetContentRegionAvail().x;
            const float min_list_w = std::clamp(bot_avail_w * 0.37f, 185.0f, 275.0f);

            // Columna izquierda: "Minerals" + botones de transferencia verticales
            ImGui::BeginChild("##MineralsCandidateBox", ImVec2(min_list_w, 0.0f), true);
            ImGui::Text("Minerals");
            ImGui::Separator();

            const float list_inner_w = ImGui::GetContentRegionAvail().x - 32.0f;
            ImGui::BeginChild("##MineralsListScroll", ImVec2(list_inner_w, 0.0f), true);
            for (int i = 0; i < static_cast<int>(minerales_candidatos_.size()); ++i) {
                const bool sel = (i == idx_mineral_cand_);
                if (ImGui::Selectable(minerales_candidatos_[static_cast<size_t>(i)].c_str(), sel)) {
                    idx_mineral_cand_ = i;
                }
            }
            ImGui::EndChild();

            ImGui::SameLine(0.0f, 4.0f);
            ImGui::BeginGroup();
            // [>] Enviar mineral seleccionado a la ronda activa
            if (ImGui::Button(">", ImVec2(24.0f, 24.0f))) {
                if (idx_mineral_cand_ >= 0 && idx_mineral_cand_ < static_cast<int>(minerales_candidatos_.size()) &&
                    idx_ronda_activa_ >= 0 && idx_ronda_activa_ < static_cast<int>(rondas_.size())) {
                    const std::string& m_sel = minerales_candidatos_[static_cast<size_t>(idx_mineral_cand_)];
                    auto& fases_r = rondas_[static_cast<size_t>(idx_ronda_activa_)].fases;
                    if (std::find(fases_r.begin(), fases_r.end(), m_sel) == fases_r.end()) {
                        fases_r.push_back(m_sel);
                        ejecutar_calculo_modal();
                    }
                }
            }
            // [<] Quitar fase seleccionada de la ronda activa
            if (ImGui::Button("<", ImVec2(24.0f, 24.0f))) {
                if (idx_ronda_activa_ >= 0 && idx_ronda_activa_ < static_cast<int>(rondas_.size())) {
                    auto& fases_r = rondas_[static_cast<size_t>(idx_ronda_activa_)].fases;
                    if (!fases_r.empty()) {
                        int idx_del = std::clamp(idx_fase_ronda_sel_, 0, static_cast<int>(fases_r.size()) - 1);
                        fases_r.erase(fases_r.begin() + idx_del);
                        ejecutar_calculo_modal();
                    }
                }
            }
            // [>>] Enviar todos los minerales candidatos a la ronda activa
            if (ImGui::Button(">|", ImVec2(24.0f, 24.0f))) {
                if (idx_ronda_activa_ >= 0 && idx_ronda_activa_ < static_cast<int>(rondas_.size())) {
                    rondas_[static_cast<size_t>(idx_ronda_activa_)].fases = minerales_candidatos_;
                    ejecutar_calculo_modal();
                }
            }
            // [<<] Vaciar fases de la ronda activa
            if (ImGui::Button("|<", ImVec2(24.0f, 24.0f))) {
                if (idx_ronda_activa_ >= 0 && idx_ronda_activa_ < static_cast<int>(rondas_.size())) {
                    rondas_[static_cast<size_t>(idx_ronda_activa_)].fases.clear();
                    ejecutar_calculo_modal();
                }
            }
            // Botón rojo (-) eliminar mineral de la lista de candidatos
            ImGui::PushStyleColor(ImGuiCol_Button, IM_COL32(204, 58, 52, 255));
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(255, 255, 255, 255));
            if (ImGui::Button("-##del_cand", ImVec2(24.0f, 24.0f))) {
                if (idx_mineral_cand_ >= 0 && idx_mineral_cand_ < static_cast<int>(minerales_candidatos_.size()) &&
                    minerales_candidatos_.size() > 1) {
                    minerales_candidatos_.erase(minerales_candidatos_.begin() + idx_mineral_cand_);
                    if (idx_mineral_cand_ >= static_cast<int>(minerales_candidatos_.size()))
                        idx_mineral_cand_ = static_cast<int>(minerales_candidatos_.size()) - 1;
                }
            }
            ImGui::PopStyleColor(2);
            // Botón borrador (restaurar lista)
            if (ImGui::Button("C##clr_cand", ImVec2(24.0f, 24.0f))) {
                cargar_fuente_datos(idx_data_source_);
            }
            ImGui::EndGroup();
            ImGui::EndChild();

            ImGui::SameLine(0.0f, 6.0f);

            // Columna derecha inferior: "Phases and Elements per Rounds" + "Method"
            ImGui::BeginChild("##RoundsAndMethodCol", ImVec2(0.0f, 0.0f), false);

            ImGui::BeginChild("##RoundsBox", ImVec2(0.0f, -108.0f), true);
            ImGui::Text("Phases and Elements per Rounds");
            ImGui::SameLine(ImGui::GetWindowWidth() - 185.0f);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(46, 148, 62, 255));
            if (ImGui::SmallButton("+ Add Round")) {
                cma::application::modules::RondaModal nr;
                nr.nombre = "Round " + std::to_string(rondas_.size() + 1);
                rondas_.push_back(nr);
                idx_ronda_activa_ = static_cast<int>(rondas_.size()) - 1;
            }
            ImGui::PopStyleColor();
            ImGui::SameLine(0.0f, 4.0f);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(198, 54, 48, 255));
            if (ImGui::SmallButton("- Remove Round") && rondas_.size() > 1) {
                rondas_.pop_back();
                if (idx_ronda_activa_ >= static_cast<int>(rondas_.size()))
                    idx_ronda_activa_ = static_cast<int>(rondas_.size()) - 1;
                ejecutar_calculo_modal();
            }
            ImGui::PopStyleColor();

            if (ImGui::BeginTabBar("##RoundsTabBar")) {
                for (int r = 0; r < static_cast<int>(rondas_.size()); ++r) {
                    if (ImGui::BeginTabItem(rondas_[static_cast<size_t>(r)].nombre.c_str())) {
                        idx_ronda_activa_ = r;
                        auto& ronda = rondas_[static_cast<size_t>(r)];

                        const float r_avail = ImGui::GetContentRegionAvail().x;
                        const float phases_w = r_avail * 0.56f;

                        // Sub-lista "Phases"
                        ImGui::BeginChild("##RoundPhasesSub", ImVec2(phases_w, 0.0f), true);
                        ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.text_secondary), "Phases");
                        ImGui::Separator();
                        for (int p = 0; p < static_cast<int>(ronda.fases.size()); ++p) {
                            if (ImGui::Selectable(ronda.fases[static_cast<size_t>(p)].c_str(), idx_fase_ronda_sel_ == p)) {
                                idx_fase_ronda_sel_ = p;
                            }
                        }
                        ImGui::EndChild();

                        ImGui::SameLine(0.0f, 6.0f);

                        // Sub-lista "Elements" + botones [-] y [Eraser]
                        ImGui::BeginChild("##RoundElemsSub", ImVec2(0.0f, 0.0f), true);
                        ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.text_secondary), "Elements");
                        ImGui::Separator();
                        const float el_list_w = ImGui::GetContentRegionAvail().x - 28.0f;
                        ImGui::BeginChild("##RoundElemsInner", ImVec2(el_list_w, 0.0f), false);
                        for (int e = 0; e < static_cast<int>(ronda.elementos.size()); ++e) {
                            if (ImGui::Selectable(ronda.elementos[static_cast<size_t>(e)].c_str(), idx_elem_ronda_sel_ == e)) {
                                idx_elem_ronda_sel_ = e;
                            }
                        }
                        ImGui::EndChild();
                        ImGui::SameLine(0.0f, 3.0f);
                        ImGui::BeginGroup();
                        ImGui::PushStyleColor(ImGuiCol_Button, IM_COL32(204, 58, 52, 255));
                        ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(255, 255, 255, 255));
                        if (ImGui::Button("-##del_el_r", ImVec2(22.0f, 22.0f)) && !ronda.elementos.empty()) {
                            int idel = std::clamp(idx_elem_ronda_sel_, 0, static_cast<int>(ronda.elementos.size()) - 1);
                            ronda.elementos.erase(ronda.elementos.begin() + idel);
                            ejecutar_calculo_modal();
                        }
                        ImGui::PopStyleColor(2);
                        if (ImGui::Button("C##clr_el_r", ImVec2(22.0f, 22.0f))) {
                            ronda.elementos.clear();
                            ejecutar_calculo_modal();
                        }
                        ImGui::EndGroup();
                        ImGui::EndChild();

                        ImGui::EndTabItem();
                    }
                }
                ImGui::EndTabBar();
            }
            ImGui::EndChild(); // Fin RoundsBox

            // Recuadro "Method" con los 4 métodos matemáticos de HSC Geo 8.0
            ImGui::BeginChild("##MethodBox", ImVec2(0.0f, 0.0f), true);
            ImGui::Text("Method");
            bool m_changed = false;
            m_changed |= ImGui::RadioButton("Least Square (LS)", &current_method_, 0);
            ImGui::SameLine(230.0f);
            m_changed |= ImGui::RadioButton("Non-negative Least Square (NNLS)", &current_method_, 2);
            m_changed |= ImGui::RadioButton("Weighed Least Square (WLS)", &current_method_, 1);
            ImGui::SameLine(230.0f);
            m_changed |= ImGui::RadioButton("Weighed non-negative Least Square (WNNLS)", &current_method_, 3);
            if (m_changed) {
                ejecutar_calculo_modal();
            }
            ImGui::EndChild();

            ImGui::EndChild(); // Fin RoundsAndMethodCol
            ImGui::EndTabItem();
        }

        if (ImGui::BeginTabItem("Error Estimation")) {
            tab_modal_ = 1;
            ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_blue),
                               "Modal Mineralogy & Elemental Back-Calculation Error Analysis (%s)", data_source_);
            ImGui::Separator();

            // Tabla superior: Composición Modal Resuelta (wt-% y vol-%)
            if (ImGui::BeginTable("##ModalResultPhases", 6,
                                  ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                                  ImVec2(0.0f, 185.0f))) {
                ImGui::TableSetupColumn("Mineral Phase", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Formula", ImGuiTableColumnFlags_WidthFixed, 95.0f);
                ImGui::TableSetupColumn("Round", ImGuiTableColumnFlags_WidthFixed, 55.0f);
                ImGui::TableSetupColumn("Modal wt-%", ImGuiTableColumnFlags_WidthFixed, 88.0f);
                ImGui::TableSetupColumn("Modal vol-%", ImGuiTableColumnFlags_WidthFixed, 88.0f);
                ImGui::TableSetupColumn("Std. Error (+/-)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
                ImGui::TableHeadersRow();

                for (const auto& rf : ultimo_resultado_modal_.fases) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(rf.nombre_mineral.c_str());
                    ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(rf.formula.c_str());
                    ImGui::TableSetColumnIndex(2); ImGui::Text("Round %d", rf.ronda_resuelta);
                    ImGui::TableSetColumnIndex(3); ImGui::Text("%.3f", rf.wt_pct);
                    ImGui::TableSetColumnIndex(4); ImGui::Text("%.3f", rf.vol_pct);
                    ImGui::TableSetColumnIndex(5); ImGui::Text("+/- %.3f", rf.sigma_error);
                }
                if (unidentified_to_) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::TextColored(ImVec4(0.82f, 0.42f, 0.14f, 1.0f), "Unidentified (%s)", unidentified_label_);
                    ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted("-");
                    ImGui::TableSetColumnIndex(2); ImGui::TextUnformatted("Residual");
                    ImGui::TableSetColumnIndex(3); ImGui::Text("%.3f", ultimo_resultado_modal_.no_identificado_wt);
                    ImGui::TableSetColumnIndex(4); ImGui::TextUnformatted("-");
                    ImGui::TableSetColumnIndex(5); ImGui::TextUnformatted("-");
                }
                ImGui::EndTable();
            }

            // Barra apilada visual de fases modales
            std::vector<css::StackedSegment> segs;
            for (const auto& rf : ultimo_resultado_modal_.fases) {
                if (rf.wt_pct > 0.05) segs.push_back({rf.formula, rf.wt_pct});
            }
            if (ultimo_resultado_modal_.no_identificado_wt > 0.05) {
                segs.push_back({unidentified_label_, ultimo_resultado_modal_.no_identificado_wt});
            }
            css::draw_stacked_composition_bar("Modal Mineralogy Distribution (wt-%)", segs, 22.0f);

            // Tabla inferior: Comparación de Ensayo Elemental Medido vs Calculado
            ImGui::Spacing();
            ImGui::Text("Elemental Assay Reconciliation (RMSE = %.4f wt-%% | Chi^2 = %.4f)",
                        ultimo_resultado_modal_.rmse_elemental, ultimo_resultado_modal_.chi_cuadrado);
            if (ImGui::BeginTable("##ElemErrorTable", 5,
                                  ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                ImGui::TableSetupColumn("Element", ImGuiTableColumnFlags_WidthFixed, 70.0f);
                ImGui::TableSetupColumn("Measured XRF (wt-%)", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Calculated Modal (wt-%)", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Residual (wt-%)", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Rel. Error (%)", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableHeadersRow();

                for (const auto& ce : ultimo_resultado_modal_.balance_elemental) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(ce.elemento.c_str());
                    ImGui::TableSetColumnIndex(1); ImGui::Text("%.3f", ce.medido_wt);
                    ImGui::TableSetColumnIndex(2); ImGui::Text("%.3f", ce.calculado_wt);
                    ImGui::TableSetColumnIndex(3);
                    ImVec4 col_res = (std::abs(ce.residual_wt) < 0.25) ? ImVec4(0.20f, 0.65f, 0.28f, 1.0f)
                                                                       : ImVec4(0.85f, 0.35f, 0.18f, 1.0f);
                    ImGui::TextColored(col_res, "%+.3f", ce.residual_wt);
                    ImGui::TableSetColumnIndex(4); ImGui::Text("%+.2f %%", ce.error_rel_pct);
                }
                ImGui::EndTable();
            }
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }
    ImGui::EndChild(); // Fin GeoModalCenterCol

    ImGui::SameLine(0.0f, 6.0f);

    auto format_modal_summary = [&]() {
        std::ostringstream oss;
        oss << "Modal Composition Summary (" << data_source_ << "):\n";
        for (const auto& rf : ultimo_resultado_modal_.fases) {
            oss << "  [R" << rf.ronda_resuelta << "] " << rf.nombre_mineral << " (" << rf.formula << "): "
                << std::fixed << std::setprecision(2) << rf.wt_pct << " wt-% | " << rf.vol_pct << " vol-%\n";
        }
        oss << "Total: " << std::fixed << std::setprecision(2) << ultimo_resultado_modal_.total_mineral_wt
            << " wt-% | Unidentified (" << unidentified_label_ << "): " << ultimo_resultado_modal_.no_identificado_wt
            << " wt-% | RMSE = " << std::setprecision(4) << ultimo_resultado_modal_.rmse_elemental << " wt-%";
        return oss.str();
    };

    // ── Sub-columna 3 (Derecha): Panel "Identify" ──
    ImGui::BeginChild("##GeoIdentifyRightCol", ImVec2(0.0f, -32.0f), true);
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.text_secondary), "Identify");
    ImGui::Separator();
    ImGui::Text("Sources");
    ImGui::BeginChild("##SourcesChecklistBox", ImVec2(0.0f, 175.0f), true);
    for (int s = 0; s < 7; ++s) ImGui::Checkbox(kGeoSourcesList[s], &sources_checked_[s]);
    ImGui::EndChild();

    if (ImGui::Button("Identify structure", ImVec2(-1.0f, 24.0f))) {
        ejecutar_calculo_modal();
        tab_modal_ = 1;
        resultado_geo_ = format_modal_summary();
        mostrar_resultado_ = true;
    }
    ImGui::Checkbox("Use Thesaurus", &use_thesaurus_);
    if (ImGui::Button("Thesaurus", ImVec2(-1.0f, 24.0f))) mostrar_thesaurus_ = true;
    ImGui::Checkbox("Unidentified to:", &unidentified_to_);
    ImGui::SetNextItemWidth(-1.0f);
    ImGui::InputText("##UserXInput", unidentified_label_, sizeof(unidentified_label_));
    ImGui::Checkbox("Only unidentified", &only_unidentified_);
    ImGui::Checkbox("Source name = Method", &source_name_eq_method_);
    ImGui::EndChild();

    // ── Barra Inferior de Acción de Modal Calculations ──
    ImGui::BeginChild("##GeoBottomActionStrip", ImVec2(0.0f, 28.0f), false, ImGuiWindowFlags_NoScrollbar);
    if (ImGui::Button("Calculate and Save in DB", ImVec2(165.0f, 24.0f))) {
        ejecutar_calculo_modal();
        resultado_geo_ = "Cálculo modal guardado en la base de datos del proyecto.\n" + format_modal_summary();
        mostrar_resultado_ = true;
    }
    ImGui::SameLine(0.0f, 8.0f);
    if (ImGui::Button("Calculate", ImVec2(95.0f, 24.0f))) {
        ejecutar_calculo_modal();
        resultado_geo_ = format_modal_summary();
        mostrar_resultado_ = true;
    }
    ImGui::SameLine(0.0f, 14.0f);
    ImGui::Text("Rounds = %d", ultimo_resultado_modal_.rondas_ejecutadas);
    ImGui::SameLine(0.0f, 18.0f);
    ImGui::Text("Not Determined %d/%zu", ultimo_resultado_modal_.fases_no_determinadas, ultimo_resultado_modal_.fases.size());
    ImGui::SameLine(0.0f, 18.0f);
    if (ImGui::Button("Equations", ImVec2(92.0f, 24.0f))) { ejecutar_calculo_modal(); mostrar_ecuaciones_ = true; }
    ImGui::SameLine(ImGui::GetContentRegionAvail().x - 95.0f);
    if (ImGui::Button("Result >>>", ImVec2(95.0f, 24.0f))) {
        ejecutar_calculo_modal();
        resultado_geo_ = format_modal_summary();
        mostrar_resultado_ = true;
    }
    ImGui::EndChild();
}

void PanelGeoMineralogical::dibujar_vista_base_datos(bool modo_busqueda_avanzada) {
    const auto tk = css::tokens();

    ImGui::BeginChild("##GeoDBFilterTop", ImVec2(0.0f, modo_busqueda_avanzada ? 132.0f : 86.0f), true);
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_blue),
                       modo_busqueda_avanzada
                           ? "Find Mineral — Multi-Criteria Stoichiometric & Physical Search"
                           : "HSC Geo Mineral Database — Stoichiometric & Literature Catalog");
    ImGui::Separator();

    // Píldoras por clase mineralógica
    for (int c = 0; c < 7; ++c) {
        if (c > 0) ImGui::SameLine(0.0f, 5.0f);
        if (css::category_pill(kMineralClasses[c], idx_clase_filtro_ == c, tk.accent_blue)) {
            idx_clase_filtro_ = c;
        }
    }

    ImGui::Spacing();
    ImGui::Text("Query:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(-145.0f);
    ImGui::InputTextWithHint("##GeoQueryBox",
                             "e.g. [Cu %] Is greater than 10 AND [Specific Gravity] Is between 2.2 and 6.0",
                             filter_query_, sizeof(filter_query_));
    ImGui::SameLine();
    if (ImGui::Button("Clear Filter", ImVec2(130.0f, 0.0f))) {
        filter_query_[0] = '\0';
        idx_clase_filtro_ = 0;
        sg_min_ = 1.0f; sg_max_ = 12.0f;
        mohs_min_ = 1.0f; mohs_max_ = 10.0f;
    }

    if (modo_busqueda_avanzada) {
        ImGui::SetNextItemWidth(220.0f);
        ImGui::DragFloatRange2("Specific Gravity (g/cm3)", &sg_min_, &sg_max_, 0.05f, 1.0f, 15.0f, "Min: %.2f", "Max: %.2f");
        ImGui::SameLine(0.0f, 18.0f);
        ImGui::SetNextItemWidth(220.0f);
        ImGui::DragFloatRange2("Mohs Hardness", &mohs_min_, &mohs_max_, 0.1f, 1.0f, 10.0f, "Min: %.1f", "Max: %.1f");
    }
    ImGui::EndChild();

    auto filtrados = mineral_db_.filtrar_minerales(
        filter_query_, kMineralClasses[idx_clase_filtro_], {}, {}, sg_min_, sg_max_, mohs_min_, mohs_max_);

    // Tabla principal del catálogo mineralógico + Inspector inferior de composición estequiométrica
    if (ImGui::BeginTable("##GeoMineralDBTable", 9,
                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                          ImGuiTableFlags_Resizable | ImGuiTableFlags_ScrollY,
                          ImVec2(0.0f, -135.0f))) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Mineral Name (Locality)", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("IMA", ImGuiTableColumnFlags_WidthFixed, 46.0f);
        ImGui::TableSetupColumn("Formula", ImGuiTableColumnFlags_WidthFixed, 115.0f);
        ImGui::TableSetupColumn("Class", ImGuiTableColumnFlags_WidthFixed, 82.0f);
        ImGui::TableSetupColumn("Crystal System", ImGuiTableColumnFlags_WidthFixed, 95.0f);
        ImGui::TableSetupColumn("SG", ImGuiTableColumnFlags_WidthFixed, 48.0f);
        ImGui::TableSetupColumn("Mohs", ImGuiTableColumnFlags_WidthFixed, 48.0f);
        ImGui::TableSetupColumn("MW (g/mol)", ImGuiTableColumnFlags_WidthFixed, 76.0f);
        ImGui::TableSetupColumn("Literature Reference", ImGuiTableColumnFlags_WidthFixed, 175.0f);
        ImGui::TableHeadersRow();

        for (int i = 0; i < static_cast<int>(filtrados.size()); ++i) {
            const auto* m = filtrados[static_cast<size_t>(i)];
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            const bool sel = (idx_mineral_db_sel_ == i);
            if (ImGui::Selectable(m->nombre.c_str(), sel, ImGuiSelectableFlags_SpanAllColumns)) {
                idx_mineral_db_sel_ = i;
            }
            ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(m->simbolo_ima.c_str());
            ImGui::TableSetColumnIndex(2); ImGui::TextUnformatted(m->formula.c_str());
            ImGui::TableSetColumnIndex(3); ImGui::TextUnformatted(m->clase_mineral.c_str());
            ImGui::TableSetColumnIndex(4); ImGui::TextUnformatted(m->sistema_cristalino.c_str());
            ImGui::TableSetColumnIndex(5); ImGui::Text("%.2f", m->gravedad_especifica);
            ImGui::TableSetColumnIndex(6); ImGui::Text("%.1f", m->dureza_mohs);
            ImGui::TableSetColumnIndex(7); ImGui::Text("%.2f", m->masa_molar);
            ImGui::TableSetColumnIndex(8); ImGui::TextUnformatted(m->referencia.c_str());
        }
        ImGui::EndTable();
    }

    // Inspector inferior del mineral seleccionado
    ImGui::BeginChild("##GeoMineralInspector", ImVec2(0.0f, 0.0f), true);
    if (!filtrados.empty()) {
        int idx_ok = std::clamp(idx_mineral_db_sel_, 0, static_cast<int>(filtrados.size()) - 1);
        const auto* m = filtrados[static_cast<size_t>(idx_ok)];
        ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_blue),
                           "%s [%s] — %s", m->nombre.c_str(), m->simbolo_ima.c_str(), m->formula.c_str());
        ImGui::SameLine(ImGui::GetContentRegionAvail().x - 210.0f);
        if (ImGui::SmallButton("+ Add to Modal Candidates")) {
            if (std::find(minerales_candidatos_.begin(), minerales_candidatos_.end(), m->nombre) == minerales_candidatos_.end()) {
                minerales_candidatos_.push_back(m->nombre);
            }
        }
        ImGui::Text("Locality: %s  |  Color: %s  |  Reference: %s",
                    m->localidad.c_str(), m->color.c_str(), m->referencia.c_str());

        std::vector<css::StackedSegment> segs;
        for (const auto& [el, wt] : m->elem_wt_pct) {
            segs.push_back({el, wt});
        }
        css::draw_stacked_composition_bar("Stoichiometric Elemental Composition (wt-%)", segs, 20.0f);
    }
    ImGui::EndChild();
}

void PanelGeoMineralogical::dibujar_vista_formula_mineral() {
    const auto tk = css::tokens();
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_blue),
                       "Mineral Formula Calculator from Microprobe / Chemical Analysis (EPMA / XRF)");
    ImGui::Separator();

    ImGui::Columns(2, "##FormulaCalcCols", true);
    ImGui::BeginChild("##FormulaInputLeft", ImVec2(0.0f, 0.0f), true);
    ImGui::Text("1. Presets & Normalization Basis");
    if (ImGui::SmallButton("Preset: Chalcopyrite (EPMA)")) {
        entradas_formula_ = {{"Cu", 34.50}, {"Fe", 30.55}, {"S", 34.95}};
        std::snprintf(elem_base_norm_, sizeof(elem_base_norm_), "S");
        num_atomos_norm_ = 2.0;
        res_formula_ = mineral_db_.calcular_formula_mineral(entradas_formula_, elem_base_norm_, num_atomos_norm_);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("Preset: Olivine (Oxides)")) {
        entradas_formula_ = {{"MgO", 49.20}, {"FeO", 9.80}, {"SiO2", 41.00}};
        std::snprintf(elem_base_norm_, sizeof(elem_base_norm_), "O");
        num_atomos_norm_ = 4.0;
        res_formula_ = mineral_db_.calcular_formula_mineral(entradas_formula_, elem_base_norm_, num_atomos_norm_);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("Preset: Pentlandite")) {
        entradas_formula_ = {{"Fe", 32.55}, {"Ni", 34.22}, {"S", 33.23}};
        std::snprintf(elem_base_norm_, sizeof(elem_base_norm_), "S");
        num_atomos_norm_ = 8.0;
        res_formula_ = mineral_db_.calcular_formula_mineral(entradas_formula_, elem_base_norm_, num_atomos_norm_);
    }

    ImGui::Spacing();
    ImGui::SetNextItemWidth(80.0f);
    bool changed = ImGui::InputText("Basis Element (e.g. O, S)", elem_base_norm_, sizeof(elem_base_norm_));
    ImGui::SameLine();
    ImGui::SetNextItemWidth(90.0f);
    changed |= ImGui::InputDouble("Atoms in Formula", &num_atomos_norm_, 1.0, 2.0, "%.1f");

    if (ImGui::BeginTable("##EPMAInputTable", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg, ImVec2(0.0f, 220.0f))) {
        ImGui::TableSetupColumn("Element / Oxide", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Analysis (wt-%)", ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableSetupColumn("Action", ImGuiTableColumnFlags_WidthFixed, 55.0f);
        ImGui::TableHeadersRow();

        for (size_t i = 0; i < entradas_formula_.size(); ++i) {
            ImGui::TableNextRow();
            ImGui::PushID(static_cast<int>(i));
            ImGui::TableSetColumnIndex(0);
            char cbuf[32];
            std::snprintf(cbuf, sizeof(cbuf), "%s", entradas_formula_[i].componente.c_str());
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputText("##comp", cbuf, sizeof(cbuf))) {
                entradas_formula_[i].componente = cbuf;
                changed = true;
            }
            ImGui::TableSetColumnIndex(1);
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputDouble("##wt", &entradas_formula_[i].wt_pct, 0.1, 1.0, "%.3f")) {
                changed = true;
            }
            ImGui::TableSetColumnIndex(2);
            if (ImGui::SmallButton("Del") && entradas_formula_.size() > 1) {
                entradas_formula_.erase(entradas_formula_.begin() + static_cast<long>(i));
                changed = true;
                ImGui::PopID();
                break;
            }
            ImGui::PopID();
        }
        ImGui::EndTable();
    }

    if (ImGui::Button("+ Add Component")) {
        entradas_formula_.push_back({"SiO2", 1.0});
        changed = true;
    }
    ImGui::SameLine();
    if (ImGui::Button("Calculate Empirical Formula") || changed) {
        res_formula_ = mineral_db_.calcular_formula_mineral(entradas_formula_, elem_base_norm_, num_atomos_norm_);
    }
    ImGui::EndChild();

    ImGui::NextColumn();
    ImGui::BeginChild("##FormulaOutputRight", ImVec2(0.0f, 0.0f), true);
    ImGui::Text("2. Calculated Empirical Formula & Database Match");
    ImGui::Separator();
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_green),
                       "Empirical Formula:  %s", res_formula_.formula_empirica.c_str());
    ImGui::Text("Closest Database Mineral:  %s", res_formula_.mineral_mas_cercano.c_str());
    ImGui::Text("Ideal Stoichiometric Formula:  %s  (Match: %.2f %%)",
                res_formula_.formula_ideal.c_str(), res_formula_.similitud_pct);
    ImGui::Text("Analytical Total:  %.2f wt-%%", res_formula_.suma_analisis_wt);

    ImGui::Spacing();
    if (ImGui::BeginTable("##APFUTable", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Element");
        ImGui::TableSetupColumn("Normalized (wt-%)");
        ImGui::TableSetupColumn("Atoms per Formula Unit (apfu)");
        ImGui::TableHeadersRow();
        for (const auto& [el, apfu] : res_formula_.apfu) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(el.c_str());
            ImGui::TableSetColumnIndex(1); ImGui::Text("%.3f", res_formula_.elem_wt_normalizado[el]);
            ImGui::TableSetColumnIndex(2); ImGui::Text("%.4f", apfu);
        }
        ImGui::EndTable();
    }
    ImGui::EndChild();
    ImGui::Columns(1);
}

void PanelGeoMineralogical::dibujar_vista_petrologia_cipw() {
    const auto tk = css::tokens();
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_blue),
                       "Petrological Calculations for Rock Samples — CIPW Norm, TAS & QAPF Classification");
    ImGui::Separator();

    ImGui::Columns(2, "##CIPWCols", true);
    ImGui::BeginChild("##CIPWLeft", ImVec2(0.0f, 0.0f), true);
    ImGui::Text("Major Oxide Whole-Rock Analysis (wt-%%)");
    if (ImGui::SmallButton("Bushveld Norite")) {
        roca_oxidos_ = {"Bushveld Norite (SA-01)", 51.40, 0.45, 16.80, 1.60, 7.20, 0.15, 9.40, 10.60, 1.95, 0.35, 0.10};
        res_cipw_ = mineral_db_.calcular_norma_cipw(roca_oxidos_);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("Calc-Alkaline Granite")) {
        roca_oxidos_ = {"Rapakivi Granite (FI-02)", 71.80, 0.32, 14.10, 0.85, 1.90, 0.05, 0.65, 1.65, 3.55, 4.95, 0.12};
        res_cipw_ = mineral_db_.calcular_norma_cipw(roca_oxidos_);
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("Komatiite / Peridotite")) {
        roca_oxidos_ = {"Munro Komatiite (CA-03)", 45.20, 0.35, 6.40, 2.10, 8.90, 0.18, 28.50, 6.80, 0.55, 0.10, 0.05};
        res_cipw_ = mineral_db_.calcular_norma_cipw(roca_oxidos_);
    }

    bool ox_changed = false;
    struct OxField { const char* lbl; double* val; };
    OxField ox_fields[] = {
        {"SiO2 (wt-%)", &roca_oxidos_.SiO2},   {"TiO2 (wt-%)", &roca_oxidos_.TiO2},
        {"Al2O3 (wt-%)", &roca_oxidos_.Al2O3}, {"Fe2O3 (wt-%)", &roca_oxidos_.Fe2O3},
        {"FeO (wt-%)", &roca_oxidos_.FeO},     {"MnO (wt-%)", &roca_oxidos_.MnO},
        {"MgO (wt-%)", &roca_oxidos_.MgO},     {"CaO (wt-%)", &roca_oxidos_.CaO},
        {"Na2O (wt-%)", &roca_oxidos_.Na2O},   {"K2O (wt-%)", &roca_oxidos_.K2O},
        {"P2O5 (wt-%)", &roca_oxidos_.P2O5}
    };
    for (auto& f : ox_fields) ox_changed |= ImGui::InputDouble(f.lbl, f.val, 0.2, 1.0, "%.2f");
    if (ox_changed) res_cipw_ = mineral_db_.calcular_norma_cipw(roca_oxidos_);
    ImGui::EndChild();

    ImGui::NextColumn();
    ImGui::BeginChild("##CIPWRight", ImVec2(0.0f, 0.0f), true);
    ImGui::Text("CIPW Normative Mineralogy & Petrological Indices");
    ImGui::Separator();
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_green),
                       "TAS Classification:  %s", res_cipw_.clasificacion_tas.c_str());
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_blue),
                       "QAPF Plutonic Equivalent:  %s", res_cipw_.clasificacion_qapf.c_str());
    ImGui::Text("Alumina Saturation:  %s  (A/CNK = %.3f)",
                res_cipw_.saturacion_alumina.c_str(), res_cipw_.indice_acnk);
    ImGui::Text("Magnesium Number (Mg#):  %.2f   |   Differentiation Index (DI):  %.2f",
                res_cipw_.mg_number, res_cipw_.indice_diferenciacion);

    if (ImGui::BeginTable("##CIPWNormTable", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg, ImVec2(0.0f, 210.0f))) {
        ImGui::TableSetupColumn("Normative Mineral Phase");
        ImGui::TableSetupColumn("CIPW Norm (wt-%)");
        ImGui::TableHeadersRow();
        std::vector<css::StackedSegment> segs;
        for (const auto& [mname, wt] : res_cipw_.norma_wt) {
            if (wt <= 0.005) continue;
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(mname.c_str());
            ImGui::TableSetColumnIndex(1); ImGui::Text("%.2f", wt);
            segs.push_back({mname.substr(0, mname.find(' ')), wt});
        }
        ImGui::EndTable();
        css::draw_stacked_composition_bar("Normative Mineralogy Breakdown (wt-%)", segs, 22.0f);
    }
    ImGui::EndChild();
    ImGui::Columns(1);
}

void PanelGeoMineralogical::dibujar_vista_liberacion() {
    const auto tk = css::tokens();
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.accent_blue),
                       "Mineral Liberation & Grain Size Distribution (Gaudin / King Liberation Model)");
    ImGui::Separator();
    static float p80_um = 105.0f;
    static float d0_grain_um = 180.0f;
    ImGui::SetNextItemWidth(180.0f);
    ImGui::SliderFloat("Grind Size P80 (um)", &p80_um, 20.0f, 350.0f, "%.1f um");
    ImGui::SameLine(0.0f, 18.0f);
    ImGui::SetNextItemWidth(180.0f);
    ImGui::SliderFloat("Characteristic Mineral Grain Size (um)", &d0_grain_um, 40.0f, 500.0f, "%.1f um");

    if (ImGui::BeginTable("##LibTable", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Mineral Phase");
        ImGui::TableSetupColumn("Modal Abundance (wt-%)");
        ImGui::TableSetupColumn("Free Liberation (%)");
        ImGui::TableSetupColumn("Binary Middlings (%)");
        ImGui::TableSetupColumn("Multiphase Locked (%)");
        ImGui::TableHeadersRow();

        for (const auto& rf : ultimo_resultado_modal_.fases) {
            if (rf.wt_pct <= 0.01) continue;
            const double ratio = d0_grain_um / std::max(10.0f, p80_um);
            const double lib = std::clamp(100.0 * (1.0 - std::exp(-0.68 * ratio)), 5.0, 99.2);
            const double bin = (100.0 - lib) * 0.65;
            const double ter = (100.0 - lib) * 0.35;
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(rf.nombre_mineral.c_str());
            ImGui::TableSetColumnIndex(1); ImGui::Text("%.2f", rf.wt_pct);
            ImGui::TableSetColumnIndex(2); ImGui::TextColored(ImVec4(0.20f, 0.68f, 0.30f, 1.0f), "%.1f %%", lib);
            ImGui::TableSetColumnIndex(3); ImGui::Text("%.1f %%", bin);
            ImGui::TableSetColumnIndex(4); ImGui::Text("%.1f %%", ter);
        }
        ImGui::EndTable();
    }
}

void PanelGeoMineralogical::renderizar() {
    css::ScopedHSCTheme hsc_theme;
    const auto tk = css::tokens();

    // 1. Cinta de opciones superior (Main -> Home | Window | Help)
    dibujar_ribbon_superior();

    // 2. Cuerpo central dividido en Panel Izquierdo "Features" + Área de Trabajo Derecha
    ImGui::BeginChild("##GeoFeaturesLeftDock", ImVec2(158.0f, -26.0f), true);
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.text_secondary), "Features");
    ImGui::Separator();
    if (css::feature_nav_button("f_mindb", "Mineral database", css::RibbonIcon::MineralDB, feature_activa_ == 0, 32.0f))
        feature_activa_ = 0;
    if (css::feature_nav_button("f_find", "Find mineral", css::RibbonIcon::FindMineral, feature_activa_ == 1, 32.0f))
        feature_activa_ = 1;
    if (css::feature_nav_button("f_mins", "Minerals", css::RibbonIcon::Crystal, feature_activa_ == 2, 32.0f))
        feature_activa_ = 2;
    if (css::feature_nav_button("f_ident", "Identify mineral", css::RibbonIcon::IdentifyQuestion, feature_activa_ == 3, 32.0f))
        feature_activa_ = 3;
    if (css::feature_nav_button("f_modal", "Modal calculations", css::RibbonIcon::ModalBarChart, feature_activa_ == 4, 32.0f))
        feature_activa_ = 4;
    if (css::feature_nav_button("f_lib", "Mineral liberation", css::RibbonIcon::Liberation, feature_activa_ == 5, 32.0f))
        feature_activa_ = 5;
    ImGui::EndChild();

    ImGui::SameLine(0.0f, 6.0f);

    ImGui::BeginChild("##GeoMainWorkspaceArea", ImVec2(0.0f, -26.0f), false);
    switch (feature_activa_) {
        case 0: dibujar_vista_base_datos(false); break;
        case 1: dibujar_vista_base_datos(true);  break;
        case 2: dibujar_vista_formula_mineral(); break;
        case 3: dibujar_vista_petrologia_cipw(); break;
        case 4: dibujar_vista_modal();           break;
        case 5: dibujar_vista_liberacion();      break;
        default: dibujar_vista_modal();          break;
    }
    ImGui::EndChild();

    // 3. Barra de estado inferior estilo Outotec HSC Geo 8.0
    ImGui::BeginChild("##GeoOutotecStatusBar", ImVec2(0.0f, 22.0f), false, ImGuiWindowFlags_NoScrollbar);
    ImGui::TextColored(ImVec4(0.84f, 0.24f, 0.16f, 1.0f), "Outotec |");
    ImGui::SameLine(0.0f, 8.0f);
    ImDrawList* dl = ImGui::GetWindowDrawList();
    ImVec2 p0 = ImGui::GetCursorScreenPos();
    dl->AddRectFilledMultiColor(ImVec2(p0.x, p0.y + 3.0f), ImVec2(p0.x + 110.0f, p0.y + 16.0f),
                                IM_COL32(236, 204, 245, 255), IM_COL32(214, 172, 234, 255),
                                IM_COL32(204, 156, 226, 255), IM_COL32(228, 192, 240, 255));
    dl->AddRect(ImVec2(p0.x, p0.y + 3.0f), ImVec2(p0.x + 110.0f, p0.y + 16.0f), tk.border_strong, 2.0f);
    ImGui::Dummy(ImVec2(118.0f, 18.0f));
    ImGui::SameLine();
    ImGui::TextDisabled("Ready — %s | Active Method: %s",
                        data_source_,
                        current_method_ == 0 ? "LS" : (current_method_ == 1 ? "WLS" : (current_method_ == 2 ? "NNLS" : "WNNLS")));
    ImGui::EndChild();

    // ── Popups Interactivos ──
    if (mostrar_datos_) {
        ImGui::OpenPopup("Data Source Assay Inspector");
        if (ImGui::BeginPopupModal("Data Source Assay Inspector", &mostrar_datos_, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("Active Data Source: %s", data_source_);
            ImGui::Separator();
            if (ImGui::BeginTable("##AssayEditPopupTable", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                ImGui::TableSetupColumn("Element");
                ImGui::TableSetupColumn("Measured XRF Assay (wt-%)");
                ImGui::TableHeadersRow();
                for (auto& [el, val] : ensayo_actual_) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(el.c_str());
                    ImGui::TableSetColumnIndex(1);
                    ImGui::PushID(el.c_str());
                    if (ImGui::InputDouble("##v", &val, 0.1, 1.0, "%.3f")) { val = std::max(0.0, val); ejecutar_calculo_modal(); }
                    ImGui::PopID();
                }
                ImGui::EndTable();
            }
            if (ImGui::Button("Close", ImVec2(120, 0))) { mostrar_datos_ = false; ImGui::CloseCurrentPopup(); }
            ImGui::EndPopup();
        }
    }
    if (mostrar_ecuaciones_) {
        ImGui::OpenPopup("Modal Mass-Balance Equations");
        if (ImGui::BeginPopupModal("Modal Mass-Balance Equations", &mostrar_ecuaciones_, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("Stoichiometric System of Equations per Round (%s):", data_source_);
            ImGui::Separator();
            for (const auto& eq : ultimo_resultado_modal_.ecuaciones_rondas) ImGui::BulletText("%s", eq.c_str());
            if (ImGui::Button("Close", ImVec2(120, 0))) { mostrar_ecuaciones_ = false; ImGui::CloseCurrentPopup(); }
            ImGui::EndPopup();
        }
    }
    if (mostrar_wizard_) {
        ImGui::OpenPopup("HSC Geo Modal Calculation Wizard");
        if (ImGui::BeginPopupModal("HSC Geo Modal Calculation Wizard", &mostrar_wizard_, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::TextWrapped("1. Select Data Source (e.g. Bruvann.xls - XRF) or edit XRF assay via 'Show Data'.\n2. Assign minerals and tracer elements per Round.\n3. Select LS, WLS, NNLS, or WNNLS and click 'Calculate'.");
            if (ImGui::Button("Close", ImVec2(120, 0))) { mostrar_wizard_ = false; ImGui::CloseCurrentPopup(); }
            ImGui::EndPopup();
        }
    }
    if (mostrar_thesaurus_) {
        ImGui::OpenPopup("Mineralogical Thesaurus");
        if (ImGui::BeginPopupModal("Mineralogical Thesaurus", &mostrar_thesaurus_, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("IMA Standard Mineral Symbols & Synonyms:");
            ImGui::Separator();
            for (size_t i = 0; i < std::min<size_t>(12, mineral_db_.catalogo().size()); ++i) {
                const auto& m = mineral_db_.catalogo()[i];
                ImGui::BulletText("[%s] %s — %s (%s)", m.simbolo_ima.c_str(), m.nombre_corto.c_str(), m.formula.c_str(), m.clase_mineral.c_str());
            }
            if (ImGui::Button("Close", ImVec2(120, 0))) { mostrar_thesaurus_ = false; ImGui::CloseCurrentPopup(); }
            ImGui::EndPopup();
        }
    }
    if (mostrar_about_) {
        ImGui::OpenPopup("About HSC Geo 8.0");
        if (ImGui::BeginPopupModal("About HSC Geo 8.0", &mostrar_about_, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("HSC Geo 8.0 — Mineral Database, Modal & Petrological Module\n• Literature Mineral DB • Multi-Round Modal Solver • EPMA Formula • CIPW Norm");
            if (ImGui::Button("Close", ImVec2(120, 0))) { mostrar_about_ = false; ImGui::CloseCurrentPopup(); }
            ImGui::EndPopup();
        }
    }
    if (mostrar_resultado_) {
        ImGui::OpenPopup("Modal Calculation Result");
        if (ImGui::BeginPopupModal("Modal Calculation Result", &mostrar_resultado_, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::TextUnformatted(resultado_geo_.c_str());
            if (ImGui::Button("Close", ImVec2(120, 0))) { mostrar_resultado_ = false; ImGui::CloseCurrentPopup(); }
            ImGui::EndPopup();
        }
    }
}

} // namespace cma::interfaz
