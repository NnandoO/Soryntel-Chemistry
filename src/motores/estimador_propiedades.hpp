#pragma once
#include <vector>
#include <string>
#include <map>

namespace cma { namespace motores {

struct PropiedadesEstimadas {
    double h_formacion;   // kJ/mol
    double s_formacion;   // J/(mol·K)
    double cp_a, cp_b, cp_c, cp_d; // coeficientes Cp
    double masa_molar;    // g/mol
    double T_fusion;      // K
    double T_ebullicion;  // K
    std::string mensaje;
    bool ok = false;
};

struct PropiedadesEstimadasHSC {
    std::string formula;
    std::string tipo;
    double T_C;
    double T_K;
    double mw;
    
    double h_est;
    double h_db;
    double s_est;
    double s_db;
    double cp_est;
    double cp_db;
    
    bool has_db = false;
};

class EstimadorPropiedades {
public:
    // Estimación por contribución de grupos (Joback-Reid)
    PropiedadesEstimadas estimar_joback(const std::string& formula_smiles,
                                         const std::string& nombre = "");

    // Método de Benson para orgánicos
    PropiedadesEstimadas estimar_benson(const std::string& formula,
                                         const std::string& tipo = "orgánico");

    // Método para replicar HSC Estimates
    PropiedadesEstimadasHSC estimar_hsc(const std::string& formula, double T_C, const std::string& tipo);

    // Estimación de capacidad calorífica (Cp) por correlaciones generales
    double estimar_cp_gas(double M, double T, int tipo = 0);

    // Estimación de punto de ebullición normal (método de Joback)
    double estimar_T_ebullicion(const std::map<std::string, int>& grupos);

    // Estimación de entalpía de formación (método de Benson)
    double estimar_Hf_benson(const std::map<std::string, int>& grupos,
                              double correccion_esterica = 0);

    // Estimación de entropía absoluta (método de Benson)
    double estimar_S_benson(const std::string& formula, double M);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;
    struct GrupoJoback {
        std::string nombre;
        double contrib_Hf, contrib_Sf, contrib_Cp_a, contrib_Cp_b, contrib_Cp_c, contrib_Cp_d;
        double contrib_Tb, contrib_Tf;
    };
    std::vector<GrupoJoback> grupos_joback();
};

}} // namespace
