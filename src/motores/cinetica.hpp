#pragma once
#include "nucleo/tipos.hpp"
#include <vector>
#include <map>
#include <cmath>

namespace cma { namespace motores {

struct ResultadoCinetica {
    std::map<int64_t, double> concentracion_final; // mol/L
    double tiempo = 0.0;              // s
    double conversion = 0.0;
    double velocidad = 0.0;           // mol/(L·s)
    std::string mensaje;
};

class Cinetica {
public:
    Cinetica() = default;

    // Ley de Arrhenius: k = A * exp(-Ea/RT)
    double arrhenius(double A, double Ea_kJ, double T_K) const;

    // Velocidad de reacción para orden n
    double velocidad_reaccion(double k, const std::vector<double>& concentraciones,
                              const std::vector<int>& ordenes) const;

    // Reactor batch (discontinuo) - integración para orden 0, 1, 2
    ResultadoCinetica reactor_batch(
        double C_A0,              // Concentración inicial (mol/L)
        int orden,                // 0, 1, o 2
        double k,                 // constante cinética
        double tiempo,            // s
        double C_B0 = 0.0         // para orden 2 con estequeometría 1:1
    ) const;

    // Reactor CSTR (tanque agitado continuo)
    ResultadoCinetica reactor_cstr(
        double C_A0,              // Concentración entrada (mol/L)
        int orden,
        double k,
        double tau,               // tiempo espacial (s)
        double C_B0 = 0.0
    ) const;

    // Reactor PFR (flujo pistón)
    ResultadoCinetica reactor_pfr(
        double C_A0,
        int orden,
        double k,
        double tau,
        double C_B0 = 0.0
    ) const;

    // Tiempo de vida media
    double vida_media(double k, int orden, double C0 = 1.0) const;

    // Conversión desde concentración
    double conversion_desde_C(double C0, double C) const;
};

}} // namespace
