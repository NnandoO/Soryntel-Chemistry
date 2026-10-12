#pragma once
#include <map>
#include <string>
#include <vector>

namespace hsc_clone {

class SpeciesConverter {
public:
    // Takes elemental/compound analysis wt-% as input, returns species wt-%
    std::map<std::string, double> analisisAEspecies(const std::map<std::string, double>& input_wt, const std::vector<std::string>& target_species);
};

} // namespace hsc_clone
