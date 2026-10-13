#include "scientific.hpp"
#include <imgui.h>
#include <cmath>

namespace cma::interfaz {
void NativeAqueousPanel::render() {
    ImGui::TextUnformatted("Especiación de equilibrio · MINTEQ nativo");
    ImGui::TextWrapped("Molalidades en mol/kg de agua. pH fijado por un buffer: el residuo de carga se informa, no se impone electroneutralidad. Los sólidos muestran saturación, sin precipitar.");
    ImGui::TextWrapped("Catálogo nativo habilitado a 298.15 K. No se extrapolan las leyes térmicas MINTEQ sin verificar sus coeficientes y rangos.");
    if (database_.components.empty() || database_.reactions.empty()) {
        ImGui::TextWrapped("No hay un catálogo nativo disponible. Revise data/comp_2008.vdb, type6.vdb, gases.vdb y thermo.vdb; no se sustituye con datos de especies puras.");
        return;
    }
    ImGui::Text("Catálogo: %zu componentes / %zu reacciones / %zu diagnósticos",database_.components.size(),database_.reactions.size(),database_.diagnostics.size());
    bool edited=ImGui::InputDouble("Temperatura [K]",&kelvin_,0,0,"%.2f");
    edited|=ImGui::InputDouble("pH [-log10(aH+)]",&ph_,0,0,"%.3f");
    edited|=ImGui::Combo("Actividad",&model_,"Ideal\0Davies (298.15 K; I <= 0.5 mol/kg)\0");
    if (ImGui::BeginTable("component_totals",3,ImGuiTableFlags_Borders|ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Componente de base"); ImGui::TableSetupColumn("Total [mol/kg]"); ImGui::TableSetupColumn("Acción"); ImGui::TableHeadersRow();
        int removed=-1;
        for (auto& [id,total] : totals_) {
            ImGui::PushID(id); ImGui::TableNextRow(); ImGui::TableNextColumn();
            const auto it=database_.components.find(id);
            ImGui::Text("%d · %s",id,it==database_.components.end()?"No disponible":it->second.name.c_str());
            ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1);
            edited|=ImGui::InputDouble("##total",&total,0,0,"%.6g");
            ImGui::TableNextColumn(); if (ImGui::SmallButton("Quitar")) removed=id;
            ImGui::PopID();
        }
        if (removed>=0) { totals_.erase(removed); edited=true; }
        ImGui::EndTable();
    }
    std::string preview="Seleccionar componente";
    if (database_.components.count(new_component_)) preview=database_.components.at(new_component_).name;
    if (ImGui::BeginCombo("Añadir total",preview.c_str())) {
        for (const auto& [id,component] : database_.components) {
            if (id==1 || id==2 || id==330 || totals_.count(id)) continue;
            ImGui::PushID(id);
            if (ImGui::Selectable(component.name.c_str(),id==new_component_)) new_component_=id;
            ImGui::PopID();
        }
        ImGui::EndCombo();
    }
    if (ImGui::Button("Añadir componente") && database_.components.count(new_component_) && !totals_.count(new_component_)) {
        totals_[new_component_]=.001; edited=true;
    }
    if (edited) calculated_=false;
    if (ImGui::Button("Calcular especiación")) {
        motores::SpeciationInput input;
        input.totals_molal=totals_; input.kelvin=kelvin_;
        input.fixed_log_activities={{2,0},{330,-ph_}};
        input.model=model_==0?motores::ActivityModel::Ideal:motores::ActivityModel::Davies;
        result_=motores::AqueousSpeciation(database_).solve(input); calculated_=true;
    }
    if (!calculated_) { ImGui::TextDisabled("Calcule después de modificar los datos."); return; }
    ImGui::Separator(); ImGui::TextWrapped("%s",result_.message.c_str());
    if (!result_.ok) return;
    ImGui::Text("I: %.6g mol/kg · Carga residual: %+.6g eq/kg",result_.ionic_strength,result_.charge_residual);
    ImGui::Text("Error relativo de balance: %.3g · Iteraciones: %d",result_.mass_residual,result_.iterations);
    if (std::abs(result_.charge_residual)>1e-6) ImGui::TextWrapped("Revise contraiones y buffer: el estado calculado no es electroneutro.");
    if (ImGui::BeginTable("aqueous_result",3,ImGuiTableFlags_Borders|ImGuiTableFlags_RowBg|ImGuiTableFlags_ScrollY,ImVec2(0,240))) {
        ImGui::TableSetupColumn("Especie"); ImGui::TableSetupColumn("Molalidad [mol/kg]"); ImGui::TableSetupColumn("Origen"); ImGui::TableHeadersRow();
        for (const auto& [id,value] : result_.free_components) {
            ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::TextUnformatted(database_.components.at(id).name.c_str());
            ImGui::TableNextColumn(); ImGui::Text("%.8g",value); ImGui::TableNextColumn(); ImGui::TextUnformatted("Componente libre");
        }
        for (const auto& [id,value] : result_.dissolved_species) {
            ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::TextUnformatted(database_.reactions.at(id).name.c_str());
            ImGui::TableNextColumn(); ImGui::Text("%.8g",value); ImGui::TableNextColumn(); ImGui::TextUnformatted(database_.reactions.at(id).source_log_k.c_str());
        }
        ImGui::EndTable();
    }
    if (ImGui::CollapsingHeader("Índices de saturación: SI > 0 indica sobresaturación"))
        for (const auto& [id,index] : result_.saturation_indices)
            ImGui::Text("%s: %+.6g (%s)",database_.reactions.at(id).name.c_str(),index,database_.reactions.at(id).source_log_k.c_str());
    if (ImGui::CollapsingHeader("Diagnósticos del catálogo")) {
        ImGui::BeginChild("native_diagnostics",ImVec2(0,180),true);
        for (const auto& message : database_.diagnostics) ImGui::TextWrapped("%s",message.c_str());
        ImGui::EndChild();
    }
}
}
