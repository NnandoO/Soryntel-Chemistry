#include "base_datos/gestor_db.hpp"
#include "base_datos/datos_especies.hpp"
#include <algorithm>
#include <sstream>
#include <fstream>
#include <iostream>
#include <cctype>

namespace cma { namespace base_datos {

GestorDB& GestorDB::instancia() {
    static GestorDB db;
    if (!db.inicializada_) {
        db.poblar_elementos();
        db.poblar_especies();
        db.poblar_reacciones();
        db.inicializada_ = true;
    }
    return db;
}

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
    std::transform(q.begin(), q.end(), q.begin(), ::tolower);

    for (auto& [id, sp] : especies_) {
        std::string nom = sp.nombre;
        std::transform(nom.begin(), nom.end(), nom.begin(), ::tolower);
        std::string form = sp.formula;
        std::transform(form.begin(), form.end(), form.begin(), ::tolower);
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
    // Placeholder - exportaría a SQLite si estuviera disponible
    return false;
}

bool GestorDB::cargar_desde_csv(const std::string& ruta) {
    std::ifstream archivo(ruta);
    if (!archivo.is_open()) {
        std::cerr << "Error al abrir CSV: " << ruta << std::endl;
        return false;
    }
    
    std::string linea;
    bool primera_linea = true;
    
    // Asumimos un ID dinámico que empiece después de los hardcodeados
    int64_t next_id = 10000;
    if (!especies_.empty()) {
        next_id = especies_.rbegin()->first + 1;
    }
    
    while (std::getline(archivo, linea)) {
        if (primera_linea) {
            primera_linea = false;
            continue;
        }
        
        if (linea.empty() || linea[0] == '#' || linea[0] == '/') continue;
        
        std::stringstream ss(linea);
        std::string token;
        std::vector<std::string> columnas;
        
        while (std::getline(ss, token, ',')) {
            size_t start = token.find_first_not_of(" \r\n\t");
            size_t end = token.find_last_not_of(" \r\n\t");
            if (start != std::string::npos && end != std::string::npos) {
                columnas.push_back(token.substr(start, end - start + 1));
            } else {
                columnas.push_back("");
            }
        }
        
        if (columnas.size() >= 6) {
            nucleo::Especie esp;
            esp.id = next_id++;
            esp.nombre = columnas[0];
            esp.formula = columnas[1];
            try {
                esp.masa_molar = std::stod(columnas[2]);
                esp.h_formacion = std::stod(columnas[3]);
                esp.s_formacion = std::stod(columnas[4]);
                int f = std::stoi(columnas[5]);
                if (f == 0) esp.fase_std = nucleo::Fase::Gas;
                else if (f == 1) esp.fase_std = nucleo::Fase::Liquido;
                else esp.fase_std = nucleo::Fase::Solido;
                
                especies_[esp.id] = esp;
                nombre_a_id_especie_[esp.nombre] = esp.id;
            } catch (const std::exception& e) {
                std::cerr << "Error parseando CSV en linea: " << linea << std::endl;
            }
        }
    }
    
    return true;
}

}} // namespace


