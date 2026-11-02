#pragma once
#include "application/modules.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "nucleo/tipos.hpp"
#include <map>
#include <string>
#include <vector>

namespace cma::application::modules {

// ── Ficha Mineralógica de Literatura (HSC Geo Mineral Database) ─────────────
struct FichaMineralGeo {
    std::string nombre;              // Ej. "Chalcopyrite (Erora, Portugal)"
    std::string nombre_corto;        // Ej. "Chalcopyrite"
    std::string simbolo_ima;         // Ej. "Ccp"
    std::string formula;             // Ej. "CuFeS2"
    std::string clase_mineral;       // "Sulfides", "Silicates", "Oxides", "Carbonates", "Phosphates"
    std::string sistema_cristalino;  // "Tetragonal", "Isometric", "Orthorhombic", "Monoclinic", "Trigonal", "Hexagonal", "Triclinic"
    std::string localidad;           // Ej. "Erora, Portugal"
    std::string referencia;          // Ej. "Cabri (1973) Econ. Geol. 68"
    double gravedad_especifica = 3.0;// Specific Gravity (g/cm3)
    double dureza_mohs = 4.0;        // Mohs Hardness (1-10)
    std::string color;               // Color macroscópico / óptico
    double masa_molar = 0.0;         // g/mol calculado estequiométricamente
    std::map<std::string, double> elem_wt_pct; // % en peso estequiométrico por elemento

    double porcentaje_elemento(const std::string& elem) const {
        auto it = elem_wt_pct.find(elem);
        return (it != elem_wt_pct.end()) ? it->second : 0.0;
    }
};

// ── Muestra Geoquímica para Cálculos Modales (Bruvann.xls, Bushveld, etc.) ──
struct MuestraGeoModal {
    std::string nombre_fuente;       // Ej. "Bruvann.xls - XRF"
    std::string descripcion;         // Descripción geológica del yacimiento
    std::map<std::string, double> ensayo_elem_wt; // Ensayo elemental medido (wt-%)
    std::vector<std::string> minerales_candidatos;
};

// ── Ronda de Cálculo Modal (Phases and Elements per Rounds) ─────────────────
struct RondaModal {
    std::string nombre;              // "Round 1", "Round 2", ...
    std::vector<std::string> fases;  // Nombres de minerales asignados a esta ronda
    std::vector<std::string> elementos; // Elementos trazadores resueltos en esta ronda
};

enum class MetodoModal {
    LS = 0,    // Least Square (LS)
    WLS = 1,   // Weighed Least Square (WLS)
    NNLS = 2,  // Non-negative Least Square (NNLS)
    WNNLS = 3  // Weighed non-negative Least Square (WNNLS)
};

struct ResultadoFaseModal {
    std::string nombre_mineral;
    std::string formula;
    int ronda_resuelta = 1;
    double wt_pct = 0.0;
    double vol_pct = 0.0;
    double sigma_error = 0.0;
};

struct ComparacionElementoModal {
    std::string elemento;
    double medido_wt = 0.0;
    double calculado_wt = 0.0;
    double residual_wt = 0.0;
    double error_rel_pct = 0.0;
};

struct ResultadoCalculoModal {
    std::vector<ResultadoFaseModal> fases;
    std::vector<ComparacionElementoModal> balance_elemental;
    std::vector<std::string> ecuaciones_rondas;
    double total_mineral_wt = 0.0;
    double no_identificado_wt = 0.0; // Unidentified (USERX)
    double rmse_elemental = 0.0;
    double chi_cuadrado = 0.0;
    int rondas_ejecutadas = 0;
    int fases_no_determinadas = 0;
};

// ── Cálculo de Fórmula Mineral desde Análisis EPMA/XRF ──────────────────────
struct EntradaAnalisisMineral {
    std::string componente; // Símbolo elemental ("Cu", "Fe", "S") u óxido ("SiO2", "MgO", "FeO")
    double wt_pct = 0.0;
};

struct ResultadoFormulaMineral {
    std::string formula_empirica;
    std::string mineral_mas_cercano;
    std::string formula_ideal;
    double similitud_pct = 0.0;
    double suma_analisis_wt = 0.0;
    std::map<std::string, double> apfu; // Átomos por unidad de fórmula (atoms per formula unit)
    std::map<std::string, double> elem_wt_normalizado;
};

// ── Cálculo Petrológico CIPW Norm y Clasificación TAS/QAPF ──────────────────
struct AnalisisRocaOxidos {
    std::string nombre_muestra = "Bushveld Norite (SA-01)";
    double SiO2  = 51.40;
    double TiO2  = 0.45;
    double Al2O3 = 16.80;
    double Fe2O3 = 1.60;
    double FeO   = 7.20;
    double MnO   = 0.15;
    double MgO   = 9.40;
    double CaO   = 10.60;
    double Na2O  = 1.95;
    double K2O   = 0.35;
    double P2O5  = 0.10;
};

struct ResultadoPetrologiaCIPW {
    std::map<std::string, double> norma_wt; // Quartz, Orthoclase, Albite, Anorthite, Diopside, Hypersthene, Olivine, Magnetite, Ilmenite, Apatite, Corundum
    double mg_number = 0.0;                 // Mg# = 100 * Mg / (Mg + Fe2+) molar
    double indice_acnk = 0.0;               // Al2O3 / (CaO + Na2O + K2O) molar
    double indice_diferenciacion = 0.0;     // DI = Q + Or + Ab
    std::string saturacion_alumina;         // Metaluminous / Peraluminous / Peralkaline
    std::string clasificacion_tas;          // Basalt / Gabbro, Andesite / Diorite, etc.
    std::string clasificacion_qapf;         // Gabbro / Norite, Granodiorite, Granite, etc.
};

class MineralDB {
public:
    MineralDB();
    explicit MineralDB(const cma::domain::ThermoRepository& repository);

    // Compatibilidad con la API previa de especies sólidas del repositorio
    std::vector<const cma::nucleo::Especie*> minerales();

    // Catálogo mineralógico HSC Geo
    void cargar_catalogo_literatura();
    const std::vector<FichaMineralGeo>& catalogo() const { return catalogo_; }
    std::vector<FichaMineralGeo>& catalogo() { return catalogo_; }
    const FichaMineralGeo* buscar_por_nombre(const std::string& nombre) const;

    // Búsqueda avanzada (consulta HSC Geo + filtros por elementos presentes/excluidos)
    std::vector<const FichaMineralGeo*> filtrar_minerales(
        const std::string& texto_libre,
        const std::string& clase_filtro,
        const std::vector<std::string>& elem_requeridos,
        const std::vector<std::string>& elem_excluidos,
        double sg_min = 0.0, double sg_max = 25.0,
        double mohs_min = 0.0, double mohs_max = 10.0) const;

    // Evalúa consultas tipo HSC Geo: "[Cu %] Is greater than 10 AND [Specific Gravity] Is between 2.2 and 4.4"
    bool cumple_consulta_hsc(const FichaMineralGeo& m, const std::string& query) const;

    // Muestras predefinidas (Bruvann.xls - XRF, Bushveld UG2, Outokumpu Cu-Co, Kiruna Fe-P)
    const std::vector<MuestraGeoModal>& muestras_predefinidas() const { return muestras_; }

    // Solver de composición modal por rondas (LS, WLS, NNLS, WNNLS)
    ResultadoCalculoModal resolver_modal_rondas(
        const std::map<std::string, double>& ensayo_elem_wt,
        const std::vector<RondaModal>& rondas,
        MetodoModal metodo) const;

    // Cálculo de fórmula empírica de un mineral a partir de su análisis químico
    ResultadoFormulaMineral calcular_formula_mineral(
        const std::vector<EntradaAnalisisMineral>& analisis,
        const std::string& elemento_base_normalizacion,
        double num_atomos_base) const;

    // Cálculo petrológico CIPW Norm para muestras de roca
    ResultadoPetrologiaCIPW calcular_norma_cipw(const AnalisisRocaOxidos& oxidos) const;

    // Calcula el desglose elemental estequiométrico (wt-%) de cualquier fórmula
    static std::map<std::string, double> calcular_wt_elemental(const std::string& formula, double* out_mw = nullptr);

private:
    const cma::domain::ThermoRepository* motor_ = nullptr;
    std::vector<FichaMineralGeo> catalogo_;
    std::vector<MuestraGeoModal> muestras_;
};

} // namespace cma::application::modules
