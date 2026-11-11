#pragma once

#include "domain/StabilityDiagramTypes.hpp"
#include "domain/ports/ThermoRepository.hpp"

#include <optional>
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

    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    const domain::ThermoRepository* repositorio() const { return repo_; }

    // ── API clásica compatible con pruebas de regresión existentes ──────────
    std::vector<PuntoEstabilidad> generar(const std::string& metal, double T = 900.0) const;
    std::vector<PuntoEstabilidad> generar(const std::string& metal, double T,
                                          double log_pO2_min, double log_pO2_max, int steps_O2,
                                          double log_pSO2_min, double log_pSO2_max, int steps_SO2) const;

    // ── API Hexagonal HSC Phase Stability Diagrams (Tpp & Lpp) ──────────────

    // Analiza una selección de 3 elementos: identifica el elemento principal (metal/condensado),
    // los elementos auxiliares, la lista de especies condensadas candidatas y los gases de eje válidos.
    domain::SistemaTernarioEstabilidad analizar_sistema_ternario(
        const std::vector<std::string>& elementos,
        bool incluir_liquidos = true,
        bool incluir_organicos = false,
        const std::string& forzar_elemento_principal = "",
        bool incluir_secundarias_sin_principal = false) const;

    // Calcula un diagrama vectorial isotérmico Lpp (Kellogg): polígonos exactos por intersección
    // de semiplanos, ecuaciones de reacción de frontera, pendientes, log10(K) y puntos triples.
    domain::ResultadoDiagramaLpp calcular_lpp(const domain::ParametrosCalculoLpp& params) const;

    // Calcula un diagrama de predominancia Tpp (tanto en modo isotérmico p-p como en modo T-p con
    // temperatura en el eje X y presión parcial en el eje Y), usando subdivisión recursiva quadtree
    // y envolventes exactas de energía libre de Gibbs mínima.
    domain::ResultadoDiagramaTpp calcular_tpp(const domain::ParametrosCalculoTpp& params) const;

    // Busca los datos termodinámicos completos de una especie por su fórmula (para doble clic en lista).
    std::optional<domain::EspecieCandidataEstabilidad> buscar_info_especie(
        const std::string& formula) const;

    // Evalúa la fase predominante y su energía de Gibbs normalizada en cualquier coordenada (x, y).
    std::string evaluar_punto_cursor_tpp(const domain::ParametrosCalculoTpp& params,
                                         double x, double y,
                                         double* out_g_norm_kJ = nullptr) const;

    // Exporta las reacciones de equilibrio, fronteras y puntos triples de un diagrama Lpp a CSV/texto.
    std::string exportar_tabla_lpp_csv(const domain::ResultadoDiagramaLpp& res) const;

    // Persistencia de especificaciones de sistema (*.tpp8 / *.tpp y *.lpp8 / *.lpp)
    std::string serializar_tpp(const domain::ParametrosCalculoTpp& params) const;
    bool deserializar_tpp(const std::string& contenido, domain::ParametrosCalculoTpp& out) const;

    std::string serializar_lpp(const domain::ParametrosCalculoLpp& params) const;
    bool deserializar_lpp(const std::string& contenido, domain::ParametrosCalculoLpp& out) const;

  private:
    const domain::ThermoRepository* repo_ = nullptr;
};

} // namespace cma::application::modules
