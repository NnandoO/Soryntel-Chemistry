#pragma once
#include <string>
#include <vector>
#include <map>
#include <cmath>

namespace cma { namespace nucleo {

struct Elemento {
    int    numero_atomico = 0;
    std::string simbolo;
    std::string nombre_es;        // Nombre en español
    std::string config_electronica;
    double  masa_atomica   = 0.0; // g/mol
    double  electronegatividad = 0.0; // Pauling
    double  punto_fusion   = 0.0; // K
    double  punto_ebullicion = 0.0; // K
    double  densidad       = 0.0; // g/cm³
    double  afinidad_electronica = 0.0; // kJ/mol
    double  energia_ionizacion = 0.0; // kJ/mol
    int     periodo       = 0;
    int     grupo         = 0;
    int     estado_oxidacion_comun = 0;

    bool es_metal() const {
        return (grupo >= 1 && grupo <= 12) && grupo != 0;
    }
    bool es_gas_noble() const { return grupo == 18; }
};

class TablaPeriodica {
public:
    static TablaPeriodica& instancia();

    const Elemento* por_simbolo(const std::string& simb) const;
    const Elemento* por_numero(int z) const;
    const Elemento* por_nombre(const std::string& nombre) const;
    const std::vector<Elemento>& todos() const { return elementos_; }

    // Análisis de fórmula: H2O -> {{H,2},{O,1}}
    struct CompAtom { std::string elem; int cuenta; };
    std::vector<CompAtom> analizar_formula(const std::string& formula) const;

private:
    TablaPeriodica();
    std::vector<Elemento> elementos_;
    std::map<std::string, int> simb_a_idx_;
    std::map<int, int> num_a_idx_;
};

}} // namespace
