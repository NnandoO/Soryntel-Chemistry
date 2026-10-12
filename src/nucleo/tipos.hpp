#pragma once
#include <string>
#include <map>
#include <vector>
#include <cmath>
#include <cstdint>
#include <stdexcept>
#include <sstream>

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

// ── Especie química ─────────────────────────────────────────────────────────
struct Especie {
    int64_t     id        = 0;
    std::string nombre;
    std::string formula;
    double      masa_molar   = 0.0;   // g/mol
    double      h_formacion  = 0.0;   // kJ/mol a 298.15 K
    double      s_formacion  = 0.0;   // J/(mol·K) a 298.15 K
    double      cp_a = 0.0, cp_b = 0.0, cp_c = 0.0, cp_d = 0.0; // Cp [J/(mol·K)]

    // Coeficientes Antoine: log10(P_sat) = A - B/(C + T) [P en mmHg, T en °C]
    double      antoine_a = 0.0, antoine_b = 0.0, antoine_c = 0.0;
    double      temp_min_antoine = 0.0, temp_max_antoine = 0.0; // °C

    // Estado estándar
    Fase        fase_std = Fase::Gas;

    double cp(double T_K) const {
        return cp_a + cp_b*T_K + cp_c*T_K*T_K + cp_d*T_K*T_K*T_K;
    }

    double entalpia_ref(double T_K) const {
        double Tr = STP_TEMP;
        return (cp_a*(T_K - Tr)
              + cp_b*(T_K*T_K - Tr*Tr)/2.0
              + cp_c*(T_K*T_K*T_K - Tr*Tr*Tr)/3.0
              + cp_d*(T_K*T_K*T_K*T_K - Tr*Tr*Tr*Tr)/4.0) / 1000.0;
    }

    double entalpia_abs(double T_K) const {
        return h_formacion + entalpia_ref(T_K);
    }

    double entropia_ref(double T_K) const {
        double Tr = STP_TEMP;
        if (T_K <= 0) return 0;
        return cp_a * std::log(T_K/Tr)
             + cp_b * (T_K - Tr)
             + cp_c * (T_K*T_K - Tr*Tr)/2.0
             + cp_d * (T_K*T_K*T_K - Tr*Tr*Tr)/3.0;
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

    // Propiedades térmicas
    double temperatura = STP_TEMP;    // K
    double presion     = STP_PRES;    // atm
    double entalpia    = 0.0;         // kJ/s (kW) — entalpía total del flujo
    double energia     = 0.0;         // kW — solo para flujos de energía

    Fase fase = Fase::Desconocido;
    bool es_energia = false;          // true si es EnergyStream
    int  pagina = 0;

    // LCA Mappings
    std::string lca_equivalent = "No Mapping";
    std::string lca_group = "Not defined";

    // Mass Balance (Reconciliation) Data
    double measured_flow = -1.0;      // -1 means not initialized yet
    double sd = -1.0;
    bool mb_selected = true;
    bool mb_assayed = true;

    double flujo_total() const {
        double s = 0;
        for (auto& [_, n] : flujo_molar) s += n;
        return s;
    }

    std::map<int64_t, double> fracciones_molares() const {
        double tot = flujo_total();
        std::map<int64_t, double> x;
        if (tot > 0) for (auto& [id, n] : flujo_molar) x[id] = n / tot;
        return x;
    }

    double flujo_especie(int64_t id) const {
        auto it = flujo_molar.find(id);
        return it != flujo_molar.end() ? it->second : 0.0;
    }

    bool tiene_especie(int64_t id) const {
        return flujo_molar.count(id) > 0;
    }

    // Calcular entalpía total usando Cp polinomial de cada especie (kJ/s)
    double calcular_entalpia(const std::map<int64_t, nucleo::Especie>& db) {
        double H = 0;
        for (auto& [id, n] : flujo_molar) {
            auto it = db.find(id);
            if (it != db.end()) {
                H += n * it->second.entalpia_abs(temperatura);
            } else {
                H += n * 0.0291 * (temperatura - STP_TEMP);
            }
        }
        return H;
    }

    // Calcular Cp del flujo a temperatura actual (kJ/(K·s) = kW/K)
    double cp_flujo(const std::map<int64_t, nucleo::Especie>& db) const {
        double cp = 0;
        for (auto& [id, n] : flujo_molar) {
            auto it = db.find(id);
            if (it != db.end()) {
                cp += n * it->second.cp(temperatura);
            } else {
                cp += n * 29.1;
            }
        }
        return cp;
    }

    // Calcular entropía total del flujo (kJ/K·s)
    double entropia_total(const std::map<int64_t, nucleo::Especie>& db) const {
        double S = 0;
        for (auto& [id, n] : flujo_molar) {
            auto it = db.find(id);
            if (it != db.end())
                S += n * it->second.entropia_abs(temperatura);
        }
        return S / 1000.0;
    }

    // Calcular energía de Gibbs total del flujo (kJ/s)
    double gibbs_total(const std::map<int64_t, nucleo::Especie>& db) const {
        double G = 0;
        for (auto& [id, n] : flujo_molar) {
            auto it = db.find(id);
            if (it != db.end())
                G += n * it->second.energia_gibbs(temperatura);
        }
        return G;
    }

    // Masa total del flujo (kg/s)
    double masa_total(const std::map<int64_t, nucleo::Especie>& db) const {
        double m = 0;
        for (auto& [id, n] : flujo_molar) {
            auto it = db.find(id);
            if (it != db.end())
                m += n * it->second.masa_molar;
        }
        return m / 1000.0;
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
        return it != params.end() ? (int)it->second : 10;
    }

    int num_entradas()  const { return (int)ids_flujos_entrada.size(); }
    int num_salidas()   const { return (int)ids_flujos_salida.size(); }
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
