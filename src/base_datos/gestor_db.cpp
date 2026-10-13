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
    for (auto& d : semilla_especies()) {
        nucleo::Especie e;
        e.id = prox_id_especie_++;
        e.provenance.source = "seed:datos_especies.hpp record " + std::to_string(e.id);
        e.provenance.revision = "legacy-import; reference pending";
        e.nombre = d.nombre;
        e.formula = d.formula;
        e.masa_molar = d.M;
        e.h_formacion = d.Hf;
        e.s_formacion = d.Sf;
        e.cp_a = d.Cp_a;
        e.cp_b = d.Cp_b;
        e.cp_c = d.Cp_c;
        e.cp_d = d.Cp_d;
        e.antoine_a = d.AntA;
        e.antoine_b = d.AntB;
        e.antoine_c = d.AntC;
        e.temp_min_antoine = d.AntTmin;
        e.temp_max_antoine = d.AntTmax;
        // La tabla semilla codifica 0=Gas, 1=Líquido, 2=Sólido, 3=Acuoso/Mixto;
        // el enum Fase es {Liquido=0, Gas=1, Solido=2, Mixto=3,...}. Mapear bien.
        switch (d.fase_std) {
            case 0:  e.fase_std = nucleo::Fase::Gas;     break;
            case 1:  e.fase_std = nucleo::Fase::Liquido; break;
            case 2:  e.fase_std = nucleo::Fase::Solido;  break;
            default: e.fase_std = nucleo::Fase::Mixto;   break;
        }
        especies_[e.id] = e;
        nombre_a_id_especie_[e.nombre] = e.id;
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
        // Reformado con vapor (SMR): CH4 + H2O -> CO + 3H2
        {"Reformado de metano con vapor", 206.130, 214.420, 240.0, 1, {{"Metano",-1},{"Agua",-1},{"Monóxido de carbono",1},{"Hidrógeno",3}}},
        // Water-Gas Shift: CO + H2O -> CO2 + H2
        {"Water-Gas Shift (WGS)", -41.166, -42.382, 70.0, 1, {{"Monóxido de carbono",-1},{"Agua",-1},{"Dióxido de carbono",1},{"Hidrógeno",1}}},
        // Metanación: CO2 + 4H2 -> CH4 + 2H2O
        {"Metanación de CO₂", -165.000, -90.730, 100.0, 1, {{"Dióxido de carbono",-1},{"Hidrógeno",-4},{"Metano",1},{"Agua",2}}},
        // Haber-Bosch: N2 + 3H2 -> 2NH3
        {"Haber-Bosch (síntesis de amoníaco)", -91.880, -174.460, 200.0, 1, {{"Nitrógeno",-1},{"Hidrógeno",-3},{"Amoníaco",2}}},
        // Oxidación de SO2: SO2 + 0.5O2 -> SO3
        {"Oxidación de dióxido de azufre", -98.920, -93.600, 80.0, 1, {{"Dióxido de azufre",-1},{"Oxígeno",-0.5},{"Trióxido de azufre",1}}},
        // Combustión completa CH4: CH4 + 2O2 -> CO2 + 2H2O
        {"Combustión de metano", -802.300, -5.160, 125.0, 1, {{"Metano",-1},{"Oxígeno",-2},{"Dióxido de carbono",1},{"Agua",2}}},
        // Combustión H2: H2 + 0.5O2 -> H2O
        {"Combustión de hidrógeno", -241.826, -44.420, 80.0, 1, {{"Hidrógeno",-1},{"Oxígeno",-0.5},{"Agua",1}}},
        // Oxidación de CO: CO + 0.5O2 -> CO2
        {"Oxidación de monóxido de carbono", -282.985, -86.580, 90.0, 1, {{"Monóxido de carbono",-1},{"Oxígeno",-0.5},{"Dióxido de carbono",1}}},
        // Formación de amoníaco a partir de elementos (otra vista)
        {"Síntesis de urea: 2NH3 + CO2 -> (NH2)2CO + H2O", -101.500, 24.300, 150.0, 1, {{"Amoníaco",-2},{"Dióxido de carbono",-1},{"Urea",1},{"Agua",1}}},
        // Oxidación de amoníaco: 4NH3 + 5O2 -> 4NO + 6H2O
        {"Oxidación de amoníaco (proceso Ostwald)", -905.500, -180.000, 180.0, 1, {{"Amoníaco",-4},{"Oxígeno",-5},{"Monóxido de nitrógeno",4},{"Agua",6}}},
        // Formación de NO2: 2NO + O2 -> 2NO2
        {"Oxidación de NO", -114.200, -146.500, 60.0, 1, {{"Monóxido de nitrógeno",-2},{"Oxígeno",-1},{"Dióxido de nitrógeno",2}}},
        // Formación de HNO3: 3NO2 + H2O -> 2HNO3 + NO
        {"Formación de ácido nítrico", -117.000, -100.000, 100.0, 1, {{"Dióxido de nitrógeno",-3},{"Agua",-1},{"Ácido nítrico",2},{"Monóxido de nitrógeno",1}}},
        // Calcinación: CaCO3 -> CaO + CO2
        {"Calcinación de carbonato de calcio", 178.300, 160.400, 200.0, 1, {{"Carbonato de calcio",-1},{"Óxido de calcio",1},{"Dióxido de carbono",1}}},
        // Cloro-álcali: 2NaCl + 2H2O -> 2NaOH + Cl2 + H2
        {"Proceso cloro-álcali", 472.000, 100.000, 150.0, 1, {{"Cloruro de sodio",-2},{"Agua",-2},{"Hidróxido de sodio",2},{"Cloro",1},{"Hidrógeno",1}}},
        // Oxidación de Fe: 4Fe + 3O2 -> 2Fe2O3
        {"Oxidación del hierro", -1648.400, -549.400, 100.0, 1, {{"Hierro (α)",-4},{"Oxígeno",-3},{"Óxido de hierro(III)",2}}},
        // Reducción de Fe2O3 con CO: Fe2O3 + 3CO -> 2Fe + 3CO2
        {"Reducción de hematita con CO", -24.800, 15.000, 120.0, 1, {{"Óxido de hierro(III)",-1},{"Monóxido de carbono",-3},{"Hierro (α)",2},{"Dióxido de carbono",3}}},
        // Producción de H2SO4: SO3 + H2O -> H2SO4
        {"Formación de ácido sulfúrico", -101.540, -180.000, 60.0, 1, {{"Trióxido de azufre",-1},{"Agua",-1},{"Ácido sulfúrico",1}}},
        // Síntesis de metanol: CO + 2H2 -> CH3OH
        {"Síntesis de metanol", -90.550, -220.000, 90.0, 1, {{"Monóxido de carbono",-1},{"Hidrógeno",-2},{"Metanol",1}}},
        // Craqueo de etano: C2H6 -> C2H4 + H2
        {"Craqueo de etano", 137.000, 120.000, 250.0, 1, {{"Etano",-1},{"Eteno (Etileno)",1},{"Hidrógeno",1}}},
        // Esterificación: CH3COOH + C2H5OH -> CH3COOC2H5 + H2O
        {"Esterificación de ácido acético", -5.000, 10.000, 70.0, 1, {{"Ácido acético",-1},{"Etanol",-1},{"Acetato de etilo",1},{"Agua",1}}},
        // Combustión de C2H6: 2C2H6 + 7O2 -> 4CO2 + 6H2O
        {"Combustión de etano", -1560.000, -20.000, 130.0, 1, {{"Etano",-2},{"Oxígeno",-7},{"Dióxido de carbono",4},{"Agua",6}}},
        // Oxidación de H2S: 2H2S + 3O2 -> 2SO2 + 2H2O
        {"Oxidación de sulfuro de hidrógeno", -1036.000, -150.000, 100.0, 1, {{"Sulfuro de hidrógeno",-2},{"Oxígeno",-3},{"Dióxido de azufre",2},{"Agua",2}}},
        // Descomposición de NH3: 2NH3 -> N2 + 3H2
        {"Descomposición de amoníaco", 91.880, 174.460, 180.0, 1, {{"Amoníaco",-2},{"Nitrógeno",1},{"Hidrógeno",3}}},
        // Oxidación parcial de CH4: CH4 + 0.5O2 -> CO + 2H2
        {"Oxidación parcial de metano", -36.000, 80.000, 110.0, 1, {{"Metano",-1},{"Oxígeno",-0.5},{"Monóxido de carbono",1},{"Hidrógeno",2}}},
        // Síntesis de cloruro de vinilo: C2H2 + HCl -> C2H3Cl
        {"Hidrocloración de acetileno", -100.000, -120.000, 80.0, 1, {{"Acetileno",-1},{"Cloruro de hidrógeno",-1},{"Cloruro de vinilo",1}}},
        // Formación de ácido nítrico directa
        {"Oxidación de NO a NO₂", -114.200, -146.500, 60.0, 1, {{"Monóxido de nitrógeno",-2},{"Oxígeno",-1},{"Dióxido de nitrógeno",2}}},
        // Reacción de Boudouard: C + CO2 -> 2CO
        {"Reacción de Boudouard", 172.500, 175.600, 200.0, 1, {{"Carbono",-1},{"Dióxido de carbono",-1},{"Monóxido de carbono",2}}},
        // Síntesis de aminas: CH3OH + NH3 -> CH3NH2 + H2O
        {"Aminación de metanol", -30.000, -50.000, 100.0, 1, {{"Metanol",-1},{"Amoníaco",-1},{"Metilamina",1},{"Agua",1}}},
        // Hidrogenación de CO2 a metanol: CO2 + 3H2 -> CH3OH + H2O
        {"Hidrogenación de CO₂ a metanol", -49.430, -177.000, 85.0, 1, {{"Dióxido de carbono",-1},{"Hidrógeno",-3},{"Metanol",1},{"Agua",1}}},
        // Reacción de cloro con NaOH: Cl2 + 2NaOH -> NaCl + NaOCl + H2O
        {"Reacción de cloro con hidróxido", -200.000, -50.000, 40.0, 1, {{"Cloro",-1},{"Hidróxido de sodio",-2},{"Cloruro de sodio",1},{"Agua",1}}},
        // Formación de NO a partir de N2 y O2
        {"Formación de NO (combustión)", 180.580, 24.770, 350.0, 1, {{"Nitrógeno",-1},{"Oxígeno",-1},{"Monóxido de nitrógeno",2}}},
        // Combustión completa de C3H8: C3H8 + 5O2 -> 3CO2 + 4H2O
        {"Combustión de propano", -2043.000, -100.000, 140.0, 1, {{"Propano",-1},{"Oxígeno",-5},{"Dióxido de carbono",3},{"Agua",4}}},
        // Fermentación: C6H12O6 -> 2C2H5OH + 2CO2
        {"Fermentación alcohólica", -68.000, 300.000, 60.0, 1, {{"Glucosa",-1},{"Etanol",2},{"Dióxido de carbono",2}}},
        // Fotosíntesis: 6CO2 + 6H2O -> C6H12O6 + 6O2
        {"Fotosíntesis (simplificada)", 2803.000, -200.000, 0.0, 1, {{"Dióxido de carbono",-6},{"Agua",-6},{"Glucosa",1},{"Oxígeno",6}}},
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
    return res;
}

std::vector<const nucleo::Especie*> GestorDB::todas_especies() const {
    std::vector<const nucleo::Especie*> res;
    for (auto& [id, sp] : especies_) res.push_back(&sp);
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
    if (!exec("BEGIN IMMEDIATE; CREATE TABLE IF NOT EXISTS cma_species (id INTEGER PRIMARY KEY, name TEXT, formula TEXT, molar_mass REAL, hf REAL, sf REAL, phase INTEGER, cp_a REAL, cp_b REAL, cp_c REAL, cp_d REAL); CREATE TABLE IF NOT EXISTS cma_species_provenance (id INTEGER PRIMARY KEY, source TEXT NOT NULL, reference TEXT, revision TEXT, quality INTEGER NOT NULL, min_kelvin REAL, max_kelvin REAL); DELETE FROM cma_species; DELETE FROM cma_species_provenance;")) {
        exec("ROLLBACK"); return false;
    }
    sqlite3_stmt* statement = nullptr;
    if (sqlite3_prepare_v2(raw, "INSERT INTO cma_species VALUES (?,?,?,?,?,?,?,?,?,?,?)", -1, &statement, nullptr) != SQLITE_OK) {
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


