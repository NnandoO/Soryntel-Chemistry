#include "exergia.hpp"
#include <cmath>
#include <sstream>

namespace cma { namespace motores {

double Exergia::exergia_fisica(double h, double h0, double T, double T0,
                                double s, double s0) {
    return (h - h0) - T0 * (s - s0);
}

double Exergia::exergia_quimica_especie(const std::string& formula, double T0) {
    // Valores de exergía química estándar (kJ/mol) a 298.15K, 1 atm
    static const std::map<std::string, double> tabla = {
        {"O2", 3.97}, {"N2", 0.72}, {"H2", 236.09}, {"H2O", 0.0},
        {"H2O(g)", 0.0}, {"H2O(l)", 0.9}, {"CO2", 19.87}, {"CO", 275.43},
        {"CH4", 831.65}, {"C2H6", 1495.84}, {"C3H8", 2154.30},
        {"C4H10", 2802.79}, {"C5H12", 3463.43}, {"C6H6", 3303.18},
        {"CH3OH", 722.28}, {"C2H5OH", 1357.70}, {"NH3", 337.86},
        {"H2S", 812.00}, {"SO2", 311.78}, {"SO3", 248.99},
        {"NO", 88.91}, {"NO2", 55.60}, {"N2O", 106.70},
        {"HCl", 91.90}, {"Cl2", 123.50}, {"CaCO3", 0.0},
        {"CaO", 110.00}, {"Fe2O3", 0.0}, {"Fe3O4", 0.0},
        {"SiO2", 0.0}, {"Al2O3", 0.0}, {"MgO", 58.00},
        {"NaCl", 0.0}, {"NaOH", 100.00},
    };

    auto it = tabla.find(formula);
    if (it != tabla.end()) return it->second;

    // Estimación por defecto
    return 100.0;
}

ResultadoExergia Exergia::analizar_proceso(
    double h_entrada, double h_salida, double h0,
    double T_entrada, double T_salida, double T0,
    double s_entrada, double s_salida, double s0,
    double trabajo_util, double calor_aporte)
{
    ResultadoExergia res;

    double ex_in = exergia_fisica(h_entrada, h0, T_entrada, T0, s_entrada, s0);
    double ex_out = exergia_fisica(h_salida, h0, T_salida, T0, s_salida, s0);

    res.exergia_fisica = ex_out;
    res.exergia_total = ex_out;
    res.exergia_destruida = ex_in - ex_out + trabajo_util + calor_aporte * (1 - T0 / T_entrada);

    if (ex_in > 0)
        res.eficiencia_exergetica = (ex_out + trabajo_util) / ex_in * 100.0;
    else
        res.eficiencia_exergetica = 0;

    res.ok = true;
    res.mensaje = "Exergía física entrada: " + std::to_string(ex_in) +
                  " kJ/mol, salida: " + std::to_string(ex_out) + " kJ/mol";

    return res;
}

double Exergia::eficiencia(double exergia_producto, double exergia_combustible) {
    if (exergia_combustible <= 0) return 0;
    return exergia_producto / exergia_combustible * 100.0;
}

}} // namespace
