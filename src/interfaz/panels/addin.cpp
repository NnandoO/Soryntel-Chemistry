#include "shared.hpp"
#include <random>
using namespace cma::interfaz;
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
        FILE* zen = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Export to Excel' 2>/dev/null");
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
            fclose(zen);
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
