#pragma once
#include <string>
#include <vector>
#include <map>
#include <nlohmann/json.hpp>

namespace cma { namespace base_datos {

struct UnidadDef {
    std::string nombre;
    double factor; // Multiplier to convert FROM this unit TO the base unit. (e.g. if base is 'm', factor for 'cm' is 0.01)
    double offset; // For temperatures (e.g. +273.15)
};

struct CategoriaUnidad {
    std::string nombre;
    std::vector<UnidadDef> unidades;
};

class GestorUnidades {
public:
    static GestorUnidades& instancia();

    void inicializar();
    void cargar_desde_json(const std::string& ruta);
    void guardar_a_json(const std::string& ruta) const;

    const std::vector<CategoriaUnidad>& categorias() const { return categorias_; }

    double convertir(double valor, const std::string& categoria, int from_idx, int to_idx) const;
    double obtener_factor(const std::string& categoria, int from_idx, int to_idx) const;

private:
    GestorUnidades() = default;
    
    std::vector<CategoriaUnidad> categorias_;
};

}} // namespace
