#include "shared.hpp"
#include <random>
using namespace cma::interfaz;
void cma::interfaz::PanelMapGPS::refrescar_datos() {
    stock_db_.clear();
    if (!flowsheet_) return;
    
    // Obtener nodos del diagrama y mostrarlos como elementos geolocalizados
    for (const auto& n : flowsheet_->nodos()) {
        StockItem item;
        item.selected_for_mix = false;
        
        std::snprintf(item.place, sizeof(item.place), "%s", "Plant");
        std::snprintf(item.mat, sizeof(item.mat), "%s", n.nombre.c_str());
        
        item.amt = 0.0f; // Inventory must be entered; a flowsheet rate is not stock.
        item.fv = 0.0f;
        item.o2 = 0.0f; item.cu = 0.0f; item.fe = 0.0f; item.ni = 0.0f; item.s = 0.0f;
        
        // Coordenadas en el mapa = coordenadas en el canvas
        item.x = n.canvas_x;
        item.y = n.canvas_y;
        
        stock_db_.push_back(item);
    }
}

cma::interfaz::PanelMapGPS::PanelMapGPS() {
    refrescar_datos();
}

void cma::interfaz::PanelMapGPS::renderizar() {
    // --- Top Bar (Tabs/Modes) ---
    ImGui::BeginChild("map_top_bar", ImVec2(0, 45), true);
    if (ImGui::Button(tr("Mode"), ImVec2(60, 25))) {
        view_mode_ = (view_mode_ == 0) ? 1 : 0;
    }
    ImGui::SameLine();
    
    const char* places_items[] = {"Näytä kaikki varastopaikat", "Valitut varastopaikat"};
    static int places_idx = 0;
    ImGui::PushItemWidth(200);
    ImGui::Combo("##places_combo", &places_idx, places_items, IM_ARRAYSIZE(places_items));
    ImGui::PopItemWidth();
    ImGui::SameLine();
    
    const char* maps_items[] = {"Pori tehdasalue", "Helsinki region", "Oulu map"};
    ImGui::PushItemWidth(200);
    ImGui::Combo("##maps_combo", &maps_idx_, maps_items, IM_ARRAYSIZE(maps_items));
    ImGui::PopItemWidth();
    ImGui::EndChild();

    float right_panel_width = 340.0f;
    float left_panel_width = ImGui::GetContentRegionAvail().x - right_panel_width - 8.0f;

    ImGui::BeginChild("map_left_panel", ImVec2(left_panel_width, 0), true);

    if (view_mode_ == 0) {
        // --- MAP VIEW ---
        ImVec2 canvas_pos = ImGui::GetCursorScreenPos();
        ImVec2 canvas_size = ImGui::GetContentRegionAvail();
        if (canvas_size.x < 50.0f) canvas_size.x = 50.0f;
        if (canvas_size.y < 50.0f) canvas_size.y = 50.0f;

        ImDrawList* draw_list = ImGui::GetWindowDrawList();
        
        // --- Theme Detection ---
        bool is_dark = ImGui::GetStyle().Colors[ImGuiCol_WindowBg].x < 0.5f;
        ImU32 col_bg, col_water, col_green, col_road, col_bldg, col_text;
        
        if (is_dark) {
            col_bg    = IM_COL32(35, 35, 40, 255);
            col_water = IM_COL32(20, 50, 80, 255);
            col_green = IM_COL32(40, 100, 50, 255);
            col_road  = IM_COL32(180, 50, 50, 255);
            col_bldg  = IM_COL32(80, 80, 80, 255);
            col_text  = IM_COL32(220, 220, 220, 255);
        } else {
            col_bg    = IM_COL32(245, 245, 240, 255);
            col_water = IM_COL32(150, 200, 255, 255);
            col_green = IM_COL32(180, 230, 180, 255);
            col_road  = IM_COL32(220, 100, 100, 255);
            col_bldg  = IM_COL32(200, 200, 200, 255);
            col_text  = IM_COL32(50, 50, 50, 255);
        }
        
        // Background
        draw_list->AddRectFilled(canvas_pos, ImVec2(canvas_pos.x + canvas_size.x, canvas_pos.y + canvas_size.y), col_bg);
        
        // Projection helper lambda
        auto W2S = [&](float wx, float wy) -> ImVec2 {
            return ImVec2(canvas_pos.x + wx * zoom_ + camera_x_, canvas_pos.y + wy * zoom_ + camera_y_);
        };
        auto W2Sf = [&](float s) -> float {
            return s * zoom_;
        };
        auto S2W = [&](float sx, float sy) -> ImVec2 {
            return ImVec2((sx - canvas_pos.x - camera_x_) / zoom_, (sy - canvas_pos.y - camera_y_) / zoom_);
        };
        
        // Invisible button to capture interactions
        ImGui::InvisibleButton("canvas", canvas_size);
        bool is_hovered = ImGui::IsItemHovered();
        bool is_active = ImGui::IsItemActive();
        
        ImVec2 mouse_pos = ImGui::GetIO().MousePos;
        
        // --- 2D Camera Controls ---
        if (is_hovered && ImGui::GetIO().MouseWheel != 0.0f) {
            float zoom_delta = ImGui::GetIO().MouseWheel * 0.1f;
            float prev_zoom = zoom_;
            zoom_ += zoom_delta;
            if (zoom_ < 0.1f) zoom_ = 0.1f;
            if (zoom_ > 10.0f) zoom_ = 10.0f;
            
            // Zoom towards mouse cursor
            ImVec2 mouse_world = S2W(mouse_pos.x, mouse_pos.y);
            camera_x_ += mouse_world.x * (prev_zoom - zoom_);
            camera_y_ += mouse_world.y * (prev_zoom - zoom_);
        }
        
        if (is_active && ImGui::IsMouseDragging(1)) { // Right click drag to pan
            camera_x_ += ImGui::GetIO().MouseDelta.x;
            camera_y_ += ImGui::GetIO().MouseDelta.y;
        }

        // --- Render Terrains ---
        draw_list->PushClipRect(canvas_pos, ImVec2(canvas_pos.x + canvas_size.x, canvas_pos.y + canvas_size.y), true);
        
        if (maps_idx_ == 0) {
            // "Pori tehdasalue" - Industrial Plant
            ImVec2 p_water[4] = {W2S(400, -100), W2S(800, -100), W2S(800, 600), W2S(350, 600)};
            draw_list->AddConvexPolyFilled(p_water, 4, col_water);
            
            ImVec2 p_green[4] = {W2S(150, 50), W2S(220, 50), W2S(220, 180), W2S(150, 180)};
            draw_list->AddConvexPolyFilled(p_green, 4, col_green);
            
            draw_list->AddCircleFilled(W2S(185, 70), W2Sf(12.0f), IM_COL32(200, 180, 0, 255));
            draw_list->AddCircleFilled(W2S(205, 160), W2Sf(10.0f), IM_COL32(200, 180, 0, 255));
            
            draw_list->AddLine(W2S(50, -100), W2S(50, 600), col_road, W2Sf(6.0f));
            draw_list->AddLine(W2S(50, 200), W2S(400, 200), col_road, W2Sf(6.0f));
            draw_list->AddLine(W2S(300, 200), W2S(450, 50), col_road, W2Sf(6.0f));
            
            ImVec2 p_bldg1[4] = {W2S(400, 20), W2S(480, 20), W2S(460, 80), W2S(380, 80)};
            draw_list->AddConvexPolyFilled(p_bldg1, 4, col_bldg);
            ImVec2 p_bldg2[4] = {W2S(70, 220), W2S(130, 220), W2S(130, 260), W2S(70, 260)};
            draw_list->AddConvexPolyFilled(p_bldg2, 4, col_bldg);
            
            draw_list->AddText(W2S(160, 115), col_text, "Outokumpu Research Oy");
        } 
        else if (maps_idx_ == 1) {
            // "Helsinki region" - Metropolitan / Water Heavy
            ImVec2 p_water1[4] = {W2S(-200, 300), W2S(600, 300), W2S(600, 800), W2S(-200, 800)};
            draw_list->AddConvexPolyFilled(p_water1, 4, col_water);
            
            ImVec2 p_island[4] = {W2S(200, 400), W2S(350, 400), W2S(400, 450), W2S(250, 550)};
            draw_list->AddConvexPolyFilled(p_island, 4, col_bg);
            
            draw_list->AddLine(W2S(-100, 150), W2S(600, 150), col_road, W2Sf(10.0f)); // Highway
            draw_list->AddLine(W2S(250, -100), W2S(250, 150), col_road, W2Sf(8.0f));
            draw_list->AddLine(W2S(250, 150), W2S(280, 450), col_road, W2Sf(4.0f)); // Bridge to island
            
            draw_list->AddText(W2S(10, 120), col_text, "Ring I Highway");
            draw_list->AddText(W2S(290, 450), col_text, "Island Storage Facility");
        }
        else if (maps_idx_ == 2) {
            // "Oulu map" - Green/Forest Heavy
            ImVec2 p_forest[4] = {W2S(-200, -100), W2S(600, -100), W2S(600, 600), W2S(-200, 600)};
            draw_list->AddConvexPolyFilled(p_forest, 4, col_green);
            
            // Winding River
            draw_list->AddLine(W2S(-100, 100), W2S(200, 250), col_water, W2Sf(30.0f));
            draw_list->AddLine(W2S(200, 250), W2S(400, 150), col_water, W2Sf(30.0f));
            draw_list->AddLine(W2S(400, 150), W2S(600, 300), col_water, W2Sf(30.0f));
            
            // Small dirt roads (brownish/grey)
            draw_list->AddLine(W2S(50, -100), W2S(50, 200), IM_COL32(150, 130, 100, 255), W2Sf(4.0f));
            draw_list->AddLine(W2S(50, 200), W2S(300, 350), IM_COL32(150, 130, 100, 255), W2Sf(4.0f));
            
            draw_list->AddText(W2S(250, 200), IM_COL32(0,0,0,255), "Oulujoki River");
            draw_list->AddText(W2S(100, 300), IM_COL32(0,0,0,255), "Forest Depot");
        }

        // --- Drag & Drop Nodes ---
        static int dragged_node = -1;
        
        if (is_hovered && ImGui::IsMouseClicked(0)) {
            for (size_t i = 0; i < stock_db_.size(); ++i) {
                ImVec2 s_pos = W2S(stock_db_[i].x, stock_db_[i].y);
                float dx = mouse_pos.x - s_pos.x;
                float dy = mouse_pos.y - s_pos.y;
                if (dx*dx + dy*dy < (10.0f * zoom_ * 10.0f * zoom_)) { // Radius scales with zoom
                    dragged_node = i;
                    selected_idx_ = i;
                    std::snprintf(place_name_, sizeof(place_name_), "%s", stock_db_[i].place);
                    std::snprintf(material_, sizeof(material_), "%s", stock_db_[i].mat);
                    amount_tons_ = stock_db_[i].amt;
                    break;
                }
            }
        }
        
        if (is_active && ImGui::IsMouseDragging(0) && dragged_node >= 0 && static_cast<size_t>(dragged_node) < stock_db_.size()) {
            // Apply delta in world space
            stock_db_[dragged_node].x += ImGui::GetIO().MouseDelta.x / zoom_;
            stock_db_[dragged_node].y += ImGui::GetIO().MouseDelta.y / zoom_;
            snprintf(stock_db_[dragged_node].place, sizeof(stock_db_[dragged_node].place), "LOC-%.0f-%.0f", stock_db_[dragged_node].x, stock_db_[dragged_node].y);
            std::snprintf(place_name_, sizeof(place_name_), "%s", stock_db_[dragged_node].place);
        }
        
        if (ImGui::IsMouseReleased(0)) {
            dragged_node = -1;
        }

        // Draw Stock Location Nodes
        for (size_t i = 0; i < stock_db_.size(); ++i) {
            ImVec2 node_pos = W2S(stock_db_[i].x, stock_db_[i].y);
            ImU32 halo_col = (selected_idx_ == static_cast<int>(i)) ? IM_COL32(0, 255, 255, 128) : IM_COL32(255, 255, 0, 128);
            draw_list->AddCircleFilled(node_pos, W2Sf(10.0f), halo_col);
            draw_list->AddCircleFilled(node_pos, W2Sf(6.0f), IM_COL32(255, 50, 50, 255));
            draw_list->AddCircle(node_pos, W2Sf(6.0f), IM_COL32(0, 0, 0, 255));
            
            float dx = mouse_pos.x - node_pos.x;
            float dy = mouse_pos.y - node_pos.y;
            if (dx*dx + dy*dy < (10.0f * zoom_ * 10.0f * zoom_)) {
                ImGui::PushStyleColor(ImGuiCol_PopupBg, ImVec4(0.2f, 0.2f, 0.2f, 0.95f));
                ImGui::SetTooltip("Material: %s\nAmount: %.0f Tons\nLocation: %s", stock_db_[i].mat, stock_db_[i].amt, stock_db_[i].place);
                ImGui::PopStyleColor();
            }
        }
        
        draw_list->PopClipRect(); // End map clip
        
        // --- Map View Mode Dialog ---
        if (show_dialog_) {
            ImVec2 dialog_pos = ImVec2(canvas_pos.x + 20, canvas_pos.y + canvas_size.y - 280);
            ImGui::SetNextWindowPos(dialog_pos, ImGuiCond_FirstUseEver);
            ImGui::SetNextWindowSize(ImVec2(280, 230), ImGuiCond_FirstUseEver);
            ImGui::PushStyleColor(ImGuiCol_WindowBg, is_dark ? ImVec4(0.15f, 0.15f, 0.15f, 0.95f) : ImVec4(0.9f, 0.9f, 0.9f, 0.95f)); 
            ImGui::Begin(tr("Map View Mode Dialog"), &show_dialog_, ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_AlwaysAutoResize);
            ImGui::TextDisabled(tr("Tipo de Dato"));
            ImGui::RadioButton("Places", &data_type_, 0); ImGui::RadioButton("Materiales", &data_type_, 1); ImGui::RadioButton("Análisis", &data_type_, 2);
            ImGui::SameLine();
            const char* analysis_items[] = {"Au", "Ag", "Cu", "Fe", "Ni"};
            static int analysis_idx = 0;
            ImGui::PushItemWidth(80);
            ImGui::Combo("##analysis_combo", &analysis_idx, analysis_items, IM_ARRAYSIZE(analysis_items));
            ImGui::PopItemWidth();
            ImGui::Separator();
            ImGui::TextDisabled(tr("Radius"));
            ImGui::PushItemWidth(50);
            ImGui::InputInt("Min##rmin", &radius_min_); ImGui::SameLine(); ImGui::Text(tr("-")); ImGui::SameLine(); ImGui::InputInt("Max##rmax", &radius_max_);
            ImGui::PopItemWidth();
            ImGui::Separator();
            if (ImGui::Button(tr("Cerrar"), ImVec2(70, 25))) show_dialog_ = false;
            ImGui::SameLine(0, 30);
            if (ImGui::Button(tr("Aplicar"), ImVec2(70, 25))) show_dialog_ = false;
            ImGui::SameLine();
            if (ImGui::Button(tr("Aceptar"), ImVec2(60, 25))) show_dialog_ = false;
            ImGui::End();
            ImGui::PopStyleColor();
        }
        
        // Navigation Hints Overlay
        ImGui::SetCursorPos(ImVec2(10, 10));
        ImGui::PushStyleColor(ImGuiCol_Text, col_text);
        ImGui::Text(tr("Mouse Wheel: Zoom (%.1fx) | Right-Click + Drag: Pan Terrain"), zoom_);
        ImGui::PopStyleColor();

    } else {
        // --- STOCK VIEW (with Checkboxes for Mixture) ---
        ImGui::Text(tr("Feed Mixture Calculation - Select items to mix"));
        ImGui::Separator();
        
        ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_Resizable | ImGuiTableFlags_ScrollY;
        if (ImGui::BeginTable("stock_db", 10, flags, ImVec2(-1, ImGui::GetContentRegionAvail().y - 40))) {
            ImGui::TableSetupScrollFreeze(0, 1);
            ImGui::TableSetupColumn("Mezcla"); ImGui::TableSetupColumn("Place Name"); ImGui::TableSetupColumn("Material Name");
            ImGui::TableSetupColumn("Amount Tons"); ImGui::TableSetupColumn("Fuel Value"); ImGui::TableSetupColumn("O2 Demand");
            ImGui::TableSetupColumn("Cu wt-%"); ImGui::TableSetupColumn("Fe wt-%"); ImGui::TableSetupColumn("Ni wt-%"); ImGui::TableSetupColumn("S wt-%");
            ImGui::TableHeadersRow();
            
            for (size_t i = 0; i < stock_db_.size(); ++i) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                ImGui::PushID(i);
                ImGui::Checkbox("##mix", &stock_db_[i].selected_for_mix);
                ImGui::PopID();
                
                ImGui::TableSetColumnIndex(1); 
                if (ImGui::Selectable(stock_db_[i].place, selected_idx_ == static_cast<int>(i), ImGuiSelectableFlags_SpanAllColumns)) {
                    selected_idx_ = i;
                    std::snprintf(place_name_, sizeof(place_name_), "%s", stock_db_[i].place);
                    std::snprintf(material_, sizeof(material_), "%s", stock_db_[i].mat);
                    amount_tons_ = stock_db_[i].amt;
                }
                ImGui::TableSetColumnIndex(2); ImGui::Text(tr("%s"), stock_db_[i].mat);
                ImGui::TableSetColumnIndex(3); ImGui::Text(tr("%.0f"), stock_db_[i].amt);
                ImGui::TableSetColumnIndex(4); ImGui::Text(tr("%.2f"), stock_db_[i].fv);
                ImGui::TableSetColumnIndex(5); ImGui::Text(tr("%.2f"), stock_db_[i].o2);
                ImGui::TableSetColumnIndex(6); ImGui::Text(tr("%.2f"), stock_db_[i].cu);
                ImGui::TableSetColumnIndex(7); ImGui::Text(tr("%.2f"), stock_db_[i].fe);
                ImGui::TableSetColumnIndex(8); ImGui::Text(tr("%.2f"), stock_db_[i].ni);
                ImGui::TableSetColumnIndex(9); ImGui::Text(tr("%.2f"), stock_db_[i].s);
            }
            ImGui::EndTable();
        }
        
        // Mixture Calculation Engine
        float sum_mass = 0.0f, sum_cu = 0.0f, sum_fe = 0.0f;
        for (const auto& item : stock_db_) {
            if (item.selected_for_mix) {
                sum_mass += item.amt; sum_cu += item.amt * item.cu; sum_fe += item.amt * item.fe;
            }
        }
        
        ImGui::Text(tr("Current Mix Result: Total Mass = %.0f Tons | Avg Cu = %.2f%% | Avg Fe = %.2f%%"),
                    sum_mass, sum_mass > 0 ? (sum_cu / sum_mass) : 0.0f, sum_mass > 0 ? (sum_fe / sum_mass) : 0.0f);
    }

    ImGui::EndChild(); // End Left Panel

    ImGui::SameLine();

    // --- RIGHT PANEL (Material & Analysis) ---
    ImGui::BeginChild("map_right_panel", ImVec2(0, 0), true);
    
    ImGui::TextDisabled(tr("Material"));
    ImGui::BeginChild("mat_general", ImVec2(0, 150), true);
    ImGui::TextDisabled(tr("General"));
    if (ImGui::Button(tr("Agregar..."), ImVec2(80, 25))) {
        // Spawn at center of camera view
        float center_x = (-camera_x_ + left_panel_width/2) / zoom_;
        float center_y = (-camera_y_ + ImGui::GetContentRegionAvail().y/2) / zoom_;
        stock_db_.push_back({"New Loc", "New Material", 1000, 0, 0, 0, 0, 0, 0, center_x, center_y, false});
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Transporte..."), ImVec2(90, 25))) mostrar_transporte_ = true;
    ImGui::SameLine();
    if (ImGui::Button(tr("Exportar CSV"), ImVec2(90, 25))) {
        FILE* zen = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Exportar Stock' 2>/dev/null");
        if (zen) {
            char pbuf[512] = {0};
            if (fgets(pbuf, sizeof(pbuf), zen)) {
                std::string path(pbuf);
                if (!path.empty() && path.back()=='\n') path.pop_back();
                if (path.find(".csv")==std::string::npos) path += ".csv";
                FILE* fp = fopen(path.c_str(), "w");
                if (fp) {
                    fprintf(fp, "Lugar;Material;Toneladas;FV;O2;Cu;Fe;Ni;S\n");
                    for (auto& it : stock_db_)
                        fprintf(fp, "%s;%s;%.0f;%.2f;%.2f;%.2f;%.2f;%.2f;%.2f\n",
                                it.place, it.mat, it.amt, it.fv, it.o2, it.cu, it.fe, it.ni, it.s);
                    fclose(fp);
                }
            }
            fclose(zen);
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Save Data"), ImVec2(80, 25))) {
        if (selected_idx_ >= 0 && static_cast<size_t>(selected_idx_) < stock_db_.size()) {
            std::snprintf(stock_db_[selected_idx_].place, sizeof(stock_db_[selected_idx_].place), "%s", place_name_);
            std::snprintf(stock_db_[selected_idx_].mat, sizeof(stock_db_[selected_idx_].mat), "%s", material_);
            stock_db_[selected_idx_].amt = amount_tons_;
        }
    }
    ImGui::Separator();
    
    ImGui::PushItemWidth(-FLT_MIN);
    ImGui::Text(tr("Place Name ")); ImGui::SameLine(90); ImGui::InputText("##pname", place_name_, sizeof(place_name_));
    ImGui::Text(tr("Material   ")); ImGui::SameLine(90); ImGui::InputText("##mat", material_, sizeof(material_));
    ImGui::Text(tr("Cantidad     ")); ImGui::SameLine(90); ImGui::InputInt("##amt", &amount_tons_);
    ImGui::PopItemWidth();
    ImGui::EndChild();
    
    ImGui::BeginChild("mat_analysis", ImVec2(0, 0), true);
    ImGui::TextDisabled(tr("Analysis (Editable)"));
    ImGui::Separator();
    
    ImGuiTableFlags analysis_flags = ImGuiTableFlags_BordersInnerH | ImGuiTableFlags_RowBg | ImGuiTableFlags_SizingStretchProp;
    if (ImGui::BeginTable("analysis_table", 3, analysis_flags)) {
        ImGui::TableSetupColumn("Prop", ImGuiTableColumnFlags_WidthStretch, 0.4f);
        ImGui::TableSetupColumn("Val", ImGuiTableColumnFlags_WidthStretch, 0.4f);
        ImGui::TableSetupColumn("Unidad", ImGuiTableColumnFlags_WidthStretch, 0.2f);
        
        
        if (selected_idx_ >= 0 && static_cast<size_t>(selected_idx_) < stock_db_.size()) {
            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Fuel Value")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); ImGui::InputFloat("##fv", &stock_db_[selected_idx_].fv, 0.0f, 0.0f, "%.2f"); ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("kW/ht"));
            
            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("O2 Demand")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); ImGui::InputFloat("##o2", &stock_db_[selected_idx_].o2, 0.0f, 0.0f, "%.2f"); ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("Nm3/t"));

            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Cu")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); ImGui::InputFloat("##cu", &stock_db_[selected_idx_].cu, 0.0f, 0.0f, "%.2f"); ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("wt-%%"));

            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Fe")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); ImGui::InputFloat("##fe", &stock_db_[selected_idx_].fe, 0.0f, 0.0f, "%.2f"); ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("wt-%%"));

            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Ni")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); ImGui::InputFloat("##ni", &stock_db_[selected_idx_].ni, 0.0f, 0.0f, "%.2f"); ImGui::PopItemWidth();
            ImGui::TableSetColumnIndex(2); ImGui::Text(tr("wt-%%"));

            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("S")); ImGui::TableSetColumnIndex(1);
            ImGui::PushItemWidth(-1); ImGui::InputFloat("##s", &stock_db_[selected_idx_].s, 0.0f, 0.0f, "%.2f"); ImGui::PopItemWidth();
            // Auto-save logic removed
        } else {
            ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::Text(tr("Select a material"));
        }
        
        ImGui::EndTable();
    }
    
    ImGui::EndChild();
    ImGui::EndChild(); // End Right Panel

    // ── Popup: Transporte de Material ──
    if (mostrar_transporte_) {
        ImGui::OpenPopup(tr("Transporte de Material"));
        if (ImGui::BeginPopupModal(tr("Transporte de Material"), &mostrar_transporte_, ImGuiWindowFlags_AlwaysAutoResize)) {
            if (selected_idx_ >= 0 && selected_idx_ < (int)stock_db_.size()) {
                auto& it = stock_db_[selected_idx_];
                ImGui::Text(tr("Material: %s"), it.mat);
                ImGui::Text(tr("Desde: %s"), it.place);
                static char nuevo_lugar[64] = "";
                static int mover_t = 0;
                ImGui::InputText(tr("Destino"), nuevo_lugar, sizeof(nuevo_lugar));
                ImGui::InputInt(tr("Cantidad (t)"), &mover_t);
                if (ImGui::Button(tr("Transportar"), ImVec2(130, 0))) {
                    if (mover_t > 0 && mover_t <= (int)it.amt) {
                        it.amt -= mover_t;
                        for (auto& otro : stock_db_)
                            if (strcmp(otro.place, nuevo_lugar) == 0 && strcmp(otro.mat, it.mat) == 0)
                                otro.amt += mover_t;
                        mostrar_transporte_ = false;
                        ImGui::CloseCurrentPopup();
                    }
                }
            } else {
                ImGui::Text(tr("Seleccione un stock primero."));
                if (ImGui::Button(tr("Cerrar"))) { mostrar_transporte_ = false; ImGui::CloseCurrentPopup(); }
            }
            ImGui::EndPopup();
        }
    }
}

#include <cmath>
#include <random>
