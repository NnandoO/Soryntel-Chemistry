#include "motores/balance_masa.hpp"
#include "base_datos/gestor_db.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <set>
#include <sstream>

namespace cma { namespace motores {

ResultadoBalanceMasa BalanceMasa::resolver(const diagrama_flujo::Flowsheet& fs) {
    ResultadoBalanceMasa res;
    auto nodos = fs.nodos();
    if (nodos.empty()) { res.mensaje = "El diagrama está vacío."; return res; }

    auto orden = fs.orden_topologico();
    if (orden.empty()) { res.mensaje = "Ciclo detectado en el diagrama."; return res; }

    auto mapa_flujos = fs.mapa_flujos();
    std::map<int64_t, nucleo::Flujo> flujos_trabajo = mapa_flujos;

    const bool hay_ciclos = orden.size() < nodos.size();

    if (!hay_ciclos) {
        // ── Circuito acíclico: un solo barrido en orden topológico ──
        for (int64_t nid : orden) {
            const auto* nodo = fs.nodo_por_id(nid);
            if (!nodo) continue;
            try {
                procesar_nodo(*nodo, flujos_trabajo);
            } catch (const std::exception& e) {
                res.mensaje = "Error en nodo " + std::to_string(nodo->id) + ": " + e.what();
                return res;
            }
        }
        res.convergio = true;
        res.iteraciones = 1;
        res.mensaje = "Balance de masa resuelto exitosamente.";
    } else {
        // ── Lazo(s) de recirculación: solución secuencial modular con tearing ──
        // 1) Orden completo = orden topológico + nodos del ciclo (por id).
        std::vector<int64_t> orden_completo = orden;
        std::set<int64_t> en_orden(orden.begin(), orden.end());
        std::vector<int64_t> nodos_ciclo;
        for (auto& n : nodos)
            if (!en_orden.count(n.id)) nodos_ciclo.push_back(n.id);
        std::sort(nodos_ciclo.begin(), nodos_ciclo.end());
        orden_completo.insert(orden_completo.end(), nodos_ciclo.begin(), nodos_ciclo.end());

        // 2) Flujos de corte: todas las salidas de nodos del ciclo (son los "torn streams").
        std::vector<int64_t> flujos_corte;
        for (auto& n : nodos)
            if (!en_orden.count(n.id))
                for (int64_t fid : n.ids_flujos_salida)
                    flujos_corte.push_back(fid);

        const int    MAX_ITER = 1000;
        const double TOL      = 1e-9;

        double error = 1e30;
        int iter = 0;
        std::map<int64_t, double> x_prev, f_prev;
        for (; iter < MAX_ITER; ++iter) {
            // Instantánea de los flujos de corte antes del barrido
            std::map<int64_t, double> antes;
            for (int64_t fid : flujos_corte) {
                auto it = flujos_trabajo.find(fid);
                antes[fid] = it != flujos_trabajo.end() ? it->second.flujo_total() : 0.0;
            }

            // Barrido completo del flowsheet (las corrientes de corte conservan
            // su valor de la iteración anterior como estimación inicial)
            for (int64_t nid : orden_completo) {
                const auto* nodo = fs.nodo_por_id(nid);
                if (!nodo) continue;
                try {
                    procesar_nodo(*nodo, flujos_trabajo);
                } catch (const std::exception& e) {
                    res.mensaje = "Error en nodo " + std::to_string(nodo->id) + ": " + e.what();
                    return res;
                }
            }

            // Error = máximo cambio relativo de los flujos de corte
            error = 0.0;
            for (int64_t fid : flujos_corte) {
                auto it = flujos_trabajo.find(fid);
                double f_val = it != flujos_trabajo.end() ? it->second.flujo_total() : 0.0;
                double x_val = antes[fid];
                double e = std::abs(f_val - x_val);
                if (x_val > 1e-12) e = std::abs((f_val - x_val) / x_val);
                if (e > error) error = e;

                double x_next = f_val;
                if (iter > 0) {
                    double dx = x_val - x_prev[fid];
                    double df = f_val - f_prev[fid];
                    if (std::abs(dx) > 1e-12) {
                        double s = df / dx;
                        s = std::clamp(s, -5.0, -0.1);
                        double q = s / (s - 1.0);
                        x_next = q * x_val + (1.0 - q) * f_val;
                    }
                }
                x_prev[fid] = x_val;
                f_prev[fid] = f_val;

                if (it != flujos_trabajo.end() && f_val > 1e-12 && x_next > 0) {
                    double factor = x_next / f_val;
                    for (auto& [sid, n] : it->second.flujo_molar) {
                        n *= factor;
                    }
                }
            }

            if (error < TOL) break;
        }

        res.iteraciones = iter + 1;
        res.error_final = error;
        res.convergio = error < TOL;
        res.mensaje = res.convergio
            ? "Balance de masa convergido con recirculación en " + std::to_string(iter + 1) + " iteraciones"
            : "No convergió después de " + std::to_string(MAX_ITER) + " iteraciones (error=" + std::to_string(error) + ")";
    }

    // ── Verificación global: masa entrante (fuentes) vs masa saliente (sumideros) ──
    auto& gestor = base_datos::GestorDB::instancia();
    double masa_in_total = 0.0, masa_out_total = 0.0;
    auto masa_flujo = [&gestor](const nucleo::Flujo& f) {
        double m = 0.0;
        for (auto& [sid, n] : f.flujo_molar) {
            auto* sp = gestor.especie_por_id(sid);
            m += n * (sp ? sp->masa_molar : 0.0);
        }
        return m;
    };
    for (auto& n : nodos) {
        if (n.tipo == nucleo::TipoNodo::Fuente) {
            for (int64_t fid : n.ids_flujos_salida)
                if (auto it = flujos_trabajo.find(fid); it != flujos_trabajo.end())
                    masa_in_total += masa_flujo(it->second);
        } else if (n.tipo == nucleo::TipoNodo::Sumidero) {
            for (int64_t fid : n.ids_flujos_entrada)
                if (auto it = flujos_trabajo.find(fid); it != flujos_trabajo.end())
                    masa_out_total += masa_flujo(it->second);
        }
    }
    if (masa_in_total > 0.0 || masa_out_total > 0.0) {
        double dif = masa_out_total - masa_in_total;
        res.balance_global[-1] = dif; // clave -1 = total global (kg/s)
        double tol = 1e-6 + 1e-4 * std::max(masa_in_total, masa_out_total);
        char buf[256];
        snprintf(buf, sizeof(buf),
                 " | Entrada fuentes=%.3f kg/s, Salida sumideros=%.3f kg/s, Diferencia=%+.3e kg/s%s",
                 masa_in_total, masa_out_total, dif,
                 std::abs(dif) < tol ? " (balance cerrado)" : "");
        res.mensaje += buf;
    }

    for (auto& [_, f] : flujos_trabajo)
        res.flujos_salida.push_back(f);
    return res;
}

void BalanceMasa::procesar_nodo(const nucleo::Nodo& nodo,
                                std::map<int64_t, nucleo::Flujo>& flujos_trabajo) const
{
    std::vector<nucleo::Flujo> entradas;
    for (int64_t fid : nodo.ids_flujos_entrada) {
        auto it = flujos_trabajo.find(fid);
        if (it != flujos_trabajo.end()) entradas.push_back(it->second);
    }

    switch (nodo.tipo) {
        case nucleo::TipoNodo::Fuente: {
            nucleo::Flujo salida;
            salida.id = nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0];
            salida.nombre = nodo.nombre + "_salida";
            salida.temperatura = nodo.params.count("temperatura") ? nodo.params.at("temperatura") : nucleo::STP_TEMP;
            salida.presion = nodo.params.count("presion") ? nodo.params.at("presion") : nucleo::STP_PRES;
            for (auto& [k, v] : nodo.params) {
                if (k.rfind("flujo_", 0) == 0) {
                    try {
                        size_t pos;
                        int64_t sid = std::stoll(k.substr(6), &pos);
                        if (sid > 0 && pos == k.size() - 6) {
                            double val = std::max(0.0, v);
                            salida.flujo_molar[sid] = val;
                        }
                    } catch (...) {}
                }
            }
            if (!nodo.ids_flujos_salida.empty())
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            break;
        }
        case nucleo::TipoNodo::Sumidero:
            break;

        case nucleo::TipoNodo::Mezclador: {
            auto salida = mezclar(entradas,
                nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0]);
            auto itP = nodo.params.find("presion_salida");
            if (itP != nodo.params.end() && itP->second > 0.0)
                salida.presion = itP->second;
            if (!nodo.ids_flujos_salida.empty())
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            break;
        }
        case nucleo::TipoNodo::Separador: {
            if (entradas.empty()) break;
            std::map<int64_t, double> remocion;
            for (auto& [k, v] : nodo.params) {
                if (k.rfind("remover_", 0) == 0) {
                    try {
                        size_t pos;
                        int64_t sid = std::stoll(k.substr(8), &pos);
                        if (sid > 0 && pos == k.size() - 8)
                            remocion[sid] = v;
                    } catch (...) {}
                }
            }
            // Salida 0: corriente "limpia" (especies no removidas)
            auto salida = separador(entradas[0], remocion,
                nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0]);
            if (!nodo.ids_flujos_salida.empty())
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            // Salida 1: corriente removida (reciclo del reactivo)
            if (nodo.ids_flujos_salida.size() >= 2) {
                nucleo::Flujo removido;
                removido.id = nodo.ids_flujos_salida[1];
                removido.nombre = nodo.nombre + "_removido";
                removido.temperatura = entradas[0].temperatura;
                removido.presion = entradas[0].presion;
                for (auto& [sid, frac] : remocion) {
                    if (frac > 0.0) {
                        double extraido = entradas[0].flujo_especie(sid) * frac;
                        if (extraido > 0.0) removido.flujo_molar[sid] = extraido;
                    }
                }
                flujos_trabajo[nodo.ids_flujos_salida[1]] = removido;
            }
            break;
        }
        case nucleo::TipoNodo::Compresor: {
            if (entradas.empty()) break;
            double rP = nodo.params.count("relacion_presion") ? nodo.params.at("relacion_presion") : 2.0;
            double eff = nodo.params.count("eficiencia") ? nodo.params.at("eficiencia") : 0.85;
            auto salida = compresor(entradas[0], rP, eff,
                nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0]);
            if (!nodo.ids_flujos_salida.empty())
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            break;
        }
        case nucleo::TipoNodo::Válvula: {
            if (entradas.empty()) break;
            double P_sal = nodo.params.count("P_salida") ? nodo.params.at("P_salida") : entradas[0].presion * 0.5;
            auto salida = valvula(entradas[0], P_sal,
                nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0]);
            if (!nodo.ids_flujos_salida.empty())
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            break;
        }
        case nucleo::TipoNodo::Multiplexor: {
            if (entradas.empty()) break;
            auto salida = mezclar(entradas,
                nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0]);
            if (!nodo.ids_flujos_salida.empty())
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            break;
        }
        case nucleo::TipoNodo::Demultiplexor:
        case nucleo::TipoNodo::Divisor: {
            if (entradas.empty()) break;
            auto fracciones = nodo.fracciones_divisor();
            if (fracciones.empty()) {
                size_t n = nodo.ids_flujos_salida.size();
                if (n == 0) break;
                fracciones.resize(n, 1.0 / n);
            }
            auto salidas = dividir(entradas[0], fracciones,
                nodo.ids_flujos_salida.empty() ? 0 : nodo.ids_flujos_salida[0]);
            for (size_t i = 0; i < salidas.size() && i < nodo.ids_flujos_salida.size(); ++i)
                flujos_trabajo[nodo.ids_flujos_salida[i]] = salidas[i];
            break;
        }

        // ── Reactores alternativos: misma mecánica que Reactor ──
        case nucleo::TipoNodo::ReactorGibbs:
        case nucleo::TipoNodo::ReactorEsteq:
        case nucleo::TipoNodo::ReactorCinetica:
        case nucleo::TipoNodo::Reactor: {
            if (entradas.empty()) break;
            double conv = nodo.conversion();
            double T = nodo.params.count("temperatura") ? nodo.params.at("temperatura") : entradas[0].temperatura;
            double P = nodo.params.count("presion") ? nodo.params.at("presion") : entradas[0].presion;
            int64_t rxn_id = nodo.params.count("reaccion_id") ? (int64_t)nodo.params.at("reaccion_id") : 1;
            auto& db = cma::base_datos::GestorDB::instancia();
            const auto* rxn = db.reaccion_por_id(rxn_id);
            if (!rxn) {
                // Sin reacción definida: pasar flujo sin convertir (más seguro que inventar reacción)
                auto salida = mezclar(entradas,
                    nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0]);
                salida.temperatura = T;
                salida.presion = P;
                if (!nodo.ids_flujos_salida.empty())
                    flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            } else {
                auto salida = reactor(entradas, *rxn, conv, T, P,
                    nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0]);
                if (!nodo.ids_flujos_salida.empty())
                    flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            }
            break;
        }

        // ── Unidades de separación por especie (flotación, espesador, filtro, etc.) ──
        case nucleo::TipoNodo::CeldaFlotacion:
        case nucleo::TipoNodo::Espesador:
        case nucleo::TipoNodo::Filtro:
        case nucleo::TipoNodo::Hidrociclon:
        case nucleo::TipoNodo::Zaranda:
        case nucleo::TipoNodo::ColumnaDestilacion:
        case nucleo::TipoNodo::Absorbedor:
        case nucleo::TipoNodo::Extraccion:
        case nucleo::TipoNodo::LavadorGases:
        case nucleo::TipoNodo::TanqueLixiviacion:
        case nucleo::TipoNodo::SXMixerSettler:
        case nucleo::TipoNodo::CeldaElectroobtencion:
            procesar_separacion(nodo, entradas, flujos_trabajo);
            break;

        // ── Hornos / tostador / convertidor: reacción + división ──
        case nucleo::TipoNodo::HornoRotatorio:
        case nucleo::TipoNodo::HornoFlash:
        case nucleo::TipoNodo::HornoArco:
        case nucleo::TipoNodo::HornoReverbero:
        case nucleo::TipoNodo::Tostador:
        case nucleo::TipoNodo::Convertidor:
            procesar_reactor_generico(nodo, entradas, flujos_trabajo);
            break;

        // ── Térmicos / mecánicos de fluido ──
        case nucleo::TipoNodo::Calentador:
        case nucleo::TipoNodo::Enfriador:
        case nucleo::TipoNodo::IntercambiadorCalor: {
            if (entradas.empty()) break;
            double T_sal = nodo.temperatura_salida();
            double dP = 0;
            for (auto& [k, v] : nodo.params)
                if (k == "caida_presion") dP = v;
            auto salida = intercambiador_calor(entradas[0], T_sal, dP,
                nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0]);
            if (!nodo.ids_flujos_salida.empty())
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            break;
        }
        case nucleo::TipoNodo::Bomba: {
            if (entradas.empty()) break;
            double P_sal = nodo.params.count("P_salida") ? nodo.params.at("P_salida")
                         : entradas[0].presion * 1.5;
            nucleo::Flujo salida = entradas[0];
            salida.id = nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0];
            salida.nombre = "bomba_salida";
            salida.presion = P_sal;
            if (!nodo.ids_flujos_salida.empty())
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            break;
        }
        case nucleo::TipoNodo::Turbina: {
            if (entradas.empty()) break;
            double P_sal = nodo.params.count("P_salida") ? nodo.params.at("P_salida")
                         : entradas[0].presion * 0.5;
            double eff = nodo.params.count("eficiencia") ? nodo.params.at("eficiencia") : 0.85;
            // Gamma: permite override via params["gamma"], default 1.4 (gas diatómico)
            double gamma = nodo.params.count("gamma") ? nodo.params.at("gamma") : 1.4;
            nucleo::Flujo salida = entradas[0];
            salida.id = nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0];
            salida.nombre = "turbina_salida";
            salida.presion = P_sal;
            double relacion = entradas[0].presion > 0 ? entradas[0].presion / P_sal : 1.0;
            if (relacion > 1.0 && gamma > 1.0) {
                salida.temperatura = entradas[0].temperatura *
                    std::pow(1.0 / relacion, (gamma - 1.0) / (gamma * eff));
            }
            if (!nodo.ids_flujos_salida.empty())
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            break;
        }

        default: {
            if (entradas.empty()) break;
            auto flujo_mezclado = mezclar(entradas, -1);
            if (nodo.ids_flujos_salida.empty()) break;
            size_t n = nodo.ids_flujos_salida.size();
            if (n == 1) {
                flujo_mezclado.id = nodo.ids_flujos_salida[0];
                flujos_trabajo[nodo.ids_flujos_salida[0]] = flujo_mezclado;
            } else {
                std::vector<double> fracciones(n, 1.0 / n);
                auto salidas = dividir(flujo_mezclado, fracciones, nodo.ids_flujos_salida[0]);
                for (size_t i = 0; i < salidas.size() && i < nodo.ids_flujos_salida.size(); ++i) {
                    flujos_trabajo[nodo.ids_flujos_salida[i]] = salidas[i];
                }
            }
            break;
        }
    }
}

double BalanceMasa::fraccion_hacia_salida1(const nucleo::Nodo& nodo, int64_t sid, double por_defecto) const {
    auto it = nodo.params.find("split_" + std::to_string(sid));
    return it != nodo.params.end() ? it->second : por_defecto;
}

void BalanceMasa::procesar_separacion(const nucleo::Nodo& nodo,
                                      const std::vector<nucleo::Flujo>& entradas,
                                      std::map<int64_t, nucleo::Flujo>& flujos_trabajo) const
{
    if (entradas.empty()) return;
    if (nodo.ids_flujos_salida.size() < 2) return;
    auto mezcla = combinar_entradas(entradas);
    auto& gestor = base_datos::GestorDB::instancia();

    nucleo::Flujo sal0;
    sal0.id = nodo.ids_flujos_salida[0];
    sal0.nombre = nodo.nombre + "_salida0";
    sal0.temperatura = mezcla.temperatura;
    sal0.presion = mezcla.presion;
    nucleo::Flujo sal1;
    sal1.id = nodo.ids_flujos_salida[1];
    sal1.nombre = nodo.nombre + "_salida1";
    sal1.temperatura = mezcla.temperatura;
    sal1.presion = mezcla.presion;

    double recuperacion = nodo.params.count("recuperacion") ? nodo.params.at("recuperacion")
                        : (nodo.params.count("recovery") ? nodo.params.at("recovery") : 0.85);
    double efficiency    = nodo.params.count("efficiency") ? nodo.params.at("efficiency") : 0.98;
    double bypass        = nodo.params.count("bypass") ? nodo.params.at("bypass") : 0.15;

    for (auto& [sid, n] : mezcla.flujo_molar) {
        if (n <= 0.0) continue;
        const auto* sp = gestor.especie_por_id(sid);
        nucleo::Fase fase = sp ? sp->fase_std : nucleo::Fase::Desconocido;
        bool es_solido = (fase == nucleo::Fase::Solido);
        bool es_gas    = (fase == nucleo::Fase::Gas);

        double f1; // fracción a salida 1
        switch (nodo.tipo) {
            case nucleo::TipoNodo::CeldaFlotacion:
                // Concentrado = salida 0 (1 - f1); recuperación = fracción a concentrado.
                f1 = 1.0 - recuperacion;
                break;
            case nucleo::TipoNodo::Espesador:
                // Underflow = salida 1 (sólidos). Overflow = salida 0 (líquidos).
                f1 = es_solido ? efficiency : 0.0;
                break;
            case nucleo::TipoNodo::Filtro:
                // Torta (sólidos) = salida 0. Filtrado = salida 1.
                f1 = es_solido ? (1.0 - efficiency) : 1.0;
                break;
            case nucleo::TipoNodo::Hidrociclon:
                // Underflow (gruesos) = salida 1. Overflow (finos) = salida 0.
                f1 = es_solido ? 0.8 : bypass;
                break;
            case nucleo::TipoNodo::Zaranda:
                // Sobretamaño (sólidos) = salida 0. Bajo tamaño = salida 1.
                f1 = es_solido ? 0.5 : 1.0;
                break;
            case nucleo::TipoNodo::ColumnaDestilacion:
                // Destilado (gases) = salida 0. Fondos = salida 1.
                f1 = es_gas ? 0.05 : 0.95;
                break;
            case nucleo::TipoNodo::Absorbedor:
            case nucleo::TipoNodo::LavadorGases:
                // Gas limpio = salida 0. Líquido cargado = salida 1.
                f1 = es_gas ? 0.0 : 1.0;
                break;
            case nucleo::TipoNodo::TanqueLixiviacion:
                // Solución cargada (líquidos) = salida 0. Residuo (sólidos) = salida 1.
                f1 = es_solido ? 1.0 : 0.0;
                break;
            default: // SX, Extracción, Electroobtención
                f1 = 0.5;
                break;
        }
        f1 = fraccion_hacia_salida1(nodo, sid, f1);
        if (f1 > 1.0) f1 = 1.0;
        if (f1 < 0.0) f1 = 0.0;
        sal1.flujo_molar[sid] = n * f1;
        sal0.flujo_molar[sid] = n * (1.0 - f1);
    }

    // Espesador: ajustar agua de descarga para alcanzar sólidos wt% (si es posible)
    if (nodo.tipo == nucleo::TipoNodo::Espesador &&
        nodo.params.count("solids_underflow") > 0) {
        double ws = nodo.params.at("solids_underflow") / 100.0;
        double masa_solidos = 0, masa_agua = 0;
        int64_t id_agua = -1;
        for (auto& sp : gestor.todas_especies())
            if (sp->nombre == "Agua") { id_agua = sp->id; break; }
        for (auto& [sid, n] : sal1.flujo_molar) {
            auto* sp = gestor.especie_por_id(sid);
            double M = sp ? sp->masa_molar : 0.0;
            if (sid == id_agua) masa_agua += n * M;
            else if (sp && sp->fase_std == nucleo::Fase::Solido) masa_solidos += n * M;
        }
        if (masa_solidos > 0 && ws > 0 && ws < 1) {
            double agua_necesaria = masa_solidos * (1.0 - ws) / ws - masa_agua;
            if (agua_necesaria > 0) {
                // Quitar agua del overflow (salida 0) hacia el underflow
                const auto* sp_agua = gestor.especie_por_id(id_agua);
                double M_agua = sp_agua ? sp_agua->masa_molar : 0.0;
                double agua_disp = id_agua >= 0 && M_agua > 0
                    ? sal0.flujo_especie(id_agua) * M_agua : 0.0;
                double a_quitar = std::min(agua_necesaria, agua_disp);
                if (id_agua >= 0 && a_quitar > 0) {
                    double kmol = a_quitar / M_agua;
                    sal0.flujo_molar[id_agua] = std::max(0.0, sal0.flujo_molar[id_agua] - kmol);
                    sal1.flujo_molar[id_agua] += kmol;
                }
            }
        }
    }

    flujos_trabajo[nodo.ids_flujos_salida[0]] = sal0;
    flujos_trabajo[nodo.ids_flujos_salida[1]] = sal1;
}

void BalanceMasa::procesar_reactor_generico(const nucleo::Nodo& nodo,
                                            const std::vector<nucleo::Flujo>& entradas,
                                            std::map<int64_t, nucleo::Flujo>& flujos_trabajo) const
{
    if (entradas.empty()) return;
    auto mezcla = combinar_entradas(entradas);

    nucleo::Flujo producto = mezcla;
    // Aplicar temperatura/presión del nodo aunque no haya reacción
    if (nodo.params.count("temperatura"))
        producto.temperatura = nodo.params.at("temperatura");
    if (nodo.params.count("presion"))
        producto.presion = nodo.params.at("presion");
    if (nodo.params.count("reaccion_id")) {
        double conv = nodo.conversion();
        double T = nodo.params.count("temperatura") ? nodo.params.at("temperatura") : mezcla.temperatura;
        double P = nodo.params.count("presion") ? nodo.params.at("presion") : mezcla.presion;
        int64_t rxn_id = (int64_t)nodo.params.at("reaccion_id");
        auto& db = cma::base_datos::GestorDB::instancia();
        const auto* rxn = db.reaccion_por_id(rxn_id);
        if (rxn) producto = reactor(entradas, *rxn, conv, T, P, -1);
    }

    size_t n = nodo.ids_flujos_salida.size();
    if (n == 0) return;
    if (n == 1) {
        producto.id = nodo.ids_flujos_salida[0];
        producto.nombre = nodo.nombre + "_salida";
        flujos_trabajo[nodo.ids_flujos_salida[0]] = producto;
        return;
    }
    // Con 2 salidas: distribuir por especie (param "split_<id>", default 50/50).
    if (n == 2) {
        nucleo::Flujo sal0, sal1;
        sal0.id = nodo.ids_flujos_salida[0];
        sal1.id = nodo.ids_flujos_salida[1];
        sal0.nombre = nodo.nombre + "_salida0";
        sal1.nombre = nodo.nombre + "_salida1";
        sal0.temperatura = producto.temperatura; sal0.presion = producto.presion;
        sal1.temperatura = producto.temperatura; sal1.presion = producto.presion;
        for (auto& [sid, nsp] : producto.flujo_molar) {
            double f1 = fraccion_hacia_salida1(nodo, sid, 0.5);
            sal1.flujo_molar[sid] = nsp * f1;
            sal0.flujo_molar[sid] = nsp * (1.0 - f1);
        }
        flujos_trabajo[nodo.ids_flujos_salida[0]] = sal0;
        flujos_trabajo[nodo.ids_flujos_salida[1]] = sal1;
        return;
    }
    std::vector<double> fracciones(n, 1.0 / n);
    bool hay_fracs = false;
    for (size_t i = 0; i < n; ++i)
        if (nodo.params.count("frac_" + std::to_string(i))) {
            fracciones[i] = nodo.params.at("frac_" + std::to_string(i));
            hay_fracs = true;
        }
    if (hay_fracs) {
        double suma = 0;
        for (double f : fracciones) suma += f;
        if (suma > 0) for (double& f : fracciones) f /= suma;
    }
    auto salidas = dividir(producto, fracciones, nodo.ids_flujos_salida[0]);
    for (size_t i = 0; i < salidas.size() && i < nodo.ids_flujos_salida.size(); ++i)
        flujos_trabajo[nodo.ids_flujos_salida[i]] = salidas[i];
}

nucleo::Flujo BalanceMasa::combinar_entradas(const std::vector<nucleo::Flujo>& entradas) const {
    nucleo::Flujo out;
    for (auto& e : entradas) {
        for (auto& [sid, n] : e.flujo_molar) out.flujo_molar[sid] += n;
    }
    double tot = out.flujo_total();
    if (tot > 0) {
        double T_pond = 0, P_pond = 0;
        for (auto& e : entradas) {
            double ft = e.flujo_total();
            if (ft > 0) {
                T_pond += e.temperatura * ft;
                P_pond += e.presion * ft;
            }
        }
        out.temperatura = T_pond / tot;
        out.presion = P_pond / tot;
    }
    return out;
}

nucleo::Flujo BalanceMasa::mezclar(const std::vector<nucleo::Flujo>& entradas, int64_t id_salida) const {
    auto out = combinar_entradas(entradas);
    out.id = id_salida;
    out.nombre = "mezcla";
    return out;
}

std::vector<nucleo::Flujo> BalanceMasa::dividir(const nucleo::Flujo& entrada,
    const std::vector<double>& fracciones, int64_t base_id) const
{
    std::vector<nucleo::Flujo> salidas;
    double total_frac = 0;
    for (double f : fracciones) total_frac += f;
    if (total_frac <= 0) return salidas;

    for (size_t i = 0; i < fracciones.size(); ++i) {
        nucleo::Flujo out;
        out.id = base_id + (int64_t)i;
        out.nombre = "division_" + std::to_string(i);
        out.temperatura = entrada.temperatura;
        out.presion = entrada.presion;
        double frac = fracciones[i] / total_frac;
        for (auto& [sid, n] : entrada.flujo_molar)
            out.flujo_molar[sid] = n * frac;
        salidas.push_back(out);
    }
    return salidas;
}

nucleo::Flujo BalanceMasa::reactor(const std::vector<nucleo::Flujo>& entradas,
    const nucleo::Reaccion& rxn, double conv, double T, double P, int64_t id_salida) const
{
    auto mezcla = combinar_entradas(entradas);
    nucleo::Flujo out;
    out.id = id_salida;
    out.nombre = "reactor_salida";
    out.temperatura = T;
    out.presion = P;

    // Encontrar reactivo limitante
    double ext_max = 1e30;
    for (auto& [sid, coeff] : rxn.estequiometria) {
        if (coeff < 0) {
            double disp = mezcla.flujo_especie(sid);
            double ext = disp / (-coeff);
            if (ext < ext_max) ext_max = ext;
        }
    }
    if (ext_max > 1e29) ext_max = 0;

    conv = std::clamp(conv, 0.0, 1.0);
    double extension = ext_max * conv;
    out.flujo_molar = mezcla.flujo_molar;
    for (auto& [sid, coeff] : rxn.estequiometria) {
        out.flujo_molar[sid] += coeff * extension;
        if (out.flujo_molar[sid] < 0) out.flujo_molar[sid] = 0;
    }

    return out;
}

nucleo::Flujo BalanceMasa::separador(const nucleo::Flujo& entrada,
    const std::map<int64_t, double>& frac_remocion, int64_t id_salida) const
{
    nucleo::Flujo out = entrada;
    out.id = id_salida;
    out.nombre = "separador_salida";

    for (auto& [sid, frac] : frac_remocion) {
        if (frac >= 1.0) out.flujo_molar.erase(sid);
        else if (frac > 0) out.flujo_molar[sid] *= (1.0 - frac);
    }
    return out;
}

nucleo::Flujo BalanceMasa::intercambiador_calor(const nucleo::Flujo& entrada,
    double T_salida, double dP, int64_t id_salida) const
{
    nucleo::Flujo out = entrada;
    out.id = id_salida;
    out.nombre = "intercambiador_salida";
    out.temperatura = T_salida;
    out.presion = std::max(0.01, entrada.presion - dP);
    return out;
}

nucleo::Flujo BalanceMasa::compresor(const nucleo::Flujo& entrada,
    double relacion_P, double eficiencia, int64_t id_salida) const
{
    nucleo::Flujo out = entrada;
    out.id = id_salida;
    out.nombre = "compresor_salida";
    out.presion = entrada.presion * relacion_P;

    // Gamma (Cp/Cv): 1.4 para gases diatómicos (default), puede sobreescribirse via params["gamma"]
    double gamma = 1.4;
    auto it_gamma = entrada.flujo_molar.begin();
    if (it_gamma != entrada.flujo_molar.end()) {
        // Intentar obtener gamma del flujo si está almacenado como parámetro
        // (por ahora, usar 1.4 como aproximación estándar)
    }
    if (relacion_P > 0 && eficiencia > 0) {
        out.temperatura = entrada.temperatura *
            std::pow(relacion_P, (gamma - 1.0) / (gamma * eficiencia));
    }
    return out;
}

nucleo::Flujo BalanceMasa::valvula(const nucleo::Flujo& entrada,
    double P_salida, int64_t id_salida) const
{
    nucleo::Flujo out = entrada;
    out.id = id_salida;
    out.nombre = "valvula_salida";
    out.presion = P_salida;
    // Expansión isoentálpica (Joule-Thomson) - T constante en aproximación
    return out;
}

Eigen::VectorXd BalanceMasa::resolver_sistema_lineal(
    const Eigen::SparseMatrix<double>& A, const Eigen::VectorXd& b) const
{
    Eigen::SparseQR<Eigen::SparseMatrix<double>, Eigen::COLAMDOrdering<int>> solver;
    solver.compute(A);
    if (solver.info() != Eigen::Success)
        return Eigen::VectorXd::Zero(b.size());
    return solver.solve(b);
}

}} // namespace
