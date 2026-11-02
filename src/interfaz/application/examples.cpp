#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"
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
        case 5: cargar_ejemplo_5(); break;
        case 6: cargar_ejemplo_6(); break;
        default: cargar_ejemplo_1(); break;
    }
    ejecutar_simulacion();
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
        reactor->params["modo_sim"] = 1.0; // Reactions
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
        wgs->params["modo_sim"] = 1.0;
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
    lienzo_.modo_visualizacion_flujos = 2; // kmol/s
    panel_propiedades_.seleccionar_nodo(id_fuente);
    panel_resultados_.poner_resultado_masa("Ejemplo 1: Reformado de Metano (SMR) cargado y simulado.", true);
}

void Aplicacion::cargar_ejemplo_4() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 4: SMR con lazo de recirculación de CH₄ ──
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
        reactor->params["modo_sim"] = 1.0;
    }

    auto id_sep = flowsheet_->agregar_nodo(nucleo::TipoNodo::Separador, "Separador CH₄");
    auto* sep = flowsheet_->nodo_por_id(id_sep);
    if (sep) {
        sep->canvas_x = 640; sep->canvas_y = 150;
        auto* ch4 = gestor.especie_por_nombre("Metano");
        if (ch4) sep->params["remover_" + std::to_string(ch4->id)] = 1.0;
        sep->params["modo_sim"] = 2.0; // Distribution
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
    lienzo_.modo_visualizacion_flujos = 2; // kmol/s
    panel_propiedades_.seleccionar_nodo(id_fuente);
}

void Aplicacion::cargar_ejemplo_2() {
    auto& gestor = base_datos::GestorDB::instancia();
    // ── Ejemplo 2: Flotación y Conminución de Cobre (Modo Partículas + Especies) ──
    // Alimentación (mineral con CuFeS₂, FeS₂, SiO₂ + PSD) → Molienda → Hidrociclón → Flotación → Concentrado + Relave

    auto id_feed = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Mineral Cu (ROM)");
    auto* feed = flowsheet_->nodo_por_id(id_feed);
    auto* cpy = gestor.especie_por_nombre("Calcopirita");
    auto* pyrite = gestor.especie_por_nombre("Pirita");
    auto* cuarzo = gestor.especie_por_nombre("Cuarzo");
    auto* h2o_l = gestor.especie_por_nombre("Agua (líquida)");
    if (feed) {
        feed->canvas_x = 40; feed->canvas_y = 150;
        feed->params["modo_sim"] = 0.0; // Particles
        feed->params["ley_Cu"] = 1.2;
        feed->params["ley_Fe"] = 25.0;
        feed->params["ley_S"] = 28.0;
        feed->params["ley_SiO2"] = 45.0;
        if (cpy)    feed->params["flujo_" + std::to_string(cpy->id)] = 0.055; // ~36 t/h CuFeS2
        if (pyrite) feed->params["flujo_" + std::to_string(pyrite->id)] = 0.045; // ~19 t/h FeS2
        if (cuarzo) feed->params["flujo_" + std::to_string(cuarzo->id)] = 0.210; // ~45 t/h SiO2
        if (h2o_l)  feed->params["flujo_" + std::to_string(h2o_l->id)] = 0.690;  // ~45 t/h H2O
        // PSD para el motor de Procesamiento Mineral (Bond + Plitt + Cinética de Flotación)
        feed->params["mineral_0_flow"] = 36000.0; // 36 t/h Calcopirita
        feed->params["mineral_0_d"]    = 2400.0;  // 2400 um
        feed->params["mineral_1_flow"] = 19000.0; // 19 t/h Pirita
        feed->params["mineral_1_d"]    = 2000.0;
        feed->params["mineral_2_flow"] = 45000.0; // 45 t/h Cuarzo
        feed->params["mineral_2_d"]    = 1800.0;
        feed->params["agua_kg_h"]      = 45000.0;
    }

    auto id_molienda = flowsheet_->agregar_nodo(nucleo::TipoNodo::MolinoBolas, "Molino de Bolas");
    auto* molienda = flowsheet_->nodo_por_id(id_molienda);
    if (molienda) {
        molienda->canvas_x = 250; molienda->canvas_y = 150;
        molienda->params["modo_sim"] = 0.0; // Particles
        molienda->params["wi"] = 13.5;
        molienda->params["power"] = 1850.0;
        molienda->params["potencia"] = 1850.0;
        molienda->params["P80"] = 105.0;
    }

    auto id_flotacion = flowsheet_->agregar_nodo(nucleo::TipoNodo::CeldaFlotacion, "Flotación Rougher");
    auto* flotacion = flowsheet_->nodo_por_id(id_flotacion);
    if (flotacion) {
        flotacion->canvas_x = 470; flotacion->canvas_y = 150;
        flotacion->params["modo_sim"] = 0.0; // Particles
        flotacion->params["recuperacion"] = 0.90;
        flotacion->params["residence_time"] = 12.0;
        flotacion->params["water_recovery"] = 0.14;
        flotacion->params["k_Mineral_0"] = 0.42;
        flotacion->params["Rinf_Mineral_0"] = 0.92;
        flotacion->params["k_Mineral_1"] = 0.12;
        flotacion->params["Rinf_Mineral_1"] = 0.35;
        flotacion->params["k_Mineral_2"] = 0.02;
        flotacion->params["Rinf_Mineral_2"] = 0.06;
        // Partición molar coherente (Salida 0 = Concentrado, Salida 1 = Relave)
        if (cpy)    flotacion->params["split_" + std::to_string(cpy->id)] = 0.08;    // 92% al concentrado
        if (pyrite) flotacion->params["split_" + std::to_string(pyrite->id)] = 0.68; // 32% al concentrado
        if (cuarzo) flotacion->params["split_" + std::to_string(cuarzo->id)] = 0.95; // 95% al relave
        if (h2o_l)  flotacion->params["split_" + std::to_string(h2o_l->id)] = 0.86;  // 86% agua al relave
    }

    auto id_conc = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Concentrado Cu");
    auto* conc = flowsheet_->nodo_por_id(id_conc);
    if (conc) { conc->canvas_x = 700; conc->canvas_y = 70; }

    auto id_relave = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Relave General");
    auto* relave = flowsheet_->nodo_por_id(id_relave);
    if (relave) { relave->canvas_x = 700; relave->canvas_y = 260; }

    flowsheet_->conectar(id_feed, id_molienda);
    flowsheet_->conectar(id_molienda, id_flotacion);
    flowsheet_->conectar(id_flotacion, id_conc);
    flowsheet_->conectar(id_flotacion, id_relave);

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-30, -30);
    lienzo_.modo_visualizacion_flujos = 1; // t/h
    panel_propiedades_.seleccionar_nodo(id_flotacion);
}

void Aplicacion::cargar_ejemplo_3() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 3: Alto Horno de Hierro (Pirometalurgia) ──
    auto id_mineral = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Mineral Fe₂O₃");
    auto* mineral = flowsheet_->nodo_por_id(id_mineral);
    auto* hem = gestor.especie_por_nombre("Hematita");
    auto* qz = gestor.especie_por_nombre("Cuarzo");
    auto* coq = gestor.especie_por_nombre("Negro de carbón");
    auto* o2 = gestor.especie_por_nombre("Oxígeno");
    auto* n2 = gestor.especie_por_nombre("Nitrógeno");
    auto* fe = gestor.especie_por_nombre("Hierro (α)");
    auto* co2 = gestor.especie_por_nombre("Dióxido de carbono");

    if (mineral) {
        mineral->canvas_x = 50; mineral->canvas_y = 50;
        if (hem) mineral->params["flujo_" + std::to_string(hem->id)] = 0.80;
        if (qz)  mineral->params["flujo_" + std::to_string(qz->id)] = 0.15;
        mineral->params["T"] = 298.15;
    }

    auto id_coque = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Coque (C)");
    auto* coque = flowsheet_->nodo_por_id(id_coque);
    if (coque) {
        coque->canvas_x = 50; coque->canvas_y = 220;
        if (coq) coque->params["flujo_" + std::to_string(coq->id)] = 1.40;
        coque->params["T"] = 298.15;
    }

    auto id_aire = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Aire caliente");
    auto* aire = flowsheet_->nodo_por_id(id_aire);
    if (aire) {
        aire->canvas_x = 50; aire->canvas_y = 390;
        if (o2) aire->params["flujo_" + std::to_string(o2->id)] = 0.70;
        if (n2) aire->params["flujo_" + std::to_string(n2->id)] = 2.63;
        aire->params["T"] = 1200.0;
    }

    auto id_mezcla = flowsheet_->agregar_nodo(nucleo::TipoNodo::Mezclador, "Mezcla carga");
    auto* mezcla = flowsheet_->nodo_por_id(id_mezcla);
    if (mezcla) {
        mezcla->canvas_x = 260; mezcla->canvas_y = 140;
    }

    auto id_horno = flowsheet_->agregar_nodo(nucleo::TipoNodo::HornoFlash, "Alto Horno");
    auto* horno = flowsheet_->nodo_por_id(id_horno);
    if (horno) {
        horno->canvas_x = 470; horno->canvas_y = 180;
        horno->params["modo_sim"] = 1.0; // Reactions
        horno->params["temperatura"] = 1773.15;
        horno->params["presion"] = 3.0;
        horno->params["reaccion_id"] = 40.0; // Reducción carbotérmica de Hematita
        horno->params["conversion"] = 0.94;
        horno->params["frac_0"] = 0.42; // Arrabio
        horno->params["frac_1"] = 0.14; // Escoria
        horno->params["frac_2"] = 0.44; // Gas de tragante
    }

    auto id_arrabio = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Arrabio (Fe)");
    auto* arrabio = flowsheet_->nodo_por_id(id_arrabio);
    if (arrabio) { arrabio->canvas_x = 720; arrabio->canvas_y = 60; }

    auto id_escoria = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Escoria (SiO₂)");
    auto* escoria = flowsheet_->nodo_por_id(id_escoria);
    if (escoria) { escoria->canvas_x = 720; escoria->canvas_y = 190; }

    auto id_gas = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Gas de Alto Horno");
    auto* gas = flowsheet_->nodo_por_id(id_gas);
    if (gas) { gas->canvas_x = 720; gas->canvas_y = 320; }

    flowsheet_->conectar(id_mineral, id_mezcla);
    flowsheet_->conectar(id_coque, id_mezcla);
    flowsheet_->conectar(id_mezcla, id_horno);
    flowsheet_->conectar(id_aire, id_horno);
    flowsheet_->conectar(id_horno, id_arrabio);
    flowsheet_->conectar(id_horno, id_escoria);
    flowsheet_->conectar(id_horno, id_gas);

    (void)fe; (void)co2;
    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-40, -20);
    lienzo_.modo_visualizacion_flujos = 1; // t/h
    panel_propiedades_.seleccionar_nodo(id_horno);
}

void Aplicacion::cargar_ejemplo_5() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 5: Proceso Bayer - Producción de Alúmina (HSC-Sim Reference 1) ──
    // Bauxita → Crushing → Screening → Pressure Leaching (+ NaOH) → Clarification
    //   ├── Red Mud (Sólidos Fe2O3/SiO2)
    //   └── Pregnant Liquor → Heat Exchanger → Crystallization → Calcination → Al2O3 + Vapor
    auto* aloh3  = gestor.especie_por_nombre("Hidróxido de aluminio");
    auto* naoh   = gestor.especie_por_nombre("Hidróxido de sodio (aq)");
    auto* naalo2 = gestor.especie_por_nombre("Aluminato de sodio (a)");
    auto* al2o3  = gestor.especie_por_nombre("Óxido de aluminio");
    auto* fe2o3  = gestor.especie_por_nombre("Hematita");
    auto* sio2   = gestor.especie_por_nombre("Cuarzo");
    auto* h2o_l  = gestor.especie_por_nombre("Agua (líquida)");
    auto* h2o_g  = gestor.especie_por_nombre("Agua");

    auto id_baux = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Bauxita + Licor NaOH");
    if (auto* n = flowsheet_->nodo_por_id(id_baux)) {
        n->canvas_x = 30; n->canvas_y = 90;
        n->params["modo_sim"] = 0.0; // Particles
        if (aloh3) n->params["flujo_" + std::to_string(aloh3->id)] = 0.350; // ~98 t/h Al(OH)3
        if (naoh)  n->params["flujo_" + std::to_string(naoh->id)]  = 0.380; // ~55 t/h NaOH(aq)
        if (fe2o3) n->params["flujo_" + std::to_string(fe2o3->id)] = 0.045; // ~26 t/h Fe2O3 (lodo rojo)
        if (sio2)  n->params["flujo_" + std::to_string(sio2->id)]  = 0.025; // ~5.4 t/h SiO2
        if (h2o_l) n->params["flujo_" + std::to_string(h2o_l->id)] = 0.850; // ~55 t/h H2O(l)
        n->params["temperatura"] = 318.15;
        n->params["presion"] = 1.0;
    }

    auto id_crush = flowsheet_->agregar_nodo(nucleo::TipoNodo::TrituradoraMandibula, "Crushing");
    if (auto* n = flowsheet_->nodo_por_id(id_crush)) {
        n->canvas_x = 185; n->canvas_y = 90;
        n->params["modo_sim"] = 0.0; // Particles
        n->params["css"] = 12.0;
    }

    auto id_leach = flowsheet_->agregar_nodo(nucleo::TipoNodo::TanqueLixiviacion, "Pressure Leaching");
    if (auto* n = flowsheet_->nodo_por_id(id_leach)) {
        n->canvas_x = 355; n->canvas_y = 90;
        n->params["modo_sim"] = 1.0; // Reactions
        n->params["reaccion_id"] = 35.0; // Al(OH)3 + NaOH(aq) -> NaAlO2(a) + 2 H2O
        n->params["conversion"] = 0.94;
        n->params["temperatura"] = 523.15; // 250 °C
        n->params["presion"] = 35.0;       // 35 bar
        // Salida 0 = Licor preñado (líquidos/acuosos), Salida 1 = Lodo Rojo (sólidos)
        if (naalo2) n->params["split_" + std::to_string(naalo2->id)] = 0.02;
        if (naoh)   n->params["split_" + std::to_string(naoh->id)]   = 0.05;
        if (h2o_l)  n->params["split_" + std::to_string(h2o_l->id)]  = 0.15;
        if (fe2o3)  n->params["split_" + std::to_string(fe2o3->id)]  = 0.98;
        if (sio2)   n->params["split_" + std::to_string(sio2->id)]   = 0.98;
    }

    auto id_redmud = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Red Mud (Clarification)");
    if (auto* n = flowsheet_->nodo_por_id(id_redmud)) {
        n->canvas_x = 540; n->canvas_y = 250;
    }

    auto id_hex = flowsheet_->agregar_nodo(nucleo::TipoNodo::IntercambiadorCalor, "Heat Exchanger");
    if (auto* n = flowsheet_->nodo_por_id(id_hex)) {
        n->canvas_x = 540; n->canvas_y = 90;
        n->params["modo_sim"] = 2.0; // Distribution
        n->params["T_salida"] = 338.15; // 65 °C
        n->params["caida_presion"] = 0.5;
    }

    auto id_cryst = flowsheet_->agregar_nodo(nucleo::TipoNodo::Reactor, "Crystallization");
    if (auto* n = flowsheet_->nodo_por_id(id_cryst)) {
        n->canvas_x = 720; n->canvas_y = 90;
        n->params["modo_sim"] = 1.0; // Reactions
        n->params["reaccion_id"] = 36.0; // NaAlO2(a) + 2 H2O -> Al(OH)3 + NaOH(aq)
        n->params["conversion"] = 0.88;
        n->params["temperatura"] = 333.15;
        n->params["presion"] = 1.0;
    }

    auto id_calc = flowsheet_->agregar_nodo(nucleo::TipoNodo::HornoRotatorio, "Calcination");
    if (auto* n = flowsheet_->nodo_por_id(id_calc)) {
        n->canvas_x = 900; n->canvas_y = 90;
        n->params["modo_sim"] = 1.0; // Reactions
        n->params["reaccion_id"] = 37.0; // 2 Al(OH)3 -> Al2O3 + 3 H2O(g)
        n->params["conversion"] = 0.98;
        n->params["temperatura"] = 1323.15; // 1050 °C
        n->params["presion"] = 1.0;
        // Salida 0 = Alúmina Producto (Al2O3), Salida 1 = Gases / Licor agotado
        if (al2o3)  n->params["split_" + std::to_string(al2o3->id)]  = 0.01;
        if (h2o_g)  n->params["split_" + std::to_string(h2o_g->id)]  = 0.99;
        if (h2o_l)  n->params["split_" + std::to_string(h2o_l->id)]  = 0.98;
        if (naoh)   n->params["split_" + std::to_string(naoh->id)]   = 0.98;
        if (naalo2) n->params["split_" + std::to_string(naalo2->id)] = 0.95;
    }

    auto id_prod = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Alúmina (Al₂O₃)");
    if (auto* n = flowsheet_->nodo_por_id(id_prod)) {
        n->canvas_x = 1090; n->canvas_y = 45;
    }

    auto id_offgas = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Vapor + Licor Agotado");
    if (auto* n = flowsheet_->nodo_por_id(id_offgas)) {
        n->canvas_x = 1090; n->canvas_y = 210;
    }

    flowsheet_->conectar(id_baux, id_crush);
    flowsheet_->conectar(id_crush, id_leach);
    flowsheet_->conectar(id_leach, id_hex);    // salida 0: licor preñado
    flowsheet_->conectar(id_leach, id_redmud); // salida 1: lodo rojo
    flowsheet_->conectar(id_hex, id_cryst);
    flowsheet_->conectar(id_cryst, id_calc);
    flowsheet_->conectar(id_calc, id_prod);    // salida 0: Al2O3
    flowsheet_->conectar(id_calc, id_offgas);  // salida 1: vapor + licor

    // Anotación de ecuaciones químicas en el lienzo (estilo HSC-Sim Bayer Process)
    int64_t id_txt = flowsheet_->agregar_anotacion(
        "Ecuaciones del Proceso Bayer (HSC-Sim):\n"
        "  Digestión (250 °C):   Al(OH)3(s) + NaOH(aq) -> NaAlO2(a) + 2 H2O(l)\n"
        "  Cristalización (60 °C): NaAlO2(a) + 2 H2O(l) -> Al(OH)3(s) + NaOH(aq)\n"
        "  Calcinación (1050 °C): 2 Al(OH)3(s) -> Al2O3(s) + 3 H2O(g)");
    if (auto* anot = flowsheet_->anotacion_por_id(id_txt)) {
        anot->tipo = diagrama_flujo::Anotacion::Tipo::Texto;
        anot->x = 180.0f; anot->y = 340.0f; anot->w = 680.0f; anot->h = 80.0f;
    }

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-10, 0);
    lienzo_.modo_visualizacion_flujos = 1; // Amount (t/h)
    panel_propiedades_.seleccionar_nodo(id_leach);
}

void Aplicacion::cargar_ejemplo_6() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 6: Proceso TiO₂ por Cloración con Reciclo de Cl₂ y Diagrama de Sankey (HSC-Sim Reference 2) ──
    auto* tio2  = gestor.especie_por_nombre("Dióxido de titanio");
    auto* coke  = gestor.especie_por_nombre("Negro de carbón");
    auto* cl2   = gestor.especie_por_nombre("Cloro");
    auto* ticl4 = gestor.especie_por_nombre("Cloruro de titanio (g)");
    auto* co    = gestor.especie_por_nombre("Monóxido de carbono");
    auto* o2    = gestor.especie_por_nombre("Oxígeno");

    auto id_feed = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Slag TiO₂ + Coke + Cl₂ Makeup");
    if (auto* n = flowsheet_->nodo_por_id(id_feed)) {
        n->canvas_x = 30; n->canvas_y = 140;
        n->params["modo_sim"] = 1.0;
        if (tio2) n->params["flujo_" + std::to_string(tio2->id)] = 0.080; // ~23 t/h TiO2
        if (coke) n->params["flujo_" + std::to_string(coke->id)] = 0.170; // ~7.3 t/h C
        if (cl2)  n->params["flujo_" + std::to_string(cl2->id)]  = 0.030; // Makeup Cl2
        n->params["temperatura"] = 298.15;
        n->params["presion"] = 1.0;
    }

    auto id_mix = flowsheet_->agregar_nodo(nucleo::TipoNodo::Mezclador, "Mix Cl₂");
    if (auto* n = flowsheet_->nodo_por_id(id_mix)) {
        n->canvas_x = 185; n->canvas_y = 140;
    }

    auto id_chlor = flowsheet_->agregar_nodo(nucleo::TipoNodo::Reactor, "Chlorination");
    if (auto* n = flowsheet_->nodo_por_id(id_chlor)) {
        n->canvas_x = 340; n->canvas_y = 140;
        n->params["modo_sim"] = 1.0; // Reactions
        n->params["reaccion_id"] = 38.0; // TiO2 + 2C + 2Cl2 -> TiCl4(g) + 2CO
        n->params["conversion"] = 0.95;
        n->params["temperatura"] = 1273.15; // 1000 °C
        n->params["presion"] = 2.0;
    }

    auto id_cool1 = flowsheet_->agregar_nodo(nucleo::TipoNodo::Enfriador, "Cooling 1");
    if (auto* n = flowsheet_->nodo_por_id(id_cool1)) {
        n->canvas_x = 500; n->canvas_y = 140;
        n->params["modo_sim"] = 2.0; // Distribution
        n->params["T_salida"] = 343.15; // 70 °C
    }

    auto id_sep1 = flowsheet_->agregar_nodo(nucleo::TipoNodo::Separador, "Gas Separation 1");
    if (auto* n = flowsheet_->nodo_por_id(id_sep1)) {
        n->canvas_x = 655; n->canvas_y = 140;
        n->params["modo_sim"] = 2.0; // Distribution
        // Salida 0 va a Oxidation (TiCl4), Salida 1 remueve CO e inertes hacia Offgas
        if (co)   n->params["remover_" + std::to_string(co->id)]   = 0.99;
        if (coke) n->params["remover_" + std::to_string(coke->id)] = 0.98;
    }

    auto id_offgas = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Offgas (CO)");
    if (auto* n = flowsheet_->nodo_por_id(id_offgas)) {
        n->canvas_x = 655; n->canvas_y = 20;
    }

    auto id_o2 = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Oxygen Feed (O₂)");
    if (auto* n = flowsheet_->nodo_por_id(id_o2)) {
        n->canvas_x = 655; n->canvas_y = 290;
        if (o2) n->params["flujo_" + std::to_string(o2->id)] = 0.085;
        n->params["temperatura"] = 673.15;
        n->params["presion"] = 2.0;
    }

    auto id_oxid = flowsheet_->agregar_nodo(nucleo::TipoNodo::Reactor, "Oxidation");
    if (auto* n = flowsheet_->nodo_por_id(id_oxid)) {
        n->canvas_x = 830; n->canvas_y = 140;
        n->params["modo_sim"] = 1.0; // Reactions
        n->params["reaccion_id"] = 39.0; // TiCl4(g) + O2 -> TiO2 + 2Cl2
        n->params["conversion"] = 0.98;
        n->params["temperatura"] = 1473.15; // 1200 °C
        n->params["presion"] = 2.0;
    }

    auto id_cool2 = flowsheet_->agregar_nodo(nucleo::TipoNodo::Enfriador, "Cooling 2");
    if (auto* n = flowsheet_->nodo_por_id(id_cool2)) {
        n->canvas_x = 985; n->canvas_y = 140;
        n->params["modo_sim"] = 2.0;
        n->params["T_salida"] = 353.15; // 80 °C
    }

    auto id_sep2 = flowsheet_->agregar_nodo(nucleo::TipoNodo::Separador, "Gas Separation 2");
    if (auto* n = flowsheet_->nodo_por_id(id_sep2)) {
        n->canvas_x = 1135; n->canvas_y = 140;
        n->params["modo_sim"] = 2.0; // Distribution
        // Salida 0 = TiO2 Pigment Product, Salida 1 = Reciclo de Cl2 (88% reciclado para convergencia estable)
        if (cl2)   n->params["remover_" + std::to_string(cl2->id)]   = 0.88;
        if (ticl4) n->params["remover_" + std::to_string(ticl4->id)] = 0.85;
    }

    auto id_prod = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Product TiO₂ Pigment");
    if (auto* n = flowsheet_->nodo_por_id(id_prod)) {
        n->canvas_x = 1300; n->canvas_y = 140;
    }

    flowsheet_->conectar(id_feed, id_mix);
    flowsheet_->conectar(id_mix, id_chlor);
    flowsheet_->conectar(id_chlor, id_cool1);
    flowsheet_->conectar(id_cool1, id_sep1);
    flowsheet_->conectar(id_sep1, id_oxid);   // salida 0: TiCl4 purificado
    flowsheet_->conectar(id_sep1, id_offgas); // salida 1: CO offgas
    flowsheet_->conectar(id_o2, id_oxid);
    flowsheet_->conectar(id_oxid, id_cool2);
    flowsheet_->conectar(id_cool2, id_sep2);
    flowsheet_->conectar(id_sep2, id_prod);   // salida 0: TiO2 producto
    flowsheet_->conectar(id_sep2, id_mix);    // salida 1: reciclo de Cl2

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-10, 20);
    lienzo_.modo_visualizacion_flujos = 1; // Amount (t/h)
    lienzo_.sankey_activo = true;          // Activar Sankey automáticamente como en Screenshot 2
    panel_propiedades_.seleccionar_nodo(id_chlor);
}


}} // namespace

#pragma GCC diagnostic pop
