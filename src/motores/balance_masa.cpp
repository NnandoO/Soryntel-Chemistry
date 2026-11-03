#include "motores/balance_masa.hpp"
#include "motores/mineral_processing.hpp"
#include "motores/minimizador_gibbs.hpp"
#include <algorithm>
#include <cctype>
#include <cerrno>
#include <climits>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <set>
#include <sstream>

namespace cma { namespace motores {

namespace {
bool parse_id_especie(const std::string& str, int64_t& out_id) {
    if (str.empty() || !std::isdigit(static_cast<unsigned char>(str[0]))) return false;
    char* endp = nullptr;
    errno = 0;
    const long long val = std::strtoll(str.c_str(), &endp, 10);
    if (errno == ERANGE || val <= 0 || endp != str.c_str() + str.size()) return false;
    out_id = static_cast<int64_t>(val);
    return true;
}
} // namespace

ResultadoBalanceMasa BalanceMasa::resolver(const diagrama_flujo::Flowsheet& fs, int max_iter, double tolerancia) {
    ResultadoBalanceMasa res;
    if (max_iter <= 0 || max_iter > 1000000 || !std::isfinite(tolerancia) || tolerancia <= 0) {
        res.mensaje = "Límites de convergencia inválidos"; return res;
    }
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
        // ── Lazo(s) de recirculación: solución secuencial modular con tearing (Wegstein por componente) ──
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

        const int    MAX_ITER = max_iter;
        const double TOL      = tolerancia;

        double error = 1e30;
        int iter = 0;
        // Historial por corriente y por especie para aceleración de Wegstein
        std::map<int64_t, std::map<int64_t, double>> x_prev_sp, f_prev_sp;
        std::map<int64_t, double> x_prev_agua, f_prev_agua;
        std::map<int64_t, std::vector<double>> x_prev_part, f_prev_part;

        auto wegstein_step = [](double x_val, double f_val, double& xp, double& fp, int it_idx) {
            double x_next = f_val;
            if (it_idx > 0) {
                const double dx = x_val - xp;
                const double df = f_val - fp;
                if (std::abs(dx) > 1e-12) {
                    const double s = df / dx;
                    if (std::abs(s - 1.0) < 0.02) {
                        x_next = f_val;
                    } else if (s <= -5.0) {
                        const double w = 1.0 / (1.0 - s);
                        x_next = (1.0 - w) * x_val + w * f_val;
                    } else if (s < 1.0) {
                        const double q = std::clamp(s / (s - 1.0), -20.0, 0.8);
                        x_next = q * x_val + (1.0 - q) * f_val;
                    } else {
                        const double step = (f_val - x_val) / (s - 1.0);
                        x_next = x_val - 0.9 * step;
                    }
                }
            }
            xp = x_val;
            fp = f_val;
            return std::max(0.0, x_next);
        };

        for (; iter < MAX_ITER; ++iter) {
            // Instantánea completa de los flujos de corte antes del barrido
            std::map<int64_t, nucleo::Flujo> antes;
            for (int64_t fid : flujos_corte) {
                auto it = flujos_trabajo.find(fid);
                if (it != flujos_trabajo.end()) antes[fid] = it->second;
            }

            // Barrido completo del flowsheet
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

            // Error = máximo cambio relativo por componente y flujo total en los flujos de corte
            error = 0.0;
            for (int64_t fid : flujos_corte) {
                auto it = flujos_trabajo.find(fid);
                if (it == flujos_trabajo.end()) continue;
                auto& f_cur = it->second;
                const auto& f_ant = antes[fid];

                double f_tot = f_cur.flujo_total();
                double x_tot = f_ant.flujo_total();
                double e_tot = (x_tot > 1e-12) ? std::abs((f_tot - x_tot) / x_tot) : std::abs(f_tot - x_tot);
                error = std::max(error, e_tot);

                // Unir todas las especies presentes antes y después
                std::set<int64_t> sids;
                for (const auto& [sid, _] : f_ant.flujo_molar) sids.insert(sid);
                for (const auto& [sid, _] : f_cur.flujo_molar) sids.insert(sid);

                for (int64_t sid : sids) {
                    double xv = f_ant.flujo_especie(sid);
                    double fv = f_cur.flujo_especie(sid);
                    double esp_err = (xv > 1e-12) ? std::abs((fv - xv) / xv) : std::abs(fv - xv);
                    error = std::max(error, esp_err);
                    double xn = wegstein_step(xv, fv, x_prev_sp[fid][sid], f_prev_sp[fid][sid], iter);
                    if (xn > 0.0) f_cur.flujo_molar[sid] = xn;
                    else f_cur.flujo_molar.erase(sid);
                }

                // Acelerar agua particulada
                {
                    double xv = f_ant.agua_kg_h;
                    double fv = f_cur.agua_kg_h;
                    double w_err = (xv > 1e-9) ? std::abs((fv - xv) / xv) : std::abs(fv - xv);
                    error = std::max(error, w_err);
                    f_cur.agua_kg_h = wegstein_step(xv, fv, x_prev_agua[fid], f_prev_agua[fid], iter);
                }

                // Acelerar partículas minerales si coinciden en tamaño
                if (f_cur.particulas.size() == f_ant.particulas.size() && !f_cur.particulas.empty()) {
                    auto& xp_vec = x_prev_part[fid];
                    auto& fp_vec = f_prev_part[fid];
                    if (xp_vec.size() != f_cur.particulas.size()) {
                        xp_vec.assign(f_cur.particulas.size(), 0.0);
                        fp_vec.assign(f_cur.particulas.size(), 0.0);
                    }
                    for (size_t p_i = 0; p_i < f_cur.particulas.size(); ++p_i) {
                        double xv = f_ant.particulas[p_i].mass_kg_h;
                        double fv = f_cur.particulas[p_i].mass_kg_h;
                        double p_err = (xv > 1e-9) ? std::abs((fv - xv) / xv) : std::abs(fv - xv);
                        error = std::max(error, p_err);
                        f_cur.particulas[p_i].mass_kg_h = wegstein_step(xv, fv, xp_vec[p_i], fp_vec[p_i], iter);
                    }
                }
            }

            if (error < TOL) break;
        }

        res.iteraciones = iter + 1;
        res.error_final = error;
        res.convergio = error < TOL;
        if (res.convergio) {
            res.mensaje = "Balance de masa convergido con recirculación en " + std::to_string(iter + 1) + " iteraciones";
        } else if (!std::isfinite(error) || error > 1e6) {
            res.mensaje = "No convergió: divergencia numérica detectada (error=" + std::to_string(error) + ")";
        } else {
            res.mensaje = "No convergió: iteraciones agotadas después de " + std::to_string(MAX_ITER) + " iteraciones (error=" + std::to_string(error) + ")";
        }
    }

    // ── Restaurar metadatos de corrientes, desglose por fases y entalpía ──
    const auto& gestor = repository_;
    for (auto& [fid, f] : flujos_trabajo) {
        if (auto it_orig = mapa_flujos.find(fid); it_orig != mapa_flujos.end()) {
            const auto& orig = it_orig->second;
            if (!orig.nombre.empty()) f.nombre = orig.nombre;
            f.pagina = orig.pagina;
            f.lca_equivalent = orig.lca_equivalent;
            f.lca_group = orig.lca_group;
            f.measured_flow = orig.measured_flow;
            f.sd = orig.sd;
            f.mb_selected = orig.mb_selected;
            f.mb_assayed = orig.mb_assayed;
            f.es_energia = orig.es_energia;
        }
        if (f.es_energia) continue;

        f.flujo_solido.clear();
        f.flujo_liquido.clear();
        f.flujo_gas.clear();
        double n_sol = 0.0, n_liq = 0.0, n_gas = 0.0;
        double H_total = 0.0;

        for (const auto& [sid, n] : f.flujo_molar) {
            if (n <= 0.0) continue;
            const auto* sp = gestor.especie_por_id(sid);
            nucleo::Fase fase_sp = sp ? sp->fase_std : nucleo::Fase::Liquido;
            // Si es agua líquida a T > 373.15 K a baja presión, clasificar según fase_std
            if (fase_sp == nucleo::Fase::Solido) {
                f.flujo_solido[sid] = n;
                n_sol += n;
            } else if (fase_sp == nucleo::Fase::Gas) {
                f.flujo_gas[sid] = n;
                n_gas += n;
            } else {
                f.flujo_liquido[sid] = n;
                n_liq += n;
            }
            if (sp) H_total += n * sp->entalpia_abs(f.temperatura);
            else    H_total += n * 0.0291 * (f.temperatura - nucleo::STP_TEMP);
        }

        // Contribución de partículas minerales y agua en suspensión
        if (f.agua_kg_h > 0.0) {
            double kmol_w = (f.agua_kg_h / 3600.0) / 18.01528;
            n_liq += kmol_w;
            H_total += (f.agua_kg_h / 3600.0) * (-15879.0 + 4.184 * (f.temperatura - nucleo::STP_TEMP));
        }
        double masa_part_kg_s = 0.0;
        for (const auto& p : f.particulas) {
            masa_part_kg_s += p.mass_kg_h / 3600.0;
        }
        if (masa_part_kg_s > 0.0) {
            n_sol += masa_part_kg_s / 100.0; // equivalente molar aproximado para fracción de fase
            H_total += masa_part_kg_s * (0.85 * (f.temperatura - nucleo::STP_TEMP));
        }

        double n_tot = n_sol + n_liq + n_gas;
        if (n_tot > 0.0) {
            f.fraccion_solido  = n_sol / n_tot;
            f.fraccion_liquido = n_liq / n_tot;
            f.fraccion_gas     = n_gas / n_tot;
            int fases_activas = (n_sol > 1e-9 ? 1 : 0) + (n_liq > 1e-9 ? 1 : 0) + (n_gas > 1e-9 ? 1 : 0);
            if (fases_activas > 1) f.fase = nucleo::Fase::Mixto;
            else if (n_sol > 1e-9) f.fase = nucleo::Fase::Solido;
            else if (n_gas > 1e-9) f.fase = nucleo::Fase::Gas;
            else                   f.fase = nucleo::Fase::Liquido;
        }
        f.entalpia = H_total;
    }

    // ── Verificación global: masa entrante (fuentes) vs masa saliente (sumideros) ──
    double masa_in_total = 0.0, masa_out_total = 0.0;
    auto masa_flujo = [&gestor](const nucleo::Flujo& f) {
        double m = 0.0;
        for (auto& [sid, n] : f.flujo_molar) {
            auto* sp = gestor.especie_por_id(sid);
            m += n * (sp ? sp->masa_molar : 0.0);
        }
        for (const auto& p : f.particulas) {
            m += p.mass_kg_h / 3600.0;
        }
        m += f.agua_kg_h / 3600.0;
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
            salida.temperatura = nodo.params.count("temperatura") ? nodo.params.at("temperatura")
                               : (nodo.params.count("temperature") ? nodo.params.at("temperature")
                               : (nodo.params.count("T") ? nodo.params.at("T") : nucleo::STP_TEMP));
            salida.presion = nodo.params.count("presion") ? nodo.params.at("presion")
                           : (nodo.params.count("pressure") ? nodo.params.at("pressure")
                           : (nodo.params.count("P") ? nodo.params.at("P") : nucleo::STP_PRES));
            for (auto& [k, v] : nodo.params) {
                if (k.rfind("flujo_", 0) == 0 || k.rfind("flow_", 0) == 0) {
                    const size_t pref = (k.rfind("flujo_", 0) == 0) ? 6 : 5;
                    const std::string sub = k.substr(pref);
                    int64_t sid = 0;
                    if (parse_id_especie(sub, sid)) {
                        double val = std::max(0.0, v);
                        salida.flujo_molar[sid] = val;
                    }
                }
            }
            if (nodo.params.count("agua_kg_h")) {
                salida.agua_kg_h = std::max(0.0, nodo.params.at("agua_kg_h"));
            } else if (nodo.params.count("water")) {
                salida.agua_kg_h = std::max(0.0, nodo.params.at("water"));
            }
            int min_idx = 0;
            while (nodo.params.count("mineral_" + std::to_string(min_idx) + "_flow")) {
                double flow = std::max(0.0, nodo.params.at("mineral_" + std::to_string(min_idx) + "_flow"));
                double d = nodo.params.count("mineral_" + std::to_string(min_idx) + "_d")
                         ? std::max(1e-3, nodo.params.at("mineral_" + std::to_string(min_idx) + "_d")) : 100.0;
                std::string name = "Mineral_" + std::to_string(min_idx + 1);
                if (auto itPrev = flujos_trabajo.find(salida.id); itPrev != flujos_trabajo.end() && min_idx < static_cast<int>(itPrev->second.particulas.size())) {
                    name = itPrev->second.particulas[static_cast<size_t>(min_idx)].mineral;
                }
                salida.particulas.push_back({name, d, flow});
                ++min_idx;
            }
            if (!nodo.ids_flujos_salida.empty()) {
                if (auto itPrev = flujos_trabajo.find(nodo.ids_flujos_salida[0]); itPrev != flujos_trabajo.end()) {
                    if (salida.flujo_molar.empty() && !itPrev->second.flujo_molar.empty()) {
                        salida.flujo_molar = itPrev->second.flujo_molar;
                    }
                    if (salida.particulas.empty()) {
                        salida.particulas = itPrev->second.particulas;
                    }
                    if (salida.agua_kg_h <= 0.0) {
                        salida.agua_kg_h = itPrev->second.agua_kg_h;
                    }
                }
                flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            }
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
                    const std::string sub = k.substr(8);
                    int64_t sid = 0;
                    if (parse_id_especie(sub, sid)) {
                        remocion[sid] = v;
                    }
                }
            }
            for (const auto& [sid, n] : entradas[0].flujo_molar) {
                if (n > 0.0 && remocion.count(sid) == 0) {
                    double f1 = fraccion_hacia_salida1(nodo, sid, 0.0);
                    if (f1 > 0.0) remocion[sid] = std::clamp(f1, 0.0, 1.0);
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
                fracciones.resize(n, 1.0 / static_cast<double>(n));
            }
            auto salidas = dividir(entradas[0], fracciones,
                nodo.ids_flujos_salida.empty() ? 0 : nodo.ids_flujos_salida[0]);
            for (size_t i = 0; i < salidas.size() && i < nodo.ids_flujos_salida.size(); ++i)
                flujos_trabajo[nodo.ids_flujos_salida[i]] = salidas[i];
            break;
        }

        // ── Reactores (Estequiométrico, Cinético, Equilibrio de Gibbs) ──
        case nucleo::TipoNodo::ReactorGibbs:
        case nucleo::TipoNodo::ReactorEsteq:
        case nucleo::TipoNodo::ReactorCinetica:
        case nucleo::TipoNodo::Reactor: {
            if (entradas.empty()) break;
            auto mezcla = combinar_entradas(entradas);
            double T = nodo.params.count("temperatura") ? nodo.params.at("temperatura") : mezcla.temperatura;
            double P = nodo.params.count("presion") ? nodo.params.at("presion") : mezcla.presion;
            int64_t rxn_id = nodo.params.count("reaccion_id") ? static_cast<int64_t>(nodo.params.at("reaccion_id")) : 0;
            const auto& db = repository_;
            const auto* rxn = (rxn_id > 0) ? db.reaccion_por_id(rxn_id) : nullptr;

            // ReactorGibbs sin conversión fija: minimización directa de energía libre de Gibbs
            if (nodo.tipo == nucleo::TipoNodo::ReactorGibbs && nodo.params.count("conversion") == 0) {
                const auto& db_map = db.mapa_especies();
                std::set<int64_t> posibles_set;
                for (const auto& [sid, n] : mezcla.flujo_molar) {
                    if (n > 0.0) posibles_set.insert(sid);
                }
                if (rxn) {
                    for (const auto& [sid, _] : rxn->estequiometria) posibles_set.insert(sid);
                } else {
                    // Incluir productos de reacciones cuyos reactivos estén presentes en la mezcla
                    for (int64_t rid = 1; rid <= 60; ++rid) {
                        const auto* r = db.reaccion_por_id(rid);
                        if (!r) continue;
                        bool reactivos_presentes = true;
                        for (const auto& [sid, coeff] : r->estequiometria) {
                            if (coeff < 0 && mezcla.flujo_especie(sid) <= 1e-12) {
                                reactivos_presentes = false;
                                break;
                            }
                        }
                        if (reactivos_presentes) {
                            for (const auto& [sid, _] : r->estequiometria) posibles_set.insert(sid);
                        }
                    }
                }
                std::vector<int64_t> posibles(posibles_set.begin(), posibles_set.end());
                MinimizadorGibbs solver_gibbs;
                auto res_g = solver_gibbs.minimizar(mezcla.flujo_molar, T, P, db_map, posibles);
                if (res_g.convergio && !res_g.flujo_molar.empty()) {
                    nucleo::Flujo salida = mezcla;
                    salida.id = nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0];
                    salida.nombre = "reactor_gibbs_salida";
                    salida.temperatura = T;
                    salida.presion = P;
                    salida.flujo_molar = res_g.flujo_molar;
                    if (!nodo.ids_flujos_salida.empty())
                        flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
                    break;
                }
            }

            double conv = nodo.conversion();
            if (nodo.tipo == nucleo::TipoNodo::ReactorCinetica && nodo.params.count("conversion") == 0) {
                double tau = nodo.params.count("tau") ? nodo.params.at("tau")
                           : (nodo.params.count("residence_time") ? nodo.params.at("residence_time") : 10.0);
                double k = nodo.params.count("k") ? nodo.params.at("k")
                         : (nodo.params.count("rate") ? nodo.params.at("rate") : 0.15);
                if (nodo.params.count("k0") && nodo.params.count("Ea") && T > 0.0) {
                    double k0 = nodo.params.at("k0");
                    double Ea = nodo.params.at("Ea");
                    if (Ea < 1000.0) Ea *= 1000.0; // kJ/mol -> J/mol
                    k = k0 * std::exp(-Ea / (nucleo::R_GAS * T));
                }
                double ktau = std::max(0.0, k * tau);
                bool es_cstr = nodo.params.count("cstr") && nodo.params.at("cstr") > 0.5;
                conv = es_cstr ? (ktau / (1.0 + ktau)) : (1.0 - std::exp(-ktau));
                conv = std::clamp(conv, 0.0, 1.0);
            }

            if (!rxn && rxn_id == 0) {
                rxn = db.reaccion_por_id(1);
            }
            if (!rxn) {
                auto salida = mezcla;
                salida.id = nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0];
                salida.temperatura = T;
                salida.presion = P;
                if (!nodo.ids_flujos_salida.empty())
                    flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            } else {
                auto salida = reactor(entradas, *rxn, conv, T, P,
                    nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0]);
                // Segunda reacción opcional en serie (reaccion_id_2, conversion_2)
                if (nodo.params.count("reaccion_id_2")) {
                    int64_t rxn2_id = static_cast<int64_t>(nodo.params.at("reaccion_id_2"));
                    double conv2 = nodo.params.count("conversion_2") ? nodo.params.at("conversion_2") : conv;
                    if (const auto* rxn2 = db.reaccion_por_id(rxn2_id)) {
                        salida = reactor({salida}, *rxn2, conv2, T, P, salida.id);
                    }
                }
                if (!nodo.ids_flujos_salida.empty())
                    flujos_trabajo[nodo.ids_flujos_salida[0]] = salida;
            }
            break;
        }

        // ── Unidades de separación por especie (flotación, espesador, filtro, lixiviación, etc.) ──
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
        // ── Conminución (Trituración y Molienda) ──
        case nucleo::TipoNodo::TrituradoraMandibula:
        case nucleo::TipoNodo::TrituradoraGiratoria:
        case nucleo::TipoNodo::TrituradoraCono:
        case nucleo::TipoNodo::MolinoSAG:
        case nucleo::TipoNodo::MolinoBolas:
        case nucleo::TipoNodo::MolinoBarras:
            procesar_conminucion(nodo, entradas, flujos_trabajo);
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
            auto mezcla = combinar_entradas(entradas);
            double T_sal = nodo.temperatura_salida();
            double dP = 0;
            for (auto& [k, v] : nodo.params)
                if (k == "caida_presion") dP = v;
            auto salida = intercambiador_calor(mezcla, T_sal, dP,
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
                std::vector<double> fracciones(n, 1.0 / static_cast<double>(n));
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
    if (it != nodo.params.end()) return it->second;

    // Modo Distribution (HSC-Sim): si hay distribución elemental definida (dist_elem_<Elem>),
    // calcular el factor de partición ponderado por la composición elemental de la especie.
    const auto* sp = repository_.especie_por_id(sid);
    if (sp && !sp->composicion_elemental.empty()) {
        double num = 0.0, den = 0.0;
        for (const auto& [elem, cant] : sp->composicion_elemental) {
            auto it_el = nodo.params.find("dist_elem_" + elem);
            if (it_el != nodo.params.end() && cant > 0.0) {
                num += cant * std::clamp(it_el->second, 0.0, 1.0);
                den += cant;
            }
        }
        if (den > 0.0) return num / den;
    }
    return por_defecto;
}

void BalanceMasa::procesar_separacion(const nucleo::Nodo& nodo,
                                      const std::vector<nucleo::Flujo>& entradas,
                                      std::map<int64_t, nucleo::Flujo>& flujos_trabajo) const
{
    if (entradas.empty()) return;
    if (nodo.ids_flujos_salida.empty()) return;
    auto mezcla = combinar_entradas(entradas);
    const auto& gestor = repository_;

    if (nodo.params.count("temperatura"))
        mezcla.temperatura = nodo.params.at("temperatura");
    if (nodo.params.count("presion"))
        mezcla.presion = nodo.params.at("presion");

    // Soporte HSC-Sim para unidades reactivas con separación (ej. TanqueLixiviacion / Pressure Leaching,
    // LavadorGases, CeldaElectroobtencion): si tiene reaccion_id definida, reaccionar antes de separar.
    if (nodo.params.count("reaccion_id")) {
        int64_t rxn_id = static_cast<int64_t>(nodo.params.at("reaccion_id"));
        double conv = nodo.conversion();
        if (const auto* rxn = gestor.reaccion_por_id(rxn_id)) {
            mezcla = reactor({mezcla}, *rxn, conv, mezcla.temperatura, mezcla.presion, -1);
        }
        if (nodo.params.count("reaccion_id_2")) {
            int64_t rxn2_id = static_cast<int64_t>(nodo.params.at("reaccion_id_2"));
            double conv2 = nodo.params.count("conversion_2") ? nodo.params.at("conversion_2") : conv;
            if (const auto* rxn2 = gestor.reaccion_por_id(rxn2_id)) {
                mezcla = reactor({mezcla}, *rxn2, conv2, mezcla.temperatura, mezcla.presion, -1);
            }
        }
    }

    if (nodo.ids_flujos_salida.size() == 1) {
        mezcla.id = nodo.ids_flujos_salida[0];
        mezcla.nombre = nodo.nombre + "_salida";
        flujos_trabajo[nodo.ids_flujos_salida[0]] = mezcla;
        return;
    }

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

    // ── Manejo de corrientes particuladas (Procesamiento Mineral) ──
    if (!mezcla.particulas.empty() || mezcla.agua_kg_h > 0.0) {
        if (nodo.tipo == nucleo::TipoNodo::Hidrociclon || nodo.tipo == nucleo::TipoNodo::Zaranda) {
            double d50 = nodo.params.count("d50") ? nodo.params.at("d50") : 150.0;
            double sharpness = nodo.params.count("sharpness") ? nodo.params.at("sharpness") : 2.0;
            double bp = nodo.params.count("bypass") ? nodo.params.at("bypass") : 0.15;
            double water_to_coarse = nodo.params.count("water_to_coarse") ? nodo.params.at("water_to_coarse") : 0.25;
            auto split_res = MineralProcessing::classify(mezcla.a_corriente_particulas(), d50, sharpness, bp, water_to_coarse);
            if (split_res.ok) {
                // Salida 0 = overflow (finos/residue); Salida 1 = underflow (gruesos/product)
                sal0.desde_corriente_particulas(split_res.residue);
                sal1.desde_corriente_particulas(split_res.product);
            }
        } else if (nodo.tipo == nucleo::TipoNodo::CeldaFlotacion) {
            double residence = nodo.params.count("residence_time") ? nodo.params.at("residence_time") : 10.0;
            double float_water = nodo.params.count("water_recovery") ? nodo.params.at("water_recovery") : 0.15;
            std::map<std::string, FlotationKinetics> kinetics;
            for (const auto& p : mezcla.particulas) {
                double k = nodo.params.count("k_" + p.mineral) ? nodo.params.at("k_" + p.mineral) :
                           (nodo.params.count("rate") ? nodo.params.at("rate") : 0.3);
                double Rinf = nodo.params.count("Rinf_" + p.mineral) ? nodo.params.at("Rinf_" + p.mineral) :
                              (nodo.params.count("recovery") ? nodo.params.at("recovery") : recuperacion);
                kinetics[p.mineral] = {k, Rinf};
            }
            auto float_res = MineralProcessing::flotation(mezcla.a_corriente_particulas(), kinetics, residence, float_water);
            if (float_res.ok) {
                // Salida 0 = concentrado (product); Salida 1 = relave (residue)
                sal0.desde_corriente_particulas(float_res.product);
                sal1.desde_corriente_particulas(float_res.residue);
            }
        } else {
            double f1_solids = (nodo.tipo == nucleo::TipoNodo::Espesador) ? efficiency : (1.0 - efficiency);
            double f1_water = (nodo.tipo == nucleo::TipoNodo::Espesador) ? (1.0 - efficiency) : efficiency;
            for (const auto& p : mezcla.particulas) {
                auto p0 = p, p1 = p;
                p1.mass_kg_h = p.mass_kg_h * f1_solids;
                p0.mass_kg_h = p.mass_kg_h - p1.mass_kg_h;
                sal0.particulas.push_back(p0);
                sal1.particulas.push_back(p1);
            }
            sal1.agua_kg_h = mezcla.agua_kg_h * f1_water;
            sal0.agua_kg_h = mezcla.agua_kg_h - sal1.agua_kg_h;
        }
    }

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
                // Solución cargada (líquidos/acuosos) = salida 0. Residuo (sólidos) = salida 1.
                f1 = es_solido ? 0.98 : 0.02;
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
        int64_t rxn_id = static_cast<int64_t>(nodo.params.at("reaccion_id"));
        const auto& db = repository_;
        const auto* rxn = db.reaccion_por_id(rxn_id);
        if (rxn) producto = reactor(entradas, *rxn, conv, T, P, -1);
        if (nodo.params.count("reaccion_id_2")) {
            int64_t rxn2_id = static_cast<int64_t>(nodo.params.at("reaccion_id_2"));
            double conv2 = nodo.params.count("conversion_2") ? nodo.params.at("conversion_2") : conv;
            if (const auto* rxn2 = db.reaccion_por_id(rxn2_id)) {
                producto = reactor({producto}, *rxn2, conv2, T, P, -1);
            }
        }
    }

    size_t n = nodo.ids_flujos_salida.size();
    if (n == 0) return;
    if (n == 1) {
        producto.id = nodo.ids_flujos_salida[0];
        producto.nombre = nodo.nombre + "_salida";
        flujos_trabajo[nodo.ids_flujos_salida[0]] = producto;
        return;
    }
    // Con 2 salidas: distribuir por especie (param "split_<id>", default por fase: gas a sal0, condensado a sal1 si aplica).
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
        for (auto& p : producto.particulas) {
            auto p0 = p, p1 = p;
            p1.mass_kg_h = p.mass_kg_h * 0.5;
            p0.mass_kg_h = p.mass_kg_h - p1.mass_kg_h;
            sal0.particulas.push_back(p0);
            sal1.particulas.push_back(p1);
        }
        sal1.agua_kg_h = producto.agua_kg_h * 0.5;
        sal0.agua_kg_h = producto.agua_kg_h - sal1.agua_kg_h;
        flujos_trabajo[nodo.ids_flujos_salida[0]] = sal0;
        flujos_trabajo[nodo.ids_flujos_salida[1]] = sal1;
        return;
    }
    std::vector<double> fracciones(n, 1.0 / static_cast<double>(n));
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
        for (const auto& p : e.particulas) {
            bool found = false;
            for (auto& op : out.particulas) {
                if (op.mineral == p.mineral && std::abs(op.diameter_um - p.diameter_um) < 1e-4) {
                    op.mass_kg_h += p.mass_kg_h;
                    found = true;
                    break;
                }
            }
            if (!found) out.particulas.push_back(p);
        }
        out.agua_kg_h += e.agua_kg_h;
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
        out.id = base_id + static_cast<int64_t>(i);
        out.nombre = "division_" + std::to_string(i);
        out.temperatura = entrada.temperatura;
        out.presion = entrada.presion;
        double frac = fracciones[i] / total_frac;
        for (auto& [sid, n] : entrada.flujo_molar)
            out.flujo_molar[sid] = n * frac;
        for (const auto& p : entrada.particulas) {
            auto cp = p;
            cp.mass_kg_h *= frac;
            out.particulas.push_back(cp);
        }
        out.agua_kg_h = entrada.agua_kg_h * frac;
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
    out.particulas = mezcla.particulas;
    out.agua_kg_h = mezcla.agua_kg_h;

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

    double gamma = 1.4;
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

void BalanceMasa::procesar_conminucion(const nucleo::Nodo& nodo,
                                       const std::vector<nucleo::Flujo>& entradas,
                                       std::map<int64_t, nucleo::Flujo>& flujos_trabajo) const
{
    if (entradas.empty()) return;
    auto producto = combinar_entradas(entradas);
    producto.id = nodo.ids_flujos_salida.empty() ? -1 : nodo.ids_flujos_salida[0];
    producto.nombre = nodo.nombre + "_salida";

    if (!producto.particulas.empty()) {
        double solids_t_h = 0.0;
        for (const auto& p : producto.particulas) solids_t_h += p.mass_kg_h / 1000.0;

        if (nodo.tipo == nucleo::TipoNodo::MolinoBolas ||
            nodo.tipo == nucleo::TipoNodo::MolinoSAG ||
            nodo.tipo == nucleo::TipoNodo::MolinoBarras) {
            double wi = nodo.params.count("wi") ? nodo.params.at("wi") : 13.5;
            double power = nodo.params.count("power") ? nodo.params.at("power")
                         : (nodo.params.count("potencia") ? nodo.params.at("potencia") : 2500.0);
            if (solids_t_h > 0.0 && wi > 0.0 && power > 0.0) {
                double specific_energy_kWh_t = power / solids_t_h;
                for (auto& p : producto.particulas) {
                    if (p.diameter_um > 0.0) {
                        double inv_sqrt_p = 1.0 / std::sqrt(p.diameter_um) + specific_energy_kWh_t / (10.0 * wi);
                        if (inv_sqrt_p > 0.0) {
                            p.diameter_um = 1.0 / (inv_sqrt_p * inv_sqrt_p);
                        }
                    }
                }
            }
        } else if (nodo.tipo == nucleo::TipoNodo::TrituradoraMandibula ||
                   nodo.tipo == nucleo::TipoNodo::TrituradoraGiratoria ||
                   nodo.tipo == nucleo::TipoNodo::TrituradoraCono) {
            double css_um = (nodo.params.count("css") ? nodo.params.at("css") : 25.0) * 1000.0;
            if (css_um > 0.0) {
                for (auto& p : producto.particulas) {
                    if (p.diameter_um > css_um) {
                        p.diameter_um = css_um;
                    }
                }
            }
        }
    }

    if (!nodo.ids_flujos_salida.empty())
        flujos_trabajo[nodo.ids_flujos_salida[0]] = producto;
}

}} // namespace
