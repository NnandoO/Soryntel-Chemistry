#include "PropertyDiagrams.hpp"
#include <cmath>

namespace cma::application::modules {

std::vector<PuntoPropiedad> PropertyDiagrams::generarCurvas(
    double h_ref, double s_ref, double cp_a, double cp_b,
    double cp_c, double cp_d, double T_min, double T_max)
{
    std::vector<PuntoPropiedad> out;
    int steps = 100;
    double dt = (T_max - T_min) / steps;
    
    for (int i = 0; i <= steps; ++i) {
        double T = std::max(T_min + i * dt, 1e-10);
        double T_ref = 298.15;
        double cp = cp_a + cp_b * T + cp_c * T * T + cp_d * T * T * T;
        
        // Analytical integral for Enthalpy (J/mol) -> convert to kJ/mol
        double delta_h_j = cp_a * (T - T_ref) 
                         + (cp_b / 2.0) * (T * T - T_ref * T_ref) 
                         + (cp_c / 3.0) * (T * T * T - T_ref * T_ref * T_ref) 
                         + (cp_d / 4.0) * (T * T * T * T - T_ref * T_ref * T_ref * T_ref);
        double H = h_ref + delta_h_j / 1000.0;
        
        // Analytical integral for Entropy (J/(mol*K))
        double S = s_ref;
        if (T > 0 && T_ref > 0) {
            double delta_s_j = cp_a * std::log(T / T_ref) 
                             + cp_b * (T - T_ref) 
                             + (cp_c / 2.0) * (T * T - T_ref * T_ref) 
                             + (cp_d / 3.0) * (T * T * T - T_ref * T_ref * T_ref);
            S += delta_s_j;
        }
        
        double G = H - (T * S / 1000.0); // S must be converted to kJ for G
        out.push_back({T, H, S, cp, G});
    }
    return out;
}

}
