#pragma once
#include "nucleo/tipos.hpp"
#include "nucleo/tabla_periodica.hpp"
#include <string>
#include <vector>
#include <map>
#include <memory>
#include "domain/ports/ThermoRepository.hpp"

namespace cma { namespace base_datos {

class GestorDB final : public domain::ThermoRepository {
public:
    static GestorDB& instancia();
    GestorDB();

    bool cargar_predeterminadas();
    bool exportar_sqlite(const std::string& ruta) const;

    // Especies
    const nucleo::Especie* especie_por_id(int64_t id) const;
    const nucleo::Especie* especie_por_nombre(const std::string& nombre) const;
    std::vector<const nucleo::Especie*> buscar_especies(const std::string& query) const;
    std::vector<const nucleo::Especie*> todas_especies() const;
    const std::map<int64_t, nucleo::Especie>& mapa_especies() const { return especies_; }
    int conteo_especies() const { return (int)especies_.size(); }

    // Reacciones
    const nucleo::Reaccion* reaccion_por_id(int64_t id) const;
    std::vector<const nucleo::Reaccion*> todas_reacciones() const;
    int conteo_reacciones() const { return (int)reacciones_.size(); }

    // Elementos (tabla periódica)
    const nucleo::Elemento* elemento_por_simbolo(const std::string& simb) const;
    const nucleo::Elemento* elemento_por_numero(int z) const;
    const nucleo::Elemento* elemento_por_nombre(const std::string& nombre) const;
    std::vector<const nucleo::Elemento*> todos_elementos() const;
    int conteo_elementos() const { return (int)elementos_.size(); }

    // CSV Data Import
    bool cargar_desde_csv(const std::string& ruta);

    bool inicializada() const { return inicializada_; }

private:
    GestorDB(const GestorDB&) = delete;
    GestorDB& operator=(const GestorDB&) = delete;

    void poblar_especies();
    void poblar_reacciones();
    void poblar_elementos();

    bool inicializada_ = false;
    int64_t prox_id_especie_ = 1;
    int64_t prox_id_reaccion_ = 1;

    std::map<int64_t, nucleo::Especie> especies_;
    std::map<int64_t, nucleo::Reaccion> reacciones_;
    std::map<int64_t, nucleo::Elemento> elementos_;
    std::map<std::string, int64_t> nombre_a_id_especie_;
};

}} // namespace
