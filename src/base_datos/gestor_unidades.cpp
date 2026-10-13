#include "gestor_unidades.hpp"
#include <fstream>
#include <iostream>

namespace cma { namespace base_datos {

GestorUnidades& GestorUnidades::instancia() {
    static GestorUnidades inst;
    return inst;
}

void GestorUnidades::inicializar() {
    categorias_.clear();
    
    // 1. Length (Base: m)
    categorias_.push_back({"Length", {
        {"m", 1.0, 0.0}, {"cm", 0.01, 0.0}, {"mm", 0.001, 0.0}, {"um", 1e-6, 0.0}, {"nm", 1e-9, 0.0}, {"km", 1000.0, 0.0},
        {"in", 0.0254, 0.0}, {"ft", 0.3048, 0.0}, {"yd", 0.9144, 0.0}, {"mi", 1609.344, 0.0}, {"nmi", 1852.0, 0.0},
        {"A", 1e-10, 0.0}, {"mils", 0.0000254, 0.0}
    }});

    // 2. Area (Base: m2)
    categorias_.push_back({"Area", {
        {"m2", 1.0, 0.0}, {"cm2", 1e-4, 0.0}, {"mm2", 1e-6, 0.0}, {"km2", 1e6, 0.0},
        {"in2", 0.00064516, 0.0}, {"ft2", 0.09290304, 0.0}, {"yd2", 0.83612736, 0.0},
        {"acre", 4046.856, 0.0}, {"ha", 10000.0, 0.0}, {"sq mi", 2589988.11, 0.0}
    }});

    // 3. Volume (Base: m3)
    categorias_.push_back({"Volume", {
        {"m3", 1.0, 0.0}, {"cm3", 1e-6, 0.0}, {"mm3", 1e-9, 0.0}, {"km3", 1e9, 0.0},
        {"L", 0.001, 0.0}, {"mL", 1e-6, 0.0}, {"in3", 1.6387064e-5, 0.0},
        {"ft3", 0.028316846, 0.0}, {"yd3", 0.76455485, 0.0},
        {"gal (US)", 0.00378541, 0.0}, {"gal (UK)", 0.00454609, 0.0},
        {"barrel (oil)", 0.1589873, 0.0}, {"fl oz (US)", 2.95735e-5, 0.0}
    }});

    // 4. Mass (Base: kg)
    categorias_.push_back({"Mass", {
        {"kg", 1.0, 0.0}, {"g", 0.001, 0.0}, {"mg", 1e-6, 0.0}, {"ug", 1e-9, 0.0}, {"t", 1000.0, 0.0},
        {"lb", 0.45359237, 0.0}, {"oz", 0.02834952, 0.0}, {"ton (short)", 907.1847, 0.0},
        {"ton (long)", 1016.0469, 0.0}, {"grain", 6.479891e-5, 0.0}, {"carat", 0.0002, 0.0}
    }});

    // 5. Time (Base: s)
    categorias_.push_back({"Time", {
        {"s", 1.0, 0.0}, {"min", 60.0, 0.0}, {"h", 3600.0, 0.0}, {"d", 86400.0, 0.0},
        {"week", 604800.0, 0.0}, {"y (365d)", 31536000.0, 0.0}, {"ms", 0.001, 0.0}, {"us", 1e-6, 0.0}
    }});

    // 6. Density (Base: kg/m3)
    categorias_.push_back({"Density", {
        {"kg/m3", 1.0, 0.0}, {"g/cm3", 1000.0, 0.0}, {"g/L", 1.0, 0.0}, {"kg/L", 1000.0, 0.0},
        {"lb/ft3", 16.018463, 0.0}, {"lb/in3", 27679.9, 0.0}, {"lb/gal(US)", 119.826, 0.0}
    }});

    // 7. Velocity (Base: m/s)
    categorias_.push_back({"Velocity", {
        {"m/s", 1.0, 0.0}, {"cm/s", 0.01, 0.0}, {"km/h", 0.277777778, 0.0},
        {"ft/s", 0.3048, 0.0}, {"mph", 0.44704, 0.0}, {"knot", 0.514444, 0.0}
    }});

    // 8. Force (Base: N)
    categorias_.push_back({"Force", {
        {"N", 1.0, 0.0}, {"kN", 1000.0, 0.0}, {"MN", 1e6, 0.0}, {"dyn", 1e-5, 0.0},
        {"lbf", 4.448222, 0.0}, {"kgf", 9.80665, 0.0}, {"tonf", 8896.443, 0.0}
    }});

    // 9. Pressure (Base: Pa)
    categorias_.push_back({"Pressure", {
        {"Pa", 1.0, 0.0}, {"kPa", 1000.0, 0.0}, {"MPa", 1e6, 0.0}, {"GPa", 1e9, 0.0},
        {"bar", 100000.0, 0.0}, {"mbar", 100.0, 0.0}, {"atm", 101325.0, 0.0},
        {"torr", 133.322368, 0.0}, {"mmHg", 133.322387, 0.0}, {"psi", 6894.75729, 0.0},
        {"psf", 47.880258, 0.0}, {"inHg", 3386.389, 0.0}, {"inH2O", 249.0889, 0.0}
    }});

    // 10. Energy (Base: J)
    categorias_.push_back({"Energy", {
        {"J", 1.0, 0.0}, {"kJ", 1000.0, 0.0}, {"MJ", 1e6, 0.0}, {"GJ", 1e9, 0.0},
        {"cal", 4.184, 0.0}, {"kcal", 4184.0, 0.0}, {"Mcal", 4.184e6, 0.0},
        {"BTU", 1055.056, 0.0}, {"MMBTU", 1.055056e9, 0.0}, {"therm", 1.055056e8, 0.0},
        {"Wh", 3600.0, 0.0}, {"kWh", 3.6e6, 0.0}, {"MWh", 3.6e9, 0.0},
        {"eV", 1.6021766e-19, 0.0}, {"erg", 1e-7, 0.0}
    }});

    // 11. Power (Base: W)
    categorias_.push_back({"Power", {
        {"W", 1.0, 0.0}, {"kW", 1000.0, 0.0}, {"MW", 1e6, 0.0}, {"GW", 1e9, 0.0},
        {"hp (metric)", 735.49875, 0.0}, {"hp (mechanical)", 745.69987, 0.0},
        {"BTU/h", 0.293071, 0.0}, {"kcal/h", 1.162222, 0.0}, {"J/s", 1.0, 0.0}
    }});

    // 12. Temperature (Base: K) - Uses offsets
    categorias_.push_back({"Temperature", {
        {"K", 1.0, 0.0}, {"°C", 1.0, 273.15}, {"°F", 5.0/9.0, 255.3722222}, {"R", 5.0/9.0, 0.0}
    }});

    // 13. Dynamic Viscosity (Base: Pa.s)
    categorias_.push_back({"Dynamic Viscosity", {
        {"Pa.s", 1.0, 0.0}, {"mPa.s", 0.001, 0.0}, {"P", 0.1, 0.0}, {"cP", 0.001, 0.0},
        {"lb/ft.s", 1.48816, 0.0}, {"lb/ft.h", 0.000413379, 0.0}
    }});

    // 14. Kinematic Viscosity (Base: m2/s)
    categorias_.push_back({"Kinematic Viscosity", {
        {"m2/s", 1.0, 0.0}, {"cm2/s", 1e-4, 0.0}, {"St", 1e-4, 0.0}, {"cSt", 1e-6, 0.0},
        {"ft2/s", 0.092903, 0.0}, {"ft2/h", 0.0000258064, 0.0}
    }});

    // 15. Mass Flow (Base: kg/s)
    categorias_.push_back({"Mass Flow", {
        {"kg/s", 1.0, 0.0}, {"kg/h", 0.000277778, 0.0}, {"t/h", 0.277778, 0.0}, {"t/d", 0.011574, 0.0},
        {"lb/s", 0.453592, 0.0}, {"lb/h", 0.000125998, 0.0}, {"ton(short)/h", 0.251996, 0.0}
    }});

    // 16. Volume Flow (Base: m3/s)
    categorias_.push_back({"Volume Flow", {
        {"m3/s", 1.0, 0.0}, {"m3/h", 0.000277778, 0.0}, {"L/s", 0.001, 0.0}, {"L/min", 1.66667e-5, 0.0},
        {"ft3/s", 0.0283168, 0.0}, {"ft3/min", 0.000471947, 0.0}, {"gal/min (US)", 6.30902e-5, 0.0}
    }});

    // 17. Molar Mass (Base: kg/mol)
    categorias_.push_back({"Molar Mass", {
        {"kg/mol", 1.0, 0.0}, {"g/mol", 0.001, 0.0}, {"kg/kmol", 0.001, 0.0}, {"lb/lbmol", 0.001, 0.0}
    }});

    // 18. Heat Capacity & Entropy (Base: J/K)
    categorias_.push_back({"Heat Capacity", {
        {"J/K", 1.0, 0.0}, {"kJ/K", 1000.0, 0.0}, {"cal/K", 4.184, 0.0}, {"kcal/K", 4184.0, 0.0},
        {"BTU/°F", 1899.1, 0.0}
    }});
    
    // 19. Molar Heat Capacity (Base: J/mol.K)
    categorias_.push_back({"Molar Heat Capacity", {
        {"J/mol.K", 1.0, 0.0}, {"kJ/mol.K", 1000.0, 0.0}, {"cal/mol.K", 4.184, 0.0}, {"kcal/kmol.K", 4.184, 0.0},
        {"BTU/lbmol.°F", 4.1868, 0.0} // roughly same as kcal/kmol.K in numerical value if we don't convert mol base, wait it's exact: 1 BTU/lbmol°F = 1 kcal/kmol°C = 4186.8 J/kmol K -> 4.1868 J/molK
    }});

    // 20. Thermal Conductivity (Base: W/m.K)
    categorias_.push_back({"Thermal Conductivity", {
        {"W/m.K", 1.0, 0.0}, {"W/cm.K", 100.0, 0.0}, {"kW/m.K", 1000.0, 0.0},
        {"kcal/h.m.°C", 1.16222, 0.0}, {"BTU/h.ft.°F", 1.730735, 0.0}
    }});

    // 21. Heat Transfer Coefficient (Base: W/m2.K)
    categorias_.push_back({"Heat Transfer Coeff", {
        {"W/m2.K", 1.0, 0.0}, {"kW/m2.K", 1000.0, 0.0}, {"kcal/h.m2.°C", 1.16222, 0.0},
        {"BTU/h.ft2.°F", 5.67826, 0.0}
    }});

    // 22. Angle (Base: rad)
    categorias_.push_back({"Angle", {
        {"rad", 1.0, 0.0}, {"deg", 0.0174532925, 0.0}, {"grad", 0.0157079633, 0.0},
        {"rev", 6.283185307, 0.0}
    }});
}

void GestorUnidades::cargar_desde_json(const std::string& ruta) {
    std::ifstream in(ruta);
    if (!in.is_open()) {
        inicializar();
        return;
    }
    
    try {
        nlohmann::json j;
        in >> j;
        categorias_.clear();
        for (auto& jc : j["categories"]) {
            CategoriaUnidad cat;
            cat.nombre = jc.value("name", "");
            for (auto& ju : jc["units"]) {
                UnidadDef ud;
                ud.nombre = ju.value("name", "");
                ud.factor = ju.value("factor", 1.0);
                ud.offset = ju.value("offset", 0.0);
                cat.unidades.push_back(ud);
            }
            categorias_.push_back(cat);
        }
    } catch (...) {
        inicializar();
    }
}

void GestorUnidades::guardar_a_json(const std::string& ruta) const {
    nlohmann::json j;
    nlohmann::json cats = nlohmann::json::array();
    
    for (const auto& cat : categorias_) {
        nlohmann::json jc;
        jc["name"] = cat.nombre;
        nlohmann::json units = nlohmann::json::array();
        for (const auto& u : cat.unidades) {
            nlohmann::json ju;
            ju["name"] = u.nombre;
            ju["factor"] = u.factor;
            ju["offset"] = u.offset;
            units.push_back(ju);
        }
        jc["units"] = units;
        cats.push_back(jc);
    }
    j["categories"] = cats;
    
    std::ofstream out(ruta);
    if (out.is_open()) {
        out << j.dump(4);
    }
}

double GestorUnidades::convertir(double valor, const std::string& categoria, int from_idx, int to_idx) const {
    for (const auto& cat : categorias_) {
        if (cat.nombre == categoria) {
            if (from_idx >= 0 && static_cast<size_t>(from_idx) < cat.unidades.size() && to_idx >= 0 && static_cast<size_t>(to_idx) < cat.unidades.size()) {
                const auto& u_from = cat.unidades[from_idx];
                const auto& u_to = cat.unidades[to_idx];
                
                // Step 1: Convert to base unit
                double val_base = (valor * u_from.factor) + u_from.offset;
                
                // Step 2: Convert from base to target unit
                if (u_to.factor == 0.0) return valor; // Prevent division by zero
                double val_target = (val_base - u_to.offset) / u_to.factor;
                
                return val_target;
            }
        }
    }
    return valor;
}

double GestorUnidades::obtener_factor(const std::string& categoria, int from_idx, int to_idx) const {
    // Para mostrar en la UI, devolvemos el factor directo (si es multiplicativo)
    for (const auto& cat : categorias_) {
        if (cat.nombre == categoria) {
            if (from_idx >= 0 && static_cast<size_t>(from_idx) < cat.unidades.size() && to_idx >= 0 && static_cast<size_t>(to_idx) < cat.unidades.size()) {
                if (cat.nombre == "Temperature") return 0.0; // No simple multiplier for temp
                const auto& u_from = cat.unidades[from_idx];
                const auto& u_to = cat.unidades[to_idx];
                return u_from.factor / u_to.factor;
            }
        }
    }
    return 1.0;
}

}} // namespace
