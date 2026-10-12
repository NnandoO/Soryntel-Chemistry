#pragma once
#include <vector>
#include <string>
#include <map>

namespace cma { namespace motores {

struct PropiedadesAgua {
    double T;      // K
    double P;      // MPa
    double h;      // kJ/kg
    double s;      // kJ/(kg·K)
    double v;      // m³/kg
    double cp;     // kJ/(kg·K)
    double cv;     // kJ/(kg·K)
    double mu;     // Pa·s (viscosidad)
    double k;      // W/(m·K) (conductividad)
    std::string fase; // líquido, vapor, supercrítico
};

class Agua {
public:
    // Tablas de vapor IAPWS-IF97 (implementación simplificada)
    PropiedadesAgua propiedades(double T, double P);

    // Saturación: dada T, calcular P_sat y propiedades de líquido/vapor
    std::pair<PropiedadesAgua, PropiedadesAgua> saturacion_T(double T);

    // Saturación: dada P, calcular T_sat
    double temperatura_saturacion(double P);

    // Calor latente de vaporización
    double calor_latente(double T);

    // Viscosidad del agua líquida (aproximación)
    double viscosidad_liquida(double T);

    // Conductividad térmica
    double conductividad_termica(double T, double P);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;
    double P_saturacion(double T);
    double h_liquido(double T);
    double h_vapor(double T);
    double s_liquido(double T);
    double s_vapor(double T);
};

}} // namespace
