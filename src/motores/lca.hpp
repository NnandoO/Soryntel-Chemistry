#pragma once
#include <string>
#include <utility>
#include <vector>

namespace cma { namespace motores {

// ── API previa (factores de emisión agregados) ──────────────────────────────
struct ImpactoLCA {
    double emision_CO2 = 0.0;      // kg CO2
    double emision_CH4 = 0.0;      // kg CH4
    double emision_N2O = 0.0;      // kg N2O
    double huella_carbono = 0.0;   // kg CO2-eq (GWP100 AR6)
    double consumo_energia = 0.0;  // MJ (energía primaria)
    double consumo_agua = 0.0;     // m³
    double residuos_solidos = 0.0; // kg
    std::string mensaje;
    bool ok = false;
};

// ── Catálogo de equivalentes LCA (mapeo tipo GaBi) ──────────────────────────
// Cada entrada documenta sus factores de caracterización por unidad de flujo.
struct EquivalenteLCA {
    std::string id;       // identificador estable (persistencia y exportación)
    std::string nombre;   // etiqueta legible
    std::string grupo;    // categoría GaBi: Emissions to air, Resources, ...
    std::string unidad;   // unidad del flujo (kg, kWh, ...)
    double gwp = 0.0;     // kg CO2-eq / unidad   (GWP100, IPCC AR6)
    double ap = 0.0;      // kg SO2-eq / unidad   (acidificación, CML 2016)
    double ep = 0.0;      // kg PO4-eq / unidad   (eutrofización, CML 2016)
    double agua = 0.0;    // m³ / unidad          (uso consuntivo)
    double energia = 0.0; // MJ / unidad          (demanda de energía primaria)
    std::string fuente;   // nota de trazabilidad del factor
};

// ── Fila del dataset de frontera (una especie de una corriente) ─────────────
struct FilaLCA {
    std::string equipo;
    std::string corriente;
    std::string especie;
    double cantidad = 0.0;                    // kg/h
    bool entrada = true;
    std::string equivalente = "no-mapeado";   // id del catálogo
};

// ── Resultado de la evaluación ──────────────────────────────────────────────
struct ResultadoLCA {
    double gwp_kg_co2eq = 0.0;
    double ap_kg_so2eq = 0.0;
    double ep_kg_po4eq = 0.0;
    double agua_m3 = 0.0;
    double energia_mj = 0.0;
    double residuos_kg = 0.0;

    double masa_entrada_kg = 0.0;
    double masa_salida_kg = 0.0;
    double masa_sin_mapear_kg = 0.0;
    int filas_totales = 0;
    int filas_mapeadas = 0;
    int filas_sin_mapear = 0;

    double factor_unidad_funcional = 1.0;     // 1 / kg de producto
    bool balance_masa_ok = false;
    bool ok = false;

    std::vector<std::pair<std::string, double>> contribuciones; // nombre → kg CO2-eq
    std::vector<std::string> avisos;
};

// ── Motor LCA ───────────────────────────────────────────────────────────────
class LCA {
public:
    // Catálogo estático de equivalentes y búsqueda por id.
    static const std::vector<EquivalenteLCA>& catalogo();
    static const EquivalenteLCA* buscar_equivalente(const std::string& id);

    // Sugerencia automática por especie/corriente. Devuelve "no-mapeado" si no
    // hay un equivalente razonable. No usa coincidencias de subcadena frágiles.
    static std::string sugerir_equivalente(const std::string& especie,
                                           const std::string& corriente,
                                           bool entrada);

    // Evalúa el dataset de frontera. Los consumos manuales están en base
    // horaria (kWh/h, Nm³/h, kg/h, m³/h). Si unidad_funcional_kg > 0 todos
    // los indicadores y masas quedan referidos a 1 kg de producto.
    ResultadoLCA evaluar(const std::vector<FilaLCA>& filas,
                         double electricidad_kWh_h = 0.0,
                         double gas_natural_Nm3_h = 0.0,
                         double combustible_kg_h = 0.0,
                         double agua_m3_h = 0.0,
                         double residuo_kg_h = 0.0,
                         double unidad_funcional_kg = 0.0) const;

    // ── Compatibilidad con la API anterior ──
    ImpactoLCA evaluar_proceso(double consumo_electricidad_kWh,
                               double consumo_gas_natural_m3,
                               double combustible_liquido_kg,
                               double materia_prima_kg,
                               double producto_kg,
                               double agua_consumida_m3);
    double factor_emision_electricidad(double kWh, const std::string& pais = "global");
    double factor_emision_gas_natural(double m3);
    double factor_emision_combustible(double kg, const std::string& tipo = "diésel");
    double huella_carbono_total(const ImpactoLCA& impacto);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;
};

}} // namespace cma::motores
