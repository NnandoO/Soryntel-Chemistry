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
domain::AqueousResponse PhreeqcEngine::solve(const domain::AqueousRequest& input) const {
    domain::AqueousResponse result;
    result.engine="USGS IPhreeqc; revision f107fcb2f2e1e3167ce4c1328e477b54dee3d21b";
    result.database=input.model==domain::AqueousBackend::Pitzer?"pitzer.dat":"phreeqc.dat";
    if (!available()) { result.message="Motor no disponible. Configure CMA_WITH_IPHREEQC=ON y revise data/reference/phreeqc."; return result; }
    if (!std::isfinite(input.celsius) || input.celsius<0 || input.celsius>100 ||
        !std::isfinite(input.ph) || input.ph<0 || input.ph>14 ||
        !std::isfinite(input.calcite_moles) || input.calcite_moles<0 ||
        !std::isfinite(input.log_co2_pressure) || input.log_co2_pressure<-15 || input.log_co2_pressure>0 ||
        (input.balance_sodium && !input.totals_molal.count("Na"))) {
        result.message="Entradas inválidas: 0..100 °C, pH 0..14, fase no negativa, log PCO2 -15..0; balance de carga requiere Na."; return result;
    }
    const std::set<std::string> components={"Na","Cl","Ca","Mg","K","C(4)","S(6)"};
    for (const auto& [name,total] : input.totals_molal)
        if (!components.count(name) || !std::isfinite(total) || total<0 || total>10) {
            result.message="Componente no admitido o total fuera de 0..10 mol/kg."; return result;
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
    if (input.equilibrate_calcite || input.equilibrate_co2) {
        program<<"EQUILIBRIUM_PHASES 1\n";
        if (input.equilibrate_calcite) program<<" Calcite 0 "<<input.calcite_moles<<'\n';
        if (input.equilibrate_co2) program<<" CO2(g) "<<input.log_co2_pressure<<" 10\n";
    }
    program<<"SELECTED_OUTPUT 1\n -reset false\n -high_precision true\n -pH true\n -ionic_strength true\n -charge_balance true\n -percent_error true\n -totals Na Cl Ca Mg K C(4) S(6)\n"
              " -molalities H+ OH- Na+ Cl- Ca+2 HCO3- CO3-2\n -saturation_indices Calcite\nEND\n";
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
    result.full_output=GetOutputString(id);
    result.engine+="; version "+std::string(GetVersionString());
    result.ok=!result.values.empty();
    result.message=input.balance_sodium?"Balance de carga ajustando Na: el total de sodio puede cambiar.":"Carga no impuesta: revise el error de balance eléctrico.";
    if (input.equilibrate_co2) result.message+=" CO2: reservorio finito de 10 mol; revise en la salida si se agota.";
    const std::string warnings=GetWarningString(id);
    if (!warnings.empty()) result.message+="\nAdvertencias IPhreeqc:\n"+warnings;
#endif
    return result;
}
}
