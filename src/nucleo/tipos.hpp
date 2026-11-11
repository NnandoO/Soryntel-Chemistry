#pragma once
#include <string>
#include <map>
#include <vector>
#include <cmath>
#include <cstdint>
#include <stdexcept>
#include <sstream>
#include <algorithm>
#include "domain/DataProvenance.hpp"

namespace cma { namespace nucleo {

// ── Constantes físicas ──────────────────────────────────────────────────────
constexpr double R_GAS       = 8.314462618;    // J/(mol·K)
constexpr double R_GAS_ATM   = 0.08205736608; // L·atm/(mol·K)
constexpr double STP_TEMP    = 298.15;         // K
constexpr double STP_PRES    = 1.0;            // atm
constexpr double FARADAY     = 96485.33212;    // C/mol
constexpr double CAL_A_J     = 4.184;          // J/cal
constexpr double CERO_KELVIN = 273.15;         // 0°C en K

// ── Tipos de fase ───────────────────────────────────────────────────────────
enum class Fase : uint8_t {
    Liquido, Gas, Solido, Mixto, Desconocido
};

constexpr const char* nombre_fase(Fase f) {
    switch (f) {
        case Fase::Liquido:  return "Líquido";
        case Fase::Gas:      return "Gas";
        case Fase::Solido:   return "Sólido";
        case Fase::Mixto:    return "Mixto";
        default:             return "Desconocido";
    }
}

// ── Modelos de capacidad calorífica Cp(T) ──────────────────────────────────
enum class ModeloCp : uint8_t {
    POLINOMIAL_ESTANDAR = 0, // Cp = a + b*T + c*T^2 + d*T^3
    MAIER_KELLEY        = 1, // Cp = a + b*T + c*T^-2 + d*T^-3
    SHOMATE_NIST        = 2, // Cp = A + B*t + C*t^2 + D*t^3 + E/t^2 (t = T/1000)
    CONSTANTE           = 3  // Cp = a
};

constexpr const char* nombre_modelo_cp(ModeloCp m) {
    switch (m) {
        case ModeloCp::POLINOMIAL_ESTANDAR: return "Polinomial Estándar";
        case ModeloCp::MAIER_KELLEY:        return "Maier-Kelley";
        case ModeloCp::SHOMATE_NIST:        return "Shomate (NIST)";
        case ModeloCp::CONSTANTE:           return "Constante";
        default:                            return "Desconocido";
    }
}

// ── Especie química ─────────────────────────────────────────────────────────
struct Especie {
    domain::DataProvenance provenance;
    int64_t     id        = 0;
    std::string nombre;
    std::string formula;
    double      masa_molar   = 0.0;   // g/mol
    double      h_formacion  = 0.0;   // kJ/mol a 298.15 K
    double      s_formacion  = 0.0;   // J/(mol·K) a 298.15 K
    double      cp_a = 0.0, cp_b = 0.0, cp_c = 0.0, cp_d = 0.0; // Cp [J/(mol·K)]
    double      cp_e = 0.0;           // Coeficiente E para Shomate
    ModeloCp    modelo_cp = ModeloCp::POLINOMIAL_ESTANDAR;
    double      T_min = 298.15;       // K
    double      T_max = 2000.0;       // K

    // Coeficientes Antoine: log10(P_sat) = A - B/(C + T) [P en mmHg, T en °C]
    double      antoine_a = 0.0, antoine_b = 0.0, antoine_c = 0.0;
    double      temp_min_antoine = 0.0, temp_max_antoine = 0.0; // °C

    // Estado estándar y composición elemental
    Fase        fase_std = Fase::Gas;
    std::map<std::string, int> composicion_elemental;

    bool es_curada() const { return id > 0 && id <= 445; }

    bool en_rango_temperatura(double T_K) const {
        return (T_min < T_max) && (T_K >= (T_min - 1e-4)) && (T_K <= (T_max + 1e-4));
    }

    struct EstadoEvaluacionTermo {
        bool en_rango = true;
        std::string advertencia;
    };

    EstadoEvaluacionTermo comprobar_rango(double T_K) const {
        EstadoEvaluacionTermo st;
        if (T_min >= T_max) {
            st.en_rango = false;
            st.advertencia = "Rango de temperatura no valido [T_min=" + std::to_string(T_min) +
                             ", T_max=" + std::to_string(T_max) + "] para " + formula;
        } else if (T_K < (T_min - 1e-4)) {
            st.en_rango = false;
            st.advertencia = "Temperatura " + std::to_string(T_K) + " K inferior a T_min (" +
                             std::to_string(T_min) + " K) para " + formula;
        } else if (T_K > (T_max + 1e-4)) {
            st.en_rango = false;
            st.advertencia = "Temperatura " + std::to_string(T_K) + " K superior a T_max (" +
                             std::to_string(T_max) + " K) para " + formula;
        }
        return st;
    }

    double cp(double T_K) const {
        if (T_K <= 0.0) return (modelo_cp == ModeloCp::CONSTANTE) ? cp_a : 0.0;
        switch (modelo_cp) {
            case ModeloCp::POLINOMIAL_ESTANDAR: {
                return cp_a + cp_b * T_K + cp_c * T_K * T_K + cp_d * T_K * T_K * T_K;
            }
            case ModeloCp::MAIER_KELLEY: {
                double Tk = std::max(T_K, 1e-6);
                double invT = 1.0 / Tk;
                double invT2 = invT * invT;
                return cp_a + cp_b * Tk + cp_c * invT2 + cp_d * invT2 * invT;
            }
            case ModeloCp::SHOMATE_NIST: {
                double Tk = std::max(T_K, 1e-6);
                double t = Tk / 1000.0;
                return cp_a + cp_b * t + cp_c * t * t + cp_d * t * t * t + cp_e / (t * t);
            }
            case ModeloCp::CONSTANTE: {
                return cp_a;
            }
            default:
                return cp_a + cp_b * T_K + cp_c * T_K * T_K + cp_d * T_K * T_K * T_K;
        }
    }

    double entalpia_ref(double T_K) const {
        double Tr = STP_TEMP;
        if (T_K <= 0.0) T_K = 1e-6;
        switch (modelo_cp) {
            case ModeloCp::POLINOMIAL_ESTANDAR: {
                return (cp_a * (T_K - Tr)
                      + cp_b * (T_K * T_K - Tr * Tr) / 2.0
                      + cp_c * (T_K * T_K * T_K - Tr * Tr * Tr) / 3.0
                      + cp_d * (T_K * T_K * T_K * T_K - Tr * Tr * Tr * Tr) / 4.0) / 1000.0;
            }
            case ModeloCp::MAIER_KELLEY: {
                double Tk = std::max(T_K, 1e-6);
                double c_term = -cp_c * (1.0 / Tk - 1.0 / Tr);
                double d_term = -0.5 * cp_d * (1.0 / (Tk * Tk) - 1.0 / (Tr * Tr));
                return (cp_a * (Tk - Tr)
                      + cp_b * (Tk * Tk - Tr * Tr) / 2.0
                      + c_term
                      + d_term) / 1000.0;
            }
            case ModeloCp::SHOMATE_NIST: {
                double Tk = std::max(T_K, 1e-6);
                double t = Tk / 1000.0;
                double tr = Tr / 1000.0;
                return (cp_a * (t - tr)
                      + cp_b * (t * t - tr * tr) / 2.0
                      + cp_c * (t * t * t - tr * tr * tr) / 3.0
                      + cp_d * (t * t * t * t - tr * tr * tr * tr) / 4.0
                      - cp_e * (1.0 / t - 1.0 / tr));
            }
            case ModeloCp::CONSTANTE: {
                return (cp_a * (T_K - Tr)) / 1000.0;
            }
            default: {
                return (cp_a * (T_K - Tr)
                      + cp_b * (T_K * T_K - Tr * Tr) / 2.0
                      + cp_c * (T_K * T_K * T_K - Tr * Tr * Tr) / 3.0
                      + cp_d * (T_K * T_K * T_K * T_K - Tr * Tr * Tr * Tr) / 4.0) / 1000.0;
            }
        }
    }

    double entalpia_abs(double T_K) const {
        return h_formacion + entalpia_ref(T_K);
    }

    double entropia_ref(double T_K) const {
        double Tr = STP_TEMP;
        if (T_K <= 0.0) return 0.0;
        switch (modelo_cp) {
            case ModeloCp::POLINOMIAL_ESTANDAR: {
                return cp_a * std::log(T_K / Tr)
                     + cp_b * (T_K - Tr)
                     + cp_c * (T_K * T_K - Tr * Tr) / 2.0
                     + cp_d * (T_K * T_K * T_K - Tr * Tr * Tr) / 3.0;
            }
            case ModeloCp::MAIER_KELLEY: {
                double Tk = std::max(T_K, 1e-6);
                double c_term = -0.5 * cp_c * (1.0 / (Tk * Tk) - 1.0 / (Tr * Tr));
                double d_term = -(1.0 / 3.0) * cp_d * (1.0 / (Tk * Tk * Tk) - 1.0 / (Tr * Tr * Tr));
                return cp_a * std::log(Tk / Tr)
                     + cp_b * (Tk - Tr)
                     + c_term
                     + d_term;
            }
            case ModeloCp::SHOMATE_NIST: {
                double Tk = std::max(T_K, 1e-6);
                double t = Tk / 1000.0;
                double tr = Tr / 1000.0;
                return cp_a * std::log(t / tr)
                     + cp_b * (t - tr)
                     + cp_c * (t * t - tr * tr) / 2.0
                     + cp_d * (t * t * t - tr * tr * tr) / 3.0
                     - 0.5 * cp_e * (1.0 / (t * t) - 1.0 / (tr * tr));
            }
            case ModeloCp::CONSTANTE: {
                return cp_a * std::log(T_K / Tr);
            }
            default: {
                return cp_a * std::log(T_K / Tr)
                     + cp_b * (T_K - Tr)
                     + cp_c * (T_K * T_K - Tr * Tr) / 2.0
                     + cp_d * (T_K * T_K * T_K - Tr * Tr * Tr) / 3.0;
            }
        }
    }

    double entropia_abs(double T_K) const {
        return s_formacion + entropia_ref(T_K);
    }

    double energia_gibbs(double T_K) const {
        return entalpia_abs(T_K) - T_K * entropia_abs(T_K) / 1000.0;
    }

    // Presión de vapor (Antoine). Retorna atm.
    double presion_vapor(double T_K) const {
        if (antoine_a == 0 && antoine_b == 0) return 0;
        double T_C = T_K - CERO_KELVIN;
        if (T_C < temp_min_antoine || T_C > temp_max_antoine) return 0;
        double P_mmHg = std::pow(10.0, antoine_a - antoine_b / (antoine_c + T_C));
        return P_mmHg / 760.0; // mmHg -> atm
    }

    std::string resumen() const {
        std::ostringstream os;
        os << nombre << " (" << formula << ") M=" << masa_molar
           << " Hf=" << h_formacion << " Sf=" << s_formacion;
        return os.str();
    }
};

// ── Reacción química ────────────────────────────────────────────────────────
struct Reaccion {
    int64_t     id   = 0;
    std::string nombre;
    std::map<int64_t, double> estequiometria; // especie_id -> coeficiente (negativo = reactivo)
    double      delta_h_298   = 0.0;   // kJ/mol
    double      delta_s_298   = 0.0;   // J/(mol·K)
    double      energia_activacion = 0.0; // kJ/mol (para cinética)
    int         orden_reaccion = 1;    // Orden de reacción

    double delta_g_298() const {
        return delta_h_298 - STP_TEMP * delta_s_298 / 1000.0;
    }

    double constante_equilibrio(double T_K) const {
        double dG = delta_h_298 - T_K * delta_s_298 / 1000.0;
        return std::exp(-dG * 1000.0 / (R_GAS * T_K));
    }

    double delta_g_T(double T_K) const {
        return delta_h_298 - T_K * delta_s_298 / 1000.0;
    }

    // Constante cinética (Arrhenius): k = A * exp(-Ea/RT)
    double constante_cinetica(double A_factor, double T_K) const {
        return A_factor * std::exp(-energia_activacion * 1000.0 / (R_GAS * T_K));
    }
};

// ── Partículas y Procesamiento Mineral ──────────────────────────────────────
struct MineralParticle {
    std::string mineral;
    double diameter_um = 0.0;
    double mass_kg_h   = 0.0;
};

struct ParticleStream {
    std::vector<MineralParticle> particles;
    double water_kg_h = 0.0;
};

// ── Flujo (Stream) ──────────────────────────────────────────────────────────
struct Flujo {
    int64_t     id     = 0;
    std::string nombre;

    // Flujo molar total por especie (kmol/s)
    std::map<int64_t, double> flujo_molar;

    // Fracciones por fase (0..1) y composición por fase
    std::map<int64_t, double> flujo_solido;    // especie_id -> kmol/s en sólido
    std::map<int64_t, double> flujo_liquido;   // especie_id -> kmol/s en líquido
    std::map<int64_t, double> flujo_gas;       // especie_id -> kmol/s en gas
    double fraccion_solido = 0.0;
    double fraccion_liquido = 0.0;
    double fraccion_gas = 0.0;

    // Corriente particulada (procesamiento mineral)
    std::vector<MineralParticle> particulas;
    double agua_kg_h = 0.0;

    ParticleStream a_corriente_particulas() const {
        return {particulas, agua_kg_h};
    }
    void desde_corriente_particulas(const ParticleStream& stream) {
        particulas = stream.particles;
        agua_kg_h = stream.water_kg_h;
    }

    // Propiedades térmicas
    double temperatura = STP_TEMP;    // K
    double presion     = STP_PRES;    // atm
    double entalpia    = 0.0;         // kJ/s (kW) — entalpía total del flujo
    double energia     = 0.0;         // kW — solo para flujos de energía

    Fase fase = Fase::Desconocido;
    bool es_energia = false;          // true si es EnergyStream
    int  pagina = 0;
    int64_t nodo_origen  = -1;        // ID de unidad origen (-1 si es corriente de alimentación libre)
    int64_t nodo_destino = -1;        // ID de unidad destino (-1 si es corriente de salida libre)

    // LCA Mappings
    std::string lca_equivalent = "No Mapping";
    std::string lca_group = "Not defined";
    // Mapeo LCA por especie (etiqueta de especie -> id de equivalente)
    std::map<std::string, std::string> lca_especies;

    // Mass Balance (Reconciliation) Data
    double measured_flow = -1.0;      // -1 means not initialized yet; t/h de sólidos secos
    double sd = -1.0;                 // desviación estándar absoluta; t/h
    double measured_water = -1.0;     // t/h de líquido
    double sd_water = -1.0;           // desviación estándar absoluta; t/h
    bool mb_selected = true;
    bool mb_assayed = true;
    std::map<std::string, double> mb_assays; // elemento -> fracción másica seca (0..1)

    double flujo_molar_total() const {
        double s = 0;
        for (auto& [_, n] : flujo_molar) s += n;
        return s;
    }

    double masa_particulas_kg_h() const {
        double s = agua_kg_h;
        for (const auto& p : particulas) s += p.mass_kg_h;
        return s;
    }

    double flujo_total() const {
        double s = 0;
        for (auto& [_, n] : flujo_molar) s += n;
        for (const auto& p : particulas) s += p.mass_kg_h;
        s += agua_kg_h;
        return s;
    }

    std::map<int64_t, double> fracciones_molares() const {
        double tot = flujo_molar_total();
        std::map<int64_t, double> x;
        if (tot > 0) for (auto& [sid, n] : flujo_molar) x[sid] = n / tot;
        return x;
    }

    double flujo_especie(int64_t species_id) const {
        auto it = flujo_molar.find(species_id);
        return it != flujo_molar.end() ? it->second : 0.0;
    }

    bool tiene_especie(int64_t species_id) const {
        return flujo_molar.count(species_id) > 0;
    }

    // Calcular entalpía total usando Cp polinomial de cada especie (kJ/s)
    double calcular_entalpia(const std::map<int64_t, nucleo::Especie>& db) const {
        double H = 0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end()) {
                H += n * it->second.entalpia_abs(temperatura);
            } else {
                H += n * 0.0291 * (temperatura - STP_TEMP);
            }
        }
        // Entalpía sensible de fase particulada + agua (agua_kg_h y particulas en kg/h)
        if (agua_kg_h > 0.0) {
            double m_w_kg_s = agua_kg_h / 3600.0;
            // H2O(l): Hf = -15879 kJ/kg, Cp = 4.184 kJ/(kg·K)
            H += m_w_kg_s * (-15879.0 + 4.184 * (temperatura - STP_TEMP));
        }
        for (const auto& p : particulas) {
            double m_p_kg_s = p.mass_kg_h / 3600.0;
            // Sólido mineral genérico Cp ~ 0.85 kJ/(kg·K)
            H += m_p_kg_s * (0.85 * (temperatura - STP_TEMP));
        }
        return H;
    }

    double calcular_entalpia(const std::map<int64_t, nucleo::Especie>& db) {
        entalpia = static_cast<const Flujo*>(this)->calcular_entalpia(db);
        return entalpia;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double calcular_entalpia(const Repo& repo) const {
        return calcular_entalpia(repo.mapa_especies());
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double calcular_entalpia(const Repo& repo) {
        entalpia = calcular_entalpia(repo.mapa_especies());
        return entalpia;
    }

    // Calcular Cp del flujo a temperatura actual (kJ/(K·s) = kW/K)
    double cp_flujo(const std::map<int64_t, nucleo::Especie>& db) const {
        double cp = 0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end()) {
                cp += n * it->second.cp(temperatura);
            } else {
                cp += n * 29.1;
            }
        }
        if (agua_kg_h > 0.0) cp += (agua_kg_h / 3600.0) * 4.184;
        for (const auto& p : particulas) cp += (p.mass_kg_h / 3600.0) * 0.85;
        return cp;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double cp_flujo(const Repo& repo) const {
        return cp_flujo(repo.mapa_especies());
    }

    double flujo_masico_total(const std::map<int64_t, nucleo::Especie>& db) const {
        return masa_total(db);
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double flujo_masico_total(const Repo& repo) const {
        return masa_total(repo.mapa_especies());
    }

    // Capacidad calorífica específica del flujo [kJ/(kg·K)] (Fig. 24: Cp)
    double cp_especifico_kJ_kgK(const std::map<int64_t, nucleo::Especie>& db) const {
        double m_kg_s = masa_total(db);
        if (m_kg_s <= 1e-12) return 0.0;
        return cp_flujo(db) / m_kg_s;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double cp_especifico_kJ_kgK(const Repo& repo) const {
        return cp_especifico_kJ_kgK(repo.mapa_especies());
    }

    // Heat Content Flow Hc [kW] = entalpía sensible sobre 298.15 K (Fig. 24: Hc)
    double calor_contenido_Hc_kW(const std::map<int64_t, nucleo::Especie>& db) const {
        double Hc = 0.0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end()) Hc += n * it->second.entalpia_ref(temperatura);
            else Hc += n * 0.0291 * (temperatura - STP_TEMP);
        }
        if (agua_kg_h > 0.0) Hc += (agua_kg_h / 3600.0) * 4.184 * (temperatura - STP_TEMP);
        for (const auto& p : particulas) Hc += (p.mass_kg_h / 3600.0) * 0.85 * (temperatura - STP_TEMP);
        return Hc;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double calor_contenido_Hc_kW(const Repo& repo) const {
        return calor_contenido_Hc_kW(repo.mapa_especies());
    }

    // Exergía total del flujo Ex [kW] = exergía física + exergía química estándar (Fig. 24 & 25: Ex)
    double exergia_flujo_kW(const std::map<int64_t, nucleo::Especie>& db) const {
        double Ex = 0.0;
        const double T0 = STP_TEMP;
        const double ln_ratio = (temperatura > 1.0) ? std::log(temperatura / T0) : 0.0;
        for (auto& [sid, n] : flujo_molar) {
            if (n <= 0.0) continue;
            auto it = db.find(sid);
            if (it != db.end()) {
                double dH = it->second.entalpia_ref(temperatura);
                double dS = it->second.entropia_ref(temperatura) / 1000.0;
                // Exergía química de referencia (H2O(l) ~ 900 kJ/kmol -> 49.96 kJ/kg; gases/óxidos según DGf)
                double ex_ch = 900.0;
                if (it->second.formula.find("H2O") == std::string::npos) {
                    ex_ch = std::max(900.0, std::abs(it->second.h_formacion) * 0.12);
                }
                Ex += n * ((dH - T0 * dS) + ex_ch);
            }
        }
        if (agua_kg_h > 0.0) {
            double m_w = agua_kg_h / 3600.0;
            Ex += m_w * (4.184 * ((temperatura - T0) - T0 * ln_ratio) + 49.96);
        }
        for (const auto& p : particulas) {
            double m_p = p.mass_kg_h / 3600.0;
            Ex += m_p * (0.85 * ((temperatura - T0) - T0 * ln_ratio) + 150.0);
        }
        return Ex;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double exergia_flujo_kW(const Repo& repo) const {
        return exergia_flujo_kW(repo.mapa_especies());
    }

    // Desglose másico por fases en t/h (Fig. 9, Fig. 24, Fig. 25)
    double masa_gas_t_h(const std::map<int64_t, nucleo::Especie>& db) const {
        double m = 0.0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end() && it->second.fase_std == Fase::Gas)
                m += n * it->second.masa_molar * 3.6;
        }
        return m;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double masa_gas_t_h(const Repo& repo) const {
        return masa_gas_t_h(repo.mapa_especies());
    }

    double masa_liquido_t_h(const std::map<int64_t, nucleo::Especie>& db) const {
        double m = agua_kg_h / 1000.0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end() && it->second.fase_std == Fase::Liquido)
                m += n * it->second.masa_molar * 3.6;
        }
        return m;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double masa_liquido_t_h(const Repo& repo) const {
        return masa_liquido_t_h(repo.mapa_especies());
    }

    double masa_solidos_t_h(const std::map<int64_t, nucleo::Especie>& db) const {
        double m = 0.0;
        for (const auto& p : particulas) m += p.mass_kg_h / 1000.0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end() && it->second.fase_std == Fase::Solido)
                m += n * it->second.masa_molar * 3.6;
        }
        return m;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double masa_solidos_t_h(const Repo& repo) const {
        return masa_solidos_t_h(repo.mapa_especies());
    }

    // Flujo volumétrico de gas en condiciones normales [Nm³/h a 0 °C, 1 atm] (Fig. 24 & 25)
    double volumen_gas_Nm3_h(const std::map<int64_t, nucleo::Especie>& db) const {
        double n_gas = 0.0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end() && it->second.fase_std == Fase::Gas) n_gas += n;
        }
        return n_gas * 3600.0 * 22.414;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double volumen_gas_Nm3_h(const Repo& repo) const {
        return volumen_gas_Nm3_h(repo.mapa_especies());
    }

    // Flujo volumétrico total [m³/h] (Fig. 9 & Fig. 24)
    double volumen_total_m3_h(const std::map<int64_t, nucleo::Especie>& db) const {
        double n_gas = 0.0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end() && it->second.fase_std == Fase::Gas) n_gas += n;
        }
        double v_gas = (n_gas > 0.0 && presion > 1e-6) ? (n_gas * 3600.0 * R_GAS_ATM * temperatura / presion) : 0.0;
        double v_liq = masa_liquido_t_h(db) / 1.00;
        double v_sol = masa_solidos_t_h(db) / 2.85;
        return v_gas + v_liq + v_sol;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double volumen_total_m3_h(const Repo& repo) const {
        return volumen_total_m3_h(repo.mapa_especies());
    }

    double porcentaje_solidos_wt(const std::map<int64_t, nucleo::Especie>& db) const {
        double ms = masa_solidos_t_h(db);
        double ml = masa_liquido_t_h(db);
        return (ms + ml > 1e-9) ? (ms / (ms + ml)) * 100.0 : 0.0;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double porcentaje_solidos_wt(const Repo& repo) const {
        return porcentaje_solidos_wt(repo.mapa_especies());
    }

    double densidad_pulpa_g_cm3(const std::map<int64_t, nucleo::Especie>& db) const {
        double ms = masa_solidos_t_h(db);
        double ml = masa_liquido_t_h(db);
        double vol = ms / 2.85 + ml / 1.00;
        return (vol > 1e-9) ? ((ms + ml) / vol) : 1.00;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double densidad_pulpa_g_cm3(const Repo& repo) const {
        return densidad_pulpa_g_cm3(repo.mapa_especies());
    }

    double p80_um() const {
        if (particulas.empty()) return 0.0;
        double m_tot = 0.0, d_sum = 0.0;
        for (const auto& p : particulas) {
            m_tot += p.mass_kg_h;
            d_sum += p.mass_kg_h * p.diameter_um;
        }
        return (m_tot > 1e-12) ? (d_sum / m_tot) : 0.0;
    }

    // Calcular entropía total del flujo (kJ/K·s)
    double entropia_total(const std::map<int64_t, nucleo::Especie>& db) const {
        double S = 0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end())
                S += n * it->second.entropia_abs(temperatura);
        }
        return S / 1000.0;
    }

    // Calcular energía de Gibbs total del flujo (kJ/s)
    double gibbs_total(const std::map<int64_t, nucleo::Especie>& db) const {
        double G = 0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end())
                G += n * it->second.energia_gibbs(temperatura);
        }
        return G;
    }

    // Masa total del flujo (kg/s): n [kmol/s] * M [g/mol = kg/kmol] = kg/s
    double masa_total(const std::map<int64_t, nucleo::Especie>& db) const {
        double m = 0;
        for (auto& [sid, n] : flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end())
                m += n * it->second.masa_molar;
        }
        for (const auto& p : particulas) {
            m += p.mass_kg_h / 3600.0;
        }
        m += agua_kg_h / 3600.0;
        return m;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double masa_total(const Repo& repo) const {
        return masa_total(repo.mapa_especies());
    }

    // Masa total del flujo en toneladas por hora (t/h), estándar HSC Sim
    double masa_total_t_h(const std::map<int64_t, nucleo::Especie>& db) const {
        return masa_total(db) * 3.6;
    }

    template <typename Repo, typename = decltype(std::declval<const Repo&>().mapa_especies())>
    double masa_total_t_h(const Repo& repo) const {
        return masa_total(repo.mapa_especies()) * 3.6;
    }
};

// ── Tipos de nodo (operaciones unitarias) ───────────────────────────────────
enum class TipoNodo : uint16_t {
    // Lógicos
    Fuente, Sumidero, Mezclador, Divisor, Válvula,
    Multiplexor, Demultiplexor,

    // Reactores
    Reactor, ReactorGibbs, ReactorEsteq, ReactorCinetica,

    // Conminución
    TrituradoraMandibula, TrituradoraGiratoria, TrituradoraCono,
    MolinoSAG, MolinoBolas, MolinoBarras,
    Zaranda, Hidrociclon,

    // Hidrometalurgia
    TanqueLixiviacion, SXMixerSettler, CeldaElectroobtencion,

    // Pirometalurgia
    HornoRotatorio, HornoFlash, HornoArco, HornoReverbero,
    Tostador, Convertidor,

    // Separación S/L/G
    Espesador, Filtro, LavadorGases, CeldaFlotacion,

    // Térmico/Fluidos
    IntercambiadorCalor, Calentador, Enfriador, Bomba, Compresor, Turbina,
    ColumnaDestilacion, Separador, Absorbedor, Extraccion,

    // Instrumentación
    Sensor, ControladorPID,

    // Anotación
    CajaTexto, Rectangulo, Elipse,

    // Energía
    FlujoEnergia
};

constexpr const char* nombre_tipo_nodo(TipoNodo t) {
    switch (t) {
        case TipoNodo::Fuente:               return "Fuente";
        case TipoNodo::Sumidero:             return "Sumidero";
        case TipoNodo::Mezclador:            return "Mezclador";
        case TipoNodo::Divisor:              return "Divisor";
        case TipoNodo::Válvula:              return "Válvula";
        case TipoNodo::Multiplexor:          return "Multiplexor";
        case TipoNodo::Demultiplexor:        return "Demultiplexor";
        case TipoNodo::Reactor:              return "Reactor";
        case TipoNodo::ReactorGibbs:         return "Reactor Gibbs";
        case TipoNodo::ReactorEsteq:         return "Reactor Esteq";
        case TipoNodo::ReactorCinetica:      return "Reactor Cinético";
        case TipoNodo::TrituradoraMandibula: return "Trituradora Mandíbula";
        case TipoNodo::TrituradoraGiratoria: return "Trituradora Giratoria";
        case TipoNodo::TrituradoraCono:      return "Trituradora Cono";
        case TipoNodo::MolinoSAG:            return "Molino SAG";
        case TipoNodo::MolinoBolas:          return "Molino Bolas";
        case TipoNodo::MolinoBarras:         return "Molino Barras";
        case TipoNodo::Zaranda:              return "Zaranda";
        case TipoNodo::Hidrociclon:          return "Hidrociclón";
        case TipoNodo::TanqueLixiviacion:    return "Tanque Lixiviación";
        case TipoNodo::SXMixerSettler:       return "SX Mixer-Settler";
        case TipoNodo::CeldaElectroobtencion: return "Celda Electroobtención";
        case TipoNodo::HornoRotatorio:       return "Horno Rotatorio";
        case TipoNodo::HornoFlash:           return "Horno Flash";
        case TipoNodo::HornoArco:            return "Horno Arco Eléctrico";
        case TipoNodo::HornoReverbero:       return "Horno Reverbero";
        case TipoNodo::Tostador:             return "Tostador";
        case TipoNodo::Convertidor:          return "Convertidor";
        case TipoNodo::Espesador:            return "Espesador";
        case TipoNodo::Filtro:               return "Filtro";
        case TipoNodo::LavadorGases:         return "Lavador Gases";
        case TipoNodo::CeldaFlotacion:       return "Celda Flotación";
        case TipoNodo::IntercambiadorCalor:  return "Interc. Calor";
        case TipoNodo::Calentador:           return "Calentador";
        case TipoNodo::Enfriador:            return "Enfriador";
        case TipoNodo::Bomba:                return "Bomba";
        case TipoNodo::Turbina:              return "Turbina";
        case TipoNodo::Compresor:            return "Compresor";
        case TipoNodo::ColumnaDestilacion:   return "Col. Destilación";
        case TipoNodo::Separador:            return "Separador";
        case TipoNodo::Absorbedor:           return "Absorbedor";
        case TipoNodo::Extraccion:           return "Extracción";
        case TipoNodo::Sensor:               return "Sensor";
        case TipoNodo::ControladorPID:       return "Controlador PID";
        case TipoNodo::CajaTexto:            return "Texto";
        case TipoNodo::Rectangulo:           return "Rectángulo";
        case TipoNodo::Elipse:               return "Elipse";
        case TipoNodo::FlujoEnergia:         return "Flujo Energía";
    }
    return "Desconocido";
}

// ── Reglas de conectividad por operación unitaria ──────────────────────────
// -1 = sin límite superior.
struct RangoPuertos {
    int min_entradas = 0, max_entradas = -1;
    int min_salidas  = 0, max_salidas  = -1;
};

inline RangoPuertos rango_puertos(TipoNodo t) {
    switch (t) {
        case TipoNodo::Fuente:        return {0, 0, 1, 1};
        case TipoNodo::Sumidero:      return {1, 1, 0, 0};
        case TipoNodo::Mezclador:     return {2, -1, 1, 1};
        case TipoNodo::Divisor:       return {1, 1, 2, -1};
        case TipoNodo::Multiplexor:   return {2, -1, 1, 1};
        case TipoNodo::Demultiplexor: return {1, 1, 2, -1};
        case TipoNodo::Válvula:       return {1, 1, 1, 1};

        case TipoNodo::Reactor:
        case TipoNodo::ReactorGibbs:
        case TipoNodo::ReactorEsteq:
        case TipoNodo::ReactorCinetica:
            return {1, -1, 1, 1};

        case TipoNodo::TrituradoraMandibula:
        case TipoNodo::TrituradoraGiratoria:
        case TipoNodo::TrituradoraCono:
        case TipoNodo::MolinoSAG:
        case TipoNodo::MolinoBolas:
        case TipoNodo::MolinoBarras:
        case TipoNodo::Calentador:
        case TipoNodo::Enfriador:
        case TipoNodo::Bomba:
        case TipoNodo::Turbina:
        case TipoNodo::Compresor:
        case TipoNodo::IntercambiadorCalor:
        case TipoNodo::Sensor:
        case TipoNodo::ControladorPID:
        case TipoNodo::FlujoEnergia:
            return {1, 1, 1, 1};

        case TipoNodo::Zaranda:
        case TipoNodo::Hidrociclon:
        case TipoNodo::CeldaFlotacion:
        case TipoNodo::ColumnaDestilacion:
        case TipoNodo::Separador:
            return {1, 1, 2, 2};

        case TipoNodo::Espesador:
        case TipoNodo::Filtro:
        case TipoNodo::CeldaElectroobtencion:
        case TipoNodo::Absorbedor:
            return {1, 2, 2, 2};

        case TipoNodo::TanqueLixiviacion:
            return {2, -1, 2, 2};

        case TipoNodo::SXMixerSettler:
        case TipoNodo::Extraccion:
        case TipoNodo::LavadorGases:
            return {2, 2, 2, 2};

        case TipoNodo::HornoRotatorio:
        case TipoNodo::HornoFlash:
        case TipoNodo::HornoArco:
        case TipoNodo::HornoReverbero:
        case TipoNodo::Tostador:
        case TipoNodo::Convertidor:
            return {1, -1, 2, -1};

        case TipoNodo::CajaTexto:
        case TipoNodo::Rectangulo:
        case TipoNodo::Elipse:
            return {0, 0, 0, 0};
    }
    return {0, -1, 0, -1}; // genérico: sin restricciones
}

struct Nodo {
    int64_t     id    = 0;
    std::string nombre;
    TipoNodo    tipo  = TipoNodo::Mezclador;
    float       canvas_x = 0.0f, canvas_y = 0.0f;
    int         pagina = 0;

    std::vector<int64_t> ids_flujos_entrada;
    std::vector<int64_t> ids_flujos_salida;

    // Parámetros genéricos
    std::map<std::string, double> params;

    double conversion() const {
        auto it = params.find("conversion");
        return it != params.end() ? it->second : 1.0;
    }

    std::vector<double> fracciones_divisor() const {
        std::vector<double> f;
        for (size_t i = 0; ; ++i) {
            auto it = params.find("div_" + std::to_string(i));
            if (it == params.end()) break;
            f.push_back(it->second);
        }
        return f;
    }

    double temperatura_salida() const {
        auto it = params.find("temperatura_salida");
        if (it == params.end()) it = params.find("T_salida");
        return it != params.end() ? it->second : STP_TEMP;
    }

    double calor_intercambiado() const {
        auto it = params.find("calor");
        return it != params.end() ? it->second : 0.0;
    }

    double presion_salida() const {
        auto it = params.find("P_salida");
        return it != params.end() ? it->second : STP_PRES;
    }

    double relacion_reflujo() const {
        auto it = params.find("reflujo");
        if (it == params.end()) it = params.find("R_reflujo");
        return it != params.end() ? it->second : 1.5;
    }

    int num_etapas() const {
        auto it = params.find("etapas");
        return it != params.end() ? static_cast<int>(it->second) : 10;
    }

    int num_entradas()  const { return static_cast<int>(ids_flujos_entrada.size()); }
    int num_salidas()   const { return static_cast<int>(ids_flujos_salida.size()); }
};

// ── Manejo de errores ───────────────────────────────────────────────────────
struct ErrorCMA : public std::runtime_error {
    enum class Tipo {
        Ninguno, BaseDatos, BalanceMasa, BalanceEnergia,
        Gibbs, Calculo, EntradaInvalida, Convergencia
    };
    Tipo tipo = Tipo::Ninguno;

    ErrorCMA(const std::string& msg, Tipo t = Tipo::Calculo)
        : std::runtime_error(msg), tipo(t) {}
};

}} // namespace
