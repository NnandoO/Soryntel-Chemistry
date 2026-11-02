#include "shared.hpp"
#include <cmath>
#include <algorithm>

namespace cma { namespace interfaz {

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Gráficos 3D
// ═══════════════════════════════════════════════════════════════════════════

void PanelGrafico3D::generar_datos_ejemplo() {
    data_x_.clear(); data_y_.clear(); data_z_.clear();
    // Superficie: z = sin(r)/r (sinc)
    int n = 30;
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            float x = -3.0f + 6.0f * i / (n-1);
            float y = -3.0f + 6.0f * j / (n-1);
            float r = std::sqrt(x*x + y*y);
            float z = (r > 0.01f) ? std::sin(r)/r : 1.0f;
            data_x_.push_back(x);
            data_y_.push_back(y);
            data_z_.push_back(z);
        }
    }
    datos_generados_ = true;
}

ImVec3 PanelGrafico3D::proyectar(float x, float y, float z, const ImVec2& centro, float escala) const {
    float sx = -std::sin(ang_y_);
    float sy = std::cos(ang_y_);
    float cz = std::cos(ang_x_);
    float sz = std::sin(ang_x_);

    float rx = x * cz - z * sz;
    float ry = y;
    float rz = x * sz + z * cz;

    float px = rx * sy + ry * sx;
    float py = -rx * sx + ry * sy;
    float pz = rz + distancia_;

    if (pz < 0.1f) pz = 0.1f;
    float f = escala * 2.0f / pz;
    return ImVec3(centro.x + px * f, centro.y - py * f, pz);
}

void PanelGrafico3D::renderizar() {
    if (!datos_generados_) generar_datos_ejemplo();

    ImGui::Text(tr("Gráfico 3D"));
    ImGui::Separator();

    ImGui::SliderFloat("Rotación X", &ang_x_, -3.14f, 3.14f, "%.2f");
    ImGui::SliderFloat("Rotación Y", &ang_y_, -3.14f, 3.14f, "%.2f");
    ImGui::SliderFloat("Distancia", &distancia_, 1.0f, 20.0f, "%.1f");
    ImGui::Checkbox(tr("Auto-rotar"), &auto_rotar_);
    ImGui::SameLine();
    ImGui::Combo("Tipo", &tipo_grafico_, "Superficie\0Líneas\0Puntos\0Contorno\0");

    if (auto_rotar_) {
        ang_y_ += ImGui::GetIO().DeltaTime * 0.5f;
    }

    ImVec2 area = ImGui::GetContentRegionAvail();
    float lado = std::min(area.x, area.y);
    ImVec2 centro(area.x * 0.5f, 60.0f + lado * 0.5f);
    float escala = lado * 0.25f;

    ImDrawList* draw = ImGui::GetWindowDrawList();

    // Fondo
    draw->AddRectFilled(ImVec2(0, 50), ImVec2(area.x, 50 + lado),
                        IM_COL32(15,15,22,255), 4);

    // Ejes
    ImVec3 ox = proyectar(4,0,0,centro,escala);
    ImVec3 oy = proyectar(0,4,0,centro,escala);
    ImVec3 oz = proyectar(0,0,4,centro,escala);
    ImVec3 oo = proyectar(0,0,0,centro,escala);

    draw->AddLine(ImVec2(oo.x,oo.y), ImVec2(ox.x,ox.y), IM_COL32(255,80,80,200), 2);
    draw->AddLine(ImVec2(oo.x,oo.y), ImVec2(oy.x,oy.y), IM_COL32(80,255,80,200), 2);
    draw->AddLine(ImVec2(oo.x,oo.y), ImVec2(oz.x,oz.y), IM_COL32(80,80,255,200), 2);

    if (tipo_grafico_ == 0 || tipo_grafico_ == 3) {
        // Superficie / Contorno
        int n = (int)std::sqrt((float)data_x_.size());
        if (n > 1) {
            for (int i = 0; i < n-1; i++) {
                for (int j = 0; j < n-1; j++) {
                    int idx = i * n + j;
                    ImVec3 p1 = proyectar(data_x_[idx], data_y_[idx], data_z_[idx], centro, escala);
                    ImVec3 p2 = proyectar(data_x_[idx+1], data_y_[idx+1], data_z_[idx+1], centro, escala);
                    ImVec3 p3 = proyectar(data_x_[idx+n], data_y_[idx+n], data_z_[idx+n], centro, escala);
                    ImVec3 p4 = proyectar(data_x_[idx+n+1], data_y_[idx+n+1], data_z_[idx+n+1], centro, escala);

                    float val = (data_z_[idx] + data_z_[idx+n] + data_z_[idx+n+1] + data_z_[idx]) * 0.25f;
                    int r = (int)(128 + 127 * std::max(-1.0f, std::min(1.0f, val)));
                    ImU32 col = IM_COL32(r, (int)(255-r*0.5f), (int)(255-r), 180);

                    if (tipo_grafico_ == 0) {
                        draw->AddTriangleFilled(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), ImVec2(p3.x,p3.y), col);
                        draw->AddTriangleFilled(ImVec2(p2.x,p2.y), ImVec2(p4.x,p4.y), ImVec2(p3.x,p3.y), col);
                    }
                    draw->AddTriangle(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), ImVec2(p3.x,p3.y), IM_COL32(60,60,80,80), 1);
                    draw->AddTriangle(ImVec2(p2.x,p2.y), ImVec2(p4.x,p4.y), ImVec2(p3.x,p3.y), IM_COL32(60,60,80,80), 1);
                }
            }
        }
    } else if (tipo_grafico_ == 1) {
        // Líneas (isolinhas)
        int n = (int)std::sqrt((float)data_x_.size());
        for (int i = 0; i < n; i++) {
            for (int j = 0; j < n-1; j++) {
                int idx = i * n + j;
                ImVec3 p1 = proyectar(data_x_[idx], data_y_[idx], data_z_[idx], centro, escala);
                ImVec3 p2 = proyectar(data_x_[idx+1], data_y_[idx+1], data_z_[idx+1], centro, escala);
                draw->AddLine(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), IM_COL32(100,200,255,120), 1);
            }
        }
        for (int j = 0; j < n; j++) {
            for (int i = 0; i < n-1; i++) {
                int idx = i * n + j;
                ImVec3 p1 = proyectar(data_x_[idx], data_y_[idx], data_z_[idx], centro, escala);
                ImVec3 p2 = proyectar(data_x_[idx+n], data_y_[idx+n], data_z_[idx+n], centro, escala);
                draw->AddLine(ImVec2(p1.x,p1.y), ImVec2(p2.x,p2.y), IM_COL32(100,200,255,120), 1);
            }
        }
    } else {
        // Puntos
        for (size_t i = 0; i < data_x_.size(); i++) {
            ImVec3 p = proyectar(data_x_[i], data_y_[i], data_z_[i], centro, escala);
            float val = data_z_[i];
            int r = (int)(128 + 127 * std::max(-1.0f, std::min(1.0f, val)));
            ImU32 col = IM_COL32(r, 200-r, 255-r, 200);
            draw->AddCircleFilled(ImVec2(p.x,p.y), 3, col);
        }
    }

    // Leyenda de ejes
    draw->AddText(ImVec2(ox.x+4,ox.y-6), IM_COL32(255,120,120,200), "X");
    draw->AddText(ImVec2(oy.x+4,oy.y-6), IM_COL32(120,255,120,200), "Y");
    draw->AddText(ImVec2(oz.x+4,oz.y-6), IM_COL32(120,120,255,200), "Z");
}

}} // namespace cma::interfaz
