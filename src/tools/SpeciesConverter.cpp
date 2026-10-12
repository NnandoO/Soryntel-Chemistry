#include "SpeciesConverter.hpp"
#include <algorithm>
#include <cmath>
#include <cctype>
#include <iostream>

namespace hsc_clone {

// Simple chemical formula parser. Extracts elements and their counts.
// e.g. "CuFeS2" -> {"Cu":1, "Fe":1, "S":2}
// e.g. "Fe2(SO4)3" -> Not supported yet (no parenthesis parsing for this simple version).
// We will only support flat formulas like "Al2O3", "Sb2S3" etc.
static std::map<std::string, double> parse_formula(const std::string& f) {
    std::map<std::string, double> res;
    size_t i = 0;
    while (i < f.length()) {
        if (std::isupper(f[i])) {
            std::string elem = "";
            elem += f[i++];
            if (i < f.length() && std::islower(f[i])) {
                elem += f[i++];
            }
            std::string count_str = "";
            while (i < f.length() && (std::isdigit(f[i]) || f[i] == '.')) {
                count_str += f[i++];
            }
            double count = count_str.empty() ? 1.0 : std::stod(count_str);
            res[elem] += count;
        } else {
            // Ignore other characters like brackets in "[Others]"
            if (f[i] == '[' && f.find("[Others]") != std::string::npos) {
                res["[Others]"] = 1.0;
                break;
            }
            i++;
        }
    }
    return res;
}

std::map<std::string, double> SpeciesConverter::analisisAEspecies(const std::map<std::string, double>& input_wt, const std::vector<std::string>& target_species) {
    std::map<std::string, double> especies;
    
    // Hardcoded atomic weights
    std::map<std::string, double> AW = {
        {"H", 1.008}, {"C", 12.011}, {"O", 15.999}, {"S", 32.065}, 
        {"Cu", 63.546}, {"Fe", 55.845}, {"Ni", 58.693}, {"Co", 58.933}, 
        {"Zn", 65.38}, {"Pb", 207.2}, {"Sb", 121.76}, {"As", 74.922}, 
        {"Ag", 107.868}, {"Au", 196.967}, {"Si", 28.085}, {"Al", 26.982},
        {"Mg", 24.305}, {"Ca", 40.078}, {"Na", 22.990}, {"K", 39.098},
        {"Cl", 35.45}, {"F", 18.998}, {"N", 14.007}, {"P", 30.974}
    };

    // Calculate moles of each input element
    std::map<std::string, double> available_moles;
    for (auto& pair : input_wt) {
        if (pair.first == "SiO2" || pair.first == "Al2O3" || pair.first == "MgO" || pair.first == "[Others]") {
            // Pass-throughs
            available_moles[pair.first] = pair.second; 
        } else {
            double aw = AW.count(pair.first) ? AW[pair.first] : 1.0; // Default to 1.0 if unknown
            available_moles[pair.first] = pair.second / aw;
        }
    }

    // Dynamic Distribution
    // This is a greedy sequential allocation. It is highly order-dependent.
    // In a real HSC engine, this would be a Least Squares Matrix Solver.
    
    for (const auto& sp : target_species) {
        if (sp == "SiO2" || sp == "Al2O3" || sp == "MgO" || sp == "[Others]" || sp == "Au") {
            // Special pass-through
            if (available_moles.count(sp) && available_moles[sp] > 0) {
                especies[sp] = available_moles[sp];
                available_moles[sp] = 0; // consumed
            }
            continue;
        }

        auto stoichiometry = parse_formula(sp);
        if (stoichiometry.empty()) continue;

        // Calculate Molecular Weight
        double MW = 0.0;
        for (auto& st : stoichiometry) {
            double aw = AW.count(st.first) ? AW[st.first] : 1.0;
            MW += st.second * aw;
        }

        // Find limiting element
        double max_moles_of_species = 1e9;
        for (auto& st : stoichiometry) {
            double available = available_moles[st.first];
            double possible = available / st.second;
            if (possible < max_moles_of_species) {
                max_moles_of_species = possible;
            }
        }
        
        // We might want to use some heuristics for common conflicts (e.g. CuFeS2 vs Cu2S)
        // If we are strictly greedy, we will consume all we can.
        // Let's add a small hack: If it's CuFeS2, don't consume ALL Fe if there is FeS2 next.
        // We'll just do purely greedy for this simple version.

        if (max_moles_of_species > 0) {
            especies[sp] = max_moles_of_species * MW;
            // Consume
            for (auto& st : stoichiometry) {
                available_moles[st.first] -= max_moles_of_species * st.second;
            }
        }
    }

    return especies;
}

} // namespace hsc_clone
