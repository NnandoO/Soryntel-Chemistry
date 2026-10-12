#pragma once
#include <vector>

namespace hsc_clone {

struct PuntoPropiedad {
    double T;
    double H;
    double S;
    double Cp;
    double G;
};

class PropertyDiagrams {
public:
    std::vector<PuntoPropiedad> generarCurvas(double h_ref, double s_ref,
                                              double cp_a, double cp_b,
                                              double cp_c, double cp_d,
                                              double T_min, double T_max);
};

}
