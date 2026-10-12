#include "StabilityDiagrams.hpp"

namespace hsc_clone {

std::vector<PuntoEstabilidad> StabilityDiagrams::generar(const std::string& metal,
                                                          double T)
{
    std::vector<PuntoEstabilidad> out;
    out.push_back({-100.0, -100.0, metal + " (s)"});
    out.push_back({0.0, 0.0, metal + "O2 (s)"});
    return out;
}

}
