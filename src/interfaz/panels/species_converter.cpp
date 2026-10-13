#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de Convertidor de Especies
// ═══════════════════════════════════════════════════════════════════════════

PanelConvertidorEspecies::PanelConvertidorEspecies() {
    inputs_ = {
        {"S", 32.850, 0.0},
        {"Cu", 25.700, 0.0},
        {"Fe", 24.700, 0.0},
        {"Ni", 0.550, 0.0},
        {"Co", 0.010, 0.0},
        {"Zn", 4.050, 0.0},
        {"Pb", 3.900, 0.0},
        {"Sb", 0.200, 0.0},
        {"As", 0.130, 0.0},
        {"Ag", 0.090, 0.0},
        {"Au", 0.001, 0.0},
        {"SiO2", 4.650, 0.0},
        {"Al2O3", 2.200, 0.0},
        {"MgO", 0.150, 0.0},
        {"O", 3.572, 0.0},
        {"H", 0.000, 0.0},
        {"[Others]", 0.802, 0.0}
    };
    outputs_ = {
        {"CuS", 0.0, 0.0, 0.0, ""},
        {"Cu2S", 0.0, 0.0, 0.0, ""},
        {"CuFeS2", 0.0, 60.0, 10.0, ""},
        {"FeS", 0.0, 0.0, 0.0, ""},
        {"FeS2", 0.0, 15.0, 10.0, ""},
        {"NiS", 0.0, 0.0, 0.0, ""},
        {"CoS", 0.0, 0.0, 0.0, ""},
        {"ZnS", 0.0, 0.0, 0.0, ""},
        {"PbS", 0.0, 4.5, 10.0, ""},
        {"Sb2S3", 0.0, 0.0, 0.0, ""},
        {"As2S3", 0.0, 0.0, 0.0, ""},
        {"Ag2S", 0.0, 0.0, 0.0, ""},
        {"Au", 0.0, 0.0, 0.0, ""},
        {"SiO2", 0.0, 0.0, 0.0, ""},
        {"Al2O3", 0.0, 0.0, 0.0, ""},
        {"MgO", 0.0, 0.0, 0.0, ""},
        {"[Others]", 0.0, 0.0, 0.0, ""}
    };
    calcular();
}

void PanelConvertidorEspecies::calcular() {
    std::map<std::string, double> input_map;
    input_total_wt_ = 0.0;
    for (auto& in : inputs_) {
        input_map[std::string(in.element)] = in.wt_pct;
        input_total_wt_ += in.wt_pct;
        in.calc_elem = 0;
    }
    std::vector<std::string> target_species;
    for (auto& out : outputs_) {
        target_species.push_back(std::string(out.species));
    }
    auto result_map = motor_.analisisAEspecies(input_map, target_species);
    
    output_total_wt_ = 0.0;
    for (auto& out : outputs_) {
        auto it = result_map.find(std::string(out.species));
        if (it != result_map.end()) {
            out.calc_wt = it->second;
            output_total_wt_ += it->second;
        } else {
            out.calc_wt = 0.0;
        }
    }
    for (auto& in : inputs_) {
        for (const auto& out : outputs_) {
            if (std::string(in.element) == out.species) { in.calc_elem += out.calc_wt; continue; }
            const auto composition = cma::application::modules::SpeciesConverter::composicion(out.species);
            const auto element = composition.find(in.element);
            if (element != composition.end()) in.calc_elem += out.calc_wt * element->second;
        }
    }
}

void PanelConvertidorEspecies::guardar_json(const char* path) {
    nlohmann::json j;
    for (auto& in : inputs_) j["inputs"].push_back({{"element", std::string(in.element)}, {"wt", in.wt_pct}});
    for (auto& out : outputs_) j["outputs"].push_back({{"species", std::string(out.species)}, {"target", out.target_wt}, {"coeff", out.weight_coeff}});
    std::ofstream o(path);
    if (o.is_open()) o << j.dump(4);
}

void PanelConvertidorEspecies::cargar_json(const char* path) {
    std::ifstream i(path);
    if (i.is_open()) {
        nlohmann::json j;
        i >> j;
        inputs_.clear();
        for (auto& item : j["inputs"]) {
            InputItem in{};
            strncpy(in.element, item["element"].get<std::string>().c_str(), 31);
            in.wt_pct = item["wt"];
            inputs_.push_back(in);
        }
        outputs_.clear();
        for (auto& item : j["outputs"]) {
            OutputItem out{};
            strncpy(out.species, item["species"].get<std::string>().c_str(), 31);
            out.target_wt = item.value("target", 0.0);
            out.weight_coeff = item.value("coeff", 0.0);
            outputs_.push_back(out);
        }
        calcular();
    }
}

void PanelConvertidorEspecies::renderizar() {
    // Toolbar
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0,0,0,0));
    if (ImGui::Button(tr("Nuevo"))) { inputs_.clear(); outputs_.clear(); calcular(); } ImGui::SameLine();
    if (ImGui::Button(tr("Abrir"))) { ImGui::OpenPopup("Open Conversion"); } ImGui::SameLine();
    if (ImGui::Button(tr("Guardar"))) { ImGui::OpenPopup("Save Conversion"); } ImGui::SameLine();
    ImGui::TextDisabled(tr(" | ")); ImGui::SameLine();
    if (ImGui::Button(tr("Inicializar"))) {
        inputs_.clear(); outputs_.clear();
        input_total_wt_ = 0.0; output_total_wt_ = 0.0;
        mostrar_grafico_ = false;
    } ImGui::SameLine();
    if (ImGui::Button(tr("Resolver"))) { calcular(); } ImGui::SameLine();
    if (ImGui::Button(tr("Gráficas"))) { mostrar_grafico_ = !mostrar_grafico_; }
    ImGui::PopStyleColor();
    ImGui::Separator();
    
    if (ImGui::BeginPopupModal("Save Conversion", NULL, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::InputText(tr("Filename"), save_filename_, 128);
        if (ImGui::Button(tr("Guardar"), ImVec2(120, 0))) {
            guardar_json(save_filename_);
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Cancelar"), ImVec2(120, 0))) { ImGui::CloseCurrentPopup(); }
        ImGui::EndPopup();
    }
    
    if (ImGui::BeginPopupModal("Open Conversion", NULL, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::InputText(tr("Filename"), open_filename_, 128);
        if (ImGui::Button(tr("Abrir"), ImVec2(120, 0))) {
            cargar_json(open_filename_);
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Cancelar"), ImVec2(120, 0))) { ImGui::CloseCurrentPopup(); }
        ImGui::EndPopup();
    }
    
    ImVec2 space = ImGui::GetContentRegionAvail();
    float table_w = space.x * 0.65f;
    float charts_w = mostrar_grafico_ ? (space.x - table_w - 20.0f) : 0.0f;
    
    ImGui::BeginChild("TablePane", ImVec2(table_w, space.y), true);
    // Info block
    ImGui::Text(tr("Name:   Example Conversion"));
    ImGui::Text(tr("Date:   (Today)"));
    ImGui::Text(tr("Author: CMA Chemist"));
    ImGui::Spacing();
    
    // Main Split Table for INPUT and OUTPUT
    if (ImGui::BeginTable("MainGrid", 9, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_SizingFixedFit)) {
        ImGui::TableSetupColumn("Analysis Elements", ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableSetupColumn("wt-%", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("Act", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        ImGui::TableSetupColumn("Calculated Elements", ImGuiTableColumnFlags_WidthFixed, 140.0f);
        ImGui::TableSetupColumn("Calculated wt-%", ImGuiTableColumnFlags_WidthFixed, 110.0f);
        ImGui::TableSetupColumn("Target wt-%", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("Weight Coeff", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("Tipo", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("Act", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        
        ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
        ImGui::TableSetColumnIndex(0); ImGui::Text(tr("INPUT"));
        ImGui::TableSetColumnIndex(3); ImGui::Text(tr("OUTPUT"));
        ImGui::TableHeadersRow();
        
        // Total Row
        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Total"));
        ImGui::TableSetColumnIndex(1); ImGui::Text(tr("%.3f"), input_total_wt_);
        ImGui::TableSetColumnIndex(4); ImGui::Text(tr("%.3f"), output_total_wt_);
        double reconstructed = 0;
        for (const auto& input : inputs_) reconstructed += input.calc_elem;
        ImGui::TableSetColumnIndex(5); ImGui::Text("%.3f", reconstructed);
        
        int input_to_delete = -1, output_to_delete = -1;
        int rows = static_cast<int>(std::max(inputs_.size(), outputs_.size()));
        for (int i = 0; i < rows + 1; ++i) { // +1 for Add Row buttons
            ImGui::TableNextRow();
            if (i < (int)inputs_.size()) {
                ImGui::TableSetColumnIndex(0);
                ImGui::SetNextItemWidth(-FLT_MIN);
                ImGui::PushID(i * 10 + 0);
                ImGui::InputText("##elem", inputs_[i].element, 32);
                ImGui::PopID();
                ImGui::TableSetColumnIndex(1);
                ImGui::SetNextItemWidth(-FLT_MIN);
                ImGui::PushID(i * 10 + 1);
                if (ImGui::InputDouble("##wt", &inputs_[i].wt_pct, 0, 0, "%.3f")) { calcular(); }
                ImGui::PopID();
                ImGui::TableSetColumnIndex(2);
                if (ImGui::Button(tr("-"), ImVec2(20,20))) input_to_delete = i;
                
                ImGui::TableSetColumnIndex(3);
                ImGui::Text(tr("%s"), inputs_[i].element);
            } else if (i == (int)inputs_.size()) {
                ImGui::TableSetColumnIndex(0);
                if (ImGui::Button(tr("+ Add Elem"))) {
                    InputItem item{};
                    strncpy(item.element, "New", 31);
                    inputs_.push_back(item);
                }
            }
            if (i < (int)outputs_.size()) {
                ImGui::TableSetColumnIndex(4);
                ImGui::SetNextItemWidth(90);
                ImGui::PushID(i * 10 + 3);
                ImGui::InputText("##spec", outputs_[i].species, 32);
                ImGui::PopID();
                ImGui::SameLine(90);
                ImGui::Text(tr("%.3f"), outputs_[i].calc_wt);
                
                ImGui::TableSetColumnIndex(5);
                if (outputs_[i].target_wt >= 0) {
                    ImGui::SetNextItemWidth(-FLT_MIN);
                    ImGui::PushID(i * 10 + 4);
                    ImGui::InputDouble("##twt", &outputs_[i].target_wt, 0, 0, "%.3f");
                    ImGui::PopID();
                }
                
                ImGui::TableSetColumnIndex(6);
                if (outputs_[i].weight_coeff >= 0) {
                    ImGui::SetNextItemWidth(-FLT_MIN);
                    ImGui::PushID(i * 10 + 5);
                    ImGui::InputDouble("##wc", &outputs_[i].weight_coeff, 0, 0, "%.3f");
                    ImGui::PopID();
                }
                
                ImGui::TableSetColumnIndex(7);
                ImGui::Text(tr("%s"), outputs_[i].type);
                
                ImGui::TableSetColumnIndex(8);
                if (ImGui::Button(tr("-"), ImVec2(20,20))) output_to_delete = i;
            } else if (i == (int)outputs_.size()) {
                ImGui::TableSetColumnIndex(4);
                if (ImGui::Button(tr("+ Add Species"))) {
                    OutputItem item{};
                    strncpy(item.species, "New", 31);
                    outputs_.push_back(item);
                }
            }
        }
        ImGui::EndTable();
        if (input_to_delete >= 0) inputs_.erase(inputs_.begin() + input_to_delete);
        if (output_to_delete >= 0) outputs_.erase(outputs_.begin() + output_to_delete);
        if (input_to_delete >= 0 || output_to_delete >= 0) calcular();
    }
    ImGui::EndChild();
    
    ImGui::SameLine();

    if (mostrar_grafico_) {
    ImGui::BeginChild("ChartsPane", ImVec2(charts_w, space.y), true);
    ImGui::Text(tr("Gráficas"));
    ImGui::Separator();
    
    // Process pie chart data
    std::vector<const char*> in_labels;
    std::vector<double> in_vals;
    for (const auto& in : inputs_) {
        if (in.wt_pct > 1.0) { // Only show major elements
            in_labels.push_back(in.element);
            in_vals.push_back(in.wt_pct);
        }
    }
    std::vector<const char*> out_labels;
    std::vector<double> out_vals;
    for (const auto& out : outputs_) {
        if (out.calc_wt > 1.0) { // Only show major species
            out_labels.push_back(out.species);
            out_vals.push_back(out.calc_wt);
        }
    }
    
    float plot_h = (space.y - 40) / 2.0f;
    if (ImPlot::BeginPlot("Input analysis", ImVec2(-1, plot_h), ImPlotFlags_NoInputs | ImPlotFlags_NoLegend | ImPlotFlags_NoMouseText)) {
        ImPlot::SetupAxes(NULL, NULL, ImPlotAxisFlags_NoDecorations, ImPlotAxisFlags_NoDecorations);
        if (!in_vals.empty()) {
            ImPlot::PlotPieChart(in_labels.data(), in_vals.data(), (int)in_vals.size(), 0.5, 0.5, 0.45, "%.1f", 90, ImPlotPieChartFlags_Normalize);
        }
        ImPlot::EndPlot();
    }
    
    if (ImPlot::BeginPlot("Output analysis", ImVec2(-1, plot_h), ImPlotFlags_NoInputs | ImPlotFlags_NoLegend | ImPlotFlags_NoMouseText)) {
        ImPlot::SetupAxes(NULL, NULL, ImPlotAxisFlags_NoDecorations, ImPlotAxisFlags_NoDecorations);
        if (!out_vals.empty()) {
            ImPlot::PlotPieChart(out_labels.data(), out_vals.data(), (int)out_vals.size(), 0.5, 0.5, 0.45, "%.1f", 90, ImPlotPieChartFlags_Normalize);
        }
        ImPlot::EndPlot();
    }

    ImGui::EndChild();
    } // mostrar_grafico_
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Unidades (conversor)
// ═══════════════════════════════════════════════════════════════════════════
void PanelUnidades::renderizar() {
    auto& db = base_datos::GestorUnidades::instancia();
    if (!db_inicializada_) {
        db.cargar_desde_json("unidades_db.json");
        db_inicializada_ = true;
    }
    
    if (db.categorias().empty()) {
        ImGui::Text(tr("No categories found in database."));
        return;
    }
    
    const auto& categorias = db.categorias();
    
    ImGui::Text(tr("Conversión de Unidades"));
    ImGui::Separator();
    
    // UI Layout similar to image
    ImGui::PushItemWidth(300);
    if (ImGui::BeginCombo("##categoria", categorias[categoria_].nombre.c_str())) {
        for (int i = 0; i < (int)categorias.size(); ++i) {
            bool is_selected = (categoria_ == i);
            if (ImGui::Selectable(categorias[i].nombre.c_str(), is_selected)) {
                categoria_ = i;
                unidad_origen_ = 0;
                unidad_destino_ = std::min(1, (int)categorias[i].unidades.size() - 1);
            }
            if (is_selected) ImGui::SetItemDefaultFocus();
        }
        ImGui::EndCombo();
    }
    ImGui::PopItemWidth();
    
    ImGui::Spacing();
    
    const auto& unidades = categorias[categoria_].unidades;
    
    ImGui::BeginGroup();
    ImGui::PushItemWidth(120);
    if (ImGui::BeginCombo("##origen", unidades[unidad_origen_].nombre.c_str())) {
        for (int i = 0; i < (int)unidades.size(); ++i) {
            bool is_selected = (unidad_origen_ == i);
            if (ImGui::Selectable(unidades[i].nombre.c_str(), is_selected)) unidad_origen_ = i;
            if (is_selected) ImGui::SetItemDefaultFocus();
        }
        ImGui::EndCombo();
    }
    ImGui::PopItemWidth();
    
    ImGui::SameLine();
    if (ImGui::Button(tr("Limpiar Hoja"), ImVec2(120, 0))) {
        for (auto& f : filas_) { f.d1 = 0.0; f.d2 = 0.0; f.activa = false; }
    }
    ImGui::EndGroup();
    
    ImGui::SameLine(ImGui::GetWindowWidth() / 2.0f);
    
    ImGui::BeginGroup();
    ImGui::PushItemWidth(120);
    if (ImGui::BeginCombo("##destino", unidades[unidad_destino_].nombre.c_str())) {
        for (int i = 0; i < (int)unidades.size(); ++i) {
            bool is_selected = (unidad_destino_ == i);
            if (ImGui::Selectable(unidades[i].nombre.c_str(), is_selected)) unidad_destino_ = i;
            if (is_selected) ImGui::SetItemDefaultFocus();
        }
        ImGui::EndCombo();
    }
    ImGui::PopItemWidth();
    ImGui::EndGroup();
    
    ImGui::Separator();
    
    // Formula bar
    double f = db.obtener_factor(categorias[categoria_].nombre, unidad_origen_, unidad_destino_);
    char fbuf[128];
    if (categorias[categoria_].nombre == "Temperature") snprintf(fbuf, sizeof(fbuf), "(complex formula)");
    else snprintf(fbuf, sizeof(fbuf), "%.12g", f);
    
    ImGui::PushItemWidth(-1);
    ImGui::InputText("##formula", fbuf, sizeof(fbuf), ImGuiInputTextFlags_ReadOnly);
    ImGui::PopItemWidth();
    
    ImGui::Spacing();
    
    // Spreadsheet Table
    if (ImGui::BeginTable("SheetUnits", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable | ImGuiTableFlags_SizingFixedFit)) {
        ImGui::TableSetupScrollFreeze(1, 1);
        ImGui::TableSetupColumn("Fila", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        ImGui::TableSetupColumn("Data 1", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Units 1", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("Data 2", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Units 2", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableHeadersRow();
        
        // Populate spreadsheet rows
        if (filas_.size() < 100) filas_.resize(100); // Default 100 rows
        
        for (int r = 0; r < 100; ++r) {
            ImGui::TableNextRow();
            
            // Row number
            ImGui::TableSetColumnIndex(0);
            ImGui::Text(tr("%d"), r + 1);
            
            // Data 1
            ImGui::TableSetColumnIndex(1);
            ImGui::SetNextItemWidth(-FLT_MIN);
            ImGui::PushID(r * 2);
            if (ImGui::InputDouble("##d1", &filas_[r].d1, 0.0, 0.0, "%.6g", ImGuiInputTextFlags_CharsDecimal)) {
                // If it wasn't active, activate it with current top bar settings
                if (!filas_[r].activa) {
                    filas_[r].cat = categorias[categoria_].nombre;
                    filas_[r].u1 = unidad_origen_;
                    filas_[r].u2 = unidad_destino_;
                    filas_[r].nom_u1 = unidades[unidad_origen_].nombre;
                    filas_[r].nom_u2 = unidades[unidad_destino_].nombre;
                    filas_[r].activa = true;
                }
                // Compute Data 2 using the ROW's stored category and units
                filas_[r].d2 = db.convertir(filas_[r].d1, filas_[r].cat, filas_[r].u1, filas_[r].u2);
            }
            if (ImGui::IsItemDeactivatedAfterEdit() && filas_[r].activa) {
                 filas_[r].d2 = db.convertir(filas_[r].d1, filas_[r].cat, filas_[r].u1, filas_[r].u2);
            }
            ImGui::PopID();
            
            // Units 1
            ImGui::TableSetColumnIndex(2);
            if (filas_[r].activa) {
                ImGui::Text(tr("%s"), filas_[r].nom_u1.c_str());
            }
            
            // Data 2
            ImGui::TableSetColumnIndex(3);
            ImGui::SetNextItemWidth(-FLT_MIN);
            ImGui::PushID(r * 2 + 1);
            if (ImGui::InputDouble("##d2", &filas_[r].d2, 0.0, 0.0, "%.6g", ImGuiInputTextFlags_CharsDecimal)) {
                // If it wasn't active, activate it with current top bar settings
                if (!filas_[r].activa) {
                    filas_[r].cat = categorias[categoria_].nombre;
                    filas_[r].u1 = unidad_origen_;
                    filas_[r].u2 = unidad_destino_;
                    filas_[r].nom_u1 = unidades[unidad_origen_].nombre;
                    filas_[r].nom_u2 = unidades[unidad_destino_].nombre;
                    filas_[r].activa = true;
                }
                // Compute Data 1 backwards using the ROW's stored category and units
                filas_[r].d1 = db.convertir(filas_[r].d2, filas_[r].cat, filas_[r].u2, filas_[r].u1);
            }
            if (ImGui::IsItemDeactivatedAfterEdit() && filas_[r].activa) {
                 filas_[r].d1 = db.convertir(filas_[r].d2, filas_[r].cat, filas_[r].u2, filas_[r].u1);
            }
            ImGui::PopID();
            
            // Units 2
            ImGui::TableSetColumnIndex(4);
            if (filas_[r].activa) {
                ImGui::Text(tr("%s"), filas_[r].nom_u2.c_str());
            }
        }
        
        ImGui::EndTable();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Estimación de Propiedades
// ═══════════════════════════════════════════════════════════════════════════
void PanelEstimacionProp::renderizar() {
    // Header
    ImGui::Text(tr("H, S, Cp Estimate"));
    ImGui::Separator();
    
    // Inputs layout
    ImGui::PushItemWidth(150);
    ImGui::InputText(tr("Especie"), formula_, sizeof(formula_));
    ImGui::InputDouble("Temperature", &temperatura_);
    ImGui::PopItemWidth();
    
    ImGui::SameLine();
    ImGui::Checkbox(tr("°C"), &es_celsius_);
    ImGui::SameLine();
    bool is_kelvin = !es_celsius_;
    if (ImGui::Checkbox(tr("K"), &is_kelvin)) es_celsius_ = !is_kelvin;
    
    ImGui::SameLine(350);
    ImGui::BeginGroup();
    ImGui::TextDisabled(tr("Tipo Especie"));
    ImGui::PushItemWidth(150);
    ImGui::Combo("##type", &tipo_especie_, "Not Specified\0Inorganic\0Organic\0\0");
    ImGui::PopItemWidth();
    ImGui::EndGroup();
    
    ImGui::SameLine(550);
    if (ImGui::Button(tr("Estimar"), ImVec2(100, 40))) {
        std::string t_str = "Not Specified";
        if (tipo_especie_ == 1) t_str = "Inorganic";
        else if (tipo_especie_ == 2) t_str = "Organic";
        
        double t_c = es_celsius_ ? temperatura_ : temperatura_ - 273.15;
        auto est = motor_.estimar_hsc(formula_, t_c, t_str);
        if (est.mw > 0) {
            resultados_.push_back(est);
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(mostrar_grafico_ ? "Hide Graph" : "Show Graph", ImVec2(100, 40))) {
        mostrar_grafico_ = !mostrar_grafico_;
    }
    
    ImGui::Separator();
    
    if (mostrar_grafico_) {
        if (!grafico_calculado_) {
            auto& db = base_datos::GestorDB::instancia();
            auto especies = db.todas_especies();
            std::vector<double> errors;
            for (const auto* e : especies) {
                if (e->formula.empty()) continue;
                auto est = motor_.estimar_hsc(e->formula, 25.0, "Not Specified");
                if (est.mw > 0 && est.has_db) {
                    exp_h_.push_back(est.h_db);
                    est_h_.push_back(est.h_est);
                    db_formulas_.push_back(e->formula);
                    errors.push_back(std::abs(est.h_db - est.h_est));
                }
            }
            if (!errors.empty()) {
                species_count_ = errors.size();
                double sum = 0;
                for (double err : errors) sum += err;
                mean_error_ = sum / errors.size();
                
                std::sort(errors.begin(), errors.end());
                median_error_ = errors[errors.size() / 2];
            }
            grafico_calculado_ = true;
        }

        ImGui::SetNextWindowSize(ImVec2(600, 600), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("H Enthalpy of Aqueous Species"), &mostrar_grafico_)) {
            if (ImPlot::BeginPlot("##parity_plot", ImVec2(-1, -1))) {
                ImPlot::SetupAxes("Experimental", "Estimated");
                ImPlot::SetupAxesLimits(-2500, 500, -2500, 500);
                
                std::vector<double> x_kcal, y_kcal;
                for(size_t i = 0; i < exp_h_.size(); ++i) {
                    x_kcal.push_back(exp_h_[i] / 4.184);
                    y_kcal.push_back(est_h_[i] / 4.184);
                }

                ImPlot::SetNextMarkerStyle(ImPlotMarker_Cross, 3, ImVec4(1,0,0,1), 1, ImVec4(1,0,0,1));
                ImPlot::PlotScatter("##species", x_kcal.data(), y_kcal.data(), x_kcal.size());
                
                double line_pts[2] = {-2500, 500};
                ImPlot::SetNextLineStyle(ImVec4(0,0,0,1));
                ImPlot::PlotLine("##identity", line_pts, line_pts, 2);
                
                char anno_text[256];
                snprintf(anno_text, sizeof(anno_text), "H Enthalpy of Aqueous Species\n- kcal / mol\n- %d Species\n- Mean error = %.2f kcal/mol\n- Median error = %.2f kcal/mol", 
                         species_count_, mean_error_/4.184, median_error_/4.184);
                ImPlot::PlotText(anno_text, -1500, 200); 

                if (ImPlot::IsPlotHovered()) {
                    ImPlotPoint mouse = ImPlot::GetPlotMousePos();
                    double min_dist = 1e9;
                    int closest_idx = -1;
                    for(size_t i = 0; i < x_kcal.size(); ++i) {
                        double dx = x_kcal[i] - mouse.x;
                        double dy = y_kcal[i] - mouse.y;
                        double dist = dx*dx + dy*dy;
                        if(dist < min_dist && dist < 900.0) { // 30 kcal/mol radio de hover
                            min_dist = dist;
                            closest_idx = i;
                        }
                    }
                    if (closest_idx >= 0) {
                        ImGui::BeginTooltip();
                        ImGui::Text(tr("Sustancia: %s"), db_formulas_[closest_idx].c_str());
                        ImGui::Text(tr("Experimental: %.2f kcal/mol"), x_kcal[closest_idx]);
                        ImGui::Text(tr("Estimated: %.2f kcal/mol"), y_kcal[closest_idx]);
                        ImGui::EndTooltip();
                    }
                }

                ImPlot::EndPlot();
            }
        }
        ImGui::End();
    }
    
    // Table
    if (ImGui::BeginTable("##hsc_estimates", 12, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable, ImVec2(-1, 400))) {
        ImGui::TableSetupColumn("Chemical Formula");
        ImGui::TableSetupColumn("Tipo Especie");
        ImGui::TableSetupColumn("T (°C)");
        ImGui::TableSetupColumn("T (K)");
        ImGui::TableSetupColumn("Ox. Num");
        ImGui::TableSetupColumn("Weight\ng/mol");
        ImGui::TableSetupColumn("H Estimate\nkJ/mol");
        ImGui::TableSetupColumn("H Database\nkJ/mol");
        ImGui::TableSetupColumn("S Estimate\nJ/mol*K");
        ImGui::TableSetupColumn("S Database\nJ/mol*K");
        ImGui::TableSetupColumn("Cp Estimate\nJ/mol*K");
        ImGui::TableSetupColumn("Cp Database\nJ/mol*K");
        ImGui::TableHeadersRow();
        
        for (const auto& r : resultados_) {
            ImGui::TableNextRow();
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), r.formula.c_str());
            ImGui::TableNextColumn(); ImGui::Text(tr("%s"), r.tipo.c_str());
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.T_C);
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.T_K);
            ImGui::TableNextColumn(); ImGui::TextColored(cma::interfaz::AdaptiveColor(0,0,1,1), "1.00"); 
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.mw);
            
            // H
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.h_est);
            ImGui::TableNextColumn(); if(r.has_db) ImGui::Text(tr("%.2f"), r.h_db); else ImGui::Text(tr("-"));
            // S
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.s_est);
            ImGui::TableNextColumn(); if(r.has_db) ImGui::Text(tr("%.2f"), r.s_db); else ImGui::Text(tr("-"));
            // Cp
            ImGui::TableNextColumn(); ImGui::Text(tr("%.2f"), r.cp_est);
            ImGui::TableNextColumn(); if(r.has_db) ImGui::Text(tr("%.2f"), r.cp_db); else ImGui::Text(tr("-"));
        }
        
        ImGui::EndTable();
    }
}

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
