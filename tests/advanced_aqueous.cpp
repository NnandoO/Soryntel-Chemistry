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
        check(result.values.count("gamma_Na+") && result.values.at("gamma_Na+") > 0.5 && result.values.at("gamma_Na+") < 0.8, "Pitzer Na+ activity coefficient reference");

        // Gypsum solubility in pure water: CaSO4:2H2O dissolution to equilibrium (approx 0.015 mol/kgw)
        input={}; input.model=cma::domain::AqueousBackend::IonAssociation;
        input.equilibrium_phases={{"Gypsum", 1.0}};
        result=engine.solve(input);
        check(result.ok, "Gypsum dissolution equilibrium");
        check(std::abs(result.values.at("si_Gypsum")) < 1e-6, "Gypsum saturation index is zero");
        check(result.values.at("Ca(mol/kgw)") > 0.013 && result.values.at("Ca(mol/kgw)") < 0.017, "Gypsum analytical solubility bound");
        check(std::abs(result.values.at("Ca(mol/kgw)") - result.values.at("S(6)(mol/kgw)")) < 1e-6, "Equimolar Ca and SO4 from gypsum dissolution");

        // Halite saturation in concentrated NaCl brine using Pitzer model
        input={}; input.model=cma::domain::AqueousBackend::Pitzer;
        input.totals_molal={{"Na", 6.0}, {"Cl", 6.0}};
        input.equilibrium_phases={{"Halite", 2.0}};
        result=engine.solve(input);
        check(result.ok, "Halite Pitzer saturation equilibrium");
        check(std::abs(result.values.at("si_Halite")) < 1e-6, "Halite saturation index is zero at equilibrium");
        check(result.values.at("Na(mol/kgw)") > 6.0 && result.values.at("Na(mol/kgw)") < 6.3, "Halite solubility bound in Pitzer (approx 6.16 mol/kgw)");

        // Multi-metal acidic solution (Cu, Zn, Fe with sulfate)
        input={}; input.model=cma::domain::AqueousBackend::IonAssociation;
        input.ph=2.5;
        input.totals_molal={{"Cu", 0.05}, {"Zn", 0.05}, {"Fe", 0.02}, {"S(6)", 0.15}};
        result=engine.solve(input);
        check(result.ok, "Multi-metal aqueous speciation (Cu, Zn, Fe, SO4)");
        check(std::abs(result.values.at("Cu(mol/kgw)") - 0.05) < 1e-6, "Cu conservation");
        check(std::abs(result.values.at("Zn(mol/kgw)") - 0.05) < 1e-6, "Zn conservation");
        check(std::abs(result.values.at("Fe(mol/kgw)") - 0.02) < 1e-6, "Fe conservation");
        check(result.values.count("gamma_Cu+2") && result.values.at("gamma_Cu+2") > 0, "Cu+2 activity coefficient calculated");

        // Injection and error rejection checks
        input.totals_molal={{"Na\nEND",1}};
        check(!engine.solve(input).ok,"Reject PHREEQC input injection");
        input={}; input.equilibrium_phases={{"Calcite; DROP TABLE", 1.0}};
        check(!engine.solve(input).ok,"Reject malicious phase name");
        input={}; input.ph=std::nan("");
        check(!engine.solve(input).ok,"Reject nonfinite PHREEQC input");
        std::cout<<"Advanced aqueous reference checks passed\n";
        return 0;
    } catch (const std::exception& error) { std::cerr<<error.what()<<'\n'; return 1; }
}
