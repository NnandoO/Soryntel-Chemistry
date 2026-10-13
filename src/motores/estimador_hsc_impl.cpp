#include "estimador_propiedades.hpp"
#include "nucleo/tabla_periodica.hpp"
#include "nucleo/tipos.hpp"
#include <cmath>
#include <algorithm>
#include <iostream>

namespace cma { namespace motores {

PropiedadesEstimadasHSC EstimadorPropiedades::estimar_hsc(const std::string& formula, double T_C, const std::string& tipo) {
    PropiedadesEstimadasHSC res;
    res.formula = formula;
    res.tipo = tipo;
    res.T_C = T_C;
    res.T_K = T_C + 273.15;
    res.mw = 0.0;
    res.h_est = 0.0;
    res.h_db = 0.0;
    res.s_est = 0.0;
    res.s_db = 0.0;
    res.cp_est = 0.0;
    res.cp_db = 0.0;
    res.has_db = false;

    auto& tp = nucleo::TablaPeriodica::instancia();
    auto componentes = tp.analizar_formula(formula);

    if (componentes.empty()) {
        return res; // Formula no válida
    }

    // Calcular MW
    double mw_calc = 0.0;
    for (const auto& c : componentes) {
        auto el = tp.por_simbolo(c.elem);
        if (el) {
            mw_calc += el->masa_atomica * c.cuenta;
        }
    }
    res.mw = mw_calc;

    // Buscar en BD
    auto especies = repository_ ? repository_->todas_especies() : std::vector<const nucleo::Especie*>{};
    
    for (const auto* e : especies) {
        std::string form_db = e->formula;
        if (form_db.length() > 3 && form_db.substr(form_db.length()-3) == "(s)") form_db = form_db.substr(0, form_db.length()-3);
        if (form_db.length() > 3 && form_db.substr(form_db.length()-3) == "(g)") form_db = form_db.substr(0, form_db.length()-3);
        if (form_db.length() > 3 && form_db.substr(form_db.length()-3) == "(l)") form_db = form_db.substr(0, form_db.length()-3);
        
        if (e->formula == formula || form_db == formula || e->nombre == formula) {
            res.has_db = true;
            res.h_db = e->entalpia_abs(res.T_K); // kJ/mol, no second conversion
            res.s_db = e->s_formacion + e->entropia_ref(res.T_K);
            res.cp_db = e->cp(res.T_K);
            break;
        }
    }

    // === Estimación Termodinámica Simplificada ===
    double cp_est_298 = 0.0;
    double s_est_298 = 0.0;
    double h_est_298 = 0.0;
    
    for (const auto& c : componentes) {
        double cp_atom = 26.0;
        if (c.elem == "H") cp_atom = 9.6;
        else if (c.elem == "C") cp_atom = 7.5;
        else if (c.elem == "O") cp_atom = 16.7;
        else if (c.elem == "N") cp_atom = 15.9;
        else if (c.elem == "F") cp_atom = 20.9;
        
        cp_est_298 += cp_atom * c.cuenta;

        auto el = tp.por_simbolo(c.elem);
        if (el) {
            double s_atom = 1.5 * 8.314 * log(el->masa_atomica) - 3.9;
            if (s_atom < 0) s_atom = 10.0;
            if (c.elem == "O") s_atom = 2.1; 
            s_est_298 += s_atom * c.cuenta;
            h_est_298 -= (el->electronegatividad * 100.0) * c.cuenta;
        }
    }
    
    if (res.has_db) {
        res.h_est = res.h_db; res.s_est = res.s_db; res.cp_est = res.cp_db;
        return res;
    } else {
        h_est_298 = h_est_298 - 100.0; 
    }

    res.cp_est = cp_est_298 * (1.0 + 0.0005 * (res.T_K - 298.15));
    res.s_est = s_est_298 + cp_est_298 * log(std::max(res.T_K, 1e-30) / 298.15);
    res.h_est = h_est_298 + cp_est_298 * (res.T_K - 298.15) / 1000.0;

    return res;
}

}} // namespace
