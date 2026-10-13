#include "interfaz/paneles.hpp"
#include <imgui.h>
#include <implot.h>
#include <cmath>
#include <limits>

namespace cma::interfaz {
void PanelAgua::renderizar() {
    static double temperature[2]={300,25},pressure[2]={25,1},quality[2]={0,1};
    static int mode[2]={0,0},amount_unit=0;
    static double amount=1;
    ImGui::TextUnformatted("Agua / vapor · IAPWS-IF97 (CoolProp)");
    ImGui::TextWrapped("Temperatura en °C; presión absoluta en bar. La saturación requiere calidad x. No se calcula hielo ni fronteras aproximadas de fusión/sublimación.");
    ImGui::InputDouble("Cantidad",&amount);
    ImGui::Combo("Unidad de cantidad",&amount_unit,"kg\0kmol\0");
    const bool valid_amount=std::isfinite(amount)&&amount>=0;
    if (!valid_amount) ImGui::TextWrapped("La cantidad debe ser finita y no negativa.");
    motores::PropiedadesAgua state[2];
    if (ImGui::BeginTable("water_inputs",2,ImGuiTableFlags_Borders)) {
        for (int i=0;i<2;++i) {
            ImGui::TableNextColumn(); ImGui::PushID(i); ImGui::Text("Punto %d",i+1);
            ImGui::Combo("Estado",&mode[i],"T y P\0Saturado: P y calidad x\0");
            ImGui::InputDouble("Presión [bar]",&pressure[i]);
            if (mode[i]==0) {
                ImGui::InputDouble("Temperatura [°C]",&temperature[i]);
                state[i]=motor_.propiedades(temperature[i]+273.15,pressure[i]*.1);
            } else {
                ImGui::InputDouble("Calidad x [0..1]",&quality[i]);
                state[i]=motor_.mezcla_saturada(pressure[i]*.1,quality[i]);
            }
            ImGui::TextWrapped("%s",state[i].ok?state[i].fase.c_str():state[i].mensaje.c_str());
            if (state[i].ok && !state[i].mensaje.empty()) ImGui::TextWrapped("%s",state[i].mensaje.c_str());
            ImGui::PopID();
        }
        ImGui::EndTable();
    }
    auto display=[](double value) {
        if (std::isfinite(value)) ImGui::Text("%.7g",value); else ImGui::TextUnformatted("No disponible");
    };
    const double missing=std::numeric_limits<double>::quiet_NaN();
    const double multiplier=valid_amount?amount*(amount_unit==0?1:18.01528):missing;
    if (ImGui::BeginTable("water_properties",5,ImGuiTableFlags_Borders|ImGuiTableFlags_RowBg)) {
        for (auto label : {"Propiedad","Punto 1","Punto 2","Punto 2 - Punto 1","Unidad"}) ImGui::TableSetupColumn(label);
        ImGui::TableHeadersRow();
        auto row=[&](const char* name,double first,double second,const char* unit) {
            if (!state[0].ok) first=missing;
            if (!state[1].ok) second=missing;
            ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::TextUnformatted(name);
            ImGui::TableNextColumn(); display(first); ImGui::TableNextColumn(); display(second);
            ImGui::TableNextColumn(); display(second-first); ImGui::TableNextColumn(); ImGui::TextUnformatted(unit);
        };
        row("Temperatura",state[0].T,state[1].T,"K");
        row("Entalpía específica",state[0].h,state[1].h,"kJ/kg");
        row("Entropía específica",state[0].s,state[1].s,"kJ/(kg K)");
        row("Volumen específico",state[0].v,state[1].v,"m³/kg");
        row("Cp",state[0].cp,state[1].cp,"kJ/(kg K)");
        row("Cv",state[0].cv,state[1].cv,"kJ/(kg K)");
        row("Viscosidad",state[0].mu,state[1].mu,"Pa s");
        row("Conductividad térmica",state[0].k,state[1].k,"W/(m K)");
        row("Entalpía total",state[0].h*multiplier,state[1].h*multiplier,"kJ");
        row("Entropía total",state[0].s*multiplier,state[1].s*multiplier,"kJ/K");
        const auto reference=motor_.propiedades(298.15,.101325);
        row("Exergía física (T0=298.15 K, P0=1.01325 bar)",
            reference.ok?((state[0].h-reference.h)-298.15*(state[0].s-reference.s))*multiplier:missing,
            reference.ok?((state[1].h-reference.h)-298.15*(state[1].s-reference.s))*multiplier:missing,"kJ");
        ImGui::EndTable();
    }
    if (ImPlot::BeginPlot("Saturación líquido-vapor IF97",ImVec2(-1,230))) {
        ImPlot::SetupAxes("Presión absoluta [bar]","Temperatura [°C]");
        ImPlot::SetupAxisScale(ImAxis_X1,ImPlotScale_Log10);
        std::vector<double> p,t;
        for (double kelvin=273.16;kelvin<647.096;kelvin+=2) {
            const double value=motor_.presion_saturacion(kelvin);
            if (std::isfinite(value)) { p.push_back(value*10); t.push_back(kelvin-273.15); }
        }
        ImPlot::PlotLine("Saturación",p.data(),t.data(),static_cast<int>(p.size()));
        for (int i=0;i<2;++i) if (state[i].ok) {
            const double point_p=state[i].P*10,point_t=state[i].T-273.15;
            ImPlot::PlotScatter(i==0?"Punto 1":"Punto 2",&point_p,&point_t,1);
        }
        ImPlot::EndPlot();
    }
}
}
