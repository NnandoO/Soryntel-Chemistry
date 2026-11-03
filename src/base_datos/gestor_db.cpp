#include "base_datos/gestor_db.hpp"
#include "base_datos/datos_especies.hpp"
#include <algorithm>
#include <sstream>
#include <fstream>
#include <iostream>
#include <cctype>
#include "adapters/persistence/csv.hpp"
#include <sqlite3.h>

namespace cma { namespace base_datos {

GestorDB& GestorDB::instancia() {
    static GestorDB db;
    return db;
}

GestorDB::GestorDB() {
    poblar_elementos();
    poblar_especies();
    poblar_reacciones();
    inicializada_ = true;
}

bool GestorDB::cargar_predeterminadas() { return inicializada_; }

void GestorDB::poblar_elementos() {
    // Delegar a la tabla periódica
    auto& tp = nucleo::TablaPeriodica::instancia();
    for (auto& e : tp.todos()) {
        elementos_[e.numero_atomico] = e;
    }
}

void GestorDB::poblar_especies() {
    const auto& tp = nucleo::TablaPeriodica::instancia();
    for (auto& d : semilla_especies()) {
        nucleo::Especie e;
        e.id = prox_id_especie_++;
        if (e.id <= 445) {
            e.provenance.source = "curated:NIST-JANAF-HSC seed record " + std::to_string(e.id);
            e.provenance.revision = "curated-experimental";
        } else {
            e.provenance.source = "generated:extended-combinatorial record " + std::to_string(e.id);
            e.provenance.revision = "synthetic-estimate";
        }
        e.nombre = d.nombre;
        e.formula = d.formula;
        e.masa_molar = d.M;
        e.h_formacion = d.Hf;
        e.s_formacion = d.Sf;
        e.cp_a = d.Cp_a;
        e.cp_b = d.Cp_b;
        e.cp_c = d.Cp_c;
        e.cp_d = d.Cp_d;
        if (d.modelo_cp != nucleo::ModeloCp::POLINOMIAL_ESTANDAR) {
            e.modelo_cp = d.modelo_cp;
        } else if (std::abs(d.Cp_c) > 1.0) {
            e.modelo_cp = nucleo::ModeloCp::MAIER_KELLEY;
        } else {
            e.modelo_cp = nucleo::ModeloCp::POLINOMIAL_ESTANDAR;
        }
        e.T_min = d.T_min;
        e.T_max = d.T_max;
        e.antoine_a = d.AntA;
        e.antoine_b = d.AntB;
        e.antoine_c = d.AntC;
        e.temp_min_antoine = d.AntTmin;
        e.temp_max_antoine = d.AntTmax;
        for (const auto& ca : tp.analizar_formula(e.formula)) {
            if (!ca.elem.empty() && ca.cuenta > 0)
                e.composicion_elemental[ca.elem] += ca.cuenta;
        }
        // La tabla semilla codifica 0=Gas, 1=Líquido, 2=Sólido, 3=Acuoso/Mixto;
        // el enum Fase es {Liquido=0, Gas=1, Solido=2, Mixto=3,...}. Mapear bien.
        switch (d.fase_std) {
            case 0:  e.fase_std = nucleo::Fase::Gas;     break;
            case 1:  e.fase_std = nucleo::Fase::Liquido; break;
            case 2:  e.fase_std = nucleo::Fase::Solido;  break;
            default: e.fase_std = nucleo::Fase::Mixto;   break;
        }
        especies_[e.id] = e;
        if (e.id <= 445 || nombre_a_id_especie_.find(e.nombre) == nombre_a_id_especie_.end()) {
            nombre_a_id_especie_[e.nombre] = e.id;
        }
    }
}

void GestorDB::poblar_reacciones() {
    struct DatosReaccion {
        const char* nombre;
        double dH, dS, Ea;
        int orden;
        struct Par { const char* especie; double coeff; };
        std::vector<Par> estequiometria;
    };

    std::vector<DatosReaccion> datos = {
        // 1. Reformado con vapor (SMR): CH4 + H2O -> CO + 3H2
        {"Reformado de metano con vapor", 206.130, 214.420, 240.0, 1, {{"Metano",-1},{"Agua",-1},{"Monóxido de carbono",1},{"Hidrógeno",3}}},
        // 2. Water-Gas Shift: CO + H2O -> CO2 + H2
        {"Water-Gas Shift (WGS)", -41.166, -42.382, 70.0, 1, {{"Monóxido de carbono",-1},{"Agua",-1},{"Dióxido de carbono",1},{"Hidrógeno",1}}},
        // 3. Metanación: CO2 + 4H2 -> CH4 + 2H2O
        {"Metanación de CO₂", -165.000, -90.730, 100.0, 1, {{"Dióxido de carbono",-1},{"Hidrógeno",-4},{"Metano",1},{"Agua",2}}},
        // 4. Haber-Bosch: N2 + 3H2 -> 2NH3
        {"Haber-Bosch (síntesis de amoníaco)", -91.880, -174.460, 200.0, 1, {{"Nitrógeno",-1},{"Hidrógeno",-3},{"Amoníaco",2}}},
        // 5. Oxidación de SO2: SO2 + 0.5O2 -> SO3
        {"Oxidación de dióxido de azufre", -98.920, -93.600, 80.0, 1, {{"Dióxido de azufre",-1},{"Oxígeno",-0.5},{"Trióxido de azufre",1}}},
        // 6. Combustión completa CH4: CH4 + 2O2 -> CO2 + 2H2O
        {"Combustión de metano", -802.300, -5.160, 125.0, 1, {{"Metano",-1},{"Oxígeno",-2},{"Dióxido de carbono",1},{"Agua",2}}},
        // 7. Combustión H2: H2 + 0.5O2 -> H2O
        {"Combustión de hidrógeno", -241.826, -44.420, 80.0, 1, {{"Hidrógeno",-1},{"Oxígeno",-0.5},{"Agua",1}}},
        // 8. Oxidación de CO: CO + 0.5O2 -> CO2
        {"Oxidación de monóxido de carbono", -282.985, -86.580, 90.0, 1, {{"Monóxido de carbono",-1},{"Oxígeno",-0.5},{"Dióxido de carbono",1}}},
        // 9. Formación de urea
        {"Síntesis de urea: 2NH3 + CO2 -> (NH2)2CO + H2O", -101.500, 24.300, 150.0, 1, {{"Amoníaco",-2},{"Dióxido de carbono",-1},{"Urea",1},{"Agua",1}}},
        // 10. Oxidación de amoníaco: 4NH3 + 5O2 -> 4NO + 6H2O
        {"Oxidación de amoníaco (proceso Ostwald)", -905.500, -180.000, 180.0, 1, {{"Amoníaco",-4},{"Oxígeno",-5},{"Monóxido de nitrógeno",4},{"Agua",6}}},
        // 11. Formación de NO2: 2NO + O2 -> 2NO2
        {"Oxidación de NO", -114.200, -146.500, 60.0, 1, {{"Monóxido de nitrógeno",-2},{"Oxígeno",-1},{"Dióxido de nitrógeno",2}}},
        // 12. Formación de HNO3: 3NO2 + H2O -> 2HNO3 + NO
        {"Formación de ácido nítrico", -117.000, -100.000, 100.0, 1, {{"Dióxido de nitrógeno",-3},{"Agua",-1},{"Ácido nítrico",2},{"Monóxido de nitrógeno",1}}},
        // 13. Calcinación: CaCO3 -> CaO + CO2
        {"Calcinación de carbonato de calcio", 178.300, 160.400, 200.0, 1, {{"Carbonato de calcio",-1},{"Óxido de calcio",1},{"Dióxido de carbono",1}}},
        // 14. Cloro-álcali: 2NaCl + 2H2O -> 2NaOH + Cl2 + H2
        {"Proceso cloro-álcali", 472.000, 100.000, 150.0, 1, {{"Cloruro de sodio",-2},{"Agua",-2},{"Hidróxido de sodio",2},{"Cloro",1},{"Hidrógeno",1}}},
        // 15. Oxidación de Fe: 4Fe + 3O2 -> 2Fe2O3
        {"Oxidación del hierro", -1648.400, -549.400, 100.0, 1, {{"Hierro (α)",-4},{"Oxígeno",-3},{"Óxido de hierro(III)",2}}},
        // 16. Reducción de Fe2O3 con CO: Fe2O3 + 3CO -> 2Fe + 3CO2
        {"Reducción de hematita con CO", -24.800, 15.000, 120.0, 1, {{"Óxido de hierro(III)",-1},{"Monóxido de carbono",-3},{"Hierro (α)",2},{"Dióxido de carbono",3}}},
        // 17. Producción de H2SO4: SO3 + H2O -> H2SO4
        {"Formación de ácido sulfúrico", -101.540, -180.000, 60.0, 1, {{"Trióxido de azufre",-1},{"Agua",-1},{"Ácido sulfúrico",1}}},
        // 18. Síntesis de metanol: CO + 2H2 -> CH3OH
        {"Síntesis de metanol", -90.550, -220.000, 90.0, 1, {{"Monóxido de carbono",-1},{"Hidrógeno",-2},{"Metanol",1}}},
        // 19. Craqueo de etano: C2H6 -> C2H4 + H2
        {"Craqueo de etano", 137.000, 120.000, 250.0, 1, {{"Etano",-1},{"Eteno (Etileno)",1},{"Hidrógeno",1}}},
        // 20. Esterificación: CH3COOH + C2H5OH -> CH3COOC2H5 + H2O
        {"Esterificación de ácido acético", -5.000, 10.000, 70.0, 1, {{"Ácido acético",-1},{"Etanol",-1},{"Acetato de etilo",1},{"Agua",1}}},
        // 21. Combustión de C2H6: 2C2H6 + 7O2 -> 4CO2 + 6H2O
        {"Combustión de etano", -1560.000, -20.000, 130.0, 1, {{"Etano",-2},{"Oxígeno",-7},{"Dióxido de carbono",4},{"Agua",6}}},
        // 22. Oxidación de H2S: 2H2S + 3O2 -> 2SO2 + 2H2O
        {"Oxidación de sulfuro de hidrógeno", -1036.000, -150.000, 100.0, 1, {{"Sulfuro de hidrógeno",-2},{"Oxígeno",-3},{"Dióxido de azufre",2},{"Agua",2}}},
        // 23. Descomposición de NH3: 2NH3 -> N2 + 3H2
        {"Descomposición de amoníaco", 91.880, 174.460, 180.0, 1, {{"Amoníaco",-2},{"Nitrógeno",1},{"Hidrógeno",3}}},
        // 24. Oxidación parcial de CH4: CH4 + 0.5O2 -> CO + 2H2
        {"Oxidación parcial de metano", -36.000, 80.000, 110.0, 1, {{"Metano",-1},{"Oxígeno",-0.5},{"Monóxido de carbono",1},{"Hidrógeno",2}}},
        // 25. Síntesis de cloruro de vinilo: C2H2 + HCl -> C2H3Cl
        {"Hidrocloración de acetileno", -100.000, -120.000, 80.0, 1, {{"Acetileno",-1},{"Cloruro de hidrógeno",-1},{"Cloruro de vinilo",1}}},
        // 26. Formación de ácido nítrico directa
        {"Oxidación de NO a NO₂", -114.200, -146.500, 60.0, 1, {{"Monóxido de nitrógeno",-2},{"Oxígeno",-1},{"Dióxido de nitrógeno",2}}},
        // 27. Reacción de Boudouard: C + CO2 -> 2CO
        {"Reacción de Boudouard", 172.500, 175.600, 200.0, 1, {{"Carbono",-1},{"Dióxido de carbono",-1},{"Monóxido de carbono",2}}},
        // 28. Síntesis de aminas: CH3OH + NH3 -> CH3NH2 + H2O
        {"Aminación de metanol", -30.000, -50.000, 100.0, 1, {{"Metanol",-1},{"Amoníaco",-1},{"Metilamina",1},{"Agua",1}}},
        // 29. Hidrogenación de CO2 a metanol: CO2 + 3H2 -> CH3OH + H2O
        {"Hidrogenación de CO₂ a metanol", -49.430, -177.000, 85.0, 1, {{"Dióxido de carbono",-1},{"Hidrógeno",-3},{"Metanol",1},{"Agua",1}}},
        // 30. Reacción de cloro con NaOH: Cl2 + 2NaOH -> NaCl + NaOCl + H2O
        {"Reacción de cloro con hidróxido", -200.000, -50.000, 40.0, 1, {{"Cloro",-1},{"Hidróxido de sodio",-2},{"Cloruro de sodio",1},{"Agua",1}}},
        // 31. Formación de NO a partir de N2 y O2
        {"Formación de NO (combustión)", 180.580, 24.770, 350.0, 1, {{"Nitrógeno",-1},{"Oxígeno",-1},{"Monóxido de nitrógeno",2}}},
        // 32. Combustión completa de C3H8: C3H8 + 5O2 -> 3CO2 + 4H2O
        {"Combustión de propano", -2043.000, -100.000, 140.0, 1, {{"Propano",-1},{"Oxígeno",-5},{"Dióxido de carbono",3},{"Agua",4}}},
        // 33. Fermentación: C6H12O6 -> 2C2H5OH + 2CO2
        {"Fermentación alcohólica", -68.000, 300.000, 60.0, 1, {{"Glucosa",-1},{"Etanol",2},{"Dióxido de carbono",2}}},
        // 34. Fotosíntesis: 6CO2 + 6H2O -> C6H12O6 + 6O2
        {"Fotosíntesis (simplificada)", 2803.000, -200.000, 0.0, 1, {{"Dióxido de carbono",-6},{"Agua",-6},{"Glucosa",1},{"Oxígeno",6}}},
        // 35. Digestión Bayer (Pressure Leaching): Al(OH)3 + NaOH(aq) -> NaAlO2(a) + 2 H2O(l)
        {"Digestión Bayer: Al(OH)3 + NaOH(aq) -> NaAlO2(a) + 2H2O", 32.0, 45.0, 85.0, 1, {{"Hidróxido de aluminio",-1},{"Hidróxido de sodio (aq)",-1},{"Aluminato de sodio (a)",1},{"Agua (líquida)",2}}},
        // 36. Cristalización Bayer: NaAlO2(a) + 2 H2O(l) -> Al(OH)3 + NaOH(aq)
        {"Cristalización Bayer: NaAlO2(a) + 2H2O -> Al(OH)3 + NaOH(aq)", -32.0, -45.0, 65.0, 1, {{"Aluminato de sodio (a)",-1},{"Agua (líquida)",-2},{"Hidróxido de aluminio",1},{"Hidróxido de sodio (aq)",1}}},
        // 37. Calcinación Bayer: 2 Al(OH)3 -> Al2O3 + 3 H2O(g)
        {"Calcinación Bayer: 2Al(OH)3 -> Al2O3 + 3H2O(g)", 185.0, 290.0, 140.0, 1, {{"Hidróxido de aluminio",-2},{"Óxido de aluminio",1},{"Agua",3}}},
        // 38. Cloración de TiO2: TiO2 + 2C + 2Cl2 -> TiCl4(g) + 2CO
        {"Cloración carbotérmica de TiO2", -125.0, 180.0, 150.0, 1, {{"Dióxido de titanio",-1},{"Negro de carbón",-2},{"Cloro",-2},{"Cloruro de titanio (g)",1},{"Monóxido de carbono",2}}},
        // 39. Oxidación de TiCl4: TiCl4(g) + O2 -> TiO2 + 2Cl2
        {"Oxidación de TiCl4 a TiO2", -180.0, -60.0, 110.0, 1, {{"Cloruro de titanio (g)",-1},{"Oxígeno",-1},{"Dióxido de titanio",1},{"Cloro",2}}},
        // 40. Reducción de hematita en Alto Horno: Fe2O3 + 1.5C + 0.75O2 -> 2Fe + 1.5CO2
        {"Reducción carbotérmica en Alto Horno", -165.0, 120.0, 130.0, 1, {{"Hematita",-1},{"Negro de carbón",-1.5},{"Oxígeno",-0.75},{"Hierro (α)",2},{"Dióxido de carbono",1.5}}},
    };

    for (auto& d : datos) {
        nucleo::Reaccion r;
        r.id = prox_id_reaccion_++;
        r.nombre = d.nombre;
        r.delta_h_298 = d.dH;
        r.delta_s_298 = d.dS;
        r.energia_activacion = d.Ea;
        r.orden_reaccion = d.orden;

        for (auto& par : d.estequiometria) {
            auto* sp = especie_por_nombre(par.especie);
            if (sp) r.estequiometria[sp->id] = par.coeff;
        }

        reacciones_[r.id] = r;
    }
}

// ── Consultas de especies ───────────────────────────────────────────────────

const nucleo::Especie* GestorDB::especie_por_id(int64_t id) const {
    auto it = especies_.find(id);
    return it != especies_.end() ? &it->second : nullptr;
}

const nucleo::Especie* GestorDB::especie_por_nombre(const std::string& nombre) const {
    auto it = nombre_a_id_especie_.find(nombre);
    if (it != nombre_a_id_especie_.end())
        return especie_por_id(it->second);
    return nullptr;
}

std::vector<const nucleo::Especie*> GestorDB::buscar_especies(const std::string& query) const {
    std::vector<const nucleo::Especie*> res;
    std::string q = query;
    auto lowercase = [](unsigned char c) { return static_cast<char>(std::tolower(c)); };
    std::transform(q.begin(), q.end(), q.begin(), lowercase);

    for (auto& [id, sp] : especies_) {
        std::string nom = sp.nombre;
        std::transform(nom.begin(), nom.end(), nom.begin(), lowercase);
        std::string form = sp.formula;
        std::transform(form.begin(), form.end(), form.begin(), lowercase);
        if (nom.find(q) != std::string::npos || form.find(q) != std::string::npos)
            res.push_back(&sp);
    }

    std::sort(res.begin(), res.end(), [&](const nucleo::Especie* a, const nucleo::Especie* b) {
        const bool a_curated = (a->id <= 445 || a->provenance.revision != "synthetic-estimate");
        const bool b_curated = (b->id <= 445 || b->provenance.revision != "synthetic-estimate");
        if (a_curated != b_curated) return a_curated;

        std::string a_form = a->formula;
        std::transform(a_form.begin(), a_form.end(), a_form.begin(), lowercase);
        std::string b_form = b->formula;
        std::transform(b_form.begin(), b_form.end(), b_form.begin(), lowercase);
        const bool a_exact_form = (a_form == q);
        const bool b_exact_form = (b_form == q);
        if (a_exact_form != b_exact_form) return a_exact_form;

        std::string a_nom = a->nombre;
        std::transform(a_nom.begin(), a_nom.end(), a_nom.begin(), lowercase);
        std::string b_nom = b->nombre;
        std::transform(b_nom.begin(), b_nom.end(), b_nom.begin(), lowercase);
        const bool a_exact_nom = (a_nom == q);
        const bool b_exact_nom = (b_nom == q);
        if (a_exact_nom != b_exact_nom) return a_exact_nom;

        return a->id < b->id;
    });

    return res;
}

std::vector<const nucleo::Especie*> GestorDB::todas_especies() const {
    std::vector<const nucleo::Especie*> res;
    for (auto& [id, sp] : especies_) res.push_back(&sp);
    return res;
}

std::vector<const nucleo::Especie*> GestorDB::especies_curadas() const {
    std::vector<const nucleo::Especie*> res;
    for (const auto& [id, sp] : especies_) {
        if (sp.id <= 445 || sp.provenance.revision != "synthetic-estimate") {
            res.push_back(&sp);
        }
    }
    return res;
}


const nucleo::Reaccion* GestorDB::reaccion_por_id(int64_t id) const {
    auto it = reacciones_.find(id);
    return it != reacciones_.end() ? &it->second : nullptr;
}

std::vector<const nucleo::Reaccion*> GestorDB::todas_reacciones() const {
    std::vector<const nucleo::Reaccion*> res;
    for (auto& [id, r] : reacciones_) res.push_back(&r);
    return res;
}

const nucleo::Elemento* GestorDB::elemento_por_simbolo(const std::string& simb) const {
    auto& tp = nucleo::TablaPeriodica::instancia();
    return tp.por_simbolo(simb);
}

const nucleo::Elemento* GestorDB::elemento_por_numero(int z) const {
    auto& tp = nucleo::TablaPeriodica::instancia();
    return tp.por_numero(z);
}

const nucleo::Elemento* GestorDB::elemento_por_nombre(const std::string& nombre) const {
    auto& tp = nucleo::TablaPeriodica::instancia();
    return tp.por_nombre(nombre);
}

std::vector<const nucleo::Elemento*> GestorDB::todos_elementos() const {
    auto& tp = nucleo::TablaPeriodica::instancia();
    std::vector<const nucleo::Elemento*> res;
    for (auto& e : tp.todos()) res.push_back(&e);
    return res;
}

bool GestorDB::exportar_sqlite(const std::string& ruta) const {
    sqlite3* raw = nullptr;
    if (sqlite3_open(ruta.c_str(), &raw) != SQLITE_OK) { if (raw) sqlite3_close(raw); return false; }
    std::unique_ptr<sqlite3, decltype(&sqlite3_close)> db(raw, sqlite3_close);
    auto exec = [&](const char* sql) { return sqlite3_exec(raw, sql, nullptr, nullptr, nullptr) == SQLITE_OK; };
    if (!exec("BEGIN IMMEDIATE;")) {
        return false;
    }

    // Asegurar migración de esquema si la tabla cma_species ya existía en formato legacy (11 columnas)
    sqlite3_stmt* check_stmt = nullptr;
    if (sqlite3_prepare_v2(raw, "PRAGMA table_info(cma_species)", -1, &check_stmt, nullptr) == SQLITE_OK) {
        bool has_table = false;
        bool has_cp_model = false;
        while (sqlite3_step(check_stmt) == SQLITE_ROW) {
            has_table = true;
            const unsigned char* col_name = sqlite3_column_text(check_stmt, 1);
            if (col_name && std::string(reinterpret_cast<const char*>(col_name)) == "cp_model") {
                has_cp_model = true;
            }
        }
        sqlite3_finalize(check_stmt);
        if (has_table && !has_cp_model) {
            exec("ALTER TABLE cma_species ADD COLUMN cp_model INTEGER DEFAULT 0; "
                 "ALTER TABLE cma_species ADD COLUMN cp_e REAL DEFAULT 0.0; "
                 "ALTER TABLE cma_species ADD COLUMN t_min REAL DEFAULT 298.15; "
                 "ALTER TABLE cma_species ADD COLUMN t_max REAL DEFAULT 2000.0;");
        }
    }

    if (!exec("CREATE TABLE IF NOT EXISTS cma_species ("
              "id INTEGER PRIMARY KEY, name TEXT, formula TEXT, molar_mass REAL, "
              "hf REAL, sf REAL, phase INTEGER, cp_a REAL, cp_b REAL, cp_c REAL, cp_d REAL, "
              "cp_model INTEGER DEFAULT 0, cp_e REAL DEFAULT 0.0, "
              "t_min REAL DEFAULT 298.15, t_max REAL DEFAULT 2000.0); "
              "CREATE TABLE IF NOT EXISTS cma_species_provenance ("
              "id INTEGER PRIMARY KEY, source TEXT NOT NULL, reference TEXT, revision TEXT, "
              "quality INTEGER NOT NULL, min_kelvin REAL, max_kelvin REAL); "
              "DELETE FROM cma_species; DELETE FROM cma_species_provenance;")) {
        exec("ROLLBACK"); return false;
    }
    sqlite3_stmt* statement = nullptr;
    if (sqlite3_prepare_v2(raw, "INSERT INTO cma_species VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", -1, &statement, nullptr) != SQLITE_OK) {
        exec("ROLLBACK"); return false;
    }
    std::unique_ptr<sqlite3_stmt, decltype(&sqlite3_finalize)> stmt(statement, sqlite3_finalize);
    sqlite3_stmt* provenance_statement=nullptr;
    if (sqlite3_prepare_v2(raw,"INSERT INTO cma_species_provenance VALUES (?,?,?,?,?,?,?)",-1,&provenance_statement,nullptr)!=SQLITE_OK) {
        exec("ROLLBACK"); return false;
    }
    std::unique_ptr<sqlite3_stmt,decltype(&sqlite3_finalize)> provenance_stmt(provenance_statement,sqlite3_finalize);
    for (const auto& [id, species] : especies_) {
        sqlite3_bind_int64(statement, 1, id);
        sqlite3_bind_text(statement, 2, species.nombre.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(statement, 3, species.formula.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_double(statement, 4, species.masa_molar);
        sqlite3_bind_double(statement, 5, species.h_formacion);
        sqlite3_bind_double(statement, 6, species.s_formacion);
        sqlite3_bind_int(statement, 7, static_cast<int>(species.fase_std));
        sqlite3_bind_double(statement, 8, species.cp_a);
        sqlite3_bind_double(statement, 9, species.cp_b);
        sqlite3_bind_double(statement, 10, species.cp_c);
        sqlite3_bind_double(statement, 11, species.cp_d);
        sqlite3_bind_int(statement, 12, static_cast<int>(species.modelo_cp));
        sqlite3_bind_double(statement, 13, species.cp_e);
        sqlite3_bind_double(statement, 14, species.T_min);
        sqlite3_bind_double(statement, 15, species.T_max);
        if (sqlite3_step(statement) != SQLITE_DONE) { exec("ROLLBACK"); return false; }
        sqlite3_reset(statement);
        sqlite3_clear_bindings(statement);
        const auto& provenance=species.provenance;
        sqlite3_bind_int64(provenance_statement,1,id);
        sqlite3_bind_text(provenance_statement,2,provenance.source.c_str(),-1,SQLITE_TRANSIENT);
        sqlite3_bind_text(provenance_statement,3,provenance.reference.c_str(),-1,SQLITE_TRANSIENT);
        sqlite3_bind_text(provenance_statement,4,provenance.revision.c_str(),-1,SQLITE_TRANSIENT);
        sqlite3_bind_int(provenance_statement,5,static_cast<int>(provenance.quality));
        if (provenance.min_kelvin>0) sqlite3_bind_double(provenance_statement,6,provenance.min_kelvin);
        else sqlite3_bind_null(provenance_statement,6);
        if (provenance.max_kelvin>0) sqlite3_bind_double(provenance_statement,7,provenance.max_kelvin);
        else sqlite3_bind_null(provenance_statement,7);
        if (sqlite3_step(provenance_statement)!=SQLITE_DONE) { exec("ROLLBACK"); return false; }
        sqlite3_reset(provenance_statement); sqlite3_clear_bindings(provenance_statement);
    }
    if (!exec("COMMIT")) { exec("ROLLBACK"); return false; }
    return true;
}

bool GestorDB::importar_sqlite(const std::string& ruta) {
    sqlite3* raw = nullptr;
    if (sqlite3_open(ruta.c_str(), &raw) != SQLITE_OK) {
        if (raw) sqlite3_close(raw);
        return false;
    }
    std::unique_ptr<sqlite3, decltype(&sqlite3_close)> db(raw, sqlite3_close);

    // Detectar presencia de columnas modernas en cma_species
    sqlite3_stmt* info_stmt = nullptr;
    if (sqlite3_prepare_v2(raw, "PRAGMA table_info(cma_species)", -1, &info_stmt, nullptr) != SQLITE_OK) {
        return false;
    }
    bool has_cp_model = false;
    while (sqlite3_step(info_stmt) == SQLITE_ROW) {
        const unsigned char* col_name = sqlite3_column_text(info_stmt, 1);
        if (col_name && std::string(reinterpret_cast<const char*>(col_name)) == "cp_model") {
            has_cp_model = true;
        }
    }
    sqlite3_finalize(info_stmt);

    const std::string query = has_cp_model
        ? "SELECT id, name, formula, molar_mass, hf, sf, phase, cp_a, cp_b, cp_c, cp_d, cp_model, cp_e, t_min, t_max FROM cma_species ORDER BY id"
        : "SELECT id, name, formula, molar_mass, hf, sf, phase, cp_a, cp_b, cp_c, cp_d FROM cma_species ORDER BY id";

    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2(raw, query.c_str(), -1, &stmt, nullptr) != SQLITE_OK) {
        return false;
    }
    std::unique_ptr<sqlite3_stmt, decltype(&sqlite3_finalize)> species_stmt(stmt, sqlite3_finalize);

    const auto& tp = nucleo::TablaPeriodica::instancia();
    std::map<int64_t, nucleo::Especie> loaded_species;
    std::map<std::string, int64_t> loaded_name_map;

    while (sqlite3_step(stmt) == SQLITE_ROW) {
        nucleo::Especie sp;
        sp.id = sqlite3_column_int64(stmt, 0);
        const unsigned char* n = sqlite3_column_text(stmt, 1);
        const unsigned char* f = sqlite3_column_text(stmt, 2);
        sp.nombre = n ? reinterpret_cast<const char*>(n) : "";
        sp.formula = f ? reinterpret_cast<const char*>(f) : "";
        sp.masa_molar = sqlite3_column_double(stmt, 3);
        sp.h_formacion = sqlite3_column_double(stmt, 4);
        sp.s_formacion = sqlite3_column_double(stmt, 5);
        int phase_int = sqlite3_column_int(stmt, 6);
        switch (phase_int) {
            case 0: sp.fase_std = nucleo::Fase::Liquido; break;
            case 1: sp.fase_std = nucleo::Fase::Gas;     break;
            case 2: sp.fase_std = nucleo::Fase::Solido;  break;
            case 3: sp.fase_std = nucleo::Fase::Mixto;   break;
            default: sp.fase_std = nucleo::Fase::Desconocido; break;
        }
        sp.cp_a = sqlite3_column_double(stmt, 7);
        sp.cp_b = sqlite3_column_double(stmt, 8);
        sp.cp_c = sqlite3_column_double(stmt, 9);
        sp.cp_d = sqlite3_column_double(stmt, 10);
        if (has_cp_model) {
            sp.modelo_cp = static_cast<nucleo::ModeloCp>(sqlite3_column_int(stmt, 11));
            sp.cp_e = sqlite3_column_double(stmt, 12);
            sp.T_min = sqlite3_column_double(stmt, 13);
            sp.T_max = sqlite3_column_double(stmt, 14);
        } else {
            sp.modelo_cp = (std::abs(sp.cp_c) > 1.0) ? nucleo::ModeloCp::MAIER_KELLEY : nucleo::ModeloCp::POLINOMIAL_ESTANDAR;
            sp.cp_e = 0.0;
            sp.T_min = 298.15;
            sp.T_max = 2000.0;
        }
        for (const auto& ca : tp.analizar_formula(sp.formula)) {
            if (!ca.elem.empty() && ca.cuenta > 0)
                sp.composicion_elemental[ca.elem] += ca.cuenta;
        }
        loaded_species[sp.id] = sp;
        if (sp.id <= 445 || loaded_name_map.find(sp.nombre) == loaded_name_map.end()) {
            loaded_name_map[sp.nombre] = sp.id;
        }
    }

    // Cargar procedencia
    sqlite3_stmt* prov_stmt = nullptr;
    if (sqlite3_prepare_v2(raw, "SELECT id, source, reference, revision, quality, min_kelvin, max_kelvin FROM cma_species_provenance", -1, &prov_stmt, nullptr) == SQLITE_OK) {
        std::unique_ptr<sqlite3_stmt, decltype(&sqlite3_finalize)> p_stmt(prov_stmt, sqlite3_finalize);
        while (sqlite3_step(prov_stmt) == SQLITE_ROW) {
            int64_t pid = sqlite3_column_int64(prov_stmt, 0);
            if (auto it = loaded_species.find(pid); it != loaded_species.end()) {
                const unsigned char* s = sqlite3_column_text(prov_stmt, 1);
                const unsigned char* r = sqlite3_column_text(prov_stmt, 2);
                const unsigned char* rev = sqlite3_column_text(prov_stmt, 3);
                it->second.provenance.source = s ? reinterpret_cast<const char*>(s) : "";
                it->second.provenance.reference = r ? reinterpret_cast<const char*>(r) : "";
                it->second.provenance.revision = rev ? reinterpret_cast<const char*>(rev) : "";
                it->second.provenance.quality = static_cast<domain::DataQuality>(sqlite3_column_int(prov_stmt, 4));
                if (sqlite3_column_type(prov_stmt, 5) != SQLITE_NULL)
                    it->second.provenance.min_kelvin = sqlite3_column_double(prov_stmt, 5);
                if (sqlite3_column_type(prov_stmt, 6) != SQLITE_NULL)
                    it->second.provenance.max_kelvin = sqlite3_column_double(prov_stmt, 6);
            }
        }
    }

    if (loaded_species.empty()) return false;
    especies_ = std::move(loaded_species);
    nombre_a_id_especie_ = std::move(loaded_name_map);
    return true;
}

bool GestorDB::cargar_desde_csv(const std::string& ruta) {
    std::ifstream archivo(ruta);
    if (!archivo) return false;
    std::string line;
    if (!std::getline(archivo, line)) return false;
    // UTF-8 BOM is allowed only on the header.
    if (line.compare(0, 3, "\xEF\xBB\xBF") == 0) line.erase(0, 3);
    std::vector<nucleo::Especie> pending;
    try {
        const auto header = adapters::csv_row(line);
        if (header.size() < 6 || header[0] != "Nombre" || header[1] != "Formula") return false;
        while (std::getline(archivo, line)) {
            if (line.empty() || line[0] == '#' || line[0] == '/') continue;
            const auto fields = adapters::csv_row(line);
            if (fields.size() != header.size() || fields[0].empty() || fields[1].empty()) return false;
            nucleo::Especie species;
            species.nombre = fields[0]; species.formula = fields[1];
            species.provenance.source = ruta;
            species.provenance.revision = "CSV import; Hf kJ/mol, Sf J/(mol K), Cp unavailable";
            species.masa_molar = adapters::csv_number(fields[2]);
            species.h_formacion = adapters::csv_number(fields[3]);
            species.s_formacion = adapters::csv_number(fields[4]);
            const double phase = adapters::csv_number(fields[5]);
            if (species.masa_molar < 0 || phase != std::floor(phase) || phase < 0 || phase > 3) return false;
            const nucleo::Fase phases[] = {nucleo::Fase::Gas, nucleo::Fase::Liquido, nucleo::Fase::Solido, nucleo::Fase::Mixto};
            species.fase_std = phases[static_cast<int>(phase)];
            // Cp is deliberately unspecified in the source CSV; never fabricate it.
            pending.push_back(std::move(species));
        }
    } catch (const std::exception&) { return false; }
    if (archivo.bad() || pending.empty()) return false;
    // Parse first, then commit atomically. Reimporting the same data is idempotent.
    auto species_map = especies_;
    auto names = nombre_a_id_especie_;
    int64_t next = species_map.empty() ? 1 : species_map.rbegin()->first + 1;
    for (auto& species : pending) {
        auto existing = names.find(species.nombre);
        if (existing != names.end()) {
            const auto& previous = species_map.at(existing->second);
            if (previous.formula != species.formula || previous.fase_std != species.fase_std) return false;
            continue; // retain seeded coefficients and stable project IDs
        }
        species.id = next++;
        names[species.nombre] = species.id;
        species_map[species.id] = species;
    }
    especies_.swap(species_map);
    nombre_a_id_especie_.swap(names);
    prox_id_especie_ = next;
    return true;
}

}} // namespace


