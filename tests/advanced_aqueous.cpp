#include "adapters/scientific/PhreeqcEngine.hpp"
#include <cmath>
#include <iostream>
#include <stdexcept>
static void check(bool value,const char* label) { if (!value) throw std::runtime_error(label); }
int main(int argc,char** argv) {
    try {
        check(argc==2,"Database directory argument");
        cma::adapters::PhreeqcEngine engine(argv[1]);
        check(engine.available(),"IPhreeqc adapter available");
        // USGS PHREEQC example 3, part A: water + calcite + log PCO2=-2 at 25 C.
        cma::domain::AqueousRequest input;
        input.equilibrate_calcite=true; input.calcite_moles=10;
        input.equilibrate_co2=true;
        auto result=engine.solve(input);
        if (!result.ok) std::cerr<<result.message<<'\n';
        for (const auto& [name,value] : result.values) std::cout<<name<<'='<<value<<'\n';
        check(result.ok,"USGS calcite equilibrium calculation");
        check(std::abs(result.values.at("pH")-7.306)<.003,"USGS example pH reference");
        check(std::abs(result.values.at("Ca(mol/kgw)")-1.656e-3)<5e-6,"USGS example Ca reference");
        check(std::abs(result.values.at("si_Calcite"))<1e-7,"Calcite saturation reference");
        input={}; input.model=cma::domain::AqueousBackend::Pitzer;
        input.totals_molal={{"Na",1},{"Cl",1}};
        result=engine.solve(input);
        if (!result.ok) std::cerr<<result.message<<'\n';
        check(result.ok,"Pitzer NaCl speciation");
        check(std::abs(result.values.at("Na(mol/kgw)")-1)<1e-7,"Pitzer Na balance");
        check(std::abs(result.values.at("Cl(mol/kgw)")-1)<1e-7,"Pitzer Cl balance");
        input.totals_molal={{"Na\nEND",1}};
        check(!engine.solve(input).ok,"Reject PHREEQC input injection");
        input={}; input.ph=std::nan("");
        check(!engine.solve(input).ok,"Reject nonfinite PHREEQC input");
        std::cout<<"Advanced aqueous reference checks passed\n";
        return 0;
    } catch (const std::exception& error) { std::cerr<<error.what()<<'\n'; return 1; }
}
