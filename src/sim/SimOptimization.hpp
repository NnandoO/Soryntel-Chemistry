#pragma once
// ═══════════════════════════════════════════════════════════════════════════
// SimOptimization — Optimización de modelos de simulación
//
// Replica el módulo "Sim Model Optimization" de HSC Chemistry 10, que soporta:
//   - Monte-Carlo (análisis de incertidumbre)
//   - PSO (Particle Swarm Optimization)
//   - Simplex (Nelder-Mead)
//   - MFit / SQP (Sequential Quadratic Programming via least-squares)
//
// El usuario define variables de decisión (rangos), una función objetivo
// y el método de optimización. El motor explora el espacio de soluciones
// y retorna el óptimo encontrado.
// ═══════════════════════════════════════════════════════════════════════════
#include <vector>
#include <string>
#include <map>
#include <functional>
#include <random>
#include <cmath>
#include <algorithm>
#include <limits>

namespace hsc_clone {

enum class MetodoOptimizacion {
    MonteCarlo,
    PSO,
    Simplex,
    SQP
};

struct VariableDecision {
    std::string nombre;
    double min = 0.0;
    double max = 1.0;
    double valor_inicial = 0.5;
    bool activa = true;
};

struct ResultadoOptimizacion {
    bool convergio = false;
    std::map<std::string, double> mejores_valores;
    double mejor_objetivo = std::numeric_limits<double>::max();
    int iteraciones = 0;
    int evaluaciones = 0;
    std::string mensaje;

    // Para Monte-Carlo: distribución de resultados
    std::vector<double> historial_objetivo;
    std::vector<double> valores_media;
    std::vector<double> valores_std;
    std::vector<double> valores_min;
    std::vector<double> valores_max;
    double percentil_5  = 0.0;
    double percentil_95 = 0.0;
};

class SimOptimization {
public:
    /// Función objetivo: recibe un mapa nombre->valor y retorna el valor a minimizar
    using FuncObjetivo = std::function<double(const std::map<std::string, double>&)>;

    ResultadoOptimizacion optimizar(
        const std::vector<VariableDecision>& variables,
        FuncObjetivo f,
        MetodoOptimizacion metodo,
        int max_iteraciones = 200,
        int seed = 42);

    /// Análisis de sensibilidad: varía una sola variable manteniendo las demás fijas
    std::vector<std::pair<double, double>> sensibilidad(
        const std::vector<VariableDecision>& variables,
        const std::string& variable_analizar,
        FuncObjetivo f,
        int n_puntos = 50);

private:
    // ── Monte-Carlo: muestreo aleatorio uniforme en el hipercubo ──
    ResultadoOptimizacion monte_carlo(
        const std::vector<VariableDecision>& vars,
        FuncObjetivo f,
        int n, std::mt19937& rng);

    // ── PSO (Particle Swarm Optimization) ──
    ResultadoOptimizacion pso(
        const std::vector<VariableDecision>& vars,
        FuncObjetivo f,
        int n_iter, std::mt19937& rng);

    // ── Simplex (Nelder-Mead) ──
    ResultadoOptimizacion nelder_mead(
        const std::vector<VariableDecision>& vars,
        FuncObjetivo f,
        int n_iter);

    // ── SQP simplificado (gradiente numérico + línea de búsqueda) ──
    ResultadoOptimizacion sqp(
        const std::vector<VariableDecision>& vars,
        FuncObjetivo f,
        int n_iter);

    double evaluar(FuncObjetivo f, const std::vector<VariableDecision>& vars,
                   const std::vector<double>& x);
    std::map<std::string, double> a_mapa(const std::vector<VariableDecision>& vars,
                                         const std::vector<double>& x);
    double clamp_variable(const VariableDecision& v, double val);
    double percentile(std::vector<double>& data, double p);
};

} // namespace hsc_clone
