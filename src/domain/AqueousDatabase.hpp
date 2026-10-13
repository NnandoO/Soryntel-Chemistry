#pragma once
#include <map>
#include <string>
#include <vector>
#include <cmath>
#include <stdexcept>
namespace cma::domain {
enum class AqueousPhase { Dissolved, Solid, Gas };
struct AqueousComponent {
    int id = 0;
    std::string name;
    double charge = 0;
    double molar_mass = 0;
};
struct FormationReaction {
    int id = 0;
    std::string name;
    AqueousPhase phase = AqueousPhase::Dissolved;
    std::map<int,double> components; // product = sum(nu_i * basis_i)
    double log_k_298 = 0;
    double reaction_enthalpy_kJ = 0;
    bool enthalpy_available = false;
    bool temperature_correction_supported = false;
    double charge = 0;
    double molar_mass = 0;
    std::string source_log_k, source_enthalpy, source_file;
    double log_k(double kelvin) const {
        if (!std::isfinite(kelvin) || kelvin <= 0) throw std::invalid_argument("Temperatura inválida");
        if (std::abs(kelvin - 298.15) > 1e-8 && (!enthalpy_available || !temperature_correction_supported))
            throw std::invalid_argument("Este registro solo está habilitado a 298.15 K: corrección térmica/rango sin verificar");
        return log_k_298 + reaction_enthalpy_kJ * 1000 / (8.314462618 * std::log(10.0)) * (1 / 298.15 - 1 / kelvin);
    }
};
struct AqueousDatabase {
    std::map<int,AqueousComponent> components;
    std::map<int,FormationReaction> reactions;
    std::string format = "Visual MINTEQ CSV VDB (formation reaction convention)";
    std::vector<std::string> diagnostics;
};
}
