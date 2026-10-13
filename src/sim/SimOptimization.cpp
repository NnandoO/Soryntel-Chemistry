#include "SimOptimization.hpp"
#include <set>

namespace cma::application::modules {

// ═══════════════════════════════════════════════════════════════════════════
// Helpers privados
// ═══════════════════════════════════════════════════════════════════════════

double SimOptimization::clamp_variable(const VariableDecision& v, double val) {
    return std::max(v.min, std::min(v.max, val));
}

std::map<std::string, double> SimOptimization::a_mapa(
    const std::vector<VariableDecision>& vars,
    const std::vector<double>& x)
{
    std::map<std::string, double> m;
    for (size_t i = 0; i < vars.size(); ++i)
        m[vars[i].nombre] = x[i];
    return m;
}

double SimOptimization::evaluar(FuncObjetivo f,
    const std::vector<VariableDecision>& vars,
    const std::vector<double>& x)
{
    return f(a_mapa(vars, x));
}

double SimOptimization::percentile(std::vector<double>& data, double p) {
    if (data.empty()) return 0.0;
    std::sort(data.begin(), data.end());
    size_t idx = (size_t)(p / 100.0 * (data.size() - 1));
    return data[idx];
}

// ═══════════════════════════════════════════════════════════════════════════
// Monte-Carlo
// ═══════════════════════════════════════════════════════════════════════════

ResultadoOptimizacion SimOptimization::monte_carlo(
    const std::vector<VariableDecision>& vars,
    FuncObjetivo f, int n, std::mt19937& rng)
{
    ResultadoOptimizacion res;
    res.historial_objetivo.reserve(n);

    // Variables activas
    std::vector<const VariableDecision*> activas;
    for (auto& v : vars)
        if (v.activa) activas.push_back(&v);

    if (activas.empty()) {
        res.mensaje = "No hay variables activas";
        return res;
    }

    double best = std::numeric_limits<double>::max();
    std::vector<double> best_x(vars.size(), 0.0);

    // Acumular estadísticas en lotes para gráficos de convergencia
    int lote = std::max(1, n / 100);
    double suma = 0, suma_sq = 0;
    double min_val = std::numeric_limits<double>::max();
    double max_val = std::numeric_limits<double>::lowest();
    int cuenta = 0;

    for (int i = 0; i < n; ++i) {
        std::vector<double> x(vars.size());
        for (size_t j = 0; j < vars.size(); ++j) {
            if (!vars[j].activa) {
                x[j] = vars[j].valor_inicial;
                continue;
            }
            std::uniform_real_distribution<double> unif(vars[j].min, vars[j].max);
            x[j] = unif(rng);
        }

        double val = evaluar(f, vars, x);
        res.historial_objetivo.push_back(val);
        suma += val;
        suma_sq += val * val;
        if (val < min_val) min_val = val;
        if (val > max_val) max_val = val;
        cuenta++;

        if (val < best) {
            best = val;
            best_x = x;
        }

        if (cuenta >= lote) {
            double media = suma / cuenta;
            double varianza = suma_sq / cuenta - media * media;
            res.valores_media.push_back(media);
            res.valores_std.push_back(std::sqrt(std::max(0.0, varianza)));
            res.valores_min.push_back(min_val);
            res.valores_max.push_back(max_val);
            suma = 0; suma_sq = 0;
            min_val = std::numeric_limits<double>::max();
            max_val = std::numeric_limits<double>::lowest();
            cuenta = 0;
        }
    }

    if (!res.historial_objetivo.empty()) {
        auto copia = res.historial_objetivo;
        res.percentil_5  = percentile(copia, 5.0);
        auto copia2 = res.historial_objetivo;
        res.percentil_95 = percentile(copia2, 95.0);
    }

    res.mejor_objetivo = best;
    res.mejores_valores = a_mapa(vars, best_x);
    res.iteraciones = n;
    res.evaluaciones = n;
    res.convergio = true;
    res.mensaje = "Monte-Carlo: " + std::to_string(n) + " muestras evaluadas";
    return res;
}

// ═══════════════════════════════════════════════════════════════════════════
// PSO (Particle Swarm Optimization)
// ═══════════════════════════════════════════════════════════════════════════

ResultadoOptimizacion SimOptimization::pso(
    const std::vector<VariableDecision>& vars,
    FuncObjetivo f, int n_iter, std::mt19937& rng)
{
    ResultadoOptimizacion res;
    int n_vars = 0;
    for (auto& v : vars) if (v.activa) n_vars++;
    if (n_vars == 0) {
        res.mensaje = "No hay variables activas";
        return res;
    }

    int n_particles = std::max(10, 5 * n_vars);
    double w = 0.7;   // inercia
    double c1 = 1.5;   // coeficiente cognitivo
    double c2 = 1.5;   // coeficiente social

    // Partículas: posición y velocidad
    std::vector<std::vector<double>> pos(n_particles, std::vector<double>(vars.size()));
    std::vector<std::vector<double>> vel(n_particles, std::vector<double>(vars.size()));
    std::vector<std::vector<double>> pbest(n_particles, std::vector<double>(vars.size()));
    std::vector<double> pbest_val(n_particles, std::numeric_limits<double>::max());

    std::vector<double> gbest(vars.size(), 0.0);
    double gbest_val = std::numeric_limits<double>::max();

    // Inicializar partículas
    for (int p = 0; p < n_particles; ++p) {
        for (size_t j = 0; j < vars.size(); ++j) {
            if (!vars[j].activa) {
                pos[p][j] = vars[j].valor_inicial;
                vel[p][j] = 0.0;
                pbest[p][j] = vars[j].valor_inicial;
                continue;
            }
            std::uniform_real_distribution<double> unif(vars[j].min, vars[j].max);
            pos[p][j] = unif(rng);
            double rango = vars[j].max - vars[j].min;
            vel[p][j] = unif(rng) * rango * 0.1 - rango * 0.05;
            pbest[p][j] = pos[p][j];
        }
        double val = evaluar(f, vars, pos[p]);
        pbest_val[p] = val;
        if (val < gbest_val) {
            gbest_val = val;
            gbest = pos[p];
        }
    }

    res.evaluaciones = n_particles;

    // Iteraciones
    for (int iter = 0; iter < n_iter; ++iter) {
        for (int p = 0; p < n_particles; ++p) {
            for (size_t j = 0; j < vars.size(); ++j) {
                if (!vars[j].activa) continue;
                std::uniform_real_distribution<double> unif01(0.0, 1.0);
                double r1 = unif01(rng);
                double r2 = unif01(rng);
                vel[p][j] = w * vel[p][j]
                          + c1 * r1 * (pbest[p][j] - pos[p][j])
                          + c2 * r2 * (gbest[j] - pos[p][j]);
                pos[p][j] = clamp_variable(vars[j], pos[p][j] + vel[p][j]);
            }
            double val = evaluar(f, vars, pos[p]);
            res.evaluaciones++;
            if (val < pbest_val[p]) {
                pbest_val[p] = val;
                pbest[p] = pos[p];
                if (val < gbest_val) {
                    gbest_val = val;
                    gbest = pos[p];
                }
            }
        }
        res.historial_objetivo.push_back(gbest_val);
    }

    res.mejor_objetivo = gbest_val;
    res.mejores_valores = a_mapa(vars, gbest);
    res.iteraciones = n_iter;
    res.convergio = true;
    res.mensaje = "PSO: convergió en " + std::to_string(n_iter) + " iteraciones";
    return res;
}

// ═══════════════════════════════════════════════════════════════════════════
// Nelder-Mead Simplex
// ═══════════════════════════════════════════════════════════════════════════

ResultadoOptimizacion SimOptimization::nelder_mead(
    const std::vector<VariableDecision>& vars,
    FuncObjetivo f, int n_iter)
{
    ResultadoOptimizacion res;
    int n = 0;
    for (auto& v : vars) if (v.activa) n++;
    if (n == 0) {
        res.mensaje = "No hay variables activas";
        return res;
    }

    double alpha = 1.0;  // reflexión
    double gamma = 2.0;  // expansión
    double rho   = 0.5;  // contracción
    double sigma = 0.5;  // reducción

    // Simplex: n+1 vértices, cada uno con todas las variables
    int nv = (int)vars.size();
    std::vector<std::vector<double>> simplex(n + 1, std::vector<double>(nv));
    std::vector<double> fval(n + 1);

    // Vértice inicial: valor_inicial
    for (int j = 0; j < nv; ++j)
        simplex[0][j] = vars[j].valor_inicial;
    fval[0] = evaluar(f, vars, simplex[0]);

    // Construir simplex inicial perturbando cada dimensión
    for (int i = 1; i <= n; ++i) {
        simplex[i] = simplex[0];
        int k = 0;
        for (int j = 0; j < nv; ++j) {
            if (!vars[j].activa) continue;
            if (k == i - 1) {
                double delta = 0.05 * (vars[j].max - vars[j].min);
                if (delta == 0) delta = 0.05;
                simplex[i][j] = clamp_variable(vars[j], simplex[0][j] + delta);
                break;
            }
            k++;
        }
        fval[i] = evaluar(f, vars, simplex[i]);
    }
    res.evaluaciones = n + 1;

    for (int iter = 0; iter < n_iter; ++iter) {
        // Ordenar vértices por valor de la función
        std::vector<int> idx(n + 1);
        for (int i = 0; i <= n; ++i) idx[i] = i;
        std::sort(idx.begin(), idx.end(), [&](int a, int b) { return fval[a] < fval[b]; });

        // Centroid (excluyendo el peor)
        std::vector<double> centro(nv, 0.0);
        for (int i = 0; i < n; ++i) {
            for (int j = 0; j < nv; ++j)
                centro[j] += simplex[idx[i]][j];
        }
        for (int j = 0; j < nv; ++j) centro[j] /= n;

        int worst = idx[n];
        int best  = idx[0];

        // Reflexión
        std::vector<double> xr(nv);
        for (int j = 0; j < nv; ++j)
            xr[j] = clamp_variable(vars[j], centro[j] + alpha * (centro[j] - simplex[worst][j]));
        double fr = evaluar(f, vars, xr);
        res.evaluaciones++;

        if (fr < fval[best]) {
            // Expansión
            std::vector<double> xe(nv);
            for (int j = 0; j < nv; ++j)
                xe[j] = clamp_variable(vars[j], centro[j] + gamma * (xr[j] - centro[j]));
            double fe = evaluar(f, vars, xe);
            res.evaluaciones++;
            if (fe < fr) {
                simplex[worst] = xe;
                fval[worst] = fe;
            } else {
                simplex[worst] = xr;
                fval[worst] = fr;
            }
        } else if (fr < fval[idx[n - 1]]) {
            simplex[worst] = xr;
            fval[worst] = fr;
        } else {
            // Contracción
            std::vector<double> xc(nv);
            for (int j = 0; j < nv; ++j)
                xc[j] = clamp_variable(vars[j], centro[j] + rho * (simplex[worst][j] - centro[j]));
            double fc = evaluar(f, vars, xc);
            res.evaluaciones++;
            if (fc < fval[worst]) {
                simplex[worst] = xc;
                fval[worst] = fc;
            } else {
                // Reducción
                for (int i = 1; i <= n; ++i) {
                    int vi = idx[i];
                    for (int j = 0; j < nv; ++j)
                        simplex[vi][j] = clamp_variable(vars[j],
                            simplex[best][j] + sigma * (simplex[vi][j] - simplex[best][j]));
                    fval[vi] = evaluar(f, vars, simplex[vi]);
                    res.evaluaciones++;
                }
            }
        }

        res.historial_objetivo.push_back(fval[best]);

        // Criterio de convergencia: simplex suficientemente pequeño
        double spread = 0.0;
        for (int i = 1; i <= n; ++i)
            for (int j = 0; j < nv; ++j)
                spread += (simplex[i][j] - simplex[0][j]) * (simplex[i][j] - simplex[0][j]);
        if (spread < 1e-12) break;
    }

    // Encontrar el mejor vértice
    int best_idx = 0;
    for (int i = 1; i <= n; ++i)
        if (fval[i] < fval[best_idx]) best_idx = i;

    res.mejor_objetivo = fval[best_idx];
    res.mejores_valores = a_mapa(vars, simplex[best_idx]);
    res.iteraciones = (int)res.historial_objetivo.size();
    res.convergio = true;
    res.mensaje = "Simplex (Nelder-Mead): " + std::to_string(res.iteraciones) + " iteraciones";
    return res;
}

// ═══════════════════════════════════════════════════════════════════════════
// SQP simplificado (gradiente numérico + búsqueda de línea)
// ═══════════════════════════════════════════════════════════════════════════

ResultadoOptimizacion SimOptimization::sqp(
    const std::vector<VariableDecision>& vars,
    FuncObjetivo f, int n_iter)
{
    ResultadoOptimizacion res;
    std::vector<double> x(vars.size());
    std::vector<double> grad(vars.size());
    std::vector<bool> activa(vars.size());

    for (size_t j = 0; j < vars.size(); ++j) {
        x[j] = vars[j].valor_inicial;
        activa[j] = vars[j].activa;
    }

    double f_curr = evaluar(f, vars, x);
    res.evaluaciones = 1;

    double eps = 1e-6;
    double lr = 0.01;

    for (int iter = 0; iter < n_iter; ++iter) {
        // Gradiente numérico (diferencias finitas)
        for (size_t j = 0; j < vars.size(); ++j) {
            if (!activa[j]) {
                grad[j] = 0.0;
                continue;
            }
            std::vector<double> x_plus = x;
            std::vector<double> x_minus = x;
            double h = eps * (1.0 + std::abs(x[j]));
            x_plus[j] = clamp_variable(vars[j], x[j] + h);
            x_minus[j] = clamp_variable(vars[j], x[j] - h);
            double f_plus  = evaluar(f, vars, x_plus);
            double f_minus = evaluar(f, vars, x_minus);
            grad[j] = (f_plus - f_minus) / (2.0 * h);
            res.evaluaciones += 2;
        }

        // Normalizar gradiente
        double norm = 0.0;
        for (size_t j = 0; j < vars.size(); ++j)
            norm += grad[j] * grad[j];
        norm = std::sqrt(norm);
        if (norm < 1e-10) break;

        // Búsqueda de línea (backtracking)
        double step = lr;
        std::vector<double> x_new(vars.size());
        bool mejorado = false;
        for (int ls = 0; ls < 30; ++ls) {
            for (size_t j = 0; j < vars.size(); ++j) {
                if (activa[j])
                    x_new[j] = clamp_variable(vars[j], x[j] - step * grad[j] / norm);
                else
                    x_new[j] = x[j];
            }
            double f_new = evaluar(f, vars, x_new);
            res.evaluaciones++;
            if (f_new < f_curr - 1e-10) {
                x = x_new;
                f_curr = f_new;
                mejorado = true;
                break;
            }
            step *= 0.5;
        }
        if (!mejorado) break;

        res.historial_objetivo.push_back(f_curr);
    }

    res.mejor_objetivo = f_curr;
    res.mejores_valores = a_mapa(vars, x);
    res.iteraciones = (int)res.historial_objetivo.size();
    res.convergio = true;
    res.mensaje = "SQP (gradiente): " + std::to_string(res.iteraciones) + " iteraciones";
    return res;
}

// ═══════════════════════════════════════════════════════════════════════════
// Orquestador
// ═══════════════════════════════════════════════════════════════════════════

ResultadoOptimizacion SimOptimization::optimizar(
    const std::vector<VariableDecision>& variables,
    FuncObjetivo f, MetodoOptimizacion metodo,
    int max_iteraciones, int seed)
{
    std::mt19937 rng(seed);
    ResultadoOptimizacion invalid;
    if (!f || max_iteraciones < 1 || max_iteraciones > 1000000 || variables.empty()) {
        invalid.mensaje = "Configuración de optimización inválida"; return invalid;
    }
    std::set<std::string> names;
    for (const auto& var : variables) {
        if (var.nombre.empty() || !names.insert(var.nombre).second ||
            !std::isfinite(var.min) || !std::isfinite(var.max) ||
            !std::isfinite(var.valor_inicial) || var.min > var.max ||
            var.valor_inicial < var.min || var.valor_inicial > var.max) {
            invalid.mensaje = "Variable de decisión inválida"; return invalid;
        }
    }

    switch (metodo) {
        case MetodoOptimizacion::MonteCarlo:
            return monte_carlo(variables, f, max_iteraciones, rng);
        case MetodoOptimizacion::PSO:
            return pso(variables, f, max_iteraciones, rng);
        case MetodoOptimizacion::Simplex:
            return nelder_mead(variables, f, max_iteraciones);
        case MetodoOptimizacion::SQP:
            return sqp(variables, f, max_iteraciones);
    }
    ResultadoOptimizacion r;
    r.mensaje = "Método desconocido";
    return r;
}

// ═══════════════════════════════════════════════════════════════════════════
// Análisis de sensibilidad
// ═══════════════════════════════════════════════════════════════════════════

std::vector<std::pair<double, double>> SimOptimization::sensibilidad(
    const std::vector<VariableDecision>& variables,
    const std::string& variable_analizar,
    FuncObjetivo f, int n_puntos)
{
    std::vector<std::pair<double, double>> resultado;
    if (n_puntos < 2 || n_puntos > 1000000 || !f) return resultado;
    resultado.reserve(static_cast<size_t>(n_puntos));

    // Punto base: valores iniciales
    std::map<std::string, double> base;
    for (auto& v : variables)
        base[v.nombre] = v.valor_inicial;

    // Encontrar la variable a analizar
    const VariableDecision* var = nullptr;
    for (auto& v : variables)
        if (v.nombre == variable_analizar) { var = &v; break; }

    if (!var || !f || n_puntos < 2 || n_puntos > 1000000 ||
        !std::isfinite(var->min) || !std::isfinite(var->max) || var->min > var->max)
        return resultado;

    for (int i = 0; i < n_puntos; ++i) {
        double t = (double)i / (n_puntos - 1);
        double val = var->min + t * (var->max - var->min);
        base[variable_analizar] = val;
        resultado.push_back({val, f(base)});
    }

    return resultado;
}

} // namespace cma::application::modules
