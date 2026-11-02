#pragma once
#include "nucleo/tipos.hpp"
#include <algorithm>
#include <cmath>
#include <functional>
#include <string>
#include <vector>

namespace cma { namespace motores {

// ─────────────────────────────────────────────────────────────────────────────
// Motor de Balance de Calor y Materiales (hoja de cálculo tipo HSC)
//
// Convenio de signos:
//   * CorrienteBalance::calor_kJ > 0  → calor perdido hacia el entorno
//   * CorrienteBalance::calor_kJ < 0  → calor aportado al proceso
//   * Ecuación de balance:  H_entrada - H_salida - Σ calor_kJ = 0
// ─────────────────────────────────────────────────────────────────────────────

enum class UnidadEnergia : int { Mcal = 0, MJ = 1, kWh = 2, kJ = 3, kcal = 4 };
enum class UnidadTemperatura : int { Celsius = 0, Kelvin = 1 };
enum class UnidadCantidad : int { kmol = 0, kg = 1, Nm3 = 2 };

inline double factor_energia_kJ(UnidadEnergia u) {
    switch (u) {
        case UnidadEnergia::Mcal: return 4184.0;
        case UnidadEnergia::MJ:   return 1000.0;
        case UnidadEnergia::kWh:  return 3600.0;
        case UnidadEnergia::kJ:   return 1.0;
        case UnidadEnergia::kcal: return 4.184;
    }
    return 1.0;
}

inline double energia_a_kJ(double valor, UnidadEnergia u) {
    return valor * factor_energia_kJ(u);
}

inline double energia_desde_kJ(double kJ, UnidadEnergia u) {
    return kJ / factor_energia_kJ(u);
}

inline double temperatura_a_K(double valor, UnidadTemperatura u) {
    return (u == UnidadTemperatura::Celsius) ? valor + 273.15 : valor;
}

inline double temperatura_desde_K(double K, UnidadTemperatura u) {
    return (u == UnidadTemperatura::Celsius) ? K - 273.15 : K;
}

// ── Fila (especie dentro de una corriente) ──────────────────────────────────
struct FilaBalance {
    std::string formula;                     // etiqueta introducida por el usuario
    const nucleo::Especie* especie = nullptr; // datos termodinámicos (nullptr = sin datos)

    double temperatura_K = nucleo::STP_TEMP;
    double presion_bar   = 1.0;
    double kmol          = 0.0;

    // Resultados (se llenan con BalanceHoja::actualizar)
    double masa_kg         = 0.0;  // kg/h
    double volumen_nm3     = 0.0;  // Nm³/h (gases a 0 °C / 1 atm; condensados en m³)
    double cp_kJ_K         = 0.0;  // kJ/K
    double h_formacion_kJ  = 0.0;  // n·ΔHf(298)
    double h_sensible_kJ   = 0.0;  // n·(H(T)-H(298)) = Heat Content (HSC)
    double h_total_kJ      = 0.0;  // n·H_abs(T) = Total H (HSC)
    double exergia_fisica_kJ   = 0.0;
    double exergia_quimica_kJ  = 0.0;
    double exergia_total_kJ    = 0.0;

    static double volumen_molar_nm3(const nucleo::Especie* sp) {
        if (sp == nullptr) return 0.0;
        if (sp->fase_std == nucleo::Fase::Gas) return 22.414;
        if (sp->fase_std == nucleo::Fase::Liquido) return sp->masa_molar / 1000.0;
        if (sp->fase_std == nucleo::Fase::Solido) return sp->masa_molar / 2500.0;
        return 0.0;
    }

    void fijar_masa_kg(double kg) {
        if (especie != nullptr && especie->masa_molar > 0.0) {
            kmol = kg / especie->masa_molar;
            masa_kg = kg;
        }
    }

    void fijar_volumen_nm3(double nm3) {
        const double vm = volumen_molar_nm3(especie);
        if (vm > 0.0) {
            kmol = nm3 / vm;
            volumen_nm3 = nm3;
        }
    }
};

// ── Corriente (grupo de filas) ──────────────────────────────────────────────
struct CorrienteBalance {
    std::string nombre;
    std::vector<FilaBalance> filas;
    double calor_kJ = 0.0;        // > 0 pérdida al entorno, < 0 aporte externo
    bool temperatura_fija = false; // si true, Temp. Balance mantiene fija esta corriente
};

// ── Área de balance ─────────────────────────────────────────────────────────
struct AreaBalance {
    std::string nombre = "Balance Area 1";
    std::vector<CorrienteBalance> entradas;
    std::vector<CorrienteBalance> salidas;
};

// ── Balance elemental ───────────────────────────────────────────────────────
struct BalanceElemento {
    std::string elemento;
    double masa_atomica = 0.0;
    double in_kmol_at   = 0.0;
    double out_kmol_at  = 0.0;
    double in_kg        = 0.0;
    double out_kg       = 0.0;

    double balance_kmol_at() const { return in_kmol_at - out_kmol_at; }
    double balance_kg() const { return in_kg - out_kg; }
    double error_pct() const {
        double ref = std::max(std::abs(in_kg), std::abs(out_kg));
        if (ref < 1e-12) return 0.0;
        return (in_kg - out_kg) / ref * 100.0;
    }
};

// ── Resultado global del área ───────────────────────────────────────────────
struct ResultadoBalance {
    double kmol_entrada = 0.0, kmol_salida = 0.0;
    double masa_entrada_kg = 0.0, masa_salida_kg = 0.0;
    double nm3_entrada = 0.0, nm3_salida = 0.0;
    double h_formacion_entrada_kJ = 0.0, h_formacion_salida_kJ = 0.0;
    double h_sensible_entrada_kJ  = 0.0, h_sensible_salida_kJ  = 0.0;
    double h_entrada_kJ = 0.0, h_salida_kJ = 0.0;
    double exergia_entrada_kJ = 0.0, exergia_salida_kJ = 0.0;
    double calor_externo_kJ = 0.0;   // Σ calor_kJ (positivo = pérdida neta)

    double residual_energia_kJ = 0.0; // H_in - H_out - calor
    double residual_masa_kg    = 0.0; // masa_in - masa_out
    double error_energia_pct   = 0.0;
    double error_masa_pct      = 0.0;

    bool balance_energia_ok = false;
    bool balance_masa_ok    = false;

    double exergia_destruida_kJ = 0.0;
    double eficiencia_exergetica_pct = 0.0;
    double cp_total_entrada_kJ_K = 0.0;
    double cp_total_salida_kJ_K  = 0.0;

    int filas_sin_especie = 0;
    std::vector<std::string> avisos;
};

// ── Motor ───────────────────────────────────────────────────────────────────
class BalanceHoja {
public:
    // Resuelve una etiqueta (fórmula o nombre) a una especie de la base de datos.
    // Se inyecta desde la interfaz para mantener el motor libre de persistencia.
    using ResolvedorEspecie = std::function<const nucleo::Especie*(const std::string&)>;

    struct ResultadoTemperatura {
        bool factible  = false;  // existe solución dentro del rango explorado
        bool convergio = false;  // la tolerancia solicitada fue alcanzada
        double temperatura_K = nucleo::STP_TEMP;
        double residual_kJ   = 0.0;
        int iteraciones      = 0;
        std::string mensaje;
    };

    // Resuelve especies faltantes y recalcula todas las propiedades/resultados.
    void actualizar(AreaBalance& area, const ResolvedorEspecie& resolver = {});

    // Totales y diagnóstico del área (usa los resultados ya calculados).
    ResultadoBalance evaluar(const AreaBalance& area,
                             double tol_energia_kJ = 0.1,
                             double tol_masa_kg = 1e-6) const;

    // Temperatura de equilibrio térmico (ΔH = 0). Ajusta las temperaturas de
    // salida. Si mantener_offsets=true conserva las diferencias entre salidas.
    // Barrido de malla para localizar el primer cruce por cero + bisección:
    // robusto aunque los polinomios de Cp se degraden en los extremos.
    ResultadoTemperatura temperatura_equilibrio(AreaBalance& area,
                                                bool mantener_offsets = false,
                                                double t_min_K = 250.0,
                                                double t_max_K = 6000.0,
                                                double tol_kJ = 1e-6,
                                                int max_iter = 200);

    // Calor externo (misma convención que CorrienteBalance::calor_kJ) necesario
    // para que la primera corriente de salida quede a T_objetivo_K.
    double calor_requerido(const AreaBalance& area, double T_objetivo_K,
                           bool mantener_offsets = false) const;

    // Ajusta el calor de la primera corriente de salida para cerrar el balance
    // a las temperaturas actuales. Devuelve el calor externo total resultante.
    double ajustar_calor_de_area(AreaBalance& area);

    // Temperatura óptima de precalentamiento de una corriente de entrada para
    // compensar el calentamiento eléctrico / externo (ΔH = 0 a T_salida actual).
    ResultadoTemperatura temperatura_precalentamiento(AreaBalance& area,
                                                      int idx_entrada = 0,
                                                      double t_min_K = 250.0,
                                                      double t_max_K = 4000.0,
                                                      double tol_kJ = 1e-6,
                                                      int max_iter = 200);

    // Escala todas las cantidades (kmol) y calores externos del área por 'factor'
    // (útil para escalar del laboratorio a escala piloto o industrial).
    void escalar_area(AreaBalance& area, double factor);

    // Genera un balance teórico de materia y calor a partir de una ecuación de
    // reacción química (reactivos -> productos) con conversión y exceso opcionales.
    void poblar_desde_reaccion(AreaBalance& area,
                               const nucleo::Reaccion& rxn,
                               const std::map<int64_t, nucleo::Especie>& db,
                               double kmol_base = 1.0,
                               double conversion = 1.0,
                               double exceso_oxigeno_pct = 0.0,
                               bool incluir_n2_aire = false,
                               double t_entrada_K = nucleo::STP_TEMP,
                               double t_salida_K = nucleo::STP_TEMP);

    // Balance por elemento químico (kmol-at y kg), usando la tabla periódica.
    std::vector<BalanceElemento> balance_elemental(const AreaBalance& area) const;

    // Exergía química estándar de una fila (Szargut, kJ/mol).
    static double exergia_quimica_estandar(const FilaBalance& fila);

    // Entalpía absoluta de una fila a una temperatura dada (kJ).
    static double entalpia_fila(const FilaBalance& fila, double T_K);
};

}} // namespace cma::motores
