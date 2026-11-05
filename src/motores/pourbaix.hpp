#pragma once
// ══════════════════════════════════════════════════════════════════════════════
//  CalculadorPourbaix — Motor de diagramas Eh-pH (Pourbaix / HSC EpH)
//  Soporta sistemas M-H₂O y sistemas multicomponente M-L-H₂O (S, Cl, C, N),
//  integración térmica ΔG°(T), recorte exacto de fronteras de Nernst,
//  polígonos de predominancia, puntos triples invariantes, clasificación de
//  corrosión (Inmunidad / Pasivación / Corrosión) y escalas SHE / SCE / Ag-AgCl.
// ══════════════════════════════════════════════════════════════════════════════
#include "domain/PourbaixTypes.hpp"
#include "domain/ports/ThermoRepository.hpp"

#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace cma {
namespace motores {

// ── Estructuras clásicas (compatibilidad hacia atrás) ─────────────────────────
struct LineaPourbaix {
    std::string sp_ox;
    std::string sp_red;
    std::string ecuacion;
    double E0 = 0.0;    // Potencial estándar a pH=0 [V vs SHE]
    double dEdpH = 0.0; // Pendiente Nernst [V/pH]
    double pH_min = -2.0;
    double pH_max = 16.0;
    int n_e = 0;
    int n_H = 0;
    bool es_vertical = false;
    double pH_eq = 0.0;
    bool es_horizontal = false;
    std::vector<domain::PourbaixSegment> segmentos_recortados;
};

struct EspeciePourbaix {
    int64_t id = 0;
    std::string formula;
    std::string formula_limpia;
    std::string nombre;
    std::string estado; // "s", "l", "g", "aq"
    double dG0 = 0.0;   // ΔGf°(T) [kJ/mol]
    int carga = 0;
    int n_M = 0;
    int n_H = 0;
    int n_O = 0;
    double molalidad = 1e-6;
    bool activa = true;
    domain::PourbaixCorrosionState estado_corrosion = domain::PourbaixCorrosionState::Corrosion;
};

struct DominanciaPourbaix {
    std::vector<double> pH_grid;
    std::vector<double> Eh_grid;
    // especie_dom[i_Eh][i_pH] = índice en especies_ (-1 si ninguna)
    std::vector<std::vector<int>> especie_dom;
    // corrosion_grid[i_Eh][i_pH] = 0 Inmunidad, 1 Pasivación, 2 Corrosión, 3 Gas
    std::vector<std::vector<int>> corrosion_grid;
};

class CalculadorPourbaix {
  public:
    explicit CalculadorPourbaix(const domain::ThermoRepository* repo = nullptr);

    void set_repository(const domain::ThermoRepository* repo) { repo_ = repo; }

    // ── Configuración clásica y extendida ────────────────────────────────────
    void establecer_elemento(const std::string& elem);
    void establecer_temperatura(double T_K);
    void establecer_presion(double P_bar);
    void establecer_concentracion(double mol_L);
    void establecer_rango_pH(double pH_min, double pH_max);
    void establecer_rango_Eh(double Eh_min, double Eh_max);
    void establecer_resolucion(int n_pH, int n_Eh);
    void establecer_escala_potencial(domain::PourbaixPotentialScale escala);

    // ── Ligandos secundarios (S, Cl, C, N) para sistemas M-L-H₂O ─────────────
    void limpiar_ligandos();
    void agregar_o_actualizar_ligando(const std::string& elem_ligando, double molalidad, bool activo = true);
    const std::vector<domain::PourbaixLigandSpec>& obtener_ligandos() const { return ligandos_; }
    std::vector<domain::PourbaixLigandSpec>& obtener_ligandos_mut() { return ligandos_; }

    // ── Control individual de especies (activación y molalidad) ──────────────
    void cargar_especies_disponibles();
    void establecer_especie_activa(size_t idx, bool activa);
    void establecer_molalidad_especie(size_t idx, double molalidad);

    // ── Cálculo principal ────────────────────────────────────────────────────
    void calcular();

    // ── Consultas de resultados ──────────────────────────────────────────────
    const std::vector<LineaPourbaix>& obtener_lineas() const { return lineas_; }
    const std::vector<EspeciePourbaix>& obtener_especies() const { return especies_; }
    std::vector<EspeciePourbaix>& obtener_especies_mut() { return especies_; }
    const DominanciaPourbaix& obtener_dominancia() const { return dom_; }
    const std::vector<domain::PourbaixPolygon>& obtener_poligonos() const { return poligonos_; }
    const std::vector<domain::PourbaixTriplePoint>& obtener_puntos_triples() const { return puntos_triples_; }
    const std::vector<domain::PourbaixSegment>& obtener_segmentos_frontera() const {
        return segmentos_frontera_;
    }

    // Líneas de estabilidad del agua (en la escala de potencial seleccionada)
    double eh_agua_oxidacion(double pH, double pO2 = 1.0) const;
    double eh_agua_reduccion(double pH, double pH2 = 1.0) const;
    double pendiente_nernst() const;
    double offset_escala_V() const;

    // Sonda puntual en el espacio (pH, Eh_display)
    domain::PourbaixPointEvaluation evaluar_punto(double pH, double Eh_display_V) const;

    // Exportación CSV compatible con HSC Chemistry
    std::string exportar_csv() const;
    bool guardar_csv(const std::string& filepath) const;

    // Presets hidrometalúrgicos y de corrosión de referencia
    void cargar_preset_hsc(int preset_id);

    // Getters de estado
    const std::string& elemento() const { return elemento_; }
    double temperatura_K() const { return T_K_; }
    double presion_bar() const { return P_bar_; }
    double concentracion() const { return conc_; }
    double pH_min() const { return pH_min_; }
    double pH_max() const { return pH_max_; }
    double Eh_min() const { return Eh_min_; }
    double Eh_max() const { return Eh_max_; }
    domain::PourbaixPotentialScale escala_potencial() const { return escala_; }

  private:
    const domain::ThermoRepository* repo_ = nullptr;
    std::string elemento_ = "Fe";
    double T_K_ = 298.15;
    double P_bar_ = 1.0;
    double conc_ = 1e-6;
    double pH_min_ = -1.0;
    double pH_max_ = 15.0;
    double Eh_min_ = -1.8;
    double Eh_max_ = 2.2;
    int n_pH_ = 90;
    int n_Eh_ = 90;
    domain::PourbaixPotentialScale escala_ = domain::PourbaixPotentialScale::SHE;

    std::vector<domain::PourbaixLigandSpec> ligandos_;
    std::vector<EspeciePourbaix> especies_;
    std::vector<std::map<std::string, int>> especies_ligandos_;
    std::vector<LineaPourbaix> lineas_;
    DominanciaPourbaix dom_;
    std::vector<domain::PourbaixPolygon> poligonos_;
    std::vector<domain::PourbaixTriplePoint> puntos_triples_;
    std::vector<domain::PourbaixSegment> segmentos_frontera_;
    bool especies_cargadas_ = false;
    std::string ultimo_hash_sistema_;

    std::string calcular_hash_sistema() const;
    double dG0_agua_liquida(double T_K) const;
    double potencial_ligando(const std::string& elem_L, double molalidad, double pH, double Eh_SHE) const;
    double potencial_transformado_especie(size_t idx, double pH, double Eh_SHE, double dG_H2O) const;
    void construir_lineas_y_recortes(double dG_H2O);
    void construir_malla_y_poligonos(double dG_H2O);
};

} // namespace motores
} // namespace cma
