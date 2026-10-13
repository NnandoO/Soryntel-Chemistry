#include "interfaz/aplicacion.hpp"
#include "utils/i18n.h"

#include <GLFW/glfw3.h>
#include <imgui.h>
#include <imgui_impl_glfw.h>
#include <imgui_impl_opengl3.h>
#include <implot.h>
#include <implot3d.h>

#include <cstdio>
#include <ctime>
#include <fstream>
#include <sstream>
#include <iomanip>
#include <filesystem>
#include <set>
#include <algorithm>
#include "adapters/desktop/process.hpp"
#include "application/SimulationService.hpp"

namespace cma { namespace interfaz {
void Aplicacion::cargar_ejemplo(int ejemplo) {
    nueva_simulacion();
    switch (ejemplo) {
        case 1: cargar_ejemplo_1(); break;
        case 2: cargar_ejemplo_2(); break;
        case 3: cargar_ejemplo_3(); break;
        case 4: cargar_ejemplo_4(); break;
        default: cargar_ejemplo_1(); break;
    }
}

void Aplicacion::cargar_ejemplo_1() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 1: Reformado de Metano (SMR) ──
    // CH₄ + H₂O → CO + 3H₂  (Reformador)
    // CO + H₂O → CO₂ + H₂   (WGS)
    // Mezcla final → gas de síntesis

    auto id_fuente = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "CH₄ + Vapor");
    auto* fuente = flowsheet_->nodo_por_id(id_fuente);
    if (fuente) {
        fuente->canvas_x = 50; fuente->canvas_y = 50;
        auto* ch4 = gestor.especie_por_nombre("Metano");
        auto* h2o = gestor.especie_por_nombre("Agua");
        if (ch4) fuente->params["flujo_" + std::to_string(ch4->id)] = 1.0;
        if (h2o) fuente->params["flujo_" + std::to_string(h2o->id)] = 2.0;
        fuente->params["T"] = 500.0;
        fuente->params["P"] = 20.0;
    }

    auto id_reactor = flowsheet_->agregar_nodo(nucleo::TipoNodo::Reactor, "Reformador SMR");
    auto* reactor = flowsheet_->nodo_por_id(id_reactor);
    if (reactor) {
        reactor->canvas_x = 280; reactor->canvas_y = 50;
        reactor->params["conversion"] = 0.75;
        reactor->params["temperatura"] = 1073.0;
        reactor->params["presion"] = 20.0;
        reactor->params["reaccion_id"] = 1.0;
    }

    auto id_fuente2 = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Aire (O₂)");
    auto* fuente2 = flowsheet_->nodo_por_id(id_fuente2);
    if (fuente2) {
        fuente2->canvas_x = 50; fuente2->canvas_y = 250;
        auto* o2 = gestor.especie_por_nombre("Oxígeno");
        if (o2) fuente2->params["flujo_" + std::to_string(o2->id)] = 0.5;
    }

    auto id_wgs = flowsheet_->agregar_nodo(nucleo::TipoNodo::Reactor, "WGS");
    auto* wgs = flowsheet_->nodo_por_id(id_wgs);
    if (wgs) {
        wgs->canvas_x = 500; wgs->canvas_y = 50;
        wgs->params["conversion"] = 0.90;
        wgs->params["temperatura"] = 623.0;
        wgs->params["presion"] = 20.0;
        wgs->params["reaccion_id"] = 2.0;
    }

    auto id_mezcla = flowsheet_->agregar_nodo(nucleo::TipoNodo::Mezclador, "Mezclador");
    auto* mezcla = flowsheet_->nodo_por_id(id_mezcla);
    if (mezcla) {
        mezcla->canvas_x = 700; mezcla->canvas_y = 150;
    }

    auto id_sum = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Gas Síntesis");
    auto* sum = flowsheet_->nodo_por_id(id_sum);
    if (sum) {
        sum->canvas_x = 880; sum->canvas_y = 150;
    }

    auto id_calor = flowsheet_->agregar_flujo_energia("Calor SMR");
    auto* flujo_calor = flowsheet_->flujo_por_id(id_calor);
    if (flujo_calor) flujo_calor->energia = 500.0;

    flowsheet_->conectar(id_fuente, id_reactor);
    flowsheet_->conectar(id_reactor, id_wgs);
    flowsheet_->conectar(id_wgs, id_mezcla);
    flowsheet_->conectar(id_fuente2, id_mezcla);
    flowsheet_->conectar(id_mezcla, id_sum);

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(0, 0);
    panel_propiedades_.seleccionar_nodo(id_fuente);
    panel_resultados_.poner_resultado_masa("Ejemplo 1: Reformado de Metano (SMR) cargado. Presiona F5 o Módulos→SteadyStateEngine", true);
}

void Aplicacion::cargar_ejemplo_4() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 4: SMR con lazo de recirculación de CH₄ ──
    // Fuente (CH₄+H₂O) → Mezclador → Reactor SMR → Separador → Sumidero
    //                                       │  (CH₄ sin convertir se recicla)
    //                                       └──▶ Mezclador
    // El separador retira todo el CH₄ (remover_ = 1.0) y lo devuelve al reactor.
    // Solución esperada (converge con Ctrl+R / F5):
    //   Alimentación al reactor: CH₄ = 1.3333, H₂O = 2.0  (3.3333 kmol/s)
    //   Reciclo: CH₄ = 0.3333 kmol/s   Producto: H₂ = 3.0, CO = 1.0, H₂O = 1.0 (5.0 kmol/s)

    auto id_fuente = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "CH₄ + Vapor");
    auto* fuente = flowsheet_->nodo_por_id(id_fuente);
    if (fuente) {
        fuente->canvas_x = 50; fuente->canvas_y = 150;
        auto* ch4 = gestor.especie_por_nombre("Metano");
        auto* h2o = gestor.especie_por_nombre("Agua");
        if (ch4) fuente->params["flujo_" + std::to_string(ch4->id)] = 1.0;
        if (h2o) fuente->params["flujo_" + std::to_string(h2o->id)] = 2.0;
        fuente->params["temperatura"] = 500.0;
        fuente->params["presion"] = 20.0;
    }

    auto id_mezclador = flowsheet_->agregar_nodo(nucleo::TipoNodo::Mezclador, "Mezclador");
    auto* mezclador = flowsheet_->nodo_por_id(id_mezclador);
    if (mezclador) { mezclador->canvas_x = 240; mezclador->canvas_y = 150; }

    auto id_reactor = flowsheet_->agregar_nodo(nucleo::TipoNodo::Reactor, "Reformador SMR");
    auto* reactor = flowsheet_->nodo_por_id(id_reactor);
    if (reactor) {
        reactor->canvas_x = 430; reactor->canvas_y = 150;
        reactor->params["conversion"] = 0.75;
        reactor->params["temperatura"] = 1073.0;
        reactor->params["presion"] = 20.0;
        reactor->params["reaccion_id"] = 1.0;
    }

    auto id_sep = flowsheet_->agregar_nodo(nucleo::TipoNodo::Separador, "Separador CH₄");
    auto* sep = flowsheet_->nodo_por_id(id_sep);
    if (sep) {
        sep->canvas_x = 640; sep->canvas_y = 150;
        auto* ch4 = gestor.especie_por_nombre("Metano");
        if (ch4) sep->params["remover_" + std::to_string(ch4->id)] = 1.0;
    }

    auto id_sum = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Gas de Síntesis");
    auto* sum = flowsheet_->nodo_por_id(id_sum);
    if (sum) { sum->canvas_x = 850; sum->canvas_y = 150; }

    flowsheet_->conectar(id_fuente, id_mezclador);
    flowsheet_->conectar(id_mezclador, id_reactor);
    flowsheet_->conectar(id_reactor, id_sep);
    flowsheet_->conectar(id_sep, id_sum);       // salida 0 = producto (sin CH₄)
    flowsheet_->conectar(id_sep, id_mezclador); // salida 1 = reciclo de CH₄

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-30, -50);
    panel_propiedades_.seleccionar_nodo(id_fuente);
    panel_resultados_.poner_resultado_masa(
        "Ejemplo 4: SMR con reciclo de CH₄ cargado. Presiona Ctrl+R (Resolver Recirculaciones) o F5.", true);
}

void Aplicacion::cargar_ejemplo_2() {
    auto& gestor = base_datos::GestorDB::instancia();
    // ── Ejemplo 2: Flotación de Cobre ──
    // Alimentación (mineral con CuFeS₂, FeS₂, SiO₂) → Molienda → Flotación → Concentrado + Relave

    auto id_feed = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Mineral Cu");
    auto* feed = flowsheet_->nodo_por_id(id_feed);
    if (feed) {
        feed->canvas_x = 50; feed->canvas_y = 150;
        feed->params["ley_Cu"] = 1.2;
        feed->params["ley_Fe"] = 25.0;
        feed->params["ley_S"] = 28.0;
        feed->params["ley_SiO2"] = 45.0;
        auto* pyrite = gestor.especie_por_nombre("Pirita");
        auto* cuarzo = gestor.especie_por_nombre("Cuarzo");
        if (pyrite) feed->params["flujo_" + std::to_string(pyrite->id)] = 60.0;
        if (cuarzo) feed->params["flujo_" + std::to_string(cuarzo->id)] = 40.0;
    }

    auto id_molienda = flowsheet_->agregar_nodo(nucleo::TipoNodo::MolinoBolas, "Molienda");
    auto* molienda = flowsheet_->nodo_por_id(id_molienda);
    if (molienda) {
        molienda->canvas_x = 250; molienda->canvas_y = 150;
        molienda->params["potencia"] = 1500.0;
        molienda->params["P80"] = 75.0;
    }

    auto id_flotacion = flowsheet_->agregar_nodo(nucleo::TipoNodo::CeldaFlotacion, "Flotación Rougher");
    auto* flotacion = flowsheet_->nodo_por_id(id_flotacion);
    if (flotacion) {
        flotacion->canvas_x = 450; flotacion->canvas_y = 150;
        flotacion->params["recuperacion"] = 0.90;
        flotacion->params["ley_concentrado"] = 28.0;
    }

    auto id_conc = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Concentrado Cu");
    auto* conc = flowsheet_->nodo_por_id(id_conc);
    if (conc) { conc->canvas_x = 650; conc->canvas_y = 80; }

    auto id_relave = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Relave");
    auto* relave = flowsheet_->nodo_por_id(id_relave);
    if (relave) { relave->canvas_x = 650; relave->canvas_y = 280; }

    flowsheet_->conectar(id_feed, id_molienda);
    flowsheet_->conectar(id_molienda, id_flotacion);
    flowsheet_->conectar(id_flotacion, id_conc);
    flowsheet_->conectar(id_flotacion, id_relave);

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-50, -50);
    panel_resultados_.poner_resultado_masa("Ejemplo 2: Flotación de Cobre cargado. Usa Módulos→SpeciesConverter para mineralogía", true);
}

void Aplicacion::cargar_ejemplo_3() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 3: Alto Horno de Hierro (simplificado) ──
    // Mineral + Coque → Reducción → Arrabio + Escoria + Gas

    auto id_mineral = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Mineral Fe₂O₃");
    auto* mineral = flowsheet_->nodo_por_id(id_mineral);
    if (mineral) {
        mineral->canvas_x = 50; mineral->canvas_y = 50;
        auto* hem = gestor.especie_por_nombre("Hematita");
        auto* qz = gestor.especie_por_nombre("Cuarzo");
        if (hem) mineral->params["flujo_" + std::to_string(hem->id)] = 80.0;
        if (qz) mineral->params["flujo_" + std::to_string(qz->id)] = 15.0;
        mineral->params["T"] = 298.0;
    }

    auto id_coque = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Coque (C)");
    auto* coque = flowsheet_->nodo_por_id(id_coque);
    if (coque) {
        coque->canvas_x = 50; coque->canvas_y = 250;
        auto* coq = gestor.especie_por_nombre("Negro de carbón");
        if (coq) coque->params["flujo_" + std::to_string(coq->id)] = 40.0;
        coque->params["T"] = 298.0;
    }

    auto id_aire = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Aire caliente");
    auto* aire = flowsheet_->nodo_por_id(id_aire);
    if (aire) {
        aire->canvas_x = 50; aire->canvas_y = 400;
        auto* o2 = gestor.especie_por_nombre("Oxígeno");
        auto* n2 = gestor.especie_por_nombre("Nitrógeno");
        if (o2) aire->params["flujo_" + std::to_string(o2->id)] = 50.0;
        if (n2) aire->params["flujo_" + std::to_string(n2->id)] = 188.0;
        aire->params["T"] = 1200.0;
    }

    auto id_mezcla = flowsheet_->agregar_nodo(nucleo::TipoNodo::Mezclador, "Mezcla carga");
    auto* mezcla = flowsheet_->nodo_por_id(id_mezcla);
    if (mezcla) {
        mezcla->canvas_x = 280; mezcla->canvas_y = 150;
    }

    auto id_horno = flowsheet_->agregar_nodo(nucleo::TipoNodo::HornoFlash, "Alto Horno");
    auto* horno = flowsheet_->nodo_por_id(id_horno);
    if (horno) {
        horno->canvas_x = 480; horno->canvas_y = 150;
        horno->params["temperatura"] = 1873.0;
        horno->params["presion"] = 3.0;
        horno->params["conversion"] = 0.92;
    }

    auto id_arrabio = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Arrabio");
    auto* arrabio = flowsheet_->nodo_por_id(id_arrabio);
    if (arrabio) { arrabio->canvas_x = 700; arrabio->canvas_y = 50; }

    auto id_escoria = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Escoria");
    auto* escoria = flowsheet_->nodo_por_id(id_escoria);
    if (escoria) { escoria->canvas_x = 700; escoria->canvas_y = 180; }

    auto id_gas = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Gas salida");
    auto* gas = flowsheet_->nodo_por_id(id_gas);
    if (gas) { gas->canvas_x = 700; gas->canvas_y = 310; }

    flowsheet_->conectar(id_mineral, id_mezcla);
    flowsheet_->conectar(id_coque, id_mezcla);
    flowsheet_->conectar(id_mezcla, id_horno);
    flowsheet_->conectar(id_aire, id_horno);
    flowsheet_->conectar(id_horno, id_arrabio);
    flowsheet_->conectar(id_horno, id_escoria);
    flowsheet_->conectar(id_horno, id_gas);

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-80, -30);
    panel_resultados_.poner_resultado_masa("Ejemplo 3: Alto Horno cargado. Presiona F5 para simular balance de masa", true);
}


}} // namespace
