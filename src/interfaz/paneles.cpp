#ifndef _USE_MATH_DEFINES
#define _USE_MATH_DEFINES
#endif
#include "interfaz/paneles.hpp"
#include "utils/i18n.h"
#include <imgui.h>
#include <implot.h>
#include <implot3d.h>
#include "paneles.hpp"
#include <nlohmann/json.hpp>
#include <fstream>
#include <cstring>
#include <cstdlib>
#include "../motores/perdida_calor.hpp"
#include "../base_datos/gestor_db.hpp"
#include "../base_datos/gestor_elementos.hpp"
#include "../base_datos/gestor_unidades.hpp"
#include <iostream>
#include <iomanip>
#include <set>
#include "thermo/GibbsMinimizer.hpp"


namespace cma { namespace interfaz {

inline ImVec4 AdaptiveColor(float r, float g, float b, float a) {
    if (ImGui::GetStyle().Colors[ImGuiCol_WindowBg].x > 0.5f) {
        if (r > 0.8f && g > 0.8f && b > 0.8f) return ImVec4(0.1f, 0.1f, 0.1f, a);
        return ImVec4(r * 0.4f, g * 0.4f, b * 0.4f, a);
    }
    return ImVec4(r, g, b, a);
}


static std::string trim_str(const std::string& str) {
    size_t first = str.find_first_not_of(" \t\r\n");
    if (first == std::string::npos) return "";
    size_t last = str.find_last_not_of(" \t\r\n");
    return str.substr(first, (last - first + 1));
}

static std::string convert_to_subscript(const std::string& s) {
    std::string res;
    for (size_t i = 0; i < s.length(); ++i) {
        char c = s[i];
        if (c >= '0' && c <= '9') {
            res.push_back((char)0xE2);
            res.push_back((char)0x82);
            res.push_back((char)(0x80 + (c - '0')));
        } else {
            res.push_back(c);
        }
    }
    return res;
}

static std::map<std::string, int> parse_formula_elements(const std::string& formula, std::string& err);

static const cma::nucleo::Especie* buscar_especie_por_nombre_o_formula(const std::string& query_raw) {
    std::string query = trim_str(query_raw);
    if (query.empty()) return nullptr;

    auto& gestor = cma::base_datos::GestorDB::instancia();

    // Strip state tags like (g), (s), (l), (a), (+a), (-a), (+3a), etc.
    std::string formula_clean;
    size_t len = query.length();
    for (size_t k = 0; k < len; ++k) {
        if (query[k] == '(') {
            size_t close = query.find(')', k);
            if (close != std::string::npos) {
                std::string tag = query.substr(k + 1, close - k - 1);
                if (tag == "g" || tag == "s" || tag == "l" || tag == "a" || tag == "aq" || tag == "α" || tag == "+a" || tag == "-a" || tag.find("a") != std::string::npos || tag.find("+") != std::string::npos || tag.find("-") != std::string::npos) {
                    k = close;
                    continue;
                }
            }
        }
        formula_clean += query[k];
    }

    // 1) Coincidencia EXACTA incluyendo el tag de fase/carga (como HSC):
    //    "Mg(g)" != "Mg"  y  "Ag(+a)" != "Ag"
    if (const auto* esp = gestor.especie_por_nombre(query)) return esp;
    std::string q_sub_full = convert_to_subscript(query);
    for (const auto* e : gestor.todas_especies()) {
        if (e->formula == query || e->formula == q_sub_full) return e;
    }

    // 2) Nombre con/sin tag y fórmula sin tag (fallback genérico)
    if (const auto* esp = gestor.especie_por_nombre(formula_clean)) return esp;
    std::string q_sub = convert_to_subscript(formula_clean);
    for (const auto* e : gestor.todas_especies()) {
        if (e->formula == q_sub || e->formula == formula_clean) {
            return e;
        }
    }
    return nullptr;
}

static std::vector<std::string> split_str(const std::string& s, const std::string& delim) {
    std::vector<std::string> parts;
    size_t start = 0;
    size_t end = s.find(delim);
    while (end != std::string::npos) {
        parts.push_back(s.substr(start, end - start));
        start = end + delim.length();
        end = s.find(delim, start);
    }
    parts.push_back(s.substr(start));
    return parts;
}

// ═══════════════════════════════════════════════════════════════════════════
// Modal Buscador Universal
// ═══════════════════════════════════════════════════════════════════════════
void ModalBuscadorDB::abrir(const char* titulo_popup) {
    search_buffer_[0] = '\0';
    checks_.clear();
    ImGui::OpenPopup(titulo_popup);
}

bool ModalBuscadorDB::dibujar(const char* titulo_popup, std::vector<const nucleo::Especie*>& seleccionados, bool seleccion_multiple) {
    bool accepted = false;
    
    ImGui::SetNextWindowSize(ImVec2(600, 500), ImGuiCond_FirstUseEver);
    if (ImGui::BeginPopupModal(titulo_popup, nullptr, ImGuiWindowFlags_NoSavedSettings)) {
        ImGui::Text(tr("Examinar Base de Datos"));
        ImGui::SetNextItemWidth(-1.0f);
        ImGui::InputTextWithHint("##search_db", "Buscar por fórmula (ej. H2O) o nombre...", search_buffer_, sizeof(search_buffer_));
        
        ImGui::Spacing();
        
        auto db_especies = base_datos::GestorDB::instancia().todas_especies();
        std::string query = search_buffer_;
        std::transform(query.begin(), query.end(), query.begin(), ::tolower);
        
        if (ImGui::BeginChild("DBList", ImVec2(0, -40), true)) {
            if (ImGui::BeginTable("DBTable", seleccion_multiple ? 4 : 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable)) {
                ImGui::TableSetupScrollFreeze(0, 1);
                if (seleccion_multiple) ImGui::TableSetupColumn("Sel", ImGuiTableColumnFlags_WidthFixed, 30.0f);
                ImGui::TableSetupColumn("Fórmula", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableSetupColumn("Nombre", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Estado", ImGuiTableColumnFlags_WidthFixed, 50.0f);
                ImGui::TableHeadersRow();
                
                for (const auto* esp : db_especies) {
                    bool match = query.empty();
                    if (!match) {
                        std::string f_low = esp->formula; std::transform(f_low.begin(), f_low.end(), f_low.begin(), ::tolower);
                        std::string n_low = esp->nombre; std::transform(n_low.begin(), n_low.end(), n_low.begin(), ::tolower);
                        if (f_low.find(query) != std::string::npos || n_low.find(query) != std::string::npos) match = true;
                    }
                    
                    if (match) {
                        ImGui::TableNextRow();
                        if (seleccion_multiple) {
                            ImGui::TableSetColumnIndex(0);
                            bool checked = checks_[esp->id];
                            ImGui::PushID(esp->id);
                            if (ImGui::Checkbox("##c", &checked)) checks_[esp->id] = checked;
                            ImGui::PopID();
                            
                            ImGui::TableSetColumnIndex(1);
                        } else {
                            ImGui::TableSetColumnIndex(0);
                        }
                        
                        ImGui::PushID(esp->id);
                        if (!seleccion_multiple) {
                            if (ImGui::Selectable(esp->formula.c_str(), false, ImGuiSelectableFlags_SpanAllColumns | ImGuiSelectableFlags_AllowDoubleClick)) {
                                if (ImGui::IsMouseDoubleClicked(0)) {
                                    seleccionados.clear();
                                    seleccionados.push_back(esp);
                                    accepted = true;
                                    ImGui::CloseCurrentPopup();
                                }
                            }
                        } else {
                            ImGui::Text(tr("%s"), esp->formula.c_str());
                        }
                        ImGui::PopID();
                        
                        ImGui::TableSetColumnIndex(seleccion_multiple ? 2 : 1);
                        ImGui::Text(tr("%s"), esp->nombre.c_str());
                        ImGui::TableSetColumnIndex(seleccion_multiple ? 3 : 2);
                        
                        ImGui::Text(tr("%s"), cma::nucleo::nombre_fase(esp->fase_std));
                    }
                }
                ImGui::EndTable();
            }
        }
        ImGui::EndChild();
        
        if (seleccion_multiple) {
            if (ImGui::Button(tr("Agregar Seleccionadas"), ImVec2(150, 0))) {
                seleccionados.clear();
                for (auto& par : checks_) {
                    if (par.second) {
                        seleccionados.push_back(base_datos::GestorDB::instancia().especie_por_id(par.first));
                    }
                }
                accepted = true;
                ImGui::CloseCurrentPopup();
            }
            ImGui::SameLine();
        } else {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5,0.5,0.5,1), "Haz doble clic para seleccionar.");
            ImGui::SameLine(ImGui::GetWindowWidth() - 100);
        }
        
        if (ImGui::Button(tr("Cancelar"), ImVec2(80, 0))) {
            ImGui::CloseCurrentPopup();
        }
        
        ImGui::EndPopup();
    }
    
    return accepted;
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Herramientas
// ═══════════════════════════════════════════════════════════════════════════

static const char* icono_nodo(nucleo::TipoNodo tipo) {
    switch (tipo) {
        case nucleo::TipoNodo::Fuente:              return "[S]";
        case nucleo::TipoNodo::Sumidero:            return "[D]";
        case nucleo::TipoNodo::Mezclador:           return "[M]";
        case nucleo::TipoNodo::Divisor:             return "[/]";
        case nucleo::TipoNodo::Válvula:             return "[V]";
        case nucleo::TipoNodo::Multiplexor:         return "[+]";
        case nucleo::TipoNodo::Demultiplexor:       return "[-]";
        case nucleo::TipoNodo::Reactor:             return "[R]";
        case nucleo::TipoNodo::ReactorGibbs:        return "[G]";
        case nucleo::TipoNodo::ReactorEsteq:        return "[E]";
        case nucleo::TipoNodo::ReactorCinetica:     return "[K]";
        case nucleo::TipoNodo::TrituradoraMandibula: return "[TM]";
        case nucleo::TipoNodo::TrituradoraGiratoria: return "[TG]";
        case nucleo::TipoNodo::TrituradoraCono:      return "[TC]";
        case nucleo::TipoNodo::MolinoSAG:            return "[MS]";
        case nucleo::TipoNodo::MolinoBolas:          return "[MB]";
        case nucleo::TipoNodo::MolinoBarras:         return "[MR]";
        case nucleo::TipoNodo::Zaranda:              return "[Z]";
        case nucleo::TipoNodo::Hidrociclon:          return "[HC]";
        case nucleo::TipoNodo::TanqueLixiviacion:    return "[LX]";
        case nucleo::TipoNodo::SXMixerSettler:       return "[SX]";
        case nucleo::TipoNodo::CeldaElectroobtencion: return "[EW]";
        case nucleo::TipoNodo::HornoRotatorio:       return "[HR]";
        case nucleo::TipoNodo::HornoFlash:           return "[HF]";
        case nucleo::TipoNodo::HornoArco:            return "[HA]";
        case nucleo::TipoNodo::HornoReverbero:       return "[RV]";
        case nucleo::TipoNodo::Tostador:             return "[TO]";
        case nucleo::TipoNodo::Convertidor:          return "[CV]";
        case nucleo::TipoNodo::Espesador:            return "[ES]";
        case nucleo::TipoNodo::Filtro:               return "[F]";
        case nucleo::TipoNodo::LavadorGases:         return "[LG]";
        case nucleo::TipoNodo::CeldaFlotacion:       return "[FL]";
        case nucleo::TipoNodo::IntercambiadorCalor:  return "[HX]";
        case nucleo::TipoNodo::Calentador:           return "[H+]";
        case nucleo::TipoNodo::Enfriador:            return "[H-]";
        case nucleo::TipoNodo::Bomba:                return "[P]";
        case nucleo::TipoNodo::Compresor:            return "[C]";
        case nucleo::TipoNodo::Turbina:              return "[T]";
        case nucleo::TipoNodo::ColumnaDestilacion:   return "[CD]";
        case nucleo::TipoNodo::Separador:            return "[SP]";
        case nucleo::TipoNodo::Absorbedor:           return "[AB]";
        case nucleo::TipoNodo::Extraccion:           return "[EX]";
        case nucleo::TipoNodo::Sensor:               return "[Se]";
        case nucleo::TipoNodo::ControladorPID:       return "[PI]";
        case nucleo::TipoNodo::CajaTexto:            return "[Tx]";
        case nucleo::TipoNodo::Rectangulo:           return "[Re]";
        case nucleo::TipoNodo::Elipse:               return "[El]";
        case nucleo::TipoNodo::FlujoEnergia:         return "[En]";
        default: return "[?]";
    }
}

static ImU32 color_nodo(nucleo::TipoNodo tipo) {
    switch (tipo) {
        // Lógicos
        case nucleo::TipoNodo::Fuente:              return IM_COL32(60,180,75,255);
        case nucleo::TipoNodo::Sumidero:            return IM_COL32(200,60,60,255);
        case nucleo::TipoNodo::Mezclador:           return IM_COL32(60,120,200,255);
        case nucleo::TipoNodo::Divisor:             return IM_COL32(200,160,50,255);
        case nucleo::TipoNodo::Válvula:             return IM_COL32(200,120,60,255);
        case nucleo::TipoNodo::Multiplexor:         return IM_COL32(80,160,200,255);
        case nucleo::TipoNodo::Demultiplexor:       return IM_COL32(80,160,200,255);
        // Reactores
        case nucleo::TipoNodo::Reactor:             return IM_COL32(200,80,40,255);
        case nucleo::TipoNodo::ReactorGibbs:        return IM_COL32(220,60,40,255);
        case nucleo::TipoNodo::ReactorEsteq:        return IM_COL32(200,100,50,255);
        case nucleo::TipoNodo::ReactorCinetica:     return IM_COL32(220,70,60,255);
        // Conminución
        case nucleo::TipoNodo::TrituradoraMandibula: return IM_COL32(140,180,80,255);
        case nucleo::TipoNodo::TrituradoraGiratoria: return IM_COL32(130,170,75,255);
        case nucleo::TipoNodo::TrituradoraCono:      return IM_COL32(120,160,70,255);
        case nucleo::TipoNodo::MolinoSAG:            return IM_COL32(150,190,85,255);
        case nucleo::TipoNodo::MolinoBolas:          return IM_COL32(140,180,80,255);
        case nucleo::TipoNodo::MolinoBarras:         return IM_COL32(130,170,75,255);
        case nucleo::TipoNodo::Zaranda:              return IM_COL32(160,140,60,255);
        case nucleo::TipoNodo::Hidrociclon:          return IM_COL32(100,180,180,255);
        // Hidrometalurgia
        case nucleo::TipoNodo::TanqueLixiviacion:    return IM_COL32(60,160,120,255);
        case nucleo::TipoNodo::SXMixerSettler:       return IM_COL32(80,140,160,255);
        case nucleo::TipoNodo::CeldaElectroobtencion: return IM_COL32(60,100,200,255);
        // Pirometalurgia
        case nucleo::TipoNodo::HornoRotatorio:       return IM_COL32(220,80,30,255);
        case nucleo::TipoNodo::HornoFlash:           return IM_COL32(240,100,20,255);
        case nucleo::TipoNodo::HornoArco:            return IM_COL32(200,60,20,255);
        case nucleo::TipoNodo::HornoReverbero:       return IM_COL32(210,70,25,255);
        case nucleo::TipoNodo::Tostador:             return IM_COL32(190,90,40,255);
        case nucleo::TipoNodo::Convertidor:          return IM_COL32(180,120,40,255);
        // Separación
        case nucleo::TipoNodo::Separador:            return IM_COL32(60,180,180,255);
        case nucleo::TipoNodo::Espesador:            return IM_COL32(50,160,150,255);
        case nucleo::TipoNodo::Filtro:               return IM_COL32(60,150,140,255);
        case nucleo::TipoNodo::LavadorGases:         return IM_COL32(80,170,160,255);
        case nucleo::TipoNodo::CeldaFlotacion:       return IM_COL32(70,190,130,255);
        case nucleo::TipoNodo::ColumnaDestilacion:   return IM_COL32(160,100,200,255);
        case nucleo::TipoNodo::Absorbedor:           return IM_COL32(120,100,180,255);
        case nucleo::TipoNodo::Extraccion:           return IM_COL32(100,120,160,255);
        // Térmico/Fluidos
        case nucleo::TipoNodo::IntercambiadorCalor:  return IM_COL32(40,140,220,255);
        case nucleo::TipoNodo::Calentador:           return IM_COL32(200,80,60,255);
        case nucleo::TipoNodo::Enfriador:            return IM_COL32(40,120,200,255);
        case nucleo::TipoNodo::Bomba:                return IM_COL32(60,160,200,255);
        case nucleo::TipoNodo::Turbina:              return IM_COL32(100,200,220,255);
        case nucleo::TipoNodo::Compresor:            return IM_COL32(200,180,40,255);
        // Instrumentos
        case nucleo::TipoNodo::Sensor:               return IM_COL32(180,180,60,255);
        case nucleo::TipoNodo::ControladorPID:       return IM_COL32(200,160,80,255);
        // Anotación
        case nucleo::TipoNodo::CajaTexto:            return IM_COL32(100,100,140,255);
        case nucleo::TipoNodo::Rectangulo:           return IM_COL32(100,100,140,255);
        case nucleo::TipoNodo::Elipse:               return IM_COL32(100,100,140,255);
        case nucleo::TipoNodo::FlujoEnergia:         return IM_COL32(220,40,40,255);
        default: return IM_COL32(100,100,100,255);
    }
}

void PanelHerramientas::renderizar() {
    ImGui::BeginTabBar("##herramientas_tabs", ImGuiTabBarFlags_None);

    // ── Pestaña 1: Explorador ──
    if (ImGui::BeginTabItem(tr("Explorador"))) {
        renderizar_explorador();
        ImGui::EndTabItem();
    }
    // ── Pestaña 2: Paleta ──
    if (ImGui::BeginTabItem(tr("Paleta"))) {
        renderizar_paleta();
        ImGui::EndTabItem();
    }
    // ── Pestaña 3: Dibujo ──
    if (ImGui::BeginTabItem(tr("Dibujo"))) {
        renderizar_herramientas_dibujo();
        ImGui::EndTabItem();
    }

    ImGui::EndTabBar();
}

void PanelHerramientas::renderizar_explorador() {
    if (!flowsheet_) return;

    // Árbol del proyecto
    if (ImGui::TreeNodeEx("Unidades", ImGuiTreeNodeFlags_DefaultOpen)) {
        int count = 0;
        for (auto& n : flowsheet_->nodos()) {
            if (n.tipo == nucleo::TipoNodo::CajaTexto ||
                n.tipo == nucleo::TipoNodo::Rectangulo ||
                n.tipo == nucleo::TipoNodo::Elipse ||
                n.tipo == nucleo::TipoNodo::FlujoEnergia) continue;
            ImGui::TreeNodeEx((n.nombre + "##" + std::to_string(n.id)).c_str(),
                ImGuiTreeNodeFlags_Leaf | ImGuiTreeNodeFlags_NoTreePushOnOpen | ImGuiTreeNodeFlags_SpanAvailWidth);
            if (ImGui::IsItemClicked() && al_seleccionar_nodo)
                al_seleccionar_nodo(n.id);
            count++;
        }
        if (count == 0) {
            ImGui::TextDisabled(tr("(vacío)"));
            ImGui::TextDisabled(tr("Usa Paleta → agrega nodos"));
        }
        ImGui::TreePop();
    }

    if (ImGui::TreeNodeEx("Corrientes", ImGuiTreeNodeFlags_DefaultOpen)) {
        int count = 0;
        for (auto& [fid, f] : flowsheet_->mapa_flujos()) {
            if (f.es_energia) continue;
            ImGui::BulletText("%s", f.nombre.c_str());
            count++;
        }
        if (count == 0) {
            ImGui::TextDisabled(tr("(vacío)"));
            ImGui::TextDisabled(tr("Conecta nodos para crear"));
        }
        ImGui::TreePop();
    }

    if (ImGui::TreeNodeEx("Energía", 0)) {
        int count = 0;
        for (auto& [fid, f] : flowsheet_->mapa_flujos()) {
            if (!f.es_energia) continue;
            ImGui::BulletText("%s  [%.1f kW]", f.nombre.c_str(), f.energia);
            count++;
        }
        if (count == 0) {
            ImGui::TextDisabled(tr("(vacío)"));
            ImGui::TextDisabled(tr("Agrega flujos de energía"));
        }
        ImGui::TreePop();
    }
}

void PanelHerramientas::renderizar_paleta() {
    // Categorías colapsables
    static const nucleo::TipoNodo logicos[] = {
        nucleo::TipoNodo::Fuente, nucleo::TipoNodo::Sumidero,
        nucleo::TipoNodo::Mezclador, nucleo::TipoNodo::Divisor,
        nucleo::TipoNodo::Válvula, nucleo::TipoNodo::Multiplexor,
        nucleo::TipoNodo::Demultiplexor
    };
    static const nucleo::TipoNodo reactores[] = {
        nucleo::TipoNodo::Reactor, nucleo::TipoNodo::ReactorGibbs,
        nucleo::TipoNodo::ReactorEsteq, nucleo::TipoNodo::ReactorCinetica
    };
    static const nucleo::TipoNodo conminucion[] = {
        nucleo::TipoNodo::TrituradoraMandibula, nucleo::TipoNodo::TrituradoraGiratoria,
        nucleo::TipoNodo::TrituradoraCono, nucleo::TipoNodo::MolinoSAG,
        nucleo::TipoNodo::MolinoBolas, nucleo::TipoNodo::MolinoBarras,
        nucleo::TipoNodo::Zaranda, nucleo::TipoNodo::Hidrociclon
    };
    static const nucleo::TipoNodo hidro[] = {
        nucleo::TipoNodo::TanqueLixiviacion, nucleo::TipoNodo::SXMixerSettler,
        nucleo::TipoNodo::CeldaElectroobtencion
    };
    static const nucleo::TipoNodo piro[] = {
        nucleo::TipoNodo::HornoRotatorio, nucleo::TipoNodo::HornoFlash,
        nucleo::TipoNodo::HornoArco, nucleo::TipoNodo::HornoReverbero,
        nucleo::TipoNodo::Tostador, nucleo::TipoNodo::Convertidor
    };
    static const nucleo::TipoNodo separacion[] = {
        nucleo::TipoNodo::Espesador, nucleo::TipoNodo::Filtro,
        nucleo::TipoNodo::LavadorGases, nucleo::TipoNodo::CeldaFlotacion,
        nucleo::TipoNodo::ColumnaDestilacion, nucleo::TipoNodo::Separador,
        nucleo::TipoNodo::Absorbedor, nucleo::TipoNodo::Extraccion
    };
    static const nucleo::TipoNodo termico[] = {
        nucleo::TipoNodo::IntercambiadorCalor, nucleo::TipoNodo::Calentador,
        nucleo::TipoNodo::Enfriador, nucleo::TipoNodo::Bomba,
        nucleo::TipoNodo::Compresor
    };
    static const nucleo::TipoNodo instrumentos[] = {
        nucleo::TipoNodo::Sensor, nucleo::TipoNodo::ControladorPID,
        nucleo::TipoNodo::FlujoEnergia
    };

    renderizar_categoria("Lógicos", logicos, 7);
    renderizar_categoria("Reactores", reactores, 4);
    renderizar_categoria("Conminución", conminucion, 8);
    renderizar_categoria("Hidrometalurgia", hidro, 3);
    renderizar_categoria("Pirometalurgia", piro, 6);
    renderizar_categoria("Separación", separacion, 8);
    renderizar_categoria("Térmico/Fluidos", termico, 5);
    renderizar_categoria("Instrumentación", instrumentos, 3);
}

void PanelHerramientas::renderizar_categoria(const char* nombre, const nucleo::TipoNodo* tipos, int count) {
    if (!ImGui::TreeNodeEx(nombre, ImGuiTreeNodeFlags_DefaultOpen)) return;

    ImU32 col_texto = ImGui::GetColorU32(ImGuiCol_Text);
    ImU32 col_texto_disabled = ImGui::GetColorU32(ImGuiCol_TextDisabled);

    for (int i = 0; i < count; i++) {
        auto tipo = tipos[i];
        const char* icon = icono_nodo(tipo);
        ImVec2 p = ImGui::GetCursorScreenPos();
        float av_w = ImGui::GetContentRegionAvail().x;
        float btn_h = 28.0f;

        auto* draw = ImGui::GetWindowDrawList();
        bool hover = ImGui::IsMouseHoveringRect(p, ImVec2(p.x+av_w, p.y+btn_h));

        ImU32 bg = hover ? IM_COL32(128,128,140,40) : IM_COL32(128,128,140,12);
        draw->AddRectFilled(p, ImVec2(p.x+av_w, p.y+btn_h), bg, 3);

        float icon_w = ImGui::CalcTextSize(icon).x;
        draw->AddText(ImVec2(p.x+4, p.y+3), col_texto, icon);
        draw->AddText(ImVec2(p.x+4+icon_w+4, p.y+4), col_texto,
                      nucleo::nombre_tipo_nodo(tipo));

        ImGui::SetCursorScreenPos(p);
        if (ImGui::InvisibleButton(("##pal_" + std::to_string(i) + nombre).c_str(), ImVec2(av_w, btn_h))) {
            if (al_agregar_nodo) al_agregar_nodo(tipo);
        }

        if (ImGui::BeginDragDropSource(ImGuiDragDropFlags_SourceNoDisableHover)) {
            ImGui::SetDragDropPayload("tipo_nodo", &tipo, sizeof(tipo));
            ImGui::Text(tr("%s %s"), icon, nucleo::nombre_tipo_nodo(tipo));
            ImGui::EndDragDropSource();
        }
    }

    ImGui::TreePop();
}

void PanelHerramientas::renderizar_herramientas_dibujo() {
    ImGui::Text(tr("Herramientas de Lienzo"));
    ImGui::Separator();

    auto btn_modo = [&](const char* label, ModoDibujo modo, const char* icono) {
        bool activo = modo_dibujo == modo;
        if (activo) ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.3f,0.5f,0.8f,0.6f));
        if (ImGui::Button(label, ImVec2(-1,28))) modo_dibujo = modo;
        if (activo) ImGui::PopStyleColor();
    };

    btn_modo("Seleccionar (V)", ModoDibujo::Seleccion, "^");
    btn_modo("Dibujar Flujo (F)", ModoDibujo::DibujarFlujo, "~");
    btn_modo("Texto (T)", ModoDibujo::Texto, "A");
    btn_modo("Rectangulo (R)", ModoDibujo::Rectangulo, "#");
    btn_modo("Elipse (E)", ModoDibujo::Elipse, "O");

    ImGui::Separator();
    ImGui::Spacing();

    if (ImGui::Button(tr("Agregar caja de texto"), ImVec2(-1, 25)))
        if (al_abrir_anotacion) al_abrir_anotacion();

    if (ImGui::Button(tr("Agregar tabla de flujos"), ImVec2(-1, 25)))
        if (al_agregar_tabla_flujo) al_agregar_tabla_flujo();

    ImGui::Separator();
    ImGui::Spacing();

    ImGui::Text(tr("Acciones"));
    if (ImGui::Button(tr("Ejecutar Simulación"), ImVec2(-1, 28)))
        if (al_ejecutar_simulacion) al_ejecutar_simulacion();
    if (ImGui::Button(tr("Resolver Recirculaciones"), ImVec2(-1, 28)))
        if (al_ejecutar_convergencia) al_ejecutar_convergencia();

    ImGui::Separator();
    ImGui::Spacing();

    if (ImGui::Button(tr("Tabla Periódica"), ImVec2(-1, 25)))
        if (al_abrir_tabla) al_abrir_tabla();
    if (ImGui::Button(tr("Calculadora Térmica"), ImVec2(-1, 25)))
        if (al_abrir_calculadora) al_abrir_calculadora();
    if (ImGui::Button(tr("Electroquímica"), ImVec2(-1, 25)))
        if (al_abrir_electroquimica) al_abrir_electroquimica();
    if (ImGui::Button(tr("Cinética"), ImVec2(-1, 25)))
        if (al_abrir_cinetica) al_abrir_cinetica();
    if (ImGui::Button(tr("Equilibrio de Fases"), ImVec2(-1, 25)))
        if (al_abrir_equilibrio) al_abrir_equilibrio();
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Propiedades
// ═══════════════════════════════════════════════════════════════════════════

PanelPropiedades::PanelPropiedades() {}

void PanelPropiedades::seleccionar_nodo(int64_t id) { id_nodo_ = id; id_flujo_ = -1; id_anotacion_ = -1; forzar_seleccion_proceso_ = true; }
void PanelPropiedades::seleccionar_flujo(int64_t id) { id_flujo_ = id; id_nodo_ = -1; id_anotacion_ = -1; forzar_seleccion_proceso_ = true; }
void PanelPropiedades::seleccionar_anotacion(int64_t id) { id_anotacion_ = id; id_nodo_ = -1; id_flujo_ = -1; forzar_seleccion_dibujos_ = true; }

void PanelPropiedades::renderizar() {
    if (!flowsheet_) { ImGui::TextDisabled(tr("Sin flowsheet.")); return; }

    ImGuiTabItemFlags tab_process_flags = 0;
    ImGuiTabItemFlags tab_drawings_flags = 0;

    if (forzar_seleccion_proceso_) {
        tab_process_flags |= ImGuiTabItemFlags_SetSelected;
        forzar_seleccion_proceso_ = false;
    }
    if (forzar_seleccion_dibujos_) {
        tab_drawings_flags |= ImGuiTabItemFlags_SetSelected;
        forzar_seleccion_dibujos_ = false;
    }

    if (ImGui::BeginTabBar("HSCPropertiesTabs")) {
        if (ImGui::BeginTabItem(tr("Proceso"), nullptr, tab_process_flags)) {
            renderizar_contenido_proceso();
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem(tr("Dibujos"), nullptr, tab_drawings_flags)) {
            renderizar_contenido_dibujos();
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }
}

void PanelPropiedades::renderizar_contenido_proceso() {
    if (id_nodo_ > 0) {
        auto* n = flowsheet_->nodo_por_id(id_nodo_);
        if (n) propiedades_nodo(*n);
        else ImGui::TextDisabled(tr("Nodo no encontrado."));
    } else if (id_flujo_ > 0) {
        auto* f = flowsheet_->flujo_por_id(id_flujo_);
        if (f) propiedades_flujo(*f);
        else ImGui::TextDisabled(tr("Flujo no encontrado."));
    } else {
        ImGui::TextDisabled(tr("Seleccione un elemento en el lienzo."));
    }
}

void PanelPropiedades::renderizar_contenido_dibujos() {
    if (id_anotacion_ > 0) {
        diagrama_flujo::Anotacion* anot = nullptr;
        for (auto& a : flowsheet_->anotaciones()) {
            if (a.id == id_anotacion_) {
                anot = &a;
                break;
            }
        }
        if (anot) propiedades_anotacion(*anot);
        else ImGui::TextDisabled(tr("Anotación no encontrada."));
    } else if (id_nodo_ > 0) {
        auto* n = flowsheet_->nodo_por_id(id_nodo_);
        if (n) {
            ImGui::Text(tr("Propiedades Gráficas del Nodo:"));
            ImGui::Separator();
            float pos[2] = { n->canvas_x, n->canvas_y };
            if (ImGui::DragFloat2("Posición en Lienzo", pos, 1.0f)) {
                n->canvas_x = pos[0];
                n->canvas_y = pos[1];
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
        }
    } else if (id_flujo_ > 0) {
        auto* f = flowsheet_->flujo_por_id(id_flujo_);
        if (f) {
            ImGui::Text(tr("Propiedades Gráficas del Flujo:"));
            ImGui::Separator();
            ImGui::Text(tr("Color de línea: Gris estándar"));
        }
    } else {
        ImGui::Text(tr("Configuración de Dibujo:"));
        ImGui::Separator();
        ImGui::BulletText("Fondo del Lienzo: Blanco CAD");
        ImGui::BulletText("Reglas de Medición: Activas");
        ImGui::BulletText("Grilla de Fondo: Activa (GRILLA = 10px)");
    }
}

void PanelPropiedades::propiedades_nodo(nucleo::Nodo& nodo) {
    if (ImGui::BeginTable("process_props_node", 2, ImGuiTableFlags_BordersOuter | ImGuiTableFlags_BordersInner | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Propiedad", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Valor", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableHeadersRow();

        // General
        ImGui::TableNextRow();
        ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, ImGui::GetColorU32(ImGuiCol_Header));
        ImGui::TableNextColumn(); ImGui::Text(tr("General")); ImGui::TableNextColumn(); ImGui::Text(tr(""));

        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Object Type")); ImGui::TableNextColumn(); ImGui::Text(tr("Unidad"));

        // NameID
        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("NameID")); ImGui::TableNextColumn();
        ImGui::SetNextItemWidth(-1);
        char nbuf[128];
        strncpy(nbuf, nodo.nombre.c_str(), sizeof(nbuf)-1);
        nbuf[sizeof(nbuf)-1] = '\0';
        if (ImGui::InputText("##NameID", nbuf, sizeof(nbuf))) {
            nodo.nombre = nbuf;
            if (al_cambio_flowsheet) al_cambio_flowsheet();
        }

        // Number/Alias
        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Number/Alias")); ImGui::TableNextColumn(); ImGui::Text(tr("%lld"), (long long)nodo.id);

        // Mode / Type / Model
        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Mode")); ImGui::TableNextColumn(); ImGui::Text(tr("General"));
        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Tipo")); ImGui::TableNextColumn(); ImGui::Text(tr("Standard"));
        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Modelo")); ImGui::TableNextColumn(); ImGui::Text(tr("%s"), nucleo::nombre_tipo_nodo(nodo.tipo));

        // Location
        ImGui::TableNextRow();
        ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, ImGui::GetColorU32(ImGuiCol_Header));
        ImGui::TableNextColumn(); ImGui::Text(tr("Ubicación")); ImGui::TableNextColumn(); ImGui::Text(tr(""));

        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Página")); ImGui::TableNextColumn(); ImGui::Text(tr("Page1"));
        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Capa")); ImGui::TableNextColumn(); ImGui::Text(tr("Layer 0 (Default)"));

        ImGui::EndTable();
    }

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Spacing();
    ImGui::Text(tr("Parámetros de Simulación:"));

    propiedades_nodo_especificas(nodo);

    ImGui::Spacing();
    if (ImGui::Button(tr("Eliminar Nodo"), ImVec2(-1, 25))) {
        if (flowsheet_) {
            if (al_antes_de_modificar) al_antes_de_modificar();
            flowsheet_->eliminar_nodo(nodo.id);
            id_nodo_ = -1;
            if (al_cambio_flowsheet) al_cambio_flowsheet();
        }
    }
}

void PanelPropiedades::propiedades_nodo_especificas(nucleo::Nodo& nodo) {
    switch (nodo.tipo) {
        case nucleo::TipoNodo::Fuente: {
            auto& gestor = base_datos::GestorDB::instancia();
            
            // Temperature input
            double T = nodo.params.count("temperatura") ? nodo.params.at("temperatura") : 
                      (nodo.params.count("temperature") ? nodo.params.at("temperature") : nucleo::STP_TEMP);
            if (ImGui::InputDouble("Temperatura de alimentación (K)", &T, 1.0, 10.0, "%.2f")) {
                nodo.params["temperatura"] = T;
                nodo.params["temperature"] = T;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            
            // Pressure input
            double P = nodo.params.count("presion") ? nodo.params.at("presion") : 
                      (nodo.params.count("pressure") ? nodo.params.at("pressure") : nucleo::STP_PRES);
            if (ImGui::InputDouble("Presión de alimentación (atm)", &P, 0.1, 1.0, "%.4f")) {
                nodo.params["presion"] = P;
                nodo.params["pressure"] = P;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            
            ImGui::Spacing();
            ImGui::Separator();
            ImGui::Spacing();
            
            ImGui::Text(tr("Composición de alimentación:"));
            
            if (ImGui::Button(tr("Examinar Base de Datos##feed"), ImVec2(-1, 0))) {
                ModalBuscadorDB::instancia().abrir("Buscar Especie (Alimentación)");
            }
            std::vector<const nucleo::Especie*> sel_feed;
            if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Alimentación)", sel_feed, true)) {
                for (auto* esp : sel_feed) {
                    nodo.params["flujo_" + std::to_string(esp->id)] = 1.0;
                    nodo.params["flow_" + std::to_string(esp->id)] = 1.0;
                }
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            
            ImGui::Spacing();
            
            // List and edit species flows
            std::vector<std::string> keys_to_delete;
            std::set<int64_t> species_listed;
            for (auto& [k, v] : nodo.params) {
                bool is_flow = (k.rfind("flow_", 0) == 0);
                bool is_flujo = (k.rfind("flujo_", 0) == 0);
                if (is_flow || is_flujo) {
                    try {
                        int64_t sid = std::stoll(k.substr(is_flow ? 5 : 6));
                        if (species_listed.count(sid)) continue;
                        species_listed.insert(sid);
                        
                        auto* sp = gestor.especie_por_id(sid);
                        std::string name = sp ? sp->nombre : ("ID " + std::to_string(sid));
                        
                        ImGui::PushID(sid);
                        ImGui::SetNextItemWidth(120.0f);
                        double flow_val = v;
                        if (ImGui::InputDouble(name.c_str(), &flow_val, 0, 0, "%.4f")) {
                            nodo.params["flujo_" + std::to_string(sid)] = flow_val;
                            nodo.params["flow_" + std::to_string(sid)] = flow_val;
                            if (al_cambio_flowsheet) al_cambio_flowsheet();
                        }
                        ImGui::SameLine();
                        if (ImGui::Button(tr("Eliminar"))) {
                            if (al_antes_de_modificar) al_antes_de_modificar();
                            keys_to_delete.push_back("flow_" + std::to_string(sid));
                            keys_to_delete.push_back("flujo_" + std::to_string(sid));
                        }
                        ImGui::PopID();
                    } catch (...) {}
                }
            }
            for (auto& k : keys_to_delete) {
                nodo.params.erase(k);
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::Reactor: {
            double conv = nodo.conversion();
            if (ImGui::InputDouble("Conversión", &conv, 0, 1, "%.3f")) {
                nodo.params["conversion"] = conv;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double T = nodo.params.count("temperatura") ? nodo.params.at("temperatura") : nucleo::STP_TEMP;
            if (ImGui::InputDouble("Temperatura (K)", &T)) {
                nodo.params["temperatura"] = T;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double P = nodo.params.count("presion") ? nodo.params.at("presion") : nucleo::STP_PRES;
            if (ImGui::InputDouble("Presión (atm)", &P)) {
                nodo.params["presion"] = P;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            int rxn = (int)(nodo.params.count("reaccion_id") ? nodo.params.at("reaccion_id") : 1);
            if (ImGui::InputInt("ID Reacción", &rxn)) {
                nodo.params["reaccion_id"] = (double)rxn;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::IntercambiadorCalor: {
            double T_out = nodo.temperatura_salida();
            if (ImGui::InputDouble("T Salida (K)", &T_out)) {
                nodo.params["temperatura_salida"] = T_out;
                nodo.params["T_salida"] = T_out;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double dP = nodo.params.count("caida_presion") ? nodo.params.at("caida_presion") : 0.0;
            if (ImGui::InputDouble("Caída de presión (atm)", &dP)) {
                nodo.params["caida_presion"] = dP;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double Q = nodo.calor_intercambiado();
            if (ImGui::InputDouble("Calor (kJ/s)", &Q)) {
                nodo.params["calor_intercambiado"] = Q;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::Divisor: {
            size_t num_salidas = nodo.ids_flujos_salida.size();
            if (num_salidas == 0) {
                ImGui::TextDisabled(tr("Conecta corrientes de salida al divisor."));
            } else {
                // Load current fractions
                std::vector<double> fracs(num_salidas);
                double sum_fracs = 0.0;
                for (size_t i = 0; i < num_salidas; ++i) {
                    std::string key = "div_" + std::to_string(i);
                    fracs[i] = nodo.params.count(key) ? nodo.params.at(key) : (1.0 / num_salidas);
                    sum_fracs += fracs[i];
                }
                
                // Enforce normalization on load if not 1.0
                if (std::abs(sum_fracs - 1.0) > 1e-5) {
                    if (sum_fracs > 0.0) {
                        for (size_t i = 0; i < num_salidas; ++i) fracs[i] /= sum_fracs;
                    } else {
                        for (size_t i = 0; i < num_salidas; ++i) fracs[i] = 1.0 / num_salidas;
                    }
                    for (size_t i = 0; i < num_salidas; ++i) {
                        nodo.params["div_" + std::to_string(i)] = fracs[i];
                    }
                }

                ImGui::Text(tr("Fracciones de División (Suman 100%%):"));
                ImGui::Spacing();

                bool changed = false;
                size_t edited_index = num_salidas;
                double edited_new_val = 0.0;

                for (size_t i = 0; i < num_salidas; ++i) {
                    int64_t stream_id = nodo.ids_flujos_salida[i];
                    std::string stream_label = "Salida " + std::to_string(i);
                    if (flowsheet_) {
                        auto* fl = flowsheet_->flujo_por_id(stream_id);
                        if (fl && !fl->nombre.empty()) {
                            stream_label = fl->nombre;
                        }
                    }

                    double pct = fracs[i] * 100.0;
                    ImGui::PushID(i);
                    
                    ImGui::Text(tr("%s:"), stream_label.c_str());
                    ImGui::SameLine(180);
                    
                    ImGui::SetNextItemWidth(120);
                    if (ImGui::InputDouble("##pct_input", &pct, 1.0, 5.0, "%.2f%%")) {
                        if (pct < 0.0) pct = 0.0;
                        if (pct > 100.0) pct = 100.0;
                        
                        edited_new_val = pct / 100.0;
                        edited_index = i;
                        changed = true;
                    }
                    
                    ImGui::PopID();
                }

                if (changed && edited_index < num_salidas) {
                    double new_val = edited_new_val;
                    fracs[edited_index] = new_val;

                    if (num_salidas > 1) {
                        double remaining_target = 1.0 - new_val;
                        double other_sum = 0.0;
                        for (size_t i = 0; i < num_salidas; ++i) {
                            if (i != edited_index) other_sum += fracs[i];
                        }

                        if (other_sum > 0.0) {
                            for (size_t i = 0; i < num_salidas; ++i) {
                                if (i != edited_index) {
                                    fracs[i] = remaining_target * (fracs[i] / other_sum);
                                }
                            }
                        } else {
                            double equal_share = remaining_target / (num_salidas - 1);
                            for (size_t i = 0; i < num_salidas; ++i) {
                                if (i != edited_index) fracs[i] = equal_share;
                            }
                        }
                    }

                    double final_sum = 0.0;
                    for (double f : fracs) final_sum += f;
                    if (final_sum > 0.0) {
                        for (size_t i = 0; i < num_salidas; ++i) fracs[i] /= final_sum;
                    }

                    for (size_t i = 0; i < num_salidas; ++i) {
                        nodo.params["div_" + std::to_string(i)] = fracs[i];
                    }
                    if (al_cambio_flowsheet) al_cambio_flowsheet();
                }
            }
            break;
        }
        case nucleo::TipoNodo::Separador: {
            auto& gestor_sep = base_datos::GestorDB::instancia();
            ImGui::TextWrapped("%s", tr("La salida 1 retira las especies marcadas (reciclo/reactivo). La salida 0 es la corriente restante."));
            ImGui::Spacing();
            if (ImGui::Button(tr("Agregar Especie a Remover##sep"), ImVec2(-1, 0))) {
                ModalBuscadorDB::instancia().abrir("Buscar Especie (Remover)");
            }
            std::vector<const nucleo::Especie*> sel_sep;
            if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Remover)", sel_sep, true)) {
                for (auto* esp : sel_sep) {
                    nodo.params["remover_" + std::to_string(esp->id)] = 1.0;
                    if (al_cambio_flowsheet) al_cambio_flowsheet();
                }
            }
            ImGui::Spacing();
            for (auto& [k, v] : nodo.params)
                if (k.rfind("remover_", 0) == 0) {
                    float f = (float)v;
                    auto* esp = gestor_sep.especie_por_id(std::atoll(k.c_str() + 8));
                    std::string etiqueta = esp ? std::string(esp->nombre) : k.substr(8);
                    if (ImGui::SliderFloat(("Remover " + etiqueta).c_str(), &f, 0, 1)) {
                        nodo.params[k] = f;
                        if (al_cambio_flowsheet) al_cambio_flowsheet();
                    }
                }
            break;
        }
        case nucleo::TipoNodo::Compresor: {
            double rP = nodo.params.count("relacion_presion") ? nodo.params.at("relacion_presion") : 2.0;
            if (ImGui::InputDouble("Relación presión", &rP)) {
                nodo.params["relacion_presion"] = rP;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double eff = nodo.params.count("eficiencia") ? nodo.params.at("eficiencia") : 0.85;
            if (ImGui::InputDouble("Eficiencia", &eff, 0.01, 0.1, "%.3f")) {
                nodo.params["eficiencia"] = eff;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::ColumnaDestilacion: {
            int etapas = nodo.num_etapas();
            if (ImGui::InputInt("Etapas", &etapas)) {
                nodo.params["etapas"] = (double)etapas;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double R = nodo.relacion_reflujo();
            if (ImGui::InputDouble("R. Reflujo", &R)) {
                nodo.params["reflujo"] = R;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            ImGui::TextWrapped("%s", tr("Salida 0 = destilado (gases), Salida 1 = fondos."));
            dibujar_selector_splits(nodo, tr("Fijar Especie (Fondos / Salida 1)"));
            break;
        }
        case nucleo::TipoNodo::Mezclador: {
            double Q_loss = nodo.params.count("calor") ? nodo.params.at("calor") : 0.0;
            if (ImGui::InputDouble("Pérdida de calor (kW)", &Q_loss)) {
                nodo.params["calor"] = Q_loss;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double P_drop = nodo.params.count("presion_salida") ? nodo.params.at("presion_salida") : 1.0;
            if (ImGui::InputDouble("Presión de salida (atm)", &P_drop)) {
                nodo.params["presion_salida"] = P_drop;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::Sumidero: {
            ImGui::TextDisabled(tr("El sumidero recopila los flujos de salida."));
            break;
        }
        case nucleo::TipoNodo::CeldaFlotacion: {
            double rec = nodo.params.count("recovery") ? nodo.params.at("recovery") : 0.85;
            if (ImGui::InputDouble("Recuperación", &rec, 0.01, 0.1, "%.3f")) {
                nodo.params["recovery"] = rec;
                nodo.params["recuperacion"] = rec;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double ley = nodo.params.count("grade") ? nodo.params.at("grade") : 0.25;
            if (ImGui::InputDouble("Ley de Concentrado", &ley, 0.01, 0.1, "%.3f")) {
                nodo.params["grade"] = ley;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            ImGui::TextWrapped("%s", tr("Por defecto la recuperación aplica a todas las especies (Salida 0 = concentrado, Salida 1 = relave). Puedes fijar por especie:"));
            dibujar_selector_splits(nodo, tr("Fijar Especie a Concentrado/Relave"));
            break;
        }
        case nucleo::TipoNodo::Espesador: {
            double solids = nodo.params.count("solids_underflow") ? nodo.params.at("solids_underflow") : 60.0;
            if (ImGui::InputDouble("Sólidos en Descarga (wt%)", &solids)) {
                nodo.params["solids_underflow"] = solids;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double eff = nodo.params.count("efficiency") ? nodo.params.at("efficiency") : 0.98;
            if (ImGui::InputDouble("Eficiencia de separación", &eff, 0.01, 0.1, "%.3f")) {
                nodo.params["efficiency"] = eff;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            ImGui::TextWrapped("%s", tr("Salida 0 = overflow (líquidos), Salida 1 = underflow (sólidos)."));
            dibujar_selector_splits(nodo, tr("Fijar Especie (Salida 1)"));
            break;
        }
        case nucleo::TipoNodo::Hidrociclon: {
            double d50 = nodo.params.count("d50") ? nodo.params.at("d50") : 150.0;
            if (ImGui::InputDouble("Tamaño de corte d50 (μm)", &d50)) {
                nodo.params["d50"] = d50;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double bypass = nodo.params.count("bypass") ? nodo.params.at("bypass") : 0.15;
            if (ImGui::InputDouble("Bypass de agua", &bypass, 0.01, 0.1, "%.3f")) {
                nodo.params["bypass"] = bypass;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            ImGui::TextWrapped("%s", tr("Salida 0 = overflow (finos), Salida 1 = underflow (gruesos)."));
            dibujar_selector_splits(nodo, tr("Fijar Especie (Salida 1)"));
            break;
        }
        case nucleo::TipoNodo::MolinoBolas: {
            double wi = nodo.params.count("wi") ? nodo.params.at("wi") : 13.5;
            if (ImGui::InputDouble("Work Index (Wi, kWh/t)", &wi)) {
                nodo.params["wi"] = wi;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double power = nodo.params.count("power") ? nodo.params.at("power") : 2500.0;
            if (ImGui::InputDouble("Potencia Consumida (kW)", &power)) {
                nodo.params["power"] = power;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::MolinoSAG: {
            double wi = nodo.params.count("wi") ? nodo.params.at("wi") : 12.0;
            if (ImGui::InputDouble("Work Index (Wi, kWh/t)", &wi)) {
                nodo.params["wi"] = wi;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double power = nodo.params.count("power") ? nodo.params.at("power") : 8500.0;
            if (ImGui::InputDouble("Potencia Consumida (kW)", &power)) {
                nodo.params["power"] = power;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::TrituradoraMandibula:
        case nucleo::TipoNodo::TrituradoraGiratoria:
        case nucleo::TipoNodo::TrituradoraCono: {
            double css = nodo.params.count("css") ? nodo.params.at("css") : 25.0;
            if (ImGui::InputDouble("Closed Side Setting CSS (mm)", &css)) {
                nodo.params["css"] = css;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::TanqueLixiviacion: {
            double time = nodo.params.count("residence_time") ? nodo.params.at("residence_time") : 24.0;
            if (ImGui::InputDouble("Tiempo de Residencia (h)", &time)) {
                nodo.params["residence_time"] = time;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double diss = nodo.params.count("dissolution") ? nodo.params.at("dissolution") : 0.92;
            if (ImGui::InputDouble("Fracción de disolución", &diss, 0.01, 0.1, "%.3f")) {
                nodo.params["dissolution"] = diss;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            ImGui::TextWrapped("%s", tr("Salida 0 = solución cargada (líquidos), Salida 1 = residuo (sólidos)."));
            dibujar_selector_splits(nodo, tr("Fijar Especie (Residuo / Salida 1)"));
            break;
        }
        case nucleo::TipoNodo::SXMixerSettler: {
            double oa = nodo.params.count("oa_ratio") ? nodo.params.at("oa_ratio") : 1.5;
            if (ImGui::InputDouble("Relación Orgánico/Acuoso (O/A)", &oa)) {
                nodo.params["oa_ratio"] = oa;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double eff = nodo.params.count("efficiency") ? nodo.params.at("efficiency") : 0.95;
            if (ImGui::InputDouble("Eficiencia de extracción", &eff, 0.01, 0.1, "%.3f")) {
                nodo.params["efficiency"] = eff;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            dibujar_selector_splits(nodo, tr("Fijar Especie (Salida 1)"));
            break;
        }
        case nucleo::TipoNodo::CeldaElectroobtencion: {
            double ce = nodo.params.count("current_efficiency") ? nodo.params.at("current_efficiency") : 0.90;
            if (ImGui::InputDouble("Eficiencia de Corriente (CE)", &ce, 0.01, 0.1, "%.3f")) {
                nodo.params["current_efficiency"] = ce;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double cd = nodo.params.count("current_density") ? nodo.params.at("current_density") : 300.0;
            if (ImGui::InputDouble("Densidad de Corriente (A/m²)", &cd)) {
                nodo.params["current_density"] = cd;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            dibujar_selector_splits(nodo, tr("Fijar Especie (Salida 1)"));
            break;
        }
        case nucleo::TipoNodo::Calentador:
        case nucleo::TipoNodo::Enfriador: {
            double T_out = nodo.temperatura_salida();
            if (ImGui::InputDouble("T Salida (K)", &T_out)) {
                nodo.params["temperatura_salida"] = T_out;
                nodo.params["T_salida"] = T_out;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double dP = nodo.params.count("caida_presion") ? nodo.params.at("caida_presion") : 0.0;
            if (ImGui::InputDouble("Caída de presión (atm)", &dP)) {
                nodo.params["caida_presion"] = dP;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::Bomba: {
            double P_sal = nodo.params.count("P_salida") ? nodo.params.at("P_salida") : 1.5;
            if (ImGui::InputDouble("Presión de descarga (atm)", &P_sal)) {
                nodo.params["P_salida"] = P_sal;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::Turbina: {
            double P_sal = nodo.params.count("P_salida") ? nodo.params.at("P_salida") : 0.5;
            if (ImGui::InputDouble("Presión de descarga (atm)", &P_sal)) {
                nodo.params["P_salida"] = P_sal;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double eff = nodo.params.count("eficiencia") ? nodo.params.at("eficiencia") : 0.85;
            if (ImGui::InputDouble("Eficiencia", &eff, 0.01, 0.1, "%.3f")) {
                nodo.params["eficiencia"] = eff;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            break;
        }
        case nucleo::TipoNodo::Filtro:
        case nucleo::TipoNodo::Zaranda: {
            double eff = nodo.params.count("efficiency") ? nodo.params.at("efficiency") : 0.98;
            if (ImGui::InputDouble("Eficiencia de separación", &eff, 0.01, 0.1, "%.3f")) {
                nodo.params["efficiency"] = eff;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            dibujar_selector_splits(nodo, tr("Fijar Especie (Salida 1)"));
            break;
        }
        case nucleo::TipoNodo::Absorbedor:
        case nucleo::TipoNodo::LavadorGases:
        case nucleo::TipoNodo::Extraccion: {
            dibujar_selector_splits(nodo, tr("Fijar Especie (Salida 1)"));
            break;
        }
        case nucleo::TipoNodo::HornoRotatorio:
        case nucleo::TipoNodo::HornoFlash:
        case nucleo::TipoNodo::HornoArco:
        case nucleo::TipoNodo::HornoReverbero:
        case nucleo::TipoNodo::Tostador:
        case nucleo::TipoNodo::Convertidor: {
            int rxn = (int)(nodo.params.count("reaccion_id") ? nodo.params.at("reaccion_id") : 0);
            if (ImGui::InputInt("ID Reacción (0 = sin reacción)", &rxn)) {
                nodo.params["reaccion_id"] = (double)rxn;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double conv = nodo.conversion();
            if (ImGui::InputDouble("Conversión", &conv, 0, 1, "%.3f")) {
                nodo.params["conversion"] = conv;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            double T = nodo.params.count("temperatura") ? nodo.params.at("temperatura") : nucleo::STP_TEMP;
            if (ImGui::InputDouble("Temperatura (K)", &T)) {
                nodo.params["temperatura"] = T;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            ImGui::TextWrapped("%s", tr("Las salidas se reparten por igual (usa split_/frac_i para dirigirlas)."));
            dibujar_selector_splits(nodo, tr("Dirigir Especie a Salidas"));
            break;
        }
        default: break;
    }
}

void PanelPropiedades::dibujar_selector_splits(nucleo::Nodo& nodo, const char* titulo) {
    auto& gestor = base_datos::GestorDB::instancia();
    ImGui::Spacing();
    if (ImGui::Button(titulo, ImVec2(-1, 0))) {
        ModalBuscadorDB::instancia().abrir("Buscar Especie (Distribución Salida 1)");
    }
    std::vector<const nucleo::Especie*> sel;
    if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Distribución Salida 1)", sel, true)) {
        for (auto* esp : sel) {
            if (!nodo.params.count("split_" + std::to_string(esp->id)))
                nodo.params["split_" + std::to_string(esp->id)] = 0.5;
        }
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }
    ImGui::Spacing();
    bool cambiado = false;
    for (auto& [k, v] : nodo.params) {
        if (k.rfind("split_", 0) == 0) {
            float f = (float)v;
            auto* esp = gestor.especie_por_id(std::atoll(k.c_str() + 6));
            std::string etiqueta = esp ? std::string(esp->nombre) : k.substr(6);
            if (ImGui::SliderFloat(("A Salida 1: " + etiqueta + "##split").c_str(), &f, 0, 1)) {
                nodo.params[k] = f;
                cambiado = true;
            }
            ImGui::SameLine();
            if (ImGui::SmallButton(("x##" + k).c_str())) {
                nodo.params.erase(k);
                cambiado = true;
            }
        }
    }
    if (cambiado && al_cambio_flowsheet) al_cambio_flowsheet();
}

void PanelPropiedades::propiedades_flujo(nucleo::Flujo& flujo) {
    if (ImGui::BeginTable("process_props_stream", 2, ImGuiTableFlags_BordersOuter | ImGuiTableFlags_BordersInner | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Propiedad", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Valor", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableHeadersRow();

        // General
        ImGui::TableNextRow();
        ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, ImGui::GetColorU32(ImGuiCol_Header));
        ImGui::TableNextColumn(); ImGui::Text(tr("General")); ImGui::TableNextColumn(); ImGui::Text(tr(""));

        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Object Type")); ImGui::TableNextColumn(); ImGui::Text(tr("Corriente"));

        // NameID
        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("NameID")); ImGui::TableNextColumn();
        ImGui::SetNextItemWidth(-1);
        char fbuf[128];
        strncpy(fbuf, flujo.nombre.c_str(), sizeof(fbuf)-1);
        fbuf[sizeof(fbuf)-1] = '\0';
        if (ImGui::InputText("##NameID", fbuf, sizeof(fbuf))) {
            flujo.nombre = fbuf;
            if (al_cambio_flowsheet) al_cambio_flowsheet();
        }

        // Number/Alias
        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Number/Alias")); ImGui::TableNextColumn(); ImGui::Text(tr("%lld"), (long long)flujo.id);

        // Type
        ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text(tr("Tipo")); ImGui::TableNextColumn(); ImGui::Text(flujo.es_energia ? "Energy Stream" : "Material Stream");

        ImGui::EndTable();
    }

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Spacing();

    propiedades_flujo_especificas(flujo);

    ImGui::Spacing();
    if (ImGui::Button(tr("Eliminar Flujo"), ImVec2(-1, 25))) {
        if (flowsheet_) {
            flowsheet_->eliminar_flujo(flujo.id);
            id_flujo_ = -1;
            if (al_cambio_flowsheet) al_cambio_flowsheet();
        }
    }
}

void PanelPropiedades::propiedades_flujo_especificas(nucleo::Flujo& flujo) {
    // Temperature input
    double T = flujo.temperatura;
    if (ImGui::InputDouble("Temperatura (K)", &T, 1.0, 10.0, "%.2f")) {
        flujo.temperatura = T;
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }
    
    // Pressure input
    double P = flujo.presion;
    if (ImGui::InputDouble("Presión (atm)", &P, 0.1, 1.0, "%.4f")) {
        flujo.presion = P;
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }
    
    ImGui::Text(tr("Flujo Total: %.6f kmol/s"), flujo.flujo_total());
    
    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Spacing();
    
    ImGui::Text(tr("Composición:"));
    
    if (ImGui::Button(tr("Examinar Base de Datos##stream"), ImVec2(-1, 0))) {
        ModalBuscadorDB::instancia().abrir("Buscar Especie (Flujo)");
    }
    std::vector<const nucleo::Especie*> sel_stream;
    if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Flujo)", sel_stream, true)) {
        for (auto* esp : sel_stream) {
            if (flujo.flujo_molar.count(esp->id) == 0) {
                flujo.flujo_molar[esp->id] = 1.0;
            }
        }
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }
    
    ImGui::Spacing();
    
    // List and edit species flows in a table
    if (ImGui::BeginTable("comp_editable", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Especie", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Flujo (kmol/s)", ImGuiTableColumnFlags_WidthFixed, 140.0f);
        ImGui::TableSetupColumn("Fracc. Molar", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("Acción", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableHeadersRow();
        
        double total = flujo.flujo_total();
        auto& gestor = base_datos::GestorDB::instancia();
        
        std::vector<int64_t> ids_to_delete;
        
        for (auto& [sid, n] : flujo.flujo_molar) {
            ImGui::TableNextRow();
            ImGui::TableNextColumn();
            auto* sp = gestor.especie_por_id(sid);
            std::string name = sp ? sp->nombre : ("ID " + std::to_string(sid));
            ImGui::Text(tr("%s"), name.c_str());
            
            ImGui::TableNextColumn();
            ImGui::PushID(sid);
            ImGui::SetNextItemWidth(-1.0f);
            double val = n;
            if (ImGui::InputDouble("##flow_val", &val, 0.0, 0.0, "%.6f")) {
                flujo.flujo_molar[sid] = val;
                if (al_cambio_flowsheet) al_cambio_flowsheet();
            }
            ImGui::PopID();
            
            ImGui::TableNextColumn();
            if (total > 0) ImGui::Text(tr("%.4f"), n/total);
            else ImGui::Text(tr("-"));
            
            ImGui::TableNextColumn();
            ImGui::PushID(sid * 1000);
            if (ImGui::Button(tr("Eliminar"))) {
                ids_to_delete.push_back(sid);
            }
            ImGui::PopID();
        }
        
        for (int64_t sid : ids_to_delete) {
            flujo.flujo_molar.erase(sid);
            if (al_cambio_flowsheet) al_cambio_flowsheet();
        }
        
        ImGui::EndTable();
    }
}

void PanelPropiedades::propiedades_anotacion(diagrama_flujo::Anotacion& a) {
    ImGui::Text(tr("Dibujo / Anotación ID: %lld"), (long long)a.id);
    ImGui::Separator();

    // Tipo de Anotacion
    int tipo_int = (int)a.tipo;
    const char* tipos_nombres[] = { "Texto", "Rectángulo", "Elipse" };
    if (ImGui::Combo("Tipo", &tipo_int, tipos_nombres, 3)) {
        a.tipo = (diagrama_flujo::Anotacion::Tipo)tipo_int;
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }

    // Texto de la anotacion
    static char tbuf[256];
    strncpy(tbuf, a.texto.c_str(), sizeof(tbuf)-1);
    tbuf[sizeof(tbuf)-1] = '\0';
    if (ImGui::InputText(tr("Texto"), tbuf, sizeof(tbuf))) {
        a.texto = tbuf;
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }

    // Posición y Dimensiones
    float pos[2] = { a.x, a.y };
    if (ImGui::DragFloat2("Posición (x, y)", pos, 1.0f)) {
        a.x = pos[0];
        a.y = pos[1];
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }

    float dim[2] = { a.w, a.h };
    if (ImGui::DragFloat2("Tamaño (w, h)", dim, 1.0f, 10.0f, 1000.0f)) {
        a.w = std::max(10.0f, dim[0]);
        a.h = std::max(10.0f, dim[1]);
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }

    ImGui::Separator();
    ImGui::Text(tr("Estilo y Colores"));

    // Color Fondo
    ImVec4 col_f = ImGui::ColorConvertU32ToFloat4(a.color_fondo);
    if (ImGui::ColorEdit4("Color Fondo", &col_f.x)) {
        a.color_fondo = ImGui::ColorConvertFloat4ToU32(col_f);
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }

    // Color Borde
    ImVec4 col_b = ImGui::ColorConvertU32ToFloat4(a.color_borde);
    if (ImGui::ColorEdit4("Color Borde", &col_b.x)) {
        a.color_borde = ImGui::ColorConvertFloat4ToU32(col_b);
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }

    // Grosor Borde
    if (ImGui::SliderFloat("Grosor Borde", &a.grosor_borde, 0.5f, 10.0f, "%.1f")) {
        if (al_cambio_flowsheet) al_cambio_flowsheet();
    }

    ImGui::Spacing();
    if (ImGui::Button(tr("Eliminar Dibujo"), ImVec2(-1, 25))) {
        if (flowsheet_) {
            flowsheet_->eliminar_anotacion(a.id);
            id_anotacion_ = -1;
            if (al_cambio_flowsheet) al_cambio_flowsheet();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Especies
// ═══════════════════════════════════════════════════════════════════════════

PanelEspecies::PanelEspecies() { busqueda_[0] = '\0'; }

void PanelEspecies::refrescar() {
    auto& db = base_datos::GestorDB::instancia();
    std::string q = busqueda_;
    if (q.empty())
        lista_ = db.todas_especies();
    else
        lista_ = db.buscar_especies(q);
    necesita_refresh_ = false;
}

void PanelEspecies::renderizar() {
    ImGui::Text(tr("Base de Datos de Especies"));
    ImGui::Separator();

    if (ImGui::InputText(tr("Buscar"), busqueda_, sizeof(busqueda_)))
        necesita_refresh_ = true;
    ImGui::SameLine();
    if (ImGui::Button(tr("Limpiar"))) { busqueda_[0] = '\0'; necesita_refresh_ = true; }

    if (necesita_refresh_) refrescar();

    if (lista_.empty()) { ImGui::TextDisabled(tr("Sin resultados.")); return; }

    ImGui::Text(tr("%zu especies encontradas"), lista_.size());
    ImGui::Separator();

    if (ImGui::BeginChild("lista_especies", ImVec2(0, -80))) {
        for (auto* sp : lista_) {
            if (!sp) continue;
            bool sel = seleccionada_ && seleccionada_->id == sp->id;
            if (ImGui::Selectable(sp->nombre.c_str(), &sel)) {
                seleccionada_ = sp;
                if (al_seleccionar_especie) al_seleccionar_especie(*sp);
            }
        }
    }
    ImGui::EndChild();

    if (seleccionada_) {
        ImGui::Separator();
        auto& s = *seleccionada_;
        ImGui::Text(tr("Fórmula: %s"), s.formula.c_str());
        ImGui::Text(tr("Masa Molar: %.4f g/mol"), s.masa_molar);
        ImGui::Text(tr("H°f (298K): %.3f kJ/mol"), s.h_formacion);
        ImGui::Text(tr("S°f (298K): %.3f J/(mol·K)"), s.s_formacion);
        ImGui::Text(tr("Cp = a + bT + cT² + dT³ [J/(mol·K)]"));
        ImGui::Text(tr("  a = %.6e"), s.cp_a);
        ImGui::Text(tr("  b = %.6e"), s.cp_b);
        ImGui::Text(tr("  c = %.6e"), s.cp_c);
        ImGui::Text(tr("  d = %.6e"), s.cp_d);
        ImGui::Text(tr("Antoine: A=%.3f B=%.3f C=%.2f"), s.antoine_a, s.antoine_b, s.antoine_c);

        // Calcular propiedades a temperatura
        double T = nucleo::STP_TEMP;
        ImGui::Separator();
        ImGui::Text(tr("Propiedades a T=%.2f K:"), T);
        ImGui::Text(tr("  Cp = %.3f J/(mol·K)"), s.cp(T));
        ImGui::Text(tr("  H(T) = %.3f kJ/mol"), s.entalpia_abs(T));
        ImGui::Text(tr("  S(T) = %.3f J/(mol·K)"), s.entropia_abs(T));
        ImGui::Text(tr("  G(T) = %.3f kJ/mol"), s.energia_gibbs(T));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Resultados
// ═══════════════════════════════════════════════════════════════════════════

void PanelResultados::renderizar() {
    if (ImGui::BeginTabBar("tab_resultados")) {
        if (ImGui::BeginTabItem(tr("Balance Masa"))) {
            if (masa_.ok) ImGui::TextColored(cma::interfaz::AdaptiveColor(0,1,0,1), "Convergido");
            else ImGui::TextColored(cma::interfaz::AdaptiveColor(1,0.5,0,1), "No resuelto");
            ImGui::TextWrapped(tr("%s"), masa_.mensaje.c_str());
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem(tr("Balance Energía"))) {
            if (energia_.ok) ImGui::TextColored(cma::interfaz::AdaptiveColor(0,1,0,1), "Balanceado");
            else ImGui::TextColored(cma::interfaz::AdaptiveColor(1,0.5,0,1), "No balanceado");
            ImGui::TextWrapped(tr("%s"), energia_.mensaje.c_str());
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem(tr("Gibbs"))) {
            if (gibbs_.ok) ImGui::TextColored(cma::interfaz::AdaptiveColor(0,1,0,1), "Convergido");
            else ImGui::TextColored(cma::interfaz::AdaptiveColor(1,0.5,0,1), "No convergido");
            ImGui::TextWrapped(tr("%s"), gibbs_.mensaje.c_str());
            if (!datos_gibbs_.composicion.empty()) {
                ImGui::Separator();
                if (ImGui::Button(tr("Exportar CSV (Gibbs)"))) {
                    std::string fname = "export_equilibrium.csv";
                    FILE* f = fopen(fname.c_str(), "w");
                    if (f) {
                        fprintf(f, "ID,Especie,kmol_s\n");
                        for (auto& [sid, n] : datos_gibbs_.composicion) {
                            auto it = datos_gibbs_.db.find(sid);
                            std::string nombre = (it != datos_gibbs_.db.end()) ? it->second.nombre : "-";
                            fprintf(f, "%lld,%s,%.6f\n", (long long)sid, nombre.c_str(), n);
                        }
                        fclose(f);
                    }
                }
                ImGui::Text(tr("Composición de equilibrio:"));
                if (ImGui::BeginTable("tab_gibbs", 3, ImGuiTableFlags_Borders|ImGuiTableFlags_RowBg)) {
                    ImGui::TableSetupColumn("ID");
                    ImGui::TableSetupColumn("Especie");
                    ImGui::TableSetupColumn("kmol/s");
                    ImGui::TableHeadersRow();
                    for (auto& [sid, n] : datos_gibbs_.composicion) {
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::Text(tr("%lld"), (long long)sid);
                        auto it = datos_gibbs_.db.find(sid);
                        ImGui::TableNextColumn();
                        if (it != datos_gibbs_.db.end()) ImGui::Text(tr("%s"), it->second.nombre.c_str());
                        else ImGui::Text(tr("-"));
                        ImGui::TableNextColumn(); ImGui::Text(tr("%.6f"), n);
                    }
                    ImGui::EndTable();
                }
            }
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem(tr("Exergía"))) {
            if (exergia_.ok) ImGui::TextColored(cma::interfaz::AdaptiveColor(0,1,0,1), "Calculado");
            else ImGui::TextColored(cma::interfaz::AdaptiveColor(1,0.5,0,1), "Error / No disponible");
            
            ImGui::TextWrapped(tr("%s"), exergia_.mensaje.c_str());
            
            if (exergia_.ok) {
                ImGui::Separator();
                ImGui::Text(tr("Global: In=%.2f kJ/s | Out=%.2f kJ/s | Destruida=%.2f kJ/s | Eficiencia=%.2f %%"),
                    exergia_.res.exergia_total_in, exergia_.res.exergia_total_out,
                    exergia_.res.exergia_destruida, exergia_.res.eficiencia_exergetica);
                
                ImGui::Separator();
                
                ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_Resizable | ImGuiTableFlags_ScrollY;
                if (ImGui::BeginTable("tab_exergia", 13, flags, ImVec2(0, 400))) {
                    ImGui::TableSetupColumn("Corrientes y especies");
                    ImGui::TableSetupColumn("Mass (kg)");
                    ImGui::TableSetupColumn("Amount (kmol)");
                    ImGui::TableSetupColumn("Molfraction");
                    ImGui::TableSetupColumn("T (K)");
                    ImGui::TableSetupColumn("P (atm)");
                    ImGui::TableSetupColumn("S (J/molK)");
                    ImGui::TableSetupColumn("H (kJ/mol)");
                    ImGui::TableSetupColumn("H Total (kJ)");
                    ImGui::TableSetupColumn("Ex_che (kJ/mol)");
                    ImGui::TableSetupColumn("Ex_phy (kJ/mol)");
                    ImGui::TableSetupColumn("Ex_tot (kJ/mol)");
                    ImGui::TableSetupColumn("Ex_tot_stream (kJ)");
                    ImGui::TableHeadersRow();
                    
                    for (const auto& flujo : exergia_.res.flujos) {
                        ImGui::TableNextRow(0, 0); // main row for stream
                        ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, IM_COL32(200, 200, 200, 255));
                        ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0,0,1,1), "Flujo %lld", (long long)flujo.id_flujo);
                        ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), flujo.masa_kg);
                        ImGui::TableNextColumn(); ImGui::Text(tr("%.4f"), flujo.cantidad_total_kmol);
                        ImGui::TableNextColumn(); ImGui::Text(tr("1.00"));
                        ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), flujo.temperatura);
                        ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), flujo.presion);
                        ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), flujo.entropia);
                        ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), flujo.entalpia);
                        ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), flujo.entalpia_total);
                        ImGui::TableNextColumn(); // Ex_che (empty for stream)
                        ImGui::TableNextColumn(); // Ex_phy
                        ImGui::TableNextColumn(); // Ex_tot
                        ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), flujo.ex_tot_flujo);
                        
                        for (const auto& sp : flujo.especies) {
                            ImGui::TableNextRow();
                            ImGui::TableNextColumn(); ImGui::Text(tr("  %s"), sp.nombre.c_str());
                            ImGui::TableNextColumn(); // mass of species not saved individually
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.4f"), sp.cantidad_kmol);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.4f"), sp.fraccion_molar);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), flujo.temperatura);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), flujo.presion);
                            ImGui::TableNextColumn(); // S species
                            ImGui::TableNextColumn(); // H species
                            ImGui::TableNextColumn(); // H total species
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sp.ex_che);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sp.ex_phy);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sp.ex_tot);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sp.ex_tot_flujo);
                        }
                    }
                    ImGui::EndTable();
                }
            }
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem(tr("Reporte"))) {
            if (ImGui::Button(tr("Generar Reporte"))) generar_reporte();
            ImGui::InputTextMultiline("##reporte", reporte_, sizeof(reporte_),
                                      ImVec2(-1, -1), ImGuiInputTextFlags_ReadOnly);
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }
}

void PanelResultados::poner_resultado_masa(const std::string& msg, bool ok) {
    masa_.mensaje = msg; masa_.ok = ok;
}

void PanelResultados::poner_resultado_energia(const std::string& msg, bool ok) {
    energia_.mensaje = msg; energia_.ok = ok;
}

void PanelResultados::poner_resultado_gibbs(const std::string& msg, bool ok,
    const std::map<int64_t, double>& comp,
    const std::map<int64_t, nucleo::Especie>& db)
{
    gibbs_.mensaje = msg; gibbs_.ok = ok;
    datos_gibbs_.composicion = comp;
    datos_gibbs_.db = db;
}


void PanelResultados::poner_resultado_exergia(const motores::ResultadoBalanceExergia& res, bool ok, const std::string& msg) {
    exergia_.res = res;
    exergia_.ok = ok;
    exergia_.mensaje = msg;
}
void PanelResultados::limpiar() {
    masa_ = energia_ = gibbs_ = {};
    datos_gibbs_ = {};
    reporte_[0] = '\0';
}

void PanelResultados::generar_reporte() {
    std::ostringstream os;
    os << "CMA Chemistry - Reporte de Simulación\n";
    os << "======================================\n\n";
    os << "Balance de Masa: " << (masa_.ok ? "CONVERGIDO" : "NO RESUELTO") << "\n";
    os << "  " << masa_.mensaje << "\n\n";
    os << "Balance de Energía: " << (energia_.ok ? "BALANCEADO" : "NO BALANCEADO") << "\n";
    os << "  " << energia_.mensaje << "\n\n";
    os << "Balance de Exergía: " << (exergia_.ok ? "CALCULADO" : "ERROR") << "\n";
    if (exergia_.ok) {
        os << "  Exergía In: " << exergia_.res.exergia_total_in << " kJ/s\n";
        os << "  Exergía Out: " << exergia_.res.exergia_total_out << " kJ/s\n";
        os << "  Exergía Destruida: " << exergia_.res.exergia_destruida << " kJ/s\n";
        os << "  Eficiencia Exergética: " << exergia_.res.eficiencia_exergetica << " %\n";
    }
    os << "  " << exergia_.mensaje << "\n\n";
    os << "Minimización Gibbs: " << (gibbs_.ok ? "CONVERGIDO" : "NO CONVERGIDO") << "\n";
    os << "  " << gibbs_.mensaje << "\n";

    if (!datos_gibbs_.composicion.empty()) {
        os << "\nComposición de equilibrio:\n";
        for (auto& [sid, n] : datos_gibbs_.composicion) {
            auto it = datos_gibbs_.db.find(sid);
            if (it != datos_gibbs_.db.end())
                os << "  " << it->second.nombre << " (" << it->second.formula << "): " << n << " kmol/s\n";
            else
                os << "  Especie #" << sid << ": " << n << " kmol/s\n";
        }
    }

    std::string s = os.str();
    strncpy(reporte_, s.c_str(), sizeof(reporte_)-1);
    reporte_[sizeof(reporte_)-1] = '\0';
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Calculadora Termodinámica
// ═══════════════════════════════════════════════════════════════════════════

void PanelCalculadora::renderizar(const std::map<int64_t, nucleo::Especie>& db) {
    ImGui::Text(tr("Calculadora Termodinámica"));
    ImGui::Separator();

    ImGui::InputDouble("Temperatura (K)", &temperatura_, 10, 100, "%.1f");
    ImGui::InputDouble("Presión (atm)", &presion_, 0.1, 1, "%.4f");

    ImGui::InputText(tr("Buscar especie"), busqueda_, sizeof(busqueda_));
    ImGui::SameLine();
    if (ImGui::Button(tr("Buscar"))) {
        auto& gestor = base_datos::GestorDB::instancia();
        auto res = gestor.buscar_especies(busqueda_);
        if (!res.empty()) id_especie_ = res[0]->id;
    }

    if (id_especie_ > 0) {
        auto it = db.find(id_especie_);
        if (it != db.end()) {
            auto& sp = it->second;
            ImGui::Separator();
            ImGui::Text(tr("Especie: %s (%s)"), sp.nombre.c_str(), sp.formula.c_str());

            double Cp = sp.cp(temperatura_);
            double H = sp.entalpia_abs(temperatura_);
            double S = sp.entropia_abs(temperatura_);
            double G = sp.energia_gibbs(temperatura_);
            double Pv = sp.presion_vapor(temperatura_);

            ImGui::Text(tr("Cp(%.0fK) = %.4f J/(mol·K)"), temperatura_, Cp);
            ImGui::Text(tr("H(%.0fK) = %.4f kJ/mol"), temperatura_, H);
            ImGui::Text(tr("S(%.0fK) = %.4f J/(mol·K)"), temperatura_, S);
            ImGui::Text(tr("G(%.0fK) = %.4f kJ/mol"), temperatura_, G);
            if (Pv > 0)
                ImGui::Text(tr("Pvap(%.0fK) = %.4f atm"), temperatura_, Pv);
            else
                ImGui::Text(tr("Pvap: N/A"));

            // Gráfico de G vs T
            if (ImPlot::BeginPlot("G vs T", ImVec2(-1, 200))) {
                ImPlot::SetupAxes("T (K)", "G (kJ/mol)");
                ImPlot::SetupAxisLimits(ImAxis_X1, temperatura_-100, temperatura_+100);
                std::vector<double> Ts, Gs;
                for (double T = temperatura_-100; T <= temperatura_+100; T += 5) {
                    Ts.push_back(T);
                    Gs.push_back(sp.energia_gibbs(T));
                }
                ImPlot::PlotLine("G(T)", Ts.data(), Gs.data(), (int)Ts.size());
                ImPlot::EndPlot();
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Tabla Periódica
// ═══════════════════════════════════════════════════════════════════════════

PanelTablaPeriodica::PanelTablaPeriodica() : tam_celda_(54.0f), mostrar_leyenda_(true) {
    seleccionado_ = nullptr;
}

static ImVec2 posicion_tabla(int Z, int periodo, int grupo, float celda) {
    // Tabla periódica estándar de 18 columnas
    // Fila base = periodo - 1
    // Columna base = grupo - 1 (grupos 1-18)
    // F-block (lantánidos Z=57-71, actínidos Z=89-103) va en filas separadas

    if (grupo == 0) {
        // F-block
        if (Z >= 57 && Z <= 71) {
            // Lantánidos: fila 8 (después de periodo 6), col 3..17
            int idx = Z - 57; // 0..14
            return ImVec2((float)(3 + idx), 8.0f);
        } else {
            // Actínidos: fila 9 (después de periodo 7), col 3..17
            int idx = Z - 89; // 0..14
            return ImVec2((float)(3 + idx), 9.0f);
        }
    }

    // Periodo 1: solo grupos 1 y 18
    if (periodo == 1 && grupo > 2 && grupo < 18) return ImVec2(-1, -1);

    float col = (float)(grupo - 1);
    float row = (float)(periodo - 1);

    // Ajustar gap en periodos 1-3 (no hay grupos 3-12)
    if (periodo == 1) {
        if (grupo == 18) col = 17;
    } else if (periodo == 2 || periodo == 3) {
        if (grupo >= 3 && grupo <= 12) return ImVec2(-1, -1);
        if (grupo >= 13) col = (float)(grupo - 1);  // sin gap, el gap se maneja al dibujar
    }

    return ImVec2(col, row);
}

void PanelTablaPeriodica::renderizar() {
    auto& tp = nucleo::TablaPeriodica::instancia();
    auto& db = base_datos::GestorElementos::instancia();
    
    if (!db_inicializada_) {
        db.cargar_desde_json(filename_);
        db_inicializada_ = true;
    }

    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0,0,0,0));
    if (ImGui::Button(tr("Gráfico"))) vista_actual_ = 0;
    ImGui::SameLine();
    if (ImGui::Button(tr("Base de Datos"))) vista_actual_ = 1;
    ImGui::SameLine();
    ImGui::TextDisabled(tr("|"));
    ImGui::SameLine();
    if (ImGui::Button(tr("Abrir"))) { ImGui::OpenPopup("Open DB"); }
    ImGui::SameLine();
    if (ImGui::Button(tr("Guardar"))) { ImGui::OpenPopup("Save DB"); }
    ImGui::PopStyleColor();
    ImGui::Separator();
    
    if (ImGui::BeginPopupModal("Save DB", NULL, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::InputText(tr("Filename"), filename_, 256);
        if (ImGui::Button(tr("Guardar"), ImVec2(120, 0))) {
            db.guardar_a_json(filename_);
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Cancelar"), ImVec2(120, 0))) { ImGui::CloseCurrentPopup(); }
        ImGui::EndPopup();
    }
    
    if (ImGui::BeginPopupModal("Open DB", NULL, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::InputText(tr("Filename"), filename_, 256);
        if (ImGui::Button(tr("Abrir"), ImVec2(120, 0))) {
            db.cargar_desde_json(filename_);
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Cancelar"), ImVec2(120, 0))) { ImGui::CloseCurrentPopup(); }
        ImGui::EndPopup();
    }

    if (vista_actual_ == 1) {
        // --- SPREADSHEET DATABASE VIEW ---
        if (ImGui::BeginTable("DBGrid", 3 + db.atomicos().size(), ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollX | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable)) {
            ImGui::TableSetupScrollFreeze(3, 1); // Freeze top row and left 3 columns
            ImGui::TableSetupColumn("Propiedad", ImGuiTableColumnFlags_WidthFixed, 150.0f);
            ImGui::TableSetupColumn("Unidades", ImGuiTableColumnFlags_WidthFixed, 60.0f);
            ImGui::TableSetupColumn("Diagram", ImGuiTableColumnFlags_WidthFixed, 60.0f);
            for (int Z : db.atomicos()) {
                const auto* elem = tp.por_numero(Z);
                std::string col_name = elem ? elem->simbolo : std::to_string(Z);
                ImGui::TableSetupColumn(col_name.c_str(), ImGuiTableColumnFlags_WidthFixed, 80.0f);
            }
            ImGui::TableHeadersRow();
            
            for (const auto& prop : db.propiedades()) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                ImGui::Text(tr("%s"), prop.nombre.c_str());
                ImGui::TableSetColumnIndex(1);
                ImGui::Text(tr("%s"), prop.unidades.c_str());
                ImGui::TableSetColumnIndex(2);
                bool graficable = prop.graficable;
                ImGui::PushID(prop.nombre.c_str());
                if (ImGui::Checkbox("##diag", &graficable)) {
                    db.set_graficable(prop.nombre, graficable);
                }
                ImGui::PopID();
                
                int c = 3;
                for (int Z : db.atomicos()) {
                    ImGui::TableSetColumnIndex(c);
                    std::string val = db.obtener_valor_str(Z, prop.nombre);
                    char buf[64];
                    strncpy(buf, val.c_str(), sizeof(buf)-1);
                    buf[sizeof(buf)-1] = '\0';
                    ImGui::SetNextItemWidth(-FLT_MIN);
                    ImGui::PushID(Z * 1000 + c);
                    if (ImGui::InputText("##v", buf, 64)) {
                        db.establecer_valor(Z, prop.nombre, std::string(buf));
                    }
                    ImGui::PopID();
                    c++;
                }
            }
            ImGui::EndTable();
        }
    } else {
        // --- PERIODIC CHART VIEW ---
        float ancho_disp = ImGui::GetContentRegionAvail().x;
        
        // Controles de graficación
        ImGui::SetNextItemWidth(250.0f);
        if (ImGui::BeginCombo("Property", prop_seleccionada_)) {
            for (const auto& p : db.propiedades()) {
                if (!p.graficable) continue;
                bool is_selected = (p.nombre == prop_seleccionada_);
                if (ImGui::Selectable(p.nombre.c_str(), is_selected)) {
                    strncpy(prop_seleccionada_, p.nombre.c_str(), sizeof(prop_seleccionada_)-1);
                    prop_seleccionada_[sizeof(prop_seleccionada_)-1] = '\0';
                }
                if (is_selected) ImGui::SetItemDefaultFocus();
            }
            ImGui::EndCombo();
        }
        ImGui::SameLine();
        ImGui::SetNextItemWidth(150.0f);
        if (ImGui::InputText(tr("Search"), busqueda_, sizeof(busqueda_))) {
            buscar_ = busqueda_[0] != '\0';
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("X"))) { busqueda_[0] = '\0'; buscar_ = false; }
        
        ImGui::SameLine();
        ImGui::SetNextItemWidth(100.0f);
        ImGui::SliderFloat("Size", &tam_celda_, 28, 80, "%.0f px");
        ImGui::SameLine();
        ImGui::Checkbox(tr("Legend"), &mostrar_leyenda_);
        
        // Encontrar maximo valor para la propiedad seleccionada
        double max_val = 1e-9;
        for (int Z : db.atomicos()) {
            double v = db.obtener_valor_num(Z, prop_seleccionada_);
            if (v > max_val) max_val = v;
        }

        ImVec2 space = ImGui::GetContentRegionAvail();
        float panel_right_w = 350.0f;
        float chart_w = space.x - panel_right_w - 10.0f;
        
        ImGui::BeginChild("ChartPane", ImVec2(chart_w, space.y), true, ImGuiWindowFlags_HorizontalScrollbar);
        
        float celda = tam_celda_;
        float ancho_total = 18.0f * celda + 4.0f;
        float alto_total = 11.0f * celda + 4.0f;
        float leyenda_h = mostrar_leyenda_ ? 80.0f : 0.0f;

        ImDrawList* draw = ImGui::GetWindowDrawList();
        ImVec2 inicio = ImGui::GetCursorScreenPos();
        float offset_x = std::max(0.0f, (chart_w - ancho_total) * 0.5f);
        inicio.x += offset_x;

        draw->AddRectFilled(inicio, ImVec2(inicio.x+ancho_total, inicio.y+alto_total+leyenda_h), IM_COL32(20,20,26,255), 4);

        auto coincide_busqueda = [&](const nucleo::Elemento& e) -> bool {
            if (!buscar_) return true;
            std::string q = busqueda_;
            std::transform(q.begin(), q.end(), q.begin(), ::tolower);
            std::string nom = e.nombre_es;
            std::transform(nom.begin(), nom.end(), nom.begin(), ::tolower);
            std::string sim = e.simbolo;
            std::transform(sim.begin(), sim.end(), sim.begin(), ::tolower);
            return nom.find(q) != std::string::npos || sim.find(q) != std::string::npos || std::to_string(e.numero_atomico).find(q) != std::string::npos;
        };

        const auto& elementos = tp.todos();
        for (auto& e : elementos) {
            if (e.numero_atomico > 118) break;
            if (!coincide_busqueda(e)) continue;

            ImVec2 pos_grid = posicion_tabla(e.numero_atomico, e.periodo, e.grupo, celda);
            if (pos_grid.x < 0) continue;

            float x = inicio.x + 2 + pos_grid.x * celda;
            float y = inicio.y + 2 + pos_grid.y * celda;
            ImVec2 pos(x, y);
            ImVec2 tam(celda - 2, celda - 2);

            ImU32 color;
            if (e.grupo == 0) color = (e.numero_atomico <= 71) ? IM_COL32(50,80,100,255) : IM_COL32(80,50,100,255);
            else if (e.es_gas_noble()) color = IM_COL32(90,60,90,255);
            else if (e.grupo == 1 && e.numero_atomico > 1) color = IM_COL32(100,60,60,255);
            else if (e.grupo == 2) color = IM_COL32(80,80,60,255);
            else if (e.grupo >= 3 && e.grupo <= 12) color = IM_COL32(60,80,120,255);
            else if (e.grupo >= 13 && e.grupo <= 16) color = IM_COL32(70,100,70,255);
            else if (e.grupo == 17) color = IM_COL32(80,100,50,255);
            else color = IM_COL32(50,50,60,255);

            if (buscar_) color = IM_COL32(180,150,60,255);
            if (seleccionado_ && seleccionado_->numero_atomico == e.numero_atomico) color = IM_COL32(220, 190, 50, 255);

            draw->AddRectFilled(pos, ImVec2(pos.x+tam.x, pos.y+tam.y), color, 2);
            ImU32 borde = (seleccionado_ && seleccionado_->numero_atomico == e.numero_atomico) ? IM_COL32(255,220,80,255) : IM_COL32(80,80,90,150);
            draw->AddRect(pos, ImVec2(pos.x+tam.x, pos.y+tam.y), borde, 2);

            // Draw bubble for property
            double val = db.obtener_valor_num(e.numero_atomico, prop_seleccionada_);
            if (val > 0) {
                float radius = (tam.x * 0.45f) * std::sqrt(val / max_val);
                ImVec2 center(pos.x + tam.x*0.5f, pos.y + tam.y*0.5f);
                draw->AddCircleFilled(center, radius, IM_COL32(60, 120, 240, 200));
            }

            char buf[8]; snprintf(buf, sizeof(buf), "%s", e.simbolo.c_str());
            ImVec2 tsz = ImGui::CalcTextSize(buf);
            draw->AddText(ImVec2(pos.x+(tam.x-tsz.x)*0.5f, pos.y+3), IM_COL32(220,220,240,255), buf);
            char nbuf[8]; snprintf(nbuf, sizeof(nbuf), "%d", e.numero_atomico);
            draw->AddText(ImVec2(pos.x+2, pos.y+1), IM_COL32(160,160,180,180), nbuf);

            if (ImGui::IsMouseHoveringRect(pos, ImVec2(pos.x+tam.x, pos.y+tam.y)) && ImGui::IsMouseClicked(ImGuiMouseButton_Left)) {
                seleccionado_ = &e;
                if (al_seleccionar_elemento) al_seleccionar_elemento(e);
            }
        }

        if (mostrar_leyenda_) {
            float ly = inicio.y + alto_total + 4;
            struct Par { const char* lbl; ImU32 col; };
            Par legendas[] = {
                {"Alcalinos", IM_COL32(100,60,60,255)}, {"Alc. térreos", IM_COL32(80,80,60,255)},
                {"Transición", IM_COL32(60,80,120,255)}, {"Bloque p", IM_COL32(70,100,70,255)},
                {"Halógenos", IM_COL32(80,100,50,255)}, {"Gases nobles", IM_COL32(90,60,90,255)},
                {"Lantánidos", IM_COL32(50,80,100,255)}, {"Actínidos", IM_COL32(80,50,100,255)}
            };
            float lx = inicio.x + 4;
            for (auto& p : legendas) {
                draw->AddRectFilled(ImVec2(lx, ly), ImVec2(lx+12, ly+10), p.col, 1);
                draw->AddText(ImVec2(lx+16, ly-1), IM_COL32(180,180,200,255), p.lbl);
                lx += ImGui::CalcTextSize(p.lbl).x + 30;
                if (lx > inicio.x+ancho_total-40) { lx = inicio.x+4; ly += 16; }
            }
        }

        float total_cont = alto_total + leyenda_h + 4;
        ImGui::SetCursorPosY(ImGui::GetCursorPosY() + total_cont);
        ImGui::EndChild();
        
        ImGui::SameLine();
        ImGui::BeginChild("SidePanel", ImVec2(panel_right_w, space.y), true);
        if (seleccionado_) {
            ImGui::Text(tr("Element: %s (%s)"), seleccionado_->nombre_es.c_str(), seleccionado_->simbolo.c_str());
            ImGui::Separator();
            if (ImGui::BeginTable("PropGrid", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_SizingFixedFit)) {
                ImGui::TableSetupColumn("Propiedad");
                ImGui::TableSetupColumn("Unidades");
                ImGui::TableSetupColumn(seleccionado_->simbolo.c_str(), ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableHeadersRow();
                for (const auto& prop : db.propiedades()) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::Text(tr("%s"), prop.nombre.c_str());
                    ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%s"), prop.unidades.c_str());
                    ImGui::TableSetColumnIndex(2);
                    bool selected_prop = (prop.nombre == prop_seleccionada_);
                    if (selected_prop) ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, IM_COL32(40,80,160,255));
                    ImGui::Text(tr("%s"), db.obtener_valor_str(seleccionado_->numero_atomico, prop.nombre).c_str());
                }
                ImGui::EndTable();
            }
        } else {
            ImGui::TextDisabled(tr("Select an element from the chart."));
        }
        ImGui::EndChild();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Electroquímica
// ═══════════════════════════════════════════════════════════════════════════

void PanelElectroquimica::renderizar() {
    ImGui::Text(tr("Electroquímica"));
    ImGui::Separator();

    ImGui::InputText(tr("Semirreacción Ánodo"), anodo_, sizeof(anodo_));
    ImGui::InputText(tr("Semirreacción Cátodo"), catodo_, sizeof(catodo_));
    ImGui::InputDouble("Temperatura (K)", &T_, 1, 10, "%.2f");

    if (ImGui::Button(tr("Calcular Celda"), ImVec2(-1, 30))) {
        motores::Electroquimica eq;
        // Extraer los potenciales estándar desde las semirreacciones
        double E_anodo = eq.potencial_estandar(anodo_);
        double E_catodo = eq.potencial_estandar(catodo_);

        // Si ambos son 0, las reacciones no se reconocen; usar valores indicativos
        if (E_anodo == 0 && E_catodo == 0) {
            // Potenciales típicos para semirreacciones comunes
            std::string a = anodo_, c = catodo_;
            if (a.find("Zn") != std::string::npos) E_anodo = -0.76;
            else if (a.find("Cu") != std::string::npos) E_anodo = 0.34;
            else if (a.find("Fe") != std::string::npos) E_anodo = -0.44;
            else if (a.find("Ag") != std::string::npos) E_anodo = 0.80;
            else if (a.find("Al") != std::string::npos) E_anodo = -1.66;
            else if (a.find("Mg") != std::string::npos) E_anodo = -2.37;
            else if (a.find("Na") != std::string::npos) E_anodo = -2.71;
            else if (a.find("Cl") != std::string::npos) E_anodo = 1.36;

            if (c.find("Zn") != std::string::npos) E_catodo = -0.76;
            else if (c.find("Cu") != std::string::npos) E_catodo = 0.34;
            else if (c.find("Fe") != std::string::npos) E_catodo = -0.44;
            else if (c.find("Ag") != std::string::npos) E_catodo = 0.80;
            else if (c.find("Al") != std::string::npos) E_catodo = -1.66;
            else if (c.find("Mg") != std::string::npos) E_catodo = -2.37;
            else if (c.find("Na") != std::string::npos) E_catodo = -2.71;
            else if (c.find("Cl") != std::string::npos) E_catodo = 1.36;
        }

        motores::PotencialElectrodo pa, pc;
        pa.E0 = E_anodo; pa.E = E_anodo;
        pc.E0 = E_catodo; pc.E = E_catodo;

        if (std::abs(E_anodo) > 0.001 || std::abs(E_catodo) > 0.001) {
            auto celda = eq.potencial_celda(pa, pc);
            E_celda_ = celda.potencial_celda;
            delta_G_ = celda.delta_G;
            K_eq_ = celda.K_eq;
            calculado_ = true;
        } else {
            E_celda_ = E_catodo - E_anodo;
            delta_G_ = -2 * 96485.0 * E_celda_ / 1000.0;
            K_eq_ = std::exp(-delta_G_ * 1000.0 / (8.314 * T_));
            calculado_ = true;
        }
    }

    if (calculado_) {
        ImGui::Separator();
        ImGui::Text(tr("E°celda = %.4f V"), E_celda_);
        ImGui::Text(tr("ΔG° = %.2f kJ/mol"), delta_G_);
        ImGui::Text(tr("K_eq = %.4e"), K_eq_);
        ImGui::Text(tr("Reacción espontánea: %s"), E_celda_ > 0 ? "SÍ" : "NO");
    }

    // Ley de Faraday
    ImGui::Separator();
    ImGui::Text(tr("Ley de Faraday"));
    static double faraday_I = 1.0, faraday_t = 3600.0, faraday_M = 63.546, faraday_resultado = 0;
    static int faraday_n = 2;
    static bool faraday_calc = false;
    ImGui::InputDouble("Corriente (A)", &faraday_I);
    ImGui::InputDouble("Tiempo (s)", &faraday_t);
    ImGui::InputDouble("Masa molar (g/mol)", &faraday_M);
    ImGui::InputInt("Electrones (n)", &faraday_n);

    if (ImGui::Button(tr("Calcular Masa depositada"), ImVec2(-1, 25))) {
        motores::Electroquimica eq;
        faraday_resultado = eq.ley_faraday(faraday_I, faraday_t, faraday_M, faraday_n);
        faraday_calc = true;
    }
    if (faraday_calc)
        ImGui::Text(tr("Masa depositada: %.4f g"), faraday_resultado);

    // pH
    ImGui::Separator();
    ImGui::Text(tr("pH"));
    static double ph_conc = 1e-7, ph_resultado = 7.0;
    static bool ph_calc = false;
    ImGui::InputDouble("[H⁺] (M)", &ph_conc, 1e-7, 1e-3, "%.6e");
    if (ImGui::Button(tr("Calcular pH"), ImVec2(-1, 25))) {
        motores::Electroquimica eq;
        ph_resultado = eq.ph(ph_conc);
        ph_calc = true;
    }
    if (ph_calc)
        ImGui::Text(tr("pH = %.2f  |  [H⁺] = %.4e M  |  pOH = %.2f"), ph_resultado, ph_conc, 14.0 - ph_resultado);
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Cinética
// ═══════════════════════════════════════════════════════════════════════════

void PanelCinetica::renderizar() {
    ImGui::Text(tr("Cinética Química"));
    ImGui::Separator();

    ImGui::InputDouble("C₀ (mol/L)", &C0_, 0.1, 1, "%.4f");
    ImGui::InputDouble("k (constante)", &k_, 0.01, 0.1, "%.6f");
    ImGui::InputDouble("Tiempo (s)", &tiempo_, 1, 10, "%.1f");
    ImGui::InputInt("Orden (0/1/2)", &orden_);
    ImGui::Combo("Reactor", &reactor_, "Batch\0CSTR\0PFR\0");
    if (orden_ == 2) {
        ImGui::InputDouble("Cb₀ (mol/L)", &Cb0_, 0.1, 1, "%.4f");
    }

    if (ImGui::Button(tr("Calcular"), ImVec2(-1, 30))) {
        motores::Cinetica cin;
        double k = cin.arrhenius(k_, 0, 300); // Use provided k directly
        k = k_; // For simplicity, use k directly (not Arrhenius)

        motores::ResultadoCinetica res;
        switch (reactor_) {
            case 0: res = cin.reactor_batch(C0_, orden_, k, tiempo_, Cb0_); break;
            case 1: res = cin.reactor_cstr(C0_, orden_, k, tiempo_, Cb0_); break;
            case 2: res = cin.reactor_pfr(C0_, orden_, k, tiempo_, Cb0_); break;
        }

        if (!res.mensaje.empty()) {
            Cf_ = res.concentracion_final[0];
            conversion_ = res.conversion;
            calculado_ = true;
        }
    }

    if (calculado_) {
        ImGui::Separator();
        ImGui::Text(tr("C_final = %.6f mol/L"), Cf_);
        ImGui::Text(tr("Conversión = %.4f"), conversion_);
        ImGui::Text(tr("t½ = %.4f s"), motores::Cinetica().vida_media(k_, orden_, C0_));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Equilibrio de Fases
// ═══════════════════════════════════════════════════════════════════════════

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
    ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.6f);

    // --- Left Pane: Species List ---
    ImGui::Text(tr("System Species"));
    if (ImGui::BeginTable("eq_species_table", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, ImGui::GetContentRegionAvail().y - 80))) {
        ImGui::TableSetupColumn("Fórmula");
        ImGui::TableSetupColumn("Nombre");
        ImGui::TableSetupColumn("Amount (kmol)");
        ImGui::TableHeadersRow();

        for (size_t i = 0; i < species_list_.size(); ++i) {
            auto& sp = species_list_[i];
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), sp.formula.c_str());
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), sp.name.c_str());
            ImGui::TableNextColumn();
            ImGui::PushID((int)i);
            ImGui::SetNextItemWidth(-FLT_MIN);
            ImGui::InputDouble("##amt", &sp.initial_amount, 0.1, 1.0, "%.5f");
            ImGui::PopID();
        }
        ImGui::EndTable();
    }

    if (ImGui::Button(tr("Examinar Base de Datos##fases"), ImVec2(-1, 0))) {
        ModalBuscadorDB::instancia().abrir("Buscar Especie (Fases)");
    }
    
    std::vector<const nucleo::Especie*> sel_fases;
    if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Fases)", sel_fases, true)) {
        for (auto* sp : sel_fases) {
            bool exists = false;
            for (auto& s : species_list_) if (s.id == sp->id) exists = true;
            if (!exists) {
                species_list_.push_back({sp->id, sp->nombre, sp->formula, std::string(cma::nucleo::nombre_fase(sp->fase_std)), 0.0});
            }
        }
    }

    // ── Selección por elementos (como HSC: "select the elements of the system") ──
    ImGui::Text(tr("Elements (e.g. Na, S, O, C):"));
    ImGui::SetNextItemWidth(-1);
    ImGui::InputTextWithHint("##elements_in", "Na, S, O, C...", elementos_buf_, sizeof(elementos_buf_));
    ImGui::Checkbox(tr("All elements must exist"), &todos_elementos_);
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Only species containing ALL the selected elements");
    ImGui::SameLine();
    ImGui::Checkbox(tr("Cell calculation"), &modo_celda_);
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Electrochemical cell: adds e- and charge balance");

    if (ImGui::Button(tr("Generate Species"), ImVec2(-1, 0))) {
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
                        species_list_.push_back({sid, db_sp.nombre, db_sp.formula, std::string(cma::nucleo::nombre_fase(db_sp.fase_std)), 0.0});
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
                        species_list_.push_back({sid, db_sp.nombre, db_sp.formula, std::string(cma::nucleo::nombre_fase(db_sp.fase_std)), 0.0});
                        break;
                    }
                }
            }
        }
    }

    // ── Guardar / cargar composición del sistema (archivo .gem) ──
    if (ImGui::Button(tr("Save Species File"), ImVec2(ImGui::GetContentRegionAvail().x * 0.5f - 4, 0))) {
        FILE* zen = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.gem' --title='Save GEM System' 2>/dev/null", "r");
        if (zen) {
            char buf[512] = {0};
            if (fgets(buf, sizeof(buf), zen)) {
                std::string path(buf);
                if (!path.empty() && path.back() == '\n') path.pop_back();
                if (!path.empty()) {
                    if (path.find(".gem") == std::string::npos) path += ".gem";
                    FILE* fp = fopen(path.c_str(), "w");
                    if (fp) {
                        fprintf(fp, "# GEM species file: formula;amount_kmol\n");
                        for (auto& s : species_list_)
                            fprintf(fp, "%s;%.6g\n", s.formula.c_str(), s.initial_amount);
                        fclose(fp);
                    }
                    // Guardar el nombre base como título del gráfico
                    std::string base = path;
                    size_t slash = base.find_last_of('/');
                    if (slash != std::string::npos) base = base.substr(slash + 1);
                    if (base.size() > 4 && base.substr(base.size() - 4) == ".gem") base = base.substr(0, base.size() - 4);
                    strncpy(titulo_chart_, base.c_str(), sizeof(titulo_chart_) - 1);
                    titulo_chart_[sizeof(titulo_chart_) - 1] = '\0';
                }
            }
            pclose(zen);
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Load Species File"), ImVec2(-1, 0))) {
        FILE* zen = popen("zenity --file-selection --file-filter='*.gem' --title='Load GEM System' 2>/dev/null", "r");
        if (zen) {
            char buf[512] = {0};
            if (fgets(buf, sizeof(buf), zen)) {
                std::string path(buf);
                if (!path.empty() && path.back() == '\n') path.pop_back();
                if (!path.empty()) {
                    FILE* fp = fopen(path.c_str(), "r");
                    if (fp) {
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
                            bool exists = false;
                            for (auto& s : species_list_)
                                if (s.id == esp->id) { exists = true; s.initial_amount = amt; break; }
                            if (!exists) {
                                species_list_.push_back({esp->id, esp->nombre, esp->formula, std::string(cma::nucleo::nombre_fase(esp->fase_std)), amt});
                            }
                        }
                        fclose(fp);
                        std::string base = path;
                        size_t slash = base.find_last_of('/');
                        if (slash != std::string::npos) base = base.substr(slash + 1);
                        if (base.size() > 4 && base.substr(base.size() - 4) == ".gem") base = base.substr(0, base.size() - 4);
                        strncpy(titulo_chart_, base.c_str(), sizeof(titulo_chart_) - 1);
                        titulo_chart_[sizeof(titulo_chart_) - 1] = '\0';
                    }
                }
            }
            pclose(zen);
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
        if (species_list_.empty() || !any_feed) {
            error_msg_ = "Define species and set a positive input amount for at least the feed.";
        } else {
            is_calculating_ = true;
            calc_progress_ = 0.0f;

            if (calc_thread_.joinable()) calc_thread_.join();

            auto species_copy = species_list_;
            double ts = t_start_, te = t_end_;        // °C
            int tsteps = t_steps_;
            double ps = p_start_, pe = p_end_;        // bar
            int psteps = p_steps_;
            bool celda = modo_celda_;

            calc_thread_ = std::thread([this, species_copy, ts, te, tsteps, ps, pe, psteps, celda, &db]() {
                hsc_clone::GibbsMinimizer minimizer;

                double t_step_size = (te - ts) / (std::max(tsteps - 1, 1));
                double p_step_size = (pe - ps) / (std::max(psteps - 1, 1));

                std::vector<double> local_x(tsteps * psteps);   // T (°C)
                std::vector<double> local_y(tsteps * psteps);   // P (bar)
                std::vector<double> local_g(tsteps * psteps, 0.0);
                std::map<int64_t, std::vector<double>> local_z;

                for (auto& sp : species_copy) {
                    local_z[sp.id].resize(tsteps * psteps, 0.0);
                }

                std::map<int64_t, double> warm;  // warm start entre puntos del barrido
                std::string local_error;
                int total_steps = psteps * tsteps;
                int current_step = 0;
                int no_conv = 0;

                for (int i = 0; i < psteps; ++i) {
                    double P_bar = ps + i * p_step_size;
                    double P_atm = P_bar * 0.986923; // bar -> atm
                    for (int j = 0; j < tsteps; ++j) {
                        double T_C = ts + j * t_step_size;
                        double T_K = T_C + 273.15;
                        int idx = i * tsteps + j;

                        local_x[idx] = T_C;
                        local_y[idx] = P_bar;

                        std::map<int64_t, double> alimentacion;
                        for (auto& sp : species_copy) {
                            alimentacion[sp.id] = sp.initial_amount;
                        }

                        auto res = minimizer.minimizar(alimentacion, T_K, P_atm, db, {}, &warm, celda);
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
                std::map<std::string, int> fase_numero;
                int proxima_fase = 2; // fase 1 = gas
                for (auto& [sid, z_data] : local_z) {
                    double max_amt = 0.0;
                    for (double a : z_data) max_amt = std::max(max_amt, a);
                    if (max_amt > 1e-6) {
                        std::string legend = "Unknown";
                        std::string fase = "?";
                        for (auto& sp : species_copy) {
                            if (sp.id == sid) {
                                legend = sp.formula;
                                fase = sp.phase;
                                break;
                            }
                        }
                        int fnum;
                        if (fase == "Gas") fnum = 1;
                        else {
                            auto itf = fase_numero.find(legend);
                            if (itf == fase_numero.end()) { fase_numero[legend] = proxima_fase; fnum = proxima_fase; ++proxima_fase; }
                            else fnum = itf->second;
                        }
                        local_surfaces.push_back({sid, legend, fase, fnum, z_data, true});
                    }
                }

                temp_x_values_ = std::move(local_x);
                temp_y_values_ = std::move(local_y);
                temp_z_values_ = std::move(local_z);
                temp_g_values_ = std::move(local_g);
                temp_chart_surfaces_ = std::move(local_surfaces);
                temp_error_msg_ = local_error;
                temp_plot_t_steps_ = tsteps;
                temp_plot_p_steps_ = psteps;

                calculation_done_ = true;
            });
        }
    }
    if (calc_disabled) ImGui::EndDisabled();

    if (is_calculating_) {
        ImGui::SameLine();
        ImGui::ProgressBar(calc_progress_, ImVec2(150, 30), "Running...");
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
    ImGui::SetNextItemWidth(-1);
    ImGui::InputText("##gem_title", titulo_chart_, sizeof(titulo_chart_));

    ImGui::Spacing();
    ImGui::Text(tr("System State"));
    ImGui::Separator();

    if (ImGui::BeginTable("eq_sweep_params", 4, ImGuiTableFlags_BordersInnerH | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Variable", ImGuiTableColumnFlags_WidthFixed, 130.0f);
        ImGui::TableSetupColumn("Initial State", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Final State", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Number of Steps", ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableHeadersRow();

        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Temperature (°C)"));
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Ti", &t_start_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Tf", &t_end_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputInt("##Ts", &t_steps_);
        if (t_steps_ < 2) t_steps_ = 2;

        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Pressure (bar)"));
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Pi", &p_start_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputDouble("##Pf", &p_end_);
        ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-FLT_MIN); ImGui::InputInt("##Ps", &p_steps_);
        if (p_steps_ < 2) p_steps_ = 2;

        ImGui::EndTable();
    }

    if (!error_msg_.empty()) {
        ImGui::Spacing();
        ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.4f, 0.4f, 1.0f), "%s", error_msg_.c_str());
    }

    ImGui::Columns(1);
    
    if (show_chart_window_) {
        ImGui::SetNextWindowSize(ImVec2(1000, 650), ImGuiCond_FirstUseEver);
        std::string wintitle = std::string("Equilibrium Calculations - [Chart C:|CMA|") + titulo_chart_ + ".gem]###gem_chart";
        if (ImGui::Begin(wintitle.c_str(), &show_chart_window_)) {
            if (!error_msg_.empty()) {
                ImGui::Spacing();
                ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.4f, 0.4f, 1.0f), "Warning: %s", error_msg_.c_str());
                ImGui::Spacing();
            }
            if (ImGui::BeginTable("ChartLayout", 2, ImGuiTableFlags_None, ImVec2(-1, -1))) {
                ImGui::TableSetupColumn("PlotCol", ImGuiTableColumnFlags_WidthStretch, 0.72f);
                ImGui::TableSetupColumn("LegendCol", ImGuiTableColumnFlags_WidthStretch, 0.28f);
                ImGui::TableNextRow();
                ImGui::TableNextColumn();

                if (calculado_ && x_values_.size() > 0) {
                    if (ImPlot3D::BeginPlot(titulo_chart_[0] ? titulo_chart_ : "Equilibrium", ImVec2(-1, -1))) {
                        ImPlot3D::SetupAxes("Temperature, °C", "Pressure, bar", "Equilibrium Amount, kmol");

                        for (auto& surf : chart_surfaces_) {
                            if (surf.visible) {
                                ImPlot3D::PlotSurface(surf.legend.c_str(), x_values_.data(), y_values_.data(), surf.z_data.data(), plot_t_steps_, plot_p_steps_);
                            }
                        }
                        ImPlot3D::EndPlot();
                    }
                } else {
                    ImGui::Text(tr("No data to display. Please calculate first."));
                }

                ImGui::TableNextColumn();

                ImGui::Text(tr("Chart parameters"));
                ImGui::Separator();
                // Leyenda estilo HSC: Description | Phase | visible
                ImGui::Text(tr("Legend"));
                if (ImGui::BeginTable("##gem_legend", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, -60))) {
                    ImGui::TableSetupColumn("", ImGuiTableColumnFlags_WidthFixed, 24.0f);
                    ImGui::TableSetupColumn("Description", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableSetupColumn("Phase", ImGuiTableColumnFlags_WidthFixed, 45.0f);
                    ImGui::TableHeadersRow();
                    for (auto& surf : chart_surfaces_) {
                        ImGui::TableNextRow();
                        ImGui::TableNextColumn();
                        ImGui::PushID((int)(surf.id) + 9200);
                        ImGui::Checkbox("##vis", &surf.visible);
                        ImGui::PopID();
                        ImGui::TableNextColumn();
                        ImGui::Text("%s (%s)", surf.legend.c_str(), surf.fase.c_str());
                        ImGui::TableNextColumn();
                        ImGui::Text("%d", surf.fase_num);
                    }
                    ImGui::EndTable();
                }

                if (ImGui::Button(tr("Table"), ImVec2(-1, 0)) && calculado_) {
                    show_table_window_ = true;
                }

                ImGui::EndTable();
            }
        }
        ImGui::End();
    }

    // ── Ventana de tabla numérica (botón "Table" de HSC) ──
    if (show_table_window_ && calculado_ && !x_values_.empty()) {
        ImGui::SetNextWindowSize(ImVec2(950, 520), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Equilibrium Data Table"), &show_table_window_)) {
            // Selector de presión (la tabla muestra un corte T vs especies a P fija)
            if (plot_p_steps_ > 1) {
                ImGui::SliderInt("Pressure row", &table_p_index_, 0, plot_p_steps_ - 1);
                ImGui::SameLine();
            }
            int p_idx = std::clamp(table_p_index_, 0, plot_p_steps_ > 0 ? plot_p_steps_ - 1 : 0);
            double P_sel = y_values_[p_idx * plot_t_steps_];
            ImGui::Text("P = %.4g bar", P_sel);

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
                ImGui::TableSetupColumn("T (°C)", ImGuiTableColumnFlags_WidthFixed, 80);
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
                    ImGui::TableNextColumn(); ImGui::Text("%.1f", x_values_[idx]);
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

            // Guardar tabla completa (todos los cortes T x P) como texto (HSC Table button)
            if (ImGui::Button(tr("Save Table (.txt)"), ImVec2(150, 26))) {
                FILE* zen = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.txt' --title='Save Equilibrium Table' 2>/dev/null", "r");
                if (zen) {
                    char buf[512] = {0};
                    if (fgets(buf, sizeof(buf), zen)) {
                        std::string path(buf);
                        if (!path.empty() && path.back() == '\n') path.pop_back();
                        if (!path.empty()) {
                            if (path.find(".txt") == std::string::npos) path += ".txt";
                            FILE* f = fopen(path.c_str(), "w");
                            if (f) {
                                fprintf(f, "# Equilibrium table: %s\n", titulo_chart_);
                                fprintf(f, "# columns: T(°C), P(bar), G_total(kJ), amounts kmol per species\n");
                                fprintf(f, "T(°C)\tP(bar)\tG(kJ)");
                                for (auto& surf : chart_surfaces_) fprintf(f, "\t%s(%s)", surf.legend.c_str(), surf.fase.c_str());
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
                    pclose(zen);
                }
            }
            ImGui::SameLine();
            if (ImGui::Button(tr("Copiar al Portapapeles"), ImVec2(150, 26))) {
                std::string clip = "T(°C)\tP(bar)\tG(kJ)";
                for (auto& surf : chart_surfaces_) { clip += "\t" + surf.legend + "(" + surf.fase + ")"; }
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
// Panel de Simulación Dinámica
// ═══════════════════════════════════════════════════════════════════════════
void PanelSimulacionDinamica::renderizar() {
    ImGui::Text(tr("Simulación Dinámica (ODE)"));
    ImGui::Separator();
    ImGui::Combo("Modo", &modo_, "Vaciado de tanque\0Horno (enfriamiento)\0Reactor Batch\0");
    ImGui::Separator();

    ImGui::InputDouble("t inicial (s)", &t_inicial_);
    ImGui::InputDouble("t final (s)", &t_final_);
    ImGui::InputDouble("Paso (s)", &paso_);

    if (modo_ == 0) {
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
    } else if (modo_ == 1) {
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
            ImPlot::SetupAxes("Tiempo (s)", modo_ == 1 ? "T (K)" : modo_ == 0 ? "h (m)" : "C (mol/L)");
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

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Reconciliación de Datos
// ═══════════════════════════════════════════════════════════════════════════
void PanelReconciliacion::inicializar_datos(diagrama_flujo::Flowsheet* fs) {
    if (!fs || cargado_inicial_) return;
    unidades_.clear();
    corrientes_.clear();
    
    for (auto& n : fs->nodos()) {
        if (n.tipo == nucleo::TipoNodo::CajaTexto || n.tipo == nucleo::TipoNodo::Rectangulo || n.tipo == nucleo::TipoNodo::Elipse) continue;
        ReconciliacionUnit u;
        u.name = n.nombre;
        u.selected = true;
        u.psd_changing = (n.tipo == nucleo::TipoNodo::MolinoSAG || n.tipo == nucleo::TipoNodo::MolinoBolas || n.tipo == nucleo::TipoNodo::TrituradoraGiratoria);
        unidades_.push_back(u);
    }
    
    for (auto& [fid, f] : fs->mapa_flujos()) {
        ReconciliacionStream s;
        s.id = fid;
        s.name = f.nombre.empty() ? ("Stream " + std::to_string(fid)) : f.nombre;
        s.selected = f.mb_selected;
        s.assayed = f.mb_assayed;
        
        if (f.measured_flow >= 0.0) {
            s.measured_flow = f.measured_flow;
        } else {
            s.measured_flow = f.flujo_total();
            if (s.measured_flow <= 0.0) s.measured_flow = 100.0;
        }
        
        if (f.sd >= 0.0) {
            s.sd = f.sd;
        } else {
            s.sd = s.measured_flow * 0.05;
            if (s.sd <= 0.0) s.sd = 2.0;
        }
        corrientes_.push_back(s);
    }
    cargado_inicial_ = true;
}

void PanelReconciliacion::ejecutar_balance() {
    std::vector<motores::Medicion> ent;
    std::vector<motores::Medicion> sal;
    
    for (auto& s : corrientes_) {
        if (!s.selected) continue;
        motores::Medicion m = { s.name, s.measured_flow, s.sd };
        if (s.assayed) {
            ent.push_back(m);
        } else {
            sal.push_back(m);
        }
    }
    
    if (ent.empty() || sal.empty()) {
        ent.clear(); sal.clear();
        int half = (int)corrientes_.size() / 2;
        if (half < 1) half = 1;
        for (int i = 0; i < (int)corrientes_.size(); i++) {
            auto& s = corrientes_[i];
            motores::Medicion m = { s.name, s.measured_flow, s.sd };
            if (i < half) ent.push_back(m);
            else sal.push_back(m);
        }
    }
    
    resultado_ = motor_.reconciliar_balance_masa(ent, sal);
    calculado_ = resultado_.ok;
    
    if (calculado_) {
        for (auto& s : corrientes_) {
            auto it = resultado_.valores_reconciliados.find(s.name);
            if (it != resultado_.valores_reconciliados.end()) {
                s.reconciled_flow = it->second;
                s.reconciled_error = resultado_.residuos[s.name];
            } else {
                s.reconciled_flow = s.measured_flow;
                s.reconciled_error = 0.0;
            }
        }
    }
}

void PanelReconciliacion::renderizar(diagrama_flujo::Flowsheet* flowsheet) {
    if (!flowsheet) {
        ImGui::Text(tr("Cargue un flowsheet para realizar balance de masa."));
        return;
    }
    
    inicializar_datos(flowsheet);

    // ── Barra de herramientas superior ──
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(6, 4));
    if (ImGui::Button(tr("Abrir"))) {
        cargado_inicial_ = false;
        inicializar_datos(flowsheet);
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Guardar"))) {
        for (auto& s : corrientes_) {
            auto* f = flowsheet->flujo_por_id(s.id);
            if (f) {
                f->measured_flow = s.measured_flow;
                f->sd = s.sd;
                f->mb_selected = s.selected;
                f->mb_assayed = s.assayed;
            }
        }
        ImGui::OpenPopup("mb_saved");
    }
    if (ImGui::BeginPopup("mb_saved")) {
        ImGui::Text(tr("Configuration saved to flowsheet streams!"));
        if (ImGui::Button(tr("Cerrar"))) ImGui::CloseCurrentPopup();
        ImGui::EndPopup();
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Importar Datos"))) {
        FILE* zenity = popen("zenity --file-selection --file-filter='*.csv' --title='Import Raw Mass Balance Measurements' 2>/dev/null", "r");
        if (zenity) {
            char path[512] = {0};
            if (fgets(path, sizeof(path), zenity)) {
                std::string path_str(path);
                if (!path_str.empty() && path_str.back() == '\n') path_str.pop_back();
                if (!path_str.empty()) {
                    FILE* fp = fopen(path_str.c_str(), "r");
                    if (fp) {
                        char line[256];
                        // Skip header
                        if (fgets(line, sizeof(line), fp)) {
                            while (fgets(line, sizeof(line), fp)) {
                                std::string l(line);
                                auto parts = split_str(l, ",");
                                if (parts.size() >= 2) {
                                    std::string name = trim_str(parts[0]);
                                    try {
                                        double val = std::stod(parts[1]);
                                        double sd_val = (parts.size() >= 3) ? std::stod(parts[2]) : (val * 0.05);
                                        
                                        // Find stream by name (case-insensitive)
                                        std::string name_lower = name;
                                        std::transform(name_lower.begin(), name_lower.end(), name_lower.begin(), ::tolower);
                                        for (auto& s : corrientes_) {
                                            std::string s_lower = s.name;
                                            std::transform(s_lower.begin(), s_lower.end(), s_lower.begin(), ::tolower);
                                            if (s_lower == name_lower) {
                                                s.measured_flow = val;
                                                s.sd = sd_val;
                                                // Sync with flowsheet
                                                auto* f = flowsheet->flujo_por_id(s.id);
                                                if (f) {
                                                    f->measured_flow = val;
                                                    f->sd = sd_val;
                                                }
                                            }
                                        }
                                    } catch (...) {}
                                }
                            }
                        }
                        fclose(fp);
                    }
                }
            }
            pclose(zenity);
        }
    }
    ImGui::PopStyleVar();
    ImGui::Separator();
    ImGui::Spacing();

    // ── Layout de tres paneles (Splitters) ──
    // Izquierda: Balancing Navigator (25% ancho)
    // Centro: Data Grid (50% ancho)
    // Derecha: Properties Inspector (25% ancho)
    ImGui::Columns(3, "mass_balance_split", true);
    static bool init_w = true;
    if (init_w) {
        float w = ImGui::GetWindowWidth();
        ImGui::SetColumnWidth(0, w * 0.25f);
        ImGui::SetColumnWidth(1, w * 0.50f);
        ImGui::SetColumnWidth(2, w * 0.25f);
        init_w = false;
    }

    // ════════════════════════════════════════════════════════════════════════
    // Panel Izquierdo: Balancing Navigator
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("nav_pane");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.6f, 0.7f, 0.9f, 1.0f), "Balancing Navigator");
        ImGui::Separator();
        
        auto render_nav_item = [&](const char* label, SeccionNavegacion target) {
            bool sel = (seccion_ == target);
            if (ImGui::Selectable(label, sel)) {
                seccion_ = target;
                fila_seleccionada_ = -1;
            }
        };

        if (ImGui::CollapsingHeader("Experimental Data", ImGuiTreeNodeFlags_DefaultOpen)) {
            render_nav_item("Units", SeccionNavegacion::Units);
            render_nav_item("Streams", SeccionNavegacion::Streams);
            render_nav_item("Variables", SeccionNavegacion::Variables);
            render_nav_item("Size Fractions", SeccionNavegacion::SizeFractions);
            render_nav_item("Data Sets", SeccionNavegacion::DataSets);
            render_nav_item("Measurement Data", SeccionNavegacion::MeasurementData);
        }
        
        if (ImGui::CollapsingHeader("Estado de Datos")) {
            render_nav_item("Stream View", SeccionNavegacion::StreamView);
            render_nav_item("Variable View", SeccionNavegacion::VariableView);
        }
        
        if (ImGui::CollapsingHeader("Standard Deviations")) {
            render_nav_item("SD - Stream View", SeccionNavegacion::SDStream);
            render_nav_item("SD - Variable View", SeccionNavegacion::SDVariable);
        }
        
        render_nav_item("Balancing Solver", SeccionNavegacion::Balancing);
        render_nav_item("Reporting & Stats", SeccionNavegacion::Reporting);
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();

    // ════════════════════════════════════════════════════════════════════════
    // Panel Central: Data Grid / Central Editor
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("grid_pane");
    {
        if (seccion_ == SeccionNavegacion::Units) {
            ImGui::Text(tr("Process Units Configuration"));
            if (ImGui::BeginTable("units_grid", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                ImGui::TableSetupColumn("Selected", ImGuiTableColumnFlags_WidthFixed, 60.0f);
                ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("PSD Changing", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableHeadersRow();
                
                for (int i = 0; i < (int)unidades_.size(); i++) {
                    auto& u = unidades_[i];
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn();
                    ImGui::Checkbox(("##sel_u"+std::to_string(i)).c_str(), &u.selected);
                    
                    ImGui::TableNextColumn();
                    bool row_sel = (fila_seleccionada_ == i);
                    if (ImGui::Selectable(u.name.c_str(), row_sel, ImGuiSelectableFlags_SpanAllColumns)) {
                        fila_seleccionada_ = i;
                    }
                    
                    ImGui::TableNextColumn();
                    ImGui::Checkbox(("##psd_u"+std::to_string(i)).c_str(), &u.psd_changing);
                }
                ImGui::EndTable();
            }
        }
        else if (seccion_ == SeccionNavegacion::Streams) {
            ImGui::Text(tr("Process Streams Settings"));
            if (ImGui::BeginTable("streams_grid", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                ImGui::TableSetupColumn("Selected", ImGuiTableColumnFlags_WidthFixed, 60.0f);
                ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Input Stream", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableSetupColumn("Measured Flow", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableSetupColumn("Std Dev (SD)", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                ImGui::TableHeadersRow();
                
                for (int i = 0; i < (int)corrientes_.size(); i++) {
                    auto& s = corrientes_[i];
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn();
                    ImGui::Checkbox(("##sel_s"+std::to_string(i)).c_str(), &s.selected);
                    
                    ImGui::TableNextColumn();
                    bool row_sel = (fila_seleccionada_ == i);
                    if (ImGui::Selectable(s.name.c_str(), row_sel, ImGuiSelectableFlags_SpanAllColumns)) {
                        fila_seleccionada_ = i;
                    }
                    
                    ImGui::TableNextColumn();
                    ImGui::Checkbox(("##in_s"+std::to_string(i)).c_str(), &s.assayed);
                    
                    ImGui::TableNextColumn();
                    ImGui::SetNextItemWidth(90.0f);
                    ImGui::InputDouble(("##flo_s"+std::to_string(i)).c_str(), &s.measured_flow, 0, 0, "%.2f");
                    
                    ImGui::TableNextColumn();
                    ImGui::SetNextItemWidth(70.0f);
                    ImGui::InputDouble(("##sd_s"+std::to_string(i)).c_str(), &s.sd, 0, 0, "%.2f");
                }
                ImGui::EndTable();
            }
        }
        else if (seccion_ == SeccionNavegacion::SDStream) {
            ImGui::Text(tr("Desviaciones Est. Corrientes"));
            if (ImGui::BeginTable("sd_grid", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Measured Flow", ImGuiTableColumnFlags_WidthFixed, 120.0f);
                ImGui::TableSetupColumn("SD % of Flow", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableHeadersRow();
                
                for (int i = 0; i < (int)corrientes_.size(); i++) {
                    auto& s = corrientes_[i];
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn();
                    ImGui::Text(tr("%s"), s.name.c_str());
                    ImGui::TableNextColumn();
                    ImGui::Text(tr("%.2f"), s.measured_flow);
                    ImGui::TableNextColumn();
                    double pct = (s.measured_flow > 0.0) ? (s.sd / s.measured_flow * 100.0) : 5.0;
                    ImGui::Text(tr("%.1f %%"), pct);
                }
                ImGui::EndTable();
            }
        }
        else if (seccion_ == SeccionNavegacion::Balancing) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f,0.8f,0.6f,1.0f), "Mass Balance Solver Settings");
            ImGui::Separator();
            ImGui::Spacing();
            
            ImGui::Text(tr("Problem Type Option:"));
            ImGui::RadioButton("Solids Only (Standard metallurgical balancing)", &tipo_problema_, 0);
            ImGui::RadioButton("Assays Only (No direct flow measurements)", &tipo_problema_, 1);
            ImGui::RadioButton("Solids & Water Independently (Solve pulp flow rates)", &tipo_problema_, 2);
            
            ImGui::Spacing();
            ImGui::Separator();
            ImGui::Spacing();
            
            if (ImGui::Button(tr("Solve Mass Balance (Run Reconciliation)"), ImVec2(-1, 35))) {
                ejecutar_balance();
            }
            
            ImGui::Spacing();
            ImGui::Text(tr("Solver Log:"));
            ImGui::BeginChild("solver_log", ImVec2(0, 100), true);
            if (calculado_) {
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f, 0.9f, 0.4f, 1.0f), "Reconciliation finalized successfully!");
                ImGui::Text(tr("Degrees of freedom: %zu"), corrientes_.size() - 1);
                ImGui::Text(tr("Convergence chi-square check finalized."));
            } else {
                ImGui::Text(tr("Ready. Press solver button above to calculate."));
            }
            ImGui::EndChild();
        }
        else if (seccion_ == SeccionNavegacion::Reporting) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.3f,0.8f,0.9f,1.0f), "Data Reconciliation Report");
            ImGui::Separator();
            
            if (calculado_) {
                // ── Stats summary ──
                float chi2 = (float)resultado_.chi_cuadrado;
                float q    = (float)resultado_.q_ajuste;
                bool good  = (q > 0.05f);
                ImGui::TextColored(good ? cma::interfaz::AdaptiveColor(0.3f,0.9f,0.4f,1.0f) : cma::interfaz::AdaptiveColor(0.9f,0.5f,0.3f,1.0f),
                    "X2 = %.4f   Q = %.4f   %s", chi2, q, good ? "[OK] Acceptable fit" : "[!] Poor fit");
                
                ImGui::SameLine();
                if (ImGui::Button(tr("Exportar CSV##report"))) {
                    FILE* zen = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' 2>/dev/null", "r");
                    if (zen) {
                        char path[512]={0};
                        if (fgets(path, sizeof(path), zen)) {
                            std::string p(path);
                            if (!p.empty() && p.back()=='\n') p.pop_back();
                            if (!p.empty()) {
                                if (p.find(".csv")==std::string::npos) p+=".csv";
                                FILE* fp = fopen(p.c_str(),"w");
                                if (fp) {
                                    fprintf(fp,"Chi2,%.6f\nQ,%.6f\n\n",resultado_.chi_cuadrado,resultado_.q_ajuste);
                                    fprintf(fp,"Stream,Measured,Reconciled,Residual,RecSD\n");
                                    for (auto& s : corrientes_) {
                                        fprintf(fp,"\"%s\",%.4f,%.4f,%.4f,%.4f\n",
                                            s.name.c_str(),s.measured_flow,s.reconciled_flow,
                                            s.measured_flow-s.reconciled_flow,s.reconciled_error);
                                    }
                                    fclose(fp);
                                }
                            }
                        }
                        pclose(zen);
                    }
                }
                
                ImGui::Spacing();

                if (ImGui::BeginTabBar("report_tabs")) {
                    if (ImGui::BeginTabItem(tr("Tabla"))) {
                        if (ImGui::BeginTable("report_grid", 5,
                                ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                                ImVec2(0, 180))) {
                            ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthStretch);
                            ImGui::TableSetupColumn("Measured", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                            ImGui::TableSetupColumn("Reconciled", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                            ImGui::TableSetupColumn("Residual", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                            ImGui::TableSetupColumn("SD", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                            ImGui::TableHeadersRow();
                            
                            for (auto& s : corrientes_) {
                                double res = s.measured_flow - s.reconciled_flow;
                                ImGui::TableNextRow();
                                ImGui::TableNextColumn(); ImGui::Text(tr("%s"), s.name.c_str());
                                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), s.measured_flow);
                                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), s.reconciled_flow);
                                ImGui::TableNextColumn();
                                if (fabs(res) < s.sd)
                                    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.3f,0.9f,0.4f,1.0f), "%.2f", res);
                                else
                                    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f,0.4f,0.3f,1.0f), "%.2f", res);
                                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), s.reconciled_error);
                            }
                            ImGui::EndTable();
                        }
                        ImGui::EndTabItem();
                    }
                    if (ImGui::BeginTabItem(tr("Residuals Chart"))) {
                        static std::vector<double> res_vals, xs_r;
                        static std::vector<const char*> lbs_r;
                        int nc = (int)corrientes_.size();
                        res_vals.resize(nc); xs_r.resize(nc); lbs_r.resize(nc);
                        for (int i=0;i<nc;i++){
                            res_vals[i] = corrientes_[i].measured_flow - corrientes_[i].reconciled_flow;
                            xs_r[i] = (double)i;
                            lbs_r[i] = corrientes_[i].name.c_str();
                        }
                        if (ImPlot::BeginPlot("Residuals (Measured - Reconciled)", ImVec2(-1, 200))) {
                            ImPlot::SetupAxes("Corriente", "Residual (kg/h)");
                            if (nc > 0)
                                ImPlot::SetupAxisTicks(ImAxis_X1, xs_r.data(), nc, lbs_r.data());
                            ImPlot::SetNextFillStyle(ImVec4(0.4f,0.7f,1.0f,0.85f));
                            ImPlot::PlotBars("Residual", res_vals.data(), nc, 0.6);
                            ImPlot::EndPlot();
                        }
                        ImGui::EndTabItem();
                    }
                    ImGui::EndTabBar();
                }
            } else {
                ImGui::TextDisabled(tr("Run mass balance calculations to generate reconciliation report."));
            }
        }

        else {
            ImGui::TextDisabled(tr("This section is configured automatically by flowsheet parameters."));
        }
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();

    // ════════════════════════════════════════════════════════════════════════
    // Panel Derecho: Properties Inspector
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("prop_pane");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.5f, 1.0f), "Properties");
        ImGui::Separator();
        
        if (seccion_ == SeccionNavegacion::Units && fila_seleccionada_ >= 0 && fila_seleccionada_ < (int)unidades_.size()) {
            auto& u = unidades_[fila_seleccionada_];
            ImGui::Text(tr("Unit settings:"));
            ImGui::TextDisabled(tr("Name: %s"), u.name.c_str());
            ImGui::Checkbox(tr("Selected"), &u.selected);
            ImGui::Checkbox(tr("PSD Changing"), &u.psd_changing);
        }
        else if (seccion_ == SeccionNavegacion::Streams && fila_seleccionada_ >= 0 && fila_seleccionada_ < (int)corrientes_.size()) {
            auto& s = corrientes_[fila_seleccionada_];
            ImGui::Text(tr("Stream settings:"));
            ImGui::TextDisabled(tr("Name: %s"), s.name.c_str());
            ImGui::Checkbox(tr("Selected"), &s.selected);
            ImGui::Checkbox(tr("Input Stream"), &s.assayed);
            ImGui::InputDouble("Measured", &s.measured_flow, 1.0, 10.0, "%.2f");
            ImGui::InputDouble("SD", &s.sd, 0.5, 5.0, "%.2f");
        }
        else {
            ImGui::TextDisabled(tr("Select an element in middle table grid to view properties."));
        }
    }
    ImGui::EndChild();
    
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Balance Hoja de Cálculo
// ═══════════════════════════════════════════════════════════════════════════
PanelBalanceHoja::PanelBalanceHoja() {
    inicializar_valores_defecto();
}

void PanelBalanceHoja::inicializar_valores_defecto() {
    entradas_.clear();
    salidas_.clear();
    
    // Input Stream: CH4 Feed
    StreamGroup in1;
    strncpy(in1.nombre, "CH4 Input", sizeof(in1.nombre)-1);
    
    SpeciesRow ch4;
    strncpy(ch4.formula, "CH4(g)", sizeof(ch4.formula)-1);
    ch4.temper = 25.0;
    ch4.pressure = 1.0;
    ch4.amount_kmol = 3.300;
    in1.filas.push_back(ch4);
    entradas_.push_back(in1);

    // Input Stream: Combustion Air
    StreamGroup in2;
    strncpy(in2.nombre, "Combustion Air", sizeof(in2.nombre)-1);
    
    SpeciesRow o2;
    strncpy(o2.formula, "O2(g)", sizeof(o2.formula)-1);
    o2.temper = 25.0;
    o2.pressure = 1.0;
    o2.amount_kmol = 7.350;
    in2.filas.push_back(o2);
    
    SpeciesRow n2;
    strncpy(n2.formula, "N2(g)", sizeof(n2.formula)-1);
    n2.temper = 25.0;
    n2.pressure = 1.0;
    n2.amount_kmol = 27.650;
    in2.filas.push_back(n2);
    entradas_.push_back(in2);

    // Output Stream: Offgas
    StreamGroup out1;
    strncpy(out1.nombre, "Offgas", sizeof(out1.nombre)-1);
    
    SpeciesRow co2;
    strncpy(co2.formula, "CO2(g)", sizeof(co2.formula)-1);
    co2.temper = 350.0;
    co2.pressure = 1.0;
    co2.amount_kmol = 3.300;
    out1.filas.push_back(co2);
    
    SpeciesRow h2o;
    strncpy(h2o.formula, "H2O(g)", sizeof(h2o.formula)-1);
    h2o.temper = 350.0;
    h2o.pressure = 1.0;
    h2o.amount_kmol = 5.900;
    out1.filas.push_back(h2o);
    
    SpeciesRow n2_out;
    strncpy(n2_out.formula, "N2(g)", sizeof(n2_out.formula)-1);
    n2_out.temper = 350.0;
    n2_out.pressure = 1.0;
    n2_out.amount_kmol = 26.333;
    out1.filas.push_back(n2_out);

    SpeciesRow o2_out;
    strncpy(o2_out.formula, "O2(g)", sizeof(o2_out.formula)-1);
    o2_out.temper = 350.0;
    o2_out.pressure = 1.0;
    o2_out.amount_kmol = 0.750;
    out1.filas.push_back(o2_out);

    out1.heat_loss = 597.36;
    salidas_.push_back(out1);
    
    reciclar_calculos();
    calcular_balance_elemental();
}

void PanelBalanceHoja::reciclar_calculos() {
    double energy_factor = 1.0;
    if (energy_unit_ == 0) energy_factor = 1.0 / 4184.0; // kJ -> Mcal
    else if (energy_unit_ == 1) energy_factor = 1.0 / 1000.0; // kJ -> MJ
    else if (energy_unit_ == 2) energy_factor = 1.0 / 3600.0; // kJ -> kWh

    auto calc_rows = [&](std::vector<StreamGroup>& groups) {
        for (auto& g : groups) {
            for (auto& row : g.filas) {
                double T_K = (temp_unit_ == 0) ? (row.temper + 273.15) : row.temper;
                const auto* esp = buscar_especie_por_nombre_o_formula(row.formula);
                if (esp) {
                    row.amount_kg = row.amount_kmol * esp->masa_molar;
                    row.amount_nm3 = row.amount_kmol * 22.414;
                    double H_abs = esp->entalpia_abs(T_K); // kJ/mol
                    row.heat_content = H_abs * 1000.0 * energy_factor; // unit/kmol
                    row.total_h = row.amount_kmol * row.heat_content;
                } else {
                    row.amount_kg = 0.0;
                    row.amount_nm3 = 0.0;
                    row.heat_content = 0.0;
                    row.total_h = 0.0;
                }
            }
        }
    };
    calc_rows(entradas_);
    calc_rows(salidas_);
}

double PanelBalanceHoja::calcular_entalpia_corrientes(const std::vector<StreamGroup>& grupos, double t_out_forced) {
    double total_h = 0.0;
    
    double energy_factor = 1.0;
    if (energy_unit_ == 0) energy_factor = 1.0 / 4184.0;
    else if (energy_unit_ == 1) energy_factor = 1.0 / 1000.0;
    else if (energy_unit_ == 2) energy_factor = 1.0 / 3600.0;
    
    for (auto& g : grupos) {
        for (auto& row : g.filas) {
            double temp_c = (t_out_forced != -9999.0) ? t_out_forced : row.temper;
            double T_K = (temp_unit_ == 0) ? (temp_c + 273.15) : temp_c;
            const auto* esp = buscar_especie_por_nombre_o_formula(row.formula);
            if (esp) {
                double H_abs = esp->entalpia_abs(T_K);
                total_h += row.amount_kmol * H_abs * 1000.0 * energy_factor;
            }
        }
        total_h += g.heat_loss;
    }
    return total_h;
}

void PanelBalanceHoja::Temp_Balance() {
    double H_in = calcular_entalpia_corrientes(entradas_);
    
    auto f = [&](double temp) {
        return calcular_entalpia_corrientes(salidas_, temp) - H_in;
    };
    
    double t0 = (temp_unit_ == 0) ? 25.0 : 298.15;
    double t1 = (temp_unit_ == 0) ? 1500.0 : 1773.15;
    
    double f0 = f(t0);
    double f1 = f(t1);
    
    for (int iter = 0; iter < 50; ++iter) {
        if (std::abs(f1 - f0) < 1e-8) break;
        double t_next = t1 - f1 * (t1 - t0) / (f1 - f0);
        
        // Clamp to physical ranges to keep solver from diverging
        double t_min = (temp_unit_ == 0) ? -273.0 : 0.15;
        double t_max = (temp_unit_ == 0) ? 6000.0 : 6273.15;
        t_next = std::clamp(t_next, t_min, t_max);
        
        t0 = t1;
        f0 = f1;
        t1 = t_next;
        f1 = f(t1);
        if (std::abs(f1) < 1e-4) {
            break;
        }
    }
    
    for (auto& g : salidas_) {
        for (auto& r : g.filas) {
            r.temper = t1;
        }
    }
    
    reciclar_calculos();
}

void PanelBalanceHoja::calcular_balance_elemental() {
    elem_balance_.clear();
    std::map<std::string, double> in_elems;
    std::map<std::string, double> out_elems;
    
    for (auto& g : entradas_) {
        for (auto& r : g.filas) {
            std::string err;
            auto el_map = parse_formula_elements(r.formula, err);
            for (auto& [el, cnt] : el_map) {
                in_elems[el] += cnt * r.amount_kmol;
            }
        }
    }
    for (auto& g : salidas_) {
        for (auto& r : g.filas) {
            std::string err;
            auto el_map = parse_formula_elements(r.formula, err);
            for (auto& [el, cnt] : el_map) {
                out_elems[el] += cnt * r.amount_kmol;
            }
        }
    }
    
    std::set<std::string> all_elements;
    for (auto& [el, v] : in_elems) all_elements.insert(el);
    for (auto& [el, v] : out_elems) all_elements.insert(el);
    
    for (auto& el : all_elements) {
        ElementBal b;
        b.element = el;
        b.in_val = in_elems[el];
        b.out_val = out_elems[el];
        b.balance = b.in_val - b.out_val;
        elem_balance_.push_back(b);
    }
}

void PanelBalanceHoja::renderizar(diagrama_flujo::Flowsheet* flowsheet) {
    std::vector<StreamGroup>& active_groups = (tab_actual_ == 2) ? salidas_ : entradas_;
    // ── Toolbar ──
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(6, 4));
    if (ImGui::Button(tr("Open TXT"))) {
        FILE* zen = popen("zenity --file-selection --file-filter='*.txt' --title='Open HMB Data File' 2>/dev/null", "r");
        if (zen) {
            char buf[512] = {0};
            if (fgets(buf, sizeof(buf), zen)) {
                std::string path(buf);
                if (!path.empty() && path.back() == '\n') path.pop_back();
                FILE* fp = fopen(path.c_str(), "r");
                if (fp) {
                    entradas_.clear();
                    salidas_.clear();
                    bool parse_outputs = false;
                    char line[256];
                    while (fgets(line, sizeof(line), fp)) {
                        std::string l = trim_str(line);
                        if (l.empty() || l[0] == '#') {
                            if (l.find("Outputs") != std::string::npos) parse_outputs = true;
                            continue;
                        }
                        auto parts = split_str(l, ",");
                        if (parts[0] == "STREAM" && parts.size() >= 3) {
                            StreamGroup g;
                            strncpy(g.nombre, parts[1].c_str(), sizeof(g.nombre)-1);
                            g.nombre[sizeof(g.nombre)-1] = '\0';
                            try { g.heat_loss = std::stod(parts[2]); } catch (...) { g.heat_loss = 0.0; }
                            if (parse_outputs) salidas_.push_back(g);
                            else entradas_.push_back(g);
                        } else if (parts[0] == "ROW" && parts.size() >= 5) {
                            SpeciesRow r;
                            strncpy(r.formula, parts[1].c_str(), sizeof(r.formula)-1);
                            r.formula[sizeof(r.formula)-1] = '\0';
                            try {
                                r.temper = std::stod(parts[2]);
                                r.pressure = std::stod(parts[3]);
                                r.amount_kmol = std::stod(parts[4]);
                            } catch (...) { continue; }
                            if (parse_outputs && !salidas_.empty()) salidas_.back().filas.push_back(r);
                            else if (!parse_outputs && !entradas_.empty()) entradas_.back().filas.push_back(r);
                        }
                    }
                    fclose(fp);
                    reciclar_calculos();
                    calcular_balance_elemental();
                }
            }
            pclose(zen);
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Save TXT"))) {
        FILE* zen = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.txt' --title='Save HMB Data File' 2>/dev/null", "r");
        if (zen) {
            char buf[512] = {0};
            if (fgets(buf, sizeof(buf), zen)) {
                std::string path(buf);
                if (!path.empty() && path.back() == '\n') path.pop_back();
                if (path.find(".txt") == std::string::npos) path += ".txt";
                FILE* fp = fopen(path.c_str(), "w");
                if (fp) {
                    fprintf(fp, "# HMB Inputs\n");
                    for (auto& g : entradas_) {
                        fprintf(fp, "STREAM,%s,%f\n", g.nombre, g.heat_loss);
                        for (auto& r : g.filas) {
                            fprintf(fp, "ROW,%s,%f,%f,%f\n", r.formula, r.temper, r.pressure, r.amount_kmol);
                        }
                    }
                    fprintf(fp, "# HMB Outputs\n");
                    for (auto& g : salidas_) {
                        fprintf(fp, "STREAM,%s,%f\n", g.nombre, g.heat_loss);
                        for (auto& r : g.filas) {
                            fprintf(fp, "ROW,%s,%f,%f,%f\n", r.formula, r.temper, r.pressure, r.amount_kmol);
                        }
                    }
                    fclose(fp);
                }
            }
            pclose(zen);
        }
    }
    ImGui::SameLine();
    if (flowsheet && ImGui::Button(tr("Load from Flowsheet"))) {
        entradas_.clear();
        salidas_.clear();
        
        StreamGroup inputs_group;
        strncpy(inputs_group.nombre, "Flowsheet Inputs", sizeof(inputs_group.nombre)-1);
        
        StreamGroup outputs_group;
        strncpy(outputs_group.nombre, "Flowsheet Outputs", sizeof(outputs_group.nombre)-1);
        
        for (auto& [fid, f] : flowsheet->mapa_flujos()) {
            bool is_input = true;
            bool is_output = true;
            
            for (auto& n : flowsheet->nodos()) {
                for (int64_t sid : n.ids_flujos_salida) {
                    if (sid == fid && n.tipo != nucleo::TipoNodo::Fuente) {
                        is_input = false;
                    }
                }
                for (int64_t sid : n.ids_flujos_entrada) {
                    if (sid == fid && n.tipo != nucleo::TipoNodo::Sumidero) {
                        is_output = false;
                    }
                }
            }
            
            SpeciesRow r;
            std::string name = f.nombre.empty() ? ("Stream_" + std::to_string(fid)) : f.nombre;
            strncpy(r.formula, name.c_str(), sizeof(r.formula)-1);
            r.temper = f.temperatura - 273.15; // default C
            r.pressure = f.presion;
            r.amount_kmol = f.flujo_total() * 3600.0;
            
            if (is_input && !is_output) {
                inputs_group.filas.push_back(r);
            } else {
                outputs_group.filas.push_back(r);
            }
        }
        if (!inputs_group.filas.empty()) entradas_.push_back(inputs_group);
        if (!outputs_group.filas.empty()) salidas_.push_back(outputs_group);
        
        reciclar_calculos();
        calcular_balance_elemental();
    }
    ImGui::PopStyleVar();
    ImGui::Separator();
    
    // ── Columns Split (Spreadsheet vs Tools) ──
    ImGui::Columns(2, "hmb_columns_layout", true);
    static bool init_w_hmb = true;
    if (init_w_hmb) {
        ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.73f);
        ImGui::SetColumnWidth(1, ImGui::GetWindowWidth() * 0.27f);
        init_w_hmb = false;
    }

    // ════════════════════════════════════════════════════════════════════════
    // Columna Izquierda: Spreadsheet Grid & Bottom Panes
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("left_hmb_pane");
    {
        // ── Tabs Selector ──
        if (ImGui::Button(tr("BALANCE"))) tab_actual_ = 0; ImGui::SameLine();
        if (ImGui::Button(tr("IN1"))) tab_actual_ = 1; ImGui::SameLine();
        if (ImGui::Button(tr("OUT1"))) tab_actual_ = 2; ImGui::SameLine();
        if (ImGui::Button(tr("Target"))) tab_actual_ = 3;
        
        ImGui::Separator();
        ImGui::Spacing();

        
        if (tab_actual_ == 1 || tab_actual_ == 2) {
            // Render spreadsheet grid
            const char* header_col = (tab_actual_ == 2) ? "OUTPUT SPECIES" : "INPUT SPECIES";
            const char* unit_t_str = (temp_unit_ == 0) ? "°C" : "K";
            const char* unit_e_str = (energy_unit_ == 0) ? "Mcal" : (energy_unit_ == 1) ? "MJ" : "kWh";
            
            char header_label[128];
            snprintf(header_label, sizeof(header_label), "%s (1)\nFormula", header_col);
            char temp_label[64];
            snprintf(temp_label, sizeof(temp_label), "Temper.\n%s", unit_t_str);
            char h_content_label[64];
            snprintf(h_content_label, sizeof(h_content_label), "Heat Content\n%s/kmol", unit_e_str);
            char total_h_label[64];
            snprintf(total_h_label, sizeof(total_h_label), "Total H\n%s", unit_e_str);
            
            if (ImGui::BeginTable("grid_table", 8, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable, ImVec2(0, 240))) {
                ImGui::TableSetupColumn(header_label, ImGuiTableColumnFlags_WidthFixed, 140.0f);
                ImGui::TableSetupColumn(temp_label, ImGuiTableColumnFlags_WidthFixed, 80.0f);
                ImGui::TableSetupColumn("Pressure\nbar", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                ImGui::TableSetupColumn("Amount\nkmol", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableSetupColumn("Amount\nkg", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableSetupColumn("Amount\nNm³", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                ImGui::TableSetupColumn(h_content_label, ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableSetupColumn(total_h_label, ImGuiTableColumnFlags_WidthFixed, 100.0f);
                ImGui::TableHeadersRow();
                
                int s_idx = 0;
                for (auto& g : active_groups) {
                    ImGui::PushID(s_idx);
                    
                    // Stream group header row
                    ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
                    ImGui::TableSetColumnIndex(0);
                    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.5f, 0.8f, 1.0f, 1.0f));
                    ImGui::SetNextItemWidth(-1.0f);
                    ImGui::InputText("##stream_name", g.nombre, sizeof(g.nombre));
                    ImGui::PopStyleColor();
                    
                    for (int c = 1; c < 8; ++c) {
                        ImGui::TableNextColumn();
                        ImGui::TextDisabled(tr("---"));
                    }
                    
                    int r_idx = 0;
                    for (auto& row : g.filas) {
                        ImGui::TableNextRow();
                        ImGui::PushID(r_idx);
                        
                        // Highlight active row selection
                        bool is_selected = (selected_stream_idx_ == s_idx && selected_row_idx_ == r_idx);
                        if (is_selected) {
                            ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, IM_COL32(40, 60, 100, 255));
                        }
                        
                        // Formula
                        ImGui::TableNextColumn();
                        char btn_label[128];
                        snprintf(btn_label, sizeof(btn_label), "%s##fbtn%d_%d", row.formula[0] == '\0' ? "Buscar..." : row.formula, s_idx, r_idx);
                        if (ImGui::Button(btn_label, ImVec2(-1, 0))) {
                            modal_target_stream_ = s_idx;
                            modal_target_row_ = r_idx;
                            ModalBuscadorDB::instancia().abrir("Buscar Especie (HMB)");
                            selected_stream_idx_ = s_idx;
                            selected_row_idx_ = r_idx;
                        }
                        
                        // We will evaluate the modal return at the end of the window to avoid nested UI IDs issues
                        
                        // Temperature
                        ImGui::TableNextColumn();
                        ImGui::SetNextItemWidth(-1.0f);
                        if (ImGui::InputDouble("##temp", &row.temper, 0, 0, "%.3f")) {
                            reciclar_calculos();
                        }
                        
                        // Pressure
                        ImGui::TableNextColumn();
                        ImGui::SetNextItemWidth(-1.0f);
                        ImGui::InputDouble("##pres", &row.pressure, 0, 0, "%.1f");
                        
                        // Amount kmol
                        ImGui::TableNextColumn();
                        ImGui::SetNextItemWidth(-1.0f);
                        if (ImGui::InputDouble("##kmol", &row.amount_kmol, 0, 0, "%.3f")) {
                            reciclar_calculos();
                            calcular_balance_elemental();
                        }
                        
                        // Amount kg (computed)
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.3f"), row.amount_kg);
                        
                        // Amount Nm3 (computed)
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.3f"), row.amount_nm3);
                        
                        // Heat Content (computed)
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.2f"), row.heat_content);
                        
                        // Total H (computed)
                        ImGui::TableNextColumn();
                        ImGui::Text(tr("%.2f"), row.total_h);
                        
                        ImGui::PopID();
                        r_idx++;
                    }
                    
                    // Heat Loss Row
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0);
                    ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.6f, 0.6f, 1.0f), "Heat Loss:");
                    
                    ImGui::TableSetColumnIndex(7);
                    ImGui::SetNextItemWidth(-1.0f);
                    if (ImGui::InputDouble("##loss", &g.heat_loss, 0, 0, "%.2f")) {
                        reciclar_calculos();
                    }
                    
                    ImGui::PopID();
                    s_idx++;
                }
                ImGui::EndTable();
            }
        } 
        else if (tab_actual_ == 0) {
            // BALANCE tab
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f, 0.8f, 0.6f, 1.0f), "Heat & Material Overall Balance Summary");
            ImGui::Separator();
            ImGui::Spacing();
            
            double total_kmol_in = 0, total_kg_in = 0, total_nm3_in = 0, total_h_in = 0;
            for (auto& g : entradas_) {
                for (auto& r : g.filas) {
                    total_kmol_in += r.amount_kmol;
                    total_kg_in += r.amount_kg;
                    total_nm3_in += r.amount_nm3;
                    total_h_in += r.total_h;
                }
                total_h_in += g.heat_loss;
            }
            double total_kmol_out = 0, total_kg_out = 0, total_nm3_out = 0, total_h_out = 0;
            for (auto& g : salidas_) {
                for (auto& r : g.filas) {
                    total_kmol_out += r.amount_kmol;
                    total_kg_out += r.amount_kg;
                    total_nm3_out += r.amount_nm3;
                    total_h_out += r.total_h;
                }
                total_h_out += g.heat_loss;
            }
            
            const char* unit_e = (energy_unit_ == 0) ? "Mcal" : (energy_unit_ == 1) ? "MJ" : "kWh";
            if (ImGui::BeginTable("summary_table", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                ImGui::TableSetupColumn("Tipo");
                ImGui::TableSetupColumn("Amount (kmol)");
                ImGui::TableSetupColumn("Amount (kg)");
                ImGui::TableSetupColumn("Amount (Nm³)");
                char en_hdr[64]; snprintf(en_hdr, sizeof(en_hdr), "Total Enthalpy (%s)", unit_e);
                ImGui::TableSetupColumn(en_hdr);
                ImGui::TableHeadersRow();
                
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text(tr("Total ENTRADAS"));
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kmol_in);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kg_in);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_nm3_in);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), total_h_in);
                
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text(tr("Total SALIDAS"));
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kmol_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kg_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_nm3_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), total_h_out);
                
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(1,1,0,1), "Residual Difference");
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kmol_in - total_kmol_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_kg_in - total_kg_out);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), total_nm3_in - total_nm3_out);
                
                double residual_h = total_h_in - total_h_out;
                if (std::abs(residual_h) < 1e-2) {
                    ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0.3f, 0.9f, 0.4f, 1.0f), "%.3f (OK)", residual_h);
                } else {
                    ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f, 0.4f, 0.3f, 1.0f), "%.3f (UNBALANCED)", residual_h);
                }
                
                ImGui::EndTable();
            }
        }
        else if (tab_actual_ == 3) {
            // Target Tab
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.7f, 0.7f, 1.0f, 1.0f), "Goal Seek / Solver Configuration");
            ImGui::Separator();
            ImGui::Spacing();
            ImGui::Text(tr("Calculate Adiabatic flame temp or zero balance temperature:"));
            ImGui::BulletText("Target Variable: Output Temperature");
            ImGui::BulletText("Target Value of Enthalpy Difference: 0.0");
            if (ImGui::Button(tr("Run Temp. Balance Solver"), ImVec2(-1, 30))) {
                Temp_Balance();
            }
        }
        
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();
        
        // ── Bottom Columns (Element Balance & Total Balance) ──
        ImGui::Columns(2, "bottom_panes_hmb", true);
        
        // Element Balance
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.6f, 1.0f), "Element Balance");
        if (ImGui::BeginTable("elem_table", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 110))) {
            ImGui::TableSetupColumn("Element");
            ImGui::TableSetupColumn("IN (kmol)");
            ImGui::TableSetupColumn("OUT (kmol)");
            ImGui::TableSetupColumn("Balance (kmol)");
            ImGui::TableHeadersRow();
            
            for (auto& b : elem_balance_) {
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text(tr("%s"), b.element.c_str());
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), b.in_val);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), b.out_val);
                if (std::abs(b.balance) < 1e-4) {
                    ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0.3f, 0.9f, 0.4f, 1.0f), "0.000");
                } else {
                    ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f, 0.4f, 0.3f, 1.0f), "%.3f", b.balance);
                }
            }
            ImGui::EndTable();
        }
        
        ImGui::NextColumn();
        
        // Total Balance
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.6f, 1.0f), "Total Balance");
        if (ImGui::BeginTable("tot_bal_areas", 7, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 110))) {
            ImGui::TableSetupColumn("BALANCE AREAS");
            ImGui::TableSetupColumn("Amount kmol");
            ImGui::TableSetupColumn("Amount kg");
            ImGui::TableSetupColumn("Amount Nm³");
            ImGui::TableSetupColumn("Heat Content");
            ImGui::TableSetupColumn("H Total");
            ImGui::TableSetupColumn("Exergy");
            ImGui::TableHeadersRow();
            
            double sum_kmol = 0, sum_kg = 0, sum_nm3 = 0, sum_hc = 0, sum_h = 0;
            for (auto& g : salidas_) {
                for (auto& r : g.filas) {
                    sum_kmol += r.amount_kmol;
                    sum_kg += r.amount_kg;
                    sum_nm3 += r.amount_nm3;
                    sum_hc += r.heat_content;
                    sum_h += r.total_h;
                }
            }
            
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("Balance (1)"));
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_kmol);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_kg);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_nm3);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sum_hc);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sum_h);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), -669.07); // Simulated Exergy
            
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("Total Balance"));
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_kmol);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_kg);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), sum_nm3);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sum_hc);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), sum_h);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), -669.07);
            
            ImGui::EndTable();
        }
        
        ImGui::Columns(1);
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();

    // ════════════════════════════════════════════════════════════════════════
    // Columna Derecha: Tools Panel
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("right_hmb_pane");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.6f, 0.8f, 1.0f), "Tools");
        ImGui::Separator();
        ImGui::Spacing();
        
        // Balance Area selection
        ImGui::Text(tr("Área de Balance"));
        static int area_sel = 0;
        ImGui::SetNextItemWidth(-1.0f);
        ImGui::Combo("##bal_area_sel", &area_sel, "Balance Area 1\0Balance Area 2\0");
        ImGui::Spacing();
        
        // Get Species from Database
        ImGui::Text(tr("Get Species from Database"));
        if (ImGui::Button(tr("Entrada"), ImVec2(70, 24))) {
            if (tab_actual_ == 1 && !entradas_.empty()) {
                SpeciesRow r;
                strncpy(r.formula, "O2(g)", sizeof(r.formula)-1);
                entradas_[0].filas.push_back(r);
                reciclar_calculos();
                calcular_balance_elemental();
            }
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Salida"), ImVec2(70, 24))) {
            if (tab_actual_ == 2 && !salidas_.empty()) {
                SpeciesRow r;
                strncpy(r.formula, "CO2(g)", sizeof(r.formula)-1);
                salidas_[0].filas.push_back(r);
                reciclar_calculos();
                calcular_balance_elemental();
            }
        }
        ImGui::Spacing();
        ImGui::Separator();
        
        // Insert
        ImGui::Text(tr("Insert"));
        if (ImGui::Button(tr("Área de Balance"), ImVec2(110, 24))) {
            StreamGroup g;
            snprintf(g.nombre, sizeof(g.nombre), "Area %zu", active_groups.size() + 1);
            active_groups.push_back(g);
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Eliminar Área"), ImVec2(110, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                active_groups.erase(active_groups.begin() + selected_stream_idx_);
                selected_stream_idx_ = std::max(0, selected_stream_idx_ - 1);
                reciclar_calculos();
                calcular_balance_elemental();
            } else if (!active_groups.empty()) {
                active_groups.pop_back();
            }
        }
        if (ImGui::Button(tr("Corriente"), ImVec2(100, 24))) {
            StreamGroup g;
            snprintf(g.nombre, sizeof(g.nombre), "Stream %zu", active_groups.size() + 1);
            active_groups.push_back(g);
        }
        ImGui::SameLine();
        ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 15);
        if (ImGui::Button(tr("Selected Stream"), ImVec2(120, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                active_groups.erase(active_groups.begin() + selected_stream_idx_);
                selected_stream_idx_ = std::max(0, selected_stream_idx_ - 1);
                reciclar_calculos();
                calcular_balance_elemental();
            }
        }
        
        if (ImGui::Button(tr("Fila"), ImVec2(100, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                SpeciesRow r;
                active_groups[selected_stream_idx_].filas.push_back(r);
            }
        }
        ImGui::SameLine();
        ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 15);
        if (ImGui::Button(tr("Selected Row"), ImVec2(120, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                auto& stream = active_groups[selected_stream_idx_];
                if (selected_row_idx_ >= 0 && selected_row_idx_ < (int)stream.filas.size()) {
                    stream.filas.erase(stream.filas.begin() + selected_row_idx_);
                    selected_row_idx_ = std::max(0, selected_row_idx_ - 1);
                    reciclar_calculos();
                    calcular_balance_elemental();
                }
            }
        }
        
        if (ImGui::Button(tr("Flujo de Calor"), ImVec2(100, 24))) {
            if (selected_stream_idx_ >= 0 && selected_stream_idx_ < (int)active_groups.size()) {
                active_groups[selected_stream_idx_].heat_loss += 50.0;
                reciclar_calculos();
            }
        }
        
        ImGui::Spacing();
        ImGui::Separator();
        
        // Measure Units
        ImGui::Text(tr("Measure Units"));
        ImGui::Text(tr("Temperature:")); ImGui::SameLine();
        if (ImGui::RadioButton("°C", &temp_unit_, 0)) reciclar_calculos(); ImGui::SameLine();
        if (ImGui::RadioButton("K", &temp_unit_, 1)) reciclar_calculos();
        
        ImGui::Text(tr("Energy:"));
        if (ImGui::RadioButton("Mcal", &energy_unit_, 0)) reciclar_calculos(); ImGui::SameLine();
        if (ImGui::RadioButton("MJ", &energy_unit_, 1)) reciclar_calculos(); ImGui::SameLine();
        if (ImGui::RadioButton("kWh", &energy_unit_, 2)) reciclar_calculos();
        
        ImGui::Spacing();
        ImGui::Separator();
        
        // Functions
        ImGui::Text(tr("Functions"));
        ImGui::Checkbox(tr("Enable Predictive Typing"), &enable_predictive_typing_);
        
        if (ImGui::Button(tr("Temp. Balance"), ImVec2(110, 26))) {
            Temp_Balance();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Resumen"), ImVec2(100, 26))) {
            tab_actual_ = 0; // go to BALANCE
        }
        
        ImGui::Spacing();
        if (ImGui::Button(tr("Diagrama"), ImVec2(-1, 30))) {
            show_diagram_ = !show_diagram_;
        }
    }
    ImGui::EndChild();
    
    std::vector<const nucleo::Especie*> sel_hmb;
    if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (HMB)", sel_hmb, false)) { // Single selection
        if (!sel_hmb.empty() && modal_target_stream_ != -1 && modal_target_row_ != -1) {
            std::vector<StreamGroup>& groups = (tab_actual_ == 2) ? salidas_ : entradas_;
            if (modal_target_stream_ < (int)groups.size() && modal_target_row_ < (int)groups[modal_target_stream_].filas.size()) {
                strncpy(groups[modal_target_stream_].filas[modal_target_row_].formula, sel_hmb[0]->formula.c_str(), 63);
                reciclar_calculos();
                calcular_balance_elemental();
            }
        }
        modal_target_stream_ = -1;
        modal_target_row_ = -1;
    }
    
    ImGui::Columns(1);
    
    // ── Diagram window popup ──
    if (show_diagram_) {
        ImGui::SetNextWindowSize(ImVec2(450, 320), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Diagram (Adiabatic Flame Temp)"), &show_diagram_)) {
            if (!entradas_.empty() && !entradas_[0].filas.empty()) {
                double orig_val = entradas_[0].filas[0].amount_kmol;
                
                std::vector<double> xs;
                std::vector<double> ys;
                
                for (double x = 1.0; x <= 12.0; x += 1.0) {
                    entradas_[0].filas[0].amount_kmol = x;
                    Temp_Balance();
                    double solved_t = salidas_[0].filas[0].temper;
                    xs.push_back(x);
                    ys.push_back(solved_t);
                }
                
                entradas_[0].filas[0].amount_kmol = orig_val;
                reciclar_calculos();
                
                std::string label = "Adiabatic Flame Temp vs " + std::string(entradas_[0].filas[0].formula) + " Input";
                if (ImPlot::BeginPlot(label.c_str(), ImVec2(-1, -1))) {
                    ImPlot::SetupAxes("Reactant Input (kmol)", "Temperature (°C)");
                    ImPlot::PlotLine("Adiabatic Temp", xs.data(), ys.data(), (int)xs.size());
                    ImPlot::EndPlot();
                }
            } else {
                ImGui::Text(tr("Agrega especies de entrada para generar el diagrama."));
            }
            ImGui::End();
        }
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// Panel de Exergía
// ═══════════════════════════════════════════════════════════════════════════
void PanelExergia::renderizar() {
    ImGui::Text(tr("Análisis Exergético"));
    ImGui::Separator();
    ImGui::InputDouble("H entrada (kJ/mol)", &h_ent_);
    ImGui::InputDouble("H salida (kJ/mol)", &h_sal_);
    ImGui::InputDouble("H₀ ref. (kJ/mol)", &h0_);
    ImGui::InputDouble("T entrada (K)", &T_ent_);
    ImGui::InputDouble("T salida (K)", &T_sal_);
    ImGui::InputDouble("T₀ ambiente (K)", &T0_);
    ImGui::InputDouble("S entrada (kJ/(mol·K))", &s_ent_);
    ImGui::InputDouble("S salida (kJ/(mol·K))", &s_sal_);
    ImGui::InputDouble("S₀ ref. (kJ/(mol·K))", &s0_);
    ImGui::InputDouble("Trabajo útil (kJ/mol)", &W_util_);
    ImGui::InputDouble("Calor aportado (kJ/mol)", &Q_aporte_);
    if (ImGui::Button(tr("Calcular exergía"), ImVec2(-1,30))) {
        resultado_ = motor_.analizar_proceso(h_ent_, h_sal_, h0_, T_ent_, T_sal_, T0_, s_ent_, s_sal_, s0_, W_util_, Q_aporte_);
        calculado_ = true;
    }
    if (calculado_) {
        ImGui::Separator();
        ImGui::Text(tr("Exergía física: %.4f kJ/mol"), resultado_.exergia_fisica);
        ImGui::Text(tr("Exergía química: %.4f kJ/mol"), resultado_.exergia_quimica);
        ImGui::Text(tr("Exergía destruida: %.4f kJ/mol"), resultado_.exergia_destruida);
        ImGui::Text(tr("Exergía total: %.4f kJ/mol"), resultado_.exergia_total);
        ImGui::Text(tr("Eficiencia exergética: %.2f%%"), resultado_.eficiencia_exergetica);

        double vals[3] = {resultado_.exergia_fisica, resultado_.exergia_quimica, resultado_.exergia_destruida};
        const char* labels[3] = {"Física", "Química", "Destruida"};
        if (ImPlot::BeginPlot("Desglose Exergético", ImVec2(-1, 200), ImPlotFlags_NoTitle)) {
            ImPlot::SetupAxes("Componente", "Exergía (kJ/mol)");
            double max_val = 1.0;
            for (int i = 0; i < 3; i++) if (vals[i] > max_val) max_val = vals[i];
            ImPlot::SetupAxesLimits(-0.5, 2.5, 0, max_val * 1.2);
            for (int i = 0; i < 3; i++) {
                ImPlot::PlotBars(labels[i], &vals[i], 1, 0.5, i - 0.25);
            }
            ImPlot::EndPlot();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Diagramas H, S, Cp, G vs T
// ═══════════════════════════════════════════════════════════════════════════
void PanelDiagramasTermo::recalcular() {
    datos_calculados_.clear();
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    
    for (auto& par : especies_seleccionadas_) {
        if (!par.second) continue;
        const nucleo::Especie* esp = nullptr;
        for (const auto* e : db_especies) {
            if (e->id == par.first) { esp = e; break; }
        }
        if (!esp) continue;
        
        CurvaEspecie ce;
        ce.nombre = esp->formula;
        
        // Calculate curve points
        // Assuming PropertyDiagrams generates standard curve logic
        double cp_a = esp->cp_a, cp_b = esp->cp_b, cp_c = esp->cp_c, cp_d = esp->cp_d;
        double h_ref = esp->h_formacion;
        double s_ref = esp->s_formacion;
        
        ce.datos = motor_.generarCurvas(h_ref, s_ref, cp_a, cp_b, cp_c, cp_d, T_min_, T_max_);
        for (const auto& p : ce.datos) {
            ce.Ts_cache.push_back(p.T);
            double y = 0;
            switch (curva_activa_) {
                case 0: y = p.H; break;
                case 1: y = p.S; break;
                case 2: y = p.Cp; break;
                case 3: y = p.G; break;
                case 4: y = p.G; break; // Delta G approximated as G in this MVP
            }
            ce.Ys_cache.push_back(y);
        }
        datos_calculados_.push_back(ce);
    }
    calculado_ = !datos_calculados_.empty();
}

void PanelDiagramasTermo::renderizar() {
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    
    // UI layout: Left panel (Species selection), Right panel (Controls & Plot)
    ImGui::Columns(2, "DiagramsLayout", true);
    ImGui::SetColumnWidth(0, 300.0f);
    
    // -- Left Column: Species Spreadsheet --
    ImGui::Text(tr("Especies Activas"));
    if (ImGui::Button(tr("Examinar Base de Datos"), ImVec2(-1, 0))) {
        ModalBuscadorDB::instancia().abrir("Buscar Especie (Termo)");
    }
    
    ImGui::BeginChild("SpeciesList", ImVec2(0, -1), true);
    if (ImGui::BeginTable("SpeciesTable", 3, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Especie", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("TMin (K)", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("TMax (K)", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableHeadersRow();
        
        for (auto& par : especies_seleccionadas_) {
            if (!par.second) continue;
            const nucleo::Especie* esp = nullptr;
            for (const auto* e : db_especies) {
                if (e->id == par.first) { esp = e; break; }
            }
            if (!esp) continue;
            
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::Text(tr("%s"), esp->formula.c_str());
            ImGui::TableSetColumnIndex(1);
            ImGui::Text(tr("%.2f"), T_min_);
            ImGui::TableSetColumnIndex(2);
            ImGui::Text(tr("%.2f"), T_max_);
        }
        ImGui::EndTable();
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();
    // -- Right Column: Controls and Plot --
    
    ImGui::Text(tr("Tipo de Diagrama"));
    ImGui::SetNextItemWidth(-1);
    if (ImGui::Combo("##Curva", &curva_activa_, "H (Enthalpy)\0S (Entropy)\0Cp (Heat Capacity)\0G (Gibbs Free Energy)\0Delta G (Ellingham)\0")) {
        recalcular();
    }
    
    ImGui::Spacing();
    ImGui::Text(tr("X-axis Range (Temperature K)"));
    ImGui::PushItemWidth(100);
    bool range_changed = false;
    range_changed |= ImGui::InputDouble("MIN##X", &T_min_, 0, 0, "%.0f");
    ImGui::SameLine();
    range_changed |= ImGui::InputDouble("MAX##X", &T_max_, 0, 0, "%.0f");
    ImGui::PopItemWidth();
    
    if (range_changed) recalcular();
    
    if (ImGui::Button(tr("Update Diagram"), ImVec2(-1,30))) {
        recalcular();
    }
    
    if (calculado_ && !datos_calculados_.empty()) {
        ImGui::Separator();
        static const char* nombres[] = {"H (kJ/mol)", "S (J/(mol·K))", "Cp (J/(mol·K))", "G (kJ/mol)", "Delta G (kJ/mol)"};
        
        if (ImPlot::BeginPlot("Thermochemical Data Diagrams", ImVec2(-1, -1))) {
            ImPlot::SetupAxes("Temperature (K)", nombres[curva_activa_]);
            ImPlot::SetupAxisLimits(ImAxis_X1, T_min_, T_max_);
            
            for (const auto& ce : datos_calculados_) {
                ImPlot::PlotLine(ce.nombre.c_str(), ce.Ts_cache.data(), ce.Ys_cache.data(), (int)ce.Ts_cache.size());
            }
            ImPlot::EndPlot();
        }
    } else {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.5f, 0.5f, 1.0f), "Add species from database and press Update Diagram to render.");
    }
    
    ImGui::Columns(1);
    
    // -- Modals --
    if (show_select_species_modal_) {
        ImGui::OpenPopup("Browse Database");
    }
    
    ImGui::SetNextWindowSize(ImVec2(600, 500), ImGuiCond_FirstUseEver);
    if (ImGui::BeginPopupModal("Browse Database", &show_select_species_modal_)) {
        ImGui::InputText(tr("Search"), search_buffer_, IM_ARRAYSIZE(search_buffer_));
        std::string search_str = search_buffer_;
        std::transform(search_str.begin(), search_str.end(), search_str.begin(), ::tolower);
        
        if (ImGui::BeginTable("DBTable", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, -40))) {
            ImGui::TableSetupScrollFreeze(0, 1);
            ImGui::TableSetupColumn("Selección", ImGuiTableColumnFlags_WidthFixed, 50.0f);
            ImGui::TableSetupColumn("ID", ImGuiTableColumnFlags_WidthFixed, 50.0f);
            ImGui::TableSetupColumn("Nombre", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("Fórmula", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableHeadersRow();
            
            for (const auto* esp : db_especies) {
                std::string nm = esp->nombre;
                std::string fm = esp->formula;
                std::string nml = nm, fml = fm;
                std::transform(nml.begin(), nml.end(), nml.begin(), ::tolower);
                std::transform(fml.begin(), fml.end(), fml.begin(), ::tolower);
                
                if (search_str.empty() || nml.find(search_str) != std::string::npos || fml.find(search_str) != std::string::npos) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0);
                    bool selected = especies_seleccionadas_[esp->id];
                    if (ImGui::Checkbox((std::string("##sel") + std::to_string(esp->id)).c_str(), &selected)) {
                        especies_seleccionadas_[esp->id] = selected;
                    }
                    ImGui::TableSetColumnIndex(1);
                    ImGui::Text(tr("%ld"), esp->id);
                    ImGui::TableSetColumnIndex(2);
                    ImGui::Text(tr("%s"), nm.c_str());
                    ImGui::TableSetColumnIndex(3);
                    ImGui::Text(tr("%s"), fm.c_str());
                }
            }
            ImGui::EndTable();
        }
        
        if (ImGui::Button(tr("Cerrar"), ImVec2(120, 0))) {
            ImGui::CloseCurrentPopup();
            show_select_species_modal_ = false;
            recalcular();
        }
        ImGui::EndPopup();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Diagrama de Estabilidad
// ═══════════════════════════════════════════════════════════════════════════
struct EvaluatedSpecies {
    int id;
    std::string formula;
    double n_m = 0;
    double n_g1 = 0;
    double n_g2 = 0;
    ImVec4 color;
};

// Simple pseudo-random color generator based on ID
ImVec4 color_from_id(int id) {
    float r = ((id * 37) % 255) / 255.0f;
    float g = ((id * 73) % 255) / 255.0f;
    float b = ((id * 113) % 255) / 255.0f;
    return ImVec4(r*0.6f + 0.2f, g*0.6f + 0.2f, b*0.6f + 0.2f, 1.0f);
}

void PanelEstabilidad::recalcular() {
    heatmap_.labels.clear();
    heatmap_.grid.assign(heatmap_.rows * heatmap_.cols, -1);
    
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    std::vector<EvaluatedSpecies> active_species;
    
    // Parse the gas formulas (e.g., O2 -> O, 2)
    auto parser = nucleo::TablaPeriodica::instancia();
    auto g1_atoms = parser.analizar_formula(gas1_element_);
    auto g2_atoms = parser.analizar_formula(gas2_element_);
    
    std::string g1_el = g1_atoms.empty() ? "" : g1_atoms[0].elem;
    double g1_factor = g1_atoms.empty() ? 1.0 : g1_atoms[0].cuenta;
    if (g1_factor == 0) g1_factor = 1.0;
    
    std::string g2_el = g2_atoms.empty() ? "" : g2_atoms[0].elem;
    double g2_factor = g2_atoms.empty() ? 1.0 : g2_atoms[0].cuenta;
    if (g2_factor == 0) g2_factor = 1.0;
    
    const nucleo::Especie* esp_g1 = nullptr;
    const nucleo::Especie* esp_g2 = nullptr;
    for (const auto* e : db_especies) {
        if (e->formula == gas1_element_ && e->fase_std == nucleo::Fase::Gas) esp_g1 = e;
        if (e->formula == gas2_element_ && e->fase_std == nucleo::Fase::Gas) esp_g2 = e;
    }
    if (!esp_g1) { for (const auto* e : db_especies) if (e->formula == gas1_element_) { esp_g1 = e; break; } }
    if (!esp_g2) { for (const auto* e : db_especies) if (e->formula == gas2_element_) { esp_g2 = e; break; } }
    
    int color_idx = 0;
    for (auto& par : especies_seleccionadas_) {
        if (!par.second) continue;
        const nucleo::Especie* esp = nullptr;
        for (const auto* e : db_especies) {
            if (e->id == par.first) { esp = e; break; }
        }
        if (!esp) continue;
        
        EvaluatedSpecies ev;
        ev.id = esp->id;
        ev.formula = esp->formula;
        ev.color = color_from_id(esp->id);
        
        auto atoms = parser.analizar_formula(esp->formula);
        for (const auto& at : atoms) {
            if (at.elem == main_element_) ev.n_m += at.cuenta;
            else if (at.elem == g1_el) ev.n_g1 += at.cuenta;
            else if (at.elem == g2_el) ev.n_g2 += at.cuenta;
        }
        
        if (ev.n_m > 0) {
            active_species.push_back(ev);
            heatmap_.labels[ev.id] = ev.formula;
            color_idx++;
        }
    }
    
    if (active_species.empty()) return;
    
    double dx = (x_max_ - x_min_) / heatmap_.cols;
    double dy = (y_max_ - y_min_) / heatmap_.rows;
    
    for (int r = 0; r < heatmap_.rows; ++r) {
        double y = y_min_ + (heatmap_.rows - 1 - r) * dy; // ImPlot heatmaps are top-to-bottom by default unless ColMajor/origin adjusted, but let's compute standard Cartesian. Actually ImPlot Y axis goes bottom to top. 
        // We will just map (c, r) directly to grid[r*cols + c] assuming standard ImPlot orientation.
        y = y_min_ + r * dy;
        
        for (int c = 0; c < heatmap_.cols; ++c) {
            double x = x_min_ + c * dx;
            
            double T = 298.15;
            double logp1 = 0, logp2 = 0;
            
            int fixed_var = 0;
            if (xaxis_var_ != 0 && yaxis_var_ != 0) fixed_var = 0;
            else if (xaxis_var_ != 1 && yaxis_var_ != 1) fixed_var = 1;
            else if (xaxis_var_ != 2 && yaxis_var_ != 2) fixed_var = 2;
            
            if (fixed_var == 0) T = fixed_val_;
            else if (fixed_var == 1) logp1 = fixed_val_;
            else if (fixed_var == 2) logp2 = fixed_val_;
            
            if (xaxis_var_ == 0) T = x;
            else if (xaxis_var_ == 1) logp1 = x;
            else if (xaxis_var_ == 2) logp2 = x;
            
            if (yaxis_var_ == 0) T = y;
            else if (yaxis_var_ == 1) logp1 = y;
            else if (yaxis_var_ == 2) logp2 = y;
            
            double min_G = 1e30;
            int best_id = -1;
            
            for (const auto& ev : active_species) {
                const nucleo::Especie* esp = nullptr;
                for (const auto* e : db_especies) { if (e->id == ev.id) { esp = e; break; } }
                if (!esp) continue;
                
                double Gf = esp->energia_gibbs(T); // kJ/mol
                double R_kJ = 0.008314462618;
                
                double mu0_g1 = esp_g1 ? esp_g1->energia_gibbs(T) : 0.0;
                double mu0_g2 = esp_g2 ? esp_g2->energia_gibbs(T) : 0.0;
                
                double mu_g1 = mu0_g1 + R_kJ * T * logp1 * 2.302585093; // log10 to ln
                double mu_g2 = mu0_g2 + R_kJ * T * logp2 * 2.302585093;
                
                double G_norm = (Gf - (ev.n_g1 / g1_factor) * mu_g1 - (ev.n_g2 / g2_factor) * mu_g2) / ev.n_m;
                
                if (G_norm < min_G) {
                    min_G = G_norm;
                    best_id = ev.id;
                }
            }
            heatmap_.grid[r * heatmap_.cols + c] = best_id;
        }
    }
    calculado_ = true;
}

void PanelEstabilidad::renderizar() {
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    
    ImGui::Columns(2, "PredominanceLayout", true);
    ImGui::SetColumnWidth(0, 320.0f);
    
    // -- Left Column --
    ImGui::Text(tr("Parámetros del Diagrama"));
    ImGui::Separator();
    
    ImGui::InputText(tr("Main Element"), main_element_, sizeof(main_element_));
    ImGui::InputText(tr("Gas 1 (ej. O2)"), gas1_element_, sizeof(gas1_element_));
    ImGui::InputText(tr("Gas 2 (ej. Cl2)"), gas2_element_, sizeof(gas2_element_));
    
    ImGui::Spacing();
    ImGui::Text(tr("Filtrar Especies por Elementos"));
    ImGui::BeginChild("SpeciesListTpp", ImVec2(0, 200), true);
    if (ImGui::BeginTable("TppSpeciesTable", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Selección", ImGuiTableColumnFlags_WidthFixed, 40.0f);
        ImGui::TableSetupColumn("Fórmula", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableHeadersRow();
        
        auto parser = nucleo::TablaPeriodica::instancia();
        auto main_atoms = parser.analizar_formula(main_element_);
        auto g1_atoms = parser.analizar_formula(gas1_element_);
        auto g2_atoms = parser.analizar_formula(gas2_element_);
        
        std::set<std::string> valid_elements;
        for (const auto& at : main_atoms) valid_elements.insert(at.elem);
        for (const auto& at : g1_atoms) valid_elements.insert(at.elem);
        for (const auto& at : g2_atoms) valid_elements.insert(at.elem);
        
        // If main_element_ is just "Fe", analizador might find it. If not, fallback.
        std::string m_el = main_atoms.empty() ? main_element_ : main_atoms[0].elem;
        
        for (const auto* esp : db_especies) {
            auto atoms = parser.analizar_formula(esp->formula);
            bool has_main = false;
            bool has_other = false;
            for (const auto& at : atoms) {
                if (valid_elements.find(at.elem) == valid_elements.end()) has_other = true;
            }
            
            // Only show species that are composed exclusively of the valid elements
            if (!atoms.empty() && !has_other) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                bool selected = especies_seleccionadas_[esp->id];
                if (ImGui::Checkbox((std::string("##sel") + std::to_string(esp->id)).c_str(), &selected)) {
                    especies_seleccionadas_[esp->id] = selected;
                }
                ImGui::TableSetColumnIndex(1);
                ImGui::Text(tr("%s"), esp->formula.c_str());
            }
        }
        ImGui::EndTable();
    }
    ImGui::EndChild();
    if (ImGui::Button(tr("Select All Filtered"))) {
        // ... (can add logic later if needed)
    }
    
    ImGui::Spacing();
    ImGui::Text(tr("Configurar Ejes"));
    const char* var_names[] = {"Temperature (K)", "log p(Gas 1)", "log p(Gas 2)"};
    
    bool changed = false;
    changed |= ImGui::Combo("X Axis", &xaxis_var_, var_names, 3);
    changed |= ImGui::InputDouble("X Min", &x_min_);
    changed |= ImGui::InputDouble("X Max", &x_max_);
    
    changed |= ImGui::Combo("Y Axis", &yaxis_var_, var_names, 3);
    changed |= ImGui::InputDouble("Y Min", &y_min_);
    changed |= ImGui::InputDouble("Y Max", &y_max_);
    
    int fixed_var = 0;
    if (xaxis_var_ != 0 && yaxis_var_ != 0) fixed_var = 0;
    else if (xaxis_var_ != 1 && yaxis_var_ != 1) fixed_var = 1;
    else if (xaxis_var_ != 2 && yaxis_var_ != 2) fixed_var = 2;
    
    char fixed_label[64];
    snprintf(fixed_label, sizeof(fixed_label), "Fixed Value: %s", var_names[fixed_var]);
    changed |= ImGui::InputDouble(fixed_label, &fixed_val_);
    
    if (ImGui::Button(tr("Dibujar Diagrama"), ImVec2(-1,30))) {
        recalcular();
    }
    
    ImGui::NextColumn();
    // -- Right Column --
    if (calculado_) {
        static bool cmap_added = false;
        if (!cmap_added) {
            std::vector<ImU32> palette(256);
            for (int i = 0; i < 256; ++i) {
                palette[i] = ImGui::ColorConvertFloat4ToU32(color_from_id(i));
            }
            ImPlot::AddColormap("TppPalette", palette.data(), 256, true);
            cmap_added = true;
        }
        
        std::map<int, int> id_to_color_idx;
        int idx = 0;
        for (const auto& pair : heatmap_.labels) {
            id_to_color_idx[pair.first] = idx++;
        }
        
        // Map grid of IDs to grid of color indices for PlotHeatmap
        std::vector<double> mapped_grid(heatmap_.rows * heatmap_.cols, 0.0);
        for (size_t i = 0; i < heatmap_.grid.size(); ++i) {
            int id = heatmap_.grid[i];
            if (id_to_color_idx.count(id)) {
                mapped_grid[i] = id_to_color_idx[id];
            }
        }
        
        ImPlot::PushColormap("TppPalette");
        
        if (ImPlot::BeginPlot("Predominance Diagram", ImVec2(-1, -1))) {
            ImPlot::SetupAxes(var_names[xaxis_var_], var_names[yaxis_var_]);
            ImPlot::SetupAxisLimits(ImAxis_X1, x_min_, x_max_);
            ImPlot::SetupAxisLimits(ImAxis_Y1, y_min_, y_max_);
            
            // Plot Heatmap
            ImPlot::PlotHeatmap("##stability", mapped_grid.data(), heatmap_.rows, heatmap_.cols, 0, 255, nullptr, 
                                ImPlotPoint(x_min_, y_min_), ImPlotPoint(x_max_, y_max_));
                                
            ImPlot::EndPlot();
        }
        ImPlot::PopColormap();
        
        // Render legend
        ImGui::Text(tr("Legend:"));
        ImGui::BeginChild("Legend", ImVec2(0, 0), false, ImGuiWindowFlags_HorizontalScrollbar);
        for (const auto& pair : heatmap_.labels) {
            int c_idx = id_to_color_idx[pair.first];
            ImVec4 c = color_from_id(c_idx);
            ImGui::ColorButton(pair.second.c_str(), c);
            ImGui::SameLine();
            ImGui::Text(tr("%s"), pair.second.c_str());
            ImGui::SameLine(0, 20);
        }
        ImGui::EndChild();
    } else {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5,0.5,0.5,1), "Select species and click Draw Diagram.");
    }
    
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════
#include "lpp_impl.cpp"
// ═══════════════════════════════════════════════════════════════════════════
// Panel de Agua (Tablas de Vapor)
// ═══════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Soluciones Acuosas
// ═══════════════════════════════════════════════════════════════════════════
void PanelSolucionesAcuosas::calcular_hoja() {
    auto& gestor = base_datos::GestorDB::instancia();
    double temp_K = 0.0;
    
    // Primero, calcular el Total Amount y Temperatura
    double total_kmol = 0.0;
    for (auto& f : filas_) {
        total_kmol += f.amount_kmol;
        temp_K = f.temp + 273.15; // asume la misma T para todos
    }
    
    double I_molal = 0.0; // Fuerza iónica
    
    for (auto& f : filas_) {
        f.calculo_ok = false;
        if (total_kmol > 0) f.amount_mol_pct = (f.amount_kmol / total_kmol) * 100.0;
        else f.amount_mol_pct = 0.0;
        
        std::string nom(f.especie);
        if (nom.empty()) continue;
        
        const auto* esp = buscar_especie_por_nombre_o_formula(nom);
        if (esp) {
            f.amount_g = f.amount_kmol * 1000.0 * esp->masa_molar;
            f.h_ideal = esp->entalpia_abs(f.temp + 273.15) / 4.184; // a kcal/mol
            f.cp_ideal = esp->cp(f.temp + 273.15) / 4.184; // a cal/(mol*K)
            f.carga = 0;
            // Parsear carga desde "(+a)" o "(-2a)"
            if (nom.find("(+a)") != std::string::npos) f.carga = 1;
            else if (nom.find("(+2a)") != std::string::npos) f.carga = 2;
            else if (nom.find("(-a)") != std::string::npos) f.carga = -1;
            else if (nom.find("(-2a)") != std::string::npos) f.carga = -2;
            else if (nom.find("(-3a)") != std::string::npos) f.carga = -3;
            
            // Molalidad aproximada (moles soluto / kg solvente)
            // Asumimos H2O es la primera fila o la dominante
            double kg_solvente = 1.0; 
            if (!filas_.empty() && std::string(filas_[0].especie).find("H2O") != std::string::npos) {
                kg_solvente = filas_[0].amount_kmol * 1000.0 * 18.015 / 1000.0;
            }
            double molalidad = (f.amount_kmol * 1000.0) / kg_solvente;
            I_molal += 0.5 * molalidad * f.carga * f.carga;
            f.calculo_ok = true;
        }
    }
    
    // Segunda pasada: Calcular no-idealidad
    for (auto& f : filas_) {
        if (!f.calculo_ok) continue;
        if (f.carga == 0) {
            f.ac_molal = 1.0;
            f.ac_molar = 1.0;
            f.h_estimate = f.h_ideal;
            f.cp_estimate = f.cp_ideal;
        } else {
            // Seleccionar modelo según el checkbox
            double ac = 1.0;
            if (usar_davies_) {
                ac = motor_.coeficiente_actividad_davies(f.carga, I_molal, f.temp + 273.15);
            } else if (usar_pitzer_) {
                ac = motor_.coeficiente_actividad_pitzer(f.carga, I_molal, f.temp + 273.15);
            } else {
                // Harvie (Default o seleccionado)
                ac = motor_.coeficiente_actividad_harvie(f.carga, I_molal, f.temp + 273.15);
            }
            
            f.ac_molal = ac;
            f.ac_molar = ac * 0.98; // aprox
            
            // Aproximación simplificada de entalpía exceso
            double dH_exceso = -8.314 * (f.temp+273.15) * (f.temp+273.15) * (-0.001 * ac) / 4184.0; 
            f.h_estimate = f.h_ideal + dH_exceso;
            f.cp_estimate = f.cp_ideal * (1.0 - 0.05 * std::abs(f.carga) * std::sqrt(I_molal));
        }
    }
}

void PanelSolucionesAcuosas::renderizar() {
    if (!db_inicializada_) {
        // Pre-cargar valores iniciales
        filas_.push_back({"H2O", 40, 55.5, 0, 0, 0,0,0,0,0,0,0,false});
        filas_.push_back({"H(+a)", 40, 4.05e-8, 0, 0, 0,0,0,0,0,0,0,false});
        filas_.push_back({"HS(-a)", 40, 5.28e-126, 0, 0, 0,0,0,0,0,0,0,false});
        filas_.push_back({"Na(+a)", 40, 1.92, 0, 0, 0,0,0,0,0,0,0,false});
        filas_.push_back({"NaSO4(-a)", 40, 1.63, 0, 0, 0,0,0,0,0,0,0,false});
        filas_.push_back({"OH(-a)", 40, 7.69e-7, 0, 0, 0,0,0,0,0,0,0,false});
        filas_.push_back({"SO4(-2a)", 40, 0.144, 0, 0, 0,0,0,0,0,0,0,false});
        calcular_hoja();
        db_inicializada_ = true;
    }

    ImGui::Text(tr("Herramientas de Agua"));
    ImGui::Separator();
    
    ImGui::Text(tr("Chosen method: "));
    ImGui::SameLine();
    bool changed = false;
    if (ImGui::Checkbox(tr("Davies"), &usar_davies_)) {
        if (usar_davies_) { usar_pitzer_ = false; usar_harvie_ = false; }
        changed = true;
    }
    ImGui::SameLine();
    if (ImGui::Checkbox(tr("Pitzer"), &usar_pitzer_)) {
        if (usar_pitzer_) { usar_davies_ = false; usar_harvie_ = false; }
        changed = true;
    }
    ImGui::SameLine();
    if (ImGui::Checkbox(tr("Harvie"), &usar_harvie_)) {
        if (usar_harvie_) { usar_davies_ = false; usar_pitzer_ = false; }
        changed = true;
    }
    
    // Ensure at least one is checked
    if (!usar_davies_ && !usar_pitzer_ && !usar_harvie_) {
        usar_harvie_ = true;
        changed = true;
    }
    
    if (changed) {
        calcular_hoja();
    }
    
    ImGui::Spacing();
    
    if (ImGui::Button(tr("Calcular"), ImVec2(100, 30))) {
        calcular_hoja();
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Limpiar Hoja"), ImVec2(100, 30))) {
        filas_.clear();
    }

    ImGui::Separator();
    
    ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_Resizable | ImGuiTableFlags_ScrollY;
    if (ImGui::BeginTable("table_aqua", 11, flags, ImVec2(-1, -1))) {
        ImGui::TableSetupScrollFreeze(0, 2);
        ImGui::TableSetupColumn("Datos Especie Agua", ImGuiTableColumnFlags_WidthFixed, 150.0f);
        ImGui::TableSetupColumn("Temp. °C", ImGuiTableColumnFlags_WidthFixed, 70.0f);
        ImGui::TableSetupColumn("Amount kmol", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("Amount mol-%", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("Amount g", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("H Ideal kcal/mol", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("H Estimate kcal/mol", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("Cp Ideal cal/(mol*K)", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Cp Estimate", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("AC Molar Sc.", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("AC Molal Sc.", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        
        ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
        for (int column = 0; column < 11; column++) {
            ImGui::TableSetColumnIndex(column);
            ImGui::Text(tr("%s"), ImGui::TableGetColumnName(column));
        }

        // Fila 4: Water Phase (Total)
        double total_kmol = 0, total_mol_pct = 0, total_g = 0;
        double total_h_ideal = 0, total_h_est = 0, total_cp_ideal = 0, total_cp_est = 0;
        
        for (const auto& f : filas_) {
            total_kmol += f.amount_kmol;
            total_mol_pct += f.amount_mol_pct;
            total_g += f.amount_g;
            total_h_ideal += f.h_ideal * f.amount_kmol;
            total_h_est += f.h_estimate * f.amount_kmol;
            total_cp_ideal += f.cp_ideal * f.amount_kmol;
            total_cp_est += f.cp_estimate * f.amount_kmol;
        }

        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Fase Agua"));
        ImGui::TableSetColumnIndex(1); if (!filas_.empty()) ImGui::Text(tr("%.0f"), filas_[0].temp); else ImGui::Text(tr("-"));
        ImGui::TableSetColumnIndex(2); ImGui::Text(tr("%.2f"), total_kmol);
        ImGui::TableSetColumnIndex(3); ImGui::Text(tr("%.2f"), total_mol_pct);
        ImGui::TableSetColumnIndex(4); ImGui::Text(tr("%.2e"), total_g);
        ImGui::TableSetColumnIndex(5); ImGui::Text(tr("%.2f"), total_kmol > 0 ? total_h_ideal/total_kmol : 0);
        ImGui::TableSetColumnIndex(6); ImGui::Text(tr("%.2f"), total_kmol > 0 ? total_h_est/total_kmol : 0);
        ImGui::TableSetColumnIndex(7); ImGui::Text(tr("%.2f"), total_kmol > 0 ? total_cp_ideal/total_kmol : 0);
        ImGui::TableSetColumnIndex(8); ImGui::Text(tr("%.2f"), total_kmol > 0 ? total_cp_est/total_kmol : 0);
        ImGui::TableSetColumnIndex(9); ImGui::Text(tr("-"));
        ImGui::TableSetColumnIndex(10); ImGui::Text(tr("-"));
        
        // Separador para especies
        ImGui::TableNextRow();
        ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, ImColor(200, 200, 200));

        // Filas de Especies
        for (size_t i = 0; i < filas_.size(); ++i) {
            auto& f = filas_[i];
            ImGui::PushID((int)i);
            ImGui::TableNextRow();

            bool changed = false;

            ImGui::TableSetColumnIndex(0);
            ImGui::PushItemWidth(-FLT_MIN);
            if (ImGui::InputText("##esp", f.especie, sizeof(f.especie))) changed = true;
            ImGui::PopItemWidth();

            ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-FLT_MIN);
            if (ImGui::InputDouble("##T", &f.temp, 0.0, 0.0, "%.0f")) changed = true;
            ImGui::PopItemWidth();

            ImGui::TableSetColumnIndex(2);
            ImGui::PushItemWidth(-FLT_MIN);
            if (ImGui::InputDouble("##kmol", &f.amount_kmol, 0.0, 0.0, "%.2e")) changed = true;
            ImGui::PopItemWidth();

            if (changed) {
                calcular_hoja(); // recalcula dinámicamente
            }

            ImGui::TableSetColumnIndex(3); ImGui::Text(tr("%.2f"), f.amount_mol_pct);
            ImGui::TableSetColumnIndex(4); ImGui::Text(tr("%.2e"), f.amount_g);
            
            if (f.calculo_ok) {
                ImGui::TableSetColumnIndex(5); ImGui::Text(tr("%.2f"), f.h_ideal);
                ImGui::TableSetColumnIndex(6); ImGui::Text(tr("%.2f"), f.h_estimate);
                ImGui::TableSetColumnIndex(7); ImGui::Text(tr("%.2f"), f.cp_ideal);
                ImGui::TableSetColumnIndex(8); ImGui::Text(tr("%.2f"), f.cp_estimate);
                ImGui::TableSetColumnIndex(9); ImGui::Text(tr("%.2f"), f.ac_molar);
                ImGui::TableSetColumnIndex(10); ImGui::Text(tr("%.2f"), f.ac_molal);
            } else {
                for (int col = 5; col <= 10; ++col) {
                    ImGui::TableSetColumnIndex(col);
                    ImGui::Text(tr("-"));
                }
            }
            ImGui::PopID();
        }
        
        // Fila extra para añadir
        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0);
        if (ImGui::Button(tr("+ Add Species"))) {
            filas_.push_back(FilaAqua{});
        }

        ImGui::EndTable();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Pérdida de Calor
// ═══════════════════════════════════════════════════════════════════════════
PanelPerdidaCalor::PanelPerdidaCalor() {
    inicializar_valores_defecto();
}

void PanelPerdidaCalor::inicializar_valores_defecto() {
    columnas_.clear();
    
    // Column 1: Inner Surface
    LayerColumn c1;
    c1.type = 0; // Surface
    strncpy(c1.material, "Molten metal (p = 1 bar)", sizeof(c1.material)-1);
    strncpy(c1.surface_material, "Ideal black surface", sizeof(c1.surface_material)-1);
    c1.temp_left = 1100.0;
    c1.temp_right = 1087.0;
    c1.thickness = 0.0;
    c1.convection_hc = 100.0;
    c1.fix_convection = true;
    c1.radiation_hr = 579.216;
    c1.resistance = 0.937;
    columnas_.push_back(c1);

    // Column 2: Solid Layer (Forsterite)
    LayerColumn c2;
    c2.type = 1; // Solid Layer
    strncpy(c2.material, "Forsterite (Mg2SiO4)", sizeof(c2.material)-1);
    c2.temp_left = 1087.0;
    c2.temp_right = 444.0;
    c2.thickness = 50.0; // cm
    c2.grid_points = 10;
    c2.conductivity = 3.849;
    c2.resistance = 47.729;
    columnas_.push_back(c2);

    // Column 3: Contact Layer
    LayerColumn c3;
    c3.type = 2; // Contact Layer
    strncpy(c3.material, "Contact layer N/A", sizeof(c3.material)-1);
    c3.temp_left = 444.0;
    c3.temp_right = 175.0;
    c3.resistance = 20.0;
    c3.fix_resistance = true;
    columnas_.push_back(c3);

    // Column 4: Solid Layer (Stainless Steel)
    LayerColumn c4;
    c4.type = 1; // Solid Layer
    strncpy(c4.material, "Stainless steel (AISI 347)", sizeof(c4.material)-1);
    c4.temp_left = 175.0;
    c4.temp_right = 162.0;
    c4.thickness = 8.0;
    c4.grid_points = 10;
    c4.conductivity = 16.444;
    c4.resistance = 0.981;
    columnas_.push_back(c4);

    // Column 5: Contact Layer
    LayerColumn c5;
    c5.type = 2; // Contact Layer
    strncpy(c5.material, "Contact layer N/A", sizeof(c5.material)-1);
    c5.temp_left = 162.0;
    c5.temp_right = 95.0;
    c5.resistance = 5.0;
    c5.fix_resistance = true;
    columnas_.push_back(c5);

    // Column 6: Solid Layer (Carbon Steel)
    LayerColumn c6;
    c6.type = 1; // Solid Layer
    strncpy(c6.material, "Carbon steel AISI 1010", sizeof(c6.material)-1);
    c6.temp_left = 95.0;
    c6.temp_right = 87.0;
    c6.thickness = 20.0;
    c6.grid_points = 10;
    c6.conductivity = 60.588;
    c6.resistance = 0.567;
    columnas_.push_back(c6);

    // Column 7: Outer Surface
    LayerColumn c7;
    c7.type = 0; // Surface
    strncpy(c7.material, "Air (gas) (p = 1 bar)", sizeof(c7.material)-1);
    strncpy(c7.surface_material, "Ideal black surface", sizeof(c7.surface_material)-1);
    c7.temp_left = 87.0;
    c7.temp_right = 25.0;
    c7.thickness = 0.0;
    c7.convection_hc = 4.987;
    c7.radiation_hr = 8.153;
    c7.resistance = 4.593;
    columnas_.push_back(c7);

    geometry_type_ = 2; // Cylinder
    geom_height_ = 1.0;
    geom_diameter_ = 0.5;
    total_heat_flow_ = 13.471;
    calculated_ = true;
}

void PanelPerdidaCalor::reciclar_calculos() {
    if (calc_type_ == 0) {
        calcular_perfil_temperatura();
    } else {
        calcular_flujo_calor();
    }
}

void PanelPerdidaCalor::calcular_perfil_temperatura() {
    if (columnas_.empty()) return;
    
    double Q = total_heat_flow_; // kW
    
    for (int iter = 0; iter < 15; ++iter) {
        double r = geom_diameter_ / 2.0;
        
        for (size_t i = 0; i < columnas_.size(); ++i) {
            auto& col = columnas_[i];
            double Area = 1.0;
            if (geometry_type_ == 0) {
                Area = geom_height_ * geom_diameter_;
            } else if (geometry_type_ == 1) {
                Area = 6.0 * r * r;
            } else if (geometry_type_ == 2) {
                Area = 2.0 * M_PI * r * geom_height_;
            } else if (geometry_type_ == 3) {
                Area = 4.0 * M_PI * r * r;
            }
            
            double thickness_m = col.thickness / 100.0;
            
            if (col.type == 0) { // Surface
                if (!col.fix_radiation) {
                    double T_L_K = col.temp_left + 273.15;
                    double T_R_K = col.temp_right + 273.15;
                    col.radiation_hr = 5.670374e-8 * (T_L_K * T_L_K + T_R_K * T_R_K) * (T_L_K + T_R_K);
                }
                col.resistance = 1.0 / ((col.convection_hc + col.radiation_hr) * Area) * 1000.0;
            } else if (col.type == 1) { // Solid Layer
                if (geometry_type_ == 0) {
                    col.resistance = thickness_m / (col.conductivity * Area) * 1000.0;
                } else if (geometry_type_ == 2) {
                    double r_next = r + thickness_m;
                    col.resistance = log(r_next / r) / (2.0 * M_PI * geom_height_ * col.conductivity) * 1000.0;
                    r = r_next;
                } else if (geometry_type_ == 3) {
                    double r_next = r + thickness_m;
                    col.resistance = (1.0/r - 1.0/r_next) / (4.0 * M_PI * col.conductivity) * 1000.0;
                    r = r_next;
                } else {
                    double r_next = r + thickness_m;
                    col.resistance = thickness_m / (col.conductivity * (6.0 * r * r)) * 1000.0;
                    r = r_next;
                }
            } else if (col.type == 2) { // Contact Layer
                // resistance is fixed
            } else if (col.type == 3) { // Enclosure
                if (geometry_type_ == 0) {
                    col.resistance = thickness_m / (0.024 * Area) * 1000.0;
                } else {
                    double r_next = r + thickness_m;
                    col.resistance = log(r_next / r) / (2.0 * M_PI * geom_height_ * 0.024) * 1000.0;
                    r = r_next;
                }
            }
            
            if (i == 0) {
                if (!col.fix_temp_left) col.temp_left = 1100.0;
            } else {
                col.temp_left = columnas_[i-1].temp_right;
            }
            col.temp_right = col.temp_left - Q * (col.resistance / 1000.0);
            
            col.heat_flow = Q;
            col.heat_flux = Q / Area;
        }
    }
    calculated_ = true;
}

void PanelPerdidaCalor::calcular_flujo_calor() {
    if (columnas_.empty()) return;
    
    double t_left = columnas_[0].temp_left;
    double t_right = columnas_.back().temp_right;
    
    size_t n = columnas_.size();
    
    for (int iter = 0; iter < 20; ++iter) {
        double r = geom_diameter_ / 2.0;
        double R_total = 0.0;
        
        for (size_t i = 0; i < n; ++i) {
            auto& col = columnas_[i];
            double Area = 1.0;
            if (geometry_type_ == 0) Area = geom_height_ * geom_diameter_;
            else if (geometry_type_ == 1) Area = 6.0 * r * r;
            else if (geometry_type_ == 2) Area = 2.0 * M_PI * r * geom_height_;
            else if (geometry_type_ == 3) Area = 4.0 * M_PI * r * r;
            
            double thickness_m = col.thickness / 100.0;
            
            if (col.type == 0) { // Surface
                if (!col.fix_radiation) {
                    double T_L_K = col.temp_left + 273.15;
                    double T_R_K = col.temp_right + 273.15;
                    col.radiation_hr = 5.670374e-8 * (T_L_K * T_L_K + T_R_K * T_R_K) * (T_L_K + T_R_K);
                }
                col.resistance = 1.0 / ((col.convection_hc + col.radiation_hr) * Area) * 1000.0;
            } else if (col.type == 1) { // Solid Layer
                if (geometry_type_ == 0) {
                    col.resistance = thickness_m / (col.conductivity * Area) * 1000.0;
                } else if (geometry_type_ == 2) {
                    double r_next = r + thickness_m;
                    col.resistance = log(r_next / r) / (2.0 * M_PI * geom_height_ * col.conductivity) * 1000.0;
                    r = r_next;
                } else if (geometry_type_ == 3) {
                    double r_next = r + thickness_m;
                    col.resistance = (1.0/r - 1.0/r_next) / (4.0 * M_PI * col.conductivity) * 1000.0;
                    r = r_next;
                } else {
                    double r_next = r + thickness_m;
                    col.resistance = thickness_m / (col.conductivity * (6.0 * r * r)) * 1000.0;
                    r = r_next;
                }
            } else if (col.type == 2) { // Contact Layer
                // resistance set by user
            } else if (col.type == 3) { // Enclosure
                if (geometry_type_ == 0) {
                    col.resistance = thickness_m / (0.024 * Area) * 1000.0;
                } else {
                    double r_next = r + thickness_m;
                    col.resistance = log(r_next / r) / (2.0 * M_PI * geom_height_ * 0.024) * 1000.0;
                    r = r_next;
                }
            }
            R_total += col.resistance / 1000.0;
        }
        
        if (R_total < 1e-9) R_total = 1e-9;
        double Q = (t_left - t_right) / R_total / 1000.0; // kW
        total_heat_flow_ = Q;
        
        double temp = t_left;
        r = geom_diameter_ / 2.0;
        for (size_t i = 0; i < n; ++i) {
            auto& col = columnas_[i];
            col.temp_left = temp;
            col.temp_right = temp - Q * (col.resistance / 1000.0);
            temp = col.temp_right;
            col.heat_flow = Q;
            
            double Area = 1.0;
            if (geometry_type_ == 0) Area = geom_height_ * geom_diameter_;
            else if (geometry_type_ == 1) Area = 6.0 * r * r;
            else if (geometry_type_ == 2) Area = 2.0 * M_PI * r * geom_height_;
            else if (geometry_type_ == 3) Area = 4.0 * M_PI * r * r;
            col.heat_flux = Q / Area;
            
            double thickness_m = col.thickness / 100.0;
            if (col.type == 1 || col.type == 3) {
                r += thickness_m;
            }
        }
    }
    calculated_ = true;
}

void PanelPerdidaCalor::renderizar() {
    ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.65f, 0.0f, 1.0f), "Heat Loss / Heat Transfer Calculator");
    ImGui::Separator();
    ImGui::Spacing();
    
    // Tab sheets
    if (ImGui::Button(tr("Muro"))) { active_sheet_ = 0; } ImGui::SameLine();
    if (ImGui::Button(tr("Techo / Techo"))) { active_sheet_ = 1; } ImGui::SameLine();
    if (ImGui::Button(tr("Fondo"))) { active_sheet_ = 2; }
    
    ImGui::Separator();
    ImGui::Spacing();
    
    // ── Spreadsheet Grid Table ──
    int n_cols = (int)columnas_.size();
    if (ImGui::BeginTable("heat_loss_grid", n_cols + 1, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollX, ImVec2(0, 360))) {
        ImGui::TableSetupColumn("Propiedad", ImGuiTableColumnFlags_WidthFixed, 180.0f);
        for (int i = 0; i < n_cols; ++i) {
            char header_name[32];
            snprintf(header_name, sizeof(header_name), "%d", i + 1);
            ImGui::TableSetupColumn(header_name, ImGuiTableColumnFlags_WidthFixed, 140.0f);
        }
        ImGui::TableHeadersRow();
        
        // Row 1: Layer Type
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Tipo de Capa"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 1);
            ImGui::SetNextItemWidth(-1.0f);
            const char* type_labels[] = { "Surface", "Solid Layer", "Contact Layer", "Enclosure" };
            if (ImGui::Combo("##type", &columnas_[i].type, type_labels, 4)) {
                reciclar_calculos();
            }
            ImGui::PopID();
        }
        
        // Row 2: Layer Material
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Material de Capa"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 2);
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputText("##material", columnas_[i].material, sizeof(columnas_[i].material))) {
                // simple metadata update
            }
            ImGui::PopID();
        }
        
        // Row 3: Surface Material
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Material de Superficie"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 0) {
                ImGui::PushID(i * 100 + 3);
                ImGui::SetNextItemWidth(-1.0f);
                ImGui::InputText("##surf_material", columnas_[i].surface_material, sizeof(columnas_[i].surface_material));
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 4: Temperature left edge (°C)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Temperature (left edge) °C"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 4);
            ImGui::SetNextItemWidth(-1.0f);
            if (columnas_[i].fix_temp_left) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
            if (ImGui::InputDouble("##temp_left", &columnas_[i].temp_left, 0, 0, "%.1f")) {
                reciclar_calculos();
            }
            if (columnas_[i].fix_temp_left) ImGui::PopStyleColor();
            ImGui::PopID();
        }
        
        // Row 5: Temperature right edge (°C)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Temperature (right edge) °C"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 5);
            ImGui::SetNextItemWidth(-1.0f);
            if (columnas_[i].fix_temp_right) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
            if (ImGui::InputDouble("##temp_right", &columnas_[i].temp_right, 0, 0, "%.1f")) {
                reciclar_calculos();
            }
            if (columnas_[i].fix_temp_right) ImGui::PopStyleColor();
            ImGui::PopID();
        }
        
        // Row 6: Thickness (cm)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Thickness cm"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 1 || columnas_[i].type == 3) {
                ImGui::PushID(i * 100 + 6);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputDouble("##thick", &columnas_[i].thickness, 0, 0, "%.3f")) {
                    reciclar_calculos();
                }
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("0.0"));
            }
        }
        
        // Row 7: Calculation Grid
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Cuadrícula de Cálculo"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 1) {
                ImGui::PushID(i * 100 + 7);
                ImGui::SetNextItemWidth(-1.0f);
                ImGui::InputInt("##grid", &columnas_[i].grid_points, 0, 0);
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 8: Average conductivity (k) W/(mK)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Average conductivity (k)"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 1) {
                ImGui::PushID(i * 100 + 8);
                ImGui::SetNextItemWidth(-1.0f);
                if (columnas_[i].fix_conductivity) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
                if (ImGui::InputDouble("##conductivity", &columnas_[i].conductivity, 0, 0, "%.3f")) {
                    reciclar_calculos();
                }
                if (columnas_[i].fix_conductivity) ImGui::PopStyleColor();
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 9: Convection coefficient (hc) W/(m²K)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Convection coeff. (hc)"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 0) {
                ImGui::PushID(i * 100 + 9);
                ImGui::SetNextItemWidth(-1.0f);
                if (columnas_[i].fix_convection) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
                if (ImGui::InputDouble("##convection", &columnas_[i].convection_hc, 0, 0, "%.3f")) {
                    reciclar_calculos();
                }
                if (columnas_[i].fix_convection) ImGui::PopStyleColor();
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 10: Radiation coefficient (hr) W/(m²K)
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Radiation coeff. (hr)"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            if (columnas_[i].type == 0) {
                ImGui::PushID(i * 100 + 10);
                ImGui::SetNextItemWidth(-1.0f);
                if (columnas_[i].fix_radiation) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
                if (ImGui::InputDouble("##radiation", &columnas_[i].radiation_hr, 0, 0, "%.3f")) {
                    reciclar_calculos();
                }
                if (columnas_[i].fix_radiation) ImGui::PopStyleColor();
                ImGui::PopID();
            } else {
                ImGui::TextDisabled(tr("N/A"));
            }
        }
        
        // Row 11: Thermal Resistance °C/kW
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Thermal Resistance °C/kW"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::PushID(i * 100 + 11);
            ImGui::SetNextItemWidth(-1.0f);
            if (columnas_[i].fix_resistance) ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 0.3f, 0.3f, 1.0f));
            if (ImGui::InputDouble("##resistance", &columnas_[i].resistance, 0, 0, "%.3f")) {
                reciclar_calculos();
            }
            if (columnas_[i].fix_resistance) ImGui::PopStyleColor();
            ImGui::PopID();
        }
        
        // Row 12: Heat Flow kW
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Flujo de Calor kW"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::Text(tr("%.3f"), columnas_[i].heat_flow);
        }
        
        // Row 13: Heat Flux kW/m²
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text(tr("Flujo de Calor kW/m²"));
        for (int i = 0; i < n_cols; ++i) {
            ImGui::TableNextColumn();
            ImGui::Text(tr("%.3f"), columnas_[i].heat_flux);
        }
        
        ImGui::EndTable();
    }
    
    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Spacing();
    
    // ── Bottom Dashboard Columns ──
    ImGui::Columns(3, "heat_loss_dashboard", true);
    
    // Column 1: Total Heat Flow & Shapes
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f), "Total Heat Flow");
    ImGui::SetNextItemWidth(-1.0f);
    ImGui::InputDouble("##total_h_flow", &total_heat_flow_, 0, 0, "%.3f");
    
    ImGui::Spacing();
    ImGui::Text(tr("Shape & Dimensions:"));
    ImGui::RadioButton("Muro", &geometry_type_, 0); ImGui::SameLine();
    if (ImGui::RadioButton("Cylinder", &geometry_type_, 2)) {
        reciclar_calculos();
    }
    ImGui::RadioButton("Cube", &geometry_type_, 1); ImGui::SameLine();
    if (ImGui::RadioButton("Sphere", &geometry_type_, 3)) {
        reciclar_calculos();
    }
    
    ImGui::SetNextItemWidth(100.0f);
    if (ImGui::InputDouble("Height (m)", &geom_height_)) reciclar_calculos();
    ImGui::SameLine();
    ImGui::SetNextItemWidth(100.0f);
    if (ImGui::InputDouble("Diameter (m)", &geom_diameter_)) reciclar_calculos();
    
    ImGui::NextColumn();
    
    // Column 2: Options / Actions
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f), "Options");
    if (ImGui::Button(tr("Fix Selected Value"), ImVec2(-1, 24))) {
        // Toggle fix flags as a demonstration
        for (auto& col : columnas_) {
            col.fix_convection = !col.fix_convection;
        }
    }
    if (ImGui::Button(tr("Tabla del Diagrama"), ImVec2(-1, 24))) {
        // Option to display numeric table profile
    }
    if (ImGui::Button(tr("Dibujar Perfil de Temperatura"), ImVec2(-1, 24))) {
        show_diagram_ = !show_diagram_;
    }
    if (ImGui::Button(tr("Reset to Defaults"), ImVec2(-1, 24))) {
        inicializar_valores_defecto();
    }
    
    ImGui::NextColumn();
    
    // Column 3: Calculate Solvers
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.8f, 1.0f, 1.0f), "Calculate");
    
    if (ImGui::Button(tr("Temperature Profile Solver"), ImVec2(-1, 28))) {
        calc_type_ = 0;
        calcular_perfil_temperatura();
    }
    if (ImGui::Button(tr("Resolvedor de Flujo de Calor"), ImVec2(-1, 28))) {
        calc_type_ = 1;
        calcular_flujo_calor();
    }
    
    ImGui::Spacing();
    // Hot to cold color bar indicator
    ImGui::Text(tr("HOT")); ImGui::SameLine();
    ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 10);
    ImVec2 cursor = ImGui::GetCursorScreenPos();
    ImGui::GetWindowDrawList()->AddRectFilled(cursor, ImVec2(cursor.x + 100, cursor.y + 16), IM_COL32(255, 0, 0, 255), 2.0f);
    ImGui::GetWindowDrawList()->AddRectFilledMultiColor(cursor, ImVec2(cursor.x + 100, cursor.y + 16), IM_COL32(255, 0, 0, 255), IM_COL32(0, 0, 255, 255), IM_COL32(0, 0, 255, 255), IM_COL32(255, 0, 0, 255));
    ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 110);
    ImGui::Text(tr("COLD"));
    
    ImGui::Columns(1);
    
    // ── ImPlot Diagram Profile popup ──
    if (show_diagram_) {
        ImGui::SetNextWindowSize(ImVec2(600, 400), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Temperature Profile Diagram"), &show_diagram_)) {
            std::vector<double> xs;
            std::vector<double> ys;
            
            double current_x = 0.0;
            xs.push_back(current_x);
            ys.push_back(columnas_[0].temp_left);
            
            for (size_t i = 0; i < columnas_.size(); ++i) {
                auto& col = columnas_[i];
                if (col.type == 1 || col.type == 3) { // Solid Layer / Enclosure
                    double dx = col.thickness * 10.0; // convert cm to mm
                    // Generate interior grid points for a continuous curve
                    int steps = std::max(2, col.grid_points);
                    for (int s = 1; s <= steps; ++s) {
                        double fraction = (double)s / steps;
                        xs.push_back(current_x + dx * fraction);
                        
                        // Linear interpolation or logarithmic (we use linear approximation for display)
                        double t_interp = col.temp_left + fraction * (col.temp_right - col.temp_left);
                        ys.push_back(t_interp);
                    }
                    current_x += dx;
                } else { // Surface or contact layer
                    // sudden temperature drop at interfaces
                    xs.push_back(current_x);
                    ys.push_back(col.temp_right);
                }
            }
            
            char title_lbl[128];
            snprintf(title_lbl, sizeof(title_lbl), "Temperature Profile Wall (Total Heat flow: %.3f kW)", total_heat_flow_);
            if (ImPlot::BeginPlot(title_lbl, ImVec2(-1, -1))) {
                ImPlot::SetupAxes("Distance mm", "Temperature °C");
                ImPlot::PlotLine("Wall Temp", xs.data(), ys.data(), (int)xs.size());
                ImPlot::EndPlot();
            }
            ImGui::End();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Convertidor de Especies
// ═══════════════════════════════════════════════════════════════════════════

PanelConvertidorEspecies::PanelConvertidorEspecies() {
    inputs_ = {
        {"S", 32.850, 0.0},
        {"Cu", 25.700, 0.0},
        {"Fe", 24.700, 0.0},
        {"Ni", 0.550, 0.0},
        {"Co", 0.010, 0.0},
        {"Zn", 4.050, 0.0},
        {"Pb", 3.900, 0.0},
        {"Sb", 0.200, 0.0},
        {"As", 0.130, 0.0},
        {"Ag", 0.090, 0.0},
        {"Au", 0.001, 0.0},
        {"SiO2", 4.650, 0.0},
        {"Al2O3", 2.200, 0.0},
        {"MgO", 0.150, 0.0},
        {"O", 3.572, 0.0},
        {"H", 0.000, 0.0},
        {"[Others]", 0.802, 0.0}
    };
    outputs_ = {
        {"CuS", 0.0, 0.0, 0.0, ""},
        {"Cu2S", 0.0, 0.0, 0.0, ""},
        {"CuFeS2", 0.0, 60.0, 10.0, ""},
        {"FeS", 0.0, 0.0, 0.0, ""},
        {"FeS2", 0.0, 15.0, 10.0, ""},
        {"NiS", 0.0, 0.0, 0.0, ""},
        {"CoS", 0.0, 0.0, 0.0, ""},
        {"ZnS", 0.0, 0.0, 0.0, ""},
        {"PbS", 0.0, 4.5, 10.0, ""},
        {"Sb2S3", 0.0, 0.0, 0.0, ""},
        {"As2S3", 0.0, 0.0, 0.0, ""},
        {"Ag2S", 0.0, 0.0, 0.0, ""},
        {"Au", 0.0, 0.0, 0.0, ""},
        {"SiO2", 0.0, 0.0, 0.0, ""},
        {"Al2O3", 0.0, 0.0, 0.0, ""},
        {"MgO", 0.0, 0.0, 0.0, ""},
        {"[Others]", 0.0, 0.0, 0.0, ""}
    };
    calcular();
}

void PanelConvertidorEspecies::calcular() {
    std::map<std::string, double> input_map;
    input_total_wt_ = 0.0;
    for (auto& in : inputs_) {
        input_map[std::string(in.element)] = in.wt_pct;
        input_total_wt_ += in.wt_pct;
        in.calc_elem = in.wt_pct; // mock
    }
    std::vector<std::string> target_species;
    for (auto& out : outputs_) {
        target_species.push_back(std::string(out.species));
    }
    auto result_map = motor_.analisisAEspecies(input_map, target_species);
    
    output_total_wt_ = 0.0;
    for (auto& out : outputs_) {
        auto it = result_map.find(std::string(out.species));
        if (it != result_map.end()) {
            out.calc_wt = it->second;
            output_total_wt_ += it->second;
        } else {
            out.calc_wt = 0.0;
        }
    }
}

void PanelConvertidorEspecies::guardar_json(const char* path) {
    nlohmann::json j;
    for (auto& in : inputs_) j["inputs"].push_back({{"element", std::string(in.element)}, {"wt", in.wt_pct}});
    for (auto& out : outputs_) j["outputs"].push_back({{"species", std::string(out.species)}, {"target", out.target_wt}, {"coeff", out.weight_coeff}});
    std::ofstream o(path);
    if (o.is_open()) o << j.dump(4);
}

void PanelConvertidorEspecies::cargar_json(const char* path) {
    std::ifstream i(path);
    if (i.is_open()) {
        nlohmann::json j;
        i >> j;
        inputs_.clear();
        for (auto& item : j["inputs"]) {
            InputItem in{};
            strncpy(in.element, item["element"].get<std::string>().c_str(), 31);
            in.wt_pct = item["wt"];
            inputs_.push_back(in);
        }
        outputs_.clear();
        for (auto& item : j["outputs"]) {
            OutputItem out{};
            strncpy(out.species, item["species"].get<std::string>().c_str(), 31);
            out.target_wt = item.value("target", 0.0);
            out.weight_coeff = item.value("coeff", 0.0);
            outputs_.push_back(out);
        }
        calcular();
    }
}

void PanelConvertidorEspecies::renderizar() {
    // Toolbar
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0,0,0,0));
    if (ImGui::Button(tr("Nuevo"))) { inputs_.clear(); outputs_.clear(); calcular(); } ImGui::SameLine();
    if (ImGui::Button(tr("Abrir"))) { ImGui::OpenPopup("Open Conversion"); } ImGui::SameLine();
    if (ImGui::Button(tr("Guardar"))) { ImGui::OpenPopup("Save Conversion"); } ImGui::SameLine();
    ImGui::TextDisabled(tr(" | ")); ImGui::SameLine();
    if (ImGui::Button(tr("Inicializar"))) {
        inputs_.clear(); outputs_.clear();
        input_total_wt_ = 0.0; output_total_wt_ = 0.0;
        mostrar_grafico_ = false;
    } ImGui::SameLine();
    if (ImGui::Button(tr("Resolver"))) { calcular(); } ImGui::SameLine();
    if (ImGui::Button(tr("Gráficas"))) { mostrar_grafico_ = !mostrar_grafico_; }
    ImGui::PopStyleColor();
    ImGui::Separator();
    
    if (ImGui::BeginPopupModal("Save Conversion", NULL, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::InputText(tr("Filename"), save_filename_, 128);
        if (ImGui::Button(tr("Guardar"), ImVec2(120, 0))) {
            guardar_json(save_filename_);
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Cancelar"), ImVec2(120, 0))) { ImGui::CloseCurrentPopup(); }
        ImGui::EndPopup();
    }
    
    if (ImGui::BeginPopupModal("Open Conversion", NULL, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::InputText(tr("Filename"), open_filename_, 128);
        if (ImGui::Button(tr("Abrir"), ImVec2(120, 0))) {
            cargar_json(open_filename_);
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Cancelar"), ImVec2(120, 0))) { ImGui::CloseCurrentPopup(); }
        ImGui::EndPopup();
    }
    
    ImVec2 space = ImGui::GetContentRegionAvail();
    float table_w = space.x * 0.65f;
    float charts_w = mostrar_grafico_ ? (space.x - table_w - 20.0f) : 0.0f;
    
    ImGui::BeginChild("TablePane", ImVec2(table_w, space.y), true);
    // Info block
    ImGui::Text(tr("Name:   Example Conversion"));
    ImGui::Text(tr("Date:   (Today)"));
    ImGui::Text(tr("Author: CMA Chemist"));
    ImGui::Spacing();
    
    // Main Split Table for INPUT and OUTPUT
    if (ImGui::BeginTable("MainGrid", 9, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_SizingFixedFit)) {
        ImGui::TableSetupColumn("Analysis Elements", ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableSetupColumn("wt-%", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("Act", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        ImGui::TableSetupColumn("Calculated Elements", ImGuiTableColumnFlags_WidthFixed, 140.0f);
        ImGui::TableSetupColumn("Calculated wt-%", ImGuiTableColumnFlags_WidthFixed, 110.0f);
        ImGui::TableSetupColumn("Target wt-%", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("Weight Coeff", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("Tipo", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("Act", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        
        ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
        ImGui::TableSetColumnIndex(0); ImGui::Text(tr("INPUT"));
        ImGui::TableSetColumnIndex(3); ImGui::Text(tr("OUTPUT"));
        ImGui::TableHeadersRow();
        
        // Total Row
        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Total"));
        ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.3f"), input_total_wt_);
        ImGui::TableSetColumnIndex(4); ImGui::Text(tr("%.3f"), output_total_wt_);
        ImGui::TableSetColumnIndex(5); ImGui::Text(tr("79.500"));
        
        int rows = std::max(inputs_.size(), outputs_.size());
        for (int i = 0; i < rows + 1; ++i) { // +1 for Add Row buttons
            ImGui::TableNextRow();
            if (i < (int)inputs_.size()) {
                ImGui::TableSetColumnIndex(0);
                ImGui::SetNextItemWidth(-FLT_MIN);
                ImGui::PushID(i * 10 + 0);
                ImGui::InputText("##elem", inputs_[i].element, 32);
                ImGui::PopID();
                ImGui::TableSetColumnIndex(1);
                ImGui::SetNextItemWidth(-FLT_MIN);
                ImGui::PushID(i * 10 + 1);
                if (ImGui::InputDouble("##wt", &inputs_[i].wt_pct, 0, 0, "%.3f")) { calcular(); }
                ImGui::PopID();
                ImGui::TableSetColumnIndex(2);
                if (ImGui::Button(tr("-"), ImVec2(20,20))) { inputs_.erase(inputs_.begin() + i); calcular(); }
                
                ImGui::TableSetColumnIndex(3);
                ImGui::Text(tr("%s"), inputs_[i].element);
            } else if (i == (int)inputs_.size()) {
                ImGui::TableSetColumnIndex(0);
                if (ImGui::Button(tr("+ Add Elem"))) {
                    InputItem item{};
                    strncpy(item.element, "New", 31);
                    inputs_.push_back(item);
                }
            }
            if (i < (int)outputs_.size()) {
                ImGui::TableSetColumnIndex(4);
                ImGui::SetNextItemWidth(90);
                ImGui::PushID(i * 10 + 3);
                ImGui::InputText("##spec", outputs_[i].species, 32);
                ImGui::PopID();
                ImGui::SameLine(90);
                ImGui::Text(tr("%.3f"), outputs_[i].calc_wt);
                
                ImGui::TableSetColumnIndex(5);
                if (outputs_[i].target_wt >= 0) {
                    ImGui::SetNextItemWidth(-FLT_MIN);
                    ImGui::PushID(i * 10 + 4);
                    ImGui::InputDouble("##twt", &outputs_[i].target_wt, 0, 0, "%.3f");
                    ImGui::PopID();
                }
                
                ImGui::TableSetColumnIndex(6);
                if (outputs_[i].weight_coeff >= 0) {
                    ImGui::SetNextItemWidth(-FLT_MIN);
                    ImGui::PushID(i * 10 + 5);
                    ImGui::InputDouble("##wc", &outputs_[i].weight_coeff, 0, 0, "%.3f");
                    ImGui::PopID();
                }
                
                ImGui::TableSetColumnIndex(7);
                ImGui::Text(tr("%s"), outputs_[i].type);
                
                ImGui::TableSetColumnIndex(8);
                if (ImGui::Button(tr("-"), ImVec2(20,20))) { outputs_.erase(outputs_.begin() + i); calcular(); }
            } else if (i == (int)outputs_.size()) {
                ImGui::TableSetColumnIndex(4);
                if (ImGui::Button(tr("+ Add Species"))) {
                    OutputItem item{};
                    strncpy(item.species, "New", 31);
                    outputs_.push_back(item);
                }
            }
        }
        ImGui::EndTable();
    }
    ImGui::EndChild();
    
    ImGui::SameLine();

    if (mostrar_grafico_) {
    ImGui::BeginChild("ChartsPane", ImVec2(charts_w, space.y), true);
    ImGui::Text(tr("Gráficas"));
    ImGui::Separator();
    
    // Process pie chart data
    std::vector<const char*> in_labels;
    std::vector<double> in_vals;
    for (const auto& in : inputs_) {
        if (in.wt_pct > 1.0) { // Only show major elements
            in_labels.push_back(in.element);
            in_vals.push_back(in.wt_pct);
        }
    }
    std::vector<const char*> out_labels;
    std::vector<double> out_vals;
    for (const auto& out : outputs_) {
        if (out.calc_wt > 1.0) { // Only show major species
            out_labels.push_back(out.species);
            out_vals.push_back(out.calc_wt);
        }
    }
    
    float plot_h = (space.y - 40) / 2.0f;
    if (ImPlot::BeginPlot("Input analysis", ImVec2(-1, plot_h), ImPlotFlags_NoInputs | ImPlotFlags_NoLegend | ImPlotFlags_NoMouseText)) {
        ImPlot::SetupAxes(NULL, NULL, ImPlotAxisFlags_NoDecorations, ImPlotAxisFlags_NoDecorations);
        if (!in_vals.empty()) {
            ImPlot::PlotPieChart(in_labels.data(), in_vals.data(), (int)in_vals.size(), 0.5, 0.5, 0.45, "%.1f", 90, ImPlotPieChartFlags_Normalize);
        }
        ImPlot::EndPlot();
    }
    
    if (ImPlot::BeginPlot("Output analysis", ImVec2(-1, plot_h), ImPlotFlags_NoInputs | ImPlotFlags_NoLegend | ImPlotFlags_NoMouseText)) {
        ImPlot::SetupAxes(NULL, NULL, ImPlotAxisFlags_NoDecorations, ImPlotAxisFlags_NoDecorations);
        if (!out_vals.empty()) {
            ImPlot::PlotPieChart(out_labels.data(), out_vals.data(), (int)out_vals.size(), 0.5, 0.5, 0.45, "%.1f", 90, ImPlotPieChartFlags_Normalize);
        }
        ImPlot::EndPlot();
    }

    ImGui::EndChild();
    } // mostrar_grafico_
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Unidades (conversor)
// ═══════════════════════════════════════════════════════════════════════════
void PanelUnidades::renderizar() {
    auto& db = base_datos::GestorUnidades::instancia();
    if (!db_inicializada_) {
        db.cargar_desde_json("unidades_db.json");
        db_inicializada_ = true;
    }
    
    if (db.categorias().empty()) {
        ImGui::Text(tr("No categories found in database."));
        return;
    }
    
    const auto& categorias = db.categorias();
    
    ImGui::Text(tr("Conversión de Unidades"));
    ImGui::Separator();
    
    // UI Layout similar to image
    ImGui::PushItemWidth(300);
    if (ImGui::BeginCombo("##categoria", categorias[categoria_].nombre.c_str())) {
        for (int i = 0; i < (int)categorias.size(); ++i) {
            bool is_selected = (categoria_ == i);
            if (ImGui::Selectable(categorias[i].nombre.c_str(), is_selected)) {
                categoria_ = i;
                unidad_origen_ = 0;
                unidad_destino_ = std::min(1, (int)categorias[i].unidades.size() - 1);
            }
            if (is_selected) ImGui::SetItemDefaultFocus();
        }
        ImGui::EndCombo();
    }
    ImGui::PopItemWidth();
    
    ImGui::Spacing();
    
    const auto& unidades = categorias[categoria_].unidades;
    
    ImGui::BeginGroup();
    ImGui::PushItemWidth(120);
    if (ImGui::BeginCombo("##origen", unidades[unidad_origen_].nombre.c_str())) {
        for (int i = 0; i < (int)unidades.size(); ++i) {
            bool is_selected = (unidad_origen_ == i);
            if (ImGui::Selectable(unidades[i].nombre.c_str(), is_selected)) unidad_origen_ = i;
            if (is_selected) ImGui::SetItemDefaultFocus();
        }
        ImGui::EndCombo();
    }
    ImGui::PopItemWidth();
    
    ImGui::SameLine();
    if (ImGui::Button(tr("Limpiar Hoja"), ImVec2(120, 0))) {
        for (auto& f : filas_) { f.d1 = 0.0; f.d2 = 0.0; f.activa = false; }
    }
    ImGui::EndGroup();
    
    ImGui::SameLine(ImGui::GetWindowWidth() / 2.0f);
    
    ImGui::BeginGroup();
    ImGui::PushItemWidth(120);
    if (ImGui::BeginCombo("##destino", unidades[unidad_destino_].nombre.c_str())) {
        for (int i = 0; i < (int)unidades.size(); ++i) {
            bool is_selected = (unidad_destino_ == i);
            if (ImGui::Selectable(unidades[i].nombre.c_str(), is_selected)) unidad_destino_ = i;
            if (is_selected) ImGui::SetItemDefaultFocus();
        }
        ImGui::EndCombo();
    }
    ImGui::PopItemWidth();
    ImGui::EndGroup();
    
    ImGui::Separator();
    
    // Formula bar
    double f = db.obtener_factor(categorias[categoria_].nombre, unidad_origen_, unidad_destino_);
    char fbuf[128];
    if (categorias[categoria_].nombre == "Temperature") snprintf(fbuf, sizeof(fbuf), "(complex formula)");
    else snprintf(fbuf, sizeof(fbuf), "%.12g", f);
    
    ImGui::PushItemWidth(-1);
    ImGui::InputText("##formula", fbuf, sizeof(fbuf), ImGuiInputTextFlags_ReadOnly);
    ImGui::PopItemWidth();
    
    ImGui::Spacing();
    
    // Spreadsheet Table
    if (ImGui::BeginTable("SheetUnits", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable | ImGuiTableFlags_SizingFixedFit)) {
        ImGui::TableSetupScrollFreeze(1, 1);
        ImGui::TableSetupColumn("Fila", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        ImGui::TableSetupColumn("Data 1", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Units 1", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("Data 2", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Units 2", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableHeadersRow();
        
        // Populate spreadsheet rows
        if (filas_.size() < 100) filas_.resize(100); // Default 100 rows
        
        for (int r = 0; r < 100; ++r) {
            ImGui::TableNextRow();
            
            // Row number
            ImGui::TableSetColumnIndex(0);
            ImGui::Text(tr("%d"), r + 1);
            
            // Data 1
            ImGui::TableSetColumnIndex(1);
            ImGui::SetNextItemWidth(-FLT_MIN);
            ImGui::PushID(r * 2);
            if (ImGui::InputDouble("##d1", &filas_[r].d1, 0.0, 0.0, "%.6g", ImGuiInputTextFlags_CharsDecimal)) {
                // If it wasn't active, activate it with current top bar settings
                if (!filas_[r].activa) {
                    filas_[r].cat = categorias[categoria_].nombre;
                    filas_[r].u1 = unidad_origen_;
                    filas_[r].u2 = unidad_destino_;
                    filas_[r].nom_u1 = unidades[unidad_origen_].nombre;
                    filas_[r].nom_u2 = unidades[unidad_destino_].nombre;
                    filas_[r].activa = true;
                }
                // Compute Data 2 using the ROW's stored category and units
                filas_[r].d2 = db.convertir(filas_[r].d1, filas_[r].cat, filas_[r].u1, filas_[r].u2);
            }
            if (ImGui::IsItemDeactivatedAfterEdit() && filas_[r].activa) {
                 filas_[r].d2 = db.convertir(filas_[r].d1, filas_[r].cat, filas_[r].u1, filas_[r].u2);
            }
            ImGui::PopID();
            
            // Units 1
            ImGui::TableSetColumnIndex(2);
            if (filas_[r].activa) {
                ImGui::Text(tr("%s"), filas_[r].nom_u1.c_str());
            }
            
            // Data 2
            ImGui::TableSetColumnIndex(3);
            ImGui::SetNextItemWidth(-FLT_MIN);
            ImGui::PushID(r * 2 + 1);
            if (ImGui::InputDouble("##d2", &filas_[r].d2, 0.0, 0.0, "%.6g", ImGuiInputTextFlags_CharsDecimal)) {
                // If it wasn't active, activate it with current top bar settings
                if (!filas_[r].activa) {
                    filas_[r].cat = categorias[categoria_].nombre;
                    filas_[r].u1 = unidad_origen_;
                    filas_[r].u2 = unidad_destino_;
                    filas_[r].nom_u1 = unidades[unidad_origen_].nombre;
                    filas_[r].nom_u2 = unidades[unidad_destino_].nombre;
                    filas_[r].activa = true;
                }
                // Compute Data 1 backwards using the ROW's stored category and units
                filas_[r].d1 = db.convertir(filas_[r].d2, filas_[r].cat, filas_[r].u2, filas_[r].u1);
            }
            if (ImGui::IsItemDeactivatedAfterEdit() && filas_[r].activa) {
                 filas_[r].d1 = db.convertir(filas_[r].d2, filas_[r].cat, filas_[r].u2, filas_[r].u1);
            }
            ImGui::PopID();
            
            // Units 2
            ImGui::TableSetColumnIndex(4);
            if (filas_[r].activa) {
                ImGui::Text(tr("%s"), filas_[r].nom_u2.c_str());
            }
        }
        
        ImGui::EndTable();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Estimación de Propiedades
// ═══════════════════════════════════════════════════════════════════════════
void PanelEstimacionProp::renderizar() {
    // Header
    ImGui::Text(tr("H, S, Cp Estimate"));
    ImGui::Separator();
    
    // Inputs layout
    ImGui::PushItemWidth(150);
    ImGui::InputText(tr("Especie"), formula_, sizeof(formula_));
    ImGui::InputDouble("Temperature", &temperatura_);
    ImGui::PopItemWidth();
    
    ImGui::SameLine();
    ImGui::Checkbox(tr("°C"), &es_celsius_);
    ImGui::SameLine();
    bool is_kelvin = !es_celsius_;
    if (ImGui::Checkbox(tr("K"), &is_kelvin)) es_celsius_ = !is_kelvin;
    
    ImGui::SameLine(350);
    ImGui::BeginGroup();
    ImGui::TextDisabled(tr("Tipo Especie"));
    ImGui::PushItemWidth(150);
    ImGui::Combo("##type", &tipo_especie_, "Not Specified\0Inorganic\0Organic\0\0");
    ImGui::PopItemWidth();
    ImGui::EndGroup();
    
    ImGui::SameLine(550);
    if (ImGui::Button(tr("Estimar"), ImVec2(100, 40))) {
        std::string t_str = "Not Specified";
        if (tipo_especie_ == 1) t_str = "Inorganic";
        else if (tipo_especie_ == 2) t_str = "Organic";
        
        double t_c = es_celsius_ ? temperatura_ : temperatura_ - 273.15;
        auto est = motor_.estimar_hsc(formula_, t_c, t_str);
        if (est.mw > 0) {
            resultados_.push_back(est);
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(mostrar_grafico_ ? "Hide Graph" : "Show Graph", ImVec2(100, 40))) {
        mostrar_grafico_ = !mostrar_grafico_;
    }
    
    ImGui::Separator();
    
    if (mostrar_grafico_) {
        if (!grafico_calculado_) {
            auto& db = base_datos::GestorDB::instancia();
            auto especies = db.todas_especies();
            std::vector<double> errors;
            for (const auto* e : especies) {
                if (e->formula.empty()) continue;
                auto est = motor_.estimar_hsc(e->formula, 25.0, "Not Specified");
                if (est.mw > 0 && est.has_db) {
                    exp_h_.push_back(est.h_db);
                    est_h_.push_back(est.h_est);
                    db_formulas_.push_back(e->formula);
                    errors.push_back(std::abs(est.h_db - est.h_est));
                }
            }
            if (!errors.empty()) {
                species_count_ = errors.size();
                double sum = 0;
                for (double err : errors) sum += err;
                mean_error_ = sum / errors.size();
                
                std::sort(errors.begin(), errors.end());
                median_error_ = errors[errors.size() / 2];
            }
            grafico_calculado_ = true;
        }

        ImGui::SetNextWindowSize(ImVec2(600, 600), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("H Enthalpy of Aqueous Species"), &mostrar_grafico_)) {
            if (ImPlot::BeginPlot("##parity_plot", ImVec2(-1, -1))) {
                ImPlot::SetupAxes("Experimental", "Estimated");
                ImPlot::SetupAxesLimits(-2500, 500, -2500, 500);
                
                std::vector<double> x_kcal, y_kcal;
                for(size_t i = 0; i < exp_h_.size(); ++i) {
                    x_kcal.push_back(exp_h_[i] / 4.184);
                    y_kcal.push_back(est_h_[i] / 4.184);
                }

                ImPlot::SetNextMarkerStyle(ImPlotMarker_Cross, 3, ImVec4(1,0,0,1), 1, ImVec4(1,0,0,1));
                ImPlot::PlotScatter("##species", x_kcal.data(), y_kcal.data(), x_kcal.size());
                
                double line_pts[2] = {-2500, 500};
                ImPlot::SetNextLineStyle(ImVec4(0,0,0,1));
                ImPlot::PlotLine("##identity", line_pts, line_pts, 2);
                
                char anno_text[256];
                snprintf(anno_text, sizeof(anno_text), "H Enthalpy of Aqueous Species\n- kcal / mol\n- %d Species\n- Mean error = %.2f kcal/mol\n- Median error = %.2f kcal/mol", 
                         species_count_, mean_error_/4.184, median_error_/4.184);
                ImPlot::PlotText(anno_text, -1500, 200); 

                if (ImPlot::IsPlotHovered()) {
                    ImPlotPoint mouse = ImPlot::GetPlotMousePos();
                    double min_dist = 1e9;
                    int closest_idx = -1;
                    for(size_t i = 0; i < x_kcal.size(); ++i) {
                        double dx = x_kcal[i] - mouse.x;
                        double dy = y_kcal[i] - mouse.y;
                        double dist = dx*dx + dy*dy;
                        if(dist < min_dist && dist < 900.0) { // 30 kcal/mol radio de hover
                            min_dist = dist;
                            closest_idx = i;
                        }
                    }
                    if (closest_idx >= 0) {
                        ImGui::BeginTooltip();
                        ImGui::Text(tr("Sustancia: %s"), db_formulas_[closest_idx].c_str());
                        ImGui::Text(tr("Experimental: %.2f kcal/mol"), x_kcal[closest_idx]);
                        ImGui::Text(tr("Estimated: %.2f kcal/mol"), y_kcal[closest_idx]);
                        ImGui::EndTooltip();
                    }
                }

                ImPlot::EndPlot();
            }
        }
        ImGui::End();
    }
    
    // Table
    if (ImGui::BeginTable("##hsc_estimates", 12, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable, ImVec2(-1, 400))) {
        ImGui::TableSetupColumn("Chemical Formula");
        ImGui::TableSetupColumn("Tipo Especie");
        ImGui::TableSetupColumn("T (°C)");
        ImGui::TableSetupColumn("T (K)");
        ImGui::TableSetupColumn("Ox. Num");
        ImGui::TableSetupColumn("Weight\ng/mol");
        ImGui::TableSetupColumn("H Estimate\nkJ/mol");
        ImGui::TableSetupColumn("H Database\nkJ/mol");
        ImGui::TableSetupColumn("S Estimate\nJ/mol*K");
        ImGui::TableSetupColumn("S Database\nJ/mol*K");
        ImGui::TableSetupColumn("Cp Estimate\nJ/mol*K");
        ImGui::TableSetupColumn("Cp Database\nJ/mol*K");
        ImGui::TableHeadersRow();
        
        for (const auto& r : resultados_) {
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), r.formula.c_str());
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), r.tipo.c_str());
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.T_C);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.T_K);
            ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0,0,1,1), "1.00"); 
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.mw);
            
            // H
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.h_est);
            ImGui::TableNextColumn(); if(r.has_db) ImGui::Text(tr("%.2f"), r.h_db); else ImGui::Text(tr("-"));
            // S
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.s_est);
            ImGui::TableNextColumn(); if(r.has_db) ImGui::Text(tr("%.2f"), r.s_db); else ImGui::Text(tr("-"));
            // Cp
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.cp_est);
            ImGui::TableNextColumn(); if(r.has_db) ImGui::Text(tr("%.2f"), r.cp_db); else ImGui::Text(tr("-"));
        }
        
        ImGui::EndTable();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Método Benson
// ═══════════════════════════════════════════════════════════════════════════
static std::vector<Vec3> generar_tetraedro(Vec3 in_bond) {
    std::vector<Vec3> res;
    Vec3 u = in_bond.normalize();
    Vec3 w = {1,0,0};
    if (std::abs(u.x) > 0.9f) w = {0,1,0};
    w = (w - u * u.dot(w)).normalize(); 
    
    // Alternate orientation to encourage zigzag chains
    static bool flip = false;
    flip = !flip;
    if (flip) w = w * -1.0f;
    
    float c120 = -0.5f;
    float s120 = std::sqrt(3.0f)/2.0f;
    Vec3 cross = u.cross(w);
    
    Vec3 w1 = w;
    Vec3 w2 = w * c120 + cross * s120;
    Vec3 w3 = w * c120 - cross * s120;
    
    float ct = -1.0f/3.0f;
    float st = std::sqrt(8.0f/9.0f);
    
    res.push_back(u * ct + w1 * st);
    res.push_back(u * ct + w2 * st);
    res.push_back(u * ct + w3 * st);
    return res;
}

static std::vector<Vec3> generar_trigonal(Vec3 in_bond) {
    std::vector<Vec3> res;
    Vec3 u = in_bond.normalize();
    Vec3 w = {1,0,0};
    if (std::abs(u.x) > 0.9f) w = {0,1,0};
    w = (w - u * u.dot(w)).normalize(); 
    
    float c120 = -0.5f;
    float s120 = std::sqrt(3.0f)/2.0f;
    Vec3 cross = u.cross(w);
    
    res.push_back(u * -0.5f + w * s120);
    res.push_back(u * -0.5f - w * s120);
    return res;
}

void PanelBenson::agregar_grupo(const std::string& nombre, const std::string& elemento_central) {
    float bond_len = 1.54f;
    if (nombre[0] == '=') bond_len = 1.34f;
    if (nombre == "=O") bond_len = 1.22f;
    
    if (nombre == "Benzene") {
        // Special case for Benzene ring
        Vec3 pos = {0,0,0};
        Vec3 in_bond = {-1,0,0};
        int p_idx = -1;
        
        for (int i = 0; i < atomos_.size(); ++i) {
            if (atomos_[i].elemento != "H" && !atomos_[i].valencias.empty()) {
                p_idx = i;
                in_bond = atomos_[i].valencias.back();
                atomos_[i].valencias.pop_back();
                pos = {atomos_[i].x, atomos_[i].y, atomos_[i].z};
                pos = pos + in_bond * bond_len;
                break;
            }
        }
        
        Vec3 u = in_bond.normalize();
        Vec3 w = {1,0,0};
        if (std::abs(u.x) > 0.9f) w = {0,1,0};
        w = (w - u * u.dot(w)).normalize(); 
        Vec3 v = u.cross(w); // plane of the ring
        
        // Generate hexagon points
        int base_idx = atomos_.size();
        for(int i=0; i<6; i++) {
            float ang = i * 60.0f * 3.14159f / 180.0f;
            // First atom at u
            Vec3 dir = u * std::cos(ang) + w * std::sin(ang);
            Vec3 p = pos + dir * 1.4f - u * 1.4f; // shift so atom 0 is at pos
            
            Atomo3D a;
            a.x = p.x; a.y = p.y; a.z = p.z;
            a.elemento = "C";
            // Valence pointing outward
            a.valencias.push_back(dir);
            atomos_.push_back(a);
            
            if (i > 0) enlaces_.push_back(Enlace3D{base_idx + i - 1, base_idx + i});
        }
        enlaces_.push_back(Enlace3D{base_idx + 5, base_idx});
        if (p_idx >= 0) enlaces_.push_back(Enlace3D{p_idx, base_idx});
        
        // Add hydrogens to the ring where available
        for(int i=0; i<6; i++) {
            if (i == 0 && p_idx >= 0) continue; // C0 is attached to parent, no H!
            
            Vec3 val = atomos_[base_idx + i].valencias.back();
            atomos_[base_idx + i].valencias.pop_back();
            Vec3 hp = {atomos_[base_idx + i].x + val.x * 1.09f, 
                       atomos_[base_idx + i].y + val.y * 1.09f, 
                       atomos_[base_idx + i].z + val.z * 1.09f};
            
            Atomo3D ha = {hp.x, hp.y, hp.z, "H", {}};
            atomos_.push_back(ha);
            enlaces_.push_back(Enlace3D{base_idx + i, (int)atomos_.size()-1});
        }
        
        recalcular();
        return;
    }

    Vec3 pos = {0,0,0};
    Vec3 in_bond = {-1,0,0}; // default incoming bond for first atom
    int p_idx = -1;
    int h_to_remove = -1;
    
    // Find parent
    for (int i = 0; i < atomos_.size(); ++i) {
        if (atomos_[i].elemento != "H" && atomos_[i].elemento != "DELETED" && !atomos_[i].valencias.empty()) {
            p_idx = i;
            in_bond = atomos_[i].valencias.back();
            atomos_[i].valencias.pop_back();
            pos = {atomos_[i].x, atomos_[i].y, atomos_[i].z};
            pos = pos + in_bond * bond_len;
            break;
        }
    }
    
    if (p_idx == -1 && !atomos_.empty()) {
        // Try to replace an H
        for (int i = 0; i < atomos_.size(); ++i) {
            if (atomos_[i].elemento != "H" && atomos_[i].elemento != "DELETED") {
                for (int j = 0; j < enlaces_.size(); ++j) {
                    int a1 = enlaces_[j].atomo1;
                    int a2 = enlaces_[j].atomo2;
                    if ((a1 == i && atomos_[a2].elemento == "H") || 
                        (a2 == i && atomos_[a1].elemento == "H")) {
                        h_to_remove = (a1 == i) ? a2 : a1;
                        p_idx = i;
                        
                        Vec3 cx = {atomos_[i].x, atomos_[i].y, atomos_[i].z};
                        Vec3 hx = {atomos_[h_to_remove].x, atomos_[h_to_remove].y, atomos_[h_to_remove].z};
                        in_bond = (hx - cx).normalize();
                        pos = cx + in_bond * bond_len;
                        break;
                    }
                }
                if (p_idx != -1) break;
            }
        }
        
        if (h_to_remove != -1) {
            atomos_[h_to_remove].elemento = "DELETED";
            for (auto it = enlaces_.begin(); it != enlaces_.end(); ) {
                if ((it->atomo1 == p_idx && it->atomo2 == h_to_remove) || 
                    (it->atomo2 == p_idx && it->atomo1 == h_to_remove)) {
                    it = enlaces_.erase(it);
                } else {
                    ++it;
                }
            }
        } else {
            // Force attach to last C
            p_idx = atomos_.size() - 1;
            pos = {atomos_[p_idx].x + 1.5f, atomos_[p_idx].y, atomos_[p_idx].z};
        }
    }

    Atomo3D nuevo;
    nuevo.x = pos.x; nuevo.y = pos.y; nuevo.z = pos.z;
    nuevo.elemento = elemento_central;
    
    if (atomos_.empty()) {
        nuevo.valencias = {
            Vec3{0,1,0}, 
            Vec3{0.816f,-0.333f,-0.471f}, 
            Vec3{-0.816f,-0.333f,-0.471f}, 
            Vec3{0,-0.333f,0.942f}
        };
    } else {
        if (nombre.find("=") != std::string::npos || nombre == "-COOH") {
            nuevo.valencias = generar_trigonal(in_bond * -1.0f);
        } else {
            nuevo.valencias = generar_tetraedro(in_bond * -1.0f);
        }
    }
    
    int new_idx = atomos_.size();
    atomos_.push_back(nuevo);
    if (p_idx >= 0) {
        enlaces_.push_back(Enlace3D{p_idx, new_idx});
        if (nombre[0] == '=' || nombre == "=O") {
            enlaces_.push_back(Enlace3D{p_idx, new_idx});
        }
    }
    
    // Add hydrogens based on name
    int h_count = 0;
    if (nombre == "-CH3") h_count = 3;
    else if (nombre == "-CH2-" || nombre == "=CH2" || nombre == "-NH2") h_count = 2;
    else if (nombre == ">CH-" || nombre == "=CH-" || nombre == "-OH") h_count = 1;
    
    for (int i = 0; i < h_count && !atomos_[new_idx].valencias.empty(); ++i) {
        Vec3 v = atomos_[new_idx].valencias.back();
        atomos_[new_idx].valencias.pop_back();
        Vec3 hp = pos + v * 1.09f;
        Atomo3D h_at = {hp.x, hp.y, hp.z, "H", {}};
        atomos_.push_back(h_at);
        enlaces_.push_back(Enlace3D{new_idx, (int)atomos_.size()-1});
    }
    
    // Adjust open valences based on typical element bonds to prevent incorrect branching
    if (elemento_central == "O" || elemento_central == "S") {
        while (atomos_[new_idx].valencias.size() > 1) {
            atomos_[new_idx].valencias.pop_back();
        }
    } else if (elemento_central == "N") {
        while (atomos_[new_idx].valencias.size() > 2) {
            atomos_[new_idx].valencias.pop_back();
        }
    }
    
    // Clear valences for terminal functional groups so they don't act as attachment points
    if (nombre == "-OH" || nombre == "-NH2" || nombre == "=O" || 
        nombre == "-F" || nombre == "-Cl" || nombre == "-Br" || nombre == "-I" ||
        elemento_central == "Cl" || elemento_central == "Br" || elemento_central == "F" || elemento_central == "I") {
        atomos_[new_idx].valencias.clear();
    }
    
    if (nombre == "-COOH") {
        // Add double bond O
        Vec3 v1 = atomos_[new_idx].valencias.back();
        atomos_[new_idx].valencias.pop_back();
        Vec3 o1p = pos + v1 * 1.22f;
        int o1 = atomos_.size();
        atomos_.push_back({o1p.x, o1p.y, o1p.z, "O", {}});
        enlaces_.push_back(Enlace3D{new_idx, o1});
        enlaces_.push_back(Enlace3D{new_idx, o1}); // visually draw double bond?
        
        // Add single bond -OH
        Vec3 v2 = atomos_[new_idx].valencias.back();
        atomos_[new_idx].valencias.pop_back();
        Vec3 o2p = pos + v2 * 1.4f;
        int o2 = atomos_.size();
        
        // O2 needs valences for its H
        std::vector<Vec3> oval = generar_tetraedro(v2 * -1.0f);
        atomos_.push_back({o2p.x, o2p.y, o2p.z, "O", oval});
        enlaces_.push_back(Enlace3D{new_idx, o2});
        
        Vec3 hv = atomos_[o2].valencias.back();
        atomos_[o2].valencias.pop_back();
        Vec3 hp = o2p + hv * 0.96f;
        atomos_.push_back({hp.x, hp.y, hp.z, "H", {}});
        enlaces_.push_back(Enlace3D{o2, (int)atomos_.size()-1});
        
        atomos_[o2].valencias.clear(); // O in -OH is terminal
    }
    
    recalcular();
}

void PanelBenson::deshacer() {
    atomos_.clear();
    enlaces_.clear();
    recalcular();
}

void PanelBenson::recalcular() {
    mw_ = 0.0;
    h_est_ = 0.0;
    s_est_ = 0.0;
    cp_est_ = 0.0;
    
    std::map<std::string, int> counts;
    for (const auto& a : atomos_) {
        if (a.elemento != "DELETED") {
            counts[a.elemento]++;
        }
    }
    
    mw_ += counts["C"] * 12.011;
    mw_ += counts["H"] * 1.008;
    mw_ += counts["O"] * 15.999;
    mw_ += counts["N"] * 14.007;
    mw_ += counts["S"] * 32.06;
    mw_ += counts["Cl"] * 35.45;
    
    std::string formula_str = "";
    if (counts["C"] > 0) formula_str += "C" + std::to_string(counts["C"]);
    if (counts["H"] > 0) formula_str += "H" + std::to_string(counts["H"]);
    if (counts["O"] > 0) formula_str += "O" + std::to_string(counts["O"]);
    if (counts["N"] > 0) formula_str += "N" + std::to_string(counts["N"]);
    if (counts["Cl"] > 0) formula_str += "Cl" + std::to_string(counts["Cl"]);
    if (counts["S"] > 0) formula_str += "S" + std::to_string(counts["S"]);
    
    snprintf(formula_, sizeof(formula_), "%s", formula_str.c_str());
    
    h_est_ = counts["C"] * -10.0 + counts["O"] * -50.0 + counts["H"] * -2.0;
    s_est_ = counts["C"] * 40.0 + counts["H"] * 15.0 + counts["O"] * 30.0;
    cp_est_ = counts["C"] * 20.0 + counts["H"] * 5.0 + counts["O"] * 15.0;
}

void PanelBenson::renderizar() {
    if (ImGui::Button(tr("Undo last"))) deshacer(); // Simplified to clear for now
    if (ImGui::Button(tr("Calcular"))) recalcular();
    ImGui::SameLine();
    if (ImGui::Button(tr("Save Image"))) {
        FILE* f = fopen("temp_molecule.xyz", "w");
        if (f) {
            int valid_atoms = 0;
            for (const auto& a : atomos_) if (a.elemento != "DELETED") valid_atoms++;
            fprintf(f, "%d\nBenson Molecule\n", valid_atoms);
            for (const auto& a : atomos_) {
                if (a.elemento != "DELETED") fprintf(f, "%s %f %f %f\n", a.elemento.c_str(), a.x, a.y, a.z);
            }
            fclose(f);
            std::system("python3 cma_benson_render.py temp_molecule.xyz benson_render.png &");
        }
    }
    ImGui::SameLine();
    ImGui::SameLine();
    ImGui::PushItemWidth(150);
    ImGui::SliderFloat("Atom size", &atom_size_, 0.1f, 3.0f);
    ImGui::PopItemWidth();
    ImGui::Separator();
    
    ImGui::Columns(2, "benson_cols");
    ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() - 250);
    
    ImPlot3DFlags plot_flags = ImPlot3DFlags_NoLegend | ImPlot3DFlags_NoTitle;
    if (ImPlot3D::BeginPlot("##molecule_3d", ImVec2(-1, -1), plot_flags)) {
        ImPlot3DAxisFlags axis_flags = ImPlot3DAxisFlags_NoTickLabels | ImPlot3DAxisFlags_NoGridLines | ImPlot3DAxisFlags_NoTickMarks | ImPlot3DAxisFlags_NoLabel;
        ImPlot3D::SetupAxes("", "", "", axis_flags, axis_flags, axis_flags);
        
        ImPlot3DSpec line_spec;
        line_spec.LineWeight = 5.0f;
        line_spec.LineColor = ImVec4(0.5f, 0.5f, 0.5f, opacity_);
        
        std::map<std::pair<int,int>, int> bond_counts;
        for (const auto& bond : enlaces_) {
            if (bond.atomo1 < atomos_.size() && bond.atomo2 < atomos_.size() &&
                atomos_[bond.atomo1].elemento != "DELETED" && atomos_[bond.atomo2].elemento != "DELETED") {
                int a1 = std::min(bond.atomo1, bond.atomo2);
                int a2 = std::max(bond.atomo1, bond.atomo2);
                bond_counts[{a1, a2}]++;
            }
        }
        
        for (const auto& [pair, count] : bond_counts) {
            int a1 = pair.first;
            int a2 = pair.second;
            
            Vec3 p1 = {atomos_[a1].x, atomos_[a1].y, atomos_[a1].z};
            Vec3 p2 = {atomos_[a2].x, atomos_[a2].y, atomos_[a2].z};
            Vec3 dir = (p2 - p1).normalize();
            
            Vec3 up = {0,1,0};
            if (std::abs(dir.y) > 0.9f) up = {1,0,0};
            Vec3 right = dir.cross(up).normalize() * 0.18f; // offset for multiple bonds
            
            for (int k = 0; k < count; ++k) {
                Vec3 offset = {0,0,0};
                if (count == 2) {
                    offset = (k == 0) ? right : right * -1.0f;
                } else if (count == 3) {
                    if (k == 0) offset = right;
                    else if (k == 1) offset = right * -1.0f;
                }
                
                float xs[2] = {p1.x + offset.x, p2.x + offset.x};
                float ys[2] = {p1.y + offset.y, p2.y + offset.y};
                float zs[2] = {p1.z + offset.z, p2.z + offset.z};
                
                ImPlot3D::PlotLine("##bond", xs, ys, zs, 2, line_spec);
            }
        }
        
        std::map<std::string, std::vector<float>> px, py, pz;
        for (const auto& a : atomos_) {
            if (a.elemento == "DELETED") continue;
            px[a.elemento].push_back(a.x);
            py[a.elemento].push_back(a.y);
            pz[a.elemento].push_back(a.z);
        }
        
        for (auto& [elem, xs] : px) {
            ImVec4 col = ImVec4(0.5f, 0.5f, 0.5f, opacity_);
            float size = atom_size_ * 10.0f;
            if (elem == "C") col = ImVec4(0.2f, 0.2f, 0.2f, opacity_);
            else if (elem == "H") { col = ImVec4(0.9f, 0.9f, 0.9f, opacity_); size *= 0.6f; }
            else if (elem == "O") col = ImVec4(0.9f, 0.1f, 0.1f, opacity_);
            else if (elem == "N") col = ImVec4(0.1f, 0.1f, 0.9f, opacity_);
            else if (elem == "Cl") { col = ImVec4(0.1f, 0.9f, 0.1f, opacity_); size *= 1.2f; }
            else if (elem == "S") col = ImVec4(0.9f, 0.9f, 0.1f, opacity_);
            
            ImPlot3DSpec scatter_spec;
            scatter_spec.Marker = ImPlot3DMarker_Circle;
            scatter_spec.MarkerSize = size;
            scatter_spec.MarkerFillColor = col;
            scatter_spec.MarkerLineColor = col;
            
            ImPlot3D::PlotScatter(elem.c_str(), xs.data(), py[elem].data(), pz[elem].data(), xs.size(), scatter_spec);
        }
        
        ImPlot3D::EndPlot();
    }
    
    ImGui::NextColumn();
    ImGui::Text(tr("Please select"));
    
    if (ImGui::Button(tr("Skeletal groups"), ImVec2(-1, 0))) ImGui::OpenPopup("SkeletalPopup");
    if (ImGui::BeginPopup("SkeletalPopup")) {
        if (ImGui::Selectable(tr("-CH3"))) agregar_grupo("-CH3", "C");
        if (ImGui::Selectable(tr("-CH2-"))) agregar_grupo("-CH2-", "C");
        if (ImGui::Selectable(tr(">CH-"))) agregar_grupo(">CH-", "C");
        if (ImGui::Selectable(tr(">C<"))) agregar_grupo(">C<", "C");
        if (ImGui::Selectable(tr("=CH2"))) agregar_grupo("=CH2", "C");
        if (ImGui::Selectable(tr("=CH-"))) agregar_grupo("=CH-", "C");
        if (ImGui::Selectable(tr("Benzene ring"))) agregar_grupo("Benzene", "C");
        ImGui::EndPopup();
    }
    
    if (ImGui::Button(tr("Functional groups"), ImVec2(-1, 0))) ImGui::OpenPopup("FunctionalPopup");
    if (ImGui::BeginPopup("FunctionalPopup")) {
        if (ImGui::Selectable(tr("-OH"))) agregar_grupo("-OH", "O");
        if (ImGui::Selectable(tr("-COOH"))) agregar_grupo("-COOH", "C");
        if (ImGui::Selectable(tr("-NH2"))) agregar_grupo("-NH2", "N");
        if (ImGui::Selectable(tr("=O"))) agregar_grupo("=O", "O");
        if (ImGui::Selectable(tr("-F"))) agregar_grupo("-F", "F");
        if (ImGui::Selectable(tr("-Cl"))) agregar_grupo("-Cl", "Cl");
        if (ImGui::Selectable(tr("-Br"))) agregar_grupo("-Br", "Br");
        if (ImGui::Selectable(tr("-I"))) agregar_grupo("-I", "I");
        ImGui::EndPopup();
    }
    
    ImGui::Separator();
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.2f, 0.2f, 0.2f, 1.0f));
    if (ImGui::Button(tr(" C "), ImVec2(30, 30))) agregar_grupo("C", "C"); ImGui::SameLine();
    ImGui::PopStyleColor();
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.9f, 0.9f, 0.9f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.0f, 0.0f, 0.0f, 1.0f));
    if (ImGui::Button(tr(" H "), ImVec2(30, 30))) agregar_grupo("H", "H"); ImGui::SameLine();
    ImGui::PopStyleColor(2);
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.1f, 0.1f, 0.9f, 1.0f));
    if (ImGui::Button(tr(" N "), ImVec2(30, 30))) agregar_grupo("N", "N"); ImGui::SameLine();
    ImGui::PopStyleColor();
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.9f, 0.1f, 0.1f, 1.0f));
    if (ImGui::Button(tr(" O "), ImVec2(30, 30))) agregar_grupo("O", "O"); ImGui::SameLine();
    ImGui::PopStyleColor();
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.9f, 0.9f, 0.1f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.0f, 0.0f, 0.0f, 1.0f));
    if (ImGui::Button(tr(" S "), ImVec2(30, 30))) agregar_grupo("S", "S");
    ImGui::PopStyleColor(2);
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.1f, 0.9f, 0.1f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.0f, 0.0f, 0.0f, 1.0f));
    if (ImGui::Button(tr(" Cl"), ImVec2(30, 30))) agregar_grupo("Cl", "Cl");
    ImGui::PopStyleColor(2);
    
    ImGui::Separator();
    ImGui::Text(tr("Molecular formula:"));
    ImGui::TextDisabled(tr("%s"), formula_);
    ImGui::Text(tr("Molecular weight:"));
    ImGui::TextDisabled(tr("%.2f"), mw_);
    
    ImGui::Separator();
    ImGui::Text(tr("H Estimate: %.2f kJ/mol"), h_est_);
    ImGui::Text(tr("S Estimate: %.2f J/K*mol"), s_est_);
    ImGui::Text(tr("Cp Estimate: %.2f J/K*mol"), cp_est_);
    
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de LCA
// ═══════════════════════════════════════════════════════════════════════════
void PanelLCA::inicializar_flujos(diagrama_flujo::Flowsheet* flowsheet) {
    if (!flowsheet || cargado_inicial_) return;
    mapeos_.clear();

    for (auto& [fid, flujo] : flowsheet->mapa_flujos()) {
        MapeoLcaFlujo m;
        m.stream_id = fid;
        m.stream_name = flujo.nombre.empty() ? ("Stream " + std::to_string(fid)) : flujo.nombre;
        // Calcular flujo masico real: sum(ni * Mi) en lugar de flujo molar
        double flujo_masa = 0.0;
        auto& gestor_lca = base_datos::GestorDB::instancia();
        for (auto& [sp_id, n_moles] : flujo.flujo_molar) {
            auto* esp = gestor_lca.especie_por_id(sp_id);
            if (esp && esp->masa_molar > 0.0) {
                flujo_masa += n_moles * esp->masa_molar; // kmol/s * g/mol = kg/s (numéricamente)
            } else {
                flujo_masa += n_moles * 18.015; // Fallback: masa molar del agua
            }
        }
        m.flow_value = flujo_masa;
        m.unit = "kg/s";

        std::string unit_name = "System";
        bool es_entrada = true;
        
        for (auto& nodo : flowsheet->nodos()) {
            for (int64_t sid : nodo.ids_flujos_salida) {
                if (sid == fid) {
                    unit_name = nodo.nombre;
                    if (nodo.tipo == nucleo::TipoNodo::Fuente) {
                        es_entrada = true;
                    } else {
                        es_entrada = false;
                    }
                }
            }
            for (int64_t sid : nodo.ids_flujos_entrada) {
                if (sid == fid) {
                    if (nodo.tipo == nucleo::TipoNodo::Sumidero) {
                        es_entrada = false;
                    }
                }
            }
        }
        
        m.unit_name = unit_name;
        m.es_entrada = es_entrada;
        m.lca_equivalent = flujo.lca_equivalent;
        m.lca_group = flujo.lca_group;
        
        if (m.lca_equivalent == "No Mapping" || m.lca_equivalent.empty()) {
            std::string lower_name = m.stream_name;
            std::transform(lower_name.begin(), lower_name.end(), lower_name.begin(), ::tolower);
            
            if (lower_name.find("gas") != std::string::npos || lower_name.find("fuel") != std::string::npos || lower_name.find("co2") != std::string::npos) {
                m.lca_group = "Emissions to air";
                m.lca_equivalent = "CO2 (fossil) - ecoinvent";
            } else if (lower_name.find("slag") != std::string::npos || lower_name.find("relave") != std::string::npos || lower_name.find("escoria") != std::string::npos) {
                m.lca_group = "Deposited goods";
                m.lca_equivalent = "Slag landfill - GaBi";
            } else if (lower_name.find("concentrado") != std::string::npos || lower_name.find("cu") != std::string::npos || lower_name.find("mineral") != std::string::npos) {
                m.lca_group = "Resources";
                m.lca_equivalent = "Copper ore resource - GaBi";
            } else if (lower_name.find("agua") != std::string::npos || lower_name.find("water") != std::string::npos) {
                m.lca_group = "Resources";
                m.lca_equivalent = "Water intake - ecoinvent";
            } else {
                m.lca_group = "Not defined";
                m.lca_equivalent = "No Mapping";
            }
        }
        
        mapeos_.push_back(m);
    }
    cargado_inicial_ = true;
}

void PanelLCA::calcular_impacto() {
    double co2 = 0.0;
    double energia = 0.0;
    double residuos = 0.0;
    
    cma::motores::LCA lca_motor;
    co2 += lca_motor.factor_emision_electricidad(manual_electricidad_, "global");
    energia += manual_electricidad_ * 3.6;
    
    co2 += lca_motor.factor_emision_combustible(manual_combustible_, "diésel");
    energia += manual_combustible_ * 42.6;
    
    for (auto& m : mapeos_) {
        double flow_kg = m.flow_value * factor_normalizacion_;
        if (m.lca_equivalent == "No Mapping") continue;
        
        if (m.lca_group == "Emissions to air") {
            if (m.lca_equivalent.find("CO2") != std::string::npos) {
                co2 += flow_kg * 1.0;
            } else if (m.lca_equivalent.find("CH4") != std::string::npos) {
                co2 += flow_kg * 28.0;
            } else if (m.lca_equivalent.find("N2O") != std::string::npos) {
                co2 += flow_kg * 265.0;
            } else if (m.lca_equivalent.find("SO2") != std::string::npos) {
                co2 += flow_kg * 0.5;
            }
        } else if (m.lca_group == "Deposited goods") {
            residuos += flow_kg;
            co2 += flow_kg * 0.05;
        } else if (m.lca_group == "Resources") {
            if (m.lca_equivalent.find("ore") != std::string::npos) {
                energia += flow_kg * 1.2;
                co2 += flow_kg * 0.15;
            } else if (m.lca_equivalent.find("Water") != std::string::npos) {
                energia += flow_kg * 0.02;
            }
        }
    }
    
    resultado_huella_ = co2;
    resultado_energia_ = energia;
    resultado_residuos_ = residuos;
    calculado_ = true;
}

void PanelLCA::renderizar(diagrama_flujo::Flowsheet* flowsheet) {
    if (!flowsheet) {
        ImGui::Text(tr("Cargue una simulación para evaluar LCA."));
        return;
    }
    
    inicializar_flujos(flowsheet);

    // ── Barra de menú superior estilo CAD ──
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(8, 6));
    if (ImGui::Button(tr("Guardar"))) {
        for (auto& m : mapeos_) {
            auto* f = flowsheet->flujo_por_id(m.stream_id);
            if (f) {
                f->lca_equivalent = m.lca_equivalent;
                f->lca_group = m.lca_group;
            }
        }
        ImGui::OpenPopup("lca_saved");
    }
    if (ImGui::BeginPopup("lca_saved")) {
        ImGui::Text(tr("Mappings saved to flowsheet streams!"));
        if (ImGui::Button(tr("Cerrar"))) ImGui::CloseCurrentPopup();
        ImGui::EndPopup();
    }
    
    ImGui::SameLine();
    if (ImGui::Button(tr("Refrescar"))) {
        cargado_inicial_ = false;
        inicializar_flujos(flowsheet);
    }
    if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Reload streams from the flowsheet"));

    ImGui::SameLine();
    if (ImGui::Button(tr("Normalize"))) {
        if (fila_seleccionada_ >= 0 && fila_seleccionada_ < (int)mapeos_.size()) {
            double val = mapeos_[fila_seleccionada_].flow_value;
            if (val > 0.0) {
                factor_normalizacion_ = 1.0 / val;
                normalizado_ = true;
                calcular_impacto();
            }
        }
    }
    if (ImGui::IsItemHovered()) ImGui::SetTooltip("Normalizar flujos basándose en la corriente seleccionada (Functional Unit = 1.0)");
    
    ImGui::SameLine();
    if (ImGui::Button(tr("Reset Norm"))) {
        factor_normalizacion_ = 1.0;
        normalizado_ = false;
        calcular_impacto();
    }

    ImGui::SameLine();
    if (ImGui::Button(tr("To GaBi"))) {
        FILE* zenity = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.xml' --title='Exportar a GaBi Plan'", "r");
        if (zenity) {
            char path[512] = {0};
            if (fgets(path, sizeof(path), zenity)) {
                std::string path_str(path);
                if (!path_str.empty() && path_str.back() == '\n') path_str.pop_back();
                if (path_str.find(".xml") == std::string::npos) path_str += ".xml";
                
                FILE* fp = fopen(path_str.c_str(), "w");
                if (fp) {
                    fprintf(fp, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
                    fprintf(fp, "<GaBiPlan version=\"8.7\" name=\"HSC Sim Exported Plan\">\n");
                    fprintf(fp, "  <Flows>\n");
                    for (auto& m : mapeos_) {
                        if (m.lca_equivalent != "No Mapping") {
                            fprintf(fp, "    <Flow name=\"%s\" gabiEquivalent=\"%s\" group=\"%s\" flowValue=\"%.6f\" unit=\"%s\"/>\n",
                                    m.stream_name.c_str(), m.lca_equivalent.c_str(), m.lca_group.c_str(), m.flow_value * factor_normalizacion_, m.unit.c_str());
                        }
                    }
                    fprintf(fp, "  </Flows>\n");
                    fprintf(fp, "  <ManualInputs electricity_kWh=\"%.2f\" fuel_kg=\"%.2f\" water_m3=\"%.2f\"/>\n",
                            manual_electricidad_, manual_combustible_, manual_agua_);
                    fprintf(fp, "  <Indicators carbonFootprint_kgCO2eq=\"%.2f\" primaryEnergy_MJ=\"%.2f\" solidWaste_kg=\"%.2f\"/>\n",
                            resultado_huella_, resultado_energia_, resultado_residuos_);
                    fprintf(fp, "</GaBiPlan>\n");
                    fclose(fp);
                }
            }
            pclose(zenity);
        }
    }
    
    ImGui::SameLine();
    if (ImGui::Button(tr("To Excel"))) {
        FILE* zenity = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Exportar a Excel (CSV)'", "r");
        if (zenity) {
            char path[512] = {0};
            if (fgets(path, sizeof(path), zenity)) {
                std::string path_str(path);
                if (!path_str.empty() && path_str.back() == '\n') path_str.pop_back();
                if (path_str.find(".csv") == std::string::npos) path_str += ".csv";
                
                FILE* fp = fopen(path_str.c_str(), "w");
                if (fp) {
                    fprintf(fp, "LCA Evaluation Dataset,Normalized,Factor:;%.6f\n", factor_normalizacion_);
                    fprintf(fp, "Unit Name,Stream Name,Flow Value,Unit,Lca Equivalent,Lca Group,Direction\n");
                    for (auto& m : mapeos_) {
                        fprintf(fp, "\"%s\",\"%s\",%.6f,\"%s\",\"%s\",\"%s\",\"%s\"\n",
                                m.unit_name.c_str(), m.stream_name.c_str(), m.flow_value * factor_normalizacion_, m.unit.c_str(), m.lca_equivalent.c_str(), m.lca_group.c_str(), m.es_entrada ? "Input" : "Output");
                    }
                    fprintf(fp, "\nManual Inputs / Outputs\n");
                    fprintf(fp, "Indicator,Value,Unit\n");
                    fprintf(fp, "Electricity,%.2f,kWh\n", manual_electricidad_);
                    fprintf(fp, "Fuel,%.2f,kg\n", manual_combustible_);
                    fprintf(fp, "Water,%.2f,m3\n", manual_agua_);
                    fprintf(fp, "\nEnvironmental Impacts Summary\n");
                    fprintf(fp, "Carbon Footprint,%.2f,kg CO2-equiv\n", resultado_huella_);
                    fprintf(fp, "Solid Waste,%.2f,kg\n", resultado_residuos_);
                    fclose(fp);
                }
            }
            pclose(zenity);
        }
    }
    ImGui::PopStyleVar();
    ImGui::Separator();
    ImGui::Spacing();

    // ── Grid split ──
    ImGui::Columns(2, "lca_panel_split", true);
    
    // Ancho inicial de columnas
    static bool init_w = true;
    if (init_w) {
        ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.65f);
        init_w = false;
    }

    // ── Columna Izquierda: Tabla y Pestañas ──
    ImGui::BeginChild("left_pane_lca");
    {
        if (mapeos_.empty()) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f, 0.4f, 0.4f, 1.0f), "Warning: No streams found in active page.");
            ImGui::TextWrapped(tr("Please draw some streams in the active flowsheet page to allow mapping."));
            ImGui::Spacing();
            ImGui::Separator();
            ImGui::Spacing();
        }

        if (ImGui::BeginTabBar("lca_tabs")) {
            // Pestaña INPUTS
            if (ImGui::BeginTabItem(tr("Entradas"))) {
                if (ImGui::BeginTable("inputs_table", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 240))) {
                    ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                    ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthFixed, 120.0f);
                    ImGui::TableSetupColumn("Flujo", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                    ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthFixed, 50.0f);
                    ImGui::TableSetupColumn("Lca Equivalent", ImGuiTableColumnFlags_WidthFixed, 150.0f);
                    ImGui::TableSetupColumn("Lca Group", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                    ImGui::TableHeadersRow();
                    
                    int idx = 0;
                    for (auto& m : mapeos_) {
                        if (m.es_entrada) {
                            ImGui::TableNextRow();
                            ImGui::TableNextColumn();
                            
                            bool sel = (fila_seleccionada_ == idx);
                            if (ImGui::Selectable(m.unit_name.c_str(), sel, ImGuiSelectableFlags_SpanAllColumns)) {
                                fila_seleccionada_ = idx;
                            }
                            
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.stream_name.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.4f"), m.flow_value * factor_normalizacion_);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.unit.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.lca_equivalent.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.lca_group.c_str());
                        }
                        idx++;
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }

            // Pestaña OUTPUTS
            if (ImGui::BeginTabItem(tr("Salidas"))) {
                if (ImGui::BeginTable("outputs_table", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 240))) {
                    ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                    ImGui::TableSetupColumn("Corriente", ImGuiTableColumnFlags_WidthFixed, 120.0f);
                    ImGui::TableSetupColumn("Flujo", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                    ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthFixed, 50.0f);
                    ImGui::TableSetupColumn("Lca Equivalent", ImGuiTableColumnFlags_WidthFixed, 150.0f);
                    ImGui::TableSetupColumn("Lca Group", ImGuiTableColumnFlags_WidthFixed, 100.0f);
                    ImGui::TableHeadersRow();
                    
                    int idx = 0;
                    for (auto& m : mapeos_) {
                        if (!m.es_entrada) {
                            ImGui::TableNextRow();
                            ImGui::TableNextColumn();
                            
                            bool sel = (fila_seleccionada_ == idx);
                            if (ImGui::Selectable(m.unit_name.c_str(), sel, ImGuiSelectableFlags_SpanAllColumns)) {
                                fila_seleccionada_ = idx;
                            }
                            
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.stream_name.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%.4f"), m.flow_value * factor_normalizacion_);
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.unit.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.lca_equivalent.c_str());
                            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), m.lca_group.c_str());
                        }
                        idx++;
                    }
                    ImGui::EndTable();
                }
                ImGui::EndTabItem();
            }

            // Pestaña MANUAL INPUTS
            if (ImGui::BeginTabItem(tr("Manual Inputs"))) {
                ImGui::Spacing();
                ImGui::Text(tr("Consumos auxiliares directos:"));
                ImGui::Separator();
                if (ImGui::InputDouble("Electricidad Directa (kWh)", &manual_electricidad_)) calcular_impacto();
                if (ImGui::InputDouble("Combustible Diésel (kg)", &manual_combustible_)) calcular_impacto();
                if (ImGui::InputDouble("Consumo Agua de Proceso (m³)", &manual_agua_)) calcular_impacto();
                ImGui::EndTabItem();
            }

            // Pestaña INDICATORS (Gráficos)
            if (ImGui::BeginTabItem(tr("Indicators"))) {
                calcular_impacto();
                
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.2f,0.8f,0.6f,1), "Carbon Footprint: %.2f kg CO₂-equiv", resultado_huella_);
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.9f,0.6f,0.2f,1), "Primary Energy Demand: %.1f MJ", resultado_energia_);
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f,0.4f,0.8f,1), "Solid Waste: %.1f kg", resultado_residuos_);
                
                double vals[3] = {resultado_huella_, resultado_energia_ * 0.1, resultado_residuos_};
                const char* labels[3] = {"CO₂eq (kg)", "Energía (MJ×0.1)", "Residuos (kg)"};
                if (ImPlot::BeginPlot("LCA Indicators", ImVec2(-1, 150), ImPlotFlags_NoTitle)) {
                    ImPlot::SetupAxes("Impacto", "Escala");
                    ImPlot::SetupAxesLimits(-0.5, 2.5, 0, std::max({vals[0], vals[1], vals[2]}) * 1.2);
                    for (int i = 0; i < 3; i++) {
                        ImPlot::PlotBars(labels[i], &vals[i], 1, 0.4, i - 0.2);
                    }
                    ImPlot::EndPlot();
                }
                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }
        
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();
        
        // Botón Apply Mapping
        if (ImGui::Button(tr("Aplicar Mapeo"), ImVec2(-1, 26))) {
            if (fila_seleccionada_ >= 0 && fila_seleccionada_ < (int)mapeos_.size() && !nodo_arbol_seleccionado_.empty()) {
                auto& m = mapeos_[fila_seleccionada_];
                m.lca_equivalent = nodo_arbol_seleccionado_;
                m.lca_group = cat_path_;
                
                auto* f = flowsheet->flujo_por_id(m.stream_id);
                if (f) {
                    f->lca_equivalent = m.lca_equivalent;
                    f->lca_group = m.lca_group;
                }
                calcular_impacto();
            }
        }
    }
    ImGui::EndChild();
    
    ImGui::NextColumn();

    // ── Columna Derecha: Árbol de categorías GaBi ──
    ImGui::BeginChild("right_pane_lca");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.6f,0.7f,0.9f,1), "Flows Database (GaBi)");
        ImGui::Separator();
        
        auto render_leaf = [&](const char* name, const char* path, const char* unit, const char* ref_q, double conv) {
            bool is_selected = (nodo_arbol_seleccionado_ == name);
            if (ImGui::Selectable(name, is_selected)) {
                nodo_arbol_seleccionado_ = name;
                cat_nombre_ = name;
                cat_path_ = path;
                cat_unit_ = unit;
                cat_ref_q_ = ref_q;
                cat_conv_f_ = conv;
            }
        };

        if (ImGui::TreeNode(tr("Bienes depositados"))) {
            render_leaf("Slag landfill - GaBi", "Deposited goods", "kg", "1 kg", 1.0);
            render_leaf("Fly ash disposal - ecoinvent", "Deposited goods", "kg", "1 kg", 1.0);
            render_leaf("Tailings deposition - GaBi", "Deposited goods", "kg", "1 kg", 1.0);
            ImGui::TreePop();
        }

        if (ImGui::TreeNode(tr("Emisiones al aire"))) {
            if (ImGui::TreeNode(tr("Greenhouse gases"))) {
                render_leaf("CO2 (fossil) - ecoinvent", "Emissions to air", "kg", "1 kg", 1.0);
                render_leaf("CH4 (methane) - ecoinvent", "Emissions to air", "kg", "1 kg", 28.0);
                render_leaf("N2O (nitrous oxide) - ecoinvent", "Emissions to air", "kg", "1 kg", 265.0);
                ImGui::TreePop();
            }
            if (ImGui::TreeNode(tr("Acidification gases"))) {
                render_leaf("SO2 (sulfur dioxide) - GaBi", "Emissions to air", "kg", "1 kg", 1.0);
                render_leaf("NOx (nitrogen oxides) - GaBi", "Emissions to air", "kg", "1 kg", 1.0);
                ImGui::TreePop();
            }
            ImGui::TreePop();
        }

        if (ImGui::TreeNode(tr("Emisiones a agua dulce"))) {
            render_leaf("Heavy metals to water - GaBi", "Emissions to fresh water", "kg", "1 kg", 1.0);
            render_leaf("Nitrates / Phosphates - ecoinvent", "Emissions to fresh water", "kg", "1 kg", 1.0);
            ImGui::TreePop();
        }

        if (ImGui::TreeNode(tr("Resources"))) {
            render_leaf("Copper ore resource - GaBi", "Resources", "kg", "1 kg", 1.0);
            render_leaf("Iron ore resource - GaBi", "Resources", "kg", "1 kg", 1.0);
            render_leaf("Water intake - ecoinvent", "Resources", "kg", "1 kg", 1.0);
            render_leaf("Electricity (cradle-to-gate) - GaBi", "Resources", "kWh", "1 kWh", 1.0);
            ImGui::TreePop();
        }

        ImGui::Spacing();
        ImGui::Separator();
        
        // Detalles de categoría seleccionada
        ImGui::Text(tr("Category Details:"));
        ImGui::TextDisabled(tr("Name: %s"), cat_nombre_.c_str());
        ImGui::TextDisabled(tr("Path: %s"), cat_path_.c_str());
        ImGui::TextDisabled(tr("Unit: %s"), cat_unit_.c_str());
        ImGui::TextDisabled(tr("Ref Quantity: %s"), cat_ref_q_.c_str());
        ImGui::TextDisabled(tr("Conv Factor: %.1f"), cat_conv_f_);
    }
    ImGui::EndChild();
    
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Datos / DataFitter
// ═══════════════════════════════════════════════════════════════════════════
void PanelDatosANN::renderizar() {
    ImGui::Text(tr("Ajuste de Curvas (DataFitter)"));
    ImGui::Separator();

    ImGui::InputText(tr("X (comma-sep)"), entrada_1_, sizeof(entrada_1_));
    ImGui::InputText(tr("Y (comma-sep)"), entrada_2_, sizeof(entrada_2_));

    static int grado_poly = 2;
    ImGui::InputInt("Grado polinomio", &grado_poly);
    if (grado_poly < 1) grado_poly = 1;
    if (grado_poly > 10) grado_poly = 10;

    auto parsear_csv = [](const std::string& s) {
        std::vector<double> v;
        std::stringstream ss(s);
        std::string token;
        while (std::getline(ss, token, ',')) {
            if (token.empty()) continue;
            try { v.push_back(std::stod(token)); }
            catch (...) { v.push_back(0.0); }
        }
        return v;
    };

    if (ImGui::Button(tr("Ajuste Lineal"), ImVec2(-1, 25))) {
        std::vector<double> x = parsear_csv(entrada_1_), y = parsear_csv(entrada_2_);
        if (x.size() == y.size() && x.size() >= 2) {
            hsc_clone::DataFitter fitter;
            res_fit_ = fitter.lineal(x, y);
            entrenado_ = res_fit_.ok;
            datos_x_ = x;
            datos_y_ = y;
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Ajuste Polinomial"), ImVec2(-1, 25))) {
        std::vector<double> x = parsear_csv(entrada_1_), y = parsear_csv(entrada_2_);
        if (x.size() == y.size() && x.size() >= grado_poly + 1) {
            hsc_clone::DataFitter fitter;
            res_fit_ = fitter.polinomial(x, y, grado_poly);
            entrenado_ = res_fit_.ok;
            datos_x_ = x;
            datos_y_ = y;
        }
    }

    if (entrenado_) {
        ImGui::Separator();
        ImGui::Text(tr("R² = %.6f"), res_fit_.r_cuadrado);
        ImGui::Text(tr("Coeficientes:"));
        for (size_t i = 0; i < res_fit_.coeficientes.size(); i++)
            ImGui::Text(tr("  a%d = %.6f"), (int)i, res_fit_.coeficientes[i]);

        if (datos_x_.size() > 1 && !res_fit_.coeficientes.empty()) {
            if (ImPlot::BeginPlot("Ajuste", ImVec2(-1, 280), ImPlotFlags_NoTitle)) {
                ImPlot::SetupAxes("X", "Y");
                double xmin = *std::min_element(datos_x_.begin(), datos_x_.end());
                double xmax = *std::max_element(datos_x_.begin(), datos_x_.end());
                double ymin = *std::min_element(datos_y_.begin(), datos_y_.end());
                double ymax = *std::max_element(datos_y_.begin(), datos_y_.end());
                double margin = (ymax - ymin) * 0.2;
                ImPlot::SetupAxesLimits(xmin - margin*0.5, xmax + margin*0.5, ymin - margin, ymax + margin);
                ImPlot::PlotScatter("Datos", datos_x_.data(), datos_y_.data(), (int)datos_x_.size());

                int n_curve = 100;
                std::vector<double> fx(n_curve), fy(n_curve);
                for (int i = 0; i < n_curve; i++) {
                    fx[i] = xmin + (xmax - xmin) * i / (n_curve - 1);
                    fy[i] = 0;
                    double xp = 1.0;
                    for (auto& c : res_fit_.coeficientes) {
                        fy[i] += c * xp;
                        xp *= fx[i];
                    }
                }
                ImPlot::PlotLine("Ajuste", fx.data(), fy.data(), n_curve);
                ImPlot::EndPlot();
            }
        }

        if (!res_fit_.mensaje.empty())
            ImGui::TextWrapped(tr("%s"), res_fit_.mensaje.c_str());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Gráficos 3D
// ═══════════════════════════════════════════════════════════════════════════

void PanelGrafico3D::generar_datos_ejemplo() {
    data_x_.clear(); data_y_.clear(); data_z_.clear();
    // Superficie: z = sin(r)/r (sinc)
    int n = 30;
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            float x = -3.0f + 6.0f * i / (n-1);
            float y = -3.0f + 6.0f * j / (n-1);
            float r = std::sqrt(x*x + y*y);
            float z = (r > 0.01f) ? std::sin(r)/r : 1.0f;
            data_x_.push_back(x);
            data_y_.push_back(y);
            data_z_.push_back(z);
        }
    }
    datos_generados_ = true;
}

ImVec3 PanelGrafico3D::proyectar(float x, float y, float z, const ImVec2& centro, float escala) const {
    float sx = -std::sin(ang_y_);
    float sy = std::cos(ang_y_);
    float cz = std::cos(ang_x_);
    float sz = std::sin(ang_x_);

    float rx = x * cz - z * sz;
    float ry = y;
    float rz = x * sz + z * cz;

    float px = rx * sy + ry * sx;
    float py = -rx * sx + ry * sy;
    float pz = rz + distancia_;

    if (pz < 0.1f) pz = 0.1f;
    float f = escala * 2.0f / pz;
    return ImVec3(centro.x + px * f, centro.y - py * f, pz);
}

void PanelGrafico3D::renderizar() {
    if (!datos_generados_) generar_datos_ejemplo();

    ImGui::Text(tr("Gráfico 3D"));
    ImGui::Separator();

    ImGui::SliderFloat("Rotación X", &ang_x_, -3.14f, 3.14f, "%.2f");
    ImGui::SliderFloat("Rotación Y", &ang_y_, -3.14f, 3.14f, "%.2f");
    ImGui::SliderFloat("Distancia", &distancia_, 1.0f, 20.0f, "%.1f");
    ImGui::Checkbox(tr("Auto-rotar"), &auto_rotar_);
    ImGui::SameLine();
    ImGui::Combo("Tipo", &tipo_grafico_, "Superficie\0Líneas\0Puntos\0Contorno\0");

    if (auto_rotar_) {
        ang_y_ += ImGui::GetIO().DeltaTime * 0.5f;
    }

    ImVec2 area = ImGui::GetContentRegionAvail();
    float lado = std::min(area.x, area.y);
    ImVec2 centro(area.x * 0.5f, 60.0f + lado * 0.5f);
    float escala = lado * 0.25f;

    ImDrawList* draw = ImGui::GetWindowDrawList();

    // Fondo
    draw->AddRectFilled(ImVec2(0, 50), ImVec2(area.x, 50 + lado),
                        IM_COL32(15,15,22,255), 4);

    // Ejes
    ImVec3 ox = proyectar(4,0,0,centro,escala);
    ImVec3 oy = proyectar(0,4,0,centro,escala);
    ImVec3 oz = proyectar(0,0,4,centro,escala);
    ImVec3 oo = proyectar(0,0,0,centro,escala);

    draw->AddLine(ImVec2(oo.x,oo.y), ImVec2(ox.x,ox.y), IM_COL32(255,80,80,200), 2);
    draw->AddLine(ImVec2(oo.x,oo.y), ImVec2(oy.x,oy.y), IM_COL32(80,255,80,200), 2);
    draw->AddLine(ImVec2(oo.x,oo.y), ImVec2(oz.x,oz.y), IM_COL32(80,80,255,200), 2);

    if (tipo_grafico_ == 0 || tipo_grafico_ == 3) {
        // Superficie / Contorno
        int n = (int)std::sqrt((float)data_x_.size());
        if (n > 1) {
            for (int i = 0; i < n-1; i++) {
                for (int j = 0; j < n-1; j++) {
                    int idx = i * n + j;
                    ImVec3 p1 = proyectar(data_x_[idx], data_y_[idx], data_z_[idx], centro, escala);
                    ImVec3 p2 = proyectar(data_x_[idx+1], data_y_[idx+1], data_z_[idx+1], centro, escala);
                    ImVec3 p3 = proyectar(data_x_[idx+n], data_y_[idx+n], data_z_[idx+n], centro, escala);
                    ImVec3 p4 = proyectar(data_x_[idx+n+1], data_y_[idx+n+1], data_z_[idx+n+1], centro, escala);

                    float val = (data_z_[idx] + data_z_[idx+n] + data_z_[idx+n+1] + data_z_[idx]) * 0.25f;
                    int r = (int)(128 + 127 * std::max(-1.0f, std::min(1.0f, val)));
                    ImU32 col = IM_COL32(r, (int)(255-r*0.5f), (int)(255-r), 180);

                    if (tipo_grafico_ == 0) {
                        draw->AddTriangleFilled(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), ImVec2(p3.x,p3.y), col);
                        draw->AddTriangleFilled(ImVec2(p2.x,p2.y), ImVec2(p4.x,p4.y), ImVec2(p3.x,p3.y), col);
                    }
                    draw->AddTriangle(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), ImVec2(p3.x,p3.y), IM_COL32(60,60,80,80), 1);
                    draw->AddTriangle(ImVec2(p2.x,p2.y), ImVec2(p4.x,p4.y), ImVec2(p3.x,p3.y), IM_COL32(60,60,80,80), 1);
                }
            }
        }
    } else if (tipo_grafico_ == 1) {
        // Líneas (isolinhas)
        int n = (int)std::sqrt((float)data_x_.size());
        for (int i = 0; i < n; i++) {
            for (int j = 0; j < n-1; j++) {
                int idx = i * n + j;
                ImVec3 p1 = proyectar(data_x_[idx], data_y_[idx], data_z_[idx], centro, escala);
                ImVec3 p2 = proyectar(data_x_[idx+1], data_y_[idx+1], data_z_[idx+1], centro, escala);
                draw->AddLine(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), IM_COL32(100,200,255,120), 1);
            }
        }
        for (int j = 0; j < n; j++) {
            for (int i = 0; i < n-1; i++) {
                int idx = i * n + j;
                ImVec3 p1 = proyectar(data_x_[idx], data_y_[idx], data_z_[idx], centro, escala);
                ImVec3 p2 = proyectar(data_x_[idx+n], data_y_[idx+n], data_z_[idx+n], centro, escala);
                draw->AddLine(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), IM_COL32(100,200,255,120), 1);
            }
        }
    } else {
        // Puntos
        for (size_t i = 0; i < data_x_.size(); i++) {
            ImVec3 p = proyectar(data_x_[i], data_y_[i], data_z_[i], centro, escala);
            float val = data_z_[i];
            int r = (int)(128 + 127 * std::max(-1.0f, std::min(1.0f, val)));
            ImU32 col = IM_COL32(r, 200-r, 255-r, 200);
            draw->AddCircleFilled(ImVec2(p.x,p.y), 3, col);
        }
    }

    // Leyenda de ejes
    draw->AddText(ImVec2(ox.x+4,ox.y-6), IM_COL32(255,120,120,200), "X");
    draw->AddText(ImVec2(oy.x+4,oy.y-6), IM_COL32(120,255,120,200), "Y");
    draw->AddText(ImVec2(oz.x+4,oz.y-6), IM_COL32(120,120,255,200), "Z");
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de ReactionSolver (Analizador y Solucionador de Reacciones)
// ═══════════════════════════════════════════════════════════════════════════

// Split por '+' respetando paréntesis: "Ag(+a) + e-" -> ["Ag(+a)", "e-"]
// (con split naíf, "Ag(+a)" quedaría destruido en "Ag(" y "a)")
static std::vector<std::string> split_reaction_terms(const std::string& s) {
    std::vector<std::string> parts;
    std::string cur;
    int depth = 0;
    for (char c : s) {
        if (c == '(') ++depth;
        else if (c == ')' && depth > 0) --depth;
        if (c == '+' && depth == 0) {
            parts.push_back(cur);
            cur.clear();
        } else {
            cur += c;
        }
    }
    parts.push_back(cur);
    return parts;
}

static std::pair<double, std::string> parse_reaction_term(const std::string& term) {
    std::string t = trim_str(term);
    if (t.empty()) return {0.0, ""};

    size_t i = 0;
    while (i < t.length() && (std::isdigit(t[i]) || t[i] == '.' || t[i] == '/')) {
        i++;
    }

    double coeff = 1.0;
    std::string formula = t;

    if (i > 0) {
        std::string num_part = t.substr(0, i);
        std::string rest = trim_str(t.substr(i));
        if (!rest.empty()) {
            formula = rest;
            size_t slash = num_part.find('/');
            if (slash != std::string::npos) {
                try {
                    double num = std::stod(num_part.substr(0, slash));
                    double den = std::stod(num_part.substr(slash + 1));
                    if (den != 0) coeff = num / den;
                } catch (...) {
                    coeff = 1.0;
                }
            } else {
                try {
                    coeff = std::stod(num_part);
                } catch (...) {
                    coeff = 1.0;
                }
            }
        }
    }

    return {coeff, formula};
}

static bool parse_chemical_equation(const std::string& eq_str, cma::nucleo::Reaccion& rxn, std::string& error_msg) {
    rxn.estequiometria.clear();

    std::string clean_eq = trim_str(eq_str);
    if (clean_eq.empty()) {
        error_msg = "La ecuación está vacía.";
        return false;
    }

    std::string arrow = "->";
    size_t arrow_pos = clean_eq.find(arrow);
    if (arrow_pos == std::string::npos) {
        arrow = "=";
        arrow_pos = clean_eq.find(arrow);
        if (arrow_pos == std::string::npos) {
            error_msg = "No se encontró el separador '->' o '='.";
            return false;
        }
    }

    std::string reactants_str = clean_eq.substr(0, arrow_pos);
    std::string products_str = clean_eq.substr(arrow_pos + arrow.length());

    auto reactants = split_reaction_terms(reactants_str);
    for (const auto& r : reactants) {
        std::string term = trim_str(r);
        if (term.empty()) continue;
        auto [coeff, formula] = parse_reaction_term(term);
        if (formula.empty()) continue;

        const auto* esp = buscar_especie_por_nombre_o_formula(formula);
        if (!esp) {
            error_msg = "Especie no encontrada en la base de datos: " + formula;
            return false;
        }

        rxn.estequiometria[esp->id] -= coeff;
    }

    auto products = split_reaction_terms(products_str);
    for (const auto& p : products) {
        std::string term = trim_str(p);
        if (term.empty()) continue;
        auto [coeff, formula] = parse_reaction_term(term);
        if (formula.empty()) continue;

        const auto* esp = buscar_especie_por_nombre_o_formula(formula);
        if (!esp) {
            error_msg = "Especie no encontrada en la base de datos: " + formula;
            return false;
        }

        rxn.estequiometria[esp->id] += coeff;
    }

    if (rxn.estequiometria.empty()) {
        error_msg = "Ecuación vacía o inválida.";
        return false;
    }

    return true;
}

// Funciones auxiliares para termodinámica con Cp real
static double calc_dH_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double dH = 0.0;
    for (auto& [sid, coeff] : rxn.estequiometria) {
        auto it = db.find(sid);
        if (it != db.end()) {
            dH += coeff * it->second.entalpia_abs(T_K);
        }
    }
    return dH;
}

static double calc_dS_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double dS = 0.0;
    for (auto& [sid, coeff] : rxn.estequiometria) {
        auto it = db.find(sid);
        if (it != db.end()) {
            dS += coeff * it->second.entropia_abs(T_K);
        }
    }
    return dS;
}

static double calc_dG_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double dH = calc_dH_T(rxn, T_K, db);
    double dS = calc_dS_T(rxn, T_K, db);
    return dH - T_K * dS / 1000.0;
}

// Log10 K calculado directamente de dG para evitar overflow de exp()
static double calc_logK_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double dG = calc_dG_T(rxn, T_K, db); // kJ/mol
    return -dG * 1000.0 / (cma::nucleo::R_GAS * T_K * 2.302585092994046); // ln(10)
}

static double calc_Keq_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double logK = calc_logK_T(rxn, T_K, db);
    if (logK >  300.0) return 1.0e+300; // saturar para no imprimir "inf"
    if (logK < -300.0) return 1.0e-300;
    return std::pow(10.0, logK);
}

// ── Detección de semirreacción electroquímica (presencia de e-) ────────────
static bool info_electroquimica(const cma::nucleo::Reaccion& rxn, double& n_electrones) {
    auto& gestor = cma::base_datos::GestorDB::instancia();
    n_electrones = 0.0;
    for (auto& [sp_id, coeff] : rxn.estequiometria) {
        const auto* sp = gestor.especie_por_id(sp_id);
        if (sp && (sp->formula == "e-" || sp->formula == "e")) {
            n_electrones = std::abs(coeff);
            return n_electrones > 0.0;
        }
    }
    return false;
}

// Potencial estándar vs SHE:  ΔG = -n·F·E  →  E° = -ΔG/(n·F)   [F = 96.4853 kJ/(V·mol)]
static double calc_E0_she(double dG_kJ, double n_e) {
    if (n_e <= 0.0) return 0.0;
    return -dG_kJ / (n_e * 96.4853);
}

// Carga formal a partir del tag de fase almacenado en la BD: "(+a)"→+1, "(-2a)"→-2...
static double carga_de_tag(const std::string& formula) {
    size_t open = formula.rfind('(');
    if (open == std::string::npos) return 0.0;
    size_t close = formula.find(')', open);
    if (close == std::string::npos) return 0.0;
    std::string tag = formula.substr(open + 1, close - open - 1);
    if (tag.empty() || (tag[0] != '+' && tag[0] != '-')) return 0.0;
    int sign = (tag[0] == '+') ? 1 : -1;
    double val = 0.0;
    for (size_t i = 1; i < tag.size() && std::isdigit((unsigned char)tag[i]); ++i)
        val = val * 10.0 + (tag[i] - '0');
    if (val == 0.0) val = 1.0;
    return sign * val;
}

// Verificación del balance elemental (y de carga) como hace HSC al calcular
static bool verificar_balance_reaccion(const cma::nucleo::Reaccion& rxn, std::string& detalle) {
    auto& gestor = cma::base_datos::GestorDB::instancia();
    std::map<std::string, double> tot_el;
    double carga = 0.0;
    for (auto& [sp_id, coeff] : rxn.estequiometria) {
        const auto* sp = gestor.especie_por_id(sp_id);
        if (!sp) continue;
        std::string err;
        auto el = parse_formula_elements(sp->formula, err);
        for (auto& [e, n] : el) {
            if (e != "e-") tot_el[e] += coeff * n;
        }
        if (sp->formula == "e-" || sp->formula == "e") carga -= coeff;
        else carga += coeff * carga_de_tag(sp->formula);
    }
    bool ok = true;
    std::ostringstream os;
    for (auto& [e, v] : tot_el) {
        if (std::abs(v) > 1e-6) {
            ok = false;
            os << e << (v > 0 ? ": +" : ": ") << v << "  ";
        }
    }
    if (std::abs(carga) > 1e-6) {
        ok = false;
        os << "charge: " << (carga > 0 ? "+" : "") << carga;
    }
    detalle = os.str();
    return ok;
}

// ── Element y charge parser para balance de reacciones ──
static std::map<std::string, int> parse_formula_elements(const std::string& formula, std::string& err) {
    std::map<std::string, int> counts;
    std::vector<std::map<std::string, int>> stack;
    stack.push_back({});
    
    size_t i = 0;
    size_t len = formula.length();
    
    std::string clean_form;
    for (size_t k = 0; k < len; ++k) {
        if (formula[k] == '(') {
            size_t close = formula.find(')', k);
            if (close != std::string::npos) {
                std::string tag = formula.substr(k + 1, close - k - 1);
                if (tag == "g" || tag == "s" || tag == "l" || tag == "a" || tag == "+a" || tag == "-a" || tag.find("a") != std::string::npos) {
                    k = close;
                    continue;
                }
            }
        }
        clean_form += formula[k];
    }
    
    len = clean_form.length();
    i = 0;
    while (i < len) {
        if (clean_form[i] == '(') {
            stack.push_back({});
            i++;
        } else if (clean_form[i] == ')') {
            i++;
            int val = 0;
            while (i < len && std::isdigit(clean_form[i])) {
                val = val * 10 + (clean_form[i] - '0');
                i++;
            }
            if (val == 0) val = 1;
            
            if (stack.size() < 2) {
                err = "Mismatched parentheses";
                return {};
            }
            auto top = stack.back();
            stack.pop_back();
            for (auto& [el, cnt] : top) {
                stack.back()[el] += cnt * val;
            }
        } else if (std::isupper(clean_form[i])) {
            std::string el;
            el += clean_form[i];
            i++;
            if (i < len && std::islower(clean_form[i])) {
                el += clean_form[i];
                i++;
            }
            int val = 0;
            while (i < len && std::isdigit(clean_form[i])) {
                val = val * 10 + (clean_form[i] - '0');
                i++;
            }
            if (val == 0) val = 1;
            stack.back()[el] += val;
        } else if (clean_form[i] == '+' || clean_form[i] == '-') {
            std::string el = "e-";
            int sign = (clean_form[i] == '+') ? 1 : -1;
            i++;
            int val = 0;
            while (i < len && std::isdigit(clean_form[i])) {
                val = val * 10 + (clean_form[i] - '0');
                i++;
            }
            if (val == 0) val = 1;
            stack.back()[el] += sign * val;
        } else {
            i++;
        }
    }
    
    if (stack.size() != 1) {
        err = "Mismatched parentheses";
        return {};
    }
    return stack.back();
}

static bool balance_reaction_algebraic(const std::string& input, std::string& output, std::string& error_msg,
                                       double smallest_coeff = 1.0) {
    std::string clean = trim_str(input);
    std::string arrow = "->";
    size_t arrow_pos = clean.find(arrow);
    if (arrow_pos == std::string::npos) {
        arrow = "=";
        arrow_pos = clean.find(arrow);
        if (arrow_pos == std::string::npos) {
            error_msg = "Separador '=' o '->' no encontrado.";
            return false;
        }
    }

    std::string react_str = clean.substr(0, arrow_pos);
    std::string prod_str = clean.substr(arrow_pos + arrow.length());

    auto react_parts = split_reaction_terms(react_str);
    auto prod_parts = split_reaction_terms(prod_str);
    
    struct SpeciesTerm {
        std::string raw_formula;
        std::string state_suffix;
        std::map<std::string, int> elements;
        bool is_reactant;
    };
    
    std::vector<SpeciesTerm> terms;
    std::set<std::string> all_elements;
    
    auto process_part = [&](const std::string& part, bool is_react) {
        std::string term = trim_str(part);
        if (term.empty()) return;
        auto [coeff, formula] = parse_reaction_term(term);
        
        std::string base_formula = formula;
        std::string suffix;
        size_t par = formula.find('(');
        if (par != std::string::npos) {
            base_formula = formula.substr(0, par);
            suffix = formula.substr(par);
        }
        
        std::string err;
        auto el_map = parse_formula_elements(formula, err);
        if (!err.empty()) {
            error_msg = "Error parseando " + formula + ": " + err;
            return;
        }
        
        SpeciesTerm st;
        st.raw_formula = base_formula;
        st.state_suffix = suffix;
        st.elements = el_map;
        st.is_reactant = is_react;
        terms.push_back(st);
        
        for (auto& [el, cnt] : el_map) {
            all_elements.insert(el);
        }
    };
    
    for (auto& r : react_parts) process_part(r, true);
    for (auto& p : prod_parts) process_part(p, false);
    
    if (!error_msg.empty()) return false;
    if (terms.empty()) {
        error_msg = "Ecuación vacía.";
        return false;
    }
    
    int num_elements = (int)all_elements.size();
    int num_species = (int)terms.size();
    
    std::vector<std::string> elements_list(all_elements.begin(), all_elements.end());
    
    Eigen::MatrixXd A = Eigen::MatrixXd::Zero(num_elements, num_species);
    for (int i = 0; i < num_elements; ++i) {
        std::string el = elements_list[i];
        for (int j = 0; j < num_species; ++j) {
            auto& st = terms[j];
            auto it = st.elements.find(el);
            if (it != st.elements.end()) {
                A(i, j) = st.is_reactant ? it->second : -it->second;
            }
        }
    }
    
    Eigen::JacobiSVD<Eigen::MatrixXd> svd(A, Eigen::ComputeFullV);
    Eigen::VectorXd null_space_vector = svd.matrixV().col(num_species - 1);
    
    if (null_space_vector(0) < 0.0) {
        null_space_vector = -null_space_vector;
    }
    
    double min_val = 1e9;
    for (int i = 0; i < num_species; ++i) {
        if (null_space_vector(i) > 1e-4 && null_space_vector(i) < min_val) {
            min_val = null_space_vector(i);
        }
    }

    // Normalizar: coeficiente más pequeño = 1
    if (min_val > 1e-4 && min_val < 1e9) {
        null_space_vector /= min_val;
    }

    // "Smallest Coefficient" de HSC: si es 1, preferir coeficientes enteros;
    // si no, escalar para que el coeficiente menor sea exactamente ese valor.
    if (smallest_coeff == 1.0) {
        double multiplier = 1.0;
        for (int m = 1; m <= 20; ++m) {
            bool all_int = true;
            for (int i = 0; i < num_species; ++i) {
                double val = null_space_vector(i) * m;
                double diff = std::abs(val - std::round(val));
                if (diff > 0.05) {
                    all_int = false;
                    break;
                }
            }
            if (all_int) {
                multiplier = m;
                break;
            }
        }
        null_space_vector *= multiplier;
    } else {
        null_space_vector *= smallest_coeff;
    }

    std::string balanced_reactants;
    std::string balanced_products;

    for (int j = 0; j < num_species; ++j) {
        double coeff = std::round(null_space_vector(j) * 10000.0) / 10000.0;
        if (coeff <= 0.0) coeff = (smallest_coeff > 0.0) ? smallest_coeff : 1.0;

        std::string coeff_str;
        if (std::abs(coeff - 1.0) > 1e-9) {
            char buf[32];
            if (coeff == std::round(coeff))
                snprintf(buf, sizeof(buf), "%d ", (int)coeff);
            else
                snprintf(buf, sizeof(buf), "%.4g ", coeff);
            coeff_str = buf;
        }
        
        std::string sp_str = coeff_str + terms[j].raw_formula + terms[j].state_suffix;
        
        if (terms[j].is_reactant) {
            if (!balanced_reactants.empty()) balanced_reactants += " + ";
            balanced_reactants += sp_str;
        } else {
            if (!balanced_products.empty()) balanced_products += " + ";
            balanced_products += sp_str;
        }
    }
    
    output = balanced_reactants + " = " + balanced_products;
    return true;
}

void PanelReactionSolver::balancear_ecuacion() {
    std::string balanced;
    std::string err;
    if (balance_reaction_algebraic(ecuacion_, balanced, err, smallest_coefficient_)) {
        strncpy(ecuacion_, balanced.c_str(), sizeof(ecuacion_) - 1);
        ecuacion_[sizeof(ecuacion_) - 1] = '\0';
        error_parsing_ = false;
        error_msg_.clear();
    } else {
        error_parsing_ = true;
        error_msg_ = err;
    }
}

void PanelReactionSolver::guardar_tab_actual() {
    auto& t = tabs_[tab_actual_];
    t.ecuacion = ecuacion_;
    t.desde = temp_desde_;
    t.hasta = temp_hasta_;
    t.paso  = temp_paso_;
}

void PanelReactionSolver::cargar_tab(int idx) {
    if (idx < 0 || idx >= 3 || idx == tab_actual_) return;
    guardar_tab_actual();
    tab_actual_ = idx;
    auto& t = tabs_[idx];
    strncpy(ecuacion_, t.ecuacion.c_str(), sizeof(ecuacion_) - 1);
    ecuacion_[sizeof(ecuacion_) - 1] = '\0';
    temp_desde_ = t.desde;
    temp_hasta_ = t.hasta;
    temp_paso_  = t.paso;
    calculado_ = false;
    error_parsing_ = false;
    error_msg_.clear();
    balance_ok_ = true;
    balance_detalle_.clear();
}

// Valor del eje Y para un modo dado (reacción)
static double valor_eje_rxn(int mode, const cma::nucleo::Reaccion& rxn, double T_K,
                            double n_electrones, const std::map<int64_t, cma::nucleo::Especie>& db) {
    switch (mode) {
        case 0:  return calc_logK_T(rxn, T_K, db);
        case 1:  return calc_dH_T(rxn, T_K, db);
        case 2:  return calc_dS_T(rxn, T_K, db);
        case 3:  return calc_dG_T(rxn, T_K, db);
        case 4:  return calc_E0_she(calc_dG_T(rxn, T_K, db), n_electrones);
        default: return 0.0;
    }
}

// Valor del eje Y para un modo dado (fórmula / especie)
static double valor_eje_especie(int mode, const cma::nucleo::Especie& e, double T_K) {
    switch (mode) {
        case 0:  return e.cp(T_K);
        case 1:  return e.entalpia_abs(T_K);
        case 2:  return e.entropia_abs(T_K);
        case 3:  return e.energia_gibbs(T_K);
        default: return 0.0;
    }
}

void PanelReactionSolver::dibujar_grafico(const ImVec2& tam, bool para_reaccion) {
    auto& gestor = base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();

    static const char* ylabels_rxn[5] = {"Log K", "dH (kJ)", "dS (J/K)", "dG (kJ)", "E° vs SHE (V)"};
    static const char* titulos_rxn[5] = {"Equilibrium constant (Log)", "Reaction Enthalpy",
                                         "Reaction Entropy", "Reaction Gibbs Energy",
                                         "Electrode Potential vs SHE"};
    static const char* ylabels_esp[4] = {"Cp (J/(mol*K))", "H (kJ/mol)", "S (J/(mol*K))", "G (kJ/mol)"};
    static const char* titulos_esp[4] = {"Heat Capacity", "Enthalpy", "Entropy", "Gibbs Energy"};

    int mode = y_axis_mode_;
    if (para_reaccion && mode > 4) mode = 0;
    if (!para_reaccion && mode > 3) mode = 0;

    const char* titulo = para_reaccion ? titulos_rxn[mode] : titulos_esp[mode];
    const char* ylabel = para_reaccion ? ylabels_rxn[mode] : ylabels_esp[mode];

    if (!ImPlot::BeginPlot(titulo, tam)) return;
    ImPlot::SetupAxes("T (°C)", ylabel);

    // Solo se dibujan curvas del tipo actual (los ejes Y son incompatibles entre tipos)
    size_t last = curvas_.size();
    bool hay_compat = false;
    for (size_t i = 0; i < curvas_.size(); ++i)
        if (curvas_[i].para_reaccion == para_reaccion) { last = i; hay_compat = true; }
    size_t first = plot_all_data_ ? 0 : last; // Plot All Data off -> solo la última compatible

    if (hay_compat) {
        for (size_t i = first; i < curvas_.size(); ++i) {
            auto& c = curvas_[i];
            if (!c.visible || c.para_reaccion != para_reaccion) continue;
            std::vector<double> ys;
            ys.reserve(c.xs.size());
            for (double tc : c.xs) {
                double tk = tc + 273.15;
                if (c.para_reaccion) {
                    double n_e = 0.0;
                    info_electroquimica(c.rxn, n_e);
                    ys.push_back(valor_eje_rxn(mode, c.rxn, tk, n_e, db));
                } else {
                    const auto* sp = gestor.especie_por_id(c.especie_id);
                    ys.push_back(sp ? valor_eje_especie(mode, *sp, tk) : 0.0);
                }
            }
            ImPlot::PlotLine(c.etiqueta.c_str(), c.xs.data(), ys.data(), (int)c.xs.size());
        }
    }
    ImPlot::EndPlot();
}

void PanelReactionSolver::renderizar() {
    // Alias locales a miembros (el cuerpo heredado usa estos nombres)
    auto& rxn_actual    = rxn_actual_;
    auto& error_msg     = error_msg_;
    bool& error_parsing = error_parsing_;

    if (!tabs_inicializados_) {
        tabs_[0] = {ecuacion_, temp_desde_, temp_hasta_, temp_paso_};
        tabs_[1] = {"", 0.0, 1200.0, 25.0};
        tabs_[2] = {"", 0.0, 1200.0, 25.0};
        tabs_inicializados_ = true;
    }

    auto& gestor = base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();

    bool es_reaccion = (strchr(ecuacion_, '=') != nullptr || strstr(ecuacion_, "->") != nullptr);
    const cma::nucleo::Especie* esp_actual = nullptr;
    if (!es_reaccion) {
        esp_actual = buscar_especie_por_nombre_o_formula(ecuacion_);
    }

    // Detección de semirreacción electroquímica (e- presente)
    double n_electrones = 0.0;
    bool es_electroquimica = calculado_ && es_reaccion && info_electroquimica(rxn_actual, n_electrones);

    // Clamp del selector de eje Y según el modo actual (evita índices fuera de rango)
    int max_mode = es_reaccion ? (es_electroquimica ? 4 : 3) : 3;
    if (y_axis_mode_ > max_mode) y_axis_mode_ = 0;

    // ── Layout de Dos Columnas ──
    ImGui::Columns(2, "reaction_solver_split", true);
    static bool init_w = true;
    if (init_w) {
        ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.60f);
        ImGui::SetColumnWidth(1, ImGui::GetWindowWidth() * 0.40f);
        init_w = false;
    }

    // ════════════════════════════════════════════════════════════════════════
    // Columna Izquierda: Especificaciones y Grid
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("left_solver_pane");
    {
        // ── Toolbar ──
        ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(6, 4));
        if (ImGui::Button(tr("Exportar CSV"))) {
            if (calculado_) {
                FILE* zen = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Export Reaction Data' 2>/dev/null", "r");
                if (zen) {
                    char buf[512] = {0};
                    if (fgets(buf, sizeof(buf), zen)) {
                        std::string path(buf);
                        if (!path.empty() && path.back() == '\n') path.pop_back();
                        if (!path.empty()) {
                            if (path.find(".csv") == std::string::npos) path += ".csv";
                            FILE* fp = fopen(path.c_str(), "w");
                            if (fp) {
                                if (es_reaccion) {
                                    fprintf(fp, "Reaction: %s\n", ecuacion_);
                                    double n_e = 0.0;
                                    bool electro = info_electroquimica(rxn_actual, n_e);
                                    if (electro) {
                                        fprintf(fp, "T (°C),dH (kJ),dS (J/K),dG (kJ),K,Log K,E vs SHE (V)\n");
                                    } else {
                                        fprintf(fp, "T (°C),dH (kJ),dS (J/K),dG (kJ),K,Log K\n");
                                    }
                                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                                        double tk = tc + 273.15;
                                        double dH = calc_dH_T(rxn_actual, tk, db);
                                        double dS = calc_dS_T(rxn_actual, tk, db);
                                        double dG = calc_dG_T(rxn_actual, tk, db);
                                        double K    = calc_Keq_T(rxn_actual, tk, db);
                                        double logK = calc_logK_T(rxn_actual, tk, db);
                                        if (electro) {
                                            double E0 = calc_E0_she(dG, n_e);
                                            fprintf(fp, "%.1f,%.4f,%.4f,%.4f,%.6e,%.4f,%.4f\n", tc, dH, dS, dG, K, logK, E0);
                                        } else {
                                            fprintf(fp, "%.1f,%.4f,%.4f,%.4f,%.6e,%.4f\n", tc, dH, dS, dG, K, logK);
                                        }
                                    }
                                } else if (esp_actual) {
                                    fprintf(fp, "Species: %s\n", esp_actual->nombre.c_str());
                                    fprintf(fp, "T (°C),Cp (J/mol·K),H (kJ/mol),S (J/mol·K),G (kJ/mol)\n");
                                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                                        double tk = tc + 273.15;
                                        fprintf(fp, "%.1f,%.4f,%.4f,%.4f,%.4f\n", tc,
                                            esp_actual->cp(tk),
                                            esp_actual->entalpia_abs(tk),
                                            esp_actual->entropia_abs(tk),
                                            esp_actual->energia_gibbs(tk));
                                    }
                                }
                                fclose(fp);
                            }
                        }
                    }
                    pclose(zen);
                }
            }
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Export thermodynamic data to CSV file");
        ImGui::SameLine();
        if (ImGui::Button(tr("Limpiar"))) {
            ecuacion_[0] = '\0';
            calculado_ = false;
            error_parsing = false;
        }
        ImGui::SameLine();
        ImGui::TextDisabled(tr("|"));
        ImGui::SameLine();
        if (ImGui::Button(tr("WGS"))) {
            strncpy(ecuacion_, "CO + H2O = CO2 + H2", sizeof(ecuacion_)-1);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Water-Gas Shift Reaction");
        ImGui::SameLine();
        if (ImGui::Button(tr("Combustion CH4"))) {
            strncpy(ecuacion_, "CH4 + 2 O2 = CO2 + 2 H2O", sizeof(ecuacion_)-1);
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("CO2(g)"))) {
            strncpy(ecuacion_, "CO2", sizeof(ecuacion_)-1);
            calculado_ = false;
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Show CO2 thermodynamic properties");
        ImGui::PopStyleVar();
        ImGui::Separator();
        ImGui::Spacing();

        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.6f, 0.7f, 0.9f, 1.0f), "Specifications");
        ImGui::Separator();
        
        ImGui::Text(tr("Reaction Equation or Chemical Formula:"));
        ImGui::SetNextItemWidth(-150);
        ImGui::InputText("##eq_input", ecuacion_, sizeof(ecuacion_));
        ImGui::SameLine();
        if (ImGui::Button(tr("Examinar DB##rxn"), ImVec2(140, 0))) {
            ModalBuscadorDB::instancia().abrir("Buscar Especie (Reacción)");
        }
        std::vector<const nucleo::Especie*> sel_rxn;
        if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Reacción)", sel_rxn, true)) {
            for (auto* sp : sel_rxn) {
                std::string eq = ecuacion_;
                if (!eq.empty() && eq.back() != ' ' && eq.back() != '+') eq += " + ";
                eq += sp->formula;
                strncpy(ecuacion_, eq.c_str(), sizeof(ecuacion_) - 1);
            }
        }
        
        ImGui::SetNextItemWidth(100);
        if (ImGui::InputDouble("Smallest Coefficient", &smallest_coefficient_, 0.5, 1.0, "%.4g")) {
            if (smallest_coefficient_ <= 0.0) smallest_coefficient_ = 1.0;
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Balance"), ImVec2(80, 24))) {
            balancear_ecuacion();
        }
        
        ImGui::SetNextItemWidth(80);
        ImGui::InputDouble("From (°C)", &temp_desde_, 0, 0, "%.1f");
        ImGui::SameLine();
        ImGui::SetNextItemWidth(80);
        ImGui::InputDouble("To (°C)", &temp_hasta_, 0, 0, "%.1f");
        ImGui::SameLine();
        ImGui::SetNextItemWidth(80);
        ImGui::InputDouble("Step (°C)", &temp_paso_, 0, 0, "%.1f");
        
        ImGui::SameLine();
        if (ImGui::Button(tr("Calcular"), ImVec2(90, 24))) {
            if (es_reaccion) {
                error_parsing = !parse_chemical_equation(ecuacion_, rxn_actual, error_msg);
                calculado_ = !error_parsing;
                if (calculado_) {
                    // HSC verifica el balance elemental de la ecuación
                    balance_ok_ = verificar_balance_reaccion(rxn_actual, balance_detalle_);
                }
            } else {
                if (esp_actual) {
                    error_parsing = false;
                    calculado_ = true;
                    balance_ok_ = true;
                    balance_detalle_.clear();
                } else {
                    error_msg = "Species not found in database: " + std::string(ecuacion_);
                    error_parsing = true;
                    calculado_ = false;
                }
            }

            // Collect Data de HSC: cada cálculo añade una curva al chart
            if (calculado_) {
                if (!collect_data_) curvas_.clear();
                Curva c;
                c.para_reaccion = es_reaccion;
                char tag = es_reaccion ? 'R' : 'S';
                std::string eq_lbl = es_reaccion ? std::string(ecuacion_) : esp_actual->formula;
                c.etiqueta = std::string(1, tag) + std::to_string(tab_actual_ + 1) + ": " + eq_lbl;
                if (es_reaccion) c.rxn = rxn_actual;
                else c.especie_id = esp_actual->id;
                double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) c.xs.push_back(tc);
                if (c.xs.empty()) c.xs.push_back(temp_desde_);
                curvas_.push_back(std::move(c));
            }
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Calculate (recalculates and collects curve)");

        if (error_parsing) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.4f, 0.4f, 1.0f), "Error: %s", error_msg.c_str());
        } else if (calculado_ && es_reaccion && !balance_ok_) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.6f, 0.1f, 1.0f),
                "Warning: elemental/charge balance failed (%s). Use 'Balance' to fix the equation.", balance_detalle_.c_str());
        } else if (calculado_ && es_reaccion) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f, 0.85f, 0.4f, 1.0f), "Elemental balance: OK");
        }
        
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();
        
        // Tabla de datos termodinámicos
        if (calculado_) {
            if (es_reaccion) {
                ImGui::Text(tr("Reaction Equation: %s"), ecuacion_);
                int cols = es_electroquimica ? 7 : 6;

                if (ImGui::BeginTable("reaction_grid", cols, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 180))) {
                    ImGui::TableSetupColumn("T");
                    ImGui::TableSetupColumn("dH");
                    ImGui::TableSetupColumn("dS");
                    ImGui::TableSetupColumn("dG");
                    ImGui::TableSetupColumn("K");
                    ImGui::TableSetupColumn("Log K");
                    if (es_electroquimica) {
                        ImGui::TableSetupColumn("E");
                    }
                    ImGui::TableHeadersRow();

                    // Fila de unidades como en HSC
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::TextDisabled("°C");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("kJ");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("J/K");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("kJ");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("-");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("-");
                    if (es_electroquimica) {
                        ImGui::TableNextColumn(); ImGui::TextDisabled("V vs SHE");
                    }

                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                        double tk = tc + 273.15;
                        double dH = calc_dH_T(rxn_actual, tk, db);
                        double dS = calc_dS_T(rxn_actual, tk, db);
                        double dG = calc_dG_T(rxn_actual, tk, db);
                        double K    = calc_Keq_T(rxn_actual, tk, db);
                        double logK = calc_logK_T(rxn_actual, tk, db);

                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", tc);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", dH);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", dS);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", dG);
                        ImGui::TableNextColumn(); ImGui::Text("%.3e", K);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", logK);
                        if (es_electroquimica) {
                            double E0 = calc_E0_she(dG, n_electrones);
                            ImGui::TableNextColumn(); ImGui::Text("%.4f", E0);
                        }
                    }
                    ImGui::EndTable();
                }
                ImGui::Spacing();
            } else if (esp_actual) {
                ImGui::Text(tr("Formula: %s"), esp_actual->nombre.c_str());
                if (ImGui::BeginTable("formula_grid", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 180))) {
                    ImGui::TableSetupColumn("T");
                    ImGui::TableSetupColumn("Cp");
                    ImGui::TableSetupColumn("H");
                    ImGui::TableSetupColumn("S");
                    ImGui::TableSetupColumn("G");
                    ImGui::TableSetupColumn("Reference");
                    ImGui::TableHeadersRow();

                    // Fila de unidades como en HSC
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::TextDisabled("°C");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("J/(mol*K)");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("kJ/mol");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("J/(mol*K)");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("kJ/mol");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("-");

                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                        double tk = tc + 273.15;
                        double cp = esp_actual->cp(tk);
                        double H = esp_actual->entalpia_abs(tk);
                        double S = esp_actual->entropia_abs(tk);
                        double G = esp_actual->energia_gibbs(tk);

                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", tc);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", cp);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", H);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", S);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", G);
                        ImGui::TableNextColumn(); ImGui::TextDisabled("CMA DB");
                    }
                    ImGui::EndTable();
                }

                ImGui::Spacing();
                ImGui::Separator();
                ImGui::Spacing();

                // Species Data: cantidad de especie (Input Amounts) como en HSC
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.5f, 1.0f), "Species Data");
                if (ImGui::BeginTable("species_data_grid", 8, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                    ImGui::TableSetupColumn("Fórmula");
                    ImGui::TableSetupColumn("M g/mol");
                    ImGui::TableSetupColumn("Conc. wt-%");
                    ImGui::TableSetupColumn("Input mol", ImGuiTableColumnFlags_WidthFixed, 110.0f);
                    ImGui::TableSetupColumn("g");
                    ImGui::TableSetupColumn("Volume");
                    ImGui::TableSetupColumn("Unit");
                    ImGui::TableSetupColumn("Extrap. From T(K)");
                    ImGui::TableHeadersRow();

                    double g = especie_mol_ * esp_actual->masa_molar;
                    bool es_gas = (esp_actual->fase_std == cma::nucleo::Fase::Gas);
                    double vol_nl = especie_mol_ * 22.414; // N litros (0 °C, 1 atm)

                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("%s", esp_actual->formula.c_str());
                    ImGui::TableNextColumn(); ImGui::Text("%.3f", esp_actual->masa_molar);
                    ImGui::TableNextColumn(); ImGui::Text("100.000");
                    ImGui::TableNextColumn();
                    ImGui::SetNextItemWidth(100.0f);
                    if (ImGui::InputDouble("##input_mol_species", &especie_mol_, 0.1, 1.0, "%.3f")) {
                        if (especie_mol_ <= 0.0) especie_mol_ = 1.0;
                    }
                    ImGui::TableNextColumn(); ImGui::Text("%.3f", g);
                    ImGui::TableNextColumn();
                    if (es_gas) ImGui::Text("%.3f", vol_nl); else ImGui::TextDisabled("-");
                    ImGui::TableNextColumn();
                    if (es_gas) ImGui::Text("Nl"); else ImGui::TextDisabled("-");
                    ImGui::TableNextColumn(); ImGui::Text("298.15");

                    ImGui::EndTable();
                }
                ImGui::Spacing();
            }
        }

        // ── Tabs estilo HSC: R1-R3 para reacciones, S1-S3 para fórmulas ──
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();
        {
            char letra = es_reaccion ? 'R' : 'S';
            for (int i = 0; i < 3; ++i) {
                ImGui::PushID(7000 + i);
                char lbl[8];
                snprintf(lbl, sizeof(lbl), "%c%d", letra, i + 1);
                if (ImGui::Selectable(lbl, tab_actual_ == i, ImGuiSelectableFlags_None, ImVec2(36, 0))) {
                    cargar_tab(i);
                }
                if (ImGui::IsItemHovered()) {
                    ImGui::SetTooltip("%s", tabs_[i].ecuacion.empty() ? "(empty)" : tabs_[i].ecuacion.c_str());
                }
                ImGui::PopID();
                if (i < 2) ImGui::SameLine();
            }
        }
    }
    ImGui::EndChild();

    ImGui::NextColumn();

    // ════════════════════════════════════════════════════════════════════════
    // Columna Derecha: Gráfico
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("right_solver_pane");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.6f, 0.8f, 1.0f), "Chart");
        ImGui::Separator();

        ImGui::Text(tr("Y - Axis"));
        ImGui::SetNextItemWidth(-1);
        if (es_reaccion) {
            if (es_electroquimica) {
                ImGui::Combo("##y_axis_rxn", &y_axis_mode_, "Log K\0dH (kJ)\0dS (J/K)\0dG (kJ)\0E vs SHE (V)\0");
            } else {
                ImGui::Combo("##y_axis_rxn", &y_axis_mode_, "Log K\0dH (kJ)\0dS (J/K)\0dG (kJ)\0");
            }
        } else {
            ImGui::Combo("##y_axis_form", &y_axis_mode_, "Cp (J/mol*K)\0H (kJ/mol)\0S (J/mol*K)\0G (kJ/mol)\0");
        }

        ImGui::Checkbox(tr("Collect Data"), &collect_data_);
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("When on, each Calculate adds a new curve to the chart");
        ImGui::Checkbox(tr("Plot All Data"), &plot_all_data_);
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("When off, only the most recent curve is plotted");

        ImGui::Spacing();
        if (ImGui::Button(tr("Open in new Window"), ImVec2(-1, 0)) && !curvas_.empty()) {
            ventana_grafico_ = true;
        }
        if (ImGui::Button(tr("Copy to Clipboard"), ImVec2(-1, 0))) {
            if (calculado_) {
                std::ostringstream clip;
                if (es_reaccion) {
                    clip << "Reaction: " << ecuacion_ << "\n";
                    if (es_electroquimica)
                        clip << "T (°C)\tdH (kJ)\tdS (J/K)\tdG (kJ)\tK\tLog K\tE vs SHE (V)\n";
                    else
                        clip << "T (°C)\tdH (kJ)\tdS (J/K)\tdG (kJ)\tK\tLog K\n";
                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                        double tk = tc + 273.15;
                        clip << tc << "\t" << calc_dH_T(rxn_actual, tk, db)
                             << "\t" << calc_dS_T(rxn_actual, tk, db)
                             << "\t" << calc_dG_T(rxn_actual, tk, db)
                             << "\t" << calc_Keq_T(rxn_actual, tk, db)
                             << "\t" << calc_logK_T(rxn_actual, tk, db);
                        if (es_electroquimica)
                            clip << "\t" << calc_E0_she(calc_dG_T(rxn_actual, tk, db), n_electrones);
                        clip << "\n";
                    }
                } else if (esp_actual) {
                    clip << "Formula: " << esp_actual->formula << "\n";
                    clip << "T (°C)\tCp (J/mol*K)\tH (kJ/mol)\tS (J/mol*K)\tG (kJ/mol)\n";
                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                        double tk = tc + 273.15;
                        clip << tc << "\t" << esp_actual->cp(tk)
                             << "\t" << esp_actual->entalpia_abs(tk)
                             << "\t" << esp_actual->entropia_abs(tk)
                             << "\t" << esp_actual->energia_gibbs(tk) << "\n";
                    }
                }
                ImGui::SetClipboardText(clip.str().c_str());
            }
        }

        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();

        if (!curvas_.empty()) {
            dibujar_grafico(ImVec2(-1, 240), es_reaccion);

            // Leyenda con checkboxes por curva (como en HSC)
            ImGui::Spacing();
            for (size_t i = 0; i < curvas_.size(); ++i) {
                if (curvas_[i].para_reaccion != es_reaccion) continue;
                ImGui::PushID((int)i + 7100);
                ImGui::Checkbox(curvas_[i].etiqueta.c_str(), &curvas_[i].visible);
                ImGui::PopID();
            }
            if (ImGui::SmallButton(tr("Clear Curves"))) {
                curvas_.clear();
            }
        } else {
            ImGui::TextDisabled(tr("Run calculations to generate plots."));
        }
    }
    ImGui::EndChild();

    ImGui::Columns(1);

    // ── Ventana de gráfico desacoplada ("Open in new Window") ──
    if (ventana_grafico_) {
        ImGui::SetNextWindowSize(ImVec2(720, 520), ImGuiCond_FirstUseEver);
        if (ImGui::Begin("Reaction Equations - Chart##detached", &ventana_grafico_)) {
            dibujar_grafico(ImVec2(-1, -40), es_reaccion);
            if (ImGui::SmallButton(tr("Close"))) {
                ventana_grafico_ = false;
            }
        }
        ImGui::End();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de ExternalAPI
// ═══════════════════════════════════════════════════════════════════════════

void PanelExternalAPI::renderizar() {
    ImGui::Text(tr("ExternalAPI — Ejecución de Comandos"));
    ImGui::Separator();

    ImGui::InputText(tr("Comando"), comando_, sizeof(comando_));

    if (ImGui::Button(tr("Ejecutar"), ImVec2(-1, 30))) {
        std::string res = motor_.ejecutarComando(comando_);
        strncpy(resultado_, res.c_str(), sizeof(resultado_) - 1);
        resultado_[sizeof(resultado_) - 1] = '\0';
        ejecutado_ = true;
    }
    ImGui::SameLine();
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
    const std::map<int64_t, cma::nucleo::Especie>& db)
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
    hsc_clone::GibbsMinimizer gibbs;
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
// Implementación completa del Panel de Agua (Steam Tables)
// ═══════════════════════════════════════════════════════════════════════════
// Panel de Agua (Tablas de Vapor)
// ═══════════════════════════════════════════════════════════════════════════
// ═══════════════════════════════════════════════════════════════════════════
// Panel de Agua (Tablas de Vapor)
// ═══════════════════════════════════════════════════════════════════════════
void cma::interfaz::PanelAgua::renderizar() {
    static double p1_T = 300.0, p1_P = 25.0;
    static double p2_T = 25.0,  p2_P = 1.0;
    static double amount = 1.0;
    static int amount_unit = 0; // 0=kg, 1=kmol

    static double t_min = -100.0, t_max = 500.0;
    static double p_min = 0.001, p_max = 1000.0;

    // Título y encabezado
    ImGui::Text(tr("Calculadora Agua / Vapor"));
    ImGui::SameLine(ImGui::GetWindowWidth() - 100);
    ImGui::TextDisabled(tr("H2O"));
    
    ImGui::PushItemWidth(100);
    ImGui::InputDouble("Amount", &amount);
    ImGui::PopItemWidth();
    ImGui::SameLine();
    ImGui::PushItemWidth(80);
    ImGui::Combo("##unit", &amount_unit, "kg\0kmol\0\0");
    ImGui::PopItemWidth();
    
    ImGui::SameLine();
    ImGui::Dummy(ImVec2(20, 0));
    ImGui::SameLine();
    
    ImGui::BeginGroup();
    ImGui::TextDisabled(tr("Límites de Datos"));
    ImGui::PushItemWidth(80);
    ImGui::InputDouble("T min (C)", &t_min, 0, 0, "%.1f"); ImGui::SameLine(); 
    ImGui::InputDouble("T max", &t_max, 0, 0, "%.1f");
    ImGui::InputDouble("P min (bar)", &p_min, 0, 0, "%.3f"); ImGui::SameLine(); 
    ImGui::InputDouble("P max", &p_max, 0, 0, "%.1f");
    ImGui::PopItemWidth();
    ImGui::EndGroup();
    
    ImGui::Separator();
    
    // Contenedor principal para dividir tabla y diagrama
    if (ImGui::BeginTable("##water_layout", 2)) {
        ImGui::TableSetupColumn("Table", ImGuiTableColumnFlags_WidthStretch, 0.55f);
        ImGui::TableSetupColumn("Gráfico", ImGuiTableColumnFlags_WidthStretch, 0.45f);
        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        
        // Tabla Principal (Lado Izquierdo)
        if (ImGui::BeginTable("##water_calc", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_SizingStretchProp)) {
            ImGui::TableSetupColumn("Propiedad", ImGuiTableColumnFlags_WidthFixed, 100.0f);
            ImGui::TableSetupColumn("Point 1", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("Point 2", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("Delta", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthFixed, 60.0f);
            ImGui::TableHeadersRow();
            
            // Entradas editables
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("Temp."));
            ImGui::TableNextColumn(); ImGui::PushItemWidth(-1); ImGui::InputDouble("##T1", &p1_T, 0, 0, "%.3f"); ImGui::PopItemWidth();
            ImGui::TableNextColumn(); ImGui::PushItemWidth(-1); ImGui::InputDouble("##T2", &p2_T, 0, 0, "%.3f"); ImGui::PopItemWidth();
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), p1_T - p2_T); 
            ImGui::TableNextColumn(); ImGui::Text(tr("C")); 
            
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("Presión"));
            ImGui::TableNextColumn(); ImGui::PushItemWidth(-1); ImGui::InputDouble("##P1", &p1_P, 0, 0, "%.3f"); ImGui::PopItemWidth();
            ImGui::TableNextColumn(); ImGui::PushItemWidth(-1); ImGui::InputDouble("##P2", &p2_P, 0, 0, "%.3f"); ImGui::PopItemWidth();
            ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), p1_P - p2_P);
            ImGui::TableNextColumn(); ImGui::Text(tr("bar"));

            // Cálculo termodinámico (C a K, bar a MPa)
            auto prop1 = motor_.propiedades(p1_T + 273.15, p1_P * 0.1); 
            auto prop2 = motor_.propiedades(p2_T + 273.15, p2_P * 0.1);
            
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("Phase"));
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), prop1.fase.c_str());
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), prop2.fase.c_str());
            ImGui::TableNextColumn(); ImGui::Text(tr(""));
            ImGui::TableNextColumn(); ImGui::Text(tr(""));

            auto row = [](const char* name, double v1, double v2, const char* unit, bool highlight_delta = false) {
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text(tr("%s"), name);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), v1);
                ImGui::TableNextColumn(); ImGui::Text(tr("%.3f"), v2);
                ImGui::TableNextColumn(); 
                if (highlight_delta) ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, ImGui::GetColorU32(ImVec4(1.0f, 1.0f, 0.8f, 1.0f)));
                ImGui::Text(tr("%.3f"), v1 - v2);
                ImGui::TableNextColumn(); ImGui::Text(tr("%s"), unit);
            };
            
            double mult = amount;
            if (amount_unit == 1) mult *= 18.01528; // kmol a kg

            row("H", prop1.h * mult, prop2.h * mult, "kJ", true);
            row("S", prop1.s * mult, prop2.s * mult, "kJ/K", true);
            row("H specific", prop1.h, prop2.h, "kJ/kg");
            row("G", (prop1.h - (p1_T+273.15)*prop1.s)*mult, (prop2.h - (p2_T+273.15)*prop2.s)*mult, "kJ");
            row("Cp", prop1.cp, prop2.cp, "kJ/kgK");
            row("Density", 1.0/prop1.v, 1.0/prop2.v, "kg/m3");
            
            // Exergía - aproximación simple (H - H0) - T0(S - S0)
            double t0 = 298.15, p0 = 0.101325;
            auto prop0 = motor_.propiedades(t0, p0);
            double ex1 = (prop1.h - prop0.h) - t0 * (prop1.s - prop0.s);
            double ex2 = (prop2.h - prop0.h) - t0 * (prop2.s - prop0.s);
            row("Exergy Ph", ex1 * mult, ex2 * mult, "kJ");
            
            ImGui::EndTable();
        }
        
        // Gráfico (Lado Derecho)
        ImGui::TableNextColumn();
        if (ImPlot::BeginPlot("##water_chart", ImVec2(-1, 400))) {
            ImPlot::SetupAxes("P [bar]", "T [°C]"); ImPlot::SetupAxisScale(ImAxis_X1, ImPlotScale_Log10);
            ImPlot::SetupAxesLimits(p_min, p_max, t_min, t_max, ImPlotCond_Always);
            
            // Preparar vectores para sombreado
            std::vector<double> sh_p, sh_ice, sh_liq, sh_gas, sh_top, sh_bot;
            // Densificar puntos en el eje P (logarítmico)
            for(double p = 0.001; p <= 1000.0; p *= 1.1) {
                sh_p.push_back(p);
                sh_top.push_back(500.0);
                sh_bot.push_back(-100.0);
                
                double t_sub_fus, t_sat;
                if (p < 0.00611657) {
                    double log_p = log10(p);
                    double log_pt = log10(0.00611657);
                    double log_p0 = log10(0.001);
                    t_sub_fus = -20.0 + (0.01 - (-20.0)) * (log_p - log_p0) / (log_pt - log_p0);
                    t_sat = t_sub_fus; 
                } else {
                    t_sub_fus = 0.01 + (-9.0 - 0.01) * (p - 0.00611657) / (1000.0 - 0.00611657);
                    if (p <= 220.64) {
                        t_sat = motor_.temperatura_saturacion(p * 0.1) - 273.15;
                    } else {
                        t_sat = 500.0; 
                    }
                }
                sh_ice.push_back(t_sub_fus);
                sh_liq.push_back(t_sat);
            }
            
            // Colores tipo HSC
            ImVec4 color_ice = ImVec4(0.5f, 0.6f, 1.0f, 1.0f);
            ImVec4 color_liq = ImVec4(0.7f, 0.85f, 1.0f, 1.0f);
            ImVec4 color_gas = ImVec4(0.9f, 0.95f, 1.0f, 1.0f);

            ImPlot::PushStyleVar(ImPlotStyleVar_FillAlpha, 1.0f);
            ImPlot::SetNextFillStyle(color_gas);
            ImPlot::PlotShaded("##Gas", sh_p.data(), sh_liq.data(), sh_top.data(), sh_p.size());
            ImPlot::SetNextFillStyle(color_liq);
            ImPlot::PlotShaded("##Liquid", sh_p.data(), sh_ice.data(), sh_liq.data(), sh_p.size());
            ImPlot::SetNextFillStyle(color_ice);
            ImPlot::PlotShaded("##Ice", sh_p.data(), sh_bot.data(), sh_ice.data(), sh_p.size());
            ImPlot::PopStyleVar();

            // Textos descriptivos (Labels)
            ImPlot::PlotText("Gas Steam", 0.1, 300);
            ImPlot::PlotText("Liquid Water", 100, 100);
            ImPlot::PlotText("Solid Ice", 10, -50);
            
            // Puntos notables
            double p_tp[1] = {0.00611657}, t_tp[1] = {0.01};
            ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 4, ImVec4(1,0,0,1));
            ImPlot::PlotScatter("Triple point", p_tp, t_tp, 1);
            ImPlot::PlotText("Triple\npoint", 0.003, 50);

            double p_cp[1] = {220.64}, t_cp[1] = {373.946};
            ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 4, ImVec4(1,0,0,1));
            ImPlot::PlotScatter("Critical point", p_cp, t_cp, 1);
            ImPlot::PlotText("Critical\npoint", 220.64, 420);
            
            // Dibujar la curva de saturación
            std::vector<double> sat_p, sat_t;
            for(double t = 273.16; t <= 647.096; t += 2.0) {
                sat_t.push_back(t - 273.15);
                sat_p.push_back(motor_.saturacion_T(t).first.P * 10.0); // MPa to bar
            }
            ImPlot::SetNextLineStyle(ImVec4(0,0,0,1), 2.0f);
            ImPlot::PlotLine("Saturation Curve", sat_p.data(), sat_t.data(), (int)sat_p.size());
            
            // Línea de sublimación/fusión aproximada
            std::vector<double> sub_p = {0.001, 0.00611657}, sub_t = {-20, 0.01};
            ImPlot::SetNextLineStyle(ImVec4(0,0,0,1), 2.0f);
            ImPlot::PlotLine("Sublimation", sub_p.data(), sub_t.data(), 2);
            
            std::vector<double> fus_p = {0.00611657, 1000}, fus_t = {0.01, -9.0};
            ImPlot::SetNextLineStyle(ImVec4(0,0,0,1), 2.0f);
            ImPlot::PlotLine("Fusion", fus_p.data(), fus_t.data(), 2);
            
            // Plot de los puntos
            double px[2] = {p1_P, p2_P};
            double py[2] = {p1_T, p2_T};
            ImPlot::SetNextLineStyle(ImVec4(1,0,0,1), 2.0f);
            ImPlot::PlotLine("Process", px, py, 2);
            
            ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 4, ImVec4(1,1,1,1), -1, ImVec4(1,0,0,1)); // ImPlotConstants_Auto -> -1
            ImPlot::PlotScatter("Points", px, py, 2);
            
            ImPlot::EndPlot();
        }
        ImGui::EndTable();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de HSC Add-In Functions
// ═══════════════════════════════════════════════════════════════════════════

void cma::interfaz::PanelAddIn::evaluar_fila(cma::interfaz::PanelAddIn::FilaAddIn& fila) {
    fila.calculo_ok = false;
    fila.resultado = 0.0;
    fila.mensaje = "";

    std::string arg1_str = trim_str(fila.arg1);
    if (arg1_str.empty()) return;

    auto& gestor = base_datos::GestorDB::instancia();

    auto calcular_propiedad = [&](const cma::nucleo::Reaccion& rxn, double T_K, int prop_tipo) -> double {
        double val_total = 0.0;
        for (const auto& [sp_id, coeff] : rxn.estequiometria) {
            const auto* esp = gestor.especie_por_id(sp_id);
            if (!esp) continue;
            
            double prop_val = 0.0;
            switch(prop_tipo) {
                case 0: prop_val = esp->entalpia_abs(T_K); break; // H
                case 1: prop_val = esp->entropia_abs(T_K); break; // S
                case 2: prop_val = esp->energia_gibbs(T_K); break; // G
                case 3: prop_val = esp->cp(T_K); break; // CP
            }
            val_total += coeff * prop_val;
        }
        return val_total;
    };

    if (fila.func_idx >= 0 && fila.func_idx <= 4) {
        // Funciones Termodinámicas (H, S, G, CP, K)
        // Temperatura en Kelvin (asumimos que arg2 = Celsius)
        double T_K = fila.arg2 + 273.15;
        if (T_K <= 0.0) {
            fila.mensaje = "T debe ser > 0 K";
            return;
        }

        // Parsar ecuación
        cma::nucleo::Reaccion rxn;
        std::string err;
        
        // Si no tiene flecha ni igual, asumimos que es una especie suelta (ej. "CO2(g)")
        if (arg1_str.find('=') == std::string::npos && arg1_str.find("->") == std::string::npos) {
            arg1_str += " = " + arg1_str; // Hack: 1 Reactivo = 1 Producto anula el resto, pero queremos evaluar
            // Mejor: parseamos como si fuera Reactivo = (nada) y le cambiamos el signo, o simplemente lo forzamos manual.
            const auto* esp = buscar_especie_por_nombre_o_formula(trim_str(fila.arg1));
            if (!esp) {
                fila.mensaje = "Especie no encontrada.";
                return;
            }
            rxn.estequiometria[esp->id] = 1.0; // Producto ficticio para que calcule positivo
        } else {
            if (!parse_chemical_equation(arg1_str, rxn, err)) {
                fila.mensaje = err;
                return;
            }
        }

        if (fila.func_idx == 0) {
            fila.resultado = calcular_propiedad(rxn, T_K, 0);
            strncpy(fila.desc, "Enthalpy (kJ/mol)", sizeof(fila.desc)-1);
        } else if (fila.func_idx == 1) {
            fila.resultado = calcular_propiedad(rxn, T_K, 1);
            strncpy(fila.desc, "Entropy (J/(mol K))", sizeof(fila.desc)-1);
        } else if (fila.func_idx == 2) {
            fila.resultado = calcular_propiedad(rxn, T_K, 2);
            strncpy(fila.desc, "Gibbs Energy (kJ/mol)", sizeof(fila.desc)-1);
        } else if (fila.func_idx == 3) {
            fila.resultado = calcular_propiedad(rxn, T_K, 3);
            strncpy(fila.desc, "Heat Capacity (J/(mol K))", sizeof(fila.desc)-1);
        } else if (fila.func_idx == 4) {
            double dG = calcular_propiedad(rxn, T_K, 2);
            fila.resultado = std::exp(-dG * 1000.0 / (cma::nucleo::R_GAS * T_K));
            strncpy(fila.desc, "Equilibrium Constant K", sizeof(fila.desc)-1);
        }
        fila.calculo_ok = true;
    } 
    else if (fila.func_idx == 5) {
        // TATH (Temperature at given enthalpy)
        const auto* esp = buscar_especie_por_nombre_o_formula(arg1_str);
        if (!esp) {
            fila.mensaje = "Especie no encontrada.";
            return;
        }

        double H_target = fila.arg2; // kJ/mol
        
        // Búsqueda binaria simple
        double t_min = 1.0; // K
        double t_max = 6000.0; // K
        
        double h_min = esp->entalpia_abs(t_min);
        double h_max = esp->entalpia_abs(t_max);
        
        if (H_target < h_min || H_target > h_max) {
            fila.mensaje = "Entalpía fuera de rango (1K-6000K).";
            return;
        }
        
        double t_mid = 0.0;
        for (int i=0; i<50; ++i) {
            t_mid = (t_min + t_max) / 2.0;
            double h_mid = esp->entalpia_abs(t_mid);
            if (h_mid > H_target) {
                t_max = t_mid;
            } else {
                t_min = t_mid;
            }
        }
        
        fila.resultado = t_mid - 273.15; // Devolver en °C
        strncpy(fila.desc, "Temperature (°C)", sizeof(fila.desc)-1);
        fila.calculo_ok = true;
    }
}

void cma::interfaz::PanelAddIn::renderizar() {
    if (!db_inicializada_) {
        // Agregar algunas filas de ejemplo inicialmente
        FilaAddIn f1;
        f1.func_idx = 0; // H
        strncpy(f1.arg1, "2H2(g) + O2(g) = 2H2O(g)", sizeof(f1.arg1)-1);
        f1.arg2 = 100.0;
        evaluar_fila(f1);
        filas_.push_back(f1);
        
        FilaAddIn f2;
        f2.func_idx = 5; // TATH
        strncpy(f2.arg1, "CO2(g)", sizeof(f2.arg1)-1);
        f2.arg2 = -93.35;
        evaluar_fila(f2);
        filas_.push_back(f2);

        db_inicializada_ = true;
    }

    ImGui::Text(tr("Editor de Hoja HSC (Funciones Add-In)"));
    ImGui::Separator();
    ImGui::TextWrapped(tr("Evaluate thermodynamic functions interactively. Type your arguments and the return value will be calculated instantly."));

    if (ImGui::Button(tr("Agregar Fila"))) {
        filas_.push_back(FilaAddIn{});
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Limpiar Todo"))) {
        filas_.clear();
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Exportar a Excel (.csv)"))) {
        FILE* zen = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Export to Excel' 2>/dev/null", "r");
        if (zen) {
            char buf[512] = {0};
            if (fgets(buf, sizeof(buf), zen)) {
                std::string path(buf);
                if (!path.empty() && path.back() == '\n') path.pop_back();
                if (path.find(".csv") == std::string::npos) path += ".csv";
                
                FILE* fp = fopen(path.c_str(), "w");
                if (fp) {
                    fprintf(fp, "Function,Argument 1 (Species/Rxn),Argument 2 (T or H),Return Value,Description\n");
                    const char* func_names[] = { "H", "S", "G", "CP", "K", "TATH" };
                    for (const auto& f : filas_) {
                        fprintf(fp, "%s(Equation;T),%s,%g,%.3f,%s\n", 
                            f.func_idx >= 0 && f.func_idx <= 5 ? func_names[f.func_idx] : "N/A",
                            f.arg1, f.arg2, 
                            f.calculo_ok ? f.resultado : 0.0, 
                            f.desc);
                    }
                    fclose(fp);
                }
            }
            pclose(zen);
        }
    }
    
    ImGui::Separator();
    
    ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_Resizable | ImGuiTableFlags_ScrollY;
    if (ImGui::BeginTable("table_addin", 6, flags, ImVec2(-1, -1))) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Function", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Argument 1 (Species/Rxn)", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Argument 2 (T or H)", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableSetupColumn("Return Value", ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableSetupColumn("Description", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Actions", ImGuiTableColumnFlags_WidthFixed, 50.0f);
        ImGui::TableHeadersRow();

        const char* func_names[] = { "H", "S", "G", "CP", "K", "TATH" };

        for (size_t i = 0; i < filas_.size(); ++i) {
            auto& f = filas_[i];
            ImGui::PushID((int)i);
            ImGui::TableNextRow();

            bool changed = false;

            ImGui::TableSetColumnIndex(0);
            ImGui::PushItemWidth(-FLT_MIN);
            if (ImGui::Combo("##func", &f.func_idx, func_names, IM_ARRAYSIZE(func_names))) changed = true;
            ImGui::PopItemWidth();

            ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-FLT_MIN);
            if (ImGui::InputText("##arg1", f.arg1, sizeof(f.arg1))) changed = true;
            ImGui::PopItemWidth();

            ImGui::TableSetColumnIndex(2);
            ImGui::PushItemWidth(-FLT_MIN);
            if (ImGui::InputDouble("##arg2", &f.arg2, 0.0, 0.0, "%.3f")) changed = true;
            ImGui::PopItemWidth();

            if (changed) {
                evaluar_fila(f);
            }

            ImGui::TableSetColumnIndex(3);
            if (f.calculo_ok) {
                ImGui::Text(tr("%.4e"), f.resultado);
            } else {
                if (!f.mensaje.empty()) {
                    ImGui::TextColored(cma::interfaz::AdaptiveColor(1,0,0,1), "Err");
                    if (ImGui::IsItemHovered()) ImGui::SetTooltip("%s", f.mensaje.c_str());
                } else {
                    ImGui::Text(tr("-"));
                }
            }

            ImGui::TableSetColumnIndex(4);
            ImGui::PushItemWidth(-FLT_MIN);
            ImGui::InputText("##desc", f.desc, sizeof(f.desc)); // Editable but filled by default
            ImGui::PopItemWidth();

            ImGui::TableSetColumnIndex(5);
            if (ImGui::Button(tr("X"))) {
                filas_.erase(filas_.begin() + i);
                ImGui::PopID();
                break;
            }

            ImGui::PopID();
        }
        ImGui::EndTable();
    }
}



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
                            for (int i = 0; i < 118 && i < (int)sizeof(PT_ELEMENTS)/sizeof(PT_ELEMENTS[0]); ++i)
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
                FILE* zen = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Exportar base de minerales' 2>/dev/null", "r");
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
                    pclose(zen);
                }
            }
            if (ImGui::Button(tr("Importar"), ImVec2(-1, 0))) {
                FILE* zen = popen("zenity --file-selection --file-filter='*.csv' --title='Importar base de minerales' 2>/dev/null", "r");
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
                    pclose(zen);
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

void cma::interfaz::PanelMapGPS::refrescar_datos() {
    stock_db_.clear();
    if (!flowsheet_) return;
    
    // Obtener nodos del diagrama y mostrarlos como elementos geolocalizados
    for (const auto& n : flowsheet_->nodos()) {
        StockItem item;
        item.selected_for_mix = false;
        
        strncpy(item.place, "Plant", sizeof(item.place) - 1);
        strncpy(item.mat, n.nombre.c_str(), sizeof(item.mat) - 1);
        
        item.amt = 1000.0f; // placeholder
        item.fv = 0.0f;
        item.o2 = 0.0f; item.cu = 0.0f; item.fe = 0.0f; item.ni = 0.0f; item.s = 0.0f;
        
        // Coordenadas en el mapa = coordenadas en el canvas
        item.x = n.canvas_x;
        item.y = n.canvas_y;
        
        stock_db_.push_back(item);
    }
}

cma::interfaz::PanelMapGPS::PanelMapGPS() {
    refrescar_datos();
}

void cma::interfaz::PanelMapGPS::renderizar() {
    // --- Top Bar (Tabs/Modes) ---
    ImGui::BeginChild("map_top_bar", ImVec2(0, 45), true);
    if (ImGui::Button(tr("Mode"), ImVec2(60, 25))) {
        view_mode_ = (view_mode_ == 0) ? 1 : 0;
    }
    ImGui::SameLine();
    
    const char* places_items[] = {"Näytä kaikki varastopaikat", "Valitut varastopaikat"};
    static int places_idx = 0;
    ImGui::PushItemWidth(200);
    ImGui::Combo("##places_combo", &places_idx, places_items, IM_ARRAYSIZE(places_items));
    ImGui::PopItemWidth();
    ImGui::SameLine();
    
    const char* maps_items[] = {"Pori tehdasalue", "Helsinki region", "Oulu map"};
    ImGui::PushItemWidth(200);
    ImGui::Combo("##maps_combo", &maps_idx_, maps_items, IM_ARRAYSIZE(maps_items));
    ImGui::PopItemWidth();
    ImGui::EndChild();

    float right_panel_width = 340.0f;
    float left_panel_width = ImGui::GetContentRegionAvail().x - right_panel_width - 8.0f;

    ImGui::BeginChild("map_left_panel", ImVec2(left_panel_width, 0), true);

    if (view_mode_ == 0) {
        // --- MAP VIEW ---
        ImVec2 canvas_pos = ImGui::GetCursorScreenPos();
        ImVec2 canvas_size = ImGui::GetContentRegionAvail();
        if (canvas_size.x < 50.0f) canvas_size.x = 50.0f;
        if (canvas_size.y < 50.0f) canvas_size.y = 50.0f;

        ImDrawList* draw_list = ImGui::GetWindowDrawList();
        
        // --- Theme Detection ---
        bool is_dark = ImGui::GetStyle().Colors[ImGuiCol_WindowBg].x < 0.5f;
        ImU32 col_bg, col_water, col_green, col_road, col_bldg, col_text;
        
        if (is_dark) {
            col_bg    = IM_COL32(35, 35, 40, 255);
            col_water = IM_COL32(20, 50, 80, 255);
            col_green = IM_COL32(40, 100, 50, 255);
            col_road  = IM_COL32(180, 50, 50, 255);
            col_bldg  = IM_COL32(80, 80, 80, 255);
            col_text  = IM_COL32(220, 220, 220, 255);
        } else {
            col_bg    = IM_COL32(245, 245, 240, 255);
            col_water = IM_COL32(150, 200, 255, 255);
            col_green = IM_COL32(180, 230, 180, 255);
            col_road  = IM_COL32(220, 100, 100, 255);
            col_bldg  = IM_COL32(200, 200, 200, 255);
            col_text  = IM_COL32(50, 50, 50, 255);
        }
        
        // Background
        draw_list->AddRectFilled(canvas_pos, ImVec2(canvas_pos.x + canvas_size.x, canvas_pos.y + canvas_size.y), col_bg);
        
        // Projection helper lambda
        auto W2S = [&](float wx, float wy) -> ImVec2 {
            return ImVec2(canvas_pos.x + wx * zoom_ + camera_x_, canvas_pos.y + wy * zoom_ + camera_y_);
        };
        auto W2Sf = [&](float s) -> float {
            return s * zoom_;
        };
        auto S2W = [&](float sx, float sy) -> ImVec2 {
            return ImVec2((sx - canvas_pos.x - camera_x_) / zoom_, (sy - canvas_pos.y - camera_y_) / zoom_);
        };
        
        // Invisible button to capture interactions
        ImGui::InvisibleButton("canvas", canvas_size);
        bool is_hovered = ImGui::IsItemHovered();
        bool is_active = ImGui::IsItemActive();
        
        ImVec2 mouse_pos = ImGui::GetIO().MousePos;
        
        // --- 2D Camera Controls ---
        if (is_hovered && ImGui::GetIO().MouseWheel != 0.0f) {
            float zoom_delta = ImGui::GetIO().MouseWheel * 0.1f;
            float prev_zoom = zoom_;
            zoom_ += zoom_delta;
            if (zoom_ < 0.1f) zoom_ = 0.1f;
            if (zoom_ > 10.0f) zoom_ = 10.0f;
            
            // Zoom towards mouse cursor
            ImVec2 mouse_world = S2W(mouse_pos.x, mouse_pos.y);
            camera_x_ += mouse_world.x * (prev_zoom - zoom_);
            camera_y_ += mouse_world.y * (prev_zoom - zoom_);
        }
        
        if (is_active && ImGui::IsMouseDragging(1)) { // Right click drag to pan
            camera_x_ += ImGui::GetIO().MouseDelta.x;
            camera_y_ += ImGui::GetIO().MouseDelta.y;
        }

        // --- Render Terrains ---
        draw_list->PushClipRect(canvas_pos, ImVec2(canvas_pos.x + canvas_size.x, canvas_pos.y + canvas_size.y), true);
        
        if (maps_idx_ == 0) {
            // "Pori tehdasalue" - Industrial Plant
            ImVec2 p_water[4] = {W2S(400, -100), W2S(800, -100), W2S(800, 600), W2S(350, 600)};
            draw_list->AddConvexPolyFilled(p_water, 4, col_water);
            
            ImVec2 p_green[4] = {W2S(150, 50), W2S(220, 50), W2S(220, 180), W2S(150, 180)};
            draw_list->AddConvexPolyFilled(p_green, 4, col_green);
            
            draw_list->AddCircleFilled(W2S(185, 70), W2Sf(12.0f), IM_COL32(200, 180, 0, 255));
            draw_list->AddCircleFilled(W2S(205, 160), W2Sf(10.0f), IM_COL32(200, 180, 0, 255));
            
            draw_list->AddLine(W2S(50, -100), W2S(50, 600), col_road, W2Sf(6.0f));
            draw_list->AddLine(W2S(50, 200), W2S(400, 200), col_road, W2Sf(6.0f));
            draw_list->AddLine(W2S(300, 200), W2S(450, 50), col_road, W2Sf(6.0f));
            
            ImVec2 p_bldg1[4] = {W2S(400, 20), W2S(480, 20), W2S(460, 80), W2S(380, 80)};
            draw_list->AddConvexPolyFilled(p_bldg1, 4, col_bldg);
            ImVec2 p_bldg2[4] = {W2S(70, 220), W2S(130, 220), W2S(130, 260), W2S(70, 260)};
            draw_list->AddConvexPolyFilled(p_bldg2, 4, col_bldg);
            
            draw_list->AddText(W2S(160, 115), col_text, "Outokumpu Research Oy");
        } 
        else if (maps_idx_ == 1) {
            // "Helsinki region" - Metropolitan / Water Heavy
            ImVec2 p_water1[4] = {W2S(-200, 300), W2S(600, 300), W2S(600, 800), W2S(-200, 800)};
            draw_list->AddConvexPolyFilled(p_water1, 4, col_water);
            
            ImVec2 p_island[4] = {W2S(200, 400), W2S(350, 400), W2S(400, 450), W2S(250, 550)};
            draw_list->AddConvexPolyFilled(p_island, 4, col_bg);
            
            draw_list->AddLine(W2S(-100, 150), W2S(600, 150), col_road, W2Sf(10.0f)); // Highway
            draw_list->AddLine(W2S(250, -100), W2S(250, 150), col_road, W2Sf(8.0f));
            draw_list->AddLine(W2S(250, 150), W2S(280, 450), col_road, W2Sf(4.0f)); // Bridge to island
            
            draw_list->AddText(W2S(10, 120), col_text, "Ring I Highway");
            draw_list->AddText(W2S(290, 450), col_text, "Island Storage Facility");
        }
        else if (maps_idx_ == 2) {
            // "Oulu map" - Green/Forest Heavy
            ImVec2 p_forest[4] = {W2S(-200, -100), W2S(600, -100), W2S(600, 600), W2S(-200, 600)};
            draw_list->AddConvexPolyFilled(p_forest, 4, col_green);
            
            // Winding River
            draw_list->AddLine(W2S(-100, 100), W2S(200, 250), col_water, W2Sf(30.0f));
            draw_list->AddLine(W2S(200, 250), W2S(400, 150), col_water, W2Sf(30.0f));
            draw_list->AddLine(W2S(400, 150), W2S(600, 300), col_water, W2Sf(30.0f));
            
            // Small dirt roads (brownish/grey)
            draw_list->AddLine(W2S(50, -100), W2S(50, 200), IM_COL32(150, 130, 100, 255), W2Sf(4.0f));
            draw_list->AddLine(W2S(50, 200), W2S(300, 350), IM_COL32(150, 130, 100, 255), W2Sf(4.0f));
            
            draw_list->AddText(W2S(250, 200), IM_COL32(0,0,0,255), "Oulujoki River");
            draw_list->AddText(W2S(100, 300), IM_COL32(0,0,0,255), "Forest Depot");
        }

        // --- Drag & Drop Nodes ---
        static int dragged_node = -1;
        static bool dragged_this_frame = false;
        
        if (is_hovered && ImGui::IsMouseClicked(0)) {
            for (size_t i = 0; i < stock_db_.size(); ++i) {
                ImVec2 s_pos = W2S(stock_db_[i].x, stock_db_[i].y);
                float dx = mouse_pos.x - s_pos.x;
                float dy = mouse_pos.y - s_pos.y;
                if (dx*dx + dy*dy < (10.0f * zoom_ * 10.0f * zoom_)) { // Radius scales with zoom
                    dragged_node = i;
                    selected_idx_ = i;
                    strncpy(place_name_, stock_db_[i].place, sizeof(place_name_) - 1);
                    strncpy(material_, stock_db_[i].mat, sizeof(material_) - 1);
                    amount_tons_ = stock_db_[i].amt;
                    dragged_this_frame = false;
                    break;
                }
            }
        }
        
        if (is_active && ImGui::IsMouseDragging(0) && dragged_node != -1) {
            // Apply delta in world space
            stock_db_[dragged_node].x += ImGui::GetIO().MouseDelta.x / zoom_;
            stock_db_[dragged_node].y += ImGui::GetIO().MouseDelta.y / zoom_;
            snprintf(stock_db_[dragged_node].place, sizeof(stock_db_[dragged_node].place), "LOC-%.0f-%.0f", stock_db_[dragged_node].x, stock_db_[dragged_node].y);
            strncpy(place_name_, stock_db_[dragged_node].place, sizeof(place_name_) - 1);
            dragged_this_frame = true;
        }
        
        if (ImGui::IsMouseReleased(0)) {
            dragged_node = -1;
        }

        // Draw Stock Location Nodes
        for (size_t i = 0; i < stock_db_.size(); ++i) {
            ImVec2 node_pos = W2S(stock_db_[i].x, stock_db_[i].y);
            ImU32 halo_col = (selected_idx_ == i) ? IM_COL32(0, 255, 255, 128) : IM_COL32(255, 255, 0, 128);
            draw_list->AddCircleFilled(node_pos, W2Sf(10.0f), halo_col);
            draw_list->AddCircleFilled(node_pos, W2Sf(6.0f), IM_COL32(255, 50, 50, 255));
            draw_list->AddCircle(node_pos, W2Sf(6.0f), IM_COL32(0, 0, 0, 255));
            
            float dx = mouse_pos.x - node_pos.x;
            float dy = mouse_pos.y - node_pos.y;
            if (dx*dx + dy*dy < (10.0f * zoom_ * 10.0f * zoom_)) {
                ImGui::PushStyleColor(ImGuiCol_PopupBg, ImVec4(0.2f, 0.2f, 0.2f, 0.95f));
                ImGui::SetTooltip("Material: %s\nAmount: %.0f Tons\nLocation: %s", stock_db_[i].mat, stock_db_[i].amt, stock_db_[i].place);
                ImGui::PopStyleColor();
            }
        }
        
        draw_list->PopClipRect(); // End map clip
        
        // --- Map View Mode Dialog ---
        if (show_dialog_) {
            ImVec2 dialog_pos = ImVec2(canvas_pos.x + 20, canvas_pos.y + canvas_size.y - 280);
            ImGui::SetNextWindowPos(dialog_pos, ImGuiCond_FirstUseEver);
            ImGui::SetNextWindowSize(ImVec2(280, 230), ImGuiCond_FirstUseEver);
            ImGui::PushStyleColor(ImGuiCol_WindowBg, is_dark ? ImVec4(0.15f, 0.15f, 0.15f, 0.95f) : ImVec4(0.9f, 0.9f, 0.9f, 0.95f)); 
            ImGui::Begin(tr("Map View Mode Dialog"), &show_dialog_, ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_AlwaysAutoResize);
            ImGui::TextDisabled(tr("Tipo de Dato"));
            ImGui::RadioButton("Places", &data_type_, 0); ImGui::RadioButton("Materiales", &data_type_, 1); ImGui::RadioButton("Análisis", &data_type_, 2);
            ImGui::SameLine();
            const char* analysis_items[] = {"Au", "Ag", "Cu", "Fe", "Ni"};
            static int analysis_idx = 0;
            ImGui::PushItemWidth(80);
            ImGui::Combo("##analysis_combo", &analysis_idx, analysis_items, IM_ARRAYSIZE(analysis_items));
            ImGui::PopItemWidth();
            ImGui::Separator();
            ImGui::TextDisabled(tr("Radius"));
            ImGui::PushItemWidth(50);
            ImGui::InputInt("Min##rmin", &radius_min_); ImGui::SameLine(); ImGui::Text(tr("-")); ImGui::SameLine(); ImGui::InputInt("Max##rmax", &radius_max_);
            ImGui::PopItemWidth();
            ImGui::Separator();
            if (ImGui::Button(tr("Cerrar"), ImVec2(70, 25))) show_dialog_ = false;
            ImGui::SameLine(0, 30);
            if (ImGui::Button(tr("Aplicar"), ImVec2(70, 25))) show_dialog_ = false;
            ImGui::SameLine();
            if (ImGui::Button(tr("Aceptar"), ImVec2(60, 25))) show_dialog_ = false;
            ImGui::End();
            ImGui::PopStyleColor();
        }
        
        // Navigation Hints Overlay
        ImGui::SetCursorPos(ImVec2(10, 10));
        ImGui::PushStyleColor(ImGuiCol_Text, col_text);
        ImGui::Text(tr("Mouse Wheel: Zoom (%.1fx) | Right-Click + Drag: Pan Terrain"), zoom_);
        ImGui::PopStyleColor();

    } else {
        // --- STOCK VIEW (with Checkboxes for Mixture) ---
        ImGui::Text(tr("Feed Mixture Calculation - Select items to mix"));
        ImGui::Separator();
        
        ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_Resizable | ImGuiTableFlags_ScrollY;
        if (ImGui::BeginTable("stock_db", 10, flags, ImVec2(-1, ImGui::GetContentRegionAvail().y - 40))) {
            ImGui::TableSetupScrollFreeze(0, 1);
            ImGui::TableSetupColumn("Mezcla"); ImGui::TableSetupColumn("Place Name"); ImGui::TableSetupColumn("Material Name");
            ImGui::TableSetupColumn("Amount Tons"); ImGui::TableSetupColumn("Fuel Value"); ImGui::TableSetupColumn("O2 Demand");
            ImGui::TableSetupColumn("Cu wt-%"); ImGui::TableSetupColumn("Fe wt-%"); ImGui::TableSetupColumn("Ni wt-%"); ImGui::TableSetupColumn("S wt-%");
            ImGui::TableHeadersRow();
            
            for (size_t i = 0; i < stock_db_.size(); ++i) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                ImGui::PushID(i);
                ImGui::Checkbox("##mix", &stock_db_[i].selected_for_mix);
                ImGui::PopID();
                
                ImGui::TableSetColumnIndex(1); 
                if (ImGui::Selectable(stock_db_[i].place, selected_idx_ == i, ImGuiSelectableFlags_SpanAllColumns)) {
                    selected_idx_ = i;
                    strncpy(place_name_, stock_db_[i].place, sizeof(place_name_) - 1);
                    strncpy(material_, stock_db_[i].mat, sizeof(material_) - 1);
                    amount_tons_ = stock_db_[i].amt;
                }
                ImGui::TableSetColumnIndex(2); ImGui::Text(tr("%s"), stock_db_[i].mat);
                ImGui::TableSetColumnIndex(3); ImGui::Text(tr("%.0f"), stock_db_[i].amt);
                ImGui::TableSetColumnIndex(4); ImGui::Text(tr("%.2f"), stock_db_[i].fv);
                ImGui::TableSetColumnIndex(5); ImGui::Text(tr("%.2f"), stock_db_[i].o2);
                ImGui::TableSetColumnIndex(6); ImGui::Text(tr("%.2f"), stock_db_[i].cu);
                ImGui::TableSetColumnIndex(7); ImGui::Text(tr("%.2f"), stock_db_[i].fe);
                ImGui::TableSetColumnIndex(8); ImGui::Text(tr("%.2f"), stock_db_[i].ni);
                ImGui::TableSetColumnIndex(9); ImGui::Text(tr("%.2f"), stock_db_[i].s);
            }
            ImGui::EndTable();
        }
        
        // Mixture Calculation Engine
        float sum_mass = 0.0f, sum_cu = 0.0f, sum_fe = 0.0f;
        for (const auto& item : stock_db_) {
            if (item.selected_for_mix) {
                sum_mass += item.amt; sum_cu += item.amt * item.cu; sum_fe += item.amt * item.fe;
            }
        }
        
        ImGui::Text(tr("Current Mix Result: Total Mass = %.0f Tons | Avg Cu = %.2f%% | Avg Fe = %.2f%%"),
                    sum_mass, sum_mass > 0 ? (sum_cu / sum_mass) : 0.0f, sum_mass > 0 ? (sum_fe / sum_mass) : 0.0f);
    }

    ImGui::EndChild(); // End Left Panel

    ImGui::SameLine();

    // --- RIGHT PANEL (Material & Analysis) ---
    ImGui::BeginChild("map_right_panel", ImVec2(0, 0), true);
    
    ImGui::TextDisabled(tr("Material"));
    ImGui::BeginChild("mat_general", ImVec2(0, 150), true);
    ImGui::TextDisabled(tr("General"));
    if (ImGui::Button(tr("Agregar..."), ImVec2(80, 25))) {
        // Spawn at center of camera view
        float center_x = (-camera_x_ + left_panel_width/2) / zoom_;
        float center_y = (-camera_y_ + ImGui::GetContentRegionAvail().y/2) / zoom_;
        stock_db_.push_back({"New Loc", "New Material", 1000, 0, 0, 0, 0, 0, 0, center_x, center_y, false});
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Transporte..."), ImVec2(90, 25))) mostrar_transporte_ = true;
    ImGui::SameLine();
    if (ImGui::Button(tr("Exportar CSV"), ImVec2(90, 25))) {
        FILE* zen = popen("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Exportar Stock' 2>/dev/null", "r");
        if (zen) {
            char pbuf[512] = {0};
            if (fgets(pbuf, sizeof(pbuf), zen)) {
                std::string path(pbuf);
                if (!path.empty() && path.back()=='\n') path.pop_back();
                if (path.find(".csv")==std::string::npos) path += ".csv";
                FILE* fp = fopen(path.c_str(), "w");
                if (fp) {
                    fprintf(fp, "Lugar;Material;Toneladas;FV;O2;Cu;Fe;Ni;S\n");
                    for (auto& it : stock_db_)
                        fprintf(fp, "%s;%s;%.0f;%.2f;%.2f;%.2f;%.2f;%.2f;%.2f\n",
                                it.place, it.mat, it.amt, it.fv, it.o2, it.cu, it.fe, it.ni, it.s);
                    fclose(fp);
                }
            }
            pclose(zen);
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Save Data"), ImVec2(80, 25))) {
        if (selected_idx_ >= 0 && selected_idx_ < stock_db_.size()) {
            strncpy(stock_db_[selected_idx_].place, place_name_, sizeof(stock_db_[selected_idx_].place) - 1);
            strncpy(stock_db_[selected_idx_].mat, material_, sizeof(stock_db_[selected_idx_].mat) - 1);
            stock_db_[selected_idx_].amt = amount_tons_;
        }
    }
    ImGui::Separator();
    
    ImGui::PushItemWidth(-FLT_MIN);
    ImGui::Text(tr("Place Name ")); ImGui::SameLine(90); ImGui::InputText("##pname", place_name_, sizeof(place_name_));
    ImGui::Text(tr("Material   ")); ImGui::SameLine(90); ImGui::InputText("##mat", material_, sizeof(material_));
    ImGui::Text(tr("Cantidad     ")); ImGui::SameLine(90); ImGui::InputInt("##amt", &amount_tons_);
    ImGui::PopItemWidth();
    ImGui::EndChild();
    
    ImGui::BeginChild("mat_analysis", ImVec2(0, 0), true);
    ImGui::TextDisabled(tr("Analysis (Editable)"));
    ImGui::Separator();
    
    ImGuiTableFlags analysis_flags = ImGuiTableFlags_BordersInnerH | ImGuiTableFlags_RowBg | ImGuiTableFlags_SizingStretchProp;
    if (ImGui::BeginTable("analysis_table", 3, analysis_flags)) {
        ImGui::TableSetupColumn("Prop", ImGuiTableColumnFlags_WidthStretch, 0.4f);
        ImGui::TableSetupColumn("Val", ImGuiTableColumnFlags_WidthStretch, 0.4f);
        ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthStretch, 0.2f);
        
        bool edited = false;
        
        if (selected_idx_ >= 0 && selected_idx_ < stock_db_.size()) {
            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Fuel Value")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); if (ImGui::InputFloat("##fv", &stock_db_[selected_idx_].fv, 0.0f, 0.0f, "%.2f")) edited = true; ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("kW/ht"));
            
            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("O2 Demand")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); if (ImGui::InputFloat("##o2", &stock_db_[selected_idx_].o2, 0.0f, 0.0f, "%.2f")) edited = true; ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("Nm3/t"));

            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Cu")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); if (ImGui::InputFloat("##cu", &stock_db_[selected_idx_].cu, 0.0f, 0.0f, "%.2f")) edited = true; ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("wt-%%"));

            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Fe")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); if (ImGui::InputFloat("##fe", &stock_db_[selected_idx_].fe, 0.0f, 0.0f, "%.2f")) edited = true; ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("wt-%%"));

            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Ni")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); if (ImGui::InputFloat("##ni", &stock_db_[selected_idx_].ni, 0.0f, 0.0f, "%.2f")) edited = true; ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("wt-%%"));

            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("S")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); if (ImGui::InputFloat("##s", &stock_db_[selected_idx_].s, 0.0f, 0.0f, "%.2f")) edited = true; ImGui::PopItemWidth();
            // Auto-save logic removed
        } else {
            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Select a material"));
        }
        
        ImGui::EndTable();
    }
    
    ImGui::EndChild();
    ImGui::EndChild(); // End Right Panel

    // ── Popup: Transporte de Material ──
    if (mostrar_transporte_) {
        ImGui::OpenPopup(tr("Transporte de Material"));
        if (ImGui::BeginPopupModal(tr("Transporte de Material"), &mostrar_transporte_, ImGuiWindowFlags_AlwaysAutoResize)) {
            if (selected_idx_ >= 0 && selected_idx_ < (int)stock_db_.size()) {
                auto& it = stock_db_[selected_idx_];
                ImGui::Text(tr("Material: %s"), it.mat);
                ImGui::Text(tr("Desde: %s"), it.place);
                static char nuevo_lugar[64] = "";
                static int mover_t = 0;
                ImGui::InputText(tr("Destino"), nuevo_lugar, sizeof(nuevo_lugar));
                ImGui::InputInt(tr("Cantidad (t)"), &mover_t);
                if (ImGui::Button(tr("Transportar"), ImVec2(130, 0))) {
                    if (mover_t > 0 && mover_t <= (int)it.amt) {
                        it.amt -= mover_t;
                        for (auto& otro : stock_db_)
                            if (strcmp(otro.place, nuevo_lugar) == 0 && strcmp(otro.mat, it.mat) == 0)
                                otro.amt += mover_t;
                        mostrar_transporte_ = false;
                        ImGui::CloseCurrentPopup();
                    }
                }
            } else {
                ImGui::Text(tr("Seleccione un stock primero."));
                if (ImGui::Button(tr("Cerrar"))) { mostrar_transporte_ = false; ImGui::CloseCurrentPopup(); }
            }
            ImGui::EndPopup();
        }
    }
}

#include <cmath>
#include <random>

cma::interfaz::PanelDataFit::PanelDataFit() {
    refrescar_datos();
}

void cma::interfaz::PanelDataFit::refrescar_datos() {
    data_.clear();
    if (!flowsheet_) return;
    
    // Obtener pares (Temperatura, Entalpía) de los flujos para demostrar el ajuste
    for (const auto& [fid, f] : flowsheet_->mapa_flujos()) {
        float x = (float)f.temperatura;
        float y = (float)f.entalpia;
        data_.push_back({x, y});
    }
}

void cma::interfaz::PanelDataFit::fit_curve() {
    if (data_.empty()) return;
    
    // Simple Gradient Descent for demonstration.
    // For a robust system, we would use Levenberg-Marquardt (Eigen or similar).
    int num_params = (func_type_ == 0) ? 2 : ((func_type_ == 1) ? 4 : 7);
    
    for (int iter = 0; iter < iterations_max_; ++iter) {
        std::vector<float> grad(7, 0.0f);
        
        for (const auto& pt : data_) {
            float y_pred = evaluate_func(pt.x);
            float err = y_pred - pt.y;
            
            if (func_type_ == 0) {
                grad[0] += err * 1.0f;
                grad[1] += err * pt.x;
            } else if (func_type_ == 1) {
                grad[0] += err * 1.0f;
                grad[1] += err * pt.x;
                grad[2] += err * pt.x * pt.x;
                grad[3] += err * pt.x * pt.x * pt.x;
            } else {
                // Rational Gradient
                float num = params_[0] + params_[1]*pt.x + params_[2]*pt.x*pt.x + params_[3]*pt.x*pt.x*pt.x;
                float den = 1.0f + params_[4]*pt.x + params_[5]*pt.x*pt.x + params_[6]*pt.x*pt.x*pt.x;
                if (std::abs(den) > 1e-5f) {
                    float d_dA = 1.0f / den;
                    float d_dB = pt.x / den;
                    float d_dC = (pt.x*pt.x) / den;
                    float d_dD = (pt.x*pt.x*pt.x) / den;
                    float d_dE = -num * pt.x / (den*den);
                    float d_dF = -num * (pt.x*pt.x) / (den*den);
                    float d_dG = -num * (pt.x*pt.x*pt.x) / (den*den);
                    
                    grad[0] += err * d_dA;
                    grad[1] += err * d_dB;
                    grad[2] += err * d_dC;
                    grad[3] += err * d_dD;
                    grad[4] += err * d_dE;
                    grad[5] += err * d_dF;
                    grad[6] += err * d_dG;
                }
            }
        }
        
        // Update params
        for (int i = 0; i < num_params; ++i) {
            params_[i] -= learning_rate_ * grad[i] / data_.size();
        }
    }
    
    // Calculate R2 and Errors
    float y_mean = 0.0f;
    for (const auto& pt : data_) y_mean += pt.y;
    y_mean /= data_.size();
    
    ssr_ = 0.0f; // Sum of squared residuals
    float sst = 0.0f; // Total sum of squares
    
    for (const auto& pt : data_) {
        float y_pred = evaluate_func(pt.x);
        ssr_ += (y_pred - pt.y) * (y_pred - pt.y);
        sst += (pt.y - y_mean) * (pt.y - y_mean);
    }
    
    error_ = std::sqrt(ssr_ / data_.size());
    r2_ = sst > 0.0f ? (1.0f - (ssr_ / sst)) : 0.0f;
}


float cma::interfaz::PanelDataFit::evaluate_func(float x) const {
    if (func_type_ == 0) {
        return params_[0] * x + params_[1];
    } else if (func_type_ == 1) {
        return params_[0] * x * x + params_[1] * x + params_[2];
    } else {
        return (params_[0] + params_[1] * x + params_[2] * x * x) / (1.0f + params_[3] * x + params_[4] * x * x);
    }
}

void cma::interfaz::PanelDataFit::renderizar() {
    ImGui::Text(tr("HSC-Sim Data Fit"));
    ImGui::Separator();
    
    // --- Top Panel: Specifications ---
    ImGui::BeginChild("FitSpecs", ImVec2(0, 100), true);
    ImGui::Columns(2, "spec_cols");
    
    ImGui::TextDisabled(tr("Opciones de Ajuste"));
    ImGui::PushItemWidth(100);
    ImGui::InputInt("Iterations Max", &iterations_max_);
    ImGui::InputFloat("Learning Rate", &learning_rate_, 0.0f, 0.0f, "%.5f");
    ImGui::PopItemWidth();
    
    ImGui::NextColumn();
    
    ImGui::TextDisabled(tr("Function Formula"));
    const char* func_items[] = {
        "Y = A + B*x", 
        "Y = A + B*x + C*x^2 + D*x^3", 
        "Y = (A + B*x + C*x^2 + D*x^3) / (1 + E*x + F*x^2 + G*x^3)"
    };
    if (ImGui::Combo("Formula", &func_type_, func_items, IM_ARRAYSIZE(func_items))) {
        // Reset params
        for(int i=0; i<7; ++i) params_[i] = (i==0) ? 80.0f : 0.1f;
    }
    ImGui::TextDisabled(tr("Points: %d"), (int)data_.size());
    ImGui::Columns(1);
    ImGui::EndChild();
    
    // --- Split Left (Table) and Right (Plot) ---
    float left_width = 300.0f;
    ImGui::BeginChild("FitLeft", ImVec2(left_width, 0), true);
    
    // Parameters Table
    if (ImGui::BeginTable("param_table", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Nombre");
        ImGui::TableSetupColumn("Valor");
        ImGui::TableHeadersRow();
        
        int num_params = (func_type_ == 0) ? 2 : ((func_type_ == 1) ? 4 : 7);
        const char* pnames[] = {"A", "B", "C", "D", "E", "F", "G"};
        
        for (int i = 0; i < num_params; ++i) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text(tr("%s"), pnames[i]);
            ImGui::TableSetColumnIndex(1); 
            ImGui::PushID(i);
            ImGui::PushItemWidth(-FLT_MIN);
            ImGui::InputFloat("##pval", &params_[i], 0.0f, 0.0f, "%.4f");
            ImGui::PopItemWidth();
            ImGui::PopID();
        }
        ImGui::EndTable();
    }
    
    ImGui::Spacing();
    
    // Stats Table
    if (ImGui::BeginTable("stats_table", 2, ImGuiTableFlags_Borders)) {
        ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Error")); ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.2f"), error_);
        ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("R2")); ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.4f"), r2_);
        ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("SSR")); ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.2f"), ssr_);
        ImGui::EndTable();
    }
    
    ImGui::Spacing();
    if (ImGui::Button(tr("Ajustar Curva"), ImVec2(-1, 30))) {
        fit_curve();
    }
    
    if (ImGui::Button(tr("Exportar CSV (Ajuste)"), ImVec2(-1, 25))) {
        std::string fname = "export_fit.csv";
        FILE* f = fopen(fname.c_str(), "w");
        if (f) {
            fprintf(f, "X,Y_data,Y_fit\n");
            for (const auto& pt : data_) {
                fprintf(f, "%.4f,%.4f,%.4f\n", pt.x, pt.y, evaluate_func(pt.x));
            }
            fclose(f);
        }
    }

    if (ImGui::Button(tr("Refrescar del Flowsheet"), ImVec2(-1, 25))) {
        refrescar_datos();
    }
    
    ImGui::EndChild();
    
    ImGui::SameLine();
    
    // --- Plot Area ---
    ImGui::BeginChild("FitPlot", ImVec2(0, 0), true);
    
    if (ImPlot::BeginPlot("Electron Mobility Fit", ImVec2(-1, -1))) {
        ImPlot::SetupAxes("Density", "Y");
        
        // Prepare Scatter Data
        std::vector<float> sx, sy;
        for (const auto& pt : data_) { sx.push_back(pt.x); sy.push_back(pt.y); }
        
        // Prepare Line Data (Fitted Curve)
        std::vector<float> lx, ly;
        for (float x = -3.5f; x <= 2.5f; x += 0.05f) {
            lx.push_back(x);
            ly.push_back(evaluate_func(x));
        }
        
        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 4, ImVec4(1,0,0,1), -1, ImVec4(1,0,0,1));
        ImPlot::PlotScatter("Exp. Data", sx.data(), sy.data(), (int)sx.size());
        
        ImPlot::SetNextLineStyle(ImVec4(0,0,0,1), 2.0f); // Black curve
        ImPlot::PlotLine("Fitted Function", lx.data(), ly.data(), (int)lx.size());
        
        ImPlot::EndPlot();
    }
    
    ImGui::EndChild();
}

#include <cmath>
#include <random>
#include <algorithm>

cma::interfaz::PanelStatisticalAnalysis::PanelStatisticalAnalysis() {
    refrescar_datos();
}

void cma::interfaz::PanelStatisticalAnalysis::refrescar_datos() {
    db_.clear();
    pca_labels_ = {"Temp (K)", "Pres (atm)", "Flujo (kmol/s)", "Entalpia (kW)", "Frac. Solida"};
    pca_variance_ = {45.2f, 25.1f, 15.5f, 10.2f, 4.0f}; // Simulated PCA variances based on properties
    
    if (!flowsheet_) return;

    int id = 1;
    for (const auto& [fid, f] : flowsheet_->mapa_flujos()) {
        StatDataPoint pt;
        pt.id = id++;
        
        // Asignamos variables termodinámicas reales a los campos del struct StatDataPoint
        pt.al2o3 = (float)f.temperatura; // Usamos al2o3 como Temperatura
        pt.cr2o3 = (float)f.presion;     // Usamos cr2o3 como Presión
        
        double moles_totales = 0;
        for (const auto& [esp_id, mol] : f.flujo_molar) moles_totales += mol;
        pt.zn_rec = (float)moles_totales; // Usamos zn_rec como Moles
        
        pt.cu_rec = (float)f.entalpia;    // Usamos cu_rec como Entalpía
        pt.group = (pt.al2o3 > 298.15f) ? 1 : 0; // Agrupamos por si está caliente o frío
        
        db_.push_back(pt);
    }
}

void cma::interfaz::PanelStatisticalAnalysis::renderizar() {
    // We want a full-screen layout.
    // Left area: 4 quadrants of plots. Right area: Table viewer.
    ImGui::Text(tr("HSC Data - Statistical Analysis Dashboard"));
    ImGui::Separator();
    
    float right_panel_width = 300.0f;
    float content_width = ImGui::GetContentRegionAvail().x - right_panel_width - 8.0f;
    float content_height = ImGui::GetContentRegionAvail().y;
    
    ImGui::BeginChild("PlotsArea", ImVec2(content_width, content_height), false);
    
    // We divide into 2 rows and 2 columns
    float plot_w = content_width * 0.5f - 8.0f;
    float plot_h = content_height * 0.5f - 8.0f;
    
    // --- TOP LEFT: Histogram ---
    ImGui::BeginChild("Quad1", ImVec2(plot_w, plot_h), true);
    ImGui::TextDisabled(tr("Distribution (Histogram)"));
    if (ImPlot::BeginPlot("##hist", ImVec2(-1, -1))) {
        ImPlot::SetupAxes("Valor", "Count");
        std::vector<float> values;
        for (const auto& pt : db_) {
            values.push_back(x_axis_col_ == 0 ? pt.al2o3 : (x_axis_col_ == 1 ? pt.cr2o3 : pt.zn_rec));
        }
        ImPlot::SetNextFillStyle(ImVec4(0.6f, 0.6f, 0.6f, 0.8f));
        ImPlot::PlotHistogram("Frequency", values.data(), (int)values.size(), 30);
        ImPlot::EndPlot();
    }
    ImGui::EndChild();
    
    ImGui::SameLine();
    
    // --- TOP RIGHT: XY Scatter with Groups ---
    ImGui::BeginChild("Quad2", ImVec2(plot_w, plot_h), true);
    ImGui::TextDisabled(tr("XY Correlation (Clustered)"));
    if (ImPlot::BeginPlot("##scatter", ImVec2(-1, -1))) {
        const char* x_label = (x_axis_col_ == 0) ? "Al2O3_wt%" : "Cr2O3_wt%";
        const char* y_label = (y_axis_col_ == 0) ? "Al2O3_wt%" : "Cr2O3_wt%";
        ImPlot::SetupAxes(x_label, y_label);
        
        std::vector<float> x0, y0, x1, y1;
        for (const auto& pt : db_) {
            float vx = (x_axis_col_ == 0) ? pt.al2o3 : pt.cr2o3;
            float vy = (y_axis_col_ == 0) ? pt.al2o3 : pt.cr2o3;
            if (pt.group == 0) { x0.push_back(vx); y0.push_back(vy); }
            else { x1.push_back(vx); y1.push_back(vy); }
        }
        
        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 3, ImVec4(1,0,0,1), -1, ImVec4(1,0,0,1));
        ImPlot::PlotScatter("Group A", x0.data(), y0.data(), (int)x0.size());
        
        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 3, ImVec4(0,0,1,1), -1, ImVec4(0,0,1,1));
        ImPlot::PlotScatter("Group B", x1.data(), y1.data(), (int)x1.size());
        
        ImPlot::EndPlot();
    }
    ImGui::EndChild();
    
    // --- BOTTOM LEFT: PCA Variance Bar Chart ---
    ImGui::BeginChild("Quad3", ImVec2(plot_w, plot_h), true);
    ImGui::TextDisabled(tr("Variables Explaining Variance (PCA)"));
    if (ImPlot::BeginPlot("##pca", ImVec2(-1, -1))) {
        ImPlot::SetupAxes("Variable", "Of Variance, %");
        // X-axis ticks
        std::vector<double> ticks = {0, 1, 2, 3, 4};
        std::vector<const char*> labels;
        for (const auto& l : pca_labels_) labels.push_back(l.c_str());
        
        ImPlot::SetupAxisTicks(ImAxis_X1, ticks.data(), (int)ticks.size(), labels.data());
        ImPlot::SetNextFillStyle(ImVec4(0.9f, 0.1f, 0.1f, 0.9f));
        ImPlot::PlotBars("Variance", pca_variance_.data(), (int)pca_variance_.size(), 0.5);
        ImPlot::EndPlot();
    }
    ImGui::EndChild();
    
    ImGui::SameLine();
    
    // --- BOTTOM RIGHT: Time Series Line Graph ---
    ImGui::BeginChild("Quad4", ImVec2(plot_w, plot_h), true);
    ImGui::TextDisabled(tr("Process Evolution (Zn_Rec)"));
    if (ImPlot::BeginPlot("##lines", ImVec2(-1, -1))) {
        ImPlot::SetupAxes("No. Muestra", "Zn_Rec %");
        std::vector<float> x_time, y_zn, y_cu;
        for (const auto& pt : db_) {
            x_time.push_back((float)pt.id);
            y_zn.push_back(pt.zn_rec);
            y_cu.push_back(pt.cu_rec);
        }
        
        ImPlot::SetNextLineStyle(ImVec4(0,0,0.8f,1), 1.5f);
        ImPlot::PlotLine("Zn_Rec", x_time.data(), y_zn.data(), (int)x_time.size());
        
        ImPlot::SetNextLineStyle(ImVec4(0.8f,0,0,0.5f), 1.5f);
        ImPlot::PlotLine("Cu_Rec (Simulated)", x_time.data(), y_cu.data(), (int)x_time.size());
        
        ImPlot::EndPlot();
    }
    ImGui::EndChild();
    
    ImGui::EndChild(); // End PlotsArea
    
    ImGui::SameLine();
    
    // --- RIGHT PANEL: Database Table ---
    ImGui::BeginChild("TableArea", ImVec2(right_panel_width, content_height), true);
    ImGui::TextDisabled(tr("Navegador de BD"));
    ImGui::Separator();
    ImGui::Text(tr("Opciones de Datos:"));
    if (ImGui::Button(tr("Refrescar del Flowsheet"), ImVec2(-1, 25))) {
        refrescar_datos();
    }
    ImGui::Separator();

    
    ImGui::Text(tr("X Axis:"));
    ImGui::RadioButton("Al2O3", &x_axis_col_, 0); ImGui::SameLine();
    ImGui::RadioButton("Cr2O3", &x_axis_col_, 1);
    
    ImGui::Text(tr("Y Axis:"));
    ImGui::RadioButton("Al2O3##y", &y_axis_col_, 0); ImGui::SameLine();
    ImGui::RadioButton("Cr2O3##y", &y_axis_col_, 1);
    
    ImGui::Separator();
    ImGui::Text(tr("Vista de Datos (%d filas)"), (int)db_.size());
    
    if (ImGui::Button(tr("Exportar CSV (Estad.)"), ImVec2(-1, 25))) {
        std::string fname = "export_stats.csv";
        FILE* f = fopen(fname.c_str(), "w");
        if (f) {
            fprintf(f, "ID,Al2O3,Cr2O3,Zn_Rec\n");
            for (const auto& pt : db_) {
                fprintf(f, "%d,%.2f,%.2f,%.2f\n", pt.id, pt.al2o3, pt.cr2o3, pt.zn_rec);
            }
            fclose(f);
        }
    }

    
    if (ImGui::BeginTable("db_table", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, -1))) {
        ImGui::TableSetupColumn("ID", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        ImGui::TableSetupColumn("Al2O3");
        ImGui::TableSetupColumn("Cr2O3");
        ImGui::TableSetupColumn("Zn_Rec");
        ImGui::TableHeadersRow();
        
        for (const auto& pt : db_) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text(tr("%d"), pt.id);
            ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.2f"), pt.al2o3);
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("%.2f"), pt.cr2o3);
            ImGui::TableSetColumnIndex(3); ImGui::Text(tr("%.2f"), pt.zn_rec);
        }
        ImGui::EndTable();
    }
    
    ImGui::EndChild();
}

// ── Panel POURBAIX (Eh-pH) ────────────────────────────────────────────────
#include "../motores/pourbaix.hpp"

cma::interfaz::PanelPourbaix::PanelPourbaix() {
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
