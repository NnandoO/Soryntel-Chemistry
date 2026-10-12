#pragma once
#include "nucleo/tipos.hpp"
#include <vector>
#include <map>
#include <cmath>

namespace cma { namespace motores {

struct PotencialElectrodo {
    double E0 = 0.0;       // Potencial estándar (V)
    double E = 0.0;         // Potencial corregido (V)
    int n_electrones = 1;
    double delta_G = 0.0;   // kJ/mol
    double K_eq = 0.0;      // Constante de equilibrio
    bool reversibe = true;
    std::string semirreaccion;
    std::string mensaje;
};

struct CeldaElectroquimica {
    double potencial_celda = 0.0; // V
    double delta_G = 0.0;         // kJ/mol
    double K_eq = 0.0;
    bool espontanea = false;
    std::string mensaje;
};

class Electroquimica {
public:
    Electroquimica() = default;

    // Ecuación de Nernst: E = E0 - (RT/nF) * ln(Q)
    double nernst(double E0, double T_K, int n_electrones, double Q) const;

    // Potencial de electrodo corregido por concentración
    PotencialElectrodo potencial_electrodo(
        double E0, int n_electrones,
        const std::map<std::string, double>& concentraciones, // especie -> conc (M)
        const std::map<std::string, int>& coeficientes,       // especie -> coeff estequio
        double T_K = nucleo::STP_TEMP
    ) const;

    // Potencial de celda a partir de dos semirreacciones
    CeldaElectroquimica potencial_celda(
        const PotencialElectrodo& anodo,
        const PotencialElectrodo& catodo
    ) const;

    // Energía libre de Gibbs desde potencial: ΔG = -nFE
    double gibbs_desde_potencial(double E, int n_electrones) const;

    // Potencial desde energía libre: E = -ΔG/(nF)
    double potencial_desde_gibbs(double delta_G_kJ, int n_electrones) const;

    // Potenciales estándar de electrodo (V vs SHE)
    static double potencial_estandar(const std::string& semirreaccion);

    // Ley de Faraday: masa = (I*t*M)/(n*F)
    double ley_faraday(double corriente_A, double tiempo_s, double masa_molar, int n_electrones) const;

    // Conductividad electrolítica (aproximación)
    double conductividad_electrolito(double concentracion_M, double movilidad_ionica) const;

    // pH desde concentración de H+
    double ph(double concentracion_H) const;
    double concentracion_h(double ph) const;
};

}} // namespace
