#pragma once

#include "nucleo/tipos.hpp"

#include <cstdint>
#include <map>
#include <string>
#include <vector>

namespace cma::domain {

enum class StabilityDiagramMode : int {
    Lpp_Isothermal = 0,       // Diagrama isotérmico vectorial log(p_X) vs log(p_Y) (Kellogg / Lpp)
    Tpp_PartialPressure = 1,  // Diagrama Tpp isotérmico de áreas log(p_X) vs log(p_Y) a T fija
    Tpp_TempPressure = 2      // Diagrama Tpp Temperatura (X) vs log(p_Y) (Y) a p_gas fija
};

struct EspecieCandidataEstabilidad {
    int64_t id = 0;
    std::string formula;
    std::string nombre;
    nucleo::Fase fase = nucleo::Fase::Solido;
    double masa_molar = 0.0;
    double h_formacion_298 = 0.0; // kJ/mol
    double s_298 = 0.0;           // J/(mol*K)
    double cp_298 = 0.0;          // J/(mol*K)
    double T_min_K = 298.15;
    double T_max_K = 2000.0;
    double n_main = 0.0;          // Átomos del elemento principal por unidad fórmula
    std::map<std::string, double> composicion; // Admite estequiometría fraccionaria (ej. VO1.24, TiO1.01)
    bool es_gas = false;
    bool es_liquido = false;
    bool es_organico = false;
    bool contiene_elemento_principal = false;
    bool seleccionada_por_defecto = true;
};

struct SistemaTernarioEstabilidad {
    std::vector<std::string> elementos; // 3 elementos seleccionados, ej. {"V", "Cl", "O"}
    std::string elemento_principal;     // Elemento condensado base, ej. "V", "Ti", "Cu"
    std::string elem_aux1;              // Primer elemento auxiliar, ej. "Cl", "S"
    std::string elem_aux2;              // Segundo elemento auxiliar, ej. "O"
    std::vector<EspecieCandidataEstabilidad> especies_sistema;   // Lista para "Select Species"
    std::vector<EspecieCandidataEstabilidad> especies_gas_ejes;  // Gases sin elemento principal para ejes X/Y/Fixed
};

struct VerticeEstabilidad2D {
    double x = 0.0;
    double y = 0.0;
};

struct TrapecioEstabilidad2D {
    double x0 = 0.0;
    double x1 = 0.0;
    double y0_bot = 0.0;
    double y0_top = 0.0;
    double y1_bot = 0.0;
    double y1_top = 0.0;
};

struct PoligonoPredominancia {
    int64_t id = 0;
    std::string formula;          // Fórmula completa en lista (ej. "VCl4(l)")
    std::string etiqueta_grafico; // Etiqueta limpia en el área (ej. "VCl4", "CuO*CuSO4")
    std::string nombre;
    int color_index = 0;
    std::vector<VerticeEstabilidad2D> vertices;     // Contorno cerrado del polígono o banda T-p
    std::vector<TrapecioEstabilidad2D> trapecios;   // Bandas verticales convexas (usado en modo T-p curvo)
    VerticeEstabilidad2D centroide;
    double area = 0.0;
    double nu_x_norm = 0.0;       // Moles de gas X por mol de elemento principal
    double nu_y_norm = 0.0;       // Moles de gas Y por mol de elemento principal
    double Z = 0.0;               // Término independiente del plano G_norm = Z + A*x + B*y
    double A = 0.0;               // Pendiente respecto a x
    double B = 0.0;               // Pendiente respecto a y
};

struct LineaFronteraLpp {
    std::string fase_a;
    std::string fase_b;
    std::string ecuacion_reaccion; // Ej. "2 V3O7 + 0.5 O2(g) = 3 V2O5"
    double delta_G_rxn_kJ = 0.0;
    double log10_K = 0.0;
    VerticeEstabilidad2D p1;
    VerticeEstabilidad2D p2;
    double pendiente = 0.0;        // dy / dx
};

struct PuntoInvarianteTriple {
    std::string fase_1;
    std::string fase_2;
    std::string fase_3;
    double x = 0.0;
    double y = 0.0;
};

struct ParametrosCalculoLpp {
    std::vector<std::string> elementos = {"Ti", "Cl", "O"};
    std::string elemento_principal; // Si está vacío se detecta automáticamente
    std::string gas_x = "Cl2(g)";
    std::string gas_y = "O2(g)";
    std::vector<std::string> formulas_seleccionadas;
    double T_K = 573.15;            // 300 °C por defecto en Lpp (ejemplo HSC Ti-Cl-O)
    bool auto_escala = true;
    double x_min = -45.0;
    double x_max = 0.0;
    double y_min = -90.0;
    double y_max = 0.0;
    bool incluir_liquidos = false;
    bool incluir_organicos = false;
    bool omitir_estequiometria_identica = true;
};

struct ResultadoDiagramaLpp {
    bool ok = false;
    std::string mensaje_error;
    std::string titulo_sistema;     // Ej. "Ti-Cl-O"
    std::string elemento_principal;
    std::string gas_x;
    std::string gas_y;
    double T_K = 573.15;
    double x_min_usado = -45.0;
    double x_max_usado = 0.0;
    double y_min_usado = -90.0;
    double y_max_usado = 0.0;
    std::vector<PoligonoPredominancia> poligonos;
    std::vector<LineaFronteraLpp> fronteras;
    std::vector<PuntoInvarianteTriple> puntos_triples;
    std::vector<std::string> advertencias;
    std::vector<std::string> especies_omitidas_duplicadas;
};

struct ParametrosCalculoTpp {
    std::vector<std::string> elementos = {"V", "Cl", "O"};
    std::string elemento_principal;
    bool eje_x_es_temperatura = false; // false: p-p (log pX vs log pY), true: T-p (T vs log pY)
    bool temp_en_celsius = true;       // Unidad para mostrar/ingresar temperatura (°C o K)
    std::string gas_x = "Cl2(g)";      // Usado cuando !eje_x_es_temperatura
    std::string gas_y = "O2(g)";       // Eje Y siempre es presión parcial log pY
    std::string gas_fijo = "O2(g)";    // Usado cuando eje_x_es_temperatura
    double x_min = -40.0;              // log pX_min o T_min (en °C o K según temp_en_celsius)
    double x_max = 0.0;                // log pX_max o T_max
    double y_min = -40.0;              // log pY_min
    double y_max = 0.0;                // log pY_max
    double valor_fijo = 500.0;         // Si p-p: Temperatura (en °C o K); si T-p: p_gas en bar (ej. 0.1 o 1e-20)
    bool presion_fija_en_bar = true;   // true: valor_fijo > 0 es bar (1e-20, 0.1); false: es log10(p)
    std::vector<std::string> formulas_seleccionadas;
    bool incluir_organicos = false;
    int profundidad_recursiva = 6;     // Algoritmo recursivo de subdivisión quadtree (2^6 = 64x64)
};

struct ResultadoDiagramaTpp {
    bool ok = false;
    std::string mensaje_error;
    StabilityDiagramMode modo = StabilityDiagramMode::Tpp_PartialPressure;
    std::string titulo_diagrama;       // Ej. "Predominance Diagram for V-Cl-O System"
    std::string subtitulo_constante;   // Ej. "Constant value: Temperature = 500 °C"
    std::string etiqueta_eje_x;        // Ej. "log pCl2(g)" o "T / °C"
    std::string etiqueta_eje_y;        // Ej. "log pO2(g)"
    std::string elemento_principal;
    double x_min = -40.0;
    double x_max = 0.0;
    double y_min = -40.0;
    double y_max = 0.0;
    std::vector<PoligonoPredominancia> poligonos;
    std::vector<LineaFronteraLpp> fronteras;
    int grid_cols = 0;
    int grid_rows = 0;
    std::vector<int> grid_phase_idx;   // Índice de polígono predominante en cada celda
    int evaluaciones_gibbs = 0;        // Número real de evaluaciones de Gibbs del algoritmo recursivo
    std::vector<std::string> advertencias;
};

} // namespace cma::domain
