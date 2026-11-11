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

// Metadatos del proyecto y proceso (HSC Sim Properties -> Process, Fig. 1 / Fig. 23)
struct ProcesoInfo {
    std::string project_name   = "Hydro_example";
    std::string project_path   = "Hydro_example.Sim";
    std::string process_name   = "Hydrometallurgical Process";
    std::string technology     = "Hydrometallurgy";
    std::string sub_technology = "Leaching & Cooling";
    std::string plant_type     = "Industrial Plant";
    std::string products       = "Hot Water, Pregnant Leach Solution (PLS)";
    std::string more_info      = "HSC Sim Process Model";
};

// Configuración del Stream Table Editor (HSC Sim Sec. 40.1.4, Fig. 9-10)
struct ConfigTablaFlujos {
    std::string nombre_set = "Table Set 1";
    int modo_vista = 0; // 0 = Name, value and unit; 1 = Value and unit; 2 = Value only
    bool mostrar_encabezado = true;
    std::vector<std::string> variables_seleccionadas = {
        "Temperature", "Pressure", "Total Mass Flow", "Total Solids Flow",
        "Total Liquid Flow", "Total Gas Flow", "% Solids", "Enthalpy Flow"
    };
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

    // ── Herramientas comunes HSC Sim (Cap. 40) ──
    // Clasifica corriente: 0 = Entrada/Input (Azul), 1 = Interna (Negro), 2 = Salida/Output (Rojo)
    int clasificar_conexion_flujo(int64_t id_flujo) const;
    int64_t origen_del_flujo(int64_t id_flujo) const;
    int64_t destino_del_flujo_conectado(int64_t id_flujo) const;
    // Reinicia corrientes internas y de salida manteniendo las corrientes de entrada (Sec. 40.2.4, Fig. 37)
    size_t limpiar_corrientes_internas();
    // Vacía todos los tanques / acumuladores dinámicos (Sec. 40.2.4, pág. 24)
    size_t vaciar_tanques();

    int64_t prox_id_nodo()   const { return prox_id_nodo_; }
    int64_t prox_id_flujo()  const { return prox_id_flujo_; }

    // Modo del balance metalúrgico (0 = Solids, 1 = Assays Only, 2 = Solids & Water)
    int mb_modo = 0;
    ProcesoInfo info_proceso;
    ConfigTablaFlujos config_tabla_flujos;

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
