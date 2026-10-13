#include "shared.hpp"
namespace cma { namespace interfaz {
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

[[maybe_unused]] static ImU32 color_nodo(nucleo::TipoNodo tipo) {
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

    auto btn_modo = [&](const char* label, ModoDibujo modo, const char*) {
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

}} // namespace
