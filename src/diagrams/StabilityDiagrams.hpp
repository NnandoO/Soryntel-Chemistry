#pragma once
#include "domain/StabilityDiagramTypes.hpp"
#include "domain/ports/ThermoRepository.hpp"

#include <string>
#include <vector>

namespace cma::application::modules {

struct PuntoEstabilidad {
    double log_pO2;
    double log_pSO2;
    std::string fase;
};

class StabilityDiagrams {
  public:
    explicit StabilityDiagrams(const domain::ThermoRepository* repo = nullptr) : repo_(repo) {}

    void set_repository(const domain::ThermoRepository* repo) { repo_ = repo; }
    const domain::ThermoRepository* repository() const { return repo_; }

    // ── API clásica de predominancia M-O-S (compatibilidad con tests) ───────
    std::vector<PuntoEstabilidad> generar(const std::string& metal, double T = 900.0) const;
    std::vector<PuntoEstabilidad> generar(const std::string& metal, double T, double log_pO2_min,
                                          double log_pO2_max, int steps_O2, double log_pSO2_min,
                                          double log_pSO2_max, int steps_SO2) const;

    // ── Selección y balance estequiométrico 2x2 contra gases arbitrarios ────
    std::vector<domain::StabilitySpeciesSelection>
    filtrar_especies_sistema(const std::string& main_element, const std::string& gas1_formula,
                             const std::string& gas2_formula, bool solo_condensadas = true) const;

    // ── Motor Lpp (Isotérmico Vectorial de Kellogg: Polígonos, Líneas y Puntos Triples) ──
    domain::LppDiagramResult calcular_lpp(const domain::LppDiagramRequest& req) const;

    // ── Motor Tpp (T-p y p-p por Minimización de Energía Libre de Gibbs + Contornos) ──
    domain::TppDiagramResult calcular_tpp(const domain::TppDiagramRequest& req) const;

    // ── Exportación de resultados ───────────────────────────────────────────
    bool exportar_lpp_csv(const std::string& ruta, const domain::LppDiagramResult& res) const;
    bool exportar_tpp_csv(const std::string& ruta, const domain::TppDiagramRequest& req,
                          const domain::TppDiagramResult& res) const;

  private:
    const domain::ThermoRepository* repo_ = nullptr;
};

} // namespace cma::application::modules
