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
static std::string origen_del_flujo(const cma::diagrama_flujo::Flowsheet& fs, int64_t fid) {
    for (auto& n : fs.nodos())
        for (auto& sfid : n.ids_flujos_salida)
            if (sfid == fid) return n.nombre;
    return "?";
}
/// Helper: find destination node name for a flow ID
static std::string destino_del_flujo(const cma::diagrama_flujo::Flowsheet& fs, int64_t fid) {
    for (auto& n : fs.nodos())
        for (auto& efid : n.ids_flujos_entrada)
            if (efid == fid) return n.nombre;
    return "?";
}

void Aplicacion::ejecutar_simulacion() {
    auto& gestor = base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();

    panel_resultados_.limpiar();

    // ── Validación previa ──
    bool tiene_fuente = false;
    bool tiene_sumidero = false;
    for (const auto& nodo : flowsheet_->nodos()) {
        if (nodo.tipo == nucleo::TipoNodo::Fuente) { tiene_fuente = true; }
        if (nodo.tipo == nucleo::TipoNodo::Sumidero) { tiene_sumidero = true; }
    }
    if (!tiene_fuente || !tiene_sumidero) {
        std::string err = "No se puede simular: el flowsheet necesita al menos ";
        if (!tiene_fuente && !tiene_sumidero) err += "una Fuente y un Sumidero.";
        else if (!tiene_fuente) err += "una Fuente de entrada.";
        else err += "un Sumidero de salida.";
        panel_resultados_.poner_resultado_masa(err, false);
        sim_resuelto_ = true;
        sim_ultimo_resultado_.convergio = false;
        sim_ultimo_resultado_.mensaje = err;
        return;
    }
    if (flowsheet_->conteo_flujos() == 0) {
        std::string err = "No se puede simular: no hay flujos de conexión. Conecte los equipos.";
        panel_resultados_.poner_resultado_masa(err, false);
        sim_resuelto_ = true;
        sim_ultimo_resultado_.convergio = false;
        sim_ultimo_resultado_.mensaje = err;
        return;
    }

    // ── Balance de masa (SteadyStateEngine) ──
    bool masa_ok = false;
    std::string masa_msg;
    try {
        application::SimulationService simulation(repository_);
        auto res_masa = simulation.run_steady_state(*flowsheet_);
        masa_ok = res_masa.convergio;
        masa_msg = res_masa.mensaje;

        // Write solved stream properties back to flowsheet
        if (res_masa.convergio) {
            for (auto& f_res : res_masa.flujos_salida) {
                auto* f_orig = flowsheet_->flujo_por_id(f_res.id);
                if (f_orig) {
                    f_orig->flujo_molar = f_res.flujo_molar;
                    f_orig->temperatura = f_res.temperatura;
                    f_orig->presion = f_res.presion;
                    f_orig->entalpia = f_res.entalpia;
                    f_orig->energia = f_res.energia;
                    f_orig->fase = f_res.fase;
                    f_orig->es_energia = f_res.es_energia;
                    f_orig->flujo_solido = f_res.flujo_solido;
                    f_orig->flujo_liquido = f_res.flujo_liquido;
                    f_orig->flujo_gas = f_res.flujo_gas;
                    f_orig->fraccion_solido = f_res.fraccion_solido;
                    f_orig->fraccion_liquido = f_res.fraccion_liquido;
                    f_orig->fraccion_gas = f_res.fraccion_gas;
                }
            }
        }

        std::string msg = res_masa.convergio
            ? "[OK] Balance de masa convergido."
            : "[ERROR] " + res_masa.mensaje;

        // Resumen de flujos
        msg += "\n-- Flujos del flowsheet --\n";
        int nflujo = 0;
        for (auto& [fid, flujo] : flowsheet_->mapa_flujos()) {
            if (flujo.es_energia) continue;
            nflujo++;
            double total_kmol = 0.0;
            for (auto& [_, n] : flujo.flujo_molar) total_kmol += n;
            char buf[256];
            snprintf(buf, sizeof(buf), "  #%d: %s -> %s  (%.3f kmol/s  T=%.0fK)",
                     nflujo,
                     origen_del_flujo(*flowsheet_, fid).c_str(),
                     destino_del_flujo(*flowsheet_, fid).c_str(),
                     total_kmol, flujo.temperatura);
            msg += buf;
            msg += "\n";
        }

        // Tabla de especies por flujo (hasta 4 flujos, máx 4 especies)
        msg += "\n-- Especies por flujo (kmol/s) --\n";
        int cnt = 0;
        for (auto& [fid, flujo] : flowsheet_->mapa_flujos()) {
            if (flujo.es_energia) continue;
            if (++cnt > 4) { msg += "  ...\n"; break; }
            char buf[256];
            snprintf(buf, sizeof(buf), "  #%d (%s):", cnt, origen_del_flujo(*flowsheet_, fid).c_str());
            msg += buf;
            int sp_cnt = 0;
            double total = 0.0;
            for (auto& [sid, n] : flujo.flujo_molar) total += n;
            for (auto& [sid, n] : flujo.flujo_molar) {
                if (++sp_cnt > 4) { msg += " ..."; break; }
                auto* sp = gestor.especie_por_id(sid);
                char buf2[128];
                double fracc = total > 0 ? n / total : 0.0;
                if (n < 1e-4) continue;
                snprintf(buf2, sizeof(buf2), " %s=%.4f(%d%%)",
                         sp ? sp->nombre.c_str() : "?", n, (int)(fracc * 100 + 0.5));
                msg += buf2;
            }
            msg += "\n";
        }

        msg += "\n" + std::to_string(flowsheet_->conteo_flujos()) + " flujos materiales totales.\n";

        panel_resultados_.poner_resultado_masa(msg, res_masa.convergio);
    } catch (const std::exception& e) {
        panel_resultados_.poner_resultado_masa(std::string("Error (masa): ") + e.what(), false);
    }

    // ── Balance de energía (HeatMaterialBalance) ──
    try {
        application::modules::HeatMaterialBalance hmb;
        double H_fuentes = 0.0;
        double H_sumideros = 0.0;
        double Q_neto_equipos = 0.0;
        bool equipos_ok = true;
        std::string en_msg;

        for (auto& nodo : flowsheet_->nodos()) {
            std::vector<nucleo::Flujo> entradas, salidas;
            auto& mapa_f = flowsheet_->mapa_flujos();
            for (auto& fid : nodo.ids_flujos_entrada)
                if (auto it = mapa_f.find(fid); it != mapa_f.end())
                    entradas.push_back(it->second);
            for (auto& fid : nodo.ids_flujos_salida)
                if (auto it = mapa_f.find(fid); it != mapa_f.end())
                    salidas.push_back(it->second);

            auto res_nodo = hmb.calcular(nodo, entradas, salidas, db);
            
            if (nodo.tipo == nucleo::TipoNodo::Fuente) {
                H_fuentes += res_nodo.entalpia_salida;
            } else if (nodo.tipo == nucleo::TipoNodo::Sumidero) {
                H_sumideros += res_nodo.entalpia_entrada;
            } else {
                Q_neto_equipos += res_nodo.calor;
                if (!res_nodo.balanceado) equipos_ok = false;
            }

            char buf[256];
            snprintf(buf, sizeof(buf), "  %s: H_in=%.1f  H_out=%.1f  dH=%+.1f kJ/s",
                     nodo.nombre.c_str(),
                     res_nodo.entalpia_entrada,
                     res_nodo.entalpia_salida,
                     res_nodo.calor);
            en_msg += buf;
            en_msg += "\n";
        }

        char sum[1024];
        snprintf(sum, sizeof(sum),
                 "-- Balance energetico Global --\n"
                 "Entalpia de Reactivos (Fuentes): %+.1f kJ/s\n"
                 "Entalpia de Productos (Sumideros): %+.1f kJ/s\n"
                 "Calor Neto Requerido por Proceso: %+.1f kJ/s\n"
                 "Estado termico de equipos: %s\n\n"
                 "-- Detalle por equipo --\n",
                 H_fuentes, H_sumideros, Q_neto_equipos,
                 equipos_ok ? "Balanceado (Autotérmico)" : "Requiere calefacción/refrigeración externa");
        en_msg = sum + en_msg;

        panel_resultados_.poner_resultado_energia(en_msg, true);
        
        // ── BALANCE DE EXERGÍA ──
        try {
            motores::BalanceExergia motor_exergia;
            auto res_ex = motor_exergia.calcular_flowsheet(*flowsheet_, db);
            panel_resultados_.poner_resultado_exergia(res_ex, true, "Balance exergético calculado exitosamente.");
        } catch (const std::exception& e) {
            motores::ResultadoBalanceExergia res_vacio;
            panel_resultados_.poner_resultado_exergia(res_vacio, false, std::string("Error (exergía): ") + e.what());
        }

    } catch (const std::exception& e) {
        panel_resultados_.poner_resultado_energia(std::string("Error (energía): ") + e.what(), false);
    }

    // Guardar estado de simulación para la barra de estado
    sim_resuelto_ = true;
    sim_ultimo_resultado_.convergio = masa_ok;
    sim_ultimo_resultado_.mensaje = masa_ok
        ? ("Simulación convergió. " + std::to_string(flowsheet_->conteo_flujos()) + " flujos.")
        : ("Error: " + masa_msg);
}

void Aplicacion::exportar_resultados_csv() {
    std::string filepath = file_dialog_.select("Exportar Resultados CSV", "CSV | *.csv", true);
    if (!filepath.empty() && filepath.back() == '\n') filepath.pop_back();
    if (filepath.empty()) return;
    if (filepath.size() < 4 || filepath.substr(filepath.size() - 4) != ".csv")
        filepath += ".csv";

    auto& gestor = base_datos::GestorDB::instancia();

    std::ostringstream ss;
    ss << "Flujo_ID,Nombre,Origen,Destino,Temperatura(K),Presion(atm),Flujo_Total(kmol/s),Especie,Flujo_Especie(kmol/s),Fase\n";

    for (auto& [fid, flujo] : flowsheet_->mapa_flujos()) {
        if (flujo.es_energia) {
            ss << fid << "," << flujo.nombre << ",,,"
               << flujo.temperatura << "," << flujo.presion << ","
               << flujo.energia << ",(Energia kW),,\n";
            continue;
        }
        double total = flujo.flujo_total();
        std::string origen = origen_del_flujo(*flowsheet_, fid);
        std::string destino = destino_del_flujo(*flowsheet_, fid);

        if (flujo.flujo_molar.empty()) {
            ss << fid << "," << flujo.nombre << "," << origen << "," << destino
               << "," << flujo.temperatura << "," << flujo.presion << ","
               << total << ",(vacio),,\n";
        } else {
            for (auto& [sid, n] : flujo.flujo_molar) {
                const auto* esp = gestor.especie_por_id(sid);
                std::string nombre_esp = esp ? esp->nombre : ("ID_" + std::to_string(sid));
                ss << fid << "," << flujo.nombre << "," << origen << "," << destino
                   << "," << flujo.temperatura << "," << flujo.presion << ","
                   << total << "," << nombre_esp << "," << n << ","
                   << nucleo::nombre_fase(flujo.fase) << "\n";
            }
        }
    }

    std::ofstream f(filepath, std::ios::binary);
    if (f.is_open()) {
        f << ss.rdbuf();
        f.close();
        panel_resultados_.poner_resultado_masa("Resultados exportados: " + filepath, true);
    } else {
        panel_resultados_.poner_resultado_masa("Error al exportar CSV: " + filepath, false);
    }
}

void Aplicacion::ejecutar_gibbs() {
    auto& gestor = base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();

    try {
        application::modules::GibbsMinimizer gibbs;
        std::map<int64_t, double> alimentacion;
        for (auto& nodo : flowsheet_->nodos()) {
            if (nodo.tipo == nucleo::TipoNodo::Fuente)
                for (auto& [k, v] : nodo.params)
                    if (k.rfind("flujo_", 0) == 0) {
                        try {
                            size_t pos;
                            int64_t sid = std::stoll(k.substr(6), &pos);
                            if (sid > 0 && pos == k.size() - 6)
                                alimentacion[sid] += v;
                        } catch (...) {}
                    }
        }
        if (alimentacion.empty()) {
            auto* h2o = gestor.especie_por_nombre("Agua");
            auto* co2 = gestor.especie_por_nombre("Dióxido de Carbono");
            if (h2o) alimentacion[h2o->id] = 1.0;
            if (co2) alimentacion[co2->id] = 1.0;
        }
        auto res = gibbs.minimizar(alimentacion, 1000.0, 1.0, db);
        panel_resultados_.poner_resultado_gibbs(res.mensaje, res.convergio, res.flujo_molar, db);
    } catch (const std::exception& e) {
        panel_resultados_.poner_resultado_gibbs(std::string("Error: ") + e.what(), false, {}, db);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Paneles
// ═══════════════════════════════════════════════════════════════════════════



// ═══════════════════════════════════════════════════════════════════════════
// Atajos de teclado globales
// ═══════════════════════════════════════════════════════════════════════════


}} // namespace
