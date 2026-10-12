#include "gestor_elementos.hpp"
#include "../nucleo/tabla_periodica.hpp"
#include <fstream>
#include <iostream>

namespace cma { namespace base_datos {

GestorElementos& GestorElementos::instancia() {
    static GestorElementos inst;
    return inst;
}

void GestorElementos::inicializar() {
    auto& tp = nucleo::TablaPeriodica::instancia();
    const auto& elementos = tp.todos();
    
    // Configurar propiedades por defecto
    propiedades_.clear();
    propiedades_.push_back({"Atomic Number", "", false});
    propiedades_.push_back({"Symbol", "", false});
    propiedades_.push_back({"Name", "", false});
    propiedades_.push_back({"Atomic Weight", "g/mol", true});
    propiedades_.push_back({"Electronegativity", "", true});
    propiedades_.push_back({"Melting Point", "K", true});
    propiedades_.push_back({"Boiling Point", "K", true});
    propiedades_.push_back({"Density", "g/cm³", true});
    propiedades_.push_back({"Electron Affinity", "kJ/mol", true});
    propiedades_.push_back({"Ionization Energy", "kJ/mol", true});
    propiedades_.push_back({"Electron Configuration", "", false});

    datos_.clear();
    datos_num_.clear();
    zs_.clear();
    
    for (const auto& e : elementos) {
        if (e.numero_atomico <= 0 || e.numero_atomico > 118) continue;
        int Z = e.numero_atomico;
        zs_.push_back(Z);
        
        establecer_valor(Z, "Atomic Number", (double)Z);
        establecer_valor(Z, "Symbol", e.simbolo);
        establecer_valor(Z, "Name", e.nombre_es);
        establecer_valor(Z, "Atomic Weight", e.masa_atomica);
        establecer_valor(Z, "Electronegativity", e.electronegatividad);
        establecer_valor(Z, "Melting Point", e.punto_fusion);
        establecer_valor(Z, "Boiling Point", e.punto_ebullicion);
        establecer_valor(Z, "Density", e.densidad);
        establecer_valor(Z, "Electron Affinity", e.afinidad_electronica);
        establecer_valor(Z, "Ionization Energy", e.energia_ionizacion);
        establecer_valor(Z, "Electron Configuration", e.config_electronica);
    }
}

void GestorElementos::cargar_desde_json(const std::string& ruta) {
    std::ifstream in(ruta);
    if (!in.is_open()) {
        inicializar();
        return;
    }
    
    nlohmann::json j;
    try {
        in >> j;
        
        propiedades_.clear();
        for (auto& jp : j["properties"]) {
            PropiedadDef def;
            def.nombre = jp.value("name", "");
            def.unidades = jp.value("units", "");
            def.graficable = jp.value("diagram", true);
            propiedades_.push_back(def);
        }
        
        datos_.clear();
        datos_num_.clear();
        zs_.clear();
        for (auto& je : j["elements"]) {
            int Z = je.value("Z", 0);
            if (Z > 0) zs_.push_back(Z);
            for (auto it = je["data"].begin(); it != je["data"].end(); ++it) {
                if (it.value().is_number()) {
                    establecer_valor(Z, it.key(), it.value().get<double>());
                } else if (it.value().is_string()) {
                    establecer_valor(Z, it.key(), it.value().get<std::string>());
                }
            }
        }
    } catch (...) {
        inicializar();
    }
}

void GestorElementos::guardar_a_json(const std::string& ruta) const {
    nlohmann::json j;
    
    nlohmann::json props = nlohmann::json::array();
    for (const auto& p : propiedades_) {
        nlohmann::json jp;
        jp["name"] = p.nombre;
        jp["units"] = p.unidades;
        jp["diagram"] = p.graficable;
        props.push_back(jp);
    }
    j["properties"] = props;
    
    nlohmann::json elems = nlohmann::json::array();
    for (int Z : zs_) {
        nlohmann::json je;
        je["Z"] = Z;
        nlohmann::json data_j;
        auto it = datos_.find(Z);
        if (it != datos_.end()) {
            for (const auto& kv : it->second) {
                // Try to write as number if it exists in datos_num_
                auto it_num = datos_num_.find(Z);
                if (it_num != datos_num_.end() && it_num->second.count(kv.first)) {
                    data_j[kv.first] = it_num->second.at(kv.first);
                } else {
                    data_j[kv.first] = kv.second;
                }
            }
        }
        je["data"] = data_j;
        elems.push_back(je);
    }
    j["elements"] = elems;
    
    std::ofstream out(ruta);
    if (out.is_open()) {
        out << j.dump(4);
    }
}

std::string GestorElementos::obtener_valor_str(int Z, const std::string& prop) const {
    auto it1 = datos_.find(Z);
    if (it1 != datos_.end()) {
        auto it2 = it1->second.find(prop);
        if (it2 != it1->second.end()) {
            return it2->second;
        }
    }
    return "";
}

double GestorElementos::obtener_valor_num(int Z, const std::string& prop) const {
    auto it1 = datos_num_.find(Z);
    if (it1 != datos_num_.end()) {
        auto it2 = it1->second.find(prop);
        if (it2 != it1->second.end()) {
            return it2->second;
        }
    }
    return 0.0;
}

void GestorElementos::establecer_valor(int Z, const std::string& prop, const std::string& val) {
    datos_[Z][prop] = val;
    // Attempt to parse as double
    try {
        size_t idx;
        double d = std::stod(val, &idx);
        if (idx == val.length()) {
            datos_num_[Z][prop] = d;
        } else {
            datos_num_[Z].erase(prop);
        }
    } catch (...) {
        datos_num_[Z].erase(prop);
    }
}

void GestorElementos::establecer_valor(int Z, const std::string& prop, double val) {
    datos_num_[Z][prop] = val;
    
    // For string representation, remove trailing zeros if integer
    char buf[64];
    if (std::floor(val) == val) {
        snprintf(buf, sizeof(buf), "%.0f", val);
    } else {
        snprintf(buf, sizeof(buf), "%.4g", val);
    }
    datos_[Z][prop] = buf;
}

void GestorElementos::set_graficable(const std::string& prop, bool graficable) {
    for (auto& p : propiedades_) {
        if (p.nombre == prop) {
            p.graficable = graficable;
            break;
        }
    }
}

}} // namespace
