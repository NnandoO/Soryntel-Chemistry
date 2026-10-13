#include "scientific.hpp"
#include <imgui.h>
namespace cma::interfaz {
void AdvancedAqueousPanel::render() {
    ImGui::TextUnformatted("Equilibrio acuoso avanzado · USGS IPhreeqc");
    ImGui::TextWrapped("Motor y bases USGS independientes del catálogo heredado. Pitzer usa parámetros de interacción de pitzer.dat; no se supone validez para cualquier mezcla o temperatura.");
    if (!engine_.available()) { ImGui::TextWrapped("Motor no disponible: compile con CMA_WITH_IPHREEQC=ON y compruebe data/reference/phreeqc."); return; }
    bool edited=ImGui::Combo("Modelo",&model_,"Asociación iónica (phreeqc.dat)\0Pitzer (pitzer.dat)\0");
    input_.model=model_==0?domain::AqueousBackend::IonAssociation:domain::AqueousBackend::Pitzer;
    edited|=ImGui::InputDouble("Temperatura [°C; 0..100]",&input_.celsius);
    edited|=ImGui::InputDouble("pH inicial [0..14]",&input_.ph);
    if (ImGui::BeginTable("advanced_totals",2,ImGuiTableFlags_Borders|ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Total analítico"); ImGui::TableSetupColumn("mol/kg de agua"); ImGui::TableHeadersRow();
        for (const char* name : {"Na","Cl","Ca","Mg","K","C(4)","S(6)"}) {
            ImGui::PushID(name); ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::TextUnformatted(name);
            ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1); edited|=ImGui::InputDouble("##total",&input_.totals_molal[name],0,0,"%.6g"); ImGui::PopID();
        }
        ImGui::EndTable();
    }
    edited|=ImGui::Checkbox("Imponer electroneutralidad ajustando Na",&input_.balance_sodium);
    if (input_.balance_sodium) ImGui::TextWrapped("El total de Na introducido puede cambiar. Revise Na(mol/kgw) y el error eléctrico de la salida.");
    edited|=ImGui::Checkbox("Equilibrar con calcita (disolución/precipitación)",&input_.equilibrate_calcite);
    if (input_.equilibrate_calcite) edited|=ImGui::InputDouble("Calcita disponible [mol; 0 permite precipitación]",&input_.calcite_moles);
    edited|=ImGui::Checkbox("Equilibrar con CO2(g)",&input_.equilibrate_co2);
    if (input_.equilibrate_co2) edited|=ImGui::InputDouble("log10 PCO2 [atm; -15..0]",&input_.log_co2_pressure);
    if (edited) calculated_=false;
    if (ImGui::Button("Resolver equilibrio avanzado")) { result_=engine_.solve(input_); calculated_=true; }
    if (!calculated_) { ImGui::TextDisabled("Introduzca datos analíticos y resuelva el equilibrio."); return; }
    ImGui::Separator(); ImGui::TextWrapped("%s",result_.message.c_str());
    if (!result_.ok) return;
    ImGui::TextWrapped("%s · Base: %s",result_.engine.c_str(),result_.database.c_str());
    ImGui::TextWrapped("pH puede cambiar al equilibrar fases. Los totales después de reacción no son necesariamente los iniciales.");
    if (ImGui::BeginTable("advanced_results",2,ImGuiTableFlags_Borders|ImGuiTableFlags_RowBg|ImGuiTableFlags_ScrollY,ImVec2(0,220))) {
        ImGui::TableSetupColumn("Magnitud (unidad en encabezado)"); ImGui::TableSetupColumn("Valor"); ImGui::TableHeadersRow();
        for (const auto& [name,value] : result_.values) {
            ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::TextUnformatted(name.c_str());
            ImGui::TableNextColumn(); ImGui::Text("%.9g",value);
        }
        ImGui::EndTable();
    }
    if (ImGui::CollapsingHeader("Salida completa y balances del motor")) {
        ImGui::BeginChild("phreeqc_output",ImVec2(0,240),true,ImGuiWindowFlags_HorizontalScrollbar);
        ImGui::TextUnformatted(result_.full_output.c_str()); ImGui::EndChild();
    }
}
}
