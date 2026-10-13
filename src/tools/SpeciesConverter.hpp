#pragma once
#include "application/modules.hpp"
#include <map>
#include <string>
#include <vector>

namespace cma::application::modules {

class SpeciesConverter {
public:
    static std::map<std::string,double> composicion(const std::string& formula);
    // Takes elemental/compound analysis wt-% as input, returns species wt-%
    std::map<std::string, double> analisisAEspecies(const std::map<std::string, double>& input_wt, const std::vector<std::string>& target_species);
};

} // namespace cma::application::modules
