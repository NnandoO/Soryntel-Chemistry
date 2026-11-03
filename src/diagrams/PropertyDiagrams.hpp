#pragma once

#include "domain/PropertyDiagramTypes.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "application/modules.hpp"

#include <string>
#include <utility>
#include <vector>

namespace cma::application::modules {

struct PuntoPropiedad {
    double T;
    double H;
    double S;
    double Cp;
    double G;
};

struct TerminoReaccionDiagrama {
    std::string formula;
    double coef = 1.0; // Negativo = reactivo, Positivo = producto
};

class PropertyDiagrams {
  public:
    // API clasica compatible con regresion previa (soporta Maier-Kelley c*T^-2 cuando |cp_c| > 1.0)
    std::vector<PuntoPropiedad> generarCurvas(double h_ref, double s_ref, double cp_a, double cp_b,
                                              double cp_c, double cp_d, double T_min, double T_max);

    // Evalua una especie (o envolvente de polimorfos s->l->g) y calcula H, H-H298, S, Cp, G y
    // propiedades reales de formacion (Delta G°f, Delta H°f, Delta S°f, log10 Kf) desde elementos de referencia
    domain::CurvaDiagramaTermo calcular_curva_especie(
        const domain::ThermoRepository& repo,
        const std::string& formula_o_nombre,
        double T_min_K,
        double T_max_K,
        int steps = 120,
        bool usar_envolvente_fases = true,
        double multiplicador = 1.0) const;

    // Evalua una reaccion quimica arbitraria (normalizada segun EllinghamNormalizationBasis)
    domain::CurvaDiagramaTermo calcular_curva_reaccion(
        const domain::ThermoRepository& repo,
        const std::string& etiqueta,
        const std::vector<TerminoReaccionDiagrama>& terminos,
        domain::EllinghamNormalizationBasis basis,
        double T_min_K,
        double T_max_K,
        int steps = 120,
        bool usar_envolvente_fases = true) const;

    // Parsea una ecuacion en texto (ej. "2 Fe + O2(g) = 2 FeO" o "2 C + O2(g) -> 2 CO(g)") y calcula su curva
    domain::CurvaDiagramaTermo calcular_curva_ecuacion_texto(
        const domain::ThermoRepository& repo,
        const std::string& ecuacion_texto,
        domain::EllinghamNormalizationBasis basis,
        double T_min_K,
        double T_max_K,
        int steps = 120,
        bool usar_envolvente_fases = true) const;

    // Genera un diagrama completo de una familia Ellingham preconfigurada de HSC
    domain::ResultadoDiagramaPropiedades generar_familia_ellingham(
        const domain::ThermoRepository& repo,
        domain::EllinghamPresetFamily familia,
        double T_min_K = 298.15,
        double T_max_K = 2000.0,
        int steps = 120,
        bool usar_envolvente_fases = true) const;

    // Detecta todas las intersecciones entre curvas visibles para el tipo de propiedad seleccionado
    std::vector<domain::InterseccionEllingham> encontrar_intersecciones(
        const std::vector<domain::CurvaDiagramaTermo>& curvas,
        domain::PropertyDiagramType tipo = domain::PropertyDiagramType::DeltaG_Ellingham) const;

    // Evalua las escalas nomograficas de Richardson-Jeffes en cualquier punto (T_K, delta_G_kJ_per_O2)
    domain::EscalaNomograficaPunto evaluar_escala_nomografica(
        const domain::ThermoRepository& repo,
        double T_K,
        double delta_G_kJ_per_O2) const;

    // Genera el abanico de lineas isopotenciales (log10 pO2, log10(CO/CO2), log10(H2/H2O))
    void generar_lineas_isopotenciales(
        const domain::ThermoRepository& repo,
        double T_min_K,
        double T_max_K,
        std::vector<domain::LineaIsoPotencial>& iso_pO2,
        std::vector<domain::LineaIsoPotencial>& iso_CO_CO2,
        std::vector<domain::LineaIsoPotencial>& iso_H2_H2O) const;

    // Exporta las curvas e intersecciones calculadas a formato CSV
    std::string exportar_csv(
        const std::vector<domain::CurvaDiagramaTermo>& curvas,
        const std::vector<domain::InterseccionEllingham>& intersecciones,
        domain::PropertyDiagramType tipo,
        bool temp_en_celsius = false,
        bool energia_en_kcal = false) const;
};

} // namespace cma::application::modules
