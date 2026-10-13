#include "shared.hpp"
namespace cma { namespace interfaz {
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

static ImVec2 posicion_tabla(int Z, int periodo, int grupo, float /*celda*/) {
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

}} // namespace
