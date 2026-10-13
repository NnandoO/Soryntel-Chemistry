#pragma once
#include <map>
#include <string>
namespace cma::domain {
enum class AqueousBackend { IonAssociation, Pitzer };
struct AqueousRequest {
    std::map<std::string,double> totals_molal;
    double celsius=25, ph=7;
    AqueousBackend model=AqueousBackend::IonAssociation;
    bool balance_sodium=false;
    bool equilibrate_calcite=false, equilibrate_co2=false;
    double calcite_moles=0, log_co2_pressure=-2;
    // Fases de equilibrio mineral: nombre -> moles iniciales disponibles (SI objetivo = 0.0)
    std::map<std::string,double> equilibrium_phases;
    // Fases gaseosas: nombre -> log10(P en atm)
    std::map<std::string,double> gas_phases;
};
struct AqueousResponse {
    bool ok=false;
    std::string message, engine, database, full_output;
    std::map<std::string,double> values;
};
class AqueousEngine {
public:
    virtual ~AqueousEngine()=default;
    virtual bool available() const=0;
    virtual AqueousResponse solve(const AqueousRequest& input) const=0;
};
}
