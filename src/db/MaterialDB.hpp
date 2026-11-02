#pragma once
#include "application/modules.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "nucleo/tipos.hpp"
#include <string>
#include <vector>

namespace cma::application::modules {

struct ComponenteMaterial {
    std::string nombre;      // "[Concentrate]" (sub-material) o "CuFeS2" (especie directa)
    double wt_pct = 0.0;     // % en peso
    std::string referencia;  // Columna G (Reference / Notas)

    bool es_submaterial() const {
        return nombre.size() >= 3 && nombre.front() == '[' && nombre.back() == ']';
    }

    std::string nombre_submaterial() const {
        if (es_submaterial()) {
            return nombre.substr(1, nombre.size() - 2);
        }
        return nombre;
    }
};

struct EspecieResuelta {
    std::string formula;
    double wt_pct = 0.0;
    std::string referencia;
};

struct ElementoResuelto {
    std::string simbolo;
    double wt_pct = 0.0;
};

struct PropiedadMaterial {
    std::string nombre;
    double valor = 0.0;
    std::string unidad;
    std::string referencia;
    bool es_usuario = false;
};

struct RegistroMaterial {
    std::string nombre;
    std::string categoria;      // Concentrates, Fuels, Sludges, Steels, Plastics, Fluxes, Blends, Slags
    std::string codigo;
    std::string descripcion;
    std::string referencia;
    std::vector<ComponenteMaterial> composicion;
    std::vector<PropiedadMaterial> propiedades_fisicas;
    std::vector<PropiedadMaterial> analisis;
    std::vector<PropiedadMaterial> propiedades_usuario;
};

class MaterialDB {
public:
    explicit MaterialDB(const cma::domain::ThermoRepository& repository);

    // Compatibilidad con consulta de especies condensadas del repositorio termodinámico
    std::vector<const cma::nucleo::Especie*> materiales();

    // Catálogo de materiales compuestos (HSC Material Database)
    const std::vector<RegistroMaterial>& catalogo() const { return catalogo_; }
    std::vector<RegistroMaterial>& catalogo() { return catalogo_; }

    const RegistroMaterial* buscar_por_nombre(const std::string& nombre) const;
    RegistroMaterial* buscar_por_nombre(const std::string& nombre);

    void agregar_o_actualizar(const RegistroMaterial& mat);
    bool eliminar(const std::string& nombre);
    void restaurar_predeterminados();

    // Resolución jerárquica: Composition (Cols A-B) -> Species (Cols C-D) -> Elements (Cols E-F)
    std::vector<EspecieResuelta> resolver_especies(const RegistroMaterial& mat) const;
    std::vector<ElementoResuelto> resolver_elementos(const std::vector<EspecieResuelta>& especies) const;
    std::vector<ElementoResuelto> resolver_elementos(const RegistroMaterial& mat) const;

    static double suma_composicion(const RegistroMaterial& mat);
    static double suma_especies(const std::vector<EspecieResuelta>& esp);
    static double suma_elementos(const std::vector<ElementoResuelto>& elem);
    static void normalizar_composicion(RegistroMaterial& mat);

    bool exportar_csv_material(const std::string& ruta, const RegistroMaterial& mat) const;
    bool guardar_catalogo_tsv(const std::string& ruta) const;
    bool cargar_catalogo_tsv(const std::string& ruta);

private:
    const cma::domain::ThermoRepository& motor_;
    std::vector<RegistroMaterial> catalogo_;

    void resolver_especies_recursivo(
        const RegistroMaterial& mat,
        double factor_escala,
        std::vector<std::string>& pila_visitados,
        std::vector<EspecieResuelta>& acumuladas) const;
};

} // namespace cma::application::modules
