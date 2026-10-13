#pragma once
#include "interfaz/lienzo.hpp"
#include "utils/i18n.h"
#include <imgui.h>
#include <algorithm>
#include <cmath>
#include <cstring>

namespace cma { namespace interfaz {

inline std::string sanitizar_texto(std::string str) {
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



inline bool hover_rect(const ImVec2& mouse, const ImVec2& r_min, const ImVec2& r_max) {
    return mouse.x >= r_min.x && mouse.x <= r_max.x && mouse.y >= r_min.y && mouse.y <= r_max.y;
}

extern bool g_nodo_arrastrado;

inline constexpr float NODO_ANCHO   = 150.0f;
inline constexpr float NODO_ALTO    = 82.0f;
inline constexpr float CABECERA_H   = 24.0f;
inline constexpr float RADIO        = 6.0f;
inline constexpr float RADIO_PTO    = 5.0f;
inline constexpr float GRILLA       = 20.0f;
inline constexpr float INERCIA_DEC  = 0.85f;

inline ImU32 col_cab(nucleo::TipoNodo t) {
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

inline const char* descripcion_tipo_nodo(nucleo::TipoNodo t) {
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
}} // namespace
