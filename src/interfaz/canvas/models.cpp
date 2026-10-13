#include "shared.hpp"
namespace cma { namespace interfaz {
void Lienzo::dibujar_modelo(ImDrawList* draw, ImVec2 c, float s, nucleo::TipoNodo tipo) {
    ImU32 col_wh   = IM_COL32(255, 255, 255, 200);
    ImU32 col_gr   = IM_COL32(140, 140, 160, 200);
    ImU32 col_or   = IM_COL32(255, 180, 60, 220);
    ImU32 col_cy   = IM_COL32(80, 200, 220, 220);
    ImU32 col_gn   = IM_COL32(80, 220, 100, 220);
    ImU32 col_rd   = IM_COL32(220, 80, 80, 220);
    ImU32 col_bu   = IM_COL32(80, 140, 220, 220);
    ImU32 col_pu   = IM_COL32(180, 100, 220, 220);
    ImU32 col_yl   = IM_COL32(220, 200, 60, 220);

    switch (tipo) {
        case nucleo::TipoNodo::Fuente: {
            // Tanque de alimentación
            float w = s*0.45f, h = s*0.35f;
            ImVec2 p0(c.x-w,c.y-h), p1(c.x+w,c.y+h);
            draw->AddRectFilled(p0, p1, IM_COL32(60, 180, 75, 200));
            draw->AddRect(p0, p1, col_gn, 0, 2);
            draw->AddLine(ImVec2(p0.x,p0.y+h*0.3f), ImVec2(p1.x,p0.y+h*0.3f), col_gn, 1.5f);
            // Tubería de salida
            draw->AddLine(ImVec2(c.x+w,c.y), ImVec2(c.x+w+s*0.3f,c.y), col_wh, 2.5f);
            draw->AddTriangleFilled(ImVec2(c.x+w+s*0.35f,c.y), ImVec2(c.x+w+s*0.15f,c.y-5), ImVec2(c.x+w+s*0.15f,c.y+5), col_wh);
            // Válvula en tubería
            draw->AddCircleFilled(ImVec2(c.x+w+s*0.15f,c.y), 2.5f, col_wh);
            // Líquido en tanque
            draw->AddRectFilled(ImVec2(p0.x+2,p0.y+h*0.7f), ImVec2(p1.x-2,p1.y-2), IM_COL32(60, 180, 75, 180));
            break;
        }
        case nucleo::TipoNodo::Sumidero: {
            // Tanque de producto
            float w = s*0.45f, h = s*0.35f;
            ImVec2 p0(c.x-w,c.y-h), p1(c.x+w,c.y+h);
            draw->AddRectFilled(p0, p1, IM_COL32(220, 70, 70, 200));
            draw->AddRect(p0, p1, col_rd, 0, 2);
            draw->AddLine(ImVec2(p0.x,p0.y+h*0.3f), ImVec2(p1.x,p0.y+h*0.3f), col_rd, 1.5f);
            // Tubería de entrada
            draw->AddLine(ImVec2(c.x-w-s*0.3f,c.y), ImVec2(c.x-w,c.y), col_wh, 2.5f);
            draw->AddTriangleFilled(ImVec2(c.x-w-s*0.35f,c.y), ImVec2(c.x-w-s*0.15f,c.y-5), ImVec2(c.x-w-s*0.15f,c.y+5), col_wh);
            // Líquido en tanque
            draw->AddRectFilled(ImVec2(p0.x+2,p0.y+h*0.6f), ImVec2(p1.x-2,p1.y-2), IM_COL32(220, 70, 70, 180));
            break;
        }
        case nucleo::TipoNodo::Mezclador: {
            // Círculo del mezclador con aspas
            float r = s*0.38f;
            draw->AddCircleFilled(c, r, IM_COL32(70, 140, 220, 200));
            draw->AddCircle(c, r, col_bu, 0, 2.5f);
            draw->AddCircle(c, r*0.55f, col_bu, 0, 1);
            // Aspas internas (estilo HSC)
            draw->AddLine(ImVec2(c.x,c.y-r*0.6f), ImVec2(c.x,c.y+r*0.6f), col_bu, 2);
            draw->AddLine(ImVec2(c.x-r*0.6f,c.y), ImVec2(c.x+r*0.6f,c.y), col_bu, 2);
            // Aspas diagonales más delgadas
            draw->AddLine(ImVec2(c.x-r*0.4f,c.y-r*0.4f), ImVec2(c.x+r*0.4f,c.y+r*0.4f), col_bu, 1.5f);
            draw->AddLine(ImVec2(c.x+r*0.4f,c.y-r*0.4f), ImVec2(c.x-r*0.4f,c.y+r*0.4f), col_bu, 1.5f);
            // Entradas
            draw->AddLine(ImVec2(c.x-s*0.6f,c.y-r*0.5f), ImVec2(c.x-r,c.y-r*0.5f), col_wh, 2.5f);
            draw->AddLine(ImVec2(c.x-s*0.6f,c.y+r*0.5f), ImVec2(c.x-r,c.y+r*0.5f), col_wh, 2.5f);
            // Salida
            draw->AddLine(ImVec2(c.x+r,c.y), ImVec2(c.x+s*0.6f,c.y), col_wh, 2.5f);
            draw->AddTriangleFilled(ImVec2(c.x+s*0.65f,c.y), ImVec2(c.x+s*0.45f,c.y-5), ImVec2(c.x+s*0.45f,c.y+5), col_wh);
            // Punto central
            draw->AddCircleFilled(c, 3, col_wh);
            break;
        }
        case nucleo::TipoNodo::Divisor: {
            // Círculo divisor con flechas divergentes
            float r = s*0.38f;
            draw->AddCircleFilled(c, r, IM_COL32(220, 180, 50, 200));
            draw->AddCircle(c, r, col_yl, 0, 2.5f);
            draw->AddCircle(c, r*0.55f, col_yl, 0, 1);
            // Línea divisoria central
            draw->AddLine(ImVec2(c.x-r*0.7f,c.y), ImVec2(c.x+r*0.7f,c.y), col_yl, 1.5f);
            // Entrada
            draw->AddLine(ImVec2(c.x-s*0.6f,c.y), ImVec2(c.x-r,c.y), col_wh, 2.5f);
            draw->AddTriangleFilled(ImVec2(c.x-s*0.65f,c.y), ImVec2(c.x-s*0.45f,c.y-5), ImVec2(c.x-s*0.45f,c.y+5), col_wh);
            // Salidas divergentes
            draw->AddLine(ImVec2(c.x+r,c.y-r*0.5f), ImVec2(c.x+s*0.6f,c.y-s*0.4f), col_wh, 2.5f);
            draw->AddLine(ImVec2(c.x+r,c.y+r*0.5f), ImVec2(c.x+s*0.6f,c.y+s*0.4f), col_wh, 2.5f);
            draw->AddTriangleFilled(ImVec2(c.x+s*0.65f,c.y-s*0.4f), ImVec2(c.x+s*0.45f,c.y-s*0.5f), ImVec2(c.x+s*0.45f,c.y-s*0.3f), col_wh);
            draw->AddTriangleFilled(ImVec2(c.x+s*0.65f,c.y+s*0.4f), ImVec2(c.x+s*0.45f,c.y+s*0.3f), ImVec2(c.x+s*0.45f,c.y+s*0.5f), col_wh);
            break;
        }
        case nucleo::TipoNodo::Reactor: {
            // Reactor tipo CSTR con agitador (estilo HSC)
            float r = s*0.42f;
            // Cuerpo del reactor
            draw->AddCircleFilled(c, r, IM_COL32(240, 110, 50, 200));
            draw->AddCircle(c, r, col_or, 0, 2.5f);
            // Camisa de calentamiento (anillo exterior)
            draw->AddCircle(c, r+4, col_or, 0, 1);
            // Agitador
            draw->AddLine(ImVec2(c.x,c.y-r*0.7f), ImVec2(c.x,c.y+r*0.7f), col_wh, 2);
            draw->AddLine(ImVec2(c.x-r*0.5f,c.y-r*0.3f), ImVec2(c.x+r*0.5f,c.y-r*0.3f), col_wh, 2);
            draw->AddLine(ImVec2(c.x-r*0.4f,c.y+r*0.2f), ImVec2(c.x+r*0.4f,c.y+r*0.2f), col_wh, 1.5f);
            // Motor en la parte superior
            draw->AddRectFilled(ImVec2(c.x-6,c.y-r-10), ImVec2(c.x+6,c.y-r-2), col_wh);
            // Entradas/Salidas
            draw->AddLine(ImVec2(c.x-r,c.y-r*0.4f), ImVec2(c.x-r-8,c.y-r*0.4f), col_wh, 2);
            draw->AddLine(ImVec2(c.x+r,c.y+r*0.4f), ImVec2(c.x+r+8,c.y+r*0.4f), col_wh, 2);
            // Nivel de líquido interno
            draw->AddRectFilled(ImVec2(c.x-r*0.5f,c.y-r*0.1f), ImVec2(c.x+r*0.5f,c.y+r*0.6f), IM_COL32(240, 110, 50, 180));
            // Burbujas de reacción
            draw->AddCircleFilled(ImVec2(c.x-r*0.2f,c.y-r*0.1f), 2, col_wh);
            draw->AddCircleFilled(ImVec2(c.x+r*0.3f,c.y-r*0.15f), 1.5f, col_wh);
            draw->AddCircleFilled(ImVec2(c.x-r*0.1f,c.y-r*0.3f), 1.5f, col_wh);
            break;
        }
        case nucleo::TipoNodo::Separador: {
            // Separador líquido-líquido / decantador
            float w = s*0.5f, h = s*0.4f;
            ImVec2 p0(c.x-w,c.y-h), p1(c.x+w,c.y+h);
            draw->AddRectFilled(p0, p1, IM_COL32(160, 70, 200, 200));
            draw->AddRect(p0, p1, col_pu, 0, 2.5f);
            // Interfase
            draw->AddLine(ImVec2(p0.x,p0.y+h*0.1f), ImVec2(p1.x,p0.y+h*0.1f), col_pu, 2);
            // Fase ligera (arriba)
            draw->AddRectFilled(ImVec2(p0.x+2,p0.y+2), ImVec2(p1.x-2,p0.y+h*0.1f-2), IM_COL32(160,70,200,20));
            // Fase pesada (abajo)
            draw->AddRectFilled(ImVec2(p0.x+2,p0.y+h*0.1f+2), ImVec2(p1.x-2,p1.y-2), IM_COL32(160, 70, 200, 200));
            // Tubería salida ligera (arriba)
            draw->AddLine(ImVec2(c.x-w*0.3f,c.y-h), ImVec2(c.x-w*0.3f,c.y-h-10), col_wh, 2);
            draw->AddTriangleFilled(ImVec2(c.x-w*0.3f,c.y-h-14), ImVec2(c.x-w*0.3f-4,c.y-h-8), ImVec2(c.x-w*0.3f+4,c.y-h-8), col_wh);
            // Tubería salida pesada (abajo)
            draw->AddLine(ImVec2(c.x+w*0.3f,c.y+h), ImVec2(c.x+w*0.3f,c.y+h+10), col_wh, 2);
            draw->AddTriangleFilled(ImVec2(c.x+w*0.3f,c.y+h+14), ImVec2(c.x+w*0.3f-4,c.y+h+8), ImVec2(c.x+w*0.3f+4,c.y+h+8), col_wh);
            // Entrada
            draw->AddLine(ImVec2(c.x-w,c.y), ImVec2(c.x-w-10,c.y), col_wh, 2);
            break;
        }
        case nucleo::TipoNodo::IntercambiadorCalor: {
            // Intercambiador de calor de tubo y coraza (shell & tube)
            float w = s*0.55f, h = s*0.25f;
            ImVec2 p0(c.x-w,c.y-h), p1(c.x+w,c.y+h);
            // Coraza
            draw->AddRectFilled(p0, p1, IM_COL32(50, 200, 200, 200));
            draw->AddRect(p0, p1, col_cy, 0, 2.5f);
            // Tubos internos
            for (int i = -2; i <= 2; i++) {
                float y = c.y + i * h * 0.35f;
                draw->AddLine(ImVec2(p0.x+4,y), ImVec2(p1.x-4,y), col_cy, 1.5f);
            }
            // Cabezales
            draw->AddRectFilled(ImVec2(p0.x-6,p0.y+2), ImVec2(p0.x+2,p1.y-2), IM_COL32(50,200,200,60));
            draw->AddRectFilled(ImVec2(p1.x-2,p0.y+2), ImVec2(p1.x+6,p1.y-2), IM_COL32(50,200,200,60));
            // Tuberías de fluido caliente (entra por un lado)
            draw->AddLine(ImVec2(p0.x-16,c.y-h*0.6f), ImVec2(p0.x-2,c.y-h*0.6f), col_or, 2);
            draw->AddTriangleFilled(ImVec2(p0.x-20,c.y-h*0.6f), ImVec2(p0.x-14,c.y-h*0.6f-4), ImVec2(p0.x-14,c.y-h*0.6f+4), col_or);
            draw->AddLine(ImVec2(p1.x+2,c.y-h*0.6f), ImVec2(p1.x+16,c.y-h*0.6f), col_or, 2);
            // Tuberías de fluido frío
            draw->AddLine(ImVec2(p1.x+2,c.y+h*0.6f), ImVec2(p1.x+16,c.y+h*0.6f), col_bu, 2);
            draw->AddTriangleFilled(ImVec2(p1.x+20,c.y+h*0.6f), ImVec2(p1.x+14,c.y+h*0.6f-4), ImVec2(p1.x+14,c.y+h*0.6f+4), col_bu);
            draw->AddLine(ImVec2(p0.x-16,c.y+h*0.6f), ImVec2(p0.x-2,c.y+h*0.6f), col_bu, 2);
            break;
        }
        case nucleo::TipoNodo::ColumnaDestilacion: {
            // Columna de destilación con platos, condensador y reboiler
            float w = s*0.18f, h = s*0.55f;
            ImVec2 p0(c.x-w,c.y-h), p1(c.x+w,c.y+h);
            // Cuerpo de la columna
            draw->AddRectFilled(p0, p1, IM_COL32(50, 140, 220, 200));
            draw->AddRect(p0, p1, col_bu, 0, 2.5f);
            // Platos (bandejas) estilo HSC
            for (int i = 1; i <= 8; i++) {
                float y = p0.y + h * (i * 0.1f);
                draw->AddLine(ImVec2(p0.x,y), ImVec2(p1.x,y), col_bu, 1);
                // Bajantes alternados
                if (i % 2 == 0)
                    draw->AddRectFilled(ImVec2(p1.x-4,y-2), ImVec2(p1.x,y+2), IM_COL32(50,140,220,80));
                else
                    draw->AddRectFilled(ImVec2(p0.x,y-2), ImVec2(p0.x+4,y+2), IM_COL32(50,140,220,80));
            }
            // Condensador (arriba)
            float cond_w = w*1.8f, cond_h = h*0.12f;
            ImVec2 cond_p0(c.x-cond_w,c.y-h-cond_h), cond_p1(c.x+cond_w,c.y-h);
            draw->AddRectFilled(cond_p0, cond_p1, IM_COL32(100,200,255,60));
            draw->AddRect(cond_p0, cond_p1, col_cy, 0, 1.5f);
            // Serpentín del condensador
            for (int i = -2; i <= 2; i++)
                draw->AddLine(ImVec2(cond_p0.x+4, c.y-h-cond_h*0.5f), ImVec2(cond_p1.x-4, c.y-h-cond_h*0.5f), col_cy, 1);
            // Reflux
            draw->AddLine(ImVec2(c.x,c.y-h-cond_h), ImVec2(c.x,c.y-h+4), IM_COL32(100,200,255,200), 2);
            // Destilado (salida arriba)
            draw->AddLine(ImVec2(c.x+cond_w+4,c.y-h-cond_h*0.5f), ImVec2(c.x+cond_w+14,c.y-h-cond_h*0.5f), col_wh, 2);
            draw->AddTriangleFilled(ImVec2(c.x+cond_w+18,c.y-h-cond_h*0.5f), ImVec2(c.x+cond_w+12,c.y-h-cond_h*0.5f-4), ImVec2(c.x+cond_w+12,c.y-h-cond_h*0.5f+4), col_wh);
            // Reboiler (abajo)
            float reb_w = w*2.0f, reb_h = h*0.12f;
            ImVec2 reb_p0(c.x-reb_w,c.y+h), reb_p1(c.x+reb_w,c.y+h+reb_h);
            draw->AddRectFilled(reb_p0, reb_p1, IM_COL32(255,200,100,60));
            draw->AddRect(reb_p0, reb_p1, col_or, 0, 1.5f);
            // Residuo (salida abajo)
            draw->AddLine(ImVec2(c.x,c.y+h+reb_h), ImVec2(c.x,c.y+h+reb_h+10), col_wh, 2);
            draw->AddTriangleFilled(ImVec2(c.x,c.y+h+reb_h+14), ImVec2(c.x-4,c.y+h+reb_h+8), ImVec2(c.x+4,c.y+h+reb_h+8), col_wh);
            // Alimentación
            draw->AddLine(ImVec2(c.x-w-14,c.y), ImVec2(c.x-w,c.y), col_wh, 2);
            draw->AddTriangleFilled(ImVec2(c.x-w-18,c.y), ImVec2(c.x-w-12,c.y-4), ImVec2(c.x-w-12,c.y+4), col_wh);
            break;
        }
        case nucleo::TipoNodo::Compresor: {
            // Compresor tipo HSC (círculo con paletas rotatorias animadas)
            float r = s*0.38f;
            draw->AddCircleFilled(c, r, IM_COL32(220, 180, 40, 200));
            draw->AddCircle(c, r, col_yl, 0, 2.5f);
            draw->AddCircle(c, r*0.2f, col_yl, 0, 1.5f);
            // Paletas rotatorias (animadas)
            float ang = ImGui::GetTime() * 3.0f;
            for (int i = 0; i < 5; i++) {
                float a = ang + i * 1.2566f;
                ImVec2 p(c.x + cosf(a)*r*0.7f, c.y + sinf(a)*r*0.7f);
                draw->AddLine(c, p, col_yl, 2.5f);
                // Punta de paleta
                draw->AddCircleFilled(p, 3, col_yl);
            }
            // Carcasa exterior gruesa
            draw->AddCircle(c, r+3, col_yl, 0, 1);
            // Tubería de entrada
            draw->AddLine(ImVec2(c.x-r-8,c.y-r*0.4f), ImVec2(c.x-r,c.y-r*0.4f), col_wh, 2.5f);
            // Tubería de salida
            draw->AddLine(ImVec2(c.x+r,c.y+r*0.4f), ImVec2(c.x+r+8,c.y+r*0.4f), col_wh, 2.5f);
            draw->AddTriangleFilled(ImVec2(c.x+r+12,c.y+r*0.4f), ImVec2(c.x+r+6,c.y+r*0.4f-4), ImVec2(c.x+r+6,c.y+r*0.4f+4), col_wh);
            break;
        }
        case nucleo::TipoNodo::Válvula: {
            // Válvula tipo globo (estilo HSC)
            float r = s*0.32f;
            // Cuerpo de la válvula (triángulo estilo HSC)
            ImVec2 p0(c.x-r,c.y-r*0.7f), p1(c.x+r,c.y-r*0.7f), p2(c.x,c.y+r*0.8f);
            draw->AddTriangleFilled(p0, p1, p2, IM_COL32(200,140,60,50));
            draw->AddTriangle(p0, p1, p2, col_or, 2.5f);
            // Círculo interior (obturador)
            draw->AddCircleFilled(c, r*0.3f, IM_COL32(200,140,60,80));
            draw->AddCircle(c, r*0.3f, col_or, 0, 1.5f);
            // Vástago
            draw->AddLine(ImVec2(c.x,c.y-r*0.7f), ImVec2(c.x,c.y-r*1.1f), col_or, 2.5f);
            // Volante (rueda)
            draw->AddLine(ImVec2(c.x-6,c.y-r*1.3f), ImVec2(c.x+6,c.y-r*1.3f), col_or, 2);
            draw->AddLine(ImVec2(c.x,c.y-r*1.4f), ImVec2(c.x,c.y-r*1.2f), col_or, 2);
            // Tuberías
            draw->AddLine(ImVec2(c.x-r-10,c.y), ImVec2(c.x-r,c.y), col_wh, 2.5f);
            draw->AddLine(ImVec2(c.x+r,c.y), ImVec2(c.x+r+10,c.y), col_wh, 2.5f);
            break;
        }
        case nucleo::TipoNodo::Turbina: {
            // Turbina (expansor) estilo HSC
            float r = s*0.38f;
            draw->AddCircleFilled(c, r, IM_COL32(100, 200, 220, 200));
            draw->AddCircle(c, r, col_cy, 0, 2.5f);
            draw->AddCircle(c, r*0.25f, col_cy, 0, 1.5f);
            // Álabe rotatorio animado
            float ang = -ImGui::GetTime() * 4.0f;
            for (int i = 0; i < 3; i++) {
                float a = ang + i * 2.094f;
                ImVec2 p(c.x + cosf(a)*r*0.6f, c.y + sinf(a)*r*0.6f);
                draw->AddLine(c, p, col_cy, 2.5f);
                draw->AddCircleFilled(p, 2.5f, col_cy);
            }
            // Tubería de alta presión (entrada)
            draw->AddLine(ImVec2(c.x-r-8,c.y-r*0.5f), ImVec2(c.x-r,c.y-r*0.5f), col_or, 2.5f);
            // Tubería de baja presión (salida)
            draw->AddLine(ImVec2(c.x+r,c.y+r*0.5f), ImVec2(c.x+r+8,c.y+r*0.5f), col_wh, 2.5f);
            draw->AddTriangleFilled(ImVec2(c.x+r+12,c.y+r*0.5f), ImVec2(c.x+r+6,c.y+r*0.5f-4), ImVec2(c.x+r+6,c.y+r*0.5f+4), col_wh);
            break;
        }
        case nucleo::TipoNodo::Bomba: {
            // Bomba centrífuga estilo HSC
            float r = s*0.35f;
            draw->AddCircleFilled(c, r, IM_COL32(100, 160, 220, 200));
            draw->AddCircle(c, r, col_bu, 0, 2.5f);
            draw->AddCircle(c, r*0.6f, col_bu, 0, 1);
            // Impulsor
            draw->AddLine(ImVec2(c.x,c.y-r*0.3f), ImVec2(c.x,c.y+r*0.3f), col_bu, 2);
            draw->AddLine(ImVec2(c.x-r*0.3f,c.y), ImVec2(c.x+r*0.3f,c.y), col_bu, 2);
            // Ojo del impulsor (centro)
            draw->AddCircleFilled(c, r*0.2f, IM_COL32(100,160,220,80));
            draw->AddCircle(c, r*0.2f, col_bu, 0, 1);
            // Carcasa en espiral (voluta)
            draw->AddCircle(c, r+3, col_bu, 0, 1);
            // Tubería de succión
            draw->AddLine(ImVec2(c.x-r-8,c.y), ImVec2(c.x-r,c.y), col_wh, 2.5f);
            // Tubería de descarga (vertical, estilo HSC)
            draw->AddLine(ImVec2(c.x,c.y-r), ImVec2(c.x,c.y-r-12), col_wh, 2.5f);
            draw->AddTriangleFilled(ImVec2(c.x,c.y-r-16), ImVec2(c.x-4,c.y-r-10), ImVec2(c.x+4,c.y-r-10), col_wh);
            break;
        }
        case nucleo::TipoNodo::TrituradoraGiratoria: {
            // Trituradora giratoria con tolva cónica superior y manto central
            float w = s*0.4f, h = s*0.5f;
            ImVec2 pts[4] = {
                ImVec2(c.x - w, c.y - h),
                ImVec2(c.x + w, c.y - h),
                ImVec2(c.x + w*0.4f, c.y + h),
                ImVec2(c.x - w*0.4f, c.y + h)
            };
            draw->AddConvexPolyFilled(pts, 4, IM_COL32(180, 180, 180, 50));
            draw->AddPolyline(pts, 4, col_gr, ImDrawFlags_Closed, 2);
            draw->AddLine(ImVec2(c.x, c.y - h - 5), ImVec2(c.x, c.y + h - 5), col_wh, 2.5f);
            ImVec2 m_pts[3] = {
                ImVec2(c.x, c.y - h*0.2f),
                ImVec2(c.x + w*0.3f, c.y + h*0.6f),
                ImVec2(c.x - w*0.3f, c.y + h*0.6f)
            };
            draw->AddTriangleFilled(m_pts[0], m_pts[1], m_pts[2], IM_COL32(230, 140, 50, 180));
            draw->AddTriangle(m_pts[0], m_pts[1], m_pts[2], col_or, 1.5f);
            break;
        }
        case nucleo::TipoNodo::Zaranda: {
            // Zaranda vibradora inclinada (Screen)
            float w = s*0.5f, h = s*0.35f;
            float ang = 0.25f; // inclinación en radianes
            auto rot = [&](float x, float y) {
                return ImVec2(c.x + x*cosf(ang) - y*sinf(ang), c.y + x*sinf(ang) + y*cosf(ang));
            };
            ImVec2 pts[4] = {
                rot(-w, -h), rot(w, -h), rot(w, h), rot(-w, h)
            };
            draw->AddConvexPolyFilled(pts, 4, IM_COL32(220, 200, 50, 200));
            draw->AddPolyline(pts, 4, col_yl, ImDrawFlags_Closed, 2);
            draw->AddLine(rot(-w*0.8f, -h*0.3f), rot(w*0.8f, -h*0.3f), col_wh, 1.5f);
            draw->AddLine(rot(-w*0.8f, h*0.3f), rot(w*0.8f, h*0.3f), col_wh, 1.5f);
            draw->AddCircleFilled(rot(0, -h*0.6f), 4, col_wh);
            break;
        }
        case nucleo::TipoNodo::MolinoSAG: {
            // Molino SAG: cilindro corto y ancho de gran diámetro
            float w = s*0.4f, h = s*0.55f;
            ImVec2 p0(c.x-w, c.y-h), p1(c.x+w, c.y+h);
            draw->AddRectFilled(p0, p1, IM_COL32(100, 160, 220, 200));
            draw->AddRect(p0, p1, col_bu, 0, 2);
            ImVec2 t_izq[3] = { ImVec2(c.x-w, c.y-h), ImVec2(c.x-w-s*0.15f, c.y), ImVec2(c.x-w, c.y+h) };
            draw->AddTriangleFilled(t_izq[0], t_izq[1], t_izq[2], IM_COL32(100, 160, 220, 60));
            draw->AddPolyline(t_izq, 3, col_bu, 0, 2);
            ImVec2 t_der[3] = { ImVec2(c.x+w, c.y-h), ImVec2(c.x+w+s*0.15f, c.y), ImVec2(c.x+w, c.y+h) };
            draw->AddTriangleFilled(t_der[0], t_der[1], t_der[2], IM_COL32(100, 160, 220, 60));
            draw->AddPolyline(t_der, 3, col_bu, 0, 2);
            draw->AddRectFilled(ImVec2(c.x-4, c.y-h-3), ImVec2(c.x+4, c.y+h+3), col_bu);
            break;
        }
        case nucleo::TipoNodo::MolinoBolas: {
            // Molino de Bolas: cilindro más largo y esbelto
            float w = s*0.65f, h = s*0.4f;
            ImVec2 p0(c.x-w, c.y-h), p1(c.x+w, c.y+h);
            draw->AddRectFilled(p0, p1, IM_COL32(100, 160, 220, 200));
            draw->AddRect(p0, p1, col_bu, 0, 2);
            ImVec2 t_izq[3] = { ImVec2(c.x-w, c.y-h), ImVec2(c.x-w-s*0.15f, c.y), ImVec2(c.x-w, c.y+h) };
            draw->AddTriangleFilled(t_izq[0], t_izq[1], t_izq[2], IM_COL32(100, 160, 220, 60));
            draw->AddPolyline(t_izq, 3, col_bu, 0, 2);
            ImVec2 t_der[3] = { ImVec2(c.x+w, c.y-h), ImVec2(c.x+w+s*0.15f, c.y), ImVec2(c.x+w, c.y+h) };
            draw->AddTriangleFilled(t_der[0], t_der[1], t_der[2], IM_COL32(100, 160, 220, 60));
            draw->AddPolyline(t_der, 3, col_bu, 0, 2);
            draw->AddRectFilled(ImVec2(c.x-4, c.y-h-3), ImVec2(c.x+4, c.y+h+3), col_bu);
            break;
        }
        case nucleo::TipoNodo::Hidrociclon: {
            // Hidrociclón con parte cilíndrica y parte cónica aguda
            float w = s*0.35f, h = s*0.25f;
            ImVec2 p0(c.x-w, c.y-h), p1(c.x+w, c.y);
            draw->AddRectFilled(p0, p1, IM_COL32(230, 160, 50, 200));
            draw->AddRect(p0, p1, col_or, 0, 2);
            ImVec2 cone[3] = { ImVec2(c.x-w, c.y), ImVec2(c.x+w, c.y), ImVec2(c.x, c.y+h*2.2f) };
            draw->AddTriangleFilled(cone[0], cone[1], cone[2], IM_COL32(230, 160, 50, 200));
            draw->AddPolyline(cone, 3, col_or, ImDrawFlags_Closed, 2);
            draw->AddLine(ImVec2(c.x, c.y-h), ImVec2(c.x, c.y-h-12), col_wh, 2.5f);
            draw->AddLine(ImVec2(c.x, c.y+h*2.2f), ImVec2(c.x, c.y+h*2.2f+8), col_wh, 2.5f);
            break;
        }
        case nucleo::TipoNodo::Espesador: {
            // Espesador de relaves (Thickener)
            float w = s*0.75f, h = s*0.25f;
            ImVec2 p0(c.x-w, c.y-h), p1(c.x+w, c.y);
            draw->AddRectFilled(p0, p1, IM_COL32(230, 180, 50, 200));
            draw->AddRect(p0, p1, col_yl, 0, 2);
            ImVec2 cone[3] = { ImVec2(c.x-w, c.y), ImVec2(c.x+w, c.y), ImVec2(c.x, c.y+h*1.5f) };
            draw->AddTriangleFilled(cone[0], cone[1], cone[2], IM_COL32(230, 180, 50, 200));
            draw->AddPolyline(cone, 3, col_yl, ImDrawFlags_Closed, 2);
            draw->AddLine(ImVec2(c.x, c.y-h-8), ImVec2(c.x, c.y+h*0.5f), col_wh, 2.5f);
            draw->AddLine(ImVec2(c.x, c.y+h*0.5f), ImVec2(c.x-w*0.8f, c.y+h*0.1f), col_wh, 1.5f);
            draw->AddLine(ImVec2(c.x, c.y+h*0.5f), ImVec2(c.x+w*0.8f, c.y+h*0.1f), col_wh, 1.5f);
            draw->AddLine(ImVec2(c.x-w-4, c.y-h), ImVec2(c.x+w+4, c.y-h), col_wh, 3.0f);
            break;
        }
        case nucleo::TipoNodo::Filtro: {
            // Filtro de banda de vacío (Filter)
            float w = s*0.65f, h = s*0.18f;
            draw->AddRectFilled(ImVec2(c.x-w, c.y-h), ImVec2(c.x+w, c.y+h), IM_COL32(80, 180, 220, 200), h);
            draw->AddRect(ImVec2(c.x-w, c.y-h), ImVec2(c.x+w, c.y+h), col_cy, h, 0, 2);
            draw->AddCircleFilled(ImVec2(c.x-w, c.y), h*0.8f, col_cy);
            draw->AddCircleFilled(ImVec2(c.x+w, c.y), h*0.8f, col_cy);
            draw->AddLine(ImVec2(c.x, c.y+h), ImVec2(c.x, c.y+h+12), col_wh, 2.5f);
            break;
        }
        case nucleo::TipoNodo::CeldaFlotacion: {
            // Celda de flotación industrial (Flotation Cell)
            float w = s*0.45f, h = s*0.5f;
            ImVec2 p0(c.x-w, c.y-h), p1(c.x+w, c.y+h);
            draw->AddRectFilled(p0, p1, IM_COL32(240, 200, 50, 200));
            draw->AddRect(p0, p1, col_yl, 0, 2);
            draw->AddLine(ImVec2(c.x, c.y-h-6), ImVec2(c.x, c.y+h*0.6f), col_wh, 2);
            draw->AddLine(ImVec2(c.x-w*0.3f, c.y+h*0.6f), ImVec2(c.x+w*0.3f, c.y+h*0.6f), col_wh, 2);
            ImVec2 launder[4] = { ImVec2(c.x+w, c.y-h), ImVec2(c.x+w+s*0.18f, c.y-h), ImVec2(c.x+w+s*0.12f, c.y-h*0.5f), ImVec2(c.x+w, c.y-h*0.5f) };
            draw->AddConvexPolyFilled(launder, 4, IM_COL32(240, 200, 50, 80));
            draw->AddPolyline(launder, 4, col_yl, ImDrawFlags_Closed, 1.5f);
            draw->AddCircleFilled(ImVec2(c.x-w*0.4f, c.y+h*0.1f), 1.5f, col_wh);
            draw->AddCircleFilled(ImVec2(c.x+w*0.3f, c.y-h*0.2f), 2.0f, col_wh);
            draw->AddCircleFilled(ImVec2(c.x-w*0.1f, c.y-h*0.4f), 1.5f, col_wh);
            break;
        }
        default:
            draw->AddCircle(c, s*0.3f, col_gr, 0, 2);
            break;
    }
}

// ═══════════════════════════════════════════════════════════════════════════


}} // namespace
