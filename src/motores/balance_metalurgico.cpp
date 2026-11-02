#include "motores/balance_metalurgico.hpp"

#include <algorithm>
#include <cmath>
#include <iomanip>
#include <limits>
#include <set>
#include <sstream>

namespace cma {
namespace motores {
namespace {

constexpr double kTol = 1e-9;

std::string formatear(double valor) {
    std::ostringstream os;
    os << std::setprecision(6) << std::defaultfloat << valor;
    return os.str();
}

double gamma_q(double a, double x) {
    if (x <= 0.0) return 1.0;
    if (a <= 0.0) return 0.0;
    const double gln = std::lgamma(a);
    if (x < a + 1.0) {
        double ap = a;
        double suma = 1.0 / a;
        double delta = suma;
        for (int n = 0; n < 2000; ++n) {
            ap += 1.0;
            delta *= x / ap;
            suma += delta;
            if (std::fabs(delta) < std::fabs(suma) * 1e-14) break;
        }
        const double q = 1.0 - suma * std::exp(-x + a * std::log(x) - gln);
        return std::clamp(q, 0.0, 1.0);
    }
    double b = x + 1.0 - a;
    double c = 1e300;
    double d = 1.0 / b;
    double h = d;
    for (int i = 1; i <= 2000; ++i) {
        const double an = -static_cast<double>(i) * (i - a);
        b += 2.0;
        d = an * d + b;
        if (std::fabs(d) < 1e-300) d = 1e-300;
        c = b + an / c;
        if (std::fabs(c) < 1e-300) c = 1e-300;
        d = 1.0 / d;
        const double delta = d * c;
        h *= delta;
        if (std::fabs(delta - 1.0) < 1e-14) break;
    }
    const double q = std::exp(-x + a * std::log(x) - gln) * h;
    return std::clamp(q, 0.0, 1.0);
}

bool resolver_lineal(std::vector<std::vector<double>> a,
                     std::vector<double> b,
                     std::vector<double>& x) {
    const int n = static_cast<int>(b.size());
    if (n == 0) { x.clear(); return true; }
    for (int col = 0; col < n; ++col) {
        int pivote = col;
        double mejor = std::fabs(a[col][col]);
        for (int fila = col + 1; fila < n; ++fila) {
            const double v = std::fabs(a[fila][col]);
            if (v > mejor) { mejor = v; pivote = fila; }
        }
        if (mejor < 1e-12) return false;
        if (pivote != col) {
            std::swap(a[pivote], a[col]);
            std::swap(b[pivote], b[col]);
        }
        const double p = a[col][col];
        for (int fila = col + 1; fila < n; ++fila) {
            const double factor = a[fila][col] / p;
            if (factor == 0.0) continue;
            for (int c = col; c < n; ++c) a[fila][c] -= factor * a[col][c];
            b[fila] -= factor * b[col];
        }
    }
    x.assign(n, 0.0);
    for (int fila = n - 1; fila >= 0; --fila) {
        double suma = b[fila];
        for (int c = fila + 1; c < n; ++c) suma -= a[fila][c] * x[c];
        if (std::fabs(a[fila][fila]) < 1e-12) return false;
        x[fila] = suma / a[fila][fila];
        if (!std::isfinite(x[fila])) return false;
    }
    return true;
}

int rango_matriz(std::vector<std::vector<double>> a) {
    if (a.empty()) return 0;
    const int filas = static_cast<int>(a.size());
    const int columnas = static_cast<int>(a[0].size());
    for (auto& fila : a) {
        double escala = 0.0;
        for (double v : fila) escala = std::max(escala, std::fabs(v));
        if (escala > 0.0) {
            for (double& v : fila) v /= escala;
        }
    }
    int r = 0;
    for (int col = 0; col < columnas && r < filas; ++col) {
        int pivote = r;
        double mejor = 0.0;
        for (int fila = r; fila < filas; ++fila) {
            const double v = std::fabs(a[fila][col]);
            if (v > mejor) { mejor = v; pivote = fila; }
        }
        if (mejor < 1e-9) continue;
        std::swap(a[pivote], a[r]);
        const double p = a[r][col];
        for (int c = col; c < columnas; ++c) a[r][c] /= p;
        for (int fila = 0; fila < filas; ++fila) {
            if (fila == r) continue;
            const double factor = a[fila][col];
            if (factor == 0.0) continue;
            for (int c = col; c < columnas; ++c) a[fila][c] -= factor * a[r][c];
        }
        ++r;
    }
    return r;
}

struct Variable {
    size_t corriente = 0;
    bool liquido = false;
    double medido = 0.0;
    double sd = 0.0;
    double peso = 0.0;
    bool fijo = false;
    double valor = 0.0;
};

struct FilaBalance {
    std::vector<double> coef;
    double rhs = 0.0;
};

} // namespace

ResultadoBalanceMetalurgico BalanceMetalurgico::resolver(
    const ConfigBalanceMetalurgico& config) const {
    ResultadoBalanceMetalurgico res;
    std::vector<std::string> errores;

    std::map<int64_t, size_t> indice;
    for (size_t i = 0; i < config.corrientes.size(); ++i) {
        if (config.corrientes[i].seleccionada) indice[config.corrientes[i].id] = i;
    }
    if (indice.empty()) {
        res.mensaje = "No hay corrientes seleccionadas para el balance.";
        return res;
    }

    std::vector<const UnidadBalanceMetalurgico*> unidades;
    for (const auto& u : config.unidades) {
        if (u.seleccionada) unidades.push_back(&u);
    }
    if (unidades.empty()) {
        res.mensaje = "No hay unidades seleccionadas. Defina la topología del flowsheet.";
        return res;
    }

    std::set<std::string> elementos;
    for (const auto& [id, i] : indice) {
        (void)id;
        for (const auto& [elemento, fraccion] : config.corrientes[i].ensayos) {
            if (fraccion > 0.0) elementos.insert(elemento);
        }
    }

    if (config.modo == ModoBalanceMetalurgico::SoloEnsayos) {
        for (const auto& [id, i] : indice) {
            const auto& c = config.corrientes[i];
            if (!c.medida && !c.fija) {
                errores.push_back("La corriente '" + c.nombre +
                                  "' no tiene medición de sólidos totales.");
            }
            if (c.ensayos.empty()) {
                errores.push_back("La corriente '" + c.nombre +
                                  "' no tiene ensayos elementales.");
            }
        }
        if (elementos.empty() && errores.empty()) {
            errores.push_back("El modo Assays Only requiere al menos un ensayo elemental.");
        }
    }
    if (!errores.empty()) {
        std::ostringstream os;
        os << "No se puede resolver:";
        for (const auto& e : errores) os << "\n- " << e;
        res.mensaje = os.str();
        return res;
    }

    std::vector<Variable> variables;
    std::map<int64_t, size_t> var_solido;
    std::map<int64_t, size_t> var_liquido;
    for (const auto& [id, i] : indice) {
        const auto& c = config.corrientes[i];
        Variable v;
        v.corriente = i;
        v.liquido = false;
        v.medido = c.solido_t_h;
        v.sd = c.sd_solido_t_h;
        if (c.fija) {
            v.fijo = true;
            v.valor = c.solido_t_h;
        } else if (!c.medida) {
            v.peso = 0.0;
            v.valor = c.solido_t_h;
        } else if (c.sd_solido_t_h > 0.0) {
            v.peso = 1.0 / (c.sd_solido_t_h * c.sd_solido_t_h);
            v.valor = c.solido_t_h;
        } else {
            v.fijo = true;
            v.valor = c.solido_t_h;
        }
        var_solido[id] = variables.size();
        variables.push_back(v);

        if (config.modo == ModoBalanceMetalurgico::SolidosAgua) {
            Variable w;
            w.corriente = i;
            w.liquido = true;
            w.medido = c.liquido_t_h;
            w.sd = c.sd_liquido_t_h;
            if (c.fija) {
                w.fijo = true;
                w.valor = c.liquido_t_h;
            } else if (!c.medida) {
                w.peso = 0.0;
                w.valor = c.liquido_t_h;
            } else if (c.sd_liquido_t_h > 0.0) {
                w.peso = 1.0 / (c.sd_liquido_t_h * c.sd_liquido_t_h);
                w.valor = c.liquido_t_h;
            } else {
                w.fijo = true;
                w.valor = c.liquido_t_h;
            }
            var_liquido[id] = variables.size();
            variables.push_back(w);
        }
    }

    std::vector<FilaBalance> filas;
    auto agregar_fila = [&](const FilaBalance& fila) {
        double escala = 0.0;
        for (double v : fila.coef) escala = std::max(escala, std::fabs(v));
        if (escala <= 0.0) return;
        filas.push_back(fila);
    };

    auto sumar_corriente = [&](FilaBalance& fila, int64_t id, double signo,
                               bool liquido, double ensayo) {
        const auto& mapa = liquido ? var_liquido : var_solido;
        auto it = mapa.find(id);
        if (it == mapa.end()) return;
        fila.coef[it->second] += signo * ensayo;
    };

    for (const auto* u : unidades) {
        FilaBalance solido;
        solido.coef.assign(variables.size(), 0.0);
        for (int64_t id : u->entradas) sumar_corriente(solido, id, 1.0, false, 1.0);
        for (int64_t id : u->salidas) sumar_corriente(solido, id, -1.0, false, 1.0);
        agregar_fila(solido);

        if (config.modo == ModoBalanceMetalurgico::SolidosAgua) {
            FilaBalance liquido;
            liquido.coef.assign(variables.size(), 0.0);
            for (int64_t id : u->entradas) sumar_corriente(liquido, id, 1.0, true, 1.0);
            for (int64_t id : u->salidas) sumar_corriente(liquido, id, -1.0, true, 1.0);
            agregar_fila(liquido);
        }

        if (config.modo == ModoBalanceMetalurgico::Solidos) continue;
        if (elementos.empty()) continue;

        bool unidad_con_ensayos = true;
        bool algun_ensayo = false;
        std::vector<int64_t> participantes = u->entradas;
        participantes.insert(participantes.end(), u->salidas.begin(), u->salidas.end());
        for (int64_t id : participantes) {
            auto it = indice.find(id);
            if (it == indice.end()) continue;
            const auto& c = config.corrientes[it->second];
            if (c.ensayos.empty()) { unidad_con_ensayos = false; break; }
            for (const auto& [elemento, fraccion] : c.ensayos) {
                (void)elemento;
                if (fraccion > 0.0) { algun_ensayo = true; break; }
            }
        }
        if (!unidad_con_ensayos || !algun_ensayo) {
            if (!unidad_con_ensayos) {
                res.avisos.push_back("Unidad '" + u->nombre +
                    "': ensayos incompletos; se omiten los balances por elemento.");
            }
            continue;
        }

        for (const auto& elemento : elementos) {
            FilaBalance fila;
            fila.coef.assign(variables.size(), 0.0);
            bool tiene_coeficiente = false;
            for (int64_t id : u->entradas) {
                auto it = indice.find(id);
                if (it == indice.end()) continue;
                auto e = config.corrientes[it->second].ensayos.find(elemento);
                if (e != config.corrientes[it->second].ensayos.end()) {
                    sumar_corriente(fila, id, 1.0, false, e->second);
                    if (e->second != 0.0) tiene_coeficiente = true;
                }
            }
            for (int64_t id : u->salidas) {
                auto it = indice.find(id);
                if (it == indice.end()) continue;
                auto e = config.corrientes[it->second].ensayos.find(elemento);
                if (e != config.corrientes[it->second].ensayos.end()) {
                    sumar_corriente(fila, id, -1.0, false, e->second);
                    if (e->second != 0.0) tiene_coeficiente = true;
                }
            }
            if (tiene_coeficiente) agregar_fila(fila);
        }
    }

    if (filas.empty()) {
        res.mensaje = "No se generaron restricciones. Revise las unidades y las corrientes.";
        return res;
    }

    for (const auto& [id, i] : indice) {
        const auto& c = config.corrientes[i];
        double suma = 0.0;
        for (const auto& [elemento, fraccion] : c.ensayos) {
            (void)elemento;
            suma += fraccion;
        }
        if (!c.ensayos.empty() && (suma < 0.95 || suma > 1.05)) {
            res.avisos.push_back("Corriente '" + c.nombre + "': los ensayos suman " +
                                 formatear(suma * 100.0) + "%.");
        }
    }

    int iteraciones = 0;
    int ajustadas_cero = 0;
    bool inconsistencias = false;
    std::vector<double> valores(variables.size(), 0.0);
    std::vector<char> bloqueada(variables.size(), 0);
    for (size_t i = 0; i < variables.size(); ++i) {
        bloqueada[i] = variables[i].fijo ? 1 : 0;
        valores[i] = variables[i].fijo ? variables[i].valor : variables[i].medido;
    }

    for (int ciclo = 0; ciclo <= static_cast<int>(variables.size()) + 2; ++ciclo) {
        ++iteraciones;
        std::vector<size_t> libres;
        for (size_t i = 0; i < variables.size(); ++i) {
            if (!bloqueada[i]) libres.push_back(i);
        }

        std::vector<std::vector<double>> a_reducida;
        std::vector<double> b_reducida;
        for (const auto& fila : filas) {
            std::vector<double> reducida(libres.size(), 0.0);
            double b = fila.rhs;
            for (size_t i = 0; i < variables.size(); ++i) {
                if (bloqueada[i]) b -= fila.coef[i] * valores[i];
            }
            bool alguno = false;
            for (size_t k = 0; k < libres.size(); ++k) {
                reducida[k] = fila.coef[libres[k]];
                if (std::fabs(reducida[k]) > 0.0) alguno = true;
            }
            if (!alguno) {
                if (std::fabs(b) > 1e-7) inconsistencias = true;
                continue;
            }
            a_reducida.push_back(std::move(reducida));
            b_reducida.push_back(b);
        }

        if (libres.empty()) {
            if (!inconsistencias) res.ok = true;
            break;
        }

        const size_t nf = libres.size();
        const size_t nr = a_reducida.size();
        const size_t dim = nf + nr;
        std::vector<std::vector<double>> kkt(dim, std::vector<double>(dim, 0.0));
        std::vector<double> rhs(dim, 0.0);
        for (size_t k = 0; k < nf; ++k) {
            const auto& v = variables[libres[k]];
            const double regularizacion = v.peso > 0.0 ? 0.0 : 1e-8;
            kkt[k][k] = v.peso + regularizacion;
            rhs[k] = v.peso * v.medido;
        }
        for (size_t f = 0; f < nr; ++f) {
            for (size_t k = 0; k < nf; ++k) {
                kkt[k][nf + f] = a_reducida[f][k];
                kkt[nf + f][k] = a_reducida[f][k];
            }
            rhs[nf + f] = b_reducida[f];
        }

        std::vector<double> solucion;
        if (!resolver_lineal(kkt, rhs, solucion)) {
            res.mensaje = "Sistema singular. Revise unidades duplicadas, ensayos o caudales fijos.";
            return res;
        }
        for (size_t k = 0; k < nf; ++k) valores[libres[k]] = solucion[k];

        int peor = -1;
        double menor = -1e-8;
        for (size_t k = 0; k < nf; ++k) {
            const size_t i = libres[k];
            if (valores[i] < menor) { menor = valores[i]; peor = static_cast<int>(k); }
        }
        if (peor >= 0) {
            const size_t i = libres[static_cast<size_t>(peor)];
            bloqueada[i] = 1;
            valores[i] = 0.0;
            ++ajustadas_cero;
            continue;
        }
        for (size_t k = 0; k < nf; ++k) {
            if (valores[libres[k]] < 0.0) valores[libres[k]] = 0.0;
        }
        res.ok = !inconsistencias;
        break;
    }

    if (inconsistencias) {
        std::ostringstream os;
        os << "Restricciones incompatibles con las corrientes fijas o sin varianza.";
        res.mensaje = os.str();
        return res;
    }
    if (!res.ok) {
        res.mensaje = "No se alcanzó una solución factible con no negatividad.";
        return res;
    }

    std::vector<char> libre_inicial(variables.size(), 0);
    for (size_t i = 0; i < variables.size(); ++i) libre_inicial[i] = variables[i].fijo ? 0 : 1;
    int variables_libres = static_cast<int>(std::count(libre_inicial.begin(), libre_inicial.end(), 1));
    int medidas = 0;
    for (size_t i = 0; i < variables.size(); ++i) {
        if (libre_inicial[i] && variables[i].peso > 0.0) ++medidas;
    }

    res.chi2 = 0.0;
    int outliers = 0;
    for (size_t i = 0; i < variables.size(); ++i) {
        const auto& v = variables[i];
        const auto& c = config.corrientes[v.corriente];
        if (v.peso > 0.0) {
            const double diferencia = valores[i] - v.medido;
            res.chi2 += v.peso * diferencia * diferencia;
        }
        const double z = (v.sd > 0.0) ? (valores[i] - v.medido) / v.sd : 0.0;
        const bool outlier = std::fabs(z) > 3.0;
        if (outlier) ++outliers;
        if (!v.liquido) {
            res.solido_reconciliado_t_h[c.id] = valores[i];
            res.residuo_solido_t_h[c.id] = valores[i] - v.medido;
            res.z_solido[c.id] = z;
            res.outlier_solido[c.id] = outlier;
        } else {
            res.liquido_reconciliado_t_h[c.id] = valores[i];
            res.residuo_liquido_t_h[c.id] = valores[i] - v.medido;
            res.z_liquido[c.id] = z;
            res.outlier_liquido[c.id] = outlier;
        }
    }

    std::vector<std::vector<double>> matriz_rango;
    {
        std::vector<size_t> libres;
        for (size_t i = 0; i < variables.size(); ++i) {
            if (libre_inicial[i]) libres.push_back(i);
        }
        for (const auto& fila : filas) {
            std::vector<double> reducida(libres.size(), 0.0);
            bool alguno = false;
            for (size_t k = 0; k < libres.size(); ++k) {
                reducida[k] = fila.coef[libres[k]];
                if (reducida[k] != 0.0) alguno = true;
            }
            if (alguno) matriz_rango.push_back(std::move(reducida));
        }
    }
    const int rango = rango_matriz(matriz_rango);
    res.grados_libertad = std::max(0, medidas + rango - variables_libres);
    res.q_ajuste = res.grados_libertad > 0
        ? gamma_q(0.5 * res.grados_libertad, 0.5 * res.chi2)
        : 0.0;
    res.variables = static_cast<int>(variables.size());
    res.variables_medidas = medidas;
    res.restricciones = rango;
    res.iteraciones = iteraciones;

    auto valor_solido = [&](int64_t id) {
        auto it = res.solido_reconciliado_t_h.find(id);
        if (it != res.solido_reconciliado_t_h.end()) return it->second;
        auto ic = indice.find(id);
        return ic != indice.end() ? config.corrientes[ic->second].solido_t_h : 0.0;
    };
    auto valor_liquido = [&](int64_t id) {
        auto it = res.liquido_reconciliado_t_h.find(id);
        if (it != res.liquido_reconciliado_t_h.end()) return it->second;
        auto ic = indice.find(id);
        return ic != indice.end() ? config.corrientes[ic->second].liquido_t_h : 0.0;
    };

    for (const auto* u : unidades) {
        CierreUnidadBalance cierre;
        cierre.id = u->id;
        cierre.nombre = u->nombre;
        for (int64_t id : u->entradas) {
            cierre.solido_entrada_t_h += valor_solido(id);
            cierre.liquido_entrada_t_h += valor_liquido(id);
        }
        for (int64_t id : u->salidas) {
            cierre.solido_salida_t_h += valor_solido(id);
            cierre.liquido_salida_t_h += valor_liquido(id);
        }
        cierre.error_solido_t_h = cierre.solido_entrada_t_h - cierre.solido_salida_t_h;
        cierre.error_liquido_t_h = cierre.liquido_entrada_t_h - cierre.liquido_salida_t_h;
        const double escala_s = std::max({1.0, std::fabs(cierre.solido_entrada_t_h),
                                          std::fabs(cierre.solido_salida_t_h)});
        if (std::fabs(cierre.error_solido_t_h) > 1e-6 * escala_s) {
            res.avisos.push_back("Cierre de sólidos no nulo en '" + u->nombre + "'.");
        }
        if (config.modo == ModoBalanceMetalurgico::SolidosAgua) {
            const double escala_l = std::max({1.0, std::fabs(cierre.liquido_entrada_t_h),
                                              std::fabs(cierre.liquido_salida_t_h)});
            if (std::fabs(cierre.error_liquido_t_h) > 1e-6 * escala_l) {
                res.avisos.push_back("Cierre de líquido no nulo en '" + u->nombre + "'.");
            }
        }
        res.cierres.push_back(cierre);
    }

    if (ajustadas_cero > 0) {
        res.avisos.push_back("Se fijaron a cero " + std::to_string(ajustadas_cero) +
                             " variables para evitar caudales negativos.");
    }
    if (outliers > 0) {
        res.avisos.push_back(std::to_string(outliers) +
                             " mediciones con |z| > 3 (posibles valores atípicos).");
    }

    std::ostringstream os;
    os << "Balance resuelto.";
    os << " χ²=" << formatear(res.chi2);
    os << ", gl=" << res.grados_libertad;
    if (res.grados_libertad > 0) os << ", Q=" << formatear(res.q_ajuste);
    os << ". Variables: " << res.variables << ", medidas: " << res.variables_medidas
       << ", restricciones independientes: " << res.restricciones << ".";
    res.mensaje = os.str();
    return res;
}

} // namespace motores
} // namespace cma
