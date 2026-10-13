#include "shared.hpp"
#include <random>
using namespace cma::interfaz;
// Struct for periodic table elements
struct ElementData {
    const char* symbol;
    int row;
    int col;
    bool available; // true = dark grey, false = light grey in HSC geo (e.g. actinides are usually dark or white depending on db)
};

// Periodic table layout mapping
const ElementData PT_ELEMENTS[118] = {
    {"H", 0, 0, true}, {"He", 0, 17, true},
    {"Li", 1, 0, true}, {"Be", 1, 1, true},
    {"B", 1, 12, true}, {"C", 1, 13, true}, {"N", 1, 14, true}, {"O", 1, 15, true}, {"F", 1, 16, true}, {"Ne", 1, 17, true},
    {"Na", 2, 0, true}, {"Mg", 2, 1, true},
    {"Al", 2, 12, true}, {"Si", 2, 13, true}, {"P", 2, 14, true}, {"S", 2, 15, true}, {"Cl", 2, 16, true}, {"Ar", 2, 17, true},
    {"K", 3, 0, true}, {"Ca", 3, 1, true},
    {"Sc", 3, 2, true}, {"Ti", 3, 3, true}, {"V", 3, 4, true}, {"Cr", 3, 5, true}, {"Mn", 3, 6, true}, {"Fe", 3, 7, true}, {"Co", 3, 8, true}, {"Ni", 3, 9, true}, {"Cu", 3, 10, true}, {"Zn", 3, 11, true},
    {"Ga", 3, 12, true}, {"Ge", 3, 13, true}, {"As", 3, 14, true}, {"Se", 3, 15, true}, {"Br", 3, 16, true}, {"Kr", 3, 17, true},
    {"Rb", 4, 0, true}, {"Sr", 4, 1, true},
    {"Y", 4, 2, true}, {"Zr", 4, 3, true}, {"Nb", 4, 4, true}, {"Mo", 4, 5, true}, {"Tc", 4, 6, true}, {"Ru", 4, 7, true}, {"Rh", 4, 8, true}, {"Pd", 4, 9, true}, {"Ag", 4, 10, true}, {"Cd", 4, 11, true},
    {"In", 4, 12, true}, {"Sn", 4, 13, true}, {"Sb", 4, 14, true}, {"Te", 4, 15, true}, {"I", 4, 16, true}, {"Xe", 4, 17, true},
    {"Cs", 5, 0, true}, {"Ba", 5, 1, true},
    /* Lanthanides are at row 7, cols 2-16 */
    {"La", 7, 2, true}, {"Ce", 7, 3, true}, {"Pr", 7, 4, true}, {"Nd", 7, 5, true}, {"Pm", 7, 6, true}, {"Sm", 7, 7, true}, {"Eu", 7, 8, true}, {"Gd", 7, 9, true}, {"Tb", 7, 10, true}, {"Dy", 7, 11, true}, {"Ho", 7, 12, true}, {"Er", 7, 13, true}, {"Tm", 7, 14, true}, {"Yb", 7, 15, true}, {"Lu", 7, 16, true},
    
    {"Hf", 5, 3, true}, {"Ta", 5, 4, true}, {"W", 5, 5, true}, {"Re", 5, 6, true}, {"Os", 5, 7, true}, {"Ir", 5, 8, true}, {"Pt", 5, 9, true}, {"Au", 5, 10, true}, {"Hg", 5, 11, true},
    {"Tl", 5, 12, true}, {"Pb", 5, 13, true}, {"Bi", 5, 14, true}, {"Po", 5, 15, true}, {"At", 5, 16, true}, {"Rn", 5, 17, true},
    
    {"Fr", 6, 0, true}, {"Ra", 6, 1, true},
    /* Actinides are at row 8, cols 2-16 */
    {"Ac", 8, 2, true}, {"Th", 8, 3, true}, {"Pa", 8, 4, true}, {"U", 8, 5, true}, {"Np", 8, 6, true}, {"Pu", 8, 7, true}, {"Am", 8, 8, true}, {"Cm", 8, 9, true}, {"Bk", 8, 10, true}, {"Cf", 8, 11, true}, {"Es", 8, 12, true}, {"Fm", 8, 13, true}, {"Md", 8, 14, true}, {"No", 8, 15, true}, {"Lr", 8, 16, true},
    
    {"Rf", 6, 3, false}, {"Db", 6, 4, false}, {"Sg", 6, 5, false}, {"Bh", 6, 6, false}, {"Hs", 6, 7, false}, {"Mt", 6, 8, false}, {"Ds", 6, 9, false}, {"Rg", 6, 10, false}, {"Cn", 6, 11, false},
    {"Nh", 6, 12, false}, {"Fl", 6, 13, false}, {"Mc", 6, 14, false}, {"Lv", 6, 15, false}, {"Ts", 6, 16, false}, {"Og", 6, 17, false}
};


void cma::interfaz::PanelGeoMineralogical::refrescar_datos() {
    db_.clear();
    if (!flowsheet_) return;
    
    // Iterar sobre especies en los flujos y agregarlas como minerales geológicos
    std::set<std::string> especies_vistas;
    
    for (const auto& [fid, f] : flowsheet_->mapa_flujos()) {
        for (const auto& [esp_id, mol] : f.flujo_molar) {
            std::string name = "Especie " + std::to_string(esp_id);
            if (especies_vistas.insert(name).second) {
                GeoMineral m;
                m.name = name;
                m.symbol = "Min";
                m.formula = "N/A";
                m.location = f.nombre;
                m.ref = "Flujo";
                db_.push_back(m);
            }
        }
    }
}

void cma::interfaz::PanelGeoMineralogical::renderizar() {
    if (ImGui::BeginTabBar("GeoTabs")) {
        if (ImGui::BeginTabItem(tr("Modal Calculation"))) {
            
            // Left Column (20% width)
            ImGui::BeginChild("left_col", ImVec2(200, 0), true);
            ImGui::TextDisabled(tr("Modal Calculation:"));
            ImGui::Separator();
            ImGui::Spacing();
            ImGui::Text(tr("1. Data Source"));
            ImGui::PushItemWidth(-FLT_MIN);
            ImGui::InputText("##datasource", data_source_, sizeof(data_source_));
            ImGui::PopItemWidth();
            if (ImGui::Button(tr("Mostrar Datos"), ImVec2(-1, 0))) mostrar_datos_ = true;
            ImGui::Spacing(); ImGui::Spacing();

            ImGui::Text(tr("2. Minerals"));
            if (ImGui::Button(tr("Definir"), ImVec2(-1, 0))) {
                // Selecciona los elementos más comunes en la base de minerales
                std::fill(element_selected_, element_selected_ + 118, false);
                static const char* comunes[] = {"Fe","Cu","Si","S","O","Al","Mg","Ca"};
                auto& tp = nucleo::TablaPeriodica::instancia();
                for (auto& m : db_) {
                    // parsear fórmula para marcar elementos
                    if (m.formula != "N/A") {
                        for (const auto& at : tp.analizar_formula(m.formula)) {
                            for (int i = 0; i < 118 && i < static_cast<int>(sizeof(PT_ELEMENTS)/sizeof(PT_ELEMENTS[0])); ++i)
                                if (PT_ELEMENTS[i].symbol == at.elem) element_selected_[i] = true;
                        }
                    }
                }
                // Si la BD está vacía, preseleccionar los comunes
                bool ninguno = true;
                for (int i = 0; i < 118; ++i) if (element_selected_[i]) { ninguno = false; break; }
                if (ninguno) {
                    auto atomos = tp.analizar_formula("FeO");
                    int npt = (int)(sizeof(PT_ELEMENTS)/sizeof(PT_ELEMENTS[0]));
                    for (int i = 0; i < npt; ++i) {
                        for (size_t c = 0; c < sizeof(comunes)/sizeof(comunes[0]); ++c)
                            if (PT_ELEMENTS[i].symbol == comunes[c]) element_selected_[i] = true;
                    }
                }
            }
            if (ImGui::Button(tr("Limpiar Selección"), ImVec2(-1, 0)))
                std::fill(element_selected_, element_selected_ + 118, false);
            ImGui::Spacing(); ImGui::Spacing();

            ImGui::Text(tr("3. Rules"));
            if (ImGui::Button(tr("Exportar"), ImVec2(-1, 0))) {
                FILE* zen = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Exportar base de minerales' 2>/dev/null");
                if (zen) {
                    char pbuf[512] = {0};
                    if (fgets(pbuf, sizeof(pbuf), zen)) {
                        std::string path(pbuf);
                        if (!path.empty() && path.back()=='\n') path.pop_back();
                        if (path.find(".csv")==std::string::npos) path += ".csv";
                        FILE* fp = fopen(path.c_str(), "w");
                        if (fp) {
                            fprintf(fp, "Mineral;Simbolo;Formula;Ubicacion;Referencia\n");
                            for (auto& m : db_)
                                fprintf(fp, "%s;%s;%s;%s;%s\n", m.name.c_str(), m.symbol.c_str(), m.formula.c_str(), m.location.c_str(), m.ref.c_str());
                            fclose(fp);
                        }
                    }
                    fclose(zen);
                }
            }
            if (ImGui::Button(tr("Importar"), ImVec2(-1, 0))) {
                FILE* zen = adapters::dialog_pipe("zenity --file-selection --file-filter='*.csv' --title='Importar base de minerales' 2>/dev/null");
                if (zen) {
                    char pbuf[512] = {0};
                    if (fgets(pbuf, sizeof(pbuf), zen)) {
                        std::string path(pbuf);
                        if (!path.empty() && path.back()=='\n') path.pop_back();
                        FILE* fp = fopen(path.c_str(), "r");
                        if (fp) {
                            char line[512];
                            bool header = true;
                            while (fgets(line, sizeof(line), fp)) {
                                std::string l = trim_str(line);
                                if (l.empty()) continue;
                                if (header) { header = false; continue; }
                                auto parts = split_str(l, ";");
                                if (parts.size() >= 3) {
                                    GeoMineral m;
                                    m.name = parts[0];
                                    m.symbol = parts[1];
                                    m.formula = parts[2];
                                    m.location = parts.size() > 3 ? parts[3] : "";
                                    m.ref = parts.size() > 4 ? parts[4] : "";
                                    db_.push_back(m);
                                }
                            }
                            fclose(fp);
                        }
                    }
                    fclose(zen);
                }
            }
            if (ImGui::Button(tr("Asistente"), ImVec2(-1, 0))) mostrar_wizard_ = true;
            ImGui::EndChild();
            
            ImGui::SameLine();
            
            // Middle/Right Column (80% width)
            ImGui::BeginChild("mid_col", ImVec2(0, 0), false);
            
            // --- Periodic Table Block ---
            ImGui::BeginChild("periodic_table", ImVec2(0, 290), true);
            ImGui::TextDisabled(tr("Elementos de Cálculo"));
            
            float btn_w = 26.0f;
            float btn_h = 24.0f;
            float spacing = 2.0f;
            ImVec2 start_pos = ImGui::GetCursorPos();
            start_pos.x += 10.0f; // Padding
            start_pos.y += 5.0f;
            
            // Draw all 118 elements
            for (int i = 0; i < 118; ++i) {
                const auto& el = PT_ELEMENTS[i];
                float px = start_pos.x + el.col * (btn_w + spacing);
                float py = start_pos.y + el.row * (btn_h + spacing);
                // Extra spacing for lanthanides/actinides
                if (el.row >= 7) {
                    py += 15.0f; // Gap between main body and f-block
                }
                
                ImGui::SetCursorPos(ImVec2(px, py));
                
                // Color coding
                bool selected = element_selected_[i];
                if (selected) {
                    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.2f, 0.6f, 0.9f, 1.0f)); // Selected Blue
                    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
                } else if (el.available) {
                    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.4f, 0.4f, 0.4f, 1.0f)); // Available Dark Grey
                    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.9f, 0.9f, 0.9f, 1.0f));
                } else {
                    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.9f, 0.9f, 0.9f, 1.0f)); // Unavailable White/Light
                    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.2f, 0.2f, 0.2f, 1.0f));
                }
                
                if (ImGui::Button(el.symbol, ImVec2(btn_w, btn_h))) {
                    element_selected_[i] = !selected;
                }
                
                ImGui::PopStyleColor(2);
            }
            ImGui::EndChild();
            
            // --- Phases and Rounds Block ---
            ImGui::BeginChild("phases_rounds", ImVec2(0, 180), true);
            if (ImGui::BeginTable("phases_split", 2, ImGuiTableFlags_BordersInnerV | ImGuiTableFlags_Resizable)) {
                ImGui::TableSetupColumn("Minerales", ImGuiTableColumnFlags_WidthStretch, 0.4f);
                ImGui::TableSetupColumn("Phases and Elements per Rounds", ImGuiTableColumnFlags_WidthStretch, 0.6f);
                ImGui::TableHeadersRow();
                
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                ImGui::BeginChild("minerals_list", ImVec2(0, 140));
                ImGui::Selectable(tr("Chalcocite (Arizona, USA)"));
                ImGui::Selectable(tr("Chalcopyrite (Erora, Portugal)"), true); // Selected
                ImGui::Selectable(tr("Pyrite (The Wilhelmine mine, Germany)"));
                ImGui::Selectable(tr("Galena (Kansas, USA)"));
                ImGui::Selectable(tr("Quartz (Kondyor PGE Placer Deposit)"));
                ImGui::Selectable(tr("Olivine (Western Bushveld complex)"));
                ImGui::EndChild();
                
                ImGui::TableSetColumnIndex(1);
                if (ImGui::BeginTabBar("Rounds")) {
                    if (ImGui::BeginTabItem(tr("Round 1"))) {
                        if (ImGui::BeginTable("round_table", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                            ImGui::TableSetupColumn("Phases");
                            ImGui::TableSetupColumn("Elements");
                            ImGui::TableHeadersRow();
                            ImGui::TableNextRow();
                            ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Chalcopyrite (Erora, Portugal)"));
                            ImGui::TableSetColumnIndex(1); ImGui::Text(tr("Cu"));
                            ImGui::EndTable();
                        }
                        ImGui::EndTabItem();
                    }
                    if (ImGui::BeginTabItem(tr("Round 2"))) ImGui::EndTabItem();
                    if (ImGui::BeginTabItem(tr("Round 3"))) ImGui::EndTabItem();
                    ImGui::EndTabBar();
                }
                ImGui::EndTable();
            }
            ImGui::EndChild();
            
            // --- Methods & Bottom Bar Block ---
            ImGui::BeginChild("methods", ImVec2(0, 120), true);
            ImGui::TextDisabled(tr("Método"));
            ImGui::RadioButton("Least Square (LS)", &current_method_, 0); ImGui::SameLine(300);
            ImGui::RadioButton("Mínimos Cuadrados No Negativos (NNLS)", &current_method_, 2);
            ImGui::RadioButton("Mínimos Cuadrados Ponderados (WLS)", &current_method_, 1); ImGui::SameLine(300);
            ImGui::RadioButton("Mínimos Cuadrados Ponderados No Negativos (WNNLS)", &current_method_, 3);
            
            ImGui::Spacing();
            ImGui::Separator();
            ImGui::Spacing();

            // Calcular: cuenta minerales y elementos seleccionados (análisis modal simple)
            if (ImGui::Button(tr("Calcular"), ImVec2(120, 25))) {
                int n_min = (int)db_.size();
                int n_elem = 0;
                for (int i = 0; i < 118; ++i) if (element_selected_[i]) n_elem++;
                const char* met = "LS";
                if (current_method_ == 1) met = "WLS";
                else if (current_method_ == 2) met = "NNLS";
                else if (current_method_ == 3) met = "WNNLS";
                char tmp[256];
                snprintf(tmp, sizeof(tmp),
                         "Metodo: %s | Minerales: %d | Elementos seleccionados: %d",
                         met, n_min, n_elem);
                resultado_geo_ = tmp;
                mostrar_resultado_ = true;
            }
            ImGui::SameLine();
            if (ImGui::Button(tr("Ecuaciones"), ImVec2(100, 25))) mostrar_datos_ = true;

            ImGui::EndChild();
            
            ImGui::EndChild(); // End mid_col
            
            ImGui::EndTabItem();
        }
        
        if (ImGui::BeginTabItem(tr("Base de Datos de Minerales"))) {
            // Enhancing Database tab

            if (ImGui::Button(tr("Refrescar Base de Datos del Flowsheet"), ImVec2(-1, 25))) {
                refrescar_datos();
            }
            ImGui::Spacing();

            ImGui::BeginChild("db_top", ImVec2(0, 150), true);
            if (ImGui::BeginTabBar("FilterTabs")) {
                if (ImGui::BeginTabItem(tr("Visual"))) {
                    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.1f, 0.1f, 1.0f), "And");
                    ImGui::BulletText("[Cu %%] Is greater than 10");
                    ImGui::BulletText("[Specific Gravity] Is between 2.2 and 4.4");
                    ImGui::EndTabItem();
                }
                if (ImGui::BeginTabItem(tr("Texto"))) {
                    ImGui::InputTextMultiline("##filter", filter_query_, sizeof(filter_query_), ImVec2(-1, 80));
                    ImGui::EndTabItem();
                }
                ImGui::EndTabBar();
            }
            if (ImGui::Button(tr("Aplicar Filtro"), ImVec2(-1, 0))) {
                // Filtro simple: texto del query buscado en el nombre/fórmula del mineral
                std::string q = filter_query_;
                std::transform(q.begin(), q.end(), q.begin(), [](unsigned char c){ return std::tolower(c); });
                int coincidencias = 0;
                for (auto& m : db_) {
                    std::string nom = m.name;
                    std::transform(nom.begin(), nom.end(), nom.begin(), [](unsigned char c){ return std::tolower(c); });
                    // Si la consulta contiene palabras como [Cu %], agregar filtro por elemento más adelante.
                    if (q.empty() || nom.find(q) != std::string::npos) coincidencias++;
                }
                resultado_geo_ = "Filtro: coinciden " + std::to_string(coincidencias) + " de " + std::to_string((int)db_.size());
                mostrar_resultado_ = true;
            }
            ImGui::EndChild();
            
            ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_Resizable | ImGuiTableFlags_ScrollY | ImGuiTableFlags_SizingFixedFit;
            if (ImGui::BeginTable("mineral_db", 6, flags, ImVec2(-1, -1))) {
                ImGui::TableSetupScrollFreeze(0, 1);
                ImGui::TableSetupColumn("Mineral Name");
                ImGui::TableSetupColumn("Mineral Symbol");
                ImGui::TableSetupColumn("Mineral Formula");
                ImGui::TableSetupColumn("Ubicación");
                ImGui::TableSetupColumn("Referencia");
                ImGui::TableSetupColumn("No. Muestra");
                ImGui::TableHeadersRow();
                
                // Dynamic Data
                for (size_t i = 0; i < db_.size(); ++i) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::Text(tr("%s"), db_[i].name.c_str());
                    ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%s"), db_[i].symbol.c_str());
                    ImGui::TableSetColumnIndex(2); ImGui::Text(tr("%s"), db_[i].formula.c_str());
                    ImGui::TableSetColumnIndex(3); ImGui::Text(tr("%s"), db_[i].location.c_str());
                    ImGui::TableSetColumnIndex(4); ImGui::Text(tr("%s"), db_[i].ref.c_str());
                    ImGui::TableSetColumnIndex(5); ImGui::Text(tr("%zu"), i + 1);
                }
                ImGui::EndTable();
            }
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }

    // ── Popup: Mostrar Datos ──
    if (mostrar_datos_) {
        ImGui::OpenPopup(tr("Fuente de Datos"));
        if (ImGui::BeginPopupModal(tr("Fuente de Datos"), &mostrar_datos_, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text(tr("Fuente: %s"), data_source_);
            ImGui::Separator();
            ImGui::Text(tr("Minerales en base: %zu"), db_.size());
            auto& tp = nucleo::TablaPeriodica::instancia();
            std::map<std::string,int> cuenta;
            for (auto& m : db_)
                if (m.formula != "N/A")
                    for (auto& at : tp.analizar_formula(m.formula)) cuenta[at.elem]++;
            for (auto& [el, c] : cuenta)
                ImGui::BulletText("%s: %d", el.c_str(), c);
            if (ImGui::Button(tr("Cerrar"))) { mostrar_datos_ = false; ImGui::CloseCurrentPopup(); }
            ImGui::EndPopup();
        }
    }

    // ── Popup: Asistente ──
    if (mostrar_wizard_) {
        ImGui::OpenPopup(tr("Asistente"));
        if (ImGui::BeginPopupModal(tr("Asistente"), &mostrar_wizard_, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::TextWrapped(tr("1. Especifica la fuente de datos (izquierda).\n2. Selecciona los elementos del cálculo en la tabla periódica o con 'Definir'.\n3. Usa la pestaña 'Base de Datos de Minerales' para refrescar/filtrar.\n4. 'Calcular' muestra un resumen del análisis modal."));
            if (ImGui::Button(tr("Cerrar"))) { mostrar_wizard_ = false; ImGui::CloseCurrentPopup(); }
            ImGui::EndPopup();
        }
    }

    // ── Popup: Resultado Cálculo ──
    if (mostrar_resultado_) {
        ImGui::OpenPopup(tr("Resultado"));
        if (ImGui::BeginPopupModal(tr("Resultado"), &mostrar_resultado_, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("%s", resultado_geo_.c_str());
            if (ImGui::Button(tr("Cerrar"))) { mostrar_resultado_ = false; ImGui::CloseCurrentPopup(); }
            ImGui::EndPopup();
        }
    }
}



#include <cstring>
#include <cstdio>
#include <cmath>


#include <cstring>
#include <cstdio>
#include <cmath>
#include <fstream>
#include <sstream>
#include <iostream>


#include <cstring>
#include <cstdio>
#include <cmath>
#include <fstream>
#include <sstream>
#include <iostream>
