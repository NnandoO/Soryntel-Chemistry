#pragma once

#include "domain/PourbaixTypes.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "../nucleo/tipos.hpp"

#include <map>
#include <string>
#include <vector>

namespace cma { namespace motores {

struct EspeciePourbaix {
    std::string formula;
    std::string nombre;
    double dG_f = 0.0;         // ΔGf° kJ/mol (corregido por concentración si es acuoso)
    int carga = 0;
    int coef_elemento = 1;     // átomos del elemento principal
    int estado = 0;            // 0: gas, 1: líquido, 2: sólido, 3: acuoso
    double n_H = 0.0;          // átomos de H por 1 átomo del elemento principal
    double n_O = 0.0;          // átomos de O por 1 átomo del elemento principal
};

struct LineaPourbaix {
    std::string especie_a;
    std::string especie_b;
    double m = 0.0;            // pendiente (dEh/dpH)
    double b = 0.0;            // intercepto (Eh a pH=0)
    bool vertical = false;     // true para reacciones ácido-base puras
    double x_val = 0.0;        // valor de pH si es vertical
    bool es_agua = false;      // línea de estabilidad del agua
    double x1 = 0.0, y1 = 0.0; // coordenadas exactas del segmento recortado
    double x2 = 14.0, y2 = 0.0;
    std::string ecuacion;
};

struct DominanciaPourbaix {
    int n_pH = 0;
    int n_Eh = 0;
    double pH_min = 0.0, pH_max = 14.0;
    double Eh_min = -2.0, Eh_max = 2.0;
    std::vector<int> grid;     // [i_pH * n_Eh + i_Eh] = índice de especie o -1
};

class CalculadorPourbaix {
public:
    explicit CalculadorPourbaix(const domain::ThermoRepository& repository)
        : repository_(&repository) {}
    explicit CalculadorPourbaix(const domain::ThermoRepository* repository = nullptr)
        : repository_(repository) {}

    // ── Interfaz clásica y de regresión ──────────────────────────────────────
    void establecer_elemento(const std::string& elemento);
    void establecer_temperatura(double T_K);
    void establecer_concentracion(double conc);
    void establecer_escala_potencial(domain::PourbaixPotentialScale escala);

    double pendiente_nernst() const;
    double eh_agua_oxidacion(double pH, double p_O2_bar = 1.0) const;
    double eh_agua_reduccion(double pH, double p_H2_bar = 1.0) const;

    void cargar_preset_hsc(int preset_idx);
    bool calcular();

    domain::PourbaixPointEvaluation evaluar_punto(double pH, double Eh) const;
    std::string exportar_csv() const;

    const std::vector<LineaPourbaix>& obtener_lineas() const { return lineas_; }
    const std::vector<EspeciePourbaix>& obtener_especies() const { return especies_activas_; }
    const DominanciaPourbaix& obtener_dominancia() const { return dominancia_; }
    const std::vector<domain::EpHPoligonoPredominancia>& obtener_poligonos() const { return ultimo_resultado_.poligonos; }
    const std::vector<domain::EpHPuntoTriple>& obtener_puntos_triples() const { return ultimo_resultado_.puntos_triples; }
    const domain::EpHDiagramResult& ultimo_resultado_eph() const { return ultimo_resultado_; }

    // ── Interfaz completa HSC EpH (STABCAL Multicomponente) ──────────────────
    std::vector<domain::EpHCandidateSpecies> recolectar_especies_sistema(
        const domain::EpHSystemParameters& params) const;

    domain::EpHDiagramResult calcular_diagrama_eph(
        const domain::EpHSystemParameters& params);

    std::vector<domain::EpHCombinerDataset> calcular_combiner_eph(
        const domain::EpHSystemParameters& base_params,
        const std::vector<double>& temperaturas_C) const;

    std::string exportar_tabla_eph_csv(const domain::EpHDiagramResult& res) const;
    std::string serializar_eph8(const domain::EpHSystemParameters& params) const;
    bool deserializar_eph8(const std::string& contenido, domain::EpHSystemParameters& out_params) const;

private:
    const domain::ThermoRepository* repository_ = nullptr;
    std::string elemento_ = "Cu";
    double T_K_ = 298.15;
    double conc_ = 1.0;
    domain::PourbaixPotentialScale escala_ = domain::PourbaixPotentialScale::SHE;

    domain::EpHSystemParameters params_actuales_;
    domain::EpHDiagramResult ultimo_resultado_;

    std::vector<EspeciePourbaix> especies_activas_;
    std::vector<LineaPourbaix> lineas_;
    DominanciaPourbaix dominancia_;
    double dGf_H2O_l_ = -237.141;

    void sincronizar_legado_desde_resultado();
};

}} // namespace
