#include "StabilityDiagrams.hpp"

namespace cma::application::modules {

std::vector<PuntoEstabilidad> StabilityDiagrams::generar(const std::string&,
                                                          double)
{
    // This compatibility API lacks a repository/activity model. No fabricated points.
    // The desktop's stability calculator has a separate thermodynamic solver.
    return {};
}

}
