#include "motores/simulacion_dinamica.hpp"
#include "nucleo/tabla_periodica.hpp"
#include "motores/reconciliacion_datos.hpp"
#include "motores/perdida_calor.hpp"
#include "motores/pourbaix.hpp"
#include "motores/destilacion.hpp"
#include "motores/agua.hpp"
#include "motores/minimizador_gibbs.hpp"
#include "motores/balance_masa.hpp"
#include "motores/balance_hoja.hpp"
#include "motores/estimador_propiedades.hpp"
#include "motores/red_neuronal.hpp"
#include "motores/lca.hpp"
#include "sim/SimOptimization.hpp"
#include "tools/ExternalAPI.hpp"
#include "tools/DataFitter.hpp"
#include "tools/HeatLossCalc.hpp"
#include "estimation/BensonEstimator.hpp"
#include "diagrams/StabilityDiagrams.hpp"
#include "diagrams/PropertyDiagrams.hpp"
#include "application/Pca.hpp"
#include "adapters/desktop/process.hpp"
#include "base_datos/gestor_db.hpp"
#include "application/CatalogAudit.hpp"
#include "db/MaterialDB.hpp"
#include "db/MineralDB.hpp"
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

    // Verificación de recarga e importación SQLite con preservación de ModeloCp
    cma::base_datos::GestorDB loaded_repo;
    CHECK(loaded_repo.importar_sqlite((tmp/"species.sqlite").string()));
    CHECK(loaded_repo.conteo_especies() == repository.conteo_especies());
    const auto* sp_o2_loaded = loaded_repo.especie_por_nombre("Oxígeno");
    CHECK(sp_o2_loaded != nullptr);
    if (sp_o2_loaded) {
        CHECK(sp_o2_loaded->modelo_cp == cma::nucleo::ModeloCp::POLINOMIAL_ESTANDAR);
        CHECK(near(sp_o2_loaded->T_min, 298.15, 1e-3));
        CHECK(near(sp_o2_loaded->T_max, 2000.0, 1e-3));
    }
    const cma::nucleo::Especie* sp_li2o_loaded = nullptr;
    for (const auto& [id, sp] : loaded_repo.mapa_especies()) {
        if (sp.formula == "Li2O(s)") {
            sp_li2o_loaded = &sp;
            break;
        }
    }
    CHECK(sp_li2o_loaded != nullptr);
    if (sp_li2o_loaded) {
        CHECK(sp_li2o_loaded->modelo_cp == cma::nucleo::ModeloCp::MAIER_KELLEY);
    }

    // Verificación de migración compatible de base de datos SQLite legacy (11 columnas)
    {
        sqlite3* leg_sql = nullptr;
        std::string leg_path = (tmp/"legacy_11col.sqlite").string();
        CHECK(sqlite3_open(leg_path.c_str(), &leg_sql) == SQLITE_OK);
        sqlite3_exec(leg_sql,
            "CREATE TABLE cma_species (id INTEGER PRIMARY KEY, name TEXT, formula TEXT, molar_mass REAL, "
            "hf REAL, sf REAL, phase INTEGER, cp_a REAL, cp_b REAL, cp_c REAL, cp_d REAL);"
            "CREATE TABLE cma_species_provenance (id INTEGER PRIMARY KEY, source TEXT, reference TEXT, "
            "revision TEXT, quality INTEGER, min_kelvin REAL, max_kelvin REAL);"
            "INSERT INTO cma_species VALUES (1, 'LegacyGas', 'O2', 31.999, 0.0, 205.15, 1, 29.0, 0.01, -1e-6, 0.0);"
            "INSERT INTO cma_species VALUES (2, 'LegacyMineral', 'FeS', 87.91, -100.0, 60.0, 2, 45.0, 0.02, 50000.0, 0.0);",
            nullptr, nullptr, nullptr);
        sqlite3_close(leg_sql);

        cma::base_datos::GestorDB leg_repo;
        CHECK(leg_repo.importar_sqlite(leg_path));
        CHECK(leg_repo.conteo_especies() == 2);
        const auto* leg_sp1 = leg_repo.especie_por_id(1);
        const auto* leg_sp2 = leg_repo.especie_por_id(2);
        CHECK(leg_sp1 != nullptr && leg_sp1->modelo_cp == cma::nucleo::ModeloCp::POLINOMIAL_ESTANDAR);
        CHECK(leg_sp2 != nullptr && leg_sp2->modelo_cp == cma::nucleo::ModeloCp::MAIER_KELLEY);
    }
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

    // ── Balance de calor y materiales (motor de la hoja tipo HSC) ────────────
    CHECK(near(energia_a_kJ(1.0, UnidadEnergia::Mcal), 4184.0));
    CHECK(near(energia_desde_kJ(3600.0, UnidadEnergia::kWh), 1.0, 1e-12));
    CHECK(near(temperatura_a_K(25.0, UnidadTemperatura::Celsius), 298.15, 1e-9));
    CHECK(near(temperatura_desde_K(298.15, UnidadTemperatura::Celsius), 25.0, 1e-9));

    cma::base_datos::GestorDB hmb_db;
    auto resolver_hmb = [&](const std::string& etiqueta) -> const cma::nucleo::Especie* {
        for (const auto* e : hmb_db.todas_especies())
            if (e->formula == etiqueta || e->nombre == etiqueta) return e;
        return nullptr;
    };
    auto fila_hmb = [](const char* formula, double T_K, double kmol) {
        FilaBalance f;
        f.formula = formula;
        f.temperatura_K = T_K;
        f.kmol = kmol;
        return f;
    };

    BalanceHoja hmb;

    // Agua gaseosa: entalpía sensible nula a 298.15 K y formación conocida
    AreaBalance agua;
    CorrienteBalance agua_in;
    agua_in.filas.push_back(fila_hmb("H2O(g)", 298.15, 1.0));
    agua.entradas.push_back(agua_in);
    agua.salidas.push_back(agua_in);
    hmb.actualizar(agua, resolver_hmb);
    auto r_agua = hmb.evaluar(agua);
    CHECK(r_agua.balance_masa_ok);
    CHECK(r_agua.balance_energia_ok);
    CHECK(near(agua.entradas[0].filas[0].h_sensible_kJ, 0.0, 1e-9));
    CHECK(near(agua.entradas[0].filas[0].h_total_kJ, -241826.0, 1.0));
    CHECK(near(agua.entradas[0].filas[0].masa_kg, 18.015, 1e-3));

    // Aporte de 100 kJ → la temperatura de equilibrio sube
    agua.salidas[0].calor_kJ = -100.0;
    auto sol_agua = hmb.temperatura_equilibrio(agua, false);
    CHECK(sol_agua.factible && sol_agua.convergio);
    CHECK(sol_agua.temperatura_K > 298.15);
    const double h_salida_agua =
        BalanceHoja::entalpia_fila(agua.salidas[0].filas[0], sol_agua.temperatura_K);
    CHECK(near(h_salida_agua - agua.entradas[0].filas[0].h_total_kJ, 100.0, 1e-3));

    // Pérdida de 50 kJ → la temperatura de equilibrio baja
    agua.salidas[0].calor_kJ = 50.0;
    auto sol_frio = hmb.temperatura_equilibrio(agua, false);
    CHECK(sol_frio.factible && sol_frio.temperatura_K < 298.15);

    // Combustión estequiométrica de CH4 con aire (2 O2 + 7.52 N2)
    AreaBalance comb;
    CorrienteBalance comb_gas;
    comb_gas.filas.push_back(fila_hmb("CH4", 298.15, 1.0));
    CorrienteBalance comb_aire;
    comb_aire.filas.push_back(fila_hmb("O2", 298.15, 2.0));
    comb_aire.filas.push_back(fila_hmb("N2", 298.15, 7.52));
    comb.entradas.push_back(comb_gas);
    comb.entradas.push_back(comb_aire);
    CorrienteBalance comb_humos;
    comb_humos.filas.push_back(fila_hmb("CO2", 298.15, 1.0));
    comb_humos.filas.push_back(fila_hmb("H2O(g)", 298.15, 2.0));
    comb_humos.filas.push_back(fila_hmb("N2", 298.15, 7.52));
    comb.salidas.push_back(comb_humos);

    hmb.actualizar(comb, resolver_hmb);
    auto r_comb = hmb.evaluar(comb);
    CHECK(r_comb.balance_masa_ok);
    CHECK(r_comb.filas_sin_especie == 0);
    for (const auto& e : hmb.balance_elemental(comb))
        CHECK(std::abs(e.error_pct()) < 1e-6);

    // Temperatura adiabática de llama (sin disociación)
    auto llama = hmb.temperatura_equilibrio(comb, false);
    CHECK(llama.factible && llama.convergio);
    CHECK(llama.temperatura_K > 2000.0 && llama.temperatura_K < 2700.0);

    // El precalentamiento de reactivos eleva la temperatura de llama
    comb.entradas[0].filas[0].temperatura_K = 600.0;
    hmb.actualizar(comb, {});
    auto llama_pre = hmb.temperatura_equilibrio(comb, false);
    CHECK(llama_pre.temperatura_K > llama.temperatura_K);

    // Ajuste de calor: cerrar la hoja a las temperaturas actuales (25 °C)
    for (auto& f : comb.salidas[0].filas) f.temperatura_K = 298.15;
    hmb.actualizar(comb, {});
    const double calor_cierre = hmb.ajustar_calor_de_area(comb);
    auto r_cierre = hmb.evaluar(comb);
    CHECK(near(r_cierre.residual_energia_kJ, 0.0, 1e-6));
    CHECK(calor_cierre > 0.0);  // la combustión libera calor que debe retirarse

    // Calor requerido para llevar los productos a 100 °C
    comb.salidas[0].calor_kJ = 0.0;
    hmb.actualizar(comb, {});
    const double q_100 = hmb.calor_requerido(comb, 373.15, false);
    CHECK(q_100 > 0.0 && q_100 < calor_cierre);

    // Exergía química estándar (Szargut)
    CHECK(near(BalanceHoja::exergia_quimica_estandar(comb.entradas[0].filas[0]),
               831.65, 1e-9));
    CHECK(near(comb.entradas[0].filas[0].exergia_quimica_kJ, 831650.0, 1.0));

    // Verificación de referencia HSC Bal: H2O(l) 100 kg a 90 °C (363.15 K)
    {
        FilaBalance fw;
        fw.formula = "H2O(l)";
        fw.especie = resolver_hmb("H2O(l)");
        fw.temperatura_K = 363.15;
        fw.fijar_masa_kg(100.0);
        AreaBalance aw;
        CorrienteBalance cw;
        cw.filas.push_back(fw);
        aw.entradas.push_back(cw);
        aw.salidas.push_back(cw);
        hmb.actualizar(aw, resolver_hmb);
        const auto& rf = aw.salidas[0].filas[0];
        CHECK(near(rf.kmol, 5.5509, 1e-3));
        CHECK(near(rf.volumen_nm3, 0.100, 1e-3));
        CHECK(near(cma::motores::energia_desde_kJ(rf.h_sensible_kJ, cma::motores::UnidadEnergia::kWh), 7.56, 0.05));
        CHECK(near(cma::motores::energia_desde_kJ(rf.h_total_kJ, cma::motores::UnidadEnergia::kWh), -433.16, 0.25));
    }

    // Corriente con temperatura_fija (Cooling Water Out a 90 °C fija mientras se resuelve Gas Out)
    {
        AreaBalance comb_cw = comb;
        for (auto& f : comb_cw.entradas[0].filas) f.temperatura_K = 298.15;
        CorrienteBalance cw_in;
        cw_in.nombre = "Cooling Water In";
        cw_in.filas.push_back(fila_hmb("H2O(l)", 298.15, 5.5509));
        comb_cw.entradas.push_back(cw_in);

        CorrienteBalance cw_out = cw_in;
        cw_out.nombre = "Cooling Water Out";
        cw_out.temperatura_fija = true;
        cw_out.filas[0].temperatura_K = 363.15; // 90 °C fija
        comb_cw.salidas.push_back(cw_out);

        hmb.actualizar(comb_cw, resolver_hmb);
        auto sol_cw = hmb.temperatura_equilibrio(comb_cw, false);
        CHECK(sol_cw.factible && sol_cw.convergio);
        CHECK(near(comb_cw.salidas[1].filas[0].temperatura_K, 363.15, 1e-6)); // CW Out permanece a 90 °C
        CHECK(comb_cw.salidas[0].filas[0].temperatura_K > 1500.0);            // Gas Out se resuelve
    }

    // Verificación de referencia exacta HSC Bal: 12 Nm³ CH4 + aire + 100 kg agua de enfriamiento (90 °C)
    {
        AreaBalance hsc_ref;
        CorrienteBalance fuel_in;
        fuel_in.nombre = "Fuel Gas (CH4)";
        fuel_in.filas.push_back(fila_hmb("CH4", 298.15, 12.0 * (1.000 / 22.414)));
        hsc_ref.entradas.push_back(fuel_in);

        CorrienteBalance air_in;
        air_in.nombre = "Combustion Air";
        air_in.filas.push_back(fila_hmb("O2", 298.15, 12.0 * (3.600 / 22.414)));
        air_in.filas.push_back(fila_hmb("N2", 298.15, 12.0 * (13.543 / 22.414)));
        hsc_ref.entradas.push_back(air_in);

        CorrienteBalance cw_in;
        cw_in.nombre = "Cooling Water In";
        cw_in.filas.push_back(fila_hmb("H2O(l)", 298.15, 100.0 / 18.015));
        hsc_ref.entradas.push_back(cw_in);

        CorrienteBalance gas_out;
        gas_out.nombre = "Gas Out";
        gas_out.filas.push_back(fila_hmb("CO2", 298.15, 12.0 * (1.000 / 22.414)));
        gas_out.filas.push_back(fila_hmb("H2O(g)", 298.15, 12.0 * (2.000 / 22.414)));
        gas_out.filas.push_back(fila_hmb("N2", 298.15, 12.0 * (13.543 / 22.414)));
        gas_out.filas.push_back(fila_hmb("O2", 298.15, 12.0 * (1.600 / 22.414)));
        hsc_ref.salidas.push_back(gas_out);

        CorrienteBalance cw_out;
        cw_out.nombre = "Cooling Water Out";
        cw_out.temperatura_fija = true;
        cw_out.filas.push_back(fila_hmb("H2O(l)", 363.15, 100.0 / 18.015));
        hsc_ref.salidas.push_back(cw_out);

        hmb.actualizar(hsc_ref, resolver_hmb);
        auto sol_hsc = hmb.temperatura_equilibrio(hsc_ref, false);
        CHECK(sol_hsc.factible && sol_hsc.convergio);
        const double T_gas_C = sol_hsc.temperatura_K - 273.15;
        CHECK(near(T_gas_C, 1238.938, 0.25));
        const auto res_hsc = hmb.evaluar(hsc_ref);
        CHECK(near(cma::motores::energia_desde_kJ(
                       res_hsc.h_sensible_salida_kJ - res_hsc.h_sensible_entrada_kJ,
                       cma::motores::UnidadEnergia::kWh),
                   119.355, 0.05));
        CHECK(near(cma::motores::energia_desde_kJ(
                       res_hsc.residual_energia_kJ,
                       cma::motores::UnidadEnergia::kWh),
                   0.0, 1e-4));
    }

    // Solver de temperatura óptima de precalentamiento y escalado industrial
    {
        AreaBalance endo;
        CorrienteBalance in_co;
        in_co.filas.push_back(fila_hmb("CO", 298.15, 3.0));
        in_co.filas.push_back(fila_hmb("N2", 298.15, 6.0));
        endo.entradas.push_back(in_co);
        CorrienteBalance out_co;
        out_co.filas.push_back(fila_hmb("CO", 900.0, 3.0));
        out_co.filas.push_back(fila_hmb("N2", 900.0, 6.0));
        endo.salidas.push_back(out_co);
        hmb.actualizar(endo, resolver_hmb);
        auto pre = hmb.temperatura_precalentamiento(endo, 0);
        CHECK(pre.factible && pre.convergio);
        CHECK(near(pre.temperatura_K, 900.0, 1e-2));

        hmb.escalar_area(endo, 10.0);
        auto r_esc = hmb.evaluar(endo);
        CHECK(near(r_esc.kmol_entrada, 90.0, 1e-6));
        CHECK(r_esc.balance_masa_ok && r_esc.balance_energia_ok);
    }

    // ── RedNeuronal (Dat - Data Module: FFNN, PSO, RNN, Normalización, JSON/XML) ──
    {
        using cma::motores::RedNeuronal;
        using cma::motores::ConfigANN;

        // 1. FFNN con datos de gran escala estilo HSC (X in [-1000, 1000], Y in [0, 2.7e7])
        std::vector<std::vector<double>> X_hsc;
        std::vector<std::vector<double>> Y_hsc;
        std::vector<int> idx_ent_hsc, idx_val_hsc;
        std::mt19937 rng_hsc(1337);
        std::uniform_real_distribution<double> dist_hsc(-1000.0, 1000.0);
        for (int i = 0; i < 120; ++i) {
            std::vector<double> x(8);
            double y = 0.0;
            for (size_t k = 0; k < 8; ++k) {
                x[k] = dist_hsc(rng_hsc);
                y += static_cast<double>(k + 1) * x[k] * x[k];
            }
            X_hsc.push_back(x);
            Y_hsc.push_back({y});
            if (i < 100) idx_ent_hsc.push_back(i);
            else idx_val_hsc.push_back(i);
        }

        ConfigANN cfg_ffnn;
        cfg_ffnn.ocultas = {16, 16};
        cfg_ffnn.act_oculta = 0; // TanH
        cfg_ffnn.act_salida = 0; // TanH
        cfg_ffnn.tasa = 0.025;
        cfg_ffnn.batch_size = 1;
        cfg_ffnn.tipo = 0; // FFNN
        cfg_ffnn.normalizar = true;

        std::atomic<bool> no_cancel{false};
        RedNeuronal net_ffnn;
        net_ffnn.inicializar(8, cfg_ffnn, 1);
        auto res_ffnn = net_ffnn.entrenar(X_hsc, Y_hsc, idx_ent_hsc, idx_val_hsc,
                                          1e-7, 350, nullptr, no_cancel);
        CHECK(res_ffnn.ok);
        CHECK(!res_ffnn.mse_ent_hist.empty());
        CHECK(res_ffnn.mse_ent < res_ffnn.mse_ent_hist.front() * 0.15);
        CHECK(res_ffnn.mse_ent < 0.05);

        // Verificar predicción normalizada en rango [-1, 1] y predicción física > 0
        auto pred_norm = net_ffnn.predecir_normalizado(X_hsc[0]);
        auto pred_phys = net_ffnn.predecir(X_hsc[0]);
        CHECK(pred_norm.size() == 1 && std::abs(pred_norm[0]) <= 1.0);
        CHECK(pred_phys.size() == 1 && std::isfinite(pred_phys[0]));

        // Serialización JSON round-trip (incluyendo escaladores de normalización)
        std::string json_str = net_ffnn.a_json();
        CHECK(!json_str.empty());
        RedNeuronal net_loaded;
        CHECK(net_loaded.desde_json(json_str));
        auto pred_loaded = net_loaded.predecir(X_hsc[0]);
        CHECK(near(pred_phys[0], pred_loaded[0], 1e-6));

        // Exportación XML estilo HSC
        std::string xml_str = net_ffnn.a_xml();
        CHECK(xml_str.find("<NeuralNetwork") != std::string::npos);
        CHECK(xml_str.find("<Normalization") != std::string::npos);

        // 2. PSO (tipo = 1) con offset no nulo (verifica adaptación de sesgos/biases)
        std::vector<std::vector<double>> X_pso;
        std::vector<std::vector<double>> Y_pso;
        std::vector<int> idx_pso;
        for (int i = -10; i <= 10; ++i) {
            double x = i * 10.0;
            X_pso.push_back({x});
            Y_pso.push_back({x * x + 500.0});
            idx_pso.push_back(static_cast<int>(X_pso.size()) - 1);
        }
        ConfigANN cfg_pso;
        cfg_pso.ocultas = {6};
        cfg_pso.tipo = 1; // PSO
        RedNeuronal net_pso;
        net_pso.inicializar(1, cfg_pso, 1);
        auto res_pso = net_pso.entrenar(X_pso, Y_pso, idx_pso, idx_pso,
                                        1e-7, 80, nullptr, no_cancel);
        CHECK(res_pso.ok);
        CHECK(!res_pso.mse_ent_hist.empty());
        CHECK(res_pso.mse_ent < res_pso.mse_ent_hist.front());

        // 3. RNN (tipo = 2) reducción monótona de MSE
        ConfigANN cfg_rnn;
        cfg_rnn.ocultas = {6};
        cfg_rnn.tipo = 2; // RNN
        RedNeuronal net_rnn;
        net_rnn.inicializar(1, cfg_rnn, 1);
        auto res_rnn = net_rnn.entrenar(X_pso, Y_pso, idx_pso, idx_pso,
                                        1e-7, 100, nullptr, no_cancel);
        CHECK(res_rnn.ok);
        CHECK(!res_rnn.mse_ent_hist.empty());
        CHECK(res_rnn.mse_ent <= res_rnn.mse_ent_hist.front());
    }

    // ── Pruebas de regresión del módulo Sim (Process Simulation Module) ──
    {
        auto& db_sim = cma::base_datos::GestorDB::instancia();
        db_sim.cargar_predeterminadas();

        const auto* sp_h2o   = db_sim.especie_por_nombre("Agua");
        const auto* sp_co    = db_sim.especie_por_nombre("Monóxido de carbono");
        const auto* sp_o2    = db_sim.especie_por_nombre("Oxígeno");
        const auto* sp_co2   = db_sim.especie_por_nombre("Dióxido de carbono");
        const auto* sp_n2    = db_sim.especie_por_nombre("Nitrógeno");
        const auto* sp_h2    = db_sim.especie_por_nombre("Hidrógeno");
        const auto* sp_nh3   = db_sim.especie_por_nombre("Amoníaco");
        const auto* sp_aloh3 = db_sim.especie_por_nombre("Hidróxido de aluminio");
        const auto* sp_naoh  = db_sim.especie_por_nombre("Hidróxido de sodio (aq)");
        const auto* sp_naalo2= db_sim.especie_por_nombre("Aluminato de sodio (a)");
        const auto* sp_fe2o3 = db_sim.especie_por_nombre("Óxido de hierro(III)");
        const auto* sp_tio2  = db_sim.especie_por_nombre("Dióxido de titanio");
        CHECK(sp_h2o && sp_co && sp_o2 && sp_co2 && sp_n2 && sp_h2 && sp_nh3);
        CHECK(sp_aloh3 && sp_naoh && sp_naalo2 && sp_fe2o3 && sp_tio2);

        // 1. Unidades de Flujo::masa_total, masa_total_t_h y fracciones_molares con PSD + agua
        cma::nucleo::Flujo f_test;
        f_test.flujo_molar[sp_h2o->id] = 1.0; // 1 kmol/s H2O -> 18.015 kg/s
        f_test.agua_kg_h = 3600.0;            // +1.0 kg/s de agua en pulpa
        f_test.particulas.push_back({"CuFeS2", 100.0, 3600.0}); // +1.0 kg/s de mineral
        auto x_mol = f_test.fracciones_molares();
        CHECK(near(x_mol[sp_h2o->id], 1.0, 1e-9));
        CHECK(near(f_test.masa_total(db_sim), 18.015 + 2.0, 1e-2));
        CHECK(near(f_test.masa_total_t_h(db_sim), (18.015 + 2.0) * 3.6, 1e-1));

        // 2. ReactorGibbs: minimización directa de energía libre de Gibbs sin reaccion_id
        cma::diagrama_flujo::Flowsheet fs_gibbs;
        int64_t id_g_src = fs_gibbs.agregar_nodo(cma::nucleo::TipoNodo::Fuente, "Feed CO+O2");
        int64_t id_g_rx  = fs_gibbs.agregar_nodo(cma::nucleo::TipoNodo::ReactorGibbs, "Gibbs Reactor");
        int64_t id_g_snk = fs_gibbs.agregar_nodo(cma::nucleo::TipoNodo::Sumidero, "Product");
        fs_gibbs.conectar(id_g_src, id_g_rx);
        fs_gibbs.conectar(id_g_rx, id_g_snk);
        if (auto* n = fs_gibbs.nodo_por_id(id_g_src)) {
            n->params["temperatura"] = 1000.0;
            n->params["presion"] = 1.0;
            n->params["flujo_" + std::to_string(sp_co->id)] = 2.0; // CO(g)
            n->params["flujo_" + std::to_string(sp_o2->id)] = 1.0; // O2(g)
        }
        if (auto* n = fs_gibbs.nodo_por_id(id_g_rx)) {
            n->params["temperatura"] = 1000.0;
            n->params["presion"] = 1.0;
        }

        BalanceMasa solver_sim(db_sim);
        auto res_gibbs = solver_sim.resolver(fs_gibbs);
        CHECK(res_gibbs.convergio);
        CHECK(res_gibbs.flujos_salida.size() == 2);
        // A 1000 K, 2 CO + O2 -> 2 CO2(g) casi completamente
        CHECK(res_gibbs.flujos_salida[1].flujo_molar[sp_co2->id] > 1.99);
        CHECK(near(res_gibbs.flujos_salida[0].masa_total(db_sim),
                   res_gibbs.flujos_salida[1].masa_total(db_sim), 1e-2));

        // 3. ReactorCinetica: conversión cinética Arrhenius PFR
        cma::diagrama_flujo::Flowsheet fs_kin;
        int64_t id_k_src = fs_kin.agregar_nodo(cma::nucleo::TipoNodo::Fuente, "Syngas");
        int64_t id_k_rx  = fs_kin.agregar_nodo(cma::nucleo::TipoNodo::ReactorCinetica, "PFR");
        int64_t id_k_snk = fs_kin.agregar_nodo(cma::nucleo::TipoNodo::Sumidero, "Out");
        fs_kin.conectar(id_k_src, id_k_rx);
        fs_kin.conectar(id_k_rx, id_k_snk);
        if (auto* n = fs_kin.nodo_por_id(id_k_src)) {
            n->params["flujo_" + std::to_string(sp_n2->id)] = 1.0; // N2
            n->params["flujo_" + std::to_string(sp_h2->id)] = 3.0; // H2
        }
        if (auto* n = fs_kin.nodo_por_id(id_k_rx)) {
            n->params["reaccion_id"] = 4; // Haber-Bosch: N2 + 3H2 -> 2NH3
            n->params["temperatura"] = 700.0;
            n->params["tau"] = 5.0;
            n->params["k0"] = 1.0e4;
            n->params["Ea"] = 45000.0;
        }
        auto res_kin = solver_sim.resolver(fs_kin);
        CHECK(res_kin.convergio);
        CHECK(res_kin.flujos_salida.size() == 2);
        CHECK(res_kin.flujos_salida[1].flujo_molar[sp_nh3->id] > 0.5); // NH3 producido

        // 4. TanqueLixiviacion (Reacción + Separación Sólido/Líquido) y Distribución Elemental (dist_elem_)
        cma::diagrama_flujo::Flowsheet fs_bayer;
        int64_t id_b_src   = fs_bayer.agregar_nodo(cma::nucleo::TipoNodo::Fuente, "Bauxita+NaOH");
        int64_t id_b_leach = fs_bayer.agregar_nodo(cma::nucleo::TipoNodo::TanqueLixiviacion, "Autoclave Bayer");
        int64_t id_b_liq   = fs_bayer.agregar_nodo(cma::nucleo::TipoNodo::Sumidero, "Licor");
        int64_t id_b_mud   = fs_bayer.agregar_nodo(cma::nucleo::TipoNodo::Sumidero, "Lodo Rojo");
        fs_bayer.conectar(id_b_src, id_b_leach);
        fs_bayer.conectar(id_b_leach, id_b_liq);
        fs_bayer.conectar(id_b_leach, id_b_mud);
        if (auto* n = fs_bayer.nodo_por_id(id_b_src)) {
            n->params["flujo_" + std::to_string(sp_aloh3->id)] = 2.0; // Al(OH)3(s)
            n->params["flujo_" + std::to_string(sp_fe2o3->id)] = 0.5; // Fe2O3(s)
            n->params["flujo_" + std::to_string(sp_naoh->id)]  = 2.5; // NaOH(aq)
        }
        if (auto* n = fs_bayer.nodo_por_id(id_b_leach)) {
            n->params["reaccion_id"] = 35; // Al(OH)3 + NaOH(aq) -> NaAlO2(a) + 2H2O(l)
            n->params["conversion"] = 0.90;
            n->params["efficiency"] = 0.98;
        }
        auto res_bayer = solver_sim.resolver(fs_bayer);
        CHECK(res_bayer.convergio);
        CHECK(res_bayer.flujos_salida.size() == 3);
        // NaAlO2(a) va mayoritariamente a Salida 0 (flujos_salida[1]), Fe2O3(s) a Salida 1 (flujos_salida[2])
        CHECK(res_bayer.flujos_salida[1].flujo_molar[sp_naalo2->id] > 1.7);
        CHECK(res_bayer.flujos_salida[2].flujo_molar[sp_fe2o3->id] > 0.48);

        // Prueba de distribución por elementos (dist_elem_Ti y dist_elem_Fe hacia Salida 1)
        cma::diagrama_flujo::Flowsheet fs_dist;
        int64_t id_d_src = fs_dist.agregar_nodo(cma::nucleo::TipoNodo::Fuente, "Ilmenite Feed");
        int64_t id_d_sep = fs_dist.agregar_nodo(cma::nucleo::TipoNodo::Separador, "Elemental Separator");
        int64_t id_d_s0  = fs_dist.agregar_nodo(cma::nucleo::TipoNodo::Sumidero, "Out0");
        int64_t id_d_s1  = fs_dist.agregar_nodo(cma::nucleo::TipoNodo::Sumidero, "Out1");
        fs_dist.conectar(id_d_src, id_d_sep);
        fs_dist.conectar(id_d_sep, id_d_s0);
        fs_dist.conectar(id_d_sep, id_d_s1);
        if (auto* n = fs_dist.nodo_por_id(id_d_src)) {
            n->params["flujo_" + std::to_string(sp_tio2->id)]  = 1.0; // TiO2(s)
            n->params["flujo_" + std::to_string(sp_fe2o3->id)] = 1.0; // Fe2O3(s)
        }
        if (auto* n = fs_dist.nodo_por_id(id_d_sep)) {
            n->params["dist_elem_Ti"] = 0.92;
            n->params["dist_elem_Fe"] = 0.05;
        }
        auto res_dist = solver_sim.resolver(fs_dist);
        CHECK(res_dist.convergio);
        CHECK(res_dist.flujos_salida.size() == 3);
        // Salida 1 (flujos_salida[2]) recibe dist_elem_*, Salida 0 (flujos_salida[1]) recibe (1 - dist_elem_*)
        CHECK(near(res_dist.flujos_salida[2].flujo_molar[sp_tio2->id], 0.92, 1e-6));
        CHECK(near(res_dist.flujos_salida[2].flujo_molar[sp_fe2o3->id], 0.05, 1e-6));
        CHECK(near(res_dist.flujos_salida[1].flujo_molar[sp_tio2->id], 0.08, 1e-6));
        CHECK(near(res_dist.flujos_salida[1].flujo_molar[sp_fe2o3->id], 0.95, 1e-6));
    }

    // ── LCA: catálogo, sugerencias y evaluación por especie ──
    {
        LCA lca;
        CHECK(LCA::catalogo().size() == 19);
        const auto* co2 = LCA::buscar_equivalente("co2-fossil");
        CHECK(co2 != nullptr && near(co2->gwp, 1.0));
        CHECK(LCA::buscar_equivalente("CO2 (fossil)") == co2);
        CHECK(LCA::buscar_equivalente("no-existe") == nullptr);

        CHECK(LCA::sugerir_equivalente("CO2", "Chimenea", false) == "co2-fossil");
        CHECK(LCA::sugerir_equivalente("CH4", "Chimenea", false) == "ch4-fossil");
        CHECK(LCA::sugerir_equivalente("N2O", "Chimenea", false) == "n2o");
        CHECK(LCA::sugerir_equivalente("SO2", "Chimenea", false) == "so2");
        CHECK(LCA::sugerir_equivalente("NOx", "Chimenea", false) == "nox");
        CHECK(LCA::sugerir_equivalente("H2O (g)", "Vapor", false) == "no-mapeado");
        CHECK(LCA::sugerir_equivalente("H2O (l)", "Agua", true) == "water-intake");
        CHECK(LCA::sugerir_equivalente("H2O (l)", "Agua", false) == "wastewater");
        CHECK(LCA::sugerir_equivalente("N2", "Aire", true) == "no-mapeado");
        CHECK(LCA::sugerir_equivalente("O2", "Aire", true) == "no-mapeado");
        CHECK(LCA::sugerir_equivalente("Circuito", "Circuito", true) == "no-mapeado");
        CHECK(LCA::sugerir_equivalente("Escoria", "Escoria", false) == "slag-landfill");
        CHECK(LCA::sugerir_equivalente("Ceniza", "Ceniza volante", false) == "fly-ash");
        CHECK(LCA::sugerir_equivalente("Cu2O", "Mineral", true) == "copper-ore");
        CHECK(LCA::sugerir_equivalente("Cu", "Cátodo", false) == "product");
        CHECK(LCA::sugerir_equivalente("Fe2O3", "Mineral", true) == "iron-ore");
        CHECK(LCA::sugerir_equivalente("CaCO3", "Caliza", true) == "limestone");

        std::vector<FilaLCA> filas;
        filas.push_back({"Horno", "Gas de cola", "CO2", 100.0, true, "co2-fossil"});
        filas.push_back({"Horno", "Escoria", "Escoria", 20.0, false, "slag-landfill"});
        filas.push_back({"Horno", "Agua", "H2O (l)", 10.0, true, "water-intake"});
        filas.push_back({"Horno", "Aire", "N2", 5.0, true, "no-mapeado"});
        filas.push_back({"Horno", "Producto", "Cu", 95.0, false, "product"});

        auto r = lca.evaluar(filas, 100.0, 10.0, 5.0, 2.0, 3.0, 0.0);
        CHECK(r.ok);
        CHECK(near(r.gwp_kg_co2eq, 183.123, 1e-9));
        CHECK(near(r.ap_kg_so2eq, 0.127, 1e-12));
        CHECK(near(r.ep_kg_po4eq, 0.028, 1e-12));
        CHECK(near(r.agua_m3, 2.16, 1e-12));
        CHECK(near(r.energia_mj, 953.036, 1e-9));
        CHECK(near(r.residuos_kg, 23.0, 1e-12));
        CHECK(r.balance_masa_ok);
        CHECK(r.filas_totales == 5 && r.filas_mapeadas == 4 && r.filas_sin_mapear == 1);
        CHECK(near(r.masa_sin_mapear_kg, 5.0, 1e-12));
        CHECK(!r.contribuciones.empty() && r.contribuciones.front().first == "CO2 (fossil)");
        CHECK(near(r.factor_unidad_funcional, 1.0));

        auto r_uf = lca.evaluar(filas, 100.0, 10.0, 5.0, 2.0, 3.0, 50.0);
        CHECK(near(r_uf.factor_unidad_funcional, 0.02));
        CHECK(near(r_uf.gwp_kg_co2eq, 183.123 * 0.02, 1e-9));
        CHECK(near(r_uf.masa_entrada_kg, 115.0 * 0.02, 1e-12));
        CHECK(r_uf.balance_masa_ok);

        std::vector<FilaLCA> desbalanceadas = {
            {"Horno", "Entrada", "Cu", 100.0, true, "copper-ore"},
            {"Horno", "Salida", "Cu", 90.0, false, "product"},
        };
        auto r_desc = lca.evaluar(desbalanceadas);
        CHECK(!r_desc.balance_masa_ok);
        CHECK(!r_desc.avisos.empty());

        std::vector<FilaLCA> invalidas = {
            {"Horno", "Entrada", "Cu", -1.0, true, "copper-ore"},
            {"Horno", "Salida", "Cu", nan, false, "product"},
        };
        auto r_inv = lca.evaluar(invalidas);
        CHECK(r_inv.filas_totales == 0);
        CHECK(r_inv.avisos.size() == 2);

        auto r_manual_nan = lca.evaluar({}, nan, 0.0, 0.0, 0.0, 0.0, 0.0);
        CHECK(near(r_manual_nan.gwp_kg_co2eq, 0.0));

        auto legacy = lca.evaluar_proceso(100.0, 10.0, 5.0, 1000.0, 500.0, 2.0);
        CHECK(legacy.ok);
        CHECK(near(legacy.emision_CO2, 81.4));
        CHECK(near(legacy.emision_CH4, 0.05));
        CHECK(near(legacy.huella_carbono, 83.0265));
        CHECK(near(legacy.consumo_energia, 953.0));
        CHECK(near(legacy.consumo_agua, 2.0));
        CHECK(near(legacy.residuos_solidos, 100.0));
        CHECK(!lca.evaluar_proceso(-1.0, 0.0, 0.0, 0.0, 0.0, 0.0).ok);
        CHECK(near(lca.factor_emision_electricidad(100.0, "brasil"), 15.0));

        cma::diagrama_flujo::Flowsheet fs_lca;
        const auto id_flujo_lca = fs_lca.agregar_flujo("Gas de cola");
        auto* flujo_lca = fs_lca.flujo_por_id(id_flujo_lca);
        flujo_lca->lca_especies["CO2"] = "co2-fossil";
        flujo_lca->lca_especies["N2"] = "no-mapeado";
        cma::diagrama_flujo::Flowsheet fs_lca_cargado;
        std::string error_lca;
        CHECK(fs_lca_cargado.desde_json(fs_lca.a_json(), error_lca));
        const auto* flujo_recuperado = fs_lca_cargado.flujo_por_id(id_flujo_lca);
        CHECK(flujo_recuperado != nullptr);
        CHECK(flujo_recuperado->lca_especies.size() == 2);
        CHECK(flujo_recuperado->lca_especies.at("CO2") == "co2-fossil");
        CHECK(flujo_recuperado->lca_especies.at("N2") == "no-mapeado");
    }

    // ── HSC Material Database (Mat) - Resolución jerárquica de especies y elementos ──
    {
        cma::application::modules::MaterialDB mat_db(cma::base_datos::GestorDB::instancia());
        const auto* blend = mat_db.buscar_por_nombre("Smelter Feed Blend");
        CHECK(blend != nullptr);
        if (blend) {
            CHECK(near(cma::application::modules::MaterialDB::suma_composicion(*blend), 100.0, 1e-3));
            const auto especies = mat_db.resolver_especies(*blend);
            const auto elementos = mat_db.resolver_elementos(especies);
            CHECK(especies.size() == 14);
            CHECK(elementos.size() == 12);
            CHECK(near(cma::application::modules::MaterialDB::suma_especies(especies), 100.0, 1e-2));
            CHECK(near(cma::application::modules::MaterialDB::suma_elementos(elementos), 100.0, 1e-2));

            auto esp_wt = [&](const std::string& f) {
                for (const auto& s : especies) if (s.formula == f) return s.wt_pct;
                return -1.0;
            };
            auto elem_wt = [&](const std::string& sym) {
                for (const auto& e : elementos) if (e.simbolo == sym) return e.wt_pct;
                return -1.0;
            };

            // Verificación exacta contra captura de referencia HSC Material Database
            CHECK(near(esp_wt("CuFeS2"),  16.838, 1e-3));
            CHECK(near(esp_wt("Cu5FeS4"),  1.316, 1e-3));
            CHECK(near(esp_wt("Cu2S"),     8.355, 1e-3));
            CHECK(near(esp_wt("FeS"),      9.428, 1e-3));
            CHECK(near(esp_wt("FeS2"),    26.250, 1e-3));
            CHECK(near(esp_wt("ZnS"),      1.313, 1e-3));
            CHECK(near(esp_wt("NiS"),      0.659, 1e-3));
            CHECK(near(esp_wt("SiO2"),    23.805, 1e-3));
            CHECK(near(esp_wt("MgO"),      4.029, 1e-3));
            CHECK(near(esp_wt("Al2O3"),    1.455, 1e-3));
            CHECK(near(esp_wt("Fe2O3"),    0.454, 1e-3));
            CHECK(near(esp_wt("CaO"),      1.100, 1e-3));
            CHECK(near(esp_wt("C"),        3.000, 1e-3));
            CHECK(near(esp_wt("H2O"),      2.000, 1e-3));

            CHECK(near(elem_wt("Cu"), 13.335, 1e-3));
            CHECK(near(elem_wt("Fe"), 23.795, 1e-3));
            CHECK(near(elem_wt("S"),  26.038, 1e-3));
            CHECK(near(elem_wt("Zn"),  0.881, 1e-3));
            CHECK(near(elem_wt("Ni"),  0.426, 1e-3));
            CHECK(near(elem_wt("Si"), 11.127, 1e-3));
            CHECK(near(elem_wt("O"),  17.188, 1e-3));
            CHECK(near(elem_wt("Mg"),  2.429, 1e-3));
            CHECK(near(elem_wt("Al"),  0.770, 1e-3));
            CHECK(near(elem_wt("Ca"),  0.786, 1e-3));
            CHECK(near(elem_wt("C"),   3.000, 1e-3));
            CHECK(near(elem_wt("H"),   0.224, 1e-3));
        }
    }

    // ── HSC Geo 8.0 (Mineral Database, Modal Calculations, Formula & CIPW Norm) ──
    {
        cma::application::modules::MineralDB geo_db;
        CHECK(geo_db.catalogo().size() >= 30);

        const auto* ccp = geo_db.buscar_por_nombre("Chalcopyrite (Erora, Portugal)");
        CHECK(ccp != nullptr);
        if (ccp) {
            CHECK(ccp->simbolo_ima == "Ccp");
            CHECK(near(ccp->porcentaje_elemento("Cu"), 34.628, 0.05));
            CHECK(near(ccp->porcentaje_elemento("Fe"), 30.432, 0.05));
            CHECK(near(ccp->porcentaje_elemento("S"),  34.940, 0.05));
            CHECK(geo_db.cumple_consulta_hsc(*ccp, "[Cu %] Is greater than 10 AND [Specific Gravity] Is between 2.2 and 4.4"));
        }

        // Cálculo modal en 3 rondas para Bruvann.xls - XRF (25% Ccp, 20% Py, 5% Gn, 30% Qtz, 20% Ol)
        const auto& m_bruvann = geo_db.muestras_predefinidas().front();
        std::vector<cma::application::modules::RondaModal> rondas = {
            {"Round 1", {"Chalcopyrite (Erora, Portugal)"}, {"Cu"}},
            {"Round 2", {"Pyrite (The Wilhelmine mine, Spessart,Bavaria)", "Galena (Kansan, USA)"}, {"S", "Pb"}},
            {"Round 3", {"Quartz (Kondyor PGE Placer Deposit, Russia)", "Olivine (Western Bushveld complex, South Africa)"}, {"Si", "Mg"}}
        };
        auto res_modal = geo_db.resolver_modal_rondas(m_bruvann.ensayo_elem_wt, rondas, cma::application::modules::MetodoModal::NNLS);
        CHECK(res_modal.rondas_ejecutadas == 3);
        CHECK(res_modal.fases.size() == 5);
        CHECK(near(res_modal.total_mineral_wt, 100.0, 0.25));
        CHECK(res_modal.rmse_elemental < 0.15);

        // Cálculo de fórmula mineral desde análisis EPMA
        auto res_form = geo_db.calcular_formula_mineral({{"Cu", 34.63}, {"Fe", 30.43}, {"S", 34.94}}, "S", 2.0);
        CHECK(res_form.formula_empirica == "CuFeS2");
        CHECK(res_form.formula_ideal == "CuFeS2");

        // Cálculo petrológico CIPW Norm
        cma::application::modules::AnalisisRocaOxidos norita;
        auto res_cipw = geo_db.calcular_norma_cipw(norita);
        CHECK(res_cipw.mg_number > 50.0 && res_cipw.mg_number < 85.0);
        CHECK(!res_cipw.clasificacion_tas.empty());
        CHECK(!res_cipw.clasificacion_qapf.empty());
    }

    // ── HSC Heat Loss Module (Hexagonal Domain/Catalog/Engine/Application) ──
    {
        cma::application::modules::HeatLossCalc hl;
        CHECK(hl.catalog().conduction_materials().size() >= 25);
        CHECK(hl.catalog().convection_fluids().size() >= 8);
        CHECK(hl.catalog().surface_materials().size() >= 10);
        CHECK(hl.sheets().size() == 3); // Wall, Roof, Bottom

        // Verificar hoja Wall de referencia HSC (1100 °C -> 25 °C)
        const auto& wall_sheet = hl.sheets()[0];
        CHECK(wall_sheet.converged);
        const double q_wall_ref = wall_sheet.total_heat_flow_kw;
        CHECK(near(q_wall_ref, 13.85, 0.45));
        CHECK(near(wall_sheet.outer_shell_temp_c, 87.0, 2.5));
        CHECK(!wall_sheet.profile.empty());
        CHECK(near(wall_sheet.profile.front().temperature_c, 1100.0, 1e-3));
        CHECK(near(wall_sheet.profile.back().temperature_c, 25.0, 1e-3));

        // Verificar solver inverso (Temperature Profile con Q fijo)
        hl.set_active_sheet_index(0);
        hl.active_sheet().solver_mode = cma::domain::HeatLossSolverMode::TemperatureProfile;
        hl.active_sheet().total_heat_flow_kw = q_wall_ref;
        CHECK(hl.solve_active_sheet());
        CHECK(near(hl.active_sheet().ambient_temp_c, 25.0, 0.5));

        // Verificar Optimizador de Espesor (Target Value Solver)
        hl.active_sheet().solver_mode = cma::domain::HeatLossSolverMode::HeatFlowTwoTemps;
        cma::domain::TargetThicknessRequest req;
        req.target_column_index = 1; // Forsterite brick
        req.target_is_outer_temp = false;
        req.target_value = 10.0;     // Reducir pérdida de calor de 13.47 kW a 10.0 kW
        req.min_thickness_cm = 10.0;
        req.max_thickness_cm = 250.0;
        auto opt_res = hl.optimize_layer_thickness(req);
        CHECK(opt_res.ok);
        CHECK(opt_res.optimal_thickness_cm > 50.0);
        CHECK(near(opt_res.achieved_heat_flow_kw, 10.0, 0.05));

        // Verificar Calculadora de Radiación de Gases y Partículas (Hottel)
        cma::domain::GasParticleRadiationInput gas_in;
        auto gas_res = hl.calculate_gas_radiation(gas_in);
        CHECK(gas_res.gas_emissivity > 0.10 && gas_res.gas_emissivity < 0.90);
        CHECK(gas_res.total_emissivity >= gas_res.gas_emissivity);
        CHECK(gas_res.effective_hr_w_m2k > 10.0);
    }

    // ── HSC Stability Diagrams Module (Lpp Vector Kellogg & Tpp Gibbs Minimization) ──
    {
        auto& repo = cma::base_datos::GestorDB::instancia();
        cma::application::modules::StabilityDiagrams stab(&repo);

        // 1. Verificar balance estequiométrico 2x2 para Cu - O2 - SO2
        auto cands = stab.filtrar_especies_sistema("Cu", "O2", "SO2", true);
        CHECK(cands.size() >= 5);

        // 2. Verificar motor Lpp (polígonos de Kellogg, líneas de equilibrio y puntos triples)
        cma::domain::LppDiagramRequest lpp_req;
        lpp_req.main_element = "Cu";
        lpp_req.gas1_formula = "O2";
        lpp_req.gas2_formula = "SO2";
        lpp_req.temperature_k = 973.15;
        lpp_req.x_min = -20.0; lpp_req.x_max = 0.0;
        lpp_req.y_min = -15.0; lpp_req.y_max = 2.0;
        lpp_req.probe_log_p1 = -18.0;
        lpp_req.probe_log_p2 = -12.0;
        auto lpp_res = stab.calcular_lpp(lpp_req);
        CHECK(lpp_res.ok);
        CHECK(lpp_res.polygons.size() >= 4);
        CHECK(!lpp_res.boundaries.empty());
        CHECK(!lpp_res.triple_points.empty());
        CHECK(lpp_res.probe_predominant_phase == "Cu");

        // 3. Verificar motor Tpp (modo T-p: Temperatura K vs log10 p(O2) a log10 p(SO2) = -1.0)
        cma::domain::TppDiagramRequest tpp_req;
        tpp_req.main_element = "Cu";
        tpp_req.gas1_formula = "O2";
        tpp_req.gas2_formula = "SO2";
        tpp_req.x_var = cma::domain::StabilityAxisVar::TemperatureK;
        tpp_req.y_var = cma::domain::StabilityAxisVar::LogPressureGas1;
        tpp_req.fixed_value = -1.0;
        tpp_req.x_min = 600.0; tpp_req.x_max = 1400.0;
        tpp_req.y_min = -20.0; tpp_req.y_max = 0.0;
        tpp_req.rows = 60;
        tpp_req.cols = 60;
        auto tpp_res = stab.calcular_tpp(tpp_req);
        CHECK(tpp_res.ok);
        CHECK(tpp_res.legend.size() >= 3);
        CHECK(!tpp_res.contours.empty());
    }

    // ── HSC Eh-pH Diagrams Module (Pourbaix / EpH — Hexagonal Engine) ──────────
    {
        auto& repo = cma::base_datos::GestorDB::instancia();
        cma::motores::CalculadorPourbaix eph(&repo);

        // 1. Límites termodinámicos de estabilidad del agua a 298.15 K (escala SHE)
        eph.establecer_temperatura(298.15);
        eph.establecer_escala_potencial(cma::domain::PourbaixPotentialScale::SHE);
        CHECK(near(eph.pendiente_nernst(), -0.05916, 1e-4));
        CHECK(near(eph.eh_agua_oxidacion(0.0, 1.0), 1.2289, 2e-3));
        CHECK(near(eph.eh_agua_reduccion(0.0, 1.0), 0.0000, 1e-6));
        CHECK(near(eph.eh_agua_oxidacion(7.0, 1.0) - eph.eh_agua_reduccion(7.0, 1.0), 1.2289, 2e-3));

        // 2. Sistema clásico Fe - H2O (25 °C, [Fe] = 1e-6 m)
        eph.cargar_preset_hsc(0);
        CHECK(eph.obtener_especies().size() >= 8);
        CHECK(!eph.obtener_lineas().empty());
        CHECK(!eph.obtener_poligonos().empty());
        CHECK(!eph.obtener_puntos_triples().empty());

        // Inmunidad (Fe metálico a bajo Eh)
        auto p_imm = eph.evaluar_punto(7.0, -0.90);
        CHECK(p_imm.valid);
        CHECK(p_imm.predominant_formula == "Fe");
        CHECK(p_imm.corrosion_state == cma::domain::PourbaixCorrosionState::Immunity);
        CHECK(p_imm.h2_evolution);

        // Corrosión activa ácida: Fe2+ a Eh moderado y Fe3+ a alto Eh
        auto p_fe2 = eph.evaluar_punto(1.5, 0.20);
        CHECK(p_fe2.valid);
        CHECK(p_fe2.predominant_formula == "Fe(+2a)");
        CHECK(p_fe2.corrosion_state == cma::domain::PourbaixCorrosionState::Corrosion);
        CHECK(p_fe2.water_stable);

        auto p_fe3 = eph.evaluar_punto(0.5, 1.00);
        CHECK(p_fe3.valid);
        CHECK(p_fe3.predominant_formula == "Fe(+3a)");
        CHECK(p_fe3.corrosion_state == cma::domain::PourbaixCorrosionState::Corrosion);

        // Pasivación (Hematita Fe2O3 en medio neutro/alcalino oxidante)
        auto p_pas = eph.evaluar_punto(9.0, 0.40);
        CHECK(p_pas.valid);
        CHECK(p_pas.predominant_formula == "Fe2O3");
        CHECK(p_pas.corrosion_state == cma::domain::PourbaixCorrosionState::Passivation);

        // 3. Sistema multicomponente Cu - S - H2O (Calcosina Cu2S y Covelina CuS)
        eph.cargar_preset_hsc(2);
        auto p_cus = eph.evaluar_punto(4.0, -0.10);
        CHECK(p_cus.valid);
        CHECK(p_cus.predominant_formula == "Cu2S" || p_cus.predominant_formula == "CuS");
        CHECK(p_cus.corrosion_state == cma::domain::PourbaixCorrosionState::Passivation);

        // 4. Sistema hidrometalúrgico Au - Cl - H2O (Complejo AuCl4- en cloruración ácida)
        eph.cargar_preset_hsc(4);
        auto p_au_imm = eph.evaluar_punto(7.0, 0.20);
        CHECK(p_au_imm.valid);
        CHECK(p_au_imm.predominant_formula == "Au");
        CHECK(p_au_imm.corrosion_state == cma::domain::PourbaixCorrosionState::Immunity);

        auto p_au_cl = eph.evaluar_punto(1.0, 1.25);
        CHECK(p_au_cl.valid);
        CHECK(p_au_cl.predominant_formula == "AuCl4(-a)");
        CHECK(p_au_cl.corrosion_state == cma::domain::PourbaixCorrosionState::Corrosion);

        // 5. Conversión de escala de referencia (SCE y Ag/AgCl) y exportación CSV
        eph.establecer_escala_potencial(cma::domain::PourbaixPotentialScale::SCE);
        CHECK(near(eph.eh_agua_reduccion(0.0, 1.0), -0.2412, 1e-4));
        const std::string csv_eph = eph.exportar_csv();
        CHECK(csv_eph.find("[SPECIES]") != std::string::npos);
        CHECK(csv_eph.find("[NERNST_BOUNDARIES]") != std::string::npos);
        CHECK(csv_eph.find("[TRIPLE_POINTS]") != std::string::npos);
    }

    // ── Curación de Semilla Termodinámica y Soporte Maier-Kelley Cp(T) ─────────
    {
        auto& repo = cma::base_datos::GestorDB::instancia();
        const auto curadas = repo.especies_curadas();
        CHECK(curadas.size() >= 400);
        for (const auto* sp : curadas) {
            CHECK(sp != nullptr);
            CHECK(sp->es_curada() || sp->provenance.revision != "synthetic-estimate");
        }

        // Verificar integración Maier-Kelley c * T^-2 con ModeloCp explícito
        cma::nucleo::Especie mk;
        mk.modelo_cp = cma::nucleo::ModeloCp::MAIER_KELLEY;
        mk.h_formacion = -800.0;
        mk.s_formacion = 90.0;
        mk.cp_a = 100.0;
        mk.cp_b = 0.020;
        mk.cp_c = -9.0e4; // -90,000 J·K/mol -> a 300 K aporta -1.0 J/(mol·K)
        mk.cp_d = 0.0;
        CHECK(near(mk.cp(300.0), 100.0 + 0.020 * 300.0 - 9.0e4 / (300.0 * 300.0), 1e-9));
        CHECK(std::isfinite(mk.energia_gibbs(1000.0)));
        CHECK(mk.entalpia_ref(1000.0) > 50.0 && mk.entalpia_ref(1000.0) < 120.0);
    }

    // ── HSC H, S, Cp, G & Ellingham Diagrams Module (Hexagonal Engine) ─────────
    {
        auto& repo = cma::base_datos::GestorDB::instancia();
        cma::application::modules::PropertyDiagrams pd;

        // 1. API clásica generarCurvas
        auto pts = pd.generarCurvas(-285.83, 69.95, 75.3, 0.0, 0.0, 0.0, 298.15, 373.15);
        CHECK(pts.size() == 101);
        CHECK(near(pts.front().T, 298.15, 1e-4));
        CHECK(near(pts.front().H, -285.83, 1e-4));

        // 2. Curva de especie individual con formación desde elementos de referencia
        auto c_fe = pd.calcular_curva_especie(repo, "Fe", 298.15, 1500.0, 60, true);
        CHECK(!c_fe.points.empty());
        CHECK(near(c_fe.points.front().dH298, 0.0, 1e-6));
        CHECK(near(c_fe.points.front().dGf, 0.0, 1e-3));
        CHECK(near(c_fe.points.back().dGf, 0.0, 1e-3)); // Fe es elemento puro estándar -> dGf == 0

        auto c_o2 = pd.calcular_curva_especie(repo, "O2", 298.15, 1500.0, 60, true);
        CHECK(near(c_o2.points.front().dGf, 0.0, 1e-3));
        CHECK(near(c_o2.points.back().dGf, 0.0, 1e-3)); // O2(g) es gas estándar de referencia -> dGf == 0

        auto c_fe2o3 = pd.calcular_curva_especie(repo, "Fe2O3", 298.15, 1500.0, 60, true);
        CHECK(c_fe2o3.points.front().dGf < -700.0); // dGf(298) ~ -742 kJ/mol
        CHECK(c_fe2o3.points.back().dGf < -400.0);

        // 3. Familia preconfigurada Ellingham: MetallurgicalOxides (por 1 mol O2)
        auto res_ox = pd.generar_familia_ellingham(repo, cma::domain::EllinghamPresetFamily::MetallurgicalOxides, 298.15, 2000.0, 80);
        CHECK(res_ox.curves.size() >= 15);
        CHECK(!res_ox.intersections.empty());
        CHECK(!res_ox.iso_pO2_lines.empty());
        CHECK(!res_ox.iso_CO_CO2_lines.empty());
        CHECK(!res_ox.iso_H2_H2O_lines.empty());

        // Verificar jerarquía de estabilidad termodinámica de óxidos a 1000 K
        double dG_CaO = 0.0, dG_Al2O3 = 0.0, dG_SiO2 = 0.0, dG_FeO = 0.0, dG_Cu2O = 0.0;
        double dG_C_CO_500 = 0.0, dG_C_CO_1800 = 0.0;
        double dG_Fe_FeO_500 = 0.0, dG_Fe_FeO_1800 = 0.0;

        for (const auto& c : res_ox.curves) {
            for (const auto& pt : c.points) {
                if (std::abs(pt.T - 1000.0) < 15.0) {
                    if (c.label.find("CaO") != std::string::npos) dG_CaO = pt.dGf;
                    else if (c.label.find("Al2O3") != std::string::npos) dG_Al2O3 = pt.dGf;
                    else if (c.label.find("SiO2") != std::string::npos) dG_SiO2 = pt.dGf;
                    else if (c.label.find("FeO") != std::string::npos && c.label.find("Fe3O4") == std::string::npos) dG_FeO = pt.dGf;
                    else if (c.label.find("Cu2O") != std::string::npos) dG_Cu2O = pt.dGf;
                }
                if (c.label.find("2 C +") != std::string::npos && c.label.find("2 CO") != std::string::npos) {
                    if (std::abs(pt.T - 500.0) < 15.0) dG_C_CO_500 = pt.dGf;
                    if (std::abs(pt.T - 1800.0) < 25.0) dG_C_CO_1800 = pt.dGf;
                }
                if (c.label.find("FeO") != std::string::npos && c.label.find("2 Fe") != std::string::npos) {
                    if (std::abs(pt.T - 500.0) < 15.0) dG_Fe_FeO_500 = pt.dGf;
                    if (std::abs(pt.T - 1800.0) < 25.0) dG_Fe_FeO_1800 = pt.dGf;
                }
            }
        }
        CHECK(dG_CaO < dG_Al2O3);
        CHECK(dG_Al2O3 < dG_SiO2);
        CHECK(dG_SiO2 < dG_FeO);
        CHECK(dG_FeO < dG_Cu2O);

        // Pendiente negativa de 2 C + O2 = 2 CO (entropía positiva dS > 0)
        CHECK(dG_C_CO_1800 < dG_C_CO_500);
        // Pendiente positiva de oxidación metálica 2 Fe + O2 = 2 FeO (dS < 0)
        CHECK(dG_Fe_FeO_1800 > dG_Fe_FeO_500);

        // Intersección de Boudouard (2 C + O2 = 2 CO y C + O2 = CO2) entre 900 K y 1050 K
        bool boudouard_found = false;
        for (const auto& inter : res_ox.intersections) {
            if ((inter.curve_a.find("2 CO") != std::string::npos && inter.curve_b.find("CO2") != std::string::npos) ||
                (inter.curve_b.find("2 CO") != std::string::npos && inter.curve_a.find("CO2") != std::string::npos)) {
                CHECK(inter.T_K >= 880.0 && inter.T_K <= 1060.0);
                boudouard_found = true;
                break;
            }
        }
        CHECK(boudouard_found);

        // Kinks de transición de fase en Zn (Fusión ~693 K, Ebullición ~1180 K)
        bool zn_trans_found = false;
        for (const auto& c : res_ox.curves) {
            if (c.label.find("ZnO") != std::string::npos) {
                CHECK(!c.phase_transitions.empty());
                zn_trans_found = true;
                break;
            }
        }
        CHECK(zn_trans_found);

        // 4. Escala nomográfica de Richardson-Jeffes
        auto nom = pd.evaluar_escala_nomografica(repo, 1273.15, -400.0);
        CHECK(nom.log10_pO2 < -10.0);
        CHECK(std::isfinite(nom.log10_CO_CO2));
        CHECK(std::isfinite(nom.log10_H2_H2O));

        // 5. Parser de ecuaciones y exportación CSV
        auto c_parse = pd.calcular_curva_ecuacion_texto(repo, "2 Zn + O2(g) = 2 ZnO", cma::domain::EllinghamNormalizationBasis::PerMoleO2, 298.15, 1500.0);
        CHECK(!c_parse.points.empty());
        CHECK(c_parse.norm_factor == 1.0);

        std::string csv_data = pd.exportar_csv(res_ox.curves, res_ox.intersections, cma::domain::PropertyDiagramType::DeltaG_Ellingham);
        CHECK(csv_data.find("[CURVES]") != std::string::npos);
        CHECK(csv_data.find("[INTERSECTIONS]") != std::string::npos);
    }

    // ── ETAPA 7: BATERÍA DE PRUEBAS DE REGRESIÓN CIENTÍFICA (T-01 a T-08) ────────
    {
        auto& repo = cma::base_datos::GestorDB::instancia();
        const auto& tp = cma::nucleo::TablaPeriodica::instancia();

        // ── T-01: Verificación de ModeloCp (despacho correcto y valores por defecto)
        {
            cma::nucleo::Especie sp_def;
            CHECK(sp_def.modelo_cp == cma::nucleo::ModeloCp::POLINOMIAL_ESTANDAR);
            CHECK(near(sp_def.T_min, 298.15, 1e-4));
            CHECK(near(sp_def.T_max, 2000.0, 1e-4));
            CHECK(std::string(cma::nucleo::nombre_modelo_cp(cma::nucleo::ModeloCp::POLINOMIAL_ESTANDAR)) == "Polinomial Estándar");
            CHECK(std::string(cma::nucleo::nombre_modelo_cp(cma::nucleo::ModeloCp::MAIER_KELLEY)) == "Maier-Kelley");
            CHECK(std::string(cma::nucleo::nombre_modelo_cp(cma::nucleo::ModeloCp::SHOMATE_NIST)) == "Shomate (NIST)");
            CHECK(std::string(cma::nucleo::nombre_modelo_cp(cma::nucleo::ModeloCp::CONSTANTE)) == "Constante");

            // Modelo Constante
            cma::nucleo::Especie sp_c;
            sp_c.modelo_cp = cma::nucleo::ModeloCp::CONSTANTE;
            sp_c.cp_a = 75.3;
            CHECK(near(sp_c.cp(350.0), 75.3, 1e-9));
            CHECK(near(sp_c.entalpia_ref(398.15), 75.3 * (398.15 - 298.15) / 1000.0, 1e-6));
            CHECK(near(sp_c.entropia_ref(398.15), 75.3 * std::log(398.15 / 298.15), 1e-6));
        }

        // ── T-02: Identidades termodinámicas dH/dT = Cp y dS/dT = Cp/T (tol 10^-6)
        {
            // Caso A: Polinomial Estándar
            cma::nucleo::Especie sp_p;
            sp_p.modelo_cp = cma::nucleo::ModeloCp::POLINOMIAL_ESTANDAR;
            sp_p.cp_a = 28.0; sp_p.cp_b = 0.005; sp_p.cp_c = -1.2e-6; sp_p.cp_d = 4.0e-10;

            // Caso B: Maier-Kelley
            cma::nucleo::Especie sp_mk;
            sp_mk.modelo_cp = cma::nucleo::ModeloCp::MAIER_KELLEY;
            sp_mk.cp_a = 105.0; sp_mk.cp_b = 0.015; sp_mk.cp_c = -8.5e5; sp_mk.cp_d = 0.0;

            const double eps = 1e-4;
            for (double T = 250.0; T <= 1500.0; T += 50.0) {
                // Polinomial: dH/dT = Cp
                double dH_num_p = (sp_p.entalpia_ref(T + eps) - sp_p.entalpia_ref(T - eps)) / (2.0 * eps) * 1000.0;
                double cp_ana_p = sp_p.cp(T);
                CHECK(std::abs(dH_num_p - cp_ana_p) / cp_ana_p < 1e-6);

                // Polinomial: dS/dT = Cp/T
                double dS_num_p = (sp_p.entropia_ref(T + eps) - sp_p.entropia_ref(T - eps)) / (2.0 * eps);
                double dS_ana_p = cp_ana_p / T;
                CHECK(std::abs(dS_num_p - dS_ana_p) / dS_ana_p < 1e-6);

                // Maier-Kelley: dH/dT = Cp
                double dH_num_mk = (sp_mk.entalpia_ref(T + eps) - sp_mk.entalpia_ref(T - eps)) / (2.0 * eps) * 1000.0;
                double cp_ana_mk = sp_mk.cp(T);
                CHECK(std::abs(dH_num_mk - cp_ana_mk) / cp_ana_mk < 1e-6);

                // Maier-Kelley: dS/dT = Cp/T
                double dS_num_mk = (sp_mk.entropia_ref(T + eps) - sp_mk.entropia_ref(T - eps)) / (2.0 * eps);
                double dS_ana_mk = cp_ana_mk / T;
                CHECK(std::abs(dS_num_mk - dS_ana_mk) / dS_ana_mk < 1e-6);
            }
        }

        // ── T-03: Continuidad y monotonía en T = 200 K (sin artefacto de derivada nula)
        {
            cma::nucleo::Especie sp_mk;
            sp_mk.modelo_cp = cma::nucleo::ModeloCp::MAIER_KELLEY;
            sp_mk.cp_a = 90.0; sp_mk.cp_b = 0.02; sp_mk.cp_c = -5.0e5; sp_mk.cp_d = 0.0;

            const double eps = 1e-4;
            for (double T : {190.0, 199.0, 200.0, 201.0, 210.0}) {
                double dH_num = (sp_mk.entalpia_ref(T + eps) - sp_mk.entalpia_ref(T - eps)) / (2.0 * eps) * 1000.0;
                double cp_val = sp_mk.cp(T);
                CHECK(std::abs(dH_num - cp_val) / cp_val < 1e-6);
            }
            // Comprobación de estricta monotonía entre 190 K y 210 K
            CHECK(sp_mk.cp(190.0) < sp_mk.cp(200.0));
            CHECK(sp_mk.cp(200.0) < sp_mk.cp(210.0));
            CHECK(sp_mk.entalpia_ref(190.0) < sp_mk.entalpia_ref(200.0));
            CHECK(sp_mk.entalpia_ref(200.0) < sp_mk.entalpia_ref(210.0));
        }

        auto por_formula = [&](const std::string& f) -> const cma::nucleo::Especie* {
            for (const auto& [id, sp] : repo.mapa_especies()) {
                if (sp.formula == f) return &sp;
            }
            return nullptr;
        };

        // ── T-04: Metano CH4 Cp vs NIST Shomate en 298.15 K, 500 K y 1000 K
        {
            const auto* ch4 = por_formula("CH4");
            CHECK(ch4 != nullptr);
            if (ch4) {
                // NIST Shomate de referencia: 298.15 K -> 35.69 J/(mol K)
                double cp_298 = ch4->cp(298.15);
                CHECK(std::abs(cp_298 - 35.69) / 35.69 < 0.01);

                // 500 K -> 46.54 J/(mol K)
                double cp_500 = ch4->cp(500.0);
                CHECK(std::abs(cp_500 - 46.54) / 46.54 < 0.015);

                // 1000 K -> 71.79 J/(mol K) (corrección del defecto -14.79%)
                double cp_1000 = ch4->cp(1000.0);
                CHECK(cp_1000 > 70.0);
                CHECK(std::abs(cp_1000 - 71.79) / 71.79 < 0.025);
            }
        }

        // ── T-05: Calcinación de CaCO3 (descomposición a 1165–1180 K)
        {
            const auto* caco3 = por_formula("CaCO3");
            const auto* cao = por_formula("CaO");
            const auto* co2 = por_formula("CO2");
            CHECK(caco3 != nullptr);
            CHECK(cao != nullptr);
            CHECK(co2 != nullptr);
            if (caco3 && cao && co2) {
                // Control de Cp JANAF para CaCO3
                CHECK(std::abs(caco3->cp(298.15) - 81.85) / 81.85 < 0.01);
                CHECK(std::abs(caco3->cp(500.0) - 100.21) / 100.21 < 0.015);
                CHECK(std::abs(caco3->cp(1000.0) - 123.83) / 123.83 < 0.015);

                // Función de energía de Gibbs de reacción: Delta G = G(CaO) + G(CO2) - G(CaCO3)
                auto delta_G = [&](double T) {
                    return cao->energia_gibbs(T) + co2->energia_gibbs(T) - caco3->energia_gibbs(T);
                };

                // A 1160 K: Delta G > 0 (CaCO3 estable)
                CHECK(delta_G(1160.0) > 0.0);
                // A 1190 K: Delta G < 0 (Descomposición espontánea)
                CHECK(delta_G(1190.0) < 0.0);

                // Búsqueda de T_dec con bisección exacta
                double t_low = 1100.0, t_high = 1250.0;
                for (int b = 0; b < 25; ++b) {
                    double t_mid = 0.5 * (t_low + t_high);
                    if (delta_G(t_mid) > 0.0) t_low = t_mid;
                    else t_high = t_mid;
                }
                double t_dec = 0.5 * (t_low + t_high);
                // Debe encontrarse dentro del rango termodinámico JANAF [1165 K, 1180 K]
                CHECK(t_dec >= 1165.0 && t_dec <= 1180.0);
            }
        }

        // ── T-06: Wegstein con barrido completo de pendientes s
        {
            // Simulador de paso de Wegstein según la formulación corregida
            auto test_wegstein_slope = [](double s_slope) {
                const double x_star = 50.0;
                auto f_map = [=](double x) { return x_star + s_slope * (x - x_star); };
                double x = 10.0, xp = 10.0, fp = f_map(10.0);
                bool conv = false;
                for (int it = 0; it < 60; ++it) {
                    double fx = f_map(x);
                    if (std::abs(fx - x) < 1e-6) { conv = true; break; }
                    double x_next = fx;
                    if (it > 0) {
                        double dx = x - xp;
                        double df = fx - fp;
                        if (std::abs(dx) > 1e-12) {
                            double s = df / dx;
                            if (std::abs(s - 1.0) < 0.02) {
                                x_next = fx;
                            } else if (s <= -5.0) {
                                double w = 1.0 / (1.0 - s);
                                x_next = (1.0 - w) * x + w * fx;
                            } else if (s < 1.0) {
                                double q = std::clamp(s / (s - 1.0), -20.0, 0.8);
                                x_next = q * x + (1.0 - q) * fx;
                            } else {
                                double step = (fx - x) / (s - 1.0);
                                x_next = x - 0.9 * step;
                            }
                        }
                    }
                    xp = x;
                    fp = fx;
                    x = std::max(0.0, x_next);
                }
                return conv;
            };

            const std::vector<double> slopes = {-5.0, -2.0, -0.5, 0.0, 0.5, 0.9, 0.95, 0.97, 1.05, 1.20};
            for (double s_val : slopes) {
                CHECK(test_wegstein_slope(s_val));
            }
        }

        // ── T-07: Wegstein en flowsheet con reciclo (BalanceMasa)
        {
            cma::motores::BalanceMasa bm(repo);
            cma::diagrama_flujo::Flowsheet fs_recycle;

            int64_t n_in = fs_recycle.agregar_nodo(cma::nucleo::TipoNodo::Fuente, "Alimentacion");
            int64_t n_mx = fs_recycle.agregar_nodo(cma::nucleo::TipoNodo::Mezclador, "Mezclador");
            int64_t n_sp = fs_recycle.agregar_nodo(cma::nucleo::TipoNodo::Divisor, "Separador");
            int64_t n_out = fs_recycle.agregar_nodo(cma::nucleo::TipoNodo::Sumidero, "Salida");

            int64_t f_feed = fs_recycle.agregar_flujo("Feed");
            int64_t f_to_sep = fs_recycle.agregar_flujo("MixToSep");
            int64_t f_rec = fs_recycle.agregar_flujo("Recycle");
            int64_t f_prod = fs_recycle.agregar_flujo("Product");

            fs_recycle.conectar(n_in, n_mx, f_feed);
            fs_recycle.conectar(n_mx, n_sp, f_to_sep);
            fs_recycle.conectar(n_sp, n_mx, f_rec); // Bucle de reciclo
            fs_recycle.conectar(n_sp, n_out, f_prod);

            if (auto* n_div_ptr = fs_recycle.nodo_por_id(n_sp)) {
                n_div_ptr->params["frac_0"] = 0.30;
                n_div_ptr->params["frac_1"] = 0.70;
            }

            if (auto* n_src_ptr = fs_recycle.nodo_por_id(n_in)) {
                n_src_ptr->params["flujo_1"] = 100.0;
            }

            auto res_bm = bm.resolver(fs_recycle, 50, 1e-6);
            CHECK(res_bm.convergio);
            CHECK(res_bm.iteraciones <= 8);
        }

        // ── T-08: Parsing y consistencia de fórmulas minerales (Pentlandita y Pirrotita)
        {
            auto c_pent = tp.analizar_formula("Fe5Ni4S8");
            CHECK(c_pent.size() == 3);
            double m_pent = 0.0;
            for (const auto& a : c_pent) m_pent += tp.por_simbolo(a.elem)->masa_atomica * a.cuenta;
            CHECK(near(m_pent, 770.517, 0.01));

            auto c_pyr = tp.analizar_formula("Fe7S8");
            CHECK(c_pyr.size() == 2);
            double m_pyr = 0.0;
            for (const auto& a : c_pyr) m_pyr += tp.por_simbolo(a.elem)->masa_atomica * a.cuenta;
            CHECK(near(m_pyr, 647.435, 0.01));

            // Verificar que no figuren en issue_count de catalog audit
            auto audit = cma::application::audit_catalog(repo);
            for (const auto& iss : audit.issues) {
                CHECK(iss.detail.find("(Fe,Ni)9S8") == std::string::npos);
                CHECK(iss.detail.find("Fe1₋ₓS") == std::string::npos);
            }
        }
    }

    std::cout << checks << " checks; " << failures << " failures\n";
    return failures ? 1 : 0;
}
