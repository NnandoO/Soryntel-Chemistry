#pragma once
#include <string>
#include <vector>
#include <map>
#include <nlohmann/json.hpp>

namespace cma { namespace base_datos {

struct PropiedadDef {
    std::string nombre;
    std::string unidades;
    bool graficable = true; // "Diagram" in UI
};

class GestorElementos {
public:
    static GestorElementos& instancia();

    // Initialize with default values if JSON is missing
    void inicializar();
    void cargar_desde_json(const std::string& ruta);
    void guardar_a_json(const std::string& ruta) const;

    const std::vector<PropiedadDef>& propiedades() const { return propiedades_; }
    
    // Obtener valor de propiedad (vacio si no existe)
    std::string obtener_valor_str(int Z, const std::string& prop) const;
    double obtener_valor_num(int Z, const std::string& prop) const;
    
    void establecer_valor(int Z, const std::string& prop, const std::string& val);
    void establecer_valor(int Z, const std::string& prop, double val);
    
    void set_graficable(const std::string& prop, bool graficable);
    
    // Lista de Z para iterar (1 a 118)
    const std::vector<int>& atomicos() const { return zs_; }

private:
    GestorElementos() = default;
    
    std::vector<PropiedadDef> propiedades_;
    std::vector<int> zs_;
    
    // Z -> PropertyName -> Value string
    std::map<int, std::map<std::string, std::string>> datos_;
    // Z -> PropertyName -> Value double (for fast numeric queries/plots)
    std::map<int, std::map<std::string, double>> datos_num_;
};

}} // namespace
