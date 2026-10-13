#include "shared.hpp"
#include <filesystem>
#include <chrono>
namespace cma { namespace interfaz {
// Panel de Método Benson
// ═══════════════════════════════════════════════════════════════════════════
static std::vector<Vec3> generar_tetraedro(Vec3 in_bond) {
    std::vector<Vec3> res;
    Vec3 u = in_bond.normalize();
    Vec3 w = {1,0,0};
    if (std::abs(u.x) > 0.9f) w = {0,1,0};
    w = (w - u * u.dot(w)).normalize(); 
    
    // Alternate orientation to encourage zigzag chains
    static bool flip = false;
    flip = !flip;
    if (flip) w = w * -1.0f;
    
    float c120 = -0.5f;
    float s120 = std::sqrt(3.0f)/2.0f;
    Vec3 cross = u.cross(w);
    
    Vec3 w1 = w;
    Vec3 w2 = w * c120 + cross * s120;
    Vec3 w3 = w * c120 - cross * s120;
    
    float ct = -1.0f/3.0f;
    float st = std::sqrt(8.0f/9.0f);
    
    res.push_back(u * ct + w1 * st);
    res.push_back(u * ct + w2 * st);
    res.push_back(u * ct + w3 * st);
    return res;
}

static std::vector<Vec3> generar_trigonal(Vec3 in_bond) {
    std::vector<Vec3> res;
    Vec3 u = in_bond.normalize();
    Vec3 w = {1,0,0};
    if (std::abs(u.x) > 0.9f) w = {0,1,0};
    w = (w - u * u.dot(w)).normalize(); 
    
    float s120 = std::sqrt(3.0f)/2.0f;
    
    res.push_back(u * -0.5f + w * s120);
    res.push_back(u * -0.5f - w * s120);
    return res;
}

void PanelBenson::agregar_grupo(const std::string& nombre, const std::string& elemento_central) {
    float bond_len = 1.54f;
    if (nombre[0] == '=') bond_len = 1.34f;
    if (nombre == "=O") bond_len = 1.22f;
    
    if (nombre == "Benzene") {
        // Special case for Benzene ring
        Vec3 pos = {0,0,0};
        Vec3 in_bond = {-1,0,0};
        int p_idx = -1;
        
        for (int i = 0; i < static_cast<int>(atomos_.size()); ++i) {
            if (atomos_[i].elemento != "H" && !atomos_[i].valencias.empty()) {
                p_idx = i;
                in_bond = atomos_[i].valencias.back();
                atomos_[i].valencias.pop_back();
                pos = {atomos_[i].x, atomos_[i].y, atomos_[i].z};
                pos = pos + in_bond * bond_len;
                break;
            }
        }
        
        Vec3 u = in_bond.normalize();
        Vec3 w = {1,0,0};
        if (std::abs(u.x) > 0.9f) w = {0,1,0};
        w = (w - u * u.dot(w)).normalize(); 
        
        // Generate hexagon points
        int base_idx = atomos_.size();
        for(int i=0; i<6; i++) {
            float ang = i * 60.0f * 3.14159f / 180.0f;
            // First atom at u
            Vec3 dir = u * std::cos(ang) + w * std::sin(ang);
            Vec3 p = pos + dir * 1.4f - u * 1.4f; // shift so atom 0 is at pos
            
            Atomo3D a;
            a.x = p.x; a.y = p.y; a.z = p.z;
            a.elemento = "C";
            // Valence pointing outward
            a.valencias.push_back(dir);
            atomos_.push_back(a);
            
            if (i > 0) enlaces_.push_back(Enlace3D{base_idx + i - 1, base_idx + i});
        }
        enlaces_.push_back(Enlace3D{base_idx + 5, base_idx});
        if (p_idx >= 0) enlaces_.push_back(Enlace3D{p_idx, base_idx});
        
        // Add hydrogens to the ring where available
        for(int i=0; i<6; i++) {
            if (i == 0 && p_idx >= 0) continue; // C0 is attached to parent, no H!
            
            Vec3 val = atomos_[base_idx + i].valencias.back();
            atomos_[base_idx + i].valencias.pop_back();
            Vec3 hp = {atomos_[base_idx + i].x + val.x * 1.09f, 
                       atomos_[base_idx + i].y + val.y * 1.09f, 
                       atomos_[base_idx + i].z + val.z * 1.09f};
            
            Atomo3D ha = {hp.x, hp.y, hp.z, "H", {}};
            atomos_.push_back(ha);
            enlaces_.push_back(Enlace3D{base_idx + i, (int)atomos_.size()-1});
        }
        
        recalcular();
        return;
    }

    Vec3 pos = {0,0,0};
    Vec3 in_bond = {-1,0,0}; // default incoming bond for first atom
    int p_idx = -1;
    int h_to_remove = -1;
    
    // Find parent
    for (int i = 0; i < static_cast<int>(atomos_.size()); ++i) {
        if (atomos_[i].elemento != "H" && atomos_[i].elemento != "DELETED" && !atomos_[i].valencias.empty()) {
            p_idx = i;
            in_bond = atomos_[i].valencias.back();
            atomos_[i].valencias.pop_back();
            pos = {atomos_[i].x, atomos_[i].y, atomos_[i].z};
            pos = pos + in_bond * bond_len;
            break;
        }
    }
    
    if (p_idx == -1 && !atomos_.empty()) {
        // Try to replace an H
        for (int i = 0; i < static_cast<int>(atomos_.size()); ++i) {
            if (atomos_[i].elemento != "H" && atomos_[i].elemento != "DELETED") {
                for (int j = 0; j < static_cast<int>(enlaces_.size()); ++j) {
                    int a1 = enlaces_[j].atomo1;
                    int a2 = enlaces_[j].atomo2;
                    if ((a1 == i && atomos_[a2].elemento == "H") || 
                        (a2 == i && atomos_[a1].elemento == "H")) {
                        h_to_remove = (a1 == i) ? a2 : a1;
                        p_idx = i;
                        
                        Vec3 cx = {atomos_[i].x, atomos_[i].y, atomos_[i].z};
                        Vec3 hx = {atomos_[h_to_remove].x, atomos_[h_to_remove].y, atomos_[h_to_remove].z};
                        in_bond = (hx - cx).normalize();
                        pos = cx + in_bond * bond_len;
                        break;
                    }
                }
                if (p_idx != -1) break;
            }
        }
        
        if (h_to_remove != -1) {
            atomos_[h_to_remove].elemento = "DELETED";
            for (auto it = enlaces_.begin(); it != enlaces_.end(); ) {
                if ((it->atomo1 == p_idx && it->atomo2 == h_to_remove) || 
                    (it->atomo2 == p_idx && it->atomo1 == h_to_remove)) {
                    it = enlaces_.erase(it);
                } else {
                    ++it;
                }
            }
        } else {
            // Force attach to last C
            p_idx = atomos_.size() - 1;
            pos = {atomos_[p_idx].x + 1.5f, atomos_[p_idx].y, atomos_[p_idx].z};
        }
    }

    Atomo3D nuevo;
    nuevo.x = pos.x; nuevo.y = pos.y; nuevo.z = pos.z;
    nuevo.elemento = elemento_central;
    
    if (atomos_.empty()) {
        nuevo.valencias = {
            Vec3{0,1,0}, 
            Vec3{0.816f,-0.333f,-0.471f}, 
            Vec3{-0.816f,-0.333f,-0.471f}, 
            Vec3{0,-0.333f,0.942f}
        };
    } else {
        if (nombre.find("=") != std::string::npos || nombre == "-COOH") {
            nuevo.valencias = generar_trigonal(in_bond * -1.0f);
        } else {
            nuevo.valencias = generar_tetraedro(in_bond * -1.0f);
        }
    }
    
    int new_idx = atomos_.size();
    atomos_.push_back(nuevo);
    if (p_idx >= 0) {
        enlaces_.push_back(Enlace3D{p_idx, new_idx});
        if (nombre[0] == '=' || nombre == "=O") {
            enlaces_.push_back(Enlace3D{p_idx, new_idx});
        }
    }
    
    // Add hydrogens based on name
    int h_count = 0;
    if (nombre == "-CH3") h_count = 3;
    else if (nombre == "-CH2-" || nombre == "=CH2" || nombre == "-NH2") h_count = 2;
    else if (nombre == ">CH-" || nombre == "=CH-" || nombre == "-OH") h_count = 1;
    
    for (int i = 0; i < h_count && !atomos_[new_idx].valencias.empty(); ++i) {
        Vec3 v = atomos_[new_idx].valencias.back();
        atomos_[new_idx].valencias.pop_back();
        Vec3 hp = pos + v * 1.09f;
        Atomo3D h_at = {hp.x, hp.y, hp.z, "H", {}};
        atomos_.push_back(h_at);
        enlaces_.push_back(Enlace3D{new_idx, (int)atomos_.size()-1});
    }
    
    // Adjust open valences based on typical element bonds to prevent incorrect branching
    if (elemento_central == "O" || elemento_central == "S") {
        while (atomos_[new_idx].valencias.size() > 1) {
            atomos_[new_idx].valencias.pop_back();
        }
    } else if (elemento_central == "N") {
        while (atomos_[new_idx].valencias.size() > 2) {
            atomos_[new_idx].valencias.pop_back();
        }
    }
    
    // Clear valences for terminal functional groups so they don't act as attachment points
    if (nombre == "-OH" || nombre == "-NH2" || nombre == "=O" || 
        nombre == "-F" || nombre == "-Cl" || nombre == "-Br" || nombre == "-I" ||
        elemento_central == "Cl" || elemento_central == "Br" || elemento_central == "F" || elemento_central == "I") {
        atomos_[new_idx].valencias.clear();
    }
    
    if (nombre == "-COOH") {
        // Add double bond O
        Vec3 v1 = atomos_[new_idx].valencias.back();
        atomos_[new_idx].valencias.pop_back();
        Vec3 o1p = pos + v1 * 1.22f;
        int o1 = atomos_.size();
        atomos_.push_back({o1p.x, o1p.y, o1p.z, "O", {}});
        enlaces_.push_back(Enlace3D{new_idx, o1});
        enlaces_.push_back(Enlace3D{new_idx, o1}); // visually draw double bond?
        
        // Add single bond -OH
        Vec3 v2 = atomos_[new_idx].valencias.back();
        atomos_[new_idx].valencias.pop_back();
        Vec3 o2p = pos + v2 * 1.4f;
        int o2 = atomos_.size();
        
        // O2 needs valences for its H
        std::vector<Vec3> oval = generar_tetraedro(v2 * -1.0f);
        atomos_.push_back({o2p.x, o2p.y, o2p.z, "O", oval});
        enlaces_.push_back(Enlace3D{new_idx, o2});
        
        Vec3 hv = atomos_[o2].valencias.back();
        atomos_[o2].valencias.pop_back();
        Vec3 hp = o2p + hv * 0.96f;
        atomos_.push_back({hp.x, hp.y, hp.z, "H", {}});
        enlaces_.push_back(Enlace3D{o2, (int)atomos_.size()-1});
        
        atomos_[o2].valencias.clear(); // O in -OH is terminal
    }
    
    recalcular();
}

void PanelBenson::deshacer() {
    atomos_.clear();
    enlaces_.clear();
    recalcular();
}

void PanelBenson::recalcular() {
    mw_ = 0.0;
    h_est_ = 0.0;
    s_est_ = 0.0;
    cp_est_ = 0.0;
    
    std::map<std::string, int> counts;
    for (const auto& a : atomos_) {
        if (a.elemento != "DELETED") {
            counts[a.elemento]++;
        }
    }
    
    mw_ += counts["C"] * 12.011;
    mw_ += counts["H"] * 1.008;
    mw_ += counts["O"] * 15.999;
    mw_ += counts["N"] * 14.007;
    mw_ += counts["S"] * 32.06;
    mw_ += counts["Cl"] * 35.45;
    
    std::string formula_str = "";
    if (counts["C"] > 0) formula_str += "C" + std::to_string(counts["C"]);
    if (counts["H"] > 0) formula_str += "H" + std::to_string(counts["H"]);
    if (counts["O"] > 0) formula_str += "O" + std::to_string(counts["O"]);
    if (counts["N"] > 0) formula_str += "N" + std::to_string(counts["N"]);
    if (counts["Cl"] > 0) formula_str += "Cl" + std::to_string(counts["Cl"]);
    if (counts["S"] > 0) formula_str += "S" + std::to_string(counts["S"]);
    
    snprintf(formula_, sizeof(formula_), "%s", formula_str.c_str());
    
    // Molecular geometry alone does not provide validated Benson group contributions.
}

void PanelBenson::renderizar() {
    if (ImGui::Button(tr("Limpiar molécula"))) deshacer();
    if (ImGui::Button(tr("Calcular"))) recalcular();
    ImGui::SameLine();
    if (ImGui::Button(tr("Save Image"))) {
        adapters::DesktopFileDialog dialog;
        const auto output = dialog.select("Guardar molécula", "PNG | *.png", true);
        const auto directory = std::filesystem::temp_directory_path() / ("cma-molecule-" +
            std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
        std::error_code temp_error;
        const bool created = !output.empty() && std::filesystem::create_directory(directory, temp_error);
        const auto input = directory / "molecule.xyz";
        FILE* f = created ? fopen(input.string().c_str(), "w") : nullptr;
        if (f) {
            int valid_atoms = 0;
            for (const auto& a : atomos_) if (a.elemento != "DELETED") valid_atoms++;
            fprintf(f, "%d\nBenson Molecule\n", valid_atoms);
            for (const auto& a : atomos_) {
                if (a.elemento != "DELETED") fprintf(f, "%s %f %f %f\n", a.elemento.c_str(), a.x, a.y, a.z);
            }
            fclose(f);
            if (!adapters::render_molecule(adapters::resource_path("tools/rendering/cma_benson_render.py"), input.string(), output))
                ImGui::OpenPopup("Error exportando molécula");
        }
        if (created) std::filesystem::remove_all(directory, temp_error);
    }
    if (ImGui::BeginPopup("Error exportando molécula")) {
        ImGui::TextWrapped("No se pudo exportar la imagen. Requiere Python, NumPy y Matplotlib.");
        ImGui::EndPopup();
    }
    ImGui::SameLine();
    ImGui::SameLine();
    ImGui::PushItemWidth(150);
    ImGui::SliderFloat("Atom size", &atom_size_, 0.1f, 3.0f);
    ImGui::PopItemWidth();
    ImGui::Separator();
    
    ImGui::Columns(2, "benson_cols");
    ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() - 250);
    
    ImPlot3DFlags plot_flags = ImPlot3DFlags_NoLegend | ImPlot3DFlags_NoTitle;
    if (ImPlot3D::BeginPlot("##molecule_3d", ImVec2(-1, -1), plot_flags)) {
        ImPlot3DAxisFlags axis_flags = ImPlot3DAxisFlags_NoTickLabels | ImPlot3DAxisFlags_NoGridLines | ImPlot3DAxisFlags_NoTickMarks | ImPlot3DAxisFlags_NoLabel;
        ImPlot3D::SetupAxes("", "", "", axis_flags, axis_flags, axis_flags);
        
        ImPlot3DSpec line_spec;
        line_spec.LineWeight = 5.0f;
        line_spec.LineColor = ImVec4(0.5f, 0.5f, 0.5f, opacity_);
        
        std::map<std::pair<int,int>, int> bond_counts;
        for (const auto& bond : enlaces_) {
            if (bond.atomo1 < static_cast<int>(atomos_.size()) && bond.atomo2 < static_cast<int>(atomos_.size()) &&
                atomos_[bond.atomo1].elemento != "DELETED" && atomos_[bond.atomo2].elemento != "DELETED") {
                int a1 = std::min(bond.atomo1, bond.atomo2);
                int a2 = std::max(bond.atomo1, bond.atomo2);
                bond_counts[{a1, a2}]++;
            }
        }
        
        for (const auto& [pair, count] : bond_counts) {
            int a1 = pair.first;
            int a2 = pair.second;
            
            Vec3 p1 = {atomos_[a1].x, atomos_[a1].y, atomos_[a1].z};
            Vec3 p2 = {atomos_[a2].x, atomos_[a2].y, atomos_[a2].z};
            Vec3 dir = (p2 - p1).normalize();
            
            Vec3 up = {0,1,0};
            if (std::abs(dir.y) > 0.9f) up = {1,0,0};
            Vec3 right = dir.cross(up).normalize() * 0.18f; // offset for multiple bonds
            
            for (int k = 0; k < count; ++k) {
                Vec3 offset = {0,0,0};
                if (count == 2) {
                    offset = (k == 0) ? right : right * -1.0f;
                } else if (count == 3) {
                    if (k == 0) offset = right;
                    else if (k == 1) offset = right * -1.0f;
                }
                
                float xs[2] = {p1.x + offset.x, p2.x + offset.x};
                float ys[2] = {p1.y + offset.y, p2.y + offset.y};
                float zs[2] = {p1.z + offset.z, p2.z + offset.z};
                
                ImPlot3D::PlotLine("##bond", xs, ys, zs, 2, line_spec);
            }
        }
        
        std::map<std::string, std::vector<float>> px, py, pz;
        for (const auto& a : atomos_) {
            if (a.elemento == "DELETED") continue;
            px[a.elemento].push_back(a.x);
            py[a.elemento].push_back(a.y);
            pz[a.elemento].push_back(a.z);
        }
        
        for (auto& [elem, xs] : px) {
            ImVec4 col = ImVec4(0.5f, 0.5f, 0.5f, opacity_);
            float size = atom_size_ * 10.0f;
            if (elem == "C") col = ImVec4(0.2f, 0.2f, 0.2f, opacity_);
            else if (elem == "H") { col = ImVec4(0.9f, 0.9f, 0.9f, opacity_); size *= 0.6f; }
            else if (elem == "O") col = ImVec4(0.9f, 0.1f, 0.1f, opacity_);
            else if (elem == "N") col = ImVec4(0.1f, 0.1f, 0.9f, opacity_);
            else if (elem == "Cl") { col = ImVec4(0.1f, 0.9f, 0.1f, opacity_); size *= 1.2f; }
            else if (elem == "S") col = ImVec4(0.9f, 0.9f, 0.1f, opacity_);
            
            ImPlot3DSpec scatter_spec;
            scatter_spec.Marker = ImPlot3DMarker_Circle;
            scatter_spec.MarkerSize = size;
            scatter_spec.MarkerFillColor = col;
            scatter_spec.MarkerLineColor = col;
            
            ImPlot3D::PlotScatter(elem.c_str(), xs.data(), py[elem].data(), pz[elem].data(), xs.size(), scatter_spec);
        }
        
        ImPlot3D::EndPlot();
    }
    
    ImGui::NextColumn();
    ImGui::Text(tr("Please select"));
    
    if (ImGui::Button(tr("Skeletal groups"), ImVec2(-1, 0))) ImGui::OpenPopup("SkeletalPopup");
    if (ImGui::BeginPopup("SkeletalPopup")) {
        if (ImGui::Selectable(tr("-CH3"))) agregar_grupo("-CH3", "C");
        if (ImGui::Selectable(tr("-CH2-"))) agregar_grupo("-CH2-", "C");
        if (ImGui::Selectable(tr(">CH-"))) agregar_grupo(">CH-", "C");
        if (ImGui::Selectable(tr(">C<"))) agregar_grupo(">C<", "C");
        if (ImGui::Selectable(tr("=CH2"))) agregar_grupo("=CH2", "C");
        if (ImGui::Selectable(tr("=CH-"))) agregar_grupo("=CH-", "C");
        if (ImGui::Selectable(tr("Benzene ring"))) agregar_grupo("Benzene", "C");
        ImGui::EndPopup();
    }
    
    if (ImGui::Button(tr("Functional groups"), ImVec2(-1, 0))) ImGui::OpenPopup("FunctionalPopup");
    if (ImGui::BeginPopup("FunctionalPopup")) {
        if (ImGui::Selectable(tr("-OH"))) agregar_grupo("-OH", "O");
        if (ImGui::Selectable(tr("-COOH"))) agregar_grupo("-COOH", "C");
        if (ImGui::Selectable(tr("-NH2"))) agregar_grupo("-NH2", "N");
        if (ImGui::Selectable(tr("=O"))) agregar_grupo("=O", "O");
        if (ImGui::Selectable(tr("-F"))) agregar_grupo("-F", "F");
        if (ImGui::Selectable(tr("-Cl"))) agregar_grupo("-Cl", "Cl");
        if (ImGui::Selectable(tr("-Br"))) agregar_grupo("-Br", "Br");
        if (ImGui::Selectable(tr("-I"))) agregar_grupo("-I", "I");
        ImGui::EndPopup();
    }
    
    ImGui::Separator();
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.2f, 0.2f, 0.2f, 1.0f));
    if (ImGui::Button(tr(" C "), ImVec2(30, 30))) agregar_grupo("C", "C");
    ImGui::SameLine();
    ImGui::PopStyleColor();
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.9f, 0.9f, 0.9f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.0f, 0.0f, 0.0f, 1.0f));
    if (ImGui::Button(tr(" H "), ImVec2(30, 30))) agregar_grupo("H", "H");
    ImGui::SameLine();
    ImGui::PopStyleColor(2);
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.1f, 0.1f, 0.9f, 1.0f));
    if (ImGui::Button(tr(" N "), ImVec2(30, 30))) agregar_grupo("N", "N");
    ImGui::SameLine();
    ImGui::PopStyleColor();
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.9f, 0.1f, 0.1f, 1.0f));
    if (ImGui::Button(tr(" O "), ImVec2(30, 30))) agregar_grupo("O", "O");
    ImGui::SameLine();
    ImGui::PopStyleColor();
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.9f, 0.9f, 0.1f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.0f, 0.0f, 0.0f, 1.0f));
    if (ImGui::Button(tr(" S "), ImVec2(30, 30))) agregar_grupo("S", "S");
    ImGui::PopStyleColor(2);
    
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.1f, 0.9f, 0.1f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.0f, 0.0f, 0.0f, 1.0f));
    if (ImGui::Button(tr(" Cl"), ImVec2(30, 30))) agregar_grupo("Cl", "Cl");
    ImGui::PopStyleColor(2);
    
    ImGui::Separator();
    ImGui::Text(tr("Molecular formula:"));
    ImGui::TextDisabled(tr("%s"), formula_);
    ImGui::Text(tr("Molecular weight:"));
    ImGui::TextDisabled(tr("%.2f"), mw_);
    
    ImGui::Separator();
    ImGui::TextWrapped("Constructor molecular experimental. Estimación Benson no disponible: faltan contribuciones de grupos validadas.");
    
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
