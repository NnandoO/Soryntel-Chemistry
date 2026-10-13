#include "scientific.hpp"
#include <imgui.h>
#include <cmath>
#include <cstdio>
#include "tools/ExternalAPI.hpp"

namespace cma::interfaz {
void MineralProcessingPanel::render() {
    ImGui::TextUnformatted("Procesamiento mineral por tamaño");
    ImGui::TextWrapped("Introduzca caudales y parámetros medidos. No hay calibración industrial predeterminada. Flotación: modelo de primer orden batch/flujo pistón, no banco CSTR.");
    bool edited=ImGui::Combo("Operación",&operation_,"Clasificación (producto grueso)\0Flotación\0Clasificación y flotación de finos\0");
    edited|=ImGui::InputDouble("Agua de alimentación [kg/h]",&water_);
    if (ImGui::Button("Añadir fracción mineral")) {
        Row row; std::snprintf(row.name.data(),row.name.size(),"Mineral %zu",rows_.size()+1);
        rows_.push_back(row); edited=true;
    }
    if (ImGui::BeginTable("mineral_feed",6,ImGuiTableFlags_Borders|ImGuiTableFlags_RowBg|ImGuiTableFlags_ScrollX)) {
        for (auto label : {"Mineral","Tamaño [µm]","Caudal [kg/h]","k [1/min]","R infinito [0..1]","Acción"}) ImGui::TableSetupColumn(label);
        ImGui::TableHeadersRow(); int removed=-1;
        for (size_t i=0;i<rows_.size();++i) {
            auto& row=rows_[i]; ImGui::PushID(static_cast<int>(i)); ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1); edited|=ImGui::InputText("##mineral",row.name.data(),row.name.size());
            ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1); edited|=ImGui::InputDouble("##size",&row.size);
            ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1); edited|=ImGui::InputDouble("##mass",&row.mass);
            ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1); edited|=ImGui::InputDouble("##rate",&row.rate);
            ImGui::TableNextColumn(); ImGui::SetNextItemWidth(-1); edited|=ImGui::InputDouble("##limit",&row.limit);
            ImGui::TableNextColumn(); if (ImGui::SmallButton("Quitar")) removed=static_cast<int>(i); ImGui::PopID();
        }
        if (removed>=0) { rows_.erase(rows_.begin()+removed); edited=true; }
        ImGui::EndTable();
    }
    if (operation_!=1) {
        edited|=ImGui::InputDouble("d50 [µm]",&d50_);
        edited|=ImGui::InputDouble("Agudeza de clasificación [>0]",&sharpness_);
        edited|=ImGui::InputDouble("Bypass [0..1)",&bypass_);
        edited|=ImGui::InputDouble("Fracción de agua a gruesos [0..1]",&water_split_);
    }
    if (operation_!=0) {
        edited|=ImGui::InputDouble("Residencia de flotación [min]",&residence_);
        edited|=ImGui::InputDouble("Fracción de agua a concentrado [0..1]",&float_water_);
    }
    if (edited) calculated_=false;
    ImGui::BeginDisabled(rows_.empty());
    if (ImGui::Button("Calcular balances")) {
        motores::ParticleStream feed; feed.water_kg_h=water_;
        std::map<std::string,motores::FlotationKinetics> kinetics;
        bool conflicting=false;
        for (const auto& row : rows_) {
            feed.particles.push_back({row.name.data(),row.size,row.mass});
            auto [it,inserted]=kinetics.emplace(row.name.data(),motores::FlotationKinetics{row.rate,row.limit});
            if (!inserted && (it->second.rate_per_min!=row.rate || it->second.ultimate_recovery!=row.limit)) conflicting=true;
        }
        if (conflicting && operation_!=0) { result_={}; result_.message="Un mineral debe tener parámetros cinéticos consistentes en todas sus fracciones."; }
        else if (operation_==1) result_=motores::MineralProcessing::flotation(feed,kinetics,residence_,float_water_);
        else {
            classification_=motores::MineralProcessing::classify(feed,d50_,sharpness_,bypass_,water_split_);
            result_=classification_;
            if (operation_==2 && classification_.ok) result_=motores::MineralProcessing::flotation(classification_.residue,kinetics,residence_,float_water_);
        }
        calculated_=true;
    }
    ImGui::EndDisabled();
    if (rows_.empty()) ImGui::TextWrapped("Añada una fracción para introducir la alimentación. Los caudales iniciales son cero; no representan una planta real.");
    if (calculated_) {
        ImGui::Separator(); ImGui::TextWrapped("%s",result_.message.c_str());
        if (result_.ok) {
            ImGui::Text("Residual de masa: %.6g kg/h",result_.mass_residual_kg_h);
            if (operation_==2) ImGui::Text("Gruesos retirados: %.6g kg/h de sólidos + %.6g kg/h de agua",motores::MineralProcessing::solids_kg_h(classification_.product),classification_.product.water_kg_h);
            ImGui::Text("Agua: producto %.6g / residuo %.6g kg/h",result_.product.water_kg_h,result_.residue.water_kg_h);
            if (ImGui::BeginTable("mineral_products",4,ImGuiTableFlags_Borders|ImGuiTableFlags_RowBg)) {
                for (auto label : {"Mineral","Tamaño [µm]","Producto [kg/h]","Residuo [kg/h]"}) ImGui::TableSetupColumn(label);
                ImGui::TableHeadersRow();
                for (size_t i=0;i<result_.product.particles.size();++i) {
                    const auto& p=result_.product.particles[i];
                    ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::TextUnformatted(p.mineral.c_str());
                    ImGui::TableNextColumn(); ImGui::Text("%.6g",p.diameter_um);
                    ImGui::TableNextColumn(); ImGui::Text("%.6g",p.mass_kg_h);
                    ImGui::TableNextColumn(); ImGui::Text("%.6g",result_.residue.particles[i].mass_kg_h);
                }
                ImGui::EndTable();
            }
        }
    }
    if (ImGui::CollapsingHeader("Energía de Bond: estimación, no simulación de molienda")) {
        ImGui::InputDouble("Índice de trabajo [kWh/t]",&index_);
        ImGui::InputDouble("F80 [µm]",&f80_); ImGui::InputDouble("P80 [µm]",&p80_);
        const double energy=motores::MineralProcessing::bond_energy_kWh_t(index_,f80_,p80_);
        if (std::isfinite(energy)) ImGui::Text("Energía específica: %.6g kWh/t",energy);
        else ImGui::TextWrapped("Introduzca un índice positivo y F80 >= P80 > 0.");
    }
    if (ImGui::CollapsingHeader("Calibrar cinética con datos experimentales")) {
        ImGui::TextWrapped("CSV numérico sin encabezado: tiempo [min], recuperación [fracción 0..1]. R infinito debe medirse por separado. El ajuste no certifica el modelo para otra planta.");
        if (ImGui::InputDouble("R infinito medido",&calibration_limit_)) fit_={};
        if (ImGui::Button("Importar CSV y ajustar k")) {
            const auto path=dialog_.select("Datos experimentales de flotación","*.csv",false);
            if (!path.empty()) {
                std::vector<std::vector<double>> data;
                application::modules::ExternalAPI api;
                if (api.importarCSV(path,data)) fit_=application::fit_flotation(data,calibration_limit_);
                else { fit_={}; fit_.message="CSV inválido o ilegible; el ajuste anterior se descartó."; }
            }
        }
        if (!fit_.message.empty()) ImGui::TextWrapped("%s",fit_.message.c_str());
        if (fit_.ok) {
            ImGui::Text("k: %.8g 1/min · RMSE de recuperación: %.6g · %zu muestras",fit_.rate_per_min,fit_.recovery_rmse,fit_.samples);
            ImGui::TextWrapped("Copie k y R infinito al mineral correspondiente; no se asignan a otros minerales automáticamente.");
        }
    }
}
}
