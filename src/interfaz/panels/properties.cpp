#include "shared.hpp"
namespace cma { namespace interfaz {
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

}} // namespace
