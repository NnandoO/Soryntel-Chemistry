#include "diagrama_flujo/flowsheet.hpp"
#include "motores/balance_masa.hpp"
#include <nlohmann/json.hpp>
#include <queue>
#include <algorithm>
#include <sstream>
#include <cmath>
#include <cstring>

namespace cma { namespace diagrama_flujo {

// ═══════════════════════════════════════════════════════════════════════════
// Nodos
// ═══════════════════════════════════════════════════════════════════════════

int64_t Flowsheet::agregar_nodo(nucleo::TipoNodo tipo, const std::string& nombre) {
    nucleo::Nodo nodo;
    nodo.id    = prox_id_nodo_++;
    nodo.tipo  = tipo;
    nodo.nombre = nombre.empty()
        ? std::string(nucleo::nombre_tipo_nodo(tipo)) + "_" + std::to_string(nodo.id)
        : nombre;
    nodos_.push_back(nodo);
    return nodo.id;
}

bool Flowsheet::eliminar_nodo(int64_t id) {
    auto it = std::find_if(nodos_.begin(), nodos_.end(),
        [id](const nucleo::Nodo& n) { return n.id == id; });
    if (it == nodos_.end()) return false;

    // Eliminar los flujos asociados a este nodo
    for (int64_t fid : it->ids_flujos_entrada) flujos_.erase(fid);
    for (int64_t fid : it->ids_flujos_salida)  flujos_.erase(fid);

    // Limpiar referencias a esos flujos en los demás nodos
    for (auto& n : nodos_) {
        auto& in  = n.ids_flujos_entrada;
        auto& out = n.ids_flujos_salida;
        for (int64_t fid : it->ids_flujos_entrada)
            in.erase(std::remove(in.begin(), in.end(), fid), in.end());
        for (int64_t fid : it->ids_flujos_salida)
            out.erase(std::remove(out.begin(), out.end(), fid), out.end());
    }

    nodos_.erase(it);
    return true;
}

nucleo::Nodo* Flowsheet::nodo_por_id(int64_t id) {
    for (auto& n : nodos_) if (n.id == id) return &n;
    return nullptr;
}

const nucleo::Nodo* Flowsheet::nodo_por_id(int64_t id) const {
    for (auto& n : nodos_) if (n.id == id) return &n;
    return nullptr;
}

std::vector<nucleo::Nodo>& Flowsheet::nodos() { return nodos_; }
const std::vector<nucleo::Nodo>& Flowsheet::nodos() const { return nodos_; }

// ═══════════════════════════════════════════════════════════════════════════
// Flujos
// ═══════════════════════════════════════════════════════════════════════════

int64_t Flowsheet::agregar_flujo(const std::string& nombre) {
    nucleo::Flujo f;
    f.id     = prox_id_flujo_++;
    f.nombre = nombre.empty() ? "Flujo_" + std::to_string(f.id) : nombre;
    f.es_energia = false;
    flujos_[f.id] = f;
    return f.id;
}

int64_t Flowsheet::agregar_flujo_energia(const std::string& nombre) {
    nucleo::Flujo f;
    f.id     = prox_id_flujo_++;
    f.nombre = nombre.empty() ? "Energía_" + std::to_string(f.id) : nombre;
    f.es_energia = true;
    flujos_[f.id] = f;
    return f.id;
}

bool Flowsheet::eliminar_flujo(int64_t id) {
    auto it = flujos_.find(id);
    if (it == flujos_.end()) return false;
    for (auto& n : nodos_) {
        auto& in  = n.ids_flujos_entrada;
        auto& out = n.ids_flujos_salida;
        in.erase(std::remove(in.begin(), in.end(), id), in.end());
        out.erase(std::remove(out.begin(), out.end(), id), out.end());
    }
    flujos_.erase(it);
    return true;
}

nucleo::Flujo* Flowsheet::flujo_por_id(int64_t id) {
    auto it = flujos_.find(id);
    return it != flujos_.end() ? &it->second : nullptr;
}

std::map<int64_t, nucleo::Flujo>& Flowsheet::mapa_flujos() { return flujos_; }
const std::map<int64_t, nucleo::Flujo>& Flowsheet::mapa_flujos() const { return flujos_; }

std::vector<nucleo::Flujo> Flowsheet::flujos() const {
    std::vector<nucleo::Flujo> res;
    for (auto& [_, f] : flujos_) res.push_back(f);
    return res;
}

size_t Flowsheet::conteo_flujos_energia() const {
    size_t n = 0;
    for (auto& [_, f] : flujos_) if (f.es_energia) n++;
    return n;
}

// ═══════════════════════════════════════════════════════════════════════════
// Conectividad
// ═══════════════════════════════════════════════════════════════════════════

bool Flowsheet::conexion_valida(int64_t origen, int64_t destino, std::string* motivo) const {
    const auto* from = nodo_por_id(origen);
    const auto* to   = nodo_por_id(destino);
    auto falla = [&](const std::string& m) {
        ultimo_error_conexion_ = m;
        if (motivo) *motivo = m;
        return false;
    };
    if (!from || !to) return falla("Nodo de origen o destino inexistente.");
    if (origen == destino) return falla("No se puede conectar un nodo consigo mismo.");

    auto r_origen = nucleo::rango_puertos(from->tipo);
    auto r_dest   = nucleo::rango_puertos(to->tipo);

    int outs = from->num_salidas();
    if (r_origen.max_salidas >= 0 && outs + 1 > r_origen.max_salidas)
        return falla("El nodo '" + from->nombre + "' (" + nombre_tipo_nodo(from->tipo) +
                     ") admite a lo sumo " + std::to_string(r_origen.max_salidas) + " salida(s).");

    int ins = to->num_entradas();
    if (r_dest.max_entradas >= 0 && ins + 1 > r_dest.max_entradas)
        return falla("El nodo '" + to->nombre + "' (" + nombre_tipo_nodo(to->tipo) +
                     ") admite a lo sumo " + std::to_string(r_dest.max_entradas) + " entrada(s).");

    // Origen no puede producir (max_salidas == 0) o destino no puede recibir (max_entradas == 0)
    if (r_origen.max_salidas == 0)
        return falla("El nodo '" + from->nombre + "' (" + nombre_tipo_nodo(from->tipo) + ") no tiene puertos de salida.");
    if (r_dest.max_entradas == 0)
        return falla("El nodo '" + to->nombre + "' (" + nombre_tipo_nodo(to->tipo) + ") no tiene puertos de entrada.");

    ultimo_error_conexion_.clear();
    if (motivo) *motivo = "";
    return true;
}

bool Flowsheet::conectar(int64_t origen, int64_t destino, int64_t id_flujo) {
    if (!conexion_valida(origen, destino)) return false;

    auto* from = nodo_por_id(origen);
    auto* to   = nodo_por_id(destino);

    int64_t fid = id_flujo;
    if (fid < 0) {
        fid = prox_id_flujo_++;
        nucleo::Flujo f;
        f.id = fid;
        f.nombre = "Flujo_" + std::to_string(fid);
        f.pagina = from->pagina;
        flujos_[fid] = f;
    }

    from->ids_flujos_salida.push_back(fid);
    to->ids_flujos_entrada.push_back(fid);
    return true;
}

bool Flowsheet::desconectar(int64_t id_flujo) {
    auto it = flujos_.find(id_flujo);
    if (it == flujos_.end()) return false;
    for (auto& n : nodos_) {
        auto& in  = n.ids_flujos_entrada;
        auto& out = n.ids_flujos_salida;
        in.erase(std::remove(in.begin(), in.end(), id_flujo), in.end());
        out.erase(std::remove(out.begin(), out.end(), id_flujo), out.end());
    }
    flujos_.erase(it);
    return true;
}

int64_t Flowsheet::destino_del_flujo(int64_t id_flujo) const {
    for (auto& n : nodos_)
        if (std::find(n.ids_flujos_entrada.begin(), n.ids_flujos_entrada.end(), id_flujo) != n.ids_flujos_entrada.end())
            return n.id;
    return -1;
}

// ═══════════════════════════════════════════════════════════════════════════
// Anotaciones
// ═══════════════════════════════════════════════════════════════════════════

int64_t Flowsheet::agregar_anotacion(const std::string& texto) {
    Anotacion a;
    a.id = prox_id_anotacion_++;
    a.texto = texto.empty() ? "Texto" : texto;
    anotaciones_.push_back(a);
    return a.id;
}

Anotacion* Flowsheet::anotacion_por_id(int64_t id) {
    for (auto& a : anotaciones_) if (a.id == id) return &a;
    return nullptr;
}

bool Flowsheet::eliminar_anotacion(int64_t id) {
    auto it = std::find_if(anotaciones_.begin(), anotaciones_.end(),
        [id](const Anotacion& a) { return a.id == id; });
    if (it == anotaciones_.end()) return false;
    anotaciones_.erase(it);
    return true;
}

std::vector<Anotacion>& Flowsheet::anotaciones() { return anotaciones_; }
const std::vector<Anotacion>& Flowsheet::anotaciones() const { return anotaciones_; }

// ═══════════════════════════════════════════════════════════════════════════
// Tablas de flujo
// ═══════════════════════════════════════════════════════════════════════════

int64_t Flowsheet::agregar_tabla_flujo() {
    TablaFlujoLienzo t;
    t.id = prox_id_tabla_++;
    tablas_flujo_.push_back(t);
    return t.id;
}

bool Flowsheet::eliminar_tabla_flujo(int64_t id) {
    auto it = std::find_if(tablas_flujo_.begin(), tablas_flujo_.end(),
        [id](const TablaFlujoLienzo& t) { return t.id == id; });
    if (it == tablas_flujo_.end()) return false;
    tablas_flujo_.erase(it);
    return true;
}

std::vector<TablaFlujoLienzo>& Flowsheet::tablas_flujo() { return tablas_flujo_; }
const std::vector<TablaFlujoLienzo>& Flowsheet::tablas_flujo() const { return tablas_flujo_; }

// ═══════════════════════════════════════════════════════════════════════════
// Topología
// ═══════════════════════════════════════════════════════════════════════════

std::vector<int64_t> Flowsheet::orden_topologico() const {
    std::map<int64_t, int> grado_entrada;
    std::map<int64_t, std::vector<int64_t>> adyacencia;

    for (auto& n : nodos_) {
        grado_entrada[n.id] = n.num_entradas();
        for (int64_t sid : n.ids_flujos_salida) {
            for (auto& m : nodos_) {
                auto it = std::find(m.ids_flujos_entrada.begin(), m.ids_flujos_entrada.end(), sid);
                if (it != m.ids_flujos_entrada.end()) {
                    adyacencia[n.id].push_back(m.id);
                    break;
                }
            }
        }
    }

    std::queue<int64_t> q;
    for (auto& [id, g] : grado_entrada)
        if (g == 0) q.push(id);

    std::vector<int64_t> orden;
    while (!q.empty()) {
        int64_t id = q.front(); q.pop();
        orden.push_back(id);
        for (int64_t suc : adyacencia[id]) {
            grado_entrada[suc]--;
            if (grado_entrada[suc] == 0) q.push(suc);
        }
    }

    return orden;
}

bool Flowsheet::detectar_ciclo() const {
    return orden_topologico().size() != nodos_.size();
}

// ═══════════════════════════════════════════════════════════════════════════
// Motor de convergencia (Wegstein acelerado)
// ═══════════════════════════════════════════════════════════════════════════

Flowsheet::ResultadoConvergencia Flowsheet::resolver_convergencia(
    int max_iter, double tolerancia,
    std::function<void(int, double)> progreso)
{
    ResultadoConvergencia res;

    if (nodos_.empty()) {
        res.mensaje = "El diagrama está vacío.";
        res.convergio = false;
        return res;
    }

    // El motor de masa resuelve lazos de recirculación por tearing (sustitución
    // directa / iteración de punto fijo) sobre los flujos de corte y, al converger,
    // escribimos los resultados en los flujos del flowsheet para que queden visibles.
    cma::motores::BalanceMasa bm;
    auto r = bm.resolver(*this);

    if (r.convergio) {
        for (auto& f_res : r.flujos_salida) {
            auto it = flujos_.find(f_res.id);
            if (it != flujos_.end()) it->second = f_res;
        }
        res.convergio = true;
        res.iteraciones = r.iteraciones;
        res.error_final = r.error_final;
        res.mensaje = r.mensaje;
        if (progreso) progreso(res.iteraciones, res.error_final);
    } else {
        res.convergio = false;
        res.iteraciones = r.iteraciones;
        res.error_final = r.error_final;
        res.mensaje = r.mensaje;
    }

    return res;
}

// ═══════════════════════════════════════════════════════════════════════════
// Serialización JSON (manual, sin dependencias externas)
// ═══════════════════════════════════════════════════════════════════════════

using json = nlohmann::json;

std::string Flowsheet::a_json() const {
    json root;
    root["nodos"] = json::array();
    for (const auto& n : nodos_) {
        json j_nodo;
        j_nodo["id"] = n.id;
        j_nodo["tipo"] = static_cast<int>(n.tipo);
        j_nodo["nombre"] = n.nombre;
        j_nodo["x"] = n.canvas_x;
        j_nodo["y"] = n.canvas_y;
        j_nodo["pagina"] = n.pagina;
        j_nodo["entradas"] = n.ids_flujos_entrada;
        j_nodo["salidas"] = n.ids_flujos_salida;
        json j_params = json::object();
        for (const auto& [k, v] : n.params) {
            j_params[k] = v;
        }
        j_nodo["params"] = j_params;
        root["nodos"].push_back(j_nodo);
    }

    root["flujos"] = json::array();
    for (const auto& [fid, f] : flujos_) {
        json j_flujo;
        j_flujo["id"] = f.id;
        j_flujo["nombre"] = f.nombre;
        j_flujo["T"] = f.temperatura;
        j_flujo["P"] = f.presion;
        j_flujo["H"] = f.entalpia;
        j_flujo["energia"] = f.energia;
        j_flujo["es_energia"] = f.es_energia;
        j_flujo["pagina"] = f.pagina;
        j_flujo["lca_equivalent"] = f.lca_equivalent;
        j_flujo["lca_group"] = f.lca_group;
        j_flujo["measured_flow"] = f.measured_flow;
        j_flujo["sd"] = f.sd;
        j_flujo["mb_selected"] = f.mb_selected;
        j_flujo["mb_assayed"] = f.mb_assayed;
        json j_mol = json::object();
        for (const auto& [sid, val] : f.flujo_molar) {
            j_mol[std::to_string(sid)] = val;
        }
        j_flujo["flujo_molar"] = j_mol;
        root["flujos"].push_back(j_flujo);
    }

    root["anotaciones"] = json::array();
    for (const auto& a : anotaciones_) {
        json j_a;
        j_a["id"] = a.id;
        j_a["tipo"] = static_cast<int>(a.tipo);
        j_a["x"] = a.x;
        j_a["y"] = a.y;
        j_a["w"] = a.w;
        j_a["h"] = a.h;
        j_a["texto"] = a.texto;
        j_a["pagina"] = a.pagina;
        j_a["color_fondo"] = a.color_fondo;
        j_a["color_borde"] = a.color_borde;
        j_a["grosor_borde"] = a.grosor_borde;
        root["anotaciones"].push_back(j_a);
    }

    root["tablas_flujo"] = json::array();
    for (const auto& t : tablas_flujo_) {
        json j_t;
        j_t["id"] = t.id;
        j_t["x"] = t.x;
        j_t["y"] = t.y;
        j_t["w"] = t.w;
        j_t["h"] = t.h;
        j_t["visible"] = t.visible;
        j_t["pagina"] = t.pagina;
        j_t["ids_flujos_mostrados"] = t.ids_flujos_mostrados;
        root["tablas_flujo"].push_back(j_t);
    }

    return root.dump(4);
}

bool Flowsheet::desde_json(const std::string& json_str, std::string& error) {
    limpiar();
    try {
        auto root = json::parse(json_str);

        if (root.contains("nodos") && root["nodos"].is_array()) {
            for (const auto& jn : root["nodos"]) {
                nucleo::Nodo n;
                n.id = jn.value("id", 0LL);
                n.tipo = static_cast<nucleo::TipoNodo>(jn.value("tipo", 0));
                n.nombre = jn.value("nombre", "");
                n.canvas_x = jn.value("x", 0.0f);
                n.canvas_y = jn.value("y", 0.0f);
                n.pagina = jn.value("pagina", 0);
                
                if (jn.contains("entradas") && jn["entradas"].is_array()) {
                    for (auto& val : jn["entradas"]) n.ids_flujos_entrada.push_back(val.get<int64_t>());
                }
                if (jn.contains("salidas") && jn["salidas"].is_array()) {
                    for (auto& val : jn["salidas"]) n.ids_flujos_salida.push_back(val.get<int64_t>());
                }
                if (jn.contains("params") && jn["params"].is_object()) {
                    for (auto it = jn["params"].begin(); it != jn["params"].end(); ++it) {
                        n.params[it.key()] = it.value().get<double>();
                    }
                }
                nodos_.push_back(n);
                if (n.id >= prox_id_nodo_) prox_id_nodo_ = n.id + 1;
            }
        }

        if (root.contains("flujos") && root["flujos"].is_array()) {
            for (const auto& jf : root["flujos"]) {
                nucleo::Flujo f;
                f.id = jf.value("id", 0LL);
                f.nombre = jf.value("nombre", "");
                f.temperatura = jf.value("T", 298.15);
                f.presion = jf.value("P", 1.0);
                f.entalpia = jf.value("H", 0.0);
                f.energia = jf.value("energia", 0.0);
                f.es_energia = jf.value("es_energia", false);
                f.pagina = jf.value("pagina", 0);
                f.lca_equivalent = jf.value("lca_equivalent", "");
                f.lca_group = jf.value("lca_group", "");
                f.measured_flow = jf.value("measured_flow", 0.0);
                f.sd = jf.value("sd", 0.0);
                f.mb_selected = jf.value("mb_selected", false);
                f.mb_assayed = jf.value("mb_assayed", false);

                if (jf.contains("flujo_molar") && jf["flujo_molar"].is_object()) {
                    for (auto it = jf["flujo_molar"].begin(); it != jf["flujo_molar"].end(); ++it) {
                        try {
                            f.flujo_molar[std::stoll(it.key())] = it.value().get<double>();
                        } catch(...) {}
                    }
                }
                flujos_[f.id] = f;
                if (f.id >= prox_id_flujo_) prox_id_flujo_ = f.id + 1;
            }
        }

        if (root.contains("anotaciones") && root["anotaciones"].is_array()) {
            for (const auto& ja : root["anotaciones"]) {
                Anotacion a;
                a.id = ja.value("id", 0LL);
                a.tipo = static_cast<Anotacion::Tipo>(ja.value("tipo", 0));
                a.x = ja.value("x", 0.0f);
                a.y = ja.value("y", 0.0f);
                a.w = ja.value("w", 150.0f);
                a.h = ja.value("h", 40.0f);
                a.texto = ja.value("texto", "");
                a.pagina = ja.value("pagina", 0);
                a.color_fondo = ja.value("color_fondo", 0x282832C8);
                a.color_borde = ja.value("color_borde", 0x64648CC8);
                a.grosor_borde = ja.value("grosor_borde", 2.0f);
                anotaciones_.push_back(a);
                if (a.id >= prox_id_anotacion_) prox_id_anotacion_ = a.id + 1;
            }
        }

        if (root.contains("tablas_flujo") && root["tablas_flujo"].is_array()) {
            for (const auto& jt : root["tablas_flujo"]) {
                TablaFlujoLienzo t;
                t.id = jt.value("id", 0LL);
                t.x = jt.value("x", 200.0f);
                t.y = jt.value("y", 200.0f);
                t.w = jt.value("w", 300.0f);
                t.h = jt.value("h", 120.0f);
                t.visible = jt.value("visible", true);
                t.pagina = jt.value("pagina", 0);
                if (jt.contains("ids_flujos_mostrados") && jt["ids_flujos_mostrados"].is_array()) {
                    for (auto& val : jt["ids_flujos_mostrados"]) t.ids_flujos_mostrados.push_back(val.get<int64_t>());
                }
                tablas_flujo_.push_back(t);
                if (t.id >= prox_id_tabla_) prox_id_tabla_ = t.id + 1;
            }
        }

        return true;
    } catch (const std::exception& e) {
        error = e.what();
        return false;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Limpiar
// ═══════════════════════════════════════════════════════════════════════════

void Flowsheet::limpiar() {
    nodos_.clear();
    flujos_.clear();
    anotaciones_.clear();
    tablas_flujo_.clear();
    prox_id_nodo_  = 1;
    prox_id_flujo_ = 1;
    prox_id_anotacion_ = 1;
    prox_id_tabla_ = 1;
}

}} // namespace
