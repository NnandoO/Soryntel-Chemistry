#include "interfaz/lienzo.hpp"
#include "utils/i18n.h"
#include <imgui.h>
#include <algorithm>
#include <cmath>
#include <cstring>

namespace cma { namespace interfaz {

static std::string sanitizar_texto(std::string str) {
    size_t pos;
    while((pos = str.find("₄")) != std::string::npos) str.replace(pos, 3, "4");
    while((pos = str.find("₂")) != std::string::npos) str.replace(pos, 3, "2");
    while((pos = str.find("₃")) != std::string::npos) str.replace(pos, 3, "3");
    while((pos = str.find("₁")) != std::string::npos) str.replace(pos, 3, "1");
    while((pos = str.find("₀")) != std::string::npos) str.replace(pos, 3, "0");
    while((pos = str.find("₅")) != std::string::npos) str.replace(pos, 3, "5");
    while((pos = str.find("₆")) != std::string::npos) str.replace(pos, 3, "6");
    while((pos = str.find("₇")) != std::string::npos) str.replace(pos, 3, "7");
    while((pos = str.find("₈")) != std::string::npos) str.replace(pos, 3, "8");
    while((pos = str.find("₉")) != std::string::npos) str.replace(pos, 3, "9");
    while((pos = str.find("Δ")) != std::string::npos) str.replace(pos, 2, "d");
    while((pos = str.find("→")) != std::string::npos) str.replace(pos, 3, "->");
    return str;
}



static inline bool hover_rect(const ImVec2& mouse, const ImVec2& r_min, const ImVec2& r_max) {
    return mouse.x >= r_min.x && mouse.x <= r_max.x && mouse.y >= r_min.y && mouse.y <= r_max.y;
}

bool g_nodo_arrastrado = false;

static constexpr float NODO_ANCHO   = 150.0f;
static constexpr float NODO_ALTO    = 82.0f;
static constexpr float CABECERA_H   = 24.0f;
static constexpr float RADIO        = 6.0f;
static constexpr float RADIO_PTO    = 5.0f;
static constexpr float GRILLA       = 20.0f;
static constexpr float INERCIA_DEC  = 0.85f;

static ImU32 col_cab(nucleo::TipoNodo t) {
    switch (t) {
        case nucleo::TipoNodo::Fuente:              return IM_COL32(70,  200, 70,  255);
        case nucleo::TipoNodo::Sumidero:            return IM_COL32(220, 70,  70,  255);
        case nucleo::TipoNodo::Mezclador:           return IM_COL32(70,  140, 220, 255);
        case nucleo::TipoNodo::Divisor:             return IM_COL32(220, 180, 50,  255);
        case nucleo::TipoNodo::Reactor:             return IM_COL32(240, 110, 50,  255);
        case nucleo::TipoNodo::Separador:           return IM_COL32(160, 70,  200, 255);
        case nucleo::TipoNodo::IntercambiadorCalor: return IM_COL32(50,  200, 200, 255);
        case nucleo::TipoNodo::ColumnaDestilacion:  return IM_COL32(50,  140, 220, 255);
        case nucleo::TipoNodo::Compresor:           return IM_COL32(220, 140, 50,  255);
        case nucleo::TipoNodo::Turbina:             return IM_COL32(100, 200, 220, 255);
        case nucleo::TipoNodo::Bomba:               return IM_COL32(100, 160, 220, 255);
        case nucleo::TipoNodo::Válvula:             return IM_COL32(200, 140, 50,  255);
        default: return IM_COL32(100, 100, 120, 255);
    }
}

static const char* descripcion_tipo_nodo(nucleo::TipoNodo t) {
    switch (t) {
        case nucleo::TipoNodo::Fuente:               return "Fuente de alimentación de material o fluido.";
        case nucleo::TipoNodo::Sumidero:             return "Punto de salida o producto final.";
        case nucleo::TipoNodo::Mezclador:            return "Mezcla múltiples flujos de entrada en uno solo.";
        case nucleo::TipoNodo::Divisor:              return "Divide un flujo en múltiples salidas (fracciones).";
        case nucleo::TipoNodo::Válvula:              return "Controla el paso de un fluido.";
        case nucleo::TipoNodo::Multiplexor:          return "Selecciona un flujo entre varias entradas.";
        case nucleo::TipoNodo::Demultiplexor:        return "Dirige el flujo a una de varias salidas.";
        case nucleo::TipoNodo::Reactor:              return "Reactor químico genérico con conversión.";
        case nucleo::TipoNodo::ReactorGibbs:         return "Reactor con minimización de energía de Gibbs.";
        case nucleo::TipoNodo::ReactorEsteq:         return "Reactor con equilibrio estequiométrico.";
        case nucleo::TipoNodo::ReactorCinetica:      return "Reactor con cinética química (Arrhenius).";
        case nucleo::TipoNodo::TrituradoraMandibula: return "Trituración primaria de minerales.";
        case nucleo::TipoNodo::TrituradoraGiratoria: return "Trituración primaria de alta capacidad.";
        case nucleo::TipoNodo::TrituradoraCono:      return "Trituración secundaria/terciaria.";
        case nucleo::TipoNodo::MolinoSAG:            return "Molienda semiautógena (SAG).";
        case nucleo::TipoNodo::MolinoBolas:          return "Molienda por bolas.";
        case nucleo::TipoNodo::MolinoBarras:         return "Molienda por barras.";
        case nucleo::TipoNodo::Zaranda:              return "Clasificación por tamizado.";
        case nucleo::TipoNodo::Hidrociclon:         return "Clasificación hidráulica por ciclón.";
        case nucleo::TipoNodo::TanqueLixiviacion:    return "Lixiviación de minerales en tanque agitado.";
        case nucleo::TipoNodo::SXMixerSettler:       return "Extracción por solventes (mezclador-decantador).";
        case nucleo::TipoNodo::CeldaElectroobtencion: return "Electroobtención de metales (EW).";
        case nucleo::TipoNodo::HornoRotatorio:       return "Horno rotatorio para pirometalurgia.";
        case nucleo::TipoNodo::HornoFlash:           return "Horno flash (fundición rápida).";
        case nucleo::TipoNodo::HornoArco:             return "Horno de arco eléctrico (EAF).";
        case nucleo::TipoNodo::HornoReverbero:        return "Horno de reverbero.";
        case nucleo::TipoNodo::Tostador:             return "Tostación de sulfuros.";
        case nucleo::TipoNodo::Convertidor:          return "Convertidor (Peirce-Smith o similar).";
        case nucleo::TipoNodo::Espesador:            return "Espesamiento de pulpas/lamas.";
        case nucleo::TipoNodo::Filtro:               return "Filtración sólido-líquido.";
        case nucleo::TipoNodo::LavadorGases:         return "Lavado/depuración de gases.";
        case nucleo::TipoNodo::CeldaFlotacion:       return "Flotación de minerales.";
        case nucleo::TipoNodo::IntercambiadorCalor:  return "Intercambio de calor entre dos fluidos.";
        case nucleo::TipoNodo::Calentador:           return "Calienta un fluido.";
        case nucleo::TipoNodo::Enfriador:            return "Enfría un fluido.";
        case nucleo::TipoNodo::Bomba:                return "Aumenta la presión de un líquido.";
        case nucleo::TipoNodo::Turbina:              return "Genera trabajo expansiendo un fluido.";
        case nucleo::TipoNodo::Compresor:            return "Comprime un gas.";
        case nucleo::TipoNodo::ColumnaDestilacion:   return "Separación por destilación.";
        case nucleo::TipoNodo::Separador:            return "Separación de fases (líq/gas/vapor).";
        case nucleo::TipoNodo::Absorbedor:           return "Absorción de componentes gaseosos.";
        case nucleo::TipoNodo::Extraccion:           return "Extracción líquido-líquido.";
        case nucleo::TipoNodo::Sensor:               return "Sensor de medición de proceso.";
        case nucleo::TipoNodo::ControladorPID:       return "Controlador PID (lazo de control).";
        default: return nullptr;
    }
}

// ── Modelos detallados estilo HSC Chemistry ──────────────────────────────
void Lienzo::dibujar_modelo(ImDrawList* draw, ImVec2 c, float s, nucleo::TipoNodo tipo) {
    ImU32 col_fill = IM_COL32(200, 200, 220, 50);
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

Lienzo::Lienzo() {}

void Lienzo::guardar_estado() {
    if (flowsheet_) {
        undo_stack_.push_back(flowsheet_->a_json());
        // Limitar el historial a 50 estados para no consumir excesiva memoria
        if (undo_stack_.size() > 50)
            undo_stack_.erase(undo_stack_.begin());
        redo_stack_.clear();
        if (al_cambio_flowsheet_) al_cambio_flowsheet_();
    }
}

void Lienzo::deshacer() {
    if (flowsheet_ && !undo_stack_.empty()) {
        redo_stack_.push_back(flowsheet_->a_json());
        std::string estado = undo_stack_.back();
        undo_stack_.pop_back();
        std::string err;
        flowsheet_->desde_json(estado, err);
        id_nodo_seleccionado = -1;
        id_flujo_seleccionado = -1;
        id_anotacion_seleccionada = -1;
        if (al_cambio_flowsheet_) al_cambio_flowsheet_();
    }
}

void Lienzo::rehacer() {
    if (flowsheet_ && !redo_stack_.empty()) {
        undo_stack_.push_back(flowsheet_->a_json());
        std::string estado = redo_stack_.back();
        redo_stack_.pop_back();
        std::string err;
        flowsheet_->desde_json(estado, err);
        id_nodo_seleccionado = -1;
        id_flujo_seleccionado = -1;
        id_anotacion_seleccionada = -1;
        if (al_cambio_flowsheet_) al_cambio_flowsheet_();
    }
}

void Lienzo::limpiar_historial() {
    undo_stack_.clear();
    redo_stack_.clear();
}

void Lienzo::copiar() {
    if (id_nodo_seleccionado >= 0 && flowsheet_) {
        if (auto* n = flowsheet_->nodo_por_id(id_nodo_seleccionado)) {
            portapapeles_tipo_ = n->tipo;
            tiene_portapapeles_ = true;
        }
    }
}

void Lienzo::cortar() {
    copiar();
    if (id_nodo_seleccionado >= 0 && flowsheet_) {
        guardar_estado();
        flowsheet_->eliminar_nodo(id_nodo_seleccionado);
        id_nodo_seleccionado = -1;
    }
}

void Lienzo::pegar() {
    if (tiene_portapapeles_ && flowsheet_) {
        guardar_estado();
        int64_t nuevo_id = flowsheet_->agregar_nodo(portapapeles_tipo_);
        if (auto* n = flowsheet_->nodo_por_id(nuevo_id)) {
            n->canvas_x = coord_raton.x - NODO_ANCHO/2;
            n->canvas_y = coord_raton.y - NODO_ALTO/2;
            n->pagina = pagina_actual;
            id_nodo_seleccionado = nuevo_id;
        }
    }
}

void Lienzo::establecer_flowsheet(diagrama_flujo::Flowsheet* fs) {
    flowsheet_ = fs;
}

void Lienzo::centrar_en(float x, float y) {
    des_destino_ = ImVec2(-x, -y);
    animando_ = true;
}

void Lienzo::encajar_todo() {
    if (!flowsheet_ || flowsheet_->nodos().empty()) return;

    float xmin = 1e9, ymin = 1e9, xmax = -1e9, ymax = -1e9;
    bool has_nodes = false;
    for (auto& n : flowsheet_->nodos()) {
        if (n.pagina != pagina_actual) continue;
        has_nodes = true;
        xmin = std::min(xmin, n.canvas_x);
        ymin = std::min(ymin, n.canvas_y);
        xmax = std::max(xmax, n.canvas_x + NODO_ANCHO);
        ymax = std::max(ymax, n.canvas_y + NODO_ALTO);
    }
    if (!has_nodes) return;

    ImVec2 tam_lienzo = ultima_region_;
    if (tam_lienzo.x < 1 || tam_lienzo.y < 1) return;

    float cx = (xmin + xmax) * 0.5f;
    float cy = (ymin + ymax) * 0.5f;
    float escala_x = tam_lienzo.x / (xmax - xmin + 100);
    float escala_y = tam_lienzo.y / (ymax - ymin + 100);
    zoom = std::clamp(std::min(escala_x, escala_y), 0.1f, 5.0f);

    des_destino_ = ImVec2(tam_lienzo.x * 0.5f - cx * zoom,
                          tam_lienzo.y * 0.5f - cy * zoom);
    animando_ = true;
}

void Lienzo::renderizar(const ImVec2& region) {
    if (!flowsheet_) return;

    clic_detectado_en_objeto_ = false;

    ImDrawList* draw = ImGui::GetWindowDrawList();
    ImVec2 pos = ImGui::GetCursorScreenPos();
    ImVec2 tam = ImGui::GetContentRegionAvail();
    if (tam.x < 1) tam.x = 1;
    if (tam.y < 1) tam.y = 1;

    ImGui::InvisibleButton("lienzo_bg", tam);
    ImGui::SetCursorScreenPos(pos);

    bool hover = ImGui::IsItemHovered() || ImGui::IsWindowHovered(ImGuiHoveredFlags_ChildWindows);
    bool active = ImGui::IsWindowFocused(ImGuiFocusedFlags_ChildWindows);
    hover_lienzo_ = hover;
    clic_en_lienzo_ = hover && ImGui::IsMouseClicked(ImGuiMouseButton_Left);

    ultima_region_ = tam;

    // ── Atajos de Teclado (Undo, Redo, Portapapeles) ──
    ImGuiIO& io = ImGui::GetIO();
    if (active) {
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_Z)) {
            if (io.KeyShift) rehacer(); else deshacer();
        }
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_Y)) rehacer();
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_C)) copiar();
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_X)) cortar();
        if (io.KeyCtrl && ImGui::IsKeyPressed(ImGuiKey_V)) pegar();
    }

    // ── Animación suave ──
    if (animando_) {
        desplazamiento.x += (des_destino_.x - desplazamiento.x) * 0.12f;
        desplazamiento.y += (des_destino_.y - desplazamiento.y) * 0.12f;
        if (std::abs(des_destino_.x - desplazamiento.x) < 0.5f &&
            std::abs(des_destino_.y - desplazamiento.y) < 0.5f) {
            desplazamiento = des_destino_;
            animando_ = false;
        }
    }

    // Resetear el arrastre si soltamos el botón
    if (!ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
        extern bool g_nodo_arrastrado;
        g_nodo_arrastrado = false;
    }

    // ── Desplazar con botón central ──
    if (hover && ImGui::IsMouseDragging(ImGuiMouseButton_Middle)) {
        desplazamiento.x += ImGui::GetIO().MouseDelta.x;
        desplazamiento.y += ImGui::GetIO().MouseDelta.y;
        arrastrando_lienzo_ = true;
    } else {
        arrastrando_lienzo_ = false;
    }

    // ── Edge scroll (arrastrar con botón derecho cerca del borde) ──
    if (hover && active && ImGui::IsMouseDown(ImGuiMouseButton_Left) && !arrastrando_conexion_) {
        ImVec2 raton = ImGui::GetIO().MousePos;
        float edge = 20.0f;
        float vel = 8.0f;
        if (raton.x - pos.x < edge) desplazamiento.x += vel;
        if (pos.x + tam.x - raton.x < edge) desplazamiento.x -= vel;
        if (raton.y - pos.y < edge) desplazamiento.y += vel;
        if (pos.y + tam.y - raton.y < edge) desplazamiento.y -= vel;
    }

    // ── Zoom con rueda (centrado en cursor) ──
    if (hover) {
        float rueda = ImGui::GetIO().MouseWheel;
        if (rueda != 0) {
            float antiguo = zoom;
            zoom = std::clamp(zoom + rueda * 0.1f, 0.1f, 5.0f);
            ImVec2 raton = ImGui::GetIO().MousePos;
            desplazamiento.x = raton.x - (raton.x - desplazamiento.x) * (zoom / antiguo);
            desplazamiento.y = raton.y - (raton.y - desplazamiento.y) * (zoom / antiguo);
            animando_ = false;
        }
    }

    // ── Inercia ──
    if (!ImGui::IsMouseDown(ImGuiMouseButton_Middle) && !arrastrando_lienzo_) {
        vel_desplazamiento_.x *= INERCIA_DEC;
        vel_desplazamiento_.y *= INERCIA_DEC;
        if (std::abs(vel_desplazamiento_.x) > 0.1f || std::abs(vel_desplazamiento_.y) > 0.1f) {
            desplazamiento.x += vel_desplazamiento_.x;
            desplazamiento.y += vel_desplazamiento_.y;
        }
    }

    // ── Fondo ──
    draw->AddRectFilled(pos, ImVec2(pos.x + tam.x, pos.y + tam.y), ImGui::GetColorU32(ImGuiCol_WindowBg));

    // Definir el origen real del lienzo dentro del margen de la regla
    ImVec2 canvas_orig(pos.x + 25, pos.y + 25);
    bool hover_canvas = hover && (ImGui::GetIO().MousePos.x > canvas_orig.x) && (ImGui::GetIO().MousePos.y > canvas_orig.y);
    if (hover_canvas) {
        coord_raton.x = (ImGui::GetIO().MousePos.x - canvas_orig.x - desplazamiento.x) / zoom;
        coord_raton.y = (ImGui::GetIO().MousePos.y - canvas_orig.y - desplazamiento.y) / zoom;
    }

    // ── Push Clip Rect para aislar el canvas de las reglas ──
    ImGui::PushClipRect(canvas_orig, ImVec2(pos.x + tam.x, pos.y + tam.y), true);

    // ── Cuadrícula CAD (estilo HSC) ──
    float grilla = GRILLA * zoom;
    ImU32 col_g = ImGui::GetColorU32(ImGuiCol_Border, 0.3f);
    ImU32 col_gf = IM_COL32(195, 195, 205, 255);
    float ox = fmod(desplazamiento.x + 25, grilla);
    float oy = fmod(desplazamiento.y + 25, grilla);
    if (grilla > 4) {
        for (float x = ox; x < tam.x; x += grilla) {
            if (pos.x + x < canvas_orig.x) continue;
            bool f = fmod((x - ox) / grilla, 5) < 1;
            draw->AddLine(ImVec2(pos.x + x, canvas_orig.y), ImVec2(pos.x + x, pos.y + tam.y), f ? col_gf : col_g);
        }
        for (float y = oy; y < tam.y; y += grilla) {
            if (pos.y + y < canvas_orig.y) continue;
            bool f = fmod((y - oy) / grilla, 5) < 1;
            draw->AddLine(ImVec2(canvas_orig.x, pos.y + y), ImVec2(pos.x + tam.x, pos.y + y), f ? col_gf : col_g);
        }
    }

    // ── Anotaciones (debajo de los nodos) ──
    renderizar_anotaciones(canvas_orig);

    // ── Tablas de flujo ──
    renderizar_tablas_flujo(canvas_orig);

    // ── Nodos y flujos ──
    renderizar_flujos(canvas_orig);
    for (auto& nodo : flowsheet_->nodos()) {
        if (nodo.pagina != pagina_actual) continue;
        if (nodo.tipo == nucleo::TipoNodo::CajaTexto ||
            nodo.tipo == nucleo::TipoNodo::Rectangulo ||
            nodo.tipo == nucleo::TipoNodo::Elipse) continue;
        renderizar_nodo(nodo, canvas_orig);
    }

    if (arrastrando_conexion_)
        renderizar_conexion_arrastrada(canvas_orig);

    // ── Pop Clip Rect del canvas ──
    ImGui::PopClipRect();

    // ── Reglas de Medición (X e Y) al estilo HSC Sim ──
    ImU32 col_regla_bg = ImGui::GetColorU32(ImGuiCol_MenuBarBg);
    ImU32 col_regla_borde = IM_COL32(180, 180, 190, 255);
    ImU32 col_regla_marca = IM_COL32(100, 100, 110, 255);
    ImU32 col_regla_texto = IM_COL32(50, 50, 60, 255);

    // Esquina superior izquierda (intersección)
    draw->AddRectFilled(pos, canvas_orig, col_regla_bg);
    draw->AddRect(pos, canvas_orig, col_regla_borde);

    // Regla Horizontal (X)
    draw->AddRectFilled(ImVec2(canvas_orig.x, pos.y), ImVec2(pos.x + tam.x, canvas_orig.y), col_regla_bg);
    draw->AddLine(ImVec2(canvas_orig.x, canvas_orig.y), ImVec2(pos.x + tam.x, canvas_orig.y), col_regla_borde, 1.5f);

    // Regla Vertical (Y)
    draw->AddRectFilled(ImVec2(pos.x, canvas_orig.y), ImVec2(canvas_orig.x, pos.y + tam.y), col_regla_bg);
    draw->AddLine(ImVec2(canvas_orig.x, canvas_orig.y), ImVec2(canvas_orig.x, pos.y + tam.y), col_regla_borde, 1.5f);

    // Ticks regla X
    float canvas_x_min = (- desplazamiento.x) / zoom;
    float canvas_x_max = (tam.x - 25 - desplazamiento.x) / zoom;
    int start_val_x = ((int)(canvas_x_min / 10)) * 10 - 10;
    int end_val_x = ((int)(canvas_x_max / 10)) * 10 + 10;

    for (int val = start_val_x; val <= end_val_x; val += 10) {
        if (val < 0) continue;
        float sx = canvas_orig.x + (val * zoom) + desplazamiento.x;
        if (sx < canvas_orig.x || sx > pos.x + tam.x) continue;

        bool major = (val % 50 == 0);
        float h_tick = major ? 12.0f : 6.0f;
        draw->AddLine(ImVec2(sx, canvas_orig.y - h_tick), ImVec2(sx, canvas_orig.y), col_regla_marca, 1.0f);

        if (major) {
            char buf[16];
            snprintf(buf, sizeof(buf), "%d", val);
            draw->AddText(ImVec2(sx + 2, pos.y + 2), col_regla_texto, buf);
        }
    }

    // Ticks regla Y
    float canvas_y_min = (- desplazamiento.y) / zoom;
    float canvas_y_max = (tam.y - 25 - desplazamiento.y) / zoom;
    int start_val_y = ((int)(canvas_y_min / 10)) * 10 - 10;
    int end_val_y = ((int)(canvas_y_max / 10)) * 10 + 10;

    for (int val = start_val_y; val <= end_val_y; val += 10) {
        if (val < 0) continue;
        float sy = canvas_orig.y + (val * zoom) + desplazamiento.y;
        if (sy < canvas_orig.y || sy > pos.y + tam.y) continue;

        bool major = (val % 50 == 0);
        float w_tick = major ? 12.0f : 6.0f;
        draw->AddLine(ImVec2(canvas_orig.x - w_tick, sy), ImVec2(canvas_orig.x, sy), col_regla_marca, 1.0f);

        if (major) {
            char buf[16];
            snprintf(buf, sizeof(buf), "%d", val);
            draw->AddText(ImVec2(pos.x + 2, sy + 2), col_regla_texto, buf);
        }
    }

    // ── Overlays ──
    renderizar_overlay_zoom(pos, tam);
    if (mostrar_minimapa)
        renderizar_minimapa(pos, tam);

    // ── Aviso de conexión rechazada ──
    if (!error_conexion_.empty() && tiempo_error_conexion_ > 0.0f) {
        tiempo_error_conexion_ -= ImGui::GetIO().DeltaTime;
        ImDrawList* draw = ImGui::GetWindowDrawList();
        ImVec2 size = ImGui::CalcTextSize(error_conexion_.c_str());
        ImVec2 p0(pos.x + 16, pos.y + 16);
        draw->AddRectFilled(ImVec2(p0.x - 10, p0.y - 6),
                            ImVec2(p0.x + size.x + 10, p0.y + size.y + 8),
                            IM_COL32(90, 20, 20, 230), 6.0f);
        draw->AddText(p0, IM_COL32(255, 210, 210, 255), error_conexion_.c_str());
        if (tiempo_error_conexion_ <= 0.0f) error_conexion_.clear();
    }

    // ── Menú contextual ──
    if (hover_canvas && ImGui::IsMouseClicked(ImGuiMouseButton_Right))
        ImGui::OpenPopup("ctx_lienzo");
    renderizar_menu_contextual(pos);

    // ── Drop desde herramientas ──
    if (ImGui::BeginDragDropTarget()) {
        if (const ImGuiPayload* pl = ImGui::AcceptDragDropPayload("tipo_nodo")) {
            guardar_estado();
            nucleo::TipoNodo tipo;
            memcpy(&tipo, pl->Data, sizeof(tipo));
            ImVec2 raton = ImGui::GetIO().MousePos;
            float nx = (raton.x - pos.x - 25 - desplazamiento.x - NODO_ANCHO * zoom * 0.5f) / zoom;
            float ny = (raton.y - pos.y - 25 - desplazamiento.y - NODO_ALTO * zoom * 0.5f) / zoom;
            int64_t id = flowsheet_->agregar_nodo(tipo);
            auto* n = flowsheet_->nodo_por_id(id);
            if (n) {
                n->nombre = std::string(nucleo::nombre_tipo_nodo(tipo)) + " #" + std::to_string(id);
                if (ajustar_grilla) {
                    n->canvas_x = round(nx / GRILLA) * GRILLA;
                    n->canvas_y = round(ny / GRILLA) * GRILLA;
                } else {
                    n->canvas_x = nx;
                    n->canvas_y = ny;
                }
                n->pagina = pagina_actual;
            }
            id_nodo_seleccionado = id;
            id_flujo_seleccionado = -1;
            id_anotacion_seleccionada = -1;
            if (al_seleccionar_nodo) al_seleccionar_nodo(id);
        }
        ImGui::EndDragDropTarget();
    }

    // ── Creación de Dibujos (Anotaciones) ──
    if (hover_canvas && ImGui::IsMouseClicked(ImGuiMouseButton_Left) && !arrastrando_conexion_ && !creando_anotacion_ &&
        (modo_dibujo == ModoDibujo::Texto || modo_dibujo == ModoDibujo::Rectangulo || modo_dibujo == ModoDibujo::Elipse)) {
        
        ImVec2 raton = ImGui::GetIO().MousePos;
        bool sobre_nodo = false;
        for (auto& nodo : flowsheet_->nodos()) {
            if (nodo.pagina != pagina_actual) continue;
            ImVec2 np(canvas_orig.x + nodo.canvas_x + desplazamiento.x,
                      canvas_orig.y + nodo.canvas_y + desplazamiento.y);
            ImVec2 ns(NODO_ANCHO * zoom, NODO_ALTO * zoom);
            if (hover_rect(raton, np, ImVec2(np.x + ns.x, np.y + ns.y))) {
                sobre_nodo = true;
                break;
            }
        }
        
        if (!sobre_nodo) {
            float cx = (raton.x - canvas_orig.x - desplazamiento.x) / zoom;
            float cy = (raton.y - canvas_orig.y - desplazamiento.y) / zoom;
            
            guardar_estado();
            std::string default_text = (modo_dibujo == ModoDibujo::Texto) ? "Caja de Texto" :
                                       (modo_dibujo == ModoDibujo::Rectangulo) ? "Rectángulo" : "Elipse";
            int64_t aid = flowsheet_->agregar_anotacion(default_text);
            for (auto& a : flowsheet_->anotaciones()) {
                if (a.id == aid) {
                    a.x = cx;
                    a.y = cy;
                    a.w = 10.0f;
                    a.h = 10.0f;
                    a.pagina = pagina_actual;
                    a.tipo = (modo_dibujo == ModoDibujo::Texto) ? diagrama_flujo::Anotacion::Tipo::Texto :
                             (modo_dibujo == ModoDibujo::Rectangulo) ? diagrama_flujo::Anotacion::Tipo::Rectangulo :
                             diagrama_flujo::Anotacion::Tipo::Elipse;
                    
                    if (modo_dibujo == ModoDibujo::Rectangulo) {
                        a.color_fondo = 0x3C3C4A80; // Gris semi-transparente
                        a.color_borde = 0x5C9CFFC8; // Borde azul claro
                    } else if (modo_dibujo == ModoDibujo::Elipse) {
                        a.color_fondo = 0x4A3C4A80; // Púrpura semi-transparente
                        a.color_borde = 0xFF9C5FC8; // Borde púrpura
                    } else {
                        a.color_fondo = 0x1A1A24C8; // Fondo oscuro para texto
                        a.color_borde = 0x64648CC8;
                    }
                    break;
                }
            }
            
            id_anotacion_seleccionada = aid;
            id_nodo_seleccionado = -1;
            id_flujo_seleccionado = -1;
            
            creando_anotacion_ = true;
            id_anotacion_creando_ = aid;
            inicio_creacion_anotacion_ = ImVec2(cx, cy);
            
            if (al_seleccionar_anotacion) al_seleccionar_anotacion(aid);
            if (al_crear_anotacion) al_crear_anotacion(aid);
        }
    }

    // ── Actualizar redimensionamiento durante creación ──
    if (creando_anotacion_) {
        float cur_cx = (ImGui::GetIO().MousePos.x - canvas_orig.x - desplazamiento.x) / zoom;
        float cur_cy = (ImGui::GetIO().MousePos.y - canvas_orig.y - desplazamiento.y) / zoom;
        
        float x0 = std::min(inicio_creacion_anotacion_.x, cur_cx);
        float y0 = std::min(inicio_creacion_anotacion_.y, cur_cy);
        float w = std::abs(cur_cx - inicio_creacion_anotacion_.x);
        float h = std::abs(cur_cy - inicio_creacion_anotacion_.y);
        
        if (ajustar_grilla) {
            x0 = roundf(x0 / GRILLA) * GRILLA;
            y0 = roundf(y0 / GRILLA) * GRILLA;
            w = std::max(GRILLA, roundf(w / GRILLA) * GRILLA);
            h = std::max(GRILLA, roundf(h / GRILLA) * GRILLA);
        } else {
            w = std::max(10.0f, w);
            h = std::max(10.0f, h);
        }
        
        for (auto& a : flowsheet_->anotaciones()) {
            if (a.id == id_anotacion_creando_) {
                a.x = x0;
                a.y = y0;
                a.w = w;
                a.h = h;
                break;
            }
        }
        
        if (!ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
            creando_anotacion_ = false;
            id_anotacion_creando_ = -1;
            if (!persistir_herramienta) modo_dibujo = ModoDibujo::Seleccion;
        }
    }
    // ── Deseleccionar en Selección Normal ──
    else if (hover_canvas && ImGui::IsMouseClicked(ImGuiMouseButton_Left) && !arrastrando_conexion_ && !clic_detectado_en_objeto_) {
        id_nodo_seleccionado = -1;
        id_flujo_seleccionado = -1;
        id_anotacion_seleccionada = -1;
        if (al_seleccionar_nodo) al_seleccionar_nodo(-1);
        if (al_seleccionar_anotacion) al_seleccionar_anotacion(-1);
    }

    // ── Atajos ──
    if (hover) {
        if (ImGui::IsKeyPressed(ImGuiKey_F)) encajar_todo();
        if (ImGui::IsKeyPressed(ImGuiKey_M)) mostrar_minimapa = !mostrar_minimapa;
        if (ImGui::IsKeyPressed(ImGuiKey_G)) ajustar_grilla = !ajustar_grilla;
    }

    // ── Eliminación diferida de nodos ──
    for (int64_t id : nodos_a_eliminar_) {
        if (flowsheet_) flowsheet_->eliminar_nodo(id);
        puertos_entrada_.erase(id);
        puertos_salida_.erase(id);
    }
    nodos_a_eliminar_.clear();
}

// ═══════════════════════════════════════════════════════════════════════════
// Renderizar Nodo
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_nodo(nucleo::Nodo& nodo, const ImVec2& pos) {
    ImDrawList* draw = ImGui::GetWindowDrawList();
    float z = zoom;

    ImVec2 npos(pos.x + nodo.canvas_x + desplazamiento.x,
                pos.y + nodo.canvas_y + desplazamiento.y);
    ImVec2 nsz(NODO_ANCHO * z, NODO_ALTO * z);
    float cab_h = CABECERA_H * z;
    float r = RADIO * z;

    ImU32 color_cab = col_cab(nodo.tipo);
    bool sel = (id_nodo_seleccionado == nodo.id);
    ImU32 col_borde = sel ? IM_COL32(255, 210, 60, 255) : ImGui::GetColorU32(ImGuiCol_Border, 0.7f);
    float bw = sel ? 3.0f * z : 1.5f * z;

    // ── Sombra, Cabecera, Cuerpo, Barra y Borde removidos a petición del usuario ──
    // Se conserva únicamente el dibujo (modelo) y el texto.
    if (sel) {
        draw->AddRect(npos, ImVec2(npos.x + nsz.x, npos.y + nsz.y), col_borde, r, 0, bw);
    }

    // ── Nombre del tipo en cabecera ──
    std::string tstr_s = sanitizar_texto(nucleo::nombre_tipo_nodo(nodo.tipo));
    const char* tstr = tstr_s.c_str();
    draw->AddText(ImVec2(npos.x + 6*z, npos.y + 4*z), ImGui::GetColorU32(ImGuiCol_Text), tstr);

    // ── ID pequeño en la cabecera ──
    char idbuf[16];
    snprintf(idbuf, sizeof(idbuf), "#%lld", (long long)nodo.id);
    ImVec2 idtsz = ImGui::CalcTextSize(idbuf);
    draw->AddText(ImVec2(npos.x + nsz.x - idtsz.x - 4*z, npos.y + 4*z),
                  ImGui::GetColorU32(ImGuiCol_Text, 0.7f), idbuf);

    // ── Modelo ──
    ImVec2 cm(npos.x + nsz.x*0.5f, npos.y + cab_h + nsz.y*0.5f);
    dibujar_modelo(draw, cm, nsz.y*0.85f, nodo.tipo);

    // ── Nombre del nodo ──
    std::string etiq = nodo.nombre.empty() ? "" : sanitizar_texto(nodo.nombre);
    if (!etiq.empty()) {
        ImU32 col_etiq = ImGui::GetColorU32(ImGuiCol_Text, 0.7f);
        ImVec2 etsz = ImGui::CalcTextSize(etiq.c_str());
        draw->AddText(ImVec2(npos.x + (nsz.x - etsz.x)*0.5f, npos.y + nsz.y - etsz.y - 3*z),
                      col_etiq, etiq.c_str());
    }

    // ── Puertos ──
    calcular_posicion_puertos(nodo, npos, nsz);
    for (auto& p : puertos_entrada_[nodo.id]) {
        draw->AddCircleFilled(p, RADIO_PTO*z, IM_COL32(80,180,255,255));
        draw->AddCircle(p, RADIO_PTO*z, IM_COL32(200,230,255,200), 0, 1.5f*z);
    }
    for (auto& p : puertos_salida_[nodo.id]) {
        draw->AddCircleFilled(p, RADIO_PTO*z, IM_COL32(255,170,80,255));
        draw->AddCircle(p, RADIO_PTO*z, IM_COL32(255,220,180,200), 0, 1.5f*z);
    }

    // ── Interacción ──
    ImVec2 mp = ImGui::GetIO().MousePos;
    bool hn = hover_rect(mp, npos, ImVec2(npos.x+nsz.x, npos.y+nsz.y));

    if (hn && clic_en_lienzo_) {
        id_nodo_seleccionado = nodo.id;
        id_flujo_seleccionado = -1;
        id_anotacion_seleccionada = -1;
        if (al_seleccionar_nodo) al_seleccionar_nodo(nodo.id);
        nodo_arrastrando_ = nodo.id;
        clic_detectado_en_objeto_ = true;
    }

    if (nodo_arrastrando_ == nodo.id) {
        if (ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
            if (ImGui::IsMouseDragging(ImGuiMouseButton_Left)) {
                extern bool g_nodo_arrastrado;
                if (!g_nodo_arrastrado) {
                    guardar_estado();
                    g_nodo_arrastrado = true;
                }
                if (modo_dibujo == ModoDibujo::DibujarFlujo) {
                    if (!arrastrando_conexion_ && !puertos_salida_[nodo.id].empty()) {
                        arrastrando_conexion_ = true;
                        nodo_origen_ = nodo.id;
                        inicio_arrastre_ = puertos_salida_[nodo.id][0];
                        actual_arrastre_ = ImGui::GetIO().MousePos;
                        nodo_arrastrando_ = -1;
                        clic_detectado_en_objeto_ = true;
                    }
                } else {
                    float dx = ImGui::GetIO().MouseDelta.x / zoom;
                    float dy = ImGui::GetIO().MouseDelta.y / zoom;
                    nodo.canvas_x += dx;
                    nodo.canvas_y += dy;
                }
            }
        } else {
            if (ajustar_grilla) {
                float g = GRILLA;
                nodo.canvas_x = round(nodo.canvas_x / g) * g;
                nodo.canvas_y = round(nodo.canvas_y / g) * g;
            }
            nodo_arrastrando_ = -1;
        }
    }

    // Conexión desde puerto
    if (hn && clic_en_lienzo_ && !arrastrando_conexion_) {
        for (auto& p : puertos_salida_[nodo.id]) {
            float dist = std::sqrt(std::pow(ImGui::GetIO().MousePos.x-p.x,2)+
                                   std::pow(ImGui::GetIO().MousePos.y-p.y,2));
            if (dist < 12*z) {
                arrastrando_conexion_ = true;
                nodo_origen_ = nodo.id;
                inicio_arrastre_ = p;
                actual_arrastre_ = ImGui::GetIO().MousePos;
                nodo_arrastrando_ = -1;
                clic_detectado_en_objeto_ = true;
                break;
            }
        }
    }

    // Botón X funcional
    if (hn) {
        ImVec2 bx(npos.x+nsz.x-18*z, npos.y+2*z);
        ImVec2 bt(16*z, 16*z);
        bool hover_x = hover_rect(mp, bx, ImVec2(bx.x+bt.x, bx.y+bt.y));
        ImU32 col_x = hover_x ? IM_COL32(255,80,80,255) : IM_COL32(200,50,50,200);
        draw->AddRectFilled(bx, ImVec2(bx.x+bt.x, bx.y+bt.y), col_x, 3*z);
        draw->AddText(ImVec2(bx.x+4*z, bx.y+1*z), IM_COL32(255,255,255,255), "x");
        if (hover_x && clic_en_lienzo_) {
            nodos_a_eliminar_.push_back(nodo.id);
            hn = false;
        }
    }

    // Indicador de estado (punto verde/amarillo)
    {
        ImVec2 centro_estado(npos.x + nsz.x - 8*z, npos.y + nsz.y - 8*z);
        bool tiene_entradas = !nodo.ids_flujos_entrada.empty();
        bool tiene_salidas = !nodo.ids_flujos_salida.empty();
        ImU32 col_estado;
        if (tiene_entradas && tiene_salidas)
            col_estado = IM_COL32(60, 200, 80, 255); // verde: conectado
        else if (tiene_entradas || tiene_salidas)
            col_estado = IM_COL32(230, 180, 40, 255); // amarillo: parcialmente conectado
        else
            col_estado = IM_COL32(200, 80, 80, 255); // rojo: sin conexiones
        draw->AddCircleFilled(centro_estado, 4*z, col_estado);
    }

    // Tooltip
    if (hn && ImGui::IsItemHovered()) {
        ImGui::BeginTooltip();
        ImGui::Text("%s", nucleo::nombre_tipo_nodo(nodo.tipo));
        ImGui::Text("ID: %lld", (long long)nodo.id);
        if (!nodo.nombre.empty())
            ImGui::Text("Nombre: %s", nodo.nombre.c_str());
        ImGui::Text("Entradas: %d  Salidas: %d", nodo.num_entradas(), nodo.num_salidas());

        // Descripción del tipo de equipo
        const char* desc = descripcion_tipo_nodo(nodo.tipo);
        if (desc) {
            ImGui::Separator();
            ImGui::TextDisabled("%s", desc);
        }
        ImGui::EndTooltip();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Puertos
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::calcular_posicion_puertos(const nucleo::Nodo& nodo, const ImVec2& npos, const ImVec2& nsz) {
    auto& in = puertos_entrada_[nodo.id];
    auto& out = puertos_salida_[nodo.id];
    in.clear(); out.clear();

    int n_in = nodo.num_entradas();
    int n_out = nodo.num_salidas();

    // Default minimum ports for empty nodes
    int min_in = 1;
    int min_out = 1;

    if (nodo.tipo == nucleo::TipoNodo::Fuente) {
        min_in = 0;
        n_in = 0;
    }
    if (nodo.tipo == nucleo::TipoNodo::Sumidero) {
        min_out = 0;
        n_out = 0;
    }
    if (nodo.tipo == nucleo::TipoNodo::Mezclador) {
        min_in = 3; // Mixer starts with at least 3 input slots
    }
    if (nodo.tipo == nucleo::TipoNodo::Divisor) {
        min_out = 2; // Splitter starts with at least 2 output slots
    }

    // Always keep one extra free port slot for new connections
    n_in = std::max(n_in + 1, min_in);
    if (nodo.tipo == nucleo::TipoNodo::Fuente) n_in = 0;

    n_out = std::max(n_out + 1, min_out);
    if (nodo.tipo == nucleo::TipoNodo::Sumidero) n_out = 0;

    float cab_h = 24.0f * zoom;
    float center_y = npos.y + cab_h + nsz.y * 0.5f;
    float s_y = nsz.y * 0.85f;
    float start_y = center_y - s_y * 0.5f;
    for (int i = 0; i < n_in; ++i) {
        float y = start_y + (s_y * (i + 1)) / (n_in + 1);
        in.push_back(ImVec2(npos.x, y));
    }
    for (int i = 0; i < n_out; ++i) {
        float y = start_y + (s_y * (i + 1)) / (n_out + 1);
        out.push_back(ImVec2(npos.x + nsz.x, y));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Flujos
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_flujos(const ImVec2& pos) {
    if (!flowsheet_) return;
    ImDrawList* draw = ImGui::GetWindowDrawList();

    for (auto& [fid, flujo] : flowsheet_->mapa_flujos()) {
        if (flujo.pagina != pagina_actual) continue;
        ImVec2 inicio(0,0), fin(0,0);
        bool encontrado = false;

        for (auto& nodo : flowsheet_->nodos()) {
            for (int64_t sid : nodo.ids_flujos_salida)
                if (sid == fid) {
                    auto it = puertos_salida_.find(nodo.id);
                    if (it != puertos_salida_.end() && !it->second.empty()) {
                        size_t idx = 0;
                        for(size_t k=0; k<nodo.ids_flujos_salida.size(); ++k) if(nodo.ids_flujos_salida[k] == fid) idx = k;
                        if (idx < it->second.size()) inicio = it->second[idx];
                        else inicio = it->second.back();
                    }
                }
            for (int64_t sid : nodo.ids_flujos_entrada)
                if (sid == fid) {
                    auto it = puertos_entrada_.find(nodo.id);
                    if (it != puertos_entrada_.end() && !it->second.empty()) {
                        size_t idx = 0;
                        for(size_t k=0; k<nodo.ids_flujos_entrada.size(); ++k) if(nodo.ids_flujos_entrada[k] == fid) idx = k;
                        if (idx < it->second.size()) fin = it->second[idx];
                        else fin = it->second.back();
                        encontrado = true;
                    }
                }
        }

        if (!encontrado) continue;

        // ── Energy stream (rojo punteado) ──
        if (flujo.es_energia) {
            renderizar_flujo_energia(flujo, inicio, fin, fid);
            continue;
        }

        float dx = (fin.x-inicio.x)*0.4f;
        ImVec2 cp1(inicio.x+dx, inicio.y), cp2(fin.x-dx, fin.y);

        // Flecha
        float tf = 0.95f;
        ImVec2 fp((1-tf)*(1-tf)*(1-tf)*inicio.x + 3*(1-tf)*(1-tf)*tf*cp1.x + 3*(1-tf)*tf*tf*cp2.x + tf*tf*tf*fin.x,
                  (1-tf)*(1-tf)*(1-tf)*inicio.y + 3*(1-tf)*(1-tf)*tf*cp1.y + 3*(1-tf)*tf*tf*cp2.y + tf*tf*tf*fin.y);
        ImVec2 dir(fin.x-fp.x, fin.y-fp.y);
        float len = std::sqrt(dir.x*dir.x+dir.y*dir.y);
        if (len > 0) { dir.x/=len; dir.y/=len; }
        ImVec2 perp(-dir.y, dir.x);
        float fs = 8*zoom;
        draw->AddTriangleFilled(fin,
            ImVec2(fin.x-dir.x*fs+perp.x*fs*0.5f, fin.y-dir.y*fs+perp.y*fs*0.5f),
            ImVec2(fin.x-dir.x*fs-perp.x*fs*0.5f, fin.y-dir.y*fs-perp.y*fs*0.5f),
            IM_COL32(170,170,190,200));

        float flujo_total = flujo.flujo_total();
        float base_ancho = 1.8f + std::min(flujo_total * 0.5f, 15.0f);
        float ancho = (id_flujo_seleccionado==fid) ? (base_ancho + 2.0f)*zoom : base_ancho*zoom;

        ImU32 col_c;
        if (id_flujo_seleccionado == fid) {
            col_c = IM_COL32(255, 210, 60, 240);
        } else {
            float factor = std::min(flujo_total / 20.0f, 1.0f);
            int r = (int)(140 - factor * 90);
            int g = (int)(145 + factor * 70);
            int b = (int)(165 + factor * 90);
            col_c = IM_COL32(r, g, b, 180);
        }
        draw->AddBezierCubic(inicio, cp1, cp2, fin, col_c, ancho);

        // Partículas animadas a lo largo del flujo
        if (flujo_total > 0.001f) {
            float tiempo = ImGui::GetTime();
            float vel = 0.3f + flujo_total * 0.1f;
            int n_particulas = 3 + (int)(flujo_total * 2);
            if (n_particulas > 12) n_particulas = 12;
            for (int p = 0; p < n_particulas; p++) {
                float t = fmod(tiempo * vel + (float)p / (float)n_particulas, 1.0f);
                ImVec2 pp((1-t)*(1-t)*(1-t)*inicio.x + 3*(1-t)*(1-t)*t*cp1.x + 3*(1-t)*t*t*cp2.x + t*t*t*fin.x,
                          (1-t)*(1-t)*(1-t)*inicio.y + 3*(1-t)*(1-t)*t*cp1.y + 3*(1-t)*t*t*cp2.y + t*t*t*fin.y);
                float alfa = (t < 0.1f) ? t * 10 : (t > 0.9f) ? (1 - t) * 10 : 1.0f;
                draw->AddCircleFilled(pp, 3*zoom, IM_COL32(100,200,255,(int)(80*alfa)));
            }
        }

        // Etiqueta de Corriente (HSC Style: Nombre [Value])
        {
            float t = 0.5f;
            ImVec2 mid((1-t)*(1-t)*(1-t)*inicio.x + 3*(1-t)*(1-t)*t*cp1.x + 3*(1-t)*t*t*cp2.x + t*t*t*fin.x,
                       (1-t)*(1-t)*(1-t)*inicio.y + 3*(1-t)*(1-t)*t*cp1.y + 3*(1-t)*t*t*cp2.y + t*t*t*fin.y);
            
            char etiq[96];
            std::string name = flujo.nombre.empty() ? ("Flujo " + std::to_string(fid)) : flujo.nombre;
            if (flujo_total > 0.0001f) {
                snprintf(etiq, sizeof(etiq), "%s [%.2f kmol/s]", name.c_str(), flujo_total);
            } else {
                snprintf(etiq, sizeof(etiq), "%s [Value]", name.c_str());
            }

            ImVec2 etsz = ImGui::CalcTextSize(etiq);
            // Draw background badge (white with grey outline, style HSC Sim)
            draw->AddRectFilled(ImVec2(mid.x-etsz.x*0.5f-5*zoom, mid.y-etsz.y*0.5f-3*zoom),
                                ImVec2(mid.x+etsz.x*0.5f+5*zoom, mid.y+etsz.y*0.5f+3*zoom),
                                IM_COL32(255, 255, 255, 240), 3 * zoom);
            draw->AddRect(ImVec2(mid.x-etsz.x*0.5f-5*zoom, mid.y-etsz.y*0.5f-3*zoom),
                          ImVec2(mid.x+etsz.x*0.5f+5*zoom, mid.y+etsz.y*0.5f+3*zoom),
                          IM_COL32(180, 180, 190, 200), 3 * zoom, 0, 1.0f * zoom);
            
            draw->AddText(ImVec2(mid.x-etsz.x*0.5f, mid.y-etsz.y*0.5f),
                          IM_COL32(40, 40, 50, 255), etiq);
        }

        // Hit test
        for (float tt = 0; tt <= 1; tt += 0.02f) {
            ImVec2 pt((1-tt)*(1-tt)*(1-tt)*inicio.x + 3*(1-tt)*(1-tt)*tt*cp1.x + 3*(1-tt)*tt*tt*cp2.x + tt*tt*tt*fin.x,
                      (1-tt)*(1-tt)*(1-tt)*inicio.y + 3*(1-tt)*(1-tt)*tt*cp1.y + 3*(1-tt)*tt*tt*cp2.y + tt*tt*tt*fin.y);
            float d = std::sqrt(std::pow(ImGui::GetIO().MousePos.x-pt.x,2)+
                                std::pow(ImGui::GetIO().MousePos.y-pt.y,2));
            if (d < 6 && clic_en_lienzo_) {
                id_flujo_seleccionado = fid;
                id_nodo_seleccionado = -1;
                clic_detectado_en_objeto_ = true;
                if (al_seleccionar_flujo) al_seleccionar_flujo(fid);
                break;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Conexión arrastrada
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_conexion_arrastrada(const ImVec2& pos) {
    ImDrawList* draw = ImGui::GetWindowDrawList();
    bool sobre = false;
    int64_t dest = -1;
    // Primero buscar si pasa sobre un puerto de entrada
    for (auto& nodo : flowsheet_->nodos()) {
        if (nodo.id == nodo_origen_) continue;
        auto it = puertos_entrada_.find(nodo.id);
        if (it == puertos_entrada_.end()) continue;
        for (auto& p : it->second) {
            float d = std::sqrt(std::pow(ImGui::GetIO().MousePos.x-p.x,2)+
                                std::pow(ImGui::GetIO().MousePos.y-p.y,2));
            if (d < 15*zoom) { sobre=true; actual_arrastre_=p; dest=nodo.id; break; }
        }
        if (sobre) break;
    }
    // Si no sobre un puerto, ver si pasa sobre el cuerpo de algún nodo (más amigable)
    if (!sobre) {
        for (auto& nodo : flowsheet_->nodos()) {
            if (nodo.id == nodo_origen_) continue;
            float z = zoom;
            ImVec2 npos(pos.x + nodo.canvas_x + desplazamiento.x,
                        pos.y + nodo.canvas_y + desplazamiento.y);
            ImVec2 nsz(NODO_ANCHO * z, NODO_ALTO * z);
            ImVec2 m = ImGui::GetIO().MousePos;
            if (hover_rect(m, npos, ImVec2(npos.x + nsz.x, npos.y + nsz.y))) {
                auto it = puertos_entrada_.find(nodo.id);
                if (it != puertos_entrada_.end() && !it->second.empty()) {
                    sobre = true;
                    actual_arrastre_ = it->second[0]; // Snap al primer puerto de entrada
                    dest = nodo.id;
                    break;
                }
            }
        }
    }
    if (!sobre) actual_arrastre_ = ImGui::GetIO().MousePos;

    float dx = (actual_arrastre_.x-inicio_arrastre_.x)*0.4f;
    ImVec2 cp1(inicio_arrastre_.x+dx, inicio_arrastre_.y);
    ImVec2 cp2(actual_arrastre_.x-dx, actual_arrastre_.y);

    ImU32 col = sobre ? IM_COL32(100,255,100,220) : IM_COL32(255,255,80,200);
    draw->AddBezierCubic(inicio_arrastre_, cp1, cp2, actual_arrastre_, col, 2.5f*zoom);

    float ta = fmod(ImGui::GetTime()*2.5f,1.0f);
    ImVec2 pulso((1-ta)*(1-ta)*(1-ta)*inicio_arrastre_.x + 3*(1-ta)*(1-ta)*ta*cp1.x + 3*(1-ta)*ta*ta*cp2.x + ta*ta*ta*actual_arrastre_.x,
                 (1-ta)*(1-ta)*(1-ta)*inicio_arrastre_.y + 3*(1-ta)*(1-ta)*ta*cp1.y + 3*(1-ta)*ta*ta*cp2.y + ta*ta*ta*actual_arrastre_.y);
    draw->AddCircleFilled(pulso, 4*zoom, IM_COL32(255,255,200,220));

    if (!ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
        if (sobre && dest > 0) {
            std::string motivo;
            if (flowsheet_->conexion_valida(nodo_origen_, dest, &motivo)) {
                guardar_estado();
                flowsheet_->conectar(nodo_origen_, dest);
            } else {
                mostrar_error_conexion(motivo);
            }
        }
        arrastrando_conexion_ = false;
        nodo_origen_ = -1;
        if (!persistir_herramienta) modo_dibujo = ModoDibujo::Seleccion;
    }
}

void Lienzo::mostrar_error_conexion(const std::string& msg) {
    error_conexion_ = msg;
    tiempo_error_conexion_ = 5.0f;
}

// ═══════════════════════════════════════════════════════════════════════════
// Minimapa
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_minimapa(const ImVec2& pos, const ImVec2& tam) {
    if (!flowsheet_ || flowsheet_->nodos().empty()) return;

    ImDrawList* draw = ImGui::GetWindowDrawList();
    float mm_w = 150.0f, mm_h = 100.0f;
    float pad = 8.0f;
    ImVec2 mm0(pos.x+tam.x-mm_w-pad, pos.y+tam.y-mm_h-pad);
    ImVec2 mm1(mm0.x+mm_w, mm0.y+mm_h);

    draw->AddRectFilled(mm0, mm1, IM_COL32(15,15,20,200), 4);
    draw->AddRect(mm0, mm1, IM_COL32(60,60,70,150), 4);

    // Calcular bounds
    float xmin=1e9, ymin=1e9, xmax=-1e9, ymax=-1e9;
    for (auto& n : flowsheet_->nodos()) {
        xmin = std::min(xmin, n.canvas_x);
        ymin = std::min(ymin, n.canvas_y);
        xmax = std::max(xmax, n.canvas_x+NODO_ANCHO);
        ymax = std::max(ymax, n.canvas_y+NODO_ALTO);
    }
    float rw = xmax-xmin, rh = ymax-ymin;
    if (rw < 1) rw=1;
    if (rh < 1) rh=1;

    float esc = std::min(mm_w/rw, mm_h/rh)*0.9f;
    float cx = (mm0.x+mm1.x)*0.5f;
    float cy = (mm0.y+mm1.y)*0.5f;
    float ox = cx - (xmin+xmax)*0.5f*esc;
    float oy = cy - (ymin+ymax)*0.5f*esc;

    for (auto& n : flowsheet_->nodos()) {
        float nx = ox + n.canvas_x*esc;
        float ny = oy + n.canvas_y*esc;
        float nw = NODO_ANCHO*esc;
        float nh = NODO_ALTO*esc;
        ImU32 c = col_cab(n.tipo);
        if (n.id == id_nodo_seleccionado)
            draw->AddRectFilled(ImVec2(nx,ny), ImVec2(nx+nw,ny+nh), IM_COL32(255,210,60,200), 1);
        else
            draw->AddRectFilled(ImVec2(nx,ny), ImVec2(nx+nw,ny+nh), c, 1);
    }

    // Viewport
    float vx = ox - desplazamiento.x/zoom*esc;
    float vy = oy - desplazamiento.y/zoom*esc;
    float vw = tam.x/zoom*esc;
    float vh = tam.y/zoom*esc;
    draw->AddRect(ImVec2(vx,vy), ImVec2(vx+vw,vy+vh), IM_COL32(255,255,255,80), 1, 0, 1);

    // Cerrar botón
    ImVec2 cb(mm1.x-14, mm0.y+2);
    draw->AddText(cb, IM_COL32(180,180,200,150), "x");
    ImVec2 m = ImGui::GetIO().MousePos;
    
    // Interacción
    if (hover_rect(m, mm0, mm1)) {
        if (clic_en_lienzo_ && m.x > cb.x && m.x < cb.x+12 && m.y > cb.y && m.y < cb.y+12) {
            mostrar_minimapa = false;
        } else if (ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
            // Mover el desplazamiento para centrar la cámara en m
            float target_canvas_x = (m.x - ox) / esc;
            float target_canvas_y = (m.y - oy) / esc;
            
            desplazamiento.x = -(target_canvas_x * zoom) + tam.x * 0.5f;
            desplazamiento.y = -(target_canvas_y * zoom) + tam.y * 0.5f;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Overlay de zoom
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_overlay_zoom(const ImVec2& pos, const ImVec2& tam) {
    ImDrawList* draw = ImGui::GetWindowDrawList();

    // Zoom % arriba a la izquierda
    char buf[32];
    snprintf(buf, sizeof(buf), "Zoom: %.0f%% | Nodos: %zu | [F] Encajar | [M] Mapa | [G] Snap",
             zoom*100.0f, flowsheet_ ? flowsheet_->conteo_nodos() : 0);
    ImVec2 tsz = ImGui::CalcTextSize(buf);
    draw->AddRectFilled(ImVec2(pos.x+4, pos.y+4), ImVec2(pos.x+tsz.x+10, pos.y+tsz.y+8),
                        IM_COL32(15,15,20,180), 3);
    draw->AddText(ImVec2(pos.x+7, pos.y+5), IM_COL32(180,180,200,200), buf);
}

// ═══════════════════════════════════════════════════════════════════════════
// Menú contextual
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_menu_contextual(const ImVec2& pos) {
    if (ImGui::BeginPopup("ctx_lienzo")) {
        ImGui::Text(tr("Agregar Nodo")); ImGui::Separator();
        auto op = [&](const char* lbl, nucleo::TipoNodo t) {
            if (ImGui::MenuItem(lbl)) {
                if (al_solicitar_agregar_nodo) al_solicitar_agregar_nodo(t);
                ImGui::CloseCurrentPopup();
            }
        };
        op("Fuente             [F]", nucleo::TipoNodo::Fuente);
        op("Sumidero           [S]", nucleo::TipoNodo::Sumidero);
        op("Mezclador          [M]", nucleo::TipoNodo::Mezclador);
        op("Divisor            [D]", nucleo::TipoNodo::Divisor);
        op("Reactor            [R]", nucleo::TipoNodo::Reactor);
        op("Separador          [S]", nucleo::TipoNodo::Separador);
        op("Interc. Calor      [H]", nucleo::TipoNodo::IntercambiadorCalor);
        op("Col. Destilación   [C]", nucleo::TipoNodo::ColumnaDestilacion);
        op("Compresor          [C]", nucleo::TipoNodo::Compresor);
        ImGui::Separator();
        if (ImGui::MenuItem(tr("Eliminar Seleccionado"), "Supr")) {
            if (id_nodo_seleccionado > 0) { flowsheet_->eliminar_nodo(id_nodo_seleccionado); id_nodo_seleccionado=-1; }
            else if (id_flujo_seleccionado > 0) { flowsheet_->eliminar_flujo(id_flujo_seleccionado); id_flujo_seleccionado=-1; }
            else if (id_anotacion_seleccionada > 0) { flowsheet_->eliminar_anotacion(id_anotacion_seleccionada); id_anotacion_seleccionada=-1; }
        }
        if (ImGui::MenuItem(tr("Centrar Vista"), "F")) encajar_todo();
        ImGui::MenuItem(tr("Minimapa"), "M", &mostrar_minimapa);
        ImGui::MenuItem(tr("Ajustar a Grilla"), "G", &ajustar_grilla);
        ImGui::EndPopup();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Energy Stream (línea roja punteada)
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_flujo_energia(const nucleo::Flujo& flujo, const ImVec2& inicio,
                                      const ImVec2& fin, int64_t fid) {
    ImDrawList* draw = ImGui::GetWindowDrawList();
    float dx = (fin.x-inicio.x)*0.4f;
    ImVec2 cp1(inicio.x+dx, inicio.y), cp2(fin.x-dx, fin.y);

    ImU32 col = (id_flujo_seleccionado==fid) ? IM_COL32(255,100,60,255) : IM_COL32(220,40,40,180);

    // Línea punteada zigzagueante simulando energía
    int segmentos = 40;
    float ancho = 3.0f * zoom;
    for (int i = 0; i < segmentos; i++) {
        float t1 = (float)i / segmentos;
        float t2 = (float)(i+1) / segmentos;
        ImVec2 p1((1-t1)*(1-t1)*(1-t1)*inicio.x + 3*(1-t1)*(1-t1)*t1*cp1.x + 3*(1-t1)*t1*t1*cp2.x + t1*t1*t1*fin.x,
                  (1-t1)*(1-t1)*(1-t1)*inicio.y + 3*(1-t1)*(1-t1)*t1*cp1.y + 3*(1-t1)*t1*t1*cp2.y + t1*t1*t1*fin.y);
        ImVec2 p2((1-t2)*(1-t2)*(1-t2)*inicio.x + 3*(1-t2)*(1-t2)*t2*cp1.x + 3*(1-t2)*t2*t2*cp2.x + t2*t2*t2*fin.x,
                  (1-t2)*(1-t2)*(1-t2)*inicio.y + 3*(1-t2)*(1-t2)*t2*cp1.y + 3*(1-t2)*t2*t2*cp2.y + t2*t2*t2*fin.y);
        if (i % 3 == 0) continue; // efecto punteado
        draw->AddLine(p1, p2, col, ancho);
    }

    // Flecha
    float tf = 0.95f;
    ImVec2 fp((1-tf)*(1-tf)*(1-tf)*inicio.x + 3*(1-tf)*(1-tf)*tf*cp1.x + 3*(1-tf)*tf*tf*cp2.x + tf*tf*tf*fin.x,
              (1-tf)*(1-tf)*(1-tf)*inicio.y + 3*(1-tf)*(1-tf)*tf*cp1.y + 3*(1-tf)*tf*tf*cp2.y + tf*tf*tf*fin.y);
    ImVec2 dir(fin.x-fp.x, fin.y-fp.y);
    float len = std::sqrt(dir.x*dir.x+dir.y*dir.y);
    if (len > 0) { dir.x/=len; dir.y/=len; }
    ImVec2 perp(-dir.y, dir.x);
    float fs = 10*zoom;
    draw->AddTriangleFilled(fin,
        ImVec2(fin.x-dir.x*fs+perp.x*fs*0.5f, fin.y-dir.y*fs+perp.y*fs*0.5f),
        ImVec2(fin.x-dir.x*fs-perp.x*fs*0.5f, fin.y-dir.y*fs-perp.y*fs*0.5f),
        IM_COL32(255,60,60,200));

    // Etiqueta
    float t = 0.5f;
    ImVec2 mid((1-t)*(1-t)*(1-t)*inicio.x + 3*(1-t)*(1-t)*t*cp1.x + 3*(1-t)*t*t*cp2.x + t*t*t*fin.x,
               (1-t)*(1-t)*(1-t)*inicio.y + 3*(1-t)*(1-t)*t*cp1.y + 3*(1-t)*t*t*cp2.y + t*t*t*fin.y);
    char etiq[48];
    snprintf(etiq, sizeof(etiq), "%.1f kW", flujo.energia);
    ImVec2 etsz = ImGui::CalcTextSize(etiq);
    draw->AddRectFilled(ImVec2(mid.x-etsz.x*0.5f-2, mid.y-etsz.y*0.5f-1),
                        ImVec2(mid.x+etsz.x*0.5f+2, mid.y+etsz.y*0.5f+1),
                        IM_COL32(40,20,20,220), 2);
    draw->AddText(ImVec2(mid.x-etsz.x*0.5f, mid.y-etsz.y*0.5f),
                  IM_COL32(255,180,160,220), etiq);
}

// ═══════════════════════════════════════════════════════════════════════════
// Anotaciones (texto, rectángulos, elipses)
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_anotaciones(const ImVec2& pos) {
    if (!flowsheet_) return;
    ImDrawList* draw = ImGui::GetWindowDrawList();

    for (auto& a : flowsheet_->anotaciones()) {
        if (a.pagina != pagina_actual) continue;
        float ax = pos.x + a.x * zoom + desplazamiento.x;
        float ay = pos.y + a.y * zoom + desplazamiento.y;
        float aw = a.w * zoom;
        float ah = a.h * zoom;

        bool sel = (id_anotacion_seleccionada == a.id);
        ImU32 col_b = sel ? IM_COL32(255, 210, 60, 255) : a.color_borde;
        float bw = sel ? (a.grosor_borde + 1.5f) * zoom : a.grosor_borde * zoom;

        switch (a.tipo) {
            case diagrama_flujo::Anotacion::Tipo::Texto:
                draw->AddRectFilled(ImVec2(ax,ay), ImVec2(ax+aw,ay+ah), a.color_fondo, 4 * zoom);
                draw->AddRect(ImVec2(ax,ay), ImVec2(ax+aw,ay+ah), col_b, 4 * zoom, 0, bw);
                draw->AddText(ImVec2(ax + 6 * zoom, ay + 6 * zoom), IM_COL32(220,220,240,255), a.texto.c_str());
                break;
            case diagrama_flujo::Anotacion::Tipo::Rectangulo:
                draw->AddRectFilled(ImVec2(ax,ay), ImVec2(ax+aw,ay+ah), a.color_fondo, 2 * zoom);
                draw->AddRect(ImVec2(ax,ay), ImVec2(ax+aw,ay+ah), col_b, 2 * zoom, 0, bw);
                draw->AddText(ImVec2(ax + 6 * zoom, ay + 6 * zoom), IM_COL32(200,200,220,180), a.texto.c_str());
                break;
            case diagrama_flujo::Anotacion::Tipo::Elipse:
                draw->AddEllipseFilled(ImVec2(ax+aw*0.5f,ay+ah*0.5f), ImVec2(aw*0.5f,ah*0.5f), a.color_fondo);
                draw->AddEllipse(ImVec2(ax+aw*0.5f,ay+ah*0.5f), ImVec2(aw*0.5f,ah*0.5f), col_b, 0, 0, bw);
                draw->AddText(ImVec2(ax + 6 * zoom, ay + ah * 0.5f - 8 * zoom), IM_COL32(200,200,220,180), a.texto.c_str());
                break;
        }

        // Interacción: Selección y Arrastre de Anotaciones/Dibujos
        ImVec2 mpos = ImGui::GetIO().MousePos;
        bool hover_anot = hover_rect(mpos, ImVec2(ax, ay), ImVec2(ax + aw, ay + ah));
        
        if (hover_anot && clic_en_lienzo_) {
            id_anotacion_seleccionada = a.id;
            id_nodo_seleccionado = -1;
            id_flujo_seleccionado = -1;
            if (al_seleccionar_anotacion) al_seleccionar_anotacion(a.id);
            anotacion_arrastrando_ = a.id;
            clic_detectado_en_objeto_ = true;
        }

        if (anotacion_arrastrando_ == a.id) {
            if (ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
                if (ImGui::IsMouseDragging(ImGuiMouseButton_Left)) {
                    float dx = ImGui::GetIO().MouseDelta.x / zoom;
                    float dy = ImGui::GetIO().MouseDelta.y / zoom;
                    a.x += dx;
                    a.y += dy;
                }
            } else {
                if (ajustar_grilla) {
                    a.x = round(a.x / GRILLA) * GRILLA;
                    a.y = round(a.y / GRILLA) * GRILLA;
                }
                anotacion_arrastrando_ = -1;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tablas de flujo en el lienzo
// ═══════════════════════════════════════════════════════════════════════════

void Lienzo::renderizar_tablas_flujo(const ImVec2& pos) {
    if (!flowsheet_) return;
    ImDrawList* draw = ImGui::GetWindowDrawList();

    for (auto& tabla : flowsheet_->tablas_flujo()) {
        if (tabla.pagina != pagina_actual) continue;
        if (!tabla.visible) continue;

        float tx = pos.x + tabla.x + desplazamiento.x;
        float ty = pos.y + tabla.y + desplazamiento.y;

        draw->AddRectFilled(ImVec2(tx,ty), ImVec2(tx+tabla.w,ty+tabla.h), IM_COL32(20,20,30,240), 4);
        draw->AddRect(ImVec2(tx,ty), ImVec2(tx+tabla.w,ty+tabla.h), IM_COL32(80,80,120,200), 4);

        draw->AddText(ImVec2(tx+6,ty+4), IM_COL32(160,160,200,255), "Stream Table");

        float col_w[4] = {80, 80, 80, tabla.w - 280};
        const char* headers[] = {"Corriente", "Flujo (kmol/s)", "T (K)", "P (atm)"};
        float yh = ty + 24;

        for (int c = 0; c < 4; c++) {
            float cx = tx + 4;
            for (int i = 0; i < c; i++) cx += col_w[i];
            draw->AddText(ImVec2(cx+2, yh), IM_COL32(120,120,160,255), headers[c]);
        }

        int row = 0;
        for (auto& [fid, f] : flowsheet_->mapa_flujos()) {
            if (f.es_energia) continue;
            if (!tabla.ids_flujos_mostrados.empty() &&
                std::find(tabla.ids_flujos_mostrados.begin(), tabla.ids_flujos_mostrados.end(), fid)
                == tabla.ids_flujos_mostrados.end()) continue;

            float ry = yh + 18 + row * 18;
            if (ry + 18 > ty + tabla.h - 4) break;

            char buf[64];
            ImU32 col_r = (row % 2) ? IM_COL32(255,255,255,10) : IM_COL32(255,255,255,4);
            draw->AddRectFilled(ImVec2(tx+2, ry), ImVec2(tx+tabla.w-2, ry+16), col_r, 2);

            float cx = tx + 6;
            draw->AddText(ImVec2(cx, ry+1), IM_COL32(200,200,220,220), f.nombre.c_str());
            cx += col_w[0];

            snprintf(buf, sizeof(buf), "%.4f", f.flujo_total());
            draw->AddText(ImVec2(cx, ry+1), IM_COL32(180,200,180,220), buf);
            cx += col_w[1];

            snprintf(buf, sizeof(buf), "%.1f", f.temperatura);
            draw->AddText(ImVec2(cx, ry+1), IM_COL32(200,180,180,220), buf);
            cx += col_w[2];

            snprintf(buf, sizeof(buf), "%.2f", f.presion);
            draw->AddText(ImVec2(cx, ry+1), IM_COL32(180,180,200,220), buf);

            row++;
        }
    }
}

}} // namespace
