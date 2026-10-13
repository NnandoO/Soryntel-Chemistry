#include "shared.hpp"
namespace cma { namespace interfaz {
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

}} // namespace
