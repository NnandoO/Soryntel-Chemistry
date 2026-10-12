#pragma once
#include "nucleo/tipos.hpp"
#include <vector>
#include <map>
#include <cmath>

namespace cma { namespace motores {

struct PuntoBurbuja {
    double temperatura = 0.0; // K
    double presion     = 0.0; // atm
    std::map<int64_t, double> y_composicion;
    bool convergio = false;
    std::string mensaje;
};

struct PuntoRocio {
    double temperatura = 0.0; // K
    double presion     = 0.0; // atm
    std::map<int64_t, double> x_composicion;
    bool convergio = false;
    std::string mensaje;
};

struct ResultadoFlash {
    double fraccion_vapor = 0.0; // V/F
    std::map<int64_t, double> x_liquido;
    std::map<int64_t, double> y_vapor;
    double T = 0.0, P = 0.0;
    bool convergio = false;
    std::string mensaje;
};

class EquilibrioFases {
public:
    EquilibrioFases() = default;

    // Presión de vapor (Antoine)
    double presion_vapor(const nucleo::Especie& sp, double T_K) const;

    // Coeficiente de equilibrio K = y/x = P_vap/P
    double coeficiente_K(const nucleo::Especie& sp, double T_K, double P_atm) const;

    // Punto de burbuja (dado P, encontrar T y composición de vapor)
    PuntoBurbuja punto_burbuja(const std::map<int64_t, double>& x_composicion,
                                double P_atm,
                                const std::map<int64_t, nucleo::Especie>& db) const;

    // Punto de rocío (dado P, encontrar T y composición de líquido)
    PuntoRocio punto_rocio(const std::map<int64_t, double>& y_composicion,
                            double P_atm,
                            const std::map<int64_t, nucleo::Especie>& db) const;

    // Cálculo flash isotérmico
    ResultadoFlash flash_isotermico(const nucleo::Flujo& alimentacion,
                                     double T_K, double P_atm,
                                     const std::map<int64_t, nucleo::Especie>& db) const;

    // Coeficiente de actividad (modelo simplificado)
    double coeficiente_actividad(double fraccion_molar, double T_K, double parametro_interaccion = 0.0) const;

    // Presión de vapor usando ecuación de Clausius-Clapeyron
    double clausius_clapeyron(double P_ref, double H_vap, double T_ref, double T_K) const;

private:
    static constexpr int MAX_ITER = 100;
    static constexpr double TOL = 1e-8;
};

}} // namespace
