import re

with open("src/motores/diagramas.cpp", "r") as f:
    content = f.read()

# I will find the definition of Diagramas::calcular_pourbaix_2d and replace it.
pattern = re.compile(r"Diagramas::MallaPourbaix Diagramas::calcular_pourbaix_2d.*?return malla;\n}", re.DOTALL)
match = pattern.search(content)

new_func = r"""
#include <map>
#include <cctype>
#include "../base_datos/gestor_db.hpp"

static std::map<std::string, double> parse_formula_pourbaix(const std::string& formula) {
    std::map<std::string, double> counts;
    int n = formula.length();
    int i = 0;
    
    // Simplistic parser for species like Fe2O3, Fe(+2a), HFeO2(-a)
    std::string clean = formula;
    double charge = 0.0;
    
    // Check for charge suffix e.g. (+2a) or (-a) or (+3a)
    size_t p = clean.find('(');
    if (p != std::string::npos) {
        std::string tag = clean.substr(p+1);
        if (tag.find('+') != std::string::npos || tag.find('-') != std::string::npos) {
            double sign = (tag.find('-') != std::string::npos) ? -1.0 : 1.0;
            double val = 1.0;
            for (char c : tag) {
                if (isdigit(c)) { val = c - '0'; break; }
            }
            charge = sign * val;
        }
        clean = clean.substr(0, p); // remove suffix
    }
    
    // Handle subscript unicode characters
    std::string ascii_clean = "";
    for (size_t k = 0; k < clean.length(); ) {
        if (clean[k] == '\xe2' && k+2 < clean.length() && clean[k+1] == '\x82') {
            char d = clean[k+2];
            if (d >= '\x80' && d <= '\x89') {
                ascii_clean += std::to_string(d - '\x80');
            }
            k += 3;
        } else {
            ascii_clean += clean[k];
            k++;
        }
    }
    
    // Simple element parser
    int len = ascii_clean.length();
    for (int k = 0; k < len; ) {
        if (isupper(ascii_clean[k])) {
            std::string el = "";
            el += ascii_clean[k];
            k++;
            if (k < len && islower(ascii_clean[k])) {
                el += ascii_clean[k];
                k++;
            }
            std::string num = "";
            while (k < len && isdigit(ascii_clean[k])) {
                num += ascii_clean[k];
                k++;
            }
            double cnt = num.empty() ? 1.0 : std::stod(num);
            counts[el] += cnt;
        } else {
            k++; 
        }
    }
    counts["charge"] = charge;
    return counts;
}

Diagramas::MallaPourbaix Diagramas::calcular_pourbaix_2d(const std::string& metal, double T, double concentracion, int res_x, int res_y, const std::map<int64_t, bool>* especies_activas) {
    MallaPourbaix malla;
    malla.res_x = res_x;
    malla.res_y = res_y;
    malla.datos.resize(res_x * res_y, 0);
    
    double RTF = RT_over_F(T);
    double RT = 8.314 * T;
    auto limites = limites_agua(T, 1.0);
    
    malla.limites_agua_inferior = { limites.first, limites.first - RTF * std::log(10) * 14.0 };
    malla.limites_agua_superior = { limites.second, limites.second - RTF * std::log(10) * 14.0 };
    
    std::vector<FasePourbaix> fases_reg;
    
    auto& db = cma::base_datos::GestorDB::instancia();
    auto todas = db.todas_especies();
    
    struct EspActiva {
        int idx;
        std::string nombre;
        bool acuoso;
        double G_T; 
        double m; 
        double h; 
        double o; 
        double z; 
    };
    
    std::vector<EspActiva> candidatas;
    int fase_idx_counter = 1;
    
    for (auto* esp : todas) {
        if (especies_activas && especies_activas->find(esp->id) != especies_activas->end()) {
            if (!(*especies_activas).at(esp->id)) continue;
        } else {
            if (esp->formula.find(metal) == std::string::npos) continue;
        }
        
        auto comp = parse_formula_pourbaix(esp->formula);
        if (comp[metal] == 0) continue; 
        
        EspActiva act;
        act.idx = fase_idx_counter++;
        act.nombre = esp->formula;
        act.acuoso = (esp->fase_std != cma::nucleo::Fase::Solido);
        
        double G_kJ = esp->entalpia_abs(T) - T * esp->entropia_ref(T) / 1000.0;
        act.G_T = G_kJ * 1000.0; // J/mol
        
        if (act.acuoso) {
            act.G_T += RT * std::log(concentracion);
        }
        
        act.m = comp[metal];
        act.h = comp["H"];
        act.o = comp["O"];
        act.z = comp["charge"];
        
        candidatas.push_back(act);
        
        FasePourbaix fb;
        fb.id = act.idx;
        fb.nombre = act.nombre;
        fb.acuoso = act.acuoso;
        fases_reg.push_back(fb);
    }
    
    if (candidatas.empty()) {
        FasePourbaix fb; fb.id = 1; fb.nombre = metal; fb.acuoso = false;
        malla.fases.push_back(fb);
        return malla;
    }
    
    malla.fases = fases_reg;
    
    for (int y = 0; y < res_y; ++y) {
        double eh = malla.max_eh - y * (malla.max_eh - malla.min_eh) / (res_y - 1);
        for (int x = 0; x < res_x; ++x) {
            double ph = malla.min_ph + x * (malla.max_ph - malla.min_ph) / (res_x - 1);
            
            double mu_e = -96485.0 * eh;
            double mu_H = -RT * std::log(10.0) * ph;
            double mu_H2O = -237140.0; 
            
            double min_G_eff = 1e30;
            int best_idx = 1;
            
            for (const auto& c : candidatas) {
                double g_eff = (c.G_T - c.o * mu_H2O - (c.h - 2.0*c.o) * mu_H - c.z * mu_e) / c.m;
                if (g_eff < min_G_eff) {
                    min_G_eff = g_eff;
                    best_idx = c.idx;
                }
            }
            
            malla.datos[y * res_x + x] = best_idx;
        }
    }
    
    for (auto& fb : malla.fases) { fb.contador = 0; fb.centroide_ph = 0; fb.centroide_eh = 0; }
    for (int y = 0; y < res_y; ++y) {
        double eh = malla.max_eh - y * (malla.max_eh - malla.min_eh) / (res_y - 1);
        for (int x = 0; x < res_x; ++x) {
            double ph = malla.min_ph + x * (malla.max_ph - malla.min_ph) / (res_x - 1);
            int f = malla.datos[y * res_x + x];
            for (auto& fb : malla.fases) {
                if (fb.id == f) {
                    fb.contador++;
                    fb.centroide_ph += ph;
                    fb.centroide_eh += eh;
                }
            }
        }
    }
    for (auto& fb : malla.fases) {
        if (fb.contador > 0) {
            fb.centroide_ph /= fb.contador;
            fb.centroide_eh /= fb.contador;
        }
    }
    
    return malla;
}
"""

content = content.replace(match.group(0), new_func.strip())

with open("src/motores/diagramas.cpp", "w") as f:
    f.write(content)
