#pragma once
#include <vector>
#include <string>
#include <map>
#include "../nucleo/tipos.hpp"

namespace cma { namespace motores {

struct EspeciePourbaix {
    std::string formula;
    std::string nombre;
    double dG_f = 0.0;         // ΔGf° kJ/mol (corregido por concentración si es acuoso)
    int carga = 0;
    int coef_elemento = 1;     // átomos del elemento principal
    int estado = 0;            // 0: gas, 1: líquido, 2: sólido, 3: acuoso
    double n_H = 0.0;           // átomos de H por 1 átomo del elemento principal
    double n_O = 0.0;           // átomos de O por 1 átomo del elemento principal
};

struct LineaPourbaix {
    std::string especie_a;
    std::string especie_b;
    double m = 0.0;         // pendiente (dEh/dpH)
    double b = 0.0;         // intercepto (Eh a pH=0)
    bool vertical = false;  // true para reacciones ácido-base puras
    double x_val = 0.0;     // valor de pH si es vertical
    bool es_agua = false;   // línea de estabilidad del agua
};

// Celda de dominancia: índice de la especie dominante en cada (pH, Eh)
struct DominanciaPourbaix {
    int n_pH = 0;
    int n_Eh = 0;
    double pH_min = 0.0, pH_max = 14.0;
    double Eh_min = -2.0, Eh_max = 2.0;
    std::vector<int> grid;  // [i_pH * n_Eh + i_Eh] = índice de especie o -1
};

class CalculadorPourbaix {
public:
    CalculadorPourbaix() = default;

    void establecer_elemento(const std::string& elemento);
    void establecer_temperatura(double T_K);
    void establecer_concentracion(double conc);

    bool calcular();

    const std::vector<LineaPourbaix>& obtener_lineas() const { return lineas_; }
    const std::vector<EspeciePourbaix>& obtener_especies() const { return especies_activas_; }
    const DominanciaPourbaix& obtener_dominancia() const { return dominancia_; }

private:
    std::string elemento_;
    double T_K_ = 298.15;
    double conc_ = 1e-6;       // 1 µM por defecto

    std::vector<EspeciePourbaix> especies_activas_;
    std::vector<LineaPourbaix> lineas_;
    DominanciaPourbaix dominancia_;

    void buscar_especies();
    void generar_lineas();
    void calcular_dominancia();

    // ΔGf° de H2O(l) en T_K_ (kJ/mol)
    double dGf_H2O_l_ = -237.13;
};

}} // namespace
