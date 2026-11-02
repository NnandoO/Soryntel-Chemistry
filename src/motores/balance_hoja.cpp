#include "motores/balance_hoja.hpp"
#include "motores/balance_exergia.hpp"
#include "nucleo/tabla_periodica.hpp"

#include <cstdio>
#include <set>

namespace cma { namespace motores {

namespace {

constexpr double kVolumenMolarNm3 = 22.414;  // Nm³/kmol (0 °C, 1 atm)
constexpr double kTemperaturaMinimaAbsoluta = 1.0;   // K
constexpr double kTemperaturaMaximaSoportada = 6000.0; // K

void calcular_fila(FilaBalance& f) {
    if (f.especie == nullptr) {
        f.masa_kg = 0.0;
        f.volumen_nm3 = 0.0;
        f.cp_kJ_K = 0.0;
        f.h_formacion_kJ = 0.0;
        f.h_sensible_kJ = 0.0;
        f.h_total_kJ = 0.0;
        f.exergia_fisica_kJ = 0.0;
        f.exergia_quimica_kJ = 0.0;
        f.exergia_total_kJ = 0.0;
        return;
    }

    const nucleo::Especie& sp = *f.especie;
    const double T = std::max(f.temperatura_K, kTemperaturaMinimaAbsoluta);

    f.cp_kJ_K = f.kmol * sp.cp(T);
    f.h_formacion_kJ = f.kmol * 1000.0 * sp.h_formacion;
    f.h_sensible_kJ = f.kmol * 1000.0 * sp.entalpia_ref(T);
    f.h_total_kJ = f.h_formacion_kJ + f.h_sensible_kJ;
    f.masa_kg = f.kmol * sp.masa_molar;
    f.volumen_nm3 = f.kmol * FilaBalance::volumen_molar_nm3(&sp);

    const double ex_quimica_molar = BalanceHoja::exergia_quimica_estandar(f);
    double ex_fisica_molar =
        sp.entalpia_ref(T) - nucleo::STP_TEMP * sp.entropia_ref(T) / 1000.0;
    if (sp.fase_std == nucleo::Fase::Gas && f.presion_bar > 0.0 &&
        std::abs(f.presion_bar - 1.0) > 1e-9) {
        ex_fisica_molar += (nucleo::R_GAS * nucleo::STP_TEMP / 1000.0) *
                           std::log(std::max(f.presion_bar, 1e-30));
    }
    f.exergia_fisica_kJ = f.kmol * 1000.0 * ex_fisica_molar;
    f.exergia_quimica_kJ = f.kmol * 1000.0 * ex_quimica_molar;
    f.exergia_total_kJ = f.exergia_fisica_kJ + f.exergia_quimica_kJ;
}

void agregar_aviso(std::set<std::string>& avisos, const std::string& texto) {
    if (texto.empty()) return;
    if (avisos.size() < 64) avisos.insert(texto);
}

void revisar_corrientes(const std::vector<CorrienteBalance>& corrientes,
                        int& filas_sin_especie, std::set<std::string>& avisos) {
    for (const auto& c : corrientes) {
        for (const auto& f : c.filas) {
            if (f.formula.empty() && f.especie == nullptr) continue;
            if (f.especie == nullptr) {
                ++filas_sin_especie;
                agregar_aviso(avisos, "Sin datos termodinámicos: " + f.formula);
                continue;
            }
            if (f.kmol < 0.0)
                agregar_aviso(avisos, "Cantidad negativa: " + f.formula);
            if (f.temperatura_K < kTemperaturaMinimaAbsoluta ||
                f.temperatura_K > kTemperaturaMaximaSoportada) {
                agregar_aviso(avisos, "Temperatura fuera de rango (1-6000 K): " + f.formula);
            } else if (f.especie->cp(f.temperatura_K) <= 0.0) {
                agregar_aviso(avisos, "Cp ≤ 0 (polinomio fuera de rango): " + f.formula);
            }
        }
    }
}

} // namespace

// ─────────────────────────────────────────────────────────────────────────────
void BalanceHoja::actualizar(AreaBalance& area, const ResolvedorEspecie& resolver) {
    auto actualizar_corrientes = [&](std::vector<CorrienteBalance>& corrientes) {
        for (auto& c : corrientes) {
            for (auto& f : c.filas) {
                if (resolver && f.especie == nullptr && !f.formula.empty())
                    f.especie = resolver(f.formula);
                calcular_fila(f);
            }
        }
    };
    actualizar_corrientes(area.entradas);
    actualizar_corrientes(area.salidas);
}

double BalanceHoja::entalpia_fila(const FilaBalance& fila, double T_K) {
    if (fila.especie == nullptr) return 0.0;
    return fila.kmol * 1000.0 *
           fila.especie->entalpia_abs(std::max(T_K, kTemperaturaMinimaAbsoluta));
}

double BalanceHoja::exergia_quimica_estandar(const FilaBalance& fila) {
    static BalanceExergia motor_exergia;
    if (fila.especie != nullptr) {
        double v = motor_exergia.obtener_exergia_quimica_std(fila.especie->nombre);
        if (v != 0.0) return v;
        v = motor_exergia.obtener_exergia_quimica_std(fila.especie->formula);
        if (v != 0.0) return v;
    }
    return motor_exergia.obtener_exergia_quimica_std(fila.formula);
}

// ─────────────────────────────────────────────────────────────────────────────
ResultadoBalance BalanceHoja::evaluar(const AreaBalance& area,
                                      double tol_energia_kJ,
                                      double tol_masa_kg) const {
    ResultadoBalance r;
    std::set<std::string> avisos;

    auto sumar = [&](const std::vector<CorrienteBalance>& corrientes, bool entrada) {
        for (const auto& c : corrientes) {
            for (const auto& f : c.filas) {
                if (entrada) {
                    r.kmol_entrada += f.kmol;
                    r.masa_entrada_kg += f.masa_kg;
                    r.nm3_entrada += f.volumen_nm3;
                    r.h_formacion_entrada_kJ += f.h_formacion_kJ;
                    r.h_sensible_entrada_kJ += f.h_sensible_kJ;
                    r.h_entrada_kJ += f.h_total_kJ;
                    r.exergia_entrada_kJ += f.exergia_total_kJ;
                    r.cp_total_entrada_kJ_K += f.cp_kJ_K;
                } else {
                    r.kmol_salida += f.kmol;
                    r.masa_salida_kg += f.masa_kg;
                    r.nm3_salida += f.volumen_nm3;
                    r.h_formacion_salida_kJ += f.h_formacion_kJ;
                    r.h_sensible_salida_kJ += f.h_sensible_kJ;
                    r.h_salida_kJ += f.h_total_kJ;
                    r.exergia_salida_kJ += f.exergia_total_kJ;
                    r.cp_total_salida_kJ_K += f.cp_kJ_K;
                }
            }
            r.calor_externo_kJ += c.calor_kJ;
        }
    };
    sumar(area.entradas, true);
    sumar(area.salidas, false);

    r.residual_masa_kg = r.masa_entrada_kg - r.masa_salida_kg;
    r.residual_energia_kJ = r.h_entrada_kJ - r.h_salida_kJ - r.calor_externo_kJ;

    const double ref_masa = std::max({std::abs(r.masa_entrada_kg),
                                      std::abs(r.masa_salida_kg), 1e-9});
    r.error_masa_pct = r.residual_masa_kg / ref_masa * 100.0;

    const double ref_energia = std::max({std::abs(r.h_entrada_kJ),
                                         std::abs(r.h_salida_kJ), 1.0});
    r.error_energia_pct = r.residual_energia_kJ / ref_energia * 100.0;

    r.balance_masa_ok = std::abs(r.residual_masa_kg) <=
                        std::max(tol_masa_kg, 1e-5 * ref_masa);
    r.balance_energia_ok = std::abs(r.residual_energia_kJ) <=
                           std::max(tol_energia_kJ, 1e-6 * ref_energia);

    r.exergia_destruida_kJ = r.exergia_entrada_kJ - r.exergia_salida_kJ;
    if (r.exergia_entrada_kJ > 1e-9)
        r.eficiencia_exergetica_pct =
            std::clamp(r.exergia_salida_kJ / r.exergia_entrada_kJ * 100.0, 0.0, 1000.0);

    revisar_corrientes(area.entradas, r.filas_sin_especie, avisos);
    revisar_corrientes(area.salidas, r.filas_sin_especie, avisos);

    if (!r.balance_masa_ok) {
        char buf[160];
        std::snprintf(buf, sizeof(buf),
                      "Masa no balanceada: Δ = %.4g kg (%.3g %%)",
                      r.residual_masa_kg, r.error_masa_pct);
        agregar_aviso(avisos, buf);
    }
    if (!r.balance_energia_ok) {
        char buf[160];
        std::snprintf(buf, sizeof(buf),
                      "Energía no balanceada: Δ = %.4g kJ (%.3g %%)",
                      r.residual_energia_kJ, r.error_energia_pct);
        agregar_aviso(avisos, buf);
    }

    r.avisos.assign(avisos.begin(), avisos.end());
    return r;
}

// ─────────────────────────────────────────────────────────────────────────────
BalanceHoja::ResultadoTemperatura BalanceHoja::temperatura_equilibrio(
    AreaBalance& area, bool mantener_offsets, double t_min_K, double t_max_K,
    double tol_kJ, int max_iter) {
    ResultadoTemperatura res;

    if (area.salidas.empty()) {
        res.mensaje = "No hay corrientes de salida para resolver.";
        return res;
    }
    if (t_max_K <= t_min_K) std::swap(t_min_K, t_max_K);
    t_min_K = std::max(t_min_K, kTemperaturaMinimaAbsoluta);

    actualizar(area, {});

    double h_entrada = 0.0;
    double calor_total = 0.0;
    for (const auto& c : area.entradas) {
        calor_total += c.calor_kJ;
        for (const auto& f : c.filas) h_entrada += f.h_total_kJ;
    }
    for (const auto& c : area.salidas) calor_total += c.calor_kJ;
    const double objetivo = h_entrada - calor_total;

    // Verifica si existen corrientes de salida con temperatura libre (no fija).
    bool hay_salida_libre = false;
    size_t idx_ref = 0;
    for (size_t i = 0; i < area.salidas.size(); ++i) {
        if (!area.salidas[i].temperatura_fija && !area.salidas[i].filas.empty()) {
            if (!hay_salida_libre) idx_ref = i;
            hay_salida_libre = true;
        }
    }
    const bool respetar_fijas = hay_salida_libre;

    // Desplazamientos entre filas de salida respecto de la primera fila libre.
    std::vector<std::vector<double>> offsets(area.salidas.size());
    double t_ref = nucleo::STP_TEMP;
    if (idx_ref < area.salidas.size() && !area.salidas[idx_ref].filas.empty()) {
        t_ref = area.salidas[idx_ref].filas.front().temperatura_K;
    }
    for (size_t i = 0; i < area.salidas.size(); ++i) {
        offsets[i].resize(area.salidas[i].filas.size(), 0.0);
        if (mantener_offsets && (!respetar_fijas || !area.salidas[i].temperatura_fija)) {
            for (size_t j = 0; j < area.salidas[i].filas.size(); ++j)
                offsets[i][j] = area.salidas[i].filas[j].temperatura_K - t_ref;
        }
    }

    auto H_salida = [&](double T_base) {
        double h = 0.0;
        for (size_t i = 0; i < area.salidas.size(); ++i) {
            const bool fija = respetar_fijas && area.salidas[i].temperatura_fija;
            for (size_t j = 0; j < area.salidas[i].filas.size(); ++j) {
                const double T_fila = fija ? area.salidas[i].filas[j].temperatura_K
                                           : (T_base + offsets[i][j]);
                h += entalpia_fila(area.salidas[i].filas[j], T_fila);
            }
        }
        return h;
    };
    auto F = [&](double T) { return H_salida(T) - objetivo; };

    auto asignar_temperaturas = [&](double T) {
        for (size_t i = 0; i < area.salidas.size(); ++i) {
            if (respetar_fijas && area.salidas[i].temperatura_fija) continue;
            for (size_t j = 0; j < area.salidas[i].filas.size(); ++j)
                area.salidas[i].filas[j].temperatura_K = T + offsets[i][j];
        }
        actualizar(area, {});
    };

    double a = t_min_K;
    double b = t_min_K;
    double fa = 0.0;
    double fb = 0.0;

    // Localiza el primer cruce por cero sobre una malla. Es robusto frente a
    // polinomios de Cp que se degradan (o dejan de ser monótonos) en los
    // extremos del rango soportado.
    constexpr int kDivisiones = 512;
    {
        double t_prev = t_min_K;
        double f_prev = F(t_prev);
        double t_ultimo = t_prev;
        double f_ultimo = f_prev;
        bool hay_cruce = false;

        if (std::abs(f_prev) <= tol_kJ) {
            res.factible = res.convergio = true;
            res.temperatura_K = t_min_K;
            res.residual_kJ = f_prev;
            asignar_temperaturas(t_min_K);
            res.mensaje = "Balance cerrado en el límite inferior del rango.";
            return res;
        }

        for (int i = 1; i <= kDivisiones; ++i) {
            const double t = t_min_K + (t_max_K - t_min_K) *
                                        (static_cast<double>(i) / kDivisiones);
            const double ft = F(t);
            if (!std::isfinite(ft)) break;
            t_ultimo = t;
            f_ultimo = ft;
            if (f_prev <= 0.0 && ft >= 0.0) {
                a = t_prev;
                b = t;
                fa = f_prev;
                fb = ft;
                hay_cruce = true;
                break;
            }
            t_prev = t;
            f_prev = ft;
        }

        if (!hay_cruce) {
            res.factible = false;
            if (f_prev > 0.0) {
                res.temperatura_K = t_min_K;
                res.residual_kJ = F(t_min_K);
                res.mensaje = "No factible: sobra energía; la temperatura de "
                              "equilibrio queda por debajo del rango soportado.";
            } else {
                res.temperatura_K = t_ultimo;
                res.residual_kJ = f_ultimo;
                res.mensaje = "No factible: falta energía para cerrar el balance "
                              "dentro del rango válido de los datos.";
            }
            return res;
        }
    }

    if (std::abs(fb) <= tol_kJ) {
        res.factible = res.convergio = true;
        res.temperatura_K = b;
        res.residual_kJ = fb;
        asignar_temperaturas(b);
        res.mensaje = "Balance cerrado en el límite superior del rango.";
        return res;
    }

    res.factible = true;
    double T = 0.5 * (a + b);
    int iter = 0;
    for (; iter < max_iter; ++iter) {
        T = 0.5 * (a + b);
        const double fT = F(T);
        if (std::abs(fT) <= tol_kJ || (b - a) <= 1e-6) {
            res.convergio = true;
            res.residual_kJ = fT;
            break;
        }
        if (fa * fT <= 0.0) {
            b = T;
            fb = fT;
        } else {
            a = T;
            fa = fT;
        }
    }
    if (!res.convergio) {
        res.residual_kJ = F(T);
        res.mensaje = "Máximo de iteraciones alcanzado.";
    } else {
        res.mensaje = "Balance térmico cerrado.";
    }

    res.temperatura_K = T;
    res.iteraciones = iter;
    asignar_temperaturas(T);
    return res;
}

// ─────────────────────────────────────────────────────────────────────────────
double BalanceHoja::calor_requerido(const AreaBalance& area, double T_objetivo_K,
                                    bool mantener_offsets) const {
    double h_entrada = 0.0;
    for (const auto& c : area.entradas)
        for (const auto& f : c.filas) h_entrada += f.h_total_kJ;

    if (area.salidas.empty() || area.salidas.front().filas.empty()) {
        return h_entrada;
    }

    bool hay_salida_libre = false;
    size_t idx_ref = 0;
    for (size_t i = 0; i < area.salidas.size(); ++i) {
        if (!area.salidas[i].temperatura_fija && !area.salidas[i].filas.empty()) {
            if (!hay_salida_libre) idx_ref = i;
            hay_salida_libre = true;
        }
    }
    const bool respetar_fijas = hay_salida_libre;
    const double t_ref = area.salidas[idx_ref].filas.front().temperatura_K;

    double h_salida = 0.0;
    for (const auto& c : area.salidas) {
        const bool fija = respetar_fijas && c.temperatura_fija;
        for (const auto& f : c.filas) {
            if (fija) {
                h_salida += entalpia_fila(f, f.temperatura_K);
            } else {
                const double offset = mantener_offsets ? (f.temperatura_K - t_ref) : 0.0;
                h_salida += entalpia_fila(f, T_objetivo_K + offset);
            }
        }
    }
    return h_entrada - h_salida;
}

// ─────────────────────────────────────────────────────────────────────────────
double BalanceHoja::ajustar_calor_de_area(AreaBalance& area) {
    if (area.salidas.empty()) return 0.0;

    actualizar(area, {});

    double h_entrada = 0.0, h_salida = 0.0, calor_actual = 0.0;
    for (const auto& c : area.entradas) {
        calor_actual += c.calor_kJ;
        for (const auto& f : c.filas) h_entrada += f.h_total_kJ;
    }
    for (const auto& c : area.salidas) {
        calor_actual += c.calor_kJ;
        for (const auto& f : c.filas) h_salida += f.h_total_kJ;
    }

    const double calor_necesario = h_entrada - h_salida;
    area.salidas.front().calor_kJ += (calor_necesario - calor_actual);
    return calor_necesario;
}

// ─────────────────────────────────────────────────────────────────────────────
BalanceHoja::ResultadoTemperatura BalanceHoja::temperatura_precalentamiento(
    AreaBalance& area, int idx_entrada, double t_min_K, double t_max_K,
    double tol_kJ, int max_iter) {
    ResultadoTemperatura res;
    if (area.entradas.empty() || idx_entrada < 0 ||
        idx_entrada >= static_cast<int>(area.entradas.size()) ||
        area.entradas[static_cast<size_t>(idx_entrada)].filas.empty()) {
        res.mensaje = "Selecciona una corriente de entrada válida con especies.";
        return res;
    }
    if (t_max_K <= t_min_K) std::swap(t_min_K, t_max_K);
    t_min_K = std::max(t_min_K, kTemperaturaMinimaAbsoluta);

    actualizar(area, {});

    const size_t ie = static_cast<size_t>(idx_entrada);
    double h_otras_entradas = 0.0;
    double h_salida = 0.0;
    double calor_total = 0.0;

    for (size_t i = 0; i < area.entradas.size(); ++i) {
        calor_total += area.entradas[i].calor_kJ;
        if (i == ie) continue;
        for (const auto& f : area.entradas[i].filas) h_otras_entradas += f.h_total_kJ;
    }
    for (const auto& c : area.salidas) {
        calor_total += c.calor_kJ;
        for (const auto& f : c.filas) h_salida += f.h_total_kJ;
    }

    // Queremos H_entrada_ie(T) + h_otras_entradas - h_salida - calor_total = 0
    const double objetivo_ie = h_salida + calor_total - h_otras_entradas;

    auto H_ie = [&](double T_K) {
        double h = 0.0;
        for (const auto& f : area.entradas[ie].filas) h += entalpia_fila(f, T_K);
        return h;
    };
    auto F = [&](double T_K) { return H_ie(T_K) - objetivo_ie; };

    auto asignar = [&](double T_K) {
        for (auto& f : area.entradas[ie].filas) f.temperatura_K = T_K;
        actualizar(area, {});
    };

    double a = t_min_K, b = t_max_K;
    double fa = F(a), fb = F(b);
    if (std::abs(fa) <= tol_kJ) {
        res.factible = res.convergio = true;
        res.temperatura_K = a;
        res.residual_kJ = fa;
        asignar(a);
        res.mensaje = "Precalentamiento resuelto en el límite inferior.";
        return res;
    }
    if (fa * fb > 0.0) {
        res.factible = false;
        res.temperatura_K = (std::abs(fa) < std::abs(fb)) ? a : b;
        res.residual_kJ = (std::abs(fa) < std::abs(fb)) ? fa : fb;
        res.mensaje = (fa > 0.0)
                          ? "El proceso ya es exotérmico sin precalentar esta corriente."
                          : "El precalentamiento requerido supera el rango máximo (4000 K).";
        return res;
    }

    res.factible = true;
    double T = 0.5 * (a + b);
    int iter = 0;
    for (; iter < max_iter; ++iter) {
        T = 0.5 * (a + b);
        const double fT = F(T);
        if (std::abs(fT) <= tol_kJ || (b - a) <= 1e-6) {
            res.convergio = true;
            res.residual_kJ = fT;
            break;
        }
        if (fa * fT <= 0.0) {
            b = T;
            fb = fT;
        } else {
            a = T;
            fa = fT;
        }
    }
    res.temperatura_K = T;
    res.iteraciones = iter;
    asignar(T);
    res.mensaje = res.convergio ? "Temperatura óptima de precalentamiento calculada."
                                : "Máximo de iteraciones alcanzado.";
    return res;
}

// ─────────────────────────────────────────────────────────────────────────────
void BalanceHoja::escalar_area(AreaBalance& area, double factor) {
    if (factor <= 0.0 || !std::isfinite(factor)) return;
    for (auto& c : area.entradas) {
        c.calor_kJ *= factor;
        for (auto& f : c.filas) f.kmol *= factor;
    }
    for (auto& c : area.salidas) {
        c.calor_kJ *= factor;
        for (auto& f : c.filas) f.kmol *= factor;
    }
    actualizar(area, {});
}

// ─────────────────────────────────────────────────────────────────────────────
void BalanceHoja::poblar_desde_reaccion(
    AreaBalance& area, const nucleo::Reaccion& rxn,
    const std::map<int64_t, nucleo::Especie>& db, double kmol_base,
    double conversion, double exceso_oxigeno_pct, bool incluir_n2_aire,
    double t_entrada_K, double t_salida_K) {
    area.entradas.clear();
    area.salidas.clear();

    kmol_base = std::max(kmol_base, 1e-6);
    conversion = std::clamp(conversion, 0.0, 1.0);
    exceso_oxigeno_pct = std::max(exceso_oxigeno_pct, 0.0);

    CorrienteBalance c_in;
    c_in.nombre = "Materias Primas (Reactivos)";
    CorrienteBalance c_out;
    c_out.nombre = "Productos de Reacción";

    const nucleo::Especie* sp_n2 = nullptr;
    for (const auto& [id, sp] : db) {
        if (sp.formula == "N2" || sp.formula == "N2(g)") {
            sp_n2 = &sp;
            break;
        }
    }

    double n_o2_in = 0.0;
    for (const auto& [sid, coeff] : rxn.estequiometria) {
        auto it = db.find(sid);
        if (it == db.end()) continue;
        const nucleo::Especie& sp = it->second;

        if (coeff < 0.0) {
            const double n_esteq = (-coeff) * kmol_base;
            const bool es_o2 = (sp.formula == "O2" || sp.formula == "O2(g)");
            const double factor_exceso = es_o2 ? (1.0 + exceso_oxigeno_pct / 100.0) : 1.0;
            const double n_in = n_esteq * factor_exceso;
            if (es_o2) n_o2_in += n_in;

            FilaBalance f_in;
            f_in.formula = sp.formula;
            f_in.especie = &sp;
            f_in.temperatura_K = t_entrada_K;
            f_in.presion_bar = 1.0;
            f_in.kmol = n_in;
            c_in.filas.push_back(f_in);

            // Reactivo sin reaccionar o en exceso
            const double n_reacciona = n_esteq * conversion;
            const double n_rem = std::max(0.0, n_in - n_reacciona);
            if (n_rem > 1e-9) {
                FilaBalance f_rem = f_in;
                f_rem.temperatura_K = t_salida_K;
                f_rem.kmol = n_rem;
                c_out.filas.push_back(f_rem);
            }
        } else if (coeff > 0.0) {
            FilaBalance f_out;
            f_out.formula = sp.formula;
            f_out.especie = &sp;
            f_out.temperatura_K = t_salida_K;
            f_out.presion_bar = 1.0;
            f_out.kmol = coeff * kmol_base * conversion;
            c_out.filas.push_back(f_out);
        }
    }

    if (incluir_n2_aire && n_o2_in > 0.0 && sp_n2 != nullptr) {
        const double n_n2 = n_o2_in * (79.0 / 21.0);
        FilaBalance f_n2_in;
        f_n2_in.formula = sp_n2->formula;
        f_n2_in.especie = sp_n2;
        f_n2_in.temperatura_K = t_entrada_K;
        f_n2_in.presion_bar = 1.0;
        f_n2_in.kmol = n_n2;
        c_in.filas.push_back(f_n2_in);

        FilaBalance f_n2_out = f_n2_in;
        f_n2_out.temperatura_K = t_salida_K;
        c_out.filas.push_back(f_n2_out);
    }

    area.entradas.push_back(std::move(c_in));
    area.salidas.push_back(std::move(c_out));
    actualizar(area, {});
}

// ─────────────────────────────────────────────────────────────────────────────
std::vector<BalanceElemento> BalanceHoja::balance_elemental(const AreaBalance& area) const {
    std::map<std::string, BalanceElemento> mapa;
    const auto& tabla = nucleo::TablaPeriodica::instancia();

    auto acumular = [&](const std::vector<CorrienteBalance>& corrientes, bool entrada) {
        for (const auto& c : corrientes) {
            for (const auto& f : c.filas) {
                std::vector<nucleo::TablaPeriodica::CompAtom> atomos;
                if (f.especie != nullptr) {
                    atomos = tabla.analizar_formula(f.especie->formula);
                    if (atomos.empty())
                        atomos = tabla.analizar_formula(f.especie->nombre);
                } else if (!f.formula.empty()) {
                    atomos = tabla.analizar_formula(f.formula);
                }

                for (const auto& a : atomos) {
                    if (a.cuenta <= 0) continue;
                    BalanceElemento& e = mapa[a.elem];
                    e.elemento = a.elem;
                    if (const auto* el = tabla.por_simbolo(a.elem))
                        e.masa_atomica = el->masa_atomica;

                    const double n_at = static_cast<double>(a.cuenta) * f.kmol;
                    const double masa = n_at * e.masa_atomica;
                    if (entrada) {
                        e.in_kmol_at += n_at;
                        e.in_kg += masa;
                    } else {
                        e.out_kmol_at += n_at;
                        e.out_kg += masa;
                    }
                }
            }
        }
    };
    acumular(area.entradas, true);
    acumular(area.salidas, false);

    std::vector<BalanceElemento> res;
    res.reserve(mapa.size());
    for (auto& [simbolo, e] : mapa) res.push_back(e);
    return res;
}

}} // namespace cma::motores
