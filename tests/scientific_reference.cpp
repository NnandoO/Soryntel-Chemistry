#include "motores/agua.hpp"
#include "motores/especiacion_acuosa.hpp"
#include "motores/mineral_processing.hpp"
#include "adapters/persistence/MinteqImporter.hpp"
#include "application/FlotationCalibration.hpp"
#include <cmath>
#include <iostream>
#include <stdexcept>
static void check(bool value,const char* label) { if (!value) throw std::runtime_error(label); }
static bool near(double a,double b,double relative=3e-6) { return std::isfinite(a)&&std::abs(a-b)<=relative*std::max(1.0,std::abs(b)); }
int main(int argc,char** argv) {
    try {
        cma::motores::Agua water;
        // IAPWS R7-97(2012), tables 5, 15, 33 and 42. Units K, MPa, kJ/kg.
        struct Point { double t,p,h,s,cp; };
        for (const auto& x : {Point{300,3,115.331273,.392294792,4.17301218},
                Point{700,30,2631.49474,5.17540298,10.3505092},
                Point{650,25.5837018,1863.43019,4.05427273,13.8935717},
                Point{1500,.5,5219.76855,9.65408875,2.61609445}}) {
            const auto r=water.propiedades(x.t,x.p);
            if (!r.ok) std::cerr<<r.mensaje<<'\n';
            check(r.ok&&near(r.h,x.h)&&near(r.s,x.s)&&near(r.cp,x.cp),"IF97 reference point");
        }
        check(near(water.temperatura_saturacion(.1),372.755919),"IF97 saturation temperature");
        check(near(water.presion_saturacion(500),2.63889776),"IF97 saturation pressure");
        check(!water.propiedades(-1,1).ok,"IF97 invalid temperature");
        const auto endpoints=water.saturacion_T(373.15);
        const auto mix=water.mezcla_saturada(water.presion_saturacion(373.15),.5);
        check(mix.ok&&near(mix.h,(endpoints.first.h+endpoints.second.h)/2),"Steam quality balance");
        cma::domain::AqueousDatabase database;
        database.components[10]={10,"A",0,1};
        cma::domain::FormationReaction reaction;
        reaction.id=20; reaction.name="A2"; reaction.components={{10,2}}; reaction.log_k_298=0;
        database.reactions[20]=reaction;
        cma::motores::SpeciationInput input;
        input.fixed_log_activities.clear(); input.totals_molal={{10,1}};
        const auto result=cma::motores::AqueousSpeciation(database).solve(input);
        check(result.ok&&near(result.free_components.at(10),.5)&&near(result.dissolved_species.at(20),.25),"Analytical dimer equilibrium");
        cma::motores::ParticleStream feed{{{"ore",100,100}},200};
        const auto split=cma::motores::MineralProcessing::classify(feed,100,2,0,.25);
        check(split.ok&&near(split.product.particles.at(0).mass_kg_h,50)&&near(split.mass_residual_kg_h,0),"Classifier conservation");
        const auto flotation=cma::motores::MineralProcessing::flotation(feed,{{"ore",{1,.8}}},1,0);
        check(flotation.ok&&near(flotation.product.particles.at(0).mass_kg_h,80*(1-std::exp(-1))),"First order flotation");
        check(near(cma::motores::MineralProcessing::bond_energy_kWh_t(10,10000,100),9),"Bond energy units");
        check(!cma::motores::MineralProcessing::classify(feed,0,2,0,.25).ok,"Invalid classifier diameter");
        check(!cma::motores::MineralProcessing::flotation(feed,{},1,0).ok,"No invented kinetic parameters");
        check(!cma::motores::MineralProcessing::flotation(feed,{{"ore",{1,.8}}},-1,0).ok,"Invalid residence time");
        std::vector<std::vector<double>> measurements;
        for (double time : {0.,1.,2.,3.,4.}) measurements.push_back({time,.8*(1-std::exp(-.3*time))});
        const auto fit=cma::application::fit_flotation(measurements,.8);
        check(fit.ok&&near(fit.rate_per_min,.3)&&fit.recovery_rmse<1e-12,"Analytical kinetic fit (synthetic reference)");
        check(!cma::application::fit_flotation({{0,0},{1,.8},{2,.9}},.8).ok,"Recovery cannot exceed asymptote");
        check(!cma::application::fit_flotation({{0,0},{0,0},{0,0}},.8).ok,"No informative time series");
        if (argc>1) {
            cma::adapters::MinteqImporter importer;
            if (!importer.load(argv[1],database)) throw std::runtime_error(importer.error());
            check(database.components.count(150)&&database.reactions.count(5015001),"Native MINTEQ records");
            cma::motores::SpeciationInput native;
            native.totals_molal={{140,.001},{150,.001},{180,.012},{500,.01}};
            const auto solution=cma::motores::AqueousSpeciation(database).solve(native);
            check(solution.ok&&solution.mass_residual<1e-8,"Native carbonate mass balances");
            native.kelvin=310;
            check(!cma::motores::AqueousSpeciation(database).solve(native).ok,"No unverified MINTEQ thermal extrapolation");
            const auto count=database.reactions.size();
            check(!importer.load("/nonexistent-cma-catalog",database)&&database.reactions.size()==count,"Atomic failed import");
            std::cout<<"MINTEQ: "<<database.components.size()<<" components, "<<count<<" reactions\n";
        }
        std::cout<<"Scientific reference checks passed\n";
        return 0;
    } catch (const std::exception& error) { std::cerr<<error.what()<<'\n'; return 1; }
}
