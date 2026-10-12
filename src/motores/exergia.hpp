#pragma once
#include <vector>
#include <string>
#include <map>

namespace cma { namespace motores {

struct ResultadoExergia {
    double exergia_fisica = 0;    // kJ/mol
    double exergia_quimica = 0;   // kJ/mol
    double exergia_total = 0;     // kJ/mol
    double eficiencia_exergetica = 0; // %
    double exergia_destruida = 0;  // kJ/mol
    std::string mensaje;
    bool ok = false;
};

class Exergia {
public:
    // Exergía física de una corriente (cambio de T y P respecto al ambiente)
    double exergia_fisica(double h, double h0, double T, double T0, double s, double s0);

    // Exergía química de especies puras (valores de referencia)
    double exergia_quimica_especie(const std::string& formula, double T0 = 298.15);

    // Análisis completo de un proceso
    ResultadoExergia analizar_proceso(
        double h_entrada, double h_salida, double h0,
        double T_entrada, double T_salida, double T0,
        double s_entrada, double s_salida, double s0,
        double trabajo_util = 0, double calor_aporte = 0);

    // Eficiencia exergética de un equipo
    double eficiencia(double exergia_producto, double exergia_combustible);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;
    double T0_ = 298.15;
    double P0_ = 1.0;
};

}} // namespace
