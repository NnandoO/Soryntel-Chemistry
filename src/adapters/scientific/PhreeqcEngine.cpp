#include "PhreeqcEngine.hpp"
#include <cmath>
#include <filesystem>
#include <iomanip>
#include <locale>
#include <mutex>
#include <set>
#include <sstream>
#ifdef CMA_HAVE_IPHREEQC
#include <IPhreeqc.h>
#include <Var.h>
#endif
namespace cma::adapters {
bool PhreeqcEngine::available() const {
#ifdef CMA_HAVE_IPHREEQC
    return std::filesystem::is_regular_file(std::filesystem::path(directory_)/"phreeqc.dat") &&
           std::filesystem::is_regular_file(std::filesystem::path(directory_)/"pitzer.dat");
#else
    return false;
#endif
}
static bool is_valid_name(const std::string& s) {
    if (s.empty() || s.size() > 64) return false;
    for (char c : s) {
        if (!std::isalnum(static_cast<unsigned char>(c)) &&
            c != '(' && c != ')' && c != '+' && c != '-' && c != '.' && c != '_')
            return false;
    }
    return true;
}

domain::AqueousResponse PhreeqcEngine::solve(const domain::AqueousRequest& input) const {
    domain::AqueousResponse result;
    result.engine="USGS IPhreeqc; revision f107fcb2f2e1e3167ce4c1328e477b54dee3d21b";
    const bool is_pitzer = (input.model == domain::AqueousBackend::Pitzer);
    result.database = is_pitzer ? "pitzer.dat" : "phreeqc.dat";
    if (!available()) { result.message="Motor no disponible. Configure CMA_WITH_IPHREEQC=ON y revise data/reference/phreeqc."; return result; }
    if (!std::isfinite(input.celsius) || input.celsius<0 || input.celsius>100 ||
        !std::isfinite(input.ph) || input.ph<0 || input.ph>14 ||
        !std::isfinite(input.calcite_moles) || input.calcite_moles<0 ||
        !std::isfinite(input.log_co2_pressure) || input.log_co2_pressure<-15 || input.log_co2_pressure>10 ||
        (input.balance_sodium && !input.totals_molal.count("Na"))) {
        result.message="Entradas inválidas: 0..100 °C, pH 0..14, fase no negativa, log P -15..10; balance de carga requiere Na."; return result;
    }

    const std::set<std::string> pitzer_components = {
        "Na", "Cl", "Ca", "Mg", "K", "C(4)", "C", "S(6)", "S",
        "Fe", "Mn", "Ba", "Sr", "Li", "Br", "B", "Si", "Alkalinity"
    };
    const std::set<std::string> phreeqc_components = {
        "Na", "Cl", "Ca", "Mg", "K", "C(4)", "C", "S(6)", "S",
        "Fe", "Fe(+2)", "Fe(+3)", "Mn", "Al", "Ba", "Sr", "Si",
        "Cu", "Cu(+2)", "Cu(+1)", "Zn", "Pb", "Cd", "Li", "Br", "B",
        "F", "P", "N(5)", "N(-3)", "Alkalinity"
    };
    const auto& allowed_components = is_pitzer ? pitzer_components : phreeqc_components;

    for (const auto& [name, total] : input.totals_molal) {
        if (!is_valid_name(name) || !allowed_components.count(name) || !std::isfinite(total) || total < 0 || total > 20) {
            result.message = "Componente '" + name + "' no admitido en " + result.database + " o total fuera de 0..20 mol/kgw.";
            return result;
        }
    }

    std::map<std::string, double> all_phases = input.equilibrium_phases;
    if (input.equilibrate_calcite && !all_phases.count("Calcite")) {
        all_phases["Calcite"] = input.calcite_moles;
    }
    std::map<std::string, double> all_gases = input.gas_phases;
    if (input.equilibrate_co2 && !all_gases.count("CO2(g)")) {
        all_gases["CO2(g)"] = input.log_co2_pressure;
    }

    for (const auto& [name, moles] : all_phases) {
        if (!is_valid_name(name) || !std::isfinite(moles) || moles < 0 || moles > 1000) {
            result.message = "Fase de equilibrio '" + name + "' inválida o moles fuera de 0..1000.";
            return result;
        }
    }
    for (const auto& [name, logP] : all_gases) {
        if (!is_valid_name(name) || !std::isfinite(logP) || logP < -15 || logP > 10) {
            result.message = "Fase gaseosa '" + name + "' inválida o log P fuera de -15..10.";
            return result;
        }
    }

#ifdef CMA_HAVE_IPHREEQC
    // IPhreeqc's global instance registry is shared; serialize creation/execution.
    static std::mutex mutex;
    const std::lock_guard<std::mutex> lock(mutex);
    struct Instance {
        int id=CreateIPhreeqc();
        ~Instance() { if (id>=0) DestroyIPhreeqc(id); }
    } instance;
    const int id=instance.id;
    if (id<0) { result.message="No se pudo crear IPhreeqc."; return result; }
    SetOutputFileOn(id,0); SetLogFileOn(id,0); SetDumpFileOn(id,0); SetSelectedOutputFileOn(id,0);
    SetOutputStringOn(id,1);
    const auto database=std::filesystem::path(directory_)/result.database;
    if (LoadDatabase(id,database.string().c_str())!=0) { result.message=GetErrorString(id); return result; }
    std::ostringstream program; program.imbue(std::locale::classic()); program<<std::setprecision(17);
    program<<"SOLUTION 1\n temp "<<input.celsius<<"\n pH "<<input.ph<<"\n units mol/kgw\n -water 1\n";
    for (const auto& [name,total] : input.totals_molal)
        program<<' '<<name<<' '<<total<<(name=="Na"&&input.balance_sodium?" charge":"")<<'\n';

    if (!all_phases.empty() || !all_gases.empty()) {
        program<<"EQUILIBRIUM_PHASES 1\n";
        for (const auto& [name, moles] : all_phases)
            program<<' '<<name<<" 0.0 "<<moles<<'\n';
        for (const auto& [name, logP] : all_gases)
            program<<' '<<name<<' '<<logP<<" 10.0\n";
    }

    program<<"SELECTED_OUTPUT 1\n -reset false\n -high_precision true\n -pH true\n -pe true\n -temperature true\n -ionic_strength true\n -charge_balance true\n -percent_error true\n";
    if (is_pitzer) {
        program<<" -totals Na Cl Ca Mg K C(4) S(6) Fe Mn Ba Sr Li Br B Si\n"
               <<" -molalities H+ OH- Na+ Cl- Ca+2 Mg+2 K+ SO4-2 HSO4- HCO3- CO3-2 Fe+2 Mn+2 Ba+2 Sr+2 Li+ Br-\n"
               <<" -activities H+ OH- Na+ Cl- Ca+2 Mg+2 K+ SO4-2 HCO3- CO3-2 Fe+2 Mn+2 Ba+2 Sr+2 Li+ Br-\n"
               <<" -saturation_indices Calcite Aragonite Dolomite Gypsum Anhydrite Halite Sylvite Barite Bischofite Mirabilite Thenardite Epsomite Arcanite";
    } else {
        program<<" -totals Na Cl Ca Mg K C(4) S(6) Fe Mn Al Ba Sr Si Cu Zn Pb Cd Li Br B F P N(5) N(-3)\n"
               <<" -molalities H+ OH- Na+ Cl- Ca+2 Mg+2 K+ SO4-2 HSO4- HCO3- CO3-2 Fe+2 Fe+3 Cu+2 Zn+2 Pb+2 Cd+2 Al+3 Ba+2 Sr+2\n"
               <<" -activities H+ OH- Na+ Cl- Ca+2 Mg+2 K+ SO4-2 HCO3- CO3-2 Fe+2 Fe+3 Cu+2 Zn+2 Pb+2 Al+3\n"
               <<" -saturation_indices Calcite Aragonite Dolomite Siderite Rhodochrosite Gypsum Anhydrite Halite Sylvite Barite Fluorite Quartz Gibbsite Kaolinite Hematite Goethite Pyrite Melanterite Smithsonite Sphalerite Cerussite Anglesite";
    }
    for (const auto& [p, _] : all_phases) {
        program<<' '<<p;
    }
    program<<"\nEND\n";

    if (RunString(id,program.str().c_str())!=0) { result.message=GetErrorString(id); return result; }
    const int last=GetSelectedOutputRowCount(id)-1,columns=GetSelectedOutputColumnCount(id);
    if (last<1 || columns<1) { result.message="IPhreeqc no devolvió resultados numéricos."; return result; }
    for (int column=0;column<columns;++column) {
        VAR heading,value; VarInit(&heading); VarInit(&value);
        const auto head_status=GetSelectedOutputValue(id,0,column,&heading);
        const auto value_status=GetSelectedOutputValue(id,last,column,&value);
        const bool numeric=value.type==TT_DOUBLE || value.type==TT_LONG;
        if (head_status==IPQ_OK && value_status==IPQ_OK && heading.type==TT_STRING && numeric) {
            const double number=value.type==TT_DOUBLE?value.dVal:static_cast<double>(value.lVal);
            if (std::isfinite(number)) result.values[heading.sVal]=number;
        }
        VarClear(&heading); VarClear(&value);
    }

    // Extraer actividades y coeficientes de actividad iónicos gamma = a / m
    std::map<std::string, double> activities_and_gammas;
    for (const auto& [name, value] : result.values) {
        if (name.rfind("la_", 0) == 0 && name.size() > 3) {
            const std::string species = name.substr(3);
            if (value > -500.0) {
                const double act = std::pow(10.0, value);
                activities_and_gammas["act_" + species] = act;
                const std::string m_key = "m_" + species + "(mol/kgw)";
                auto it = result.values.find(m_key);
                if (it != result.values.end() && it->second > 1e-15) {
                    activities_and_gammas["gamma_" + species] = act / it->second;
                }
            }
        }
    }
    for (const auto& [k, v] : activities_and_gammas) {
        result.values[k] = v;
    }

    result.full_output=GetOutputString(id);
    result.engine+="; version "+std::string(GetVersionString());
    result.ok=!result.values.empty();
    result.message=input.balance_sodium?"Balance de carga ajustando Na: el total de sodio puede cambiar.":"Carga no impuesta: revise el error de balance eléctrico.";
    if (!all_gases.empty()) result.message+=" Fases gaseosas: reservorio de 10 mol.";
    if (!all_phases.empty()) result.message+=" Fases minerales en equilibrio: precipitación/disolución resuelta.";
    const std::string warnings=GetWarningString(id);
    if (!warnings.empty()) result.message+="\nAdvertencias IPhreeqc:\n"+warnings;
#endif
    return result;
}
}
