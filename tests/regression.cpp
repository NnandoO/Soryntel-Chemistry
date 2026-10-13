#include "motores/simulacion_dinamica.hpp"
#include "nucleo/tabla_periodica.hpp"
#include "motores/reconciliacion_datos.hpp"
#include "motores/perdida_calor.hpp"
#include "motores/destilacion.hpp"
#include "motores/agua.hpp"
#include "motores/minimizador_gibbs.hpp"
#include "motores/balance_masa.hpp"
#include "motores/estimador_propiedades.hpp"
#include "sim/SimOptimization.hpp"
#include "tools/ExternalAPI.hpp"
#include "tools/DataFitter.hpp"
#include "estimation/BensonEstimator.hpp"
#include "diagrams/StabilityDiagrams.hpp"
#include "application/Pca.hpp"
#include "adapters/desktop/process.hpp"
#include "base_datos/gestor_db.hpp"
#include <sqlite3.h>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <cmath>
#include <chrono>

static int failures = 0, checks = 0;
#define CHECK(x) do { ++checks; if (!(x)) { ++failures; std::cerr << __LINE__ << ": " << #x << '\n'; } } while (0)
static bool near(double a, double b, double tolerance = 1e-9) { return std::isfinite(a) && std::abs(a-b) < tolerance; }
class MemoryRepository final : public cma::domain::ThermoRepository {
public:
    std::map<int64_t, cma::nucleo::Especie> species;
    const cma::nucleo::Especie* especie_por_id(int64_t id) const override {
        auto it = species.find(id); return it == species.end() ? nullptr : &it->second;
    }
    const cma::nucleo::Especie* especie_por_nombre(const std::string& name) const override {
        for (const auto& [id, sp] : species) if (sp.nombre == name) return &sp;
        return nullptr;
    }
    const cma::nucleo::Reaccion* reaccion_por_id(int64_t) const override { return nullptr; }
    const std::map<int64_t, cma::nucleo::Especie>& mapa_especies() const override { return species; }
};

int main() {
    using namespace cma::motores;
    const double nan = std::numeric_limits<double>::quiet_NaN();
    SimulacionDinamica ode;
    const auto time_derivative = [](double t, const auto&, auto& derivative) { derivative["y"] = t; };
    auto result = ode.resolver(0, 1, 0.3, {{"y", 0}}, time_derivative);
    CHECK(result.ok && near(result.tiempos.back(), 1) && near(result.variables.at("y").back(), 0.5));
    CHECK(!ode.resolver(0, 1, 0, {{"y", 0}}, time_derivative).ok);
    CHECK(!ode.resolver(0, 1, nan, {{"y", 0}}, time_derivative).ok);
    CHECK(!ode.resolver(0, 1, 0.1, {{"y", 0}}, [](double, const auto&, auto&) {}).ok);
    CHECK(ode.perfil_temperatura_horno(500, 300, 10, 10, 10, 1, 0).empty());
    CHECK(!ode.reactor_batch_dinamico(300, 1, 1, 1, {}, 1, 1, 0.1).ok);
    CHECK(!ode.reactor_batch_dinamico(0, 1, 1, 1, {1}, 1, 1, 0.1).ok);
    auto cooling = ode.perfil_temperatura_horno(500, 300, 10, 10, 10, 1, 0.1);
    CHECK(!cooling.empty() && near(cooling.back(), 300 + 200 * std::exp(-0.1), 1e-8));

    ReconciliacionDatos reconciliation;
    auto rec = reconciliation.reconciliar_balance_masa({{"in", 10, 1}}, {{"out", 8, 1}});
    CHECK(rec.ok && near(rec.valores_reconciliados.at("in"), 9) && near(rec.valores_reconciliados.at("out"), 9));
    CHECK(near(rec.q_ajuste, std::erfc(1)));
    CHECK(!reconciliation.reconciliar_balance_masa({{"same", 10, 1}}, {{"same", 8, 1}}).ok);
    CHECK(!reconciliation.reconciliar_balance_masa({{"in", nan, 1}}, {{"out", 8, 1}}).ok);
    CHECK(!reconciliation.reconciliar_balance_masa({{"in", 10, -1}}, {{"out", 8, 1}}).ok);
    CHECK(!reconciliation.reconciliar_balance_masa({{"in", 10, 1}}, {{"out", 8, 1}}, 0).ok);
    CHECK(reconciliation.detectar_outliers({{"a", 0, 1}, {"b", 0, 1}, {"c", 10, 1}}, 0.05).size() == 1);
    CHECK(reconciliation.detectar_outliers({{"a", 0, 1}, {"b", 0, 1}, {"c", 10, 1}}, 0).empty());

    PerdidaCalor heat;
    auto wall = heat.pared_plana(500, 300, 1, {{"wall", 0.1, 1}}, 10, 10);
    CHECK(wall.ok && wall.T_pared_ext > 300 && wall.T_pared_ext < 500);
    const double outside = 10 * (wall.T_pared_ext - 300) + 0.9 * 5.670374419e-8 * (std::pow(wall.T_pared_ext,4)-std::pow(300,4));
    CHECK(near(wall.flujo_calor, outside, 1e-6));
    CHECK(!heat.pared_plana(500, 300, 0, {}).ok);
    CHECK(!heat.pared_plana(500, 300, 1, {{"wall", -0.1, 1}}).ok);
    CHECK(!heat.tuberia(500, 300, 1, 0, {}).ok);
    auto sphere = heat.tanque_esferico(500, 300, 1, {});
    CHECK(sphere.ok && sphere.resistencia_total > 0 && std::isfinite(sphere.T_pared_ext));
    Agua water;
    auto saturation = water.saturacion_T(373.15);
    CHECK(saturation.second.v > 1.5 && saturation.second.v < 2);
    CHECK(water.propiedades(nan, 1).fase.find("Fuera") != std::string::npos);

    Destilacion dist;
    auto column = dist.mccabe_thiele(0.5, 0.95, 0.05, 2, 1, 3);
    CHECK(column.ok && near(column.R_min, 1.7) && near(column.destilado, 0.5));
    CHECK(!dist.mccabe_thiele(0.5, 0.5, 0.5, 2, 1, 3).ok);
    CHECK(!dist.mccabe_thiele(0.5, 0.95, 0.05, 2, 1, 1).ok);
    CHECK(dist.gilliland(10, 2, 1000) < dist.gilliland(10, 2, 3));
    cma::application::modules::SimOptimization optimization;
    cma::application::modules::DataFitter fitter;
    auto fit = fitter.lineal({300,400,500}, {601,801,1001});
    CHECK(fit.ok && near(fit.coeficientes[0],1,1e-7) && near(fit.coeficientes[1],2,1e-8));
    CHECK(!fitter.lineal({1,1,1},{2,3,4}).ok);
    CHECK(!fitter.polinomial({1,2},{3,4},-1).ok);
    CHECK(!fitter.lineal({1,nan},{2,3}).ok);
    auto variance = cma::application::explained_variance({{-1,-2},{0,0},{1,2}});
    CHECK(variance.size()==2 && near(variance[0],100,1e-8));
    CHECK(cma::application::explained_variance({{1,1},{1,1}}).empty());
    const auto objective = [](const auto& values) { return values.at("x") * values.at("x"); };
    CHECK(optimization.sensibilidad({{"x",0,1,0.5}}, "x", objective, 1).empty());
    CHECK(optimization.sensibilidad({{"x",0,1,0.5}}, "x", objective, -1).empty());
    CHECK(!optimization.optimizar({{"x",1,0,0.5}}, objective, cma::application::modules::MetodoOptimizacion::PSO).convergio);

    MemoryRepository memory;
    cma::nucleo::Especie oxygen; oxygen.id = 1; oxygen.formula = "O2"; oxygen.nombre = "Oxygen"; oxygen.masa_molar = 32; oxygen.cp_a = 30;
    memory.species[1] = oxygen;
    MinimizadorGibbs gibbs;
    CHECK(!gibbs.minimizar({{1,1}},300,0,memory.species).convergio);
    CHECK(!gibbs.minimizar({{2,1}},300,1,memory.species).convergio);
    CHECK(!gibbs.minimizar({{1,-1}},300,1,memory.species).convergio);
    EstimadorPropiedades estimates(memory);
    CHECK(near(estimates.estimar_hsc("O2",126.85,"Gas").h_db, oxygen.entalpia_abs(400)));
    BalanceMasa balance(memory);
    cma::diagrama_flujo::Flowsheet fs;
    CHECK(!balance.resolver(fs, 0).convergio);
    fs.agregar_nodo(cma::nucleo::TipoNodo::Reactor, "retained");
    const auto before = fs.a_json();
    std::string error;
    CHECK(!fs.desde_json("{bad json", error) && fs.a_json() == before);
    const auto flow = fs.agregar_flujo("retained flow");
    CHECK(!fs.desde_json("{\"nodos\":[{\"id\":1},{\"id\":1}],\"flujos\":[]}", error));
    CHECK(fs.flujo_por_id(flow) != nullptr);

    const auto tmp = std::filesystem::temp_directory_path() / ("cma-regression-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    std::filesystem::create_directory(tmp);
    cma::application::modules::ExternalAPI external;
    auto write = [&](const char* name, const char* contents) { std::ofstream out(tmp / name); out << contents; };
    write("valid.csv", "1,2\n3,4\n");
    write("invalid.csv", "1,2\n3,garbage\n");
    write("ragged.csv", "1,2\n3\n");
    std::vector<std::vector<double>> data;
    CHECK(external.importarCSV((tmp/"valid.csv").string(),data) && data.size()==2);
    const auto valid_data = data;
    CHECK(!external.importarCSV((tmp/"invalid.csv").string(),data) && data==valid_data);
    CHECK(!external.importarCSV((tmp/"ragged.csv").string(),data));
    CHECK(cma::adapters::dialog_pipe("touch /tmp/cma-should-never-exist") == nullptr);
    CHECK(cma::adapters::dialog_pipe("zenity --file-selection ; touch malicious") == nullptr);
    cma::base_datos::GestorDB repository;
    const int count = repository.conteo_especies();
    write("species.csv", "Nombre,Formula,Masa_Molar,Hf,Sf,Fase\nTestOxygen,O2,32,0,205,0\n");
    CHECK(repository.cargar_desde_csv((tmp/"species.csv").string()));
    CHECK(repository.cargar_desde_csv((tmp/"species.csv").string()) && repository.conteo_especies()==count+1);
    write("bad_species.csv", "Nombre,Formula,Masa_Molar,Hf,Sf,Fase\nNewSpecies,O2,32,0,205,0\nBad,O2,NaN,0,205,0\n");
    CHECK(!repository.cargar_desde_csv((tmp/"bad_species.csv").string()) && repository.conteo_especies()==count+1);
    CHECK(repository.exportar_sqlite((tmp/"species.sqlite").string()));
    sqlite3* sql = nullptr;
    CHECK(sqlite3_open((tmp/"species.sqlite").string().c_str(), &sql)==SQLITE_OK);
    sqlite3_stmt* statement = nullptr;
    CHECK(sqlite3_prepare_v2(sql,"SELECT count(*) FROM cma_species",-1,&statement,nullptr)==SQLITE_OK);
    CHECK(sqlite3_step(statement)==SQLITE_ROW && sqlite3_column_int(statement,0)==repository.conteo_especies());
    sqlite3_finalize(statement);
    CHECK(sqlite3_prepare_v2(sql,"SELECT count(*) FROM cma_species_provenance WHERE quality=0 AND min_kelvin IS NULL",-1,&statement,nullptr)==SQLITE_OK);
    CHECK(sqlite3_step(statement)==SQLITE_ROW && sqlite3_column_int(statement,0)==repository.conteo_especies());
    sqlite3_finalize(statement); sqlite3_close(sql);
    std::filesystem::remove_all(tmp);

    const auto& periodic_table = cma::nucleo::TablaPeriodica::instancia();
    auto p_gypsum = periodic_table.analizar_formula("CaSO4·2H2O");
    CHECK(!p_gypsum.empty());
    auto p_lead = periodic_table.analizar_formula("2PbO*PbSO4");
    CHECK(!p_lead.empty());
    auto p_ion = periodic_table.analizar_formula("SO4²⁻");
    CHECK(!p_ion.empty());
    auto p_polymer = periodic_table.analizar_formula("(C8H8)n");
    CHECK(!p_polymer.empty());

    // Circuito cerrado de procesamiento mineral:
    // Alimentación -> Mezclador -> MolinoBolas -> Hidrociclon -> Flotación -> Conc & Relave
    // Reciclo: Hidrociclon Underflow -> Mezclador
    cma::diagrama_flujo::Flowsheet min_fs;
    auto n_feed = min_fs.agregar_nodo(cma::nucleo::TipoNodo::Fuente, "Feed");
    auto n_mix = min_fs.agregar_nodo(cma::nucleo::TipoNodo::Mezclador, "Mix");
    auto n_mill = min_fs.agregar_nodo(cma::nucleo::TipoNodo::MolinoBolas, "BallMill");
    auto n_cyclone = min_fs.agregar_nodo(cma::nucleo::TipoNodo::Hidrociclon, "Cyclone");
    auto n_float = min_fs.agregar_nodo(cma::nucleo::TipoNodo::CeldaFlotacion, "Flotation");
    auto n_conc = min_fs.agregar_nodo(cma::nucleo::TipoNodo::Sumidero, "Concentrate");
    auto n_tail = min_fs.agregar_nodo(cma::nucleo::TipoNodo::Sumidero, "Tailings");

    auto* feed_node = min_fs.nodo_por_id(n_feed);
    feed_node->params["mineral_0_flow"] = 100000.0; // 100 t/h mineral
    feed_node->params["mineral_0_d"] = 5000.0;     // 5000 um
    feed_node->params["agua_kg_h"] = 35000.0;       // 35 t/h agua

    auto* mill_node = min_fs.nodo_por_id(n_mill);
    mill_node->params["wi"] = 14.0;
    mill_node->params["power"] = 3000.0;

    auto* cyc_node = min_fs.nodo_por_id(n_cyclone);
    cyc_node->params["d50"] = 120.0;
    cyc_node->params["sharpness"] = 2.5;
    cyc_node->params["bypass"] = 0.1;
    cyc_node->params["water_to_coarse"] = 0.2;

    auto* fl_node = min_fs.nodo_por_id(n_float);
    fl_node->params["residence_time"] = 12.0;
    fl_node->params["water_recovery"] = 0.1;
    fl_node->params["rate"] = 0.35;
    fl_node->params["recovery"] = 0.90;

    min_fs.conectar(n_feed, n_mix);
    min_fs.conectar(n_mix, n_mill);
    min_fs.conectar(n_mill, n_cyclone);

    auto s_of = min_fs.agregar_flujo("Cyclone_OF");
    min_fs.conectar(n_cyclone, n_float, s_of);
    auto s_uf = min_fs.agregar_flujo("Cyclone_UF_Recycle");
    min_fs.conectar(n_cyclone, n_mix, s_uf);

    auto s_conc = min_fs.agregar_flujo("Flot_Conc");
    min_fs.conectar(n_float, n_conc, s_conc);
    auto s_tail = min_fs.agregar_flujo("Flot_Tails");
    min_fs.conectar(n_float, n_tail, s_tail);

    cma::motores::BalanceMasa min_bal(memory);
    auto min_res = min_bal.resolver(min_fs, 200, 1e-6);
    CHECK(min_res.convergio);
    CHECK(min_res.iteraciones > 1);

    const cma::nucleo::Flujo *f_conc = nullptr, *f_tail = nullptr;
    for (const auto& f : min_res.flujos_salida) {
        if (f.id == s_conc) f_conc = &f;
        if (f.id == s_tail) f_tail = &f;
    }
    CHECK(f_conc && f_tail);
    double solids_out = 0.0, water_out = 0.0;
    if (f_conc) {
        for (const auto& p : f_conc->particulas) solids_out += p.mass_kg_h;
        water_out += f_conc->agua_kg_h;
    }
    if (f_tail) {
        for (const auto& p : f_tail->particulas) solids_out += p.mass_kg_h;
        water_out += f_tail->agua_kg_h;
    }
    CHECK(std::abs(solids_out - 100000.0) / 100000.0 < 1e-5);
    CHECK(std::abs(water_out - 35000.0) / 35000.0 < 1e-5);

    // Estimación por aditividad de grupos de Benson (Benson 1976 / Cohen & Benson 1993)
    cma::application::modules::BensonEstimator benson_est;
    auto b_c6h6 = benson_est.estimarBenson("C6H6");
    CHECK(b_c6h6.ok);
    CHECK(near(b_c6h6.h_formacion, 82.86, 0.05)); // 6 x 13.81 kJ/mol
    CHECK(b_c6h6.mensaje.find("Benson") != std::string::npos);

    auto b_c2h6 = benson_est.estimarBenson("C2H6");
    CHECK(b_c2h6.ok);
    CHECK(near(b_c2h6.h_formacion, -84.34, 0.05)); // 2 x -42.17 kJ/mol

    auto b_c3h8 = benson_est.estimarBenson("C3H8");
    CHECK(b_c3h8.ok);
    CHECK(near(b_c3h8.h_formacion, -105.26, 0.05)); // 2 x (-42.17) + (-20.92)

    auto b_ch4 = benson_est.estimarBenson("CH4");
    CHECK(b_ch4.ok);
    CHECK(near(b_ch4.h_formacion, -74.87, 0.05));

    // Diagramas de estabilidad de Kellogg (predominancia pirometalúrgica)
    cma::application::modules::StabilityDiagrams stab_solver;
    auto cu_points = stab_solver.generar("Cu", 900.0);
    CHECK(!cu_points.empty());
    bool has_cu = false, has_cu2s = false, has_cuso4 = false;
    for (const auto& pt : cu_points) {
        if (pt.fase == "Cu") has_cu = true;
        if (pt.fase == "Cu2S") has_cu2s = true;
        if (pt.fase.find("CuSO4") != std::string::npos) has_cuso4 = true;
    }
    CHECK(has_cu && has_cu2s && has_cuso4);

    auto fe_points = stab_solver.generar("Fe", 1000.0);
    CHECK(!fe_points.empty());
    bool has_fe = false, has_fes = false, has_feso4 = false, has_oxide = false;
    for (const auto& pt : fe_points) {
        if (pt.fase == "Fe") has_fe = true;
        if (pt.fase == "FeS") has_fes = true;
        if (pt.fase.find("FeSO4") != std::string::npos) has_feso4 = true;
        if (pt.fase == "Fe3O4" || pt.fase == "Fe2O3" || pt.fase == "FeO") has_oxide = true;
    }
    CHECK(has_fe && has_fes && has_feso4 && has_oxide);

    // Comprobación de límites: rechazo de temperaturas no físicas o metales desconocidos
    CHECK(stab_solver.generar("Cu", 200.0).empty()); // T < 300 K
    CHECK(stab_solver.generar("Cu", 3000.0).empty()); // T > 2500 K
    CHECK(stab_solver.generar("Inexistente", 900.0).empty());

    std::cout << checks << " checks; " << failures << " failures\n";
    return failures ? 1 : 0;
}
