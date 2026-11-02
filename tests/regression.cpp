#include "motores/simulacion_dinamica.hpp"
#include "nucleo/tabla_periodica.hpp"
#include "motores/reconciliacion_datos.hpp"
#include "motores/perdida_calor.hpp"
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
#include "estimation/BensonEstimator.hpp"
#include "diagrams/StabilityDiagrams.hpp"
#include "application/Pca.hpp"
#include "adapters/desktop/process.hpp"
#include "base_datos/gestor_db.hpp"
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

    std::cout << checks << " checks; " << failures << " failures\n";
    return failures ? 1 : 0;
}
