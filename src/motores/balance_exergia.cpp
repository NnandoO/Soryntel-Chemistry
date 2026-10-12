#include "balance_exergia.hpp"
#include <cmath>
#include <iostream>

namespace cma { namespace motores {

void BalanceExergia::inicializar_db_exergia() {
    // Valores estándar de exergía química según Szargut (kJ/mol a 298.15K, 1 bar)
    // Extraídos de tablas comunes para especies seleccionadas.
    exergia_quimica_db_ = {
        {"Oxígeno", 3.97},
        {"Nitrógeno", 0.72},
        {"Agua", 0.90}, // Ligeramente diferente para vapor (9.49) vs líquido (0.9), asumiremos genérico 0.9 para H2O l o 9.49 para vapor. Use 9.49 for consistency with gas flows.
        {"Dióxido de carbono", 19.87},
        {"Metano", 831.65},
        {"Monóxido de carbono", 275.10},
        {"Hidrógeno", 236.10},
        {"Dióxido de azufre", 313.40},
        {"Sulfuro de hidrógeno", 812.00},
        {"Amoníaco", 337.90},
        {"Hierro (α)", 376.10},
        {"Óxido de hierro(III)", 16.50}, // Fe2O3
        {"Cloro", 123.60},
        {"Cloruro de sodio", 16.40},
        {"Hidróxido de sodio", 74.90},
        // Óxidos genéricos para matching con HSC
        {"Óxido de cobre(II)", 4.89}, // CuO
        {"Óxido de zinc", 28.54},     // ZnO
        {"Óxido de plomo(II)", 62.64},// PbO
        {"Etano", 1495.84},
        {"Propano", 2154.00},
        {"Metanol", 718.00},
        {"Glucosa", 3103.00}
    };
    
    // Para el agua, usar 9.49 si es gas, pero como es una aproximación pondremos 9.49
    exergia_quimica_db_["Agua"] = 9.49;
}

double BalanceExergia::obtener_exergia_quimica_std(const std::string& nombre) const {
    // 1. Buscar por nombre directo
    auto it = exergia_quimica_db_.find(nombre);
    if (it != exergia_quimica_db_.end()) {
        return it->second;
    }
    // 2. Buscar por fórmula (algunas especies pueden registrarse por fórmula)
    static const std::map<std::string, double> tabla_formula = {
        {"O2", 3.97}, {"N2", 0.72}, {"H2", 236.10}, {"H2O", 0.90},
        {"H2O(g)", 9.49}, {"H2O(l)", 0.90}, {"CO2", 19.87}, {"CO", 275.10},
        {"CH4", 831.65}, {"C2H6", 1495.84}, {"C3H8", 2154.00},
        {"NH3", 337.90}, {"H2S", 812.00}, {"SO2", 313.40},
        {"Cl2", 123.60}, {"NaCl", 16.40}, {"NaOH", 74.90},
        {"Fe2O3", 16.50}, {"CuO", 4.89}, {"ZnO", 28.54}, {"PbO", 62.64},
        {"CaO", 110.00}, {"MgO", 58.00}, {"SiO2", 0.0}, {"Al2O3", 0.0},
        {"Fe", 376.10},
    };
    auto itf = tabla_formula.find(nombre);
    if (itf != tabla_formula.end()) return itf->second;

    return 0.0; // Fallback para especies desconocidas
}

ResultadoBalanceExergia BalanceExergia::calcular_flowsheet(
    const diagrama_flujo::Flowsheet& fs,
    const std::map<int64_t, nucleo::Especie>& db) 
{
    ResultadoBalanceExergia res;
    
    double T0 = nucleo::STP_TEMP; // 298.15 K
    
    for (auto& [fid, flujo] : fs.mapa_flujos()) {
        if (flujo.es_energia) continue;
        
        ResultadoExergiaFlujo res_f;
        res_f.id_flujo = fid;
        res_f.origen = "Desconocido";
        res_f.destino = "Desconocido";
        bool es_fuente = false;
        bool es_sumidero = false;
        
        for (const auto& nodo : fs.nodos()) {
            for (auto id_out : nodo.ids_flujos_salida) {
                if (id_out == fid) {
                    res_f.origen = std::string(nucleo::nombre_tipo_nodo(nodo.tipo)) + " (" + nodo.nombre + ")";
                    if (nodo.tipo == nucleo::TipoNodo::Fuente) es_fuente = true;
                }
            }
            for (auto id_in : nodo.ids_flujos_entrada) {
                if (id_in == fid) {
                    res_f.destino = std::string(nucleo::nombre_tipo_nodo(nodo.tipo)) + " (" + nodo.nombre + ")";
                    if (nodo.tipo == nucleo::TipoNodo::Sumidero) es_sumidero = true;
                }
            }
        }
        res_f.temperatura = flujo.temperatura;
        res_f.presion = flujo.presion;
        
        double n_total = flujo.flujo_total();
        res_f.cantidad_total_kmol = n_total;
        
        double H_total_kJ = 0.0;
        double masa_total_kg = 0.0;
        double S_total_J_K = 0.0;
        
        auto fracciones = flujo.fracciones_molares();
        
        for (auto& [sid, n] : flujo.flujo_molar) {
            if (n <= 1e-10) continue; // Ignorar trazas numéricas
            
            auto it = db.find(sid);
            if (it == db.end()) continue;
            const auto& sp = it->second;
            
            ResultadoExergiaEspecie res_sp;
            res_sp.id_especie = sid;
            res_sp.nombre = sp.nombre;
            res_sp.cantidad_kmol = n;
            res_sp.fraccion_molar = fracciones[sid];
            
            masa_total_kg += (n * sp.masa_molar); // kg/s
            
            // 1. Exergía Química Estándar (Ex_che^0)
            res_sp.ex_che_std = obtener_exergia_quimica_std(sp.nombre);
            
            // 2. Exergía Química de Mezcla (Ex_che = Ex_che^0 + R*T0*ln(x))
            // Asumiendo mezcla ideal (gases/líquidos)
            double termino_mezcla = (nucleo::R_GAS / 1000.0) * T0 * std::log(std::max(res_sp.fraccion_molar, 1e-30));
            res_sp.ex_che = res_sp.ex_che_std + termino_mezcla;
            
            // 3. Exergía Física (Ex_phy = (H - H0) - T0*(S - S0))
            double delta_H = sp.entalpia_ref(flujo.temperatura); // kJ/mol
            
            // Calcular delta_S = int(Cp/T dT)
            double T = flujo.temperatura;
            double delta_S = 0.0; // J/(mol*K)
            if (std::abs(T - T0) > 0.01) {
                delta_S = sp.cp_a * std::log(T / T0)
                        + sp.cp_b * (T - T0)
                        + (sp.cp_c / 2.0) * (T*T - T0*T0)
                        + (sp.cp_d / 3.0) * (T*T*T - T0*T0*T0);
            }
            
            // Contribución de presión para gases (asumiendo gas ideal): -R*ln(P/P0)
            if (sp.fase_std == nucleo::Fase::Gas) {
                delta_S -= nucleo::R_GAS * std::log(std::max(flujo.presion / nucleo::STP_PRES, 1e-30));
            }
            
            res_sp.ex_phy = delta_H - T0 * (delta_S / 1000.0); // kJ/mol
            
            // 4. Exergía Total (Molar)
            res_sp.ex_tot = res_sp.ex_phy + res_sp.ex_che;
            
            // 5. Exergía del flujo total de la especie (Ex_tot * n)
            res_sp.ex_tot_flujo = res_sp.ex_tot * n;
            
            H_total_kJ += (sp.h_formacion + delta_H) * n; // Entalpía absoluta total (kJ/s)
            S_total_J_K += (sp.s_formacion + delta_S) * n; // Entropía absoluta total (J/(K*s))
            
            res_f.ex_tot_flujo += res_sp.ex_tot_flujo;
            res_f.especies.push_back(res_sp);
        }
        
        res_f.masa_kg = masa_total_kg;
        res_f.entalpia_total = H_total_kJ;
        if (n_total > 0) {
            res_f.entalpia = H_total_kJ / n_total; // kJ/mol
            res_f.entropia = S_total_J_K / n_total; // J/(mol*K)
        }
        
        res.flujos.push_back(res_f);
        
        if (es_fuente) res.exergia_total_in += res_f.ex_tot_flujo;
        if (es_sumidero) res.exergia_total_out += res_f.ex_tot_flujo;
    }
    
    // Calcular calor / pérdidas en los equipos
    // Un cálculo exergético completo requeriría el factor de Carnot del calor perdido.
    // Como aproximación, usaremos los flujos de energía directos.
    for (auto& [fid, flujo] : fs.mapa_flujos()) {
        if (!flujo.es_energia) continue;
        
        if (flujo.energia < 0) {
            // Calor perdido / extraído (Requiere refrigeración)
            res.calor_loss += std::abs(flujo.energia);
            // La exergía del calor (Work potential) = Q * (1 - T0/T)
            // Asumiendo T promedio de extracción. Si no se conoce, asumimos pérdida total de exergía útil.
        } else if (flujo.energia > 0) {
            // Calor suministrado
            // Aquí se requeriría un aporte exergético desde una fuente de calor (horno/combustible).
        }
    }
    
    res.exergia_destruida = res.exergia_total_in - res.exergia_total_out;
    if (res.exergia_total_in > 0) {
        res.eficiencia_exergetica = (res.exergia_total_out / res.exergia_total_in) * 100.0;
    }
    
    return res;
}

}} // namespace
