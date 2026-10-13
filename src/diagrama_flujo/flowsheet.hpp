#pragma once
#include "nucleo/tipos.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include <vector>
#include <map>
#include <memory>
#include <string>
#include <functional>

namespace cma { namespace diagrama_flujo {

struct Anotacion {
    int64_t id = 0;
    enum class Tipo : uint8_t { Texto, Rectangulo, Elipse } tipo = Tipo::Texto;
    float x = 0, y = 0, w = 150, h = 40;
    std::string texto;
    uint32_t color_fondo = 0x282832C8;
    uint32_t color_borde = 0x64648CC8;
    float grosor_borde = 2.0f;
    int pagina = 0;
};

struct TablaFlujoLienzo {
    int64_t id = 0;
    float x = 200, y = 200, w = 300, h = 120;
    bool visible = true;
    std::vector<int64_t> ids_flujos_mostrados;
    int pagina = 0;
};

class Flowsheet {
public:
    Flowsheet() = default;

    // ── Nodos ──
    int64_t agregar_nodo(nucleo::TipoNodo tipo, const std::string& nombre = "");
    bool    eliminar_nodo(int64_t id);
    nucleo::Nodo* nodo_por_id(int64_t id);
    const nucleo::Nodo* nodo_por_id(int64_t id) const;
    std::vector<nucleo::Nodo>& nodos();
    const std::vector<nucleo::Nodo>& nodos() const;
    size_t conteo_nodos() const { return nodos_.size(); }

    // ── Flujos (Streams) ──
    int64_t agregar_flujo(const std::string& nombre = "");
    int64_t agregar_flujo_energia(const std::string& nombre = "");
    bool    eliminar_flujo(int64_t id);
    nucleo::Flujo* flujo_por_id(int64_t id);
    std::map<int64_t, nucleo::Flujo>& mapa_flujos();
    const std::map<int64_t, nucleo::Flujo>& mapa_flujos() const;
    std::vector<nucleo::Flujo> flujos() const;
    size_t conteo_flujos() const { return flujos_.size(); }
    size_t conteo_flujos_energia() const;

    // ── Conectividad ──
    bool conectar(int64_t id_nodo_origen, int64_t id_nodo_destino, int64_t id_flujo = -1);
    bool desconectar(int64_t id_flujo);

    // Verifica si la conexión respeta las reglas de puertos del nodo origen/destino.
    // Llena `motivo` (si no es nullptr) con el motivo del rechazo.
    bool conexion_valida(int64_t id_nodo_origen, int64_t id_nodo_destino, std::string* motivo = nullptr) const;
    const std::string& ultimo_error_conexion() const { return ultimo_error_conexion_; }

    // ── Anotaciones ──
    int64_t agregar_anotacion(const std::string& texto = "");
    Anotacion* anotacion_por_id(int64_t id);
    bool eliminar_anotacion(int64_t id);
    std::vector<Anotacion>& anotaciones();
    const std::vector<Anotacion>& anotaciones() const;

    // ── Tablas de flujo ──
    int64_t agregar_tabla_flujo();
    bool eliminar_tabla_flujo(int64_t id);
    std::vector<TablaFlujoLienzo>& tablas_flujo();
    const std::vector<TablaFlujoLienzo>& tablas_flujo() const;

    // ── Topología ──
    std::vector<int64_t> orden_topologico() const;
    bool detectar_ciclo() const;

    // ── Motor de convergencia (Wegstein) ──
    struct ResultadoConvergencia {
        bool convergio = false;
        int iteraciones = 0;
        double error_final = 0.0;
        std::string mensaje;
    };
    ResultadoConvergencia resolver_convergencia(
        const domain::ThermoRepository& repository,
        int max_iter = 500,
        double tolerancia = 1e-6,
        std::function<void(int, double)> progreso = nullptr);

    // ── Serialización JSON ──
    std::string a_json() const;
    bool desde_json(const std::string& json, std::string& error);

    void limpiar();

    int64_t prox_id_nodo()   const { return prox_id_nodo_; }
    int64_t prox_id_flujo()  const { return prox_id_flujo_; }

private:
    std::vector<nucleo::Nodo> nodos_;
    std::map<int64_t, nucleo::Flujo> flujos_;
    std::vector<Anotacion> anotaciones_;
    std::vector<TablaFlujoLienzo> tablas_flujo_;
    int64_t prox_id_nodo_   = 1;
    int64_t prox_id_flujo_  = 1;
    int64_t prox_id_anotacion_ = 1;
    int64_t prox_id_tabla_ = 1;
    mutable std::string ultimo_error_conexion_;

    // Encontrar destino de un flujo (siguiente nodo downstream)
    int64_t destino_del_flujo(int64_t id_flujo) const;
};

}} // namespace
