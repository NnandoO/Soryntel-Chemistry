#pragma once
#include <vector>
#include <string>
#include <map>

namespace cma { namespace motores {

struct PropiedadesAgua {
    bool ok = false;
    std::string mensaje;
    std::string modelo = "IAPWS-IF97 / CoolProp";
    double T = 0;      // K
    double P = 0;      // MPa
    double h = 0;      // kJ/kg
    double s = 0;      // kJ/(kg·K)
    double v = 0;      // m³/kg
    double cp = 0;     // kJ/(kg·K)
    double cv = 0;     // kJ/(kg·K)
    double mu = 0;     // Pa·s (viscosidad)
    double k = 0;      // W/(m·K) (conductividad)
    std::string fase; // líquido, vapor, supercrítico
};

class Agua {
public:
    // IAPWS-IF97, regions 1/2/3/4/5; K, MPa, kJ/kg at this API boundary.
    PropiedadesAgua propiedades(double T, double P);

    // Saturación: dada T, calcular P_sat y propiedades de líquido/vapor
    std::pair<PropiedadesAgua, PropiedadesAgua> saturacion_T(double T);

    // Saturación: dada P, calcular T_sat
    double temperatura_saturacion(double P);
    double presion_saturacion(double T);
    PropiedadesAgua mezcla_saturada(double P, double calidad_vapor);

    // Calor latente de vaporización
    double calor_latente(double T);

    // Viscosidad del agua líquida (aproximación)
    double viscosidad_liquida(double T);

    // Conductividad térmica
    double conductividad_termica(double T, double P);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;
};

}} // namespace
