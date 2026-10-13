#pragma once
#include "domain/AqueousDatabase.hpp"
#include <map>
#include <string>
#include <vector>
namespace cma::motores {
enum class ActivityModel { Ideal, Davies };
struct SpeciationInput {
    std::map<int,double> totals_molal;
    std::map<int,double> fixed_log_activities = {{2,0},{330,-7}}; // solvent and buffered pH
    double kelvin = 298.15;
    ActivityModel model = ActivityModel::Ideal;
};
struct SpeciationResult {
    bool ok = false;
    std::string message;
    std::map<int,double> free_components, dissolved_species, calculated_totals;
    std::map<int,double> saturation_indices;
    double ionic_strength = 0, charge_residual = 0, mass_residual = 0;
    int iterations = 0;
};
class AqueousSpeciation {
public:
    explicit AqueousSpeciation(const domain::AqueousDatabase& database) : database_(database) {}
    SpeciationResult solve(const SpeciationInput& input) const;
private:
    const domain::AqueousDatabase& database_;
};
}
