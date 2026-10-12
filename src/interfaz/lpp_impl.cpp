#include "utils/i18n.h"
struct Vec2d { double x, y; };
static std::vector<Vec2d> ClipPolygon(const std::vector<Vec2d>& poly, double A, double B, double C_const) {
    if (poly.empty()) return poly;
    std::vector<Vec2d> res;
    auto dist = [=](const Vec2d& pt) { return A * pt.x + B * pt.y + C_const; };
    for (size_t i = 0; i < poly.size(); ++i) {
        Vec2d cur = poly[i];
        Vec2d prev = poly[(i == 0) ? poly.size() - 1 : i - 1];
        double d_cur = dist(cur);
        double d_prev = dist(prev);
        double eps = 1e-12;
        if (d_cur <= eps) {
            if (d_prev > eps) {
                double t = d_prev / (d_prev - d_cur);
                res.push_back({prev.x + t * (cur.x - prev.x), prev.y + t * (cur.y - prev.y)});
            }
            res.push_back(cur);
        } else {
            if (d_prev <= eps) {
                double t = d_prev / (d_prev - d_cur);
                res.push_back({prev.x + t * (cur.x - prev.x), prev.y + t * (cur.y - prev.y)});
            }
        }
    }
    return res;
}

void PanelDiagramasLpp::recalcular() {
    poligonos_.clear();
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    
    auto parser = nucleo::TablaPeriodica::instancia();
    auto main_atoms = parser.analizar_formula(main_element_);
    auto g1_atoms = parser.analizar_formula(gas1_element_);
    auto g2_atoms = parser.analizar_formula(gas2_element_);
    
    std::string m_el = main_atoms.empty() ? main_element_ : main_atoms[0].elem;
    
    double g1_factor = g1_atoms.empty() ? 1.0 : g1_atoms[0].cuenta;
    if (g1_factor == 0) g1_factor = 1.0;
    
    double g2_factor = g2_atoms.empty() ? 1.0 : g2_atoms[0].cuenta;
    if (g2_factor == 0) g2_factor = 1.0;
    
    std::string g1_el = g1_atoms.empty() ? "" : g1_atoms[0].elem;
    std::string g2_el = g2_atoms.empty() ? "" : g2_atoms[0].elem;

    const nucleo::Especie* esp_g1 = nullptr;
    const nucleo::Especie* esp_g2 = nullptr;
    for (const auto* e : db_especies) {
        if (e->formula == gas1_element_ && e->fase_std == nucleo::Fase::Gas) esp_g1 = e;
        if (e->formula == gas2_element_ && e->fase_std == nucleo::Fase::Gas) esp_g2 = e;
    }
    if (!esp_g1) { for (const auto* e : db_especies) if (e->formula == gas1_element_) { esp_g1 = e; break; } }
    if (!esp_g2) { for (const auto* e : db_especies) if (e->formula == gas2_element_) { esp_g2 = e; break; } }
    
    if (!esp_g1 || !esp_g2) {
        if (!esp_g1) ImGui::TextColored(ImVec4(1.0f, 0.4f, 0.0f, 1.0f), "⚠ Advertencia: Gas 1 (%s) no encontrado. Asumiendo gas ideal estándar.", gas1_element_);
        if (!esp_g2) ImGui::TextColored(ImVec4(1.0f, 0.4f, 0.0f, 1.0f), "⚠ Advertencia: Gas 2 (%s) no encontrado. Asumiendo gas ideal estándar.", gas2_element_);
    }

    struct SpeciePlane {
        int64_t id;
        std::string formula;
        ImU32 color;
        double Z, A, B; 
    };
    std::vector<SpeciePlane> planes;
    
    double mu0_g1 = esp_g1 ? esp_g1->energia_gibbs(T_) : 0.0;
    double mu0_g2 = esp_g2 ? esp_g2->energia_gibbs(T_) : 0.0;
    double R_kJ = 0.008314462618;
    double log_conv = 2.302585093 * R_kJ * T_;
    
    for (auto& par : especies_seleccionadas_) {
        if (!par.second) continue;
        const nucleo::Especie* esp = nullptr;
        for (const auto* e : db_especies) {
            if (e->id == par.first) { esp = e; break; }
        }
        if (!esp) continue;
        
        auto atoms = parser.analizar_formula(esp->formula);
        double nm = 0, ng1 = 0, ng2 = 0;
        for (const auto& at : atoms) {
            if (at.elem == m_el) nm += at.cuenta;
            else if (at.elem == g1_el) ng1 += at.cuenta;
            else if (at.elem == g2_el) ng2 += at.cuenta;
        }
        if (nm <= 0) continue;
        
        double Gf = esp->energia_gibbs(T_);
        SpeciePlane p;
        p.id = esp->id;
        p.formula = esp->formula;
        p.color = ImGui::ColorConvertFloat4ToU32(color_from_id(esp->id));
        
        p.Z = (Gf - (ng1 / g1_factor) * mu0_g1 - (ng2 / g2_factor) * mu0_g2) / nm;
        p.A = - (ng1 / g1_factor) * log_conv / nm;
        p.B = - (ng2 / g2_factor) * log_conv / nm;
        
        planes.push_back(p);
    }
    
    std::vector<Vec2d> bbox = {
        {x_min_ - 10, y_min_ - 10}, {x_max_ + 10, y_min_ - 10},
        {x_max_ + 10, y_max_ + 10}, {x_min_ - 10, y_max_ + 10}
    };
    
    for (size_t i = 0; i < planes.size(); ++i) {
        auto poly = bbox;
        const auto& pi = planes[i];
        for (size_t j = 0; j < planes.size(); ++j) {
            if (i == j) continue;
            const auto& pj = planes[j];
            poly = ClipPolygon(poly, pi.A - pj.A, pi.B - pj.B, pi.Z - pj.Z);
            if (poly.empty()) break;
        }
        
        if (!poly.empty()) {
            PoligonoFase pf;
            pf.id = pi.id;
            pf.formula = pi.formula;
            pf.color = pi.color;
            double cx = 0, cy = 0;
            double area = 0;
            for (size_t k = 0; k < poly.size(); ++k) {
                pf.vertices.push_back(ImVec2((float)poly[k].x, (float)poly[k].y));
                size_t next = (k + 1) % poly.size();
                double a = poly[k].x * poly[next].y - poly[next].x * poly[k].y;
                area += a;
                cx += (poly[k].x + poly[next].x) * a;
                cy += (poly[k].y + poly[next].y) * a;
            }
            if (std::abs(area) > 1e-9) {
                area *= 0.5;
                pf.centroide = ImVec2((float)(cx / (6.0 * area)), (float)(cy / (6.0 * area)));
            } else {
                pf.centroide = pf.vertices[0];
            }
            poligonos_.push_back(pf);
        }
    }
    calculado_ = true;
}

void PanelDiagramasLpp::renderizar() {
    auto db_especies = base_datos::GestorDB::instancia().todas_especies();
    
    ImGui::Columns(2, "LppLayout", true);
    ImGui::SetColumnWidth(0, 320.0f);
    
    ImGui::Text(tr("Diagram Parameters (Lpp)"));
    ImGui::Separator();
    
    ImGui::InputText(tr("Main Element"), main_element_, sizeof(main_element_));
    ImGui::InputText(tr("Gas 1 (X axis)"), gas1_element_, sizeof(gas1_element_));
    ImGui::InputText(tr("Gas 2 (Y axis)"), gas2_element_, sizeof(gas2_element_));
    
    ImGui::Spacing();
    ImGui::Text(tr("Filtrar Especies por Elementos"));
    ImGui::BeginChild("SpeciesListLpp", ImVec2(0, 200), true);
    if (ImGui::BeginTable("LppSpeciesTable", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("Select", ImGuiTableColumnFlags_WidthFixed, 40.0f);
        ImGui::TableSetupColumn("Formula", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableHeadersRow();
        
        auto parser = nucleo::TablaPeriodica::instancia();
        auto main_atoms = parser.analizar_formula(main_element_);
        auto g1_atoms = parser.analizar_formula(gas1_element_);
        auto g2_atoms = parser.analizar_formula(gas2_element_);
        
        std::set<std::string> valid_elements;
        for (const auto& at : main_atoms) valid_elements.insert(at.elem);
        for (const auto& at : g1_atoms) valid_elements.insert(at.elem);
        for (const auto& at : g2_atoms) valid_elements.insert(at.elem);
        
        for (const auto* esp : db_especies) {
            auto atoms = parser.analizar_formula(esp->formula);
            bool has_other = false;
            for (const auto& at : atoms) {
                if (valid_elements.find(at.elem) == valid_elements.end()) has_other = true;
            }
            if (!atoms.empty() && !has_other) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                bool selected = especies_seleccionadas_[esp->id];
                if (ImGui::Checkbox((std::string("##sel") + std::to_string(esp->id)).c_str(), &selected)) {
                    especies_seleccionadas_[esp->id] = selected;
                }
                ImGui::TableSetColumnIndex(1);
                ImGui::Text(tr("%s"), esp->formula.c_str());
            }
        }
        ImGui::EndTable();
    }
    ImGui::EndChild();
    
    ImGui::Spacing();
    ImGui::Text(tr("Axes Setup (Isothermal)"));
    bool changed = false;
    changed |= ImGui::InputDouble("Temperature (K)", &T_);
    changed |= ImGui::InputDouble("X Min", &x_min_);
    changed |= ImGui::InputDouble("X Max", &x_max_);
    changed |= ImGui::InputDouble("Y Min", &y_min_);
    changed |= ImGui::InputDouble("Y Max", &y_max_);
    
    if (ImGui::Button(tr("Dibujar Diagrama"), ImVec2(-1,30))) {
        recalcular();
    }
    
    ImGui::NextColumn();
    
    if (calculado_) {
        std::string title = std::string("Phase Stability Diagram (") + main_element_ + "-" + gas1_element_ + "-" + gas2_element_ + ")";
        if (ImPlot::BeginPlot(title.c_str(), ImVec2(-1, -1))) {
            char xlabel[64], ylabel[64];
            snprintf(xlabel, sizeof(xlabel), "log p(%s)", gas1_element_);
            snprintf(ylabel, sizeof(ylabel), "log p(%s)", gas2_element_);
            ImPlot::SetupAxes(xlabel, ylabel);
            ImPlot::SetupAxisLimits(ImAxis_X1, x_min_, x_max_);
            ImPlot::SetupAxisLimits(ImAxis_Y1, y_min_, y_max_);
            
            for (auto& pf : poligonos_) {
                if (pf.vertices.empty()) continue;
                std::vector<float> xs, ys;
                for (auto v : pf.vertices) { xs.push_back(v.x); ys.push_back(v.y); }
                
                ImPlot::SetNextLineStyle(ImVec4(0,0,0,1), 1.5f); // Black border
                // We add the first vertex to the end to close the line loop
                xs.push_back(xs[0]);
                ys.push_back(ys[0]);
                ImPlot::PlotLine(pf.formula.c_str(), xs.data(), ys.data(), (int)xs.size());
                
                // Fill using ImGui draw list for true polygons
                ImDrawList* draw_list = ImPlot::GetPlotDrawList();
                std::vector<ImVec2> screen_pts;
                for (size_t k = 0; k < pf.vertices.size(); ++k) {
                    screen_pts.push_back(ImPlot::PlotToPixels(pf.vertices[k].x, pf.vertices[k].y));
                }
                draw_list->AddConvexPolyFilled(screen_pts.data(), (int)screen_pts.size(), pf.color);
                
                ImPlot::PushStyleColor(ImPlotCol_InlayText, ImVec4(0,0,0,1));
                ImPlot::PlotText(pf.formula.c_str(), pf.centroide.x, pf.centroide.y);
                ImPlot::PopStyleColor();
            }
            
            ImPlot::EndPlot();
        }
    } else {
        ImGui::TextColored(ImVec4(0.5,0.5,0.5,1), "Select species and click Draw Diagram.");
    }
    ImGui::Columns(1);
}
