#pragma once
#include "nucleo/tipos.hpp"
#include "diagrama_flujo/flowsheet.hpp"
#include <vector>
#include <map>
#include <string>

namespace cma { namespace motores {

struct ResultadoExergiaEspecie {
    int64_t id_especie = 0;
    std::string nombre;
    double cantidad_kmol = 0.0;
    double fraccion_molar = 0.0;
    double ex_che_std = 0.0; // Exergía química estándar (kJ/mol)
    double ex_che = 0.0;     // Exergía química incluyendo mezcla (kJ/mol)
    double ex_phy = 0.0;     // Exergía física (kJ/mol)
    double ex_tot = 0.0;     // Exergía total (kJ/mol)
    double ex_tot_flujo = 0.0; // Exergía total de la especie en el flujo (kJ)
};

struct ResultadoExergiaFlujo {
    int64_t id_flujo = 0;
    std::string origen;
    std::string destino;
    double masa_kg = 0.0;
    double cantidad_total_kmol = 0.0;
    double temperatura = 298.15;
    double presion = 1.0;
    double entropia = 0.0;
    double entalpia = 0.0;
    double entalpia_total = 0.0; // kJ
    std::vector<ResultadoExergiaEspecie> especies;
    double ex_tot_flujo = 0.0; // kJ
};

struct ResultadoBalanceExergia {
    std::vector<ResultadoExergiaFlujo> flujos;
    double calor_out = 0.0; // kJ/s
    double calor_loss = 0.0; // kJ/s
    double electricidad_out = 0.0; // kW
    
    // Globales
    double exergia_total_in = 0.0; // kJ/s
    double exergia_total_out = 0.0; // kJ/s
    double exergia_destruida = 0.0; // kJ/s
    double eficiencia_exergetica = 0.0; // %
};

class BalanceExergia {
public:
    BalanceExergia() { inicializar_db_exergia(); }

    ResultadoBalanceExergia calcular_flowsheet(
        const diagrama_flujo::Flowsheet& fs,
        const std::map<int64_t, nucleo::Especie>& db
    );

    double obtener_exergia_quimica_std(const std::string& nombre) const;

private:
    void inicializar_db_exergia();
    
    std::map<std::string, double> exergia_quimica_db_; // Nombre de especie -> kJ/mol
};

}} // namespace
