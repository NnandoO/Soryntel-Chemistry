#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de Soluciones Acuosas
// ═══════════════════════════════════════════════════════════════════════════
void PanelSolucionesAcuosas::calcular_hoja() {
    const double unavailable=std::numeric_limits<double>::quiet_NaN();
    double total_kmol=0,water_kg=0;
    bool valid=true;
    for (const auto& row : filas_) {
        if (!std::isfinite(row.amount_kmol) || row.amount_kmol<0 || !std::isfinite(row.temp) || row.temp+273.15<=0) valid=false;
        total_kmol+=row.amount_kmol;
        const auto* species=buscar_especie_por_nombre_o_formula(row.especie);
        if (species && species->formula=="H2O" && species->fase_std==nucleo::Fase::Liquido)
            water_kg+=row.amount_kmol*species->masa_molar;
    }
    double strength=0;
    for (auto& row : filas_) {
        row.calculo_ok=false;
        row.ac_molal=row.ac_molar=row.h_estimate=row.cp_estimate=unavailable;
        row.amount_mol_pct=valid&&total_kmol>0?100*row.amount_kmol/total_kmol:0;
        const auto* species=buscar_especie_por_nombre_o_formula(row.especie);
        if (!valid || !species || water_kg<=0) continue;
        row.amount_g=row.amount_kmol*1000*species->masa_molar;
        row.h_ideal=species->entalpia_abs(row.temp+273.15)/4.184;
        row.cp_ideal=species->cp(row.temp+273.15)/4.184;
        row.carga=0;
        for (const auto& atom : nucleo::TablaPeriodica::instancia().analizar_formula(species->formula))
            if (atom.elem=="e-") row.carga-=static_cast<int>(atom.cuenta);
        // Catalog ionic notation; the native MINTEQ panel uses explicit charges.
        const std::string name(row.especie);
        if (name.find("(+a)")!=std::string::npos) row.carga=1;
        else if (name.find("(+2a)")!=std::string::npos) row.carga=2;
        else if (name.find("(-a)")!=std::string::npos) row.carga=-1;
        else if (name.find("(-2a)")!=std::string::npos) row.carga=-2;
        else if (name.find("(-3a)")!=std::string::npos) row.carga=-3;
        strength+=.5*(row.amount_kmol*1000/water_kg)*row.carga*row.carga;
        row.calculo_ok=true;
    }
    for (auto& row : filas_) {
        if (!row.calculo_ok) continue;
        row.ac_molal=motor_.coeficiente_actividad_davies(row.carga,strength,row.temp+273.15);
        // Density, partial molar excess enthalpy and d(gamma)/dT are unavailable.
        // Never fabricate molar-basis coefficients or nonideal H/Cp corrections.
    }
}

void PanelSolucionesAcuosas::renderizar() {
    ImGui::TextWrapped("Hoja aproximada del catálogo heredado. H/Cp no verificados; coeficientes Davies solo a 25 °C y baja fuerza iónica. Base molar y correcciones no ideales: no disponibles (NaN). Sin agua líquida especificada no se calculan molalidades.");
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
    ImGui::BeginDisabled();
    if (ImGui::Checkbox(tr("Pitzer"), &usar_pitzer_)) {
        if (usar_pitzer_) { usar_davies_ = false; usar_harvie_ = false; }
        changed = true;
    }
    ImGui::SameLine();
    if (ImGui::Checkbox(tr("Harvie"), &usar_harvie_)) {
        if (usar_harvie_) { usar_davies_ = false; usar_pitzer_ = false; }
        changed = true;
    }
    ImGui::EndDisabled();
    ImGui::TextWrapped("Pitzer/HMW no disponibles: faltan parámetros de interacción. Davies es una aproximación para soluciones diluidas.");
    
    // Ensure at least one is checked
    if (!usar_davies_ && !usar_pitzer_ && !usar_harvie_) {
        usar_davies_ = true;
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

}} // namespace
