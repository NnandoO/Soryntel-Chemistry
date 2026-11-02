#include "shared.hpp"
#include "panel_lca.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <sstream>

namespace cma { namespace interfaz {

namespace {

std::string ruta_dialogo_lca(const char* argumentos) {
    std::string ruta;
    FILE* zen = adapters::dialog_pipe(std::string("zenity ") + argumentos + " 2>/dev/null");
    if (zen != nullptr) {
        char buf[1024] = {0};
        if (std::fgets(buf, sizeof(buf), zen)) ruta = trim_str(buf);
        std::fclose(zen);
    }
    return ruta;
}

std::string campo_csv(const std::string& valor) {
    if (valor.find_first_of(",\"\n\r") == std::string::npos) return valor;
    std::string salida = "\"";
    for (char c : valor) {
        if (c == '"') salida += "\"\"";
        else salida += c;
    }
    salida += '"';
    return salida;
}

std::string minusculas(std::string s) {
    for (char& c : s) {
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c - 'A' + 'a');
    }
    return s;
}

void asegurar_extension(std::string& ruta, const char* extension) {
    const std::string objetivo = minusculas(extension);
    if (ruta.size() < objetivo.size()) {
        ruta += extension;
        return;
    }
    if (minusculas(ruta.substr(ruta.size() - objetivo.size())) != objetivo) ruta += extension;
}

std::vector<std::string> parsear_csv(const std::string& linea) {
    std::vector<std::string> campos;
    std::string actual;
    bool comillas = false;
    for (size_t i = 0; i < linea.size(); ++i) {
        const char c = linea[i];
        if (comillas) {
            if (c == '"') {
                if (i + 1 < linea.size() && linea[i + 1] == '"') {
                    actual += '"';
                    ++i;
                } else {
                    comillas = false;
                }
            } else {
                actual += c;
            }
        } else if (c == '"') {
            comillas = true;
        } else if (c == ',') {
            campos.push_back(actual);
            actual.clear();
        } else if (c != '\r') {
            actual += c;
        }
    }
    campos.push_back(actual);
    return campos;
}

std::string nombre_direccion(bool entrada) {
    return entrada ? "Entrada" : "Salida";
}

} // namespace

// ─────────────────────────────────────────────────────────────────────────────
// Exportación CSV (GaBi / Excel)
// ─────────────────────────────────────────────────────────────────────────────
void PanelLCA::exportar_gabi(bool excel) {
    if (!calculado_) calcular();

    const std::string ruta = ruta_dialogo_lca(
        excel
            ? "--file-selection --save --confirm-overwrite --file-filter='*.csv' "
              "--title='Exportar dataset LCA (Excel CSV)'"
            : "--file-selection --save --confirm-overwrite --file-filter='*.csv' "
              "--title='Exportar mapeo LCA (GaBi CSV)'");
    if (ruta.empty()) return;

    std::string destino = ruta;
    asegurar_extension(destino, ".csv");

    FILE* fp = std::fopen(destino.c_str(), "wb");
    if (fp == nullptr) {
        mensaje_estado_ = "No se pudo escribir el archivo CSV.";
        return;
    }

    if (excel) std::fputs("\xEF\xBB\xBF", fp);

    std::fprintf(fp, "# CMA Chemistry - Evaluacion LCA\n");
    std::fprintf(fp, "# Unidad funcional base: %.10g kg/h\n", unidad_funcional_kg_);
    std::fprintf(fp, "# Factor de normalizacion: %.10g 1/kg\n",
                 resultado_.factor_unidad_funcional);
    std::fprintf(fp, "# Direccion: Entrada = recurso o consumo; Salida = emision o producto\n");
    std::fprintf(fp, "stream_id,equipo,corriente,especie,kg_h,kg_h_normalizado,"
                     "equivalente_id,equivalente,grupo,direccion,estado\n");

    const double factor = resultado_.factor_unidad_funcional;
    for (const auto& f : filas_) {
        const auto* e = motor_.buscar_equivalente(f.equivalente);
        const std::string nombre = e != nullptr ? e->nombre : "Sin mapear";
        const std::string grupo = e != nullptr ? e->grupo : "Not defined";
        const std::string id = e != nullptr ? e->id : "";
        std::fprintf(fp, "%lld,%s,%s,%s,%.10g,%.10g,%s,%s,%s,%s,%s\n",
                     static_cast<long long>(f.stream_id),
                     campo_csv(f.equipo).c_str(), campo_csv(f.corriente).c_str(),
                     campo_csv(f.especie).c_str(), f.kg_h, f.kg_h * factor,
                     id.c_str(), campo_csv(nombre).c_str(), campo_csv(grupo).c_str(),
                     nombre_direccion(f.entrada).c_str(),
                     e != nullptr ? "Mapeado" : "Sin mapear");
    }

    std::fprintf(fp, "\n# INDICADORES\nindicador,valor,unidad\n");
    std::fprintf(fp, "GWP100,%.10g,kg CO2-eq\n", resultado_.gwp_kg_co2eq);
    std::fprintf(fp, "Acidificacion,%.10g,kg SO2-eq\n", resultado_.ap_kg_so2eq);
    std::fprintf(fp, "Eutrofizacion,%.10g,kg PO4-eq\n", resultado_.ep_kg_po4eq);
    std::fprintf(fp, "Agua,%.10g,m3\n", resultado_.agua_m3);
    std::fprintf(fp, "Energia primaria,%.10g,MJ\n", resultado_.energia_mj);
    std::fprintf(fp, "Residuos,%.10g,kg\n", resultado_.residuos_kg);
    std::fprintf(fp, "Masa entrada,%.10g,kg\n", resultado_.masa_entrada_kg);
    std::fprintf(fp, "Masa salida,%.10g,kg\n", resultado_.masa_salida_kg);
    std::fprintf(fp, "Balance de masa,%.10g,1/0\n", resultado_.balance_masa_ok ? 1.0 : 0.0);
    std::fprintf(fp, "Filas sin mapear,%d,\n", resultado_.filas_sin_mapear);

    std::fclose(fp);
    mensaje_estado_ = "Exportado: " + destino;
}

void PanelLCA::exportar_json() {
    if (!calculado_) calcular();

    const std::string ruta = ruta_dialogo_lca(
        "--file-selection --save --confirm-overwrite --file-filter='*.json' "
        "--title='Exportar dataset LCA (JSON)'");
    if (ruta.empty()) return;

    std::string destino = ruta;
    asegurar_extension(destino, ".json");

    nlohmann::json j;
    j["herramienta"] = "CMA Chemistry";
    j["modulo"] = "Evaluacion LCA";
    j["version"] = 1;
    j["unidad_funcional_kg_h"] = unidad_funcional_kg_;
    j["factor_normalizacion"] = resultado_.factor_unidad_funcional;

    j["consumos_manuales"] = {
        {"electricidad_kWh_h", manual_electricidad_},
        {"gas_natural_Nm3_h", manual_gas_},
        {"combustible_diesel_kg_h", manual_combustible_},
        {"agua_m3_h", manual_agua_},
        {"residuo_kg_h", manual_residuo_},
    };

    j["indicadores"] = {
        {"gwp100_kg_co2eq", resultado_.gwp_kg_co2eq},
        {"acidificacion_kg_so2eq", resultado_.ap_kg_so2eq},
        {"eutrofizacion_kg_po4eq", resultado_.ep_kg_po4eq},
        {"agua_m3", resultado_.agua_m3},
        {"energia_primaria_mj", resultado_.energia_mj},
        {"residuos_kg", resultado_.residuos_kg},
        {"masa_entrada_kg", resultado_.masa_entrada_kg},
        {"masa_salida_kg", resultado_.masa_salida_kg},
        {"balance_masa_ok", resultado_.balance_masa_ok},
        {"filas_totales", resultado_.filas_totales},
        {"filas_mapeadas", resultado_.filas_mapeadas},
        {"filas_sin_mapear", resultado_.filas_sin_mapear},
    };

    j["flujos"] = nlohmann::json::array();
    const double factor = resultado_.factor_unidad_funcional;
    for (const auto& f : filas_) {
        const auto* e = motor_.buscar_equivalente(f.equivalente);
        j["flujos"].push_back({
            {"stream_id", f.stream_id},
            {"equipo", f.equipo},
            {"corriente", f.corriente},
            {"especie", f.especie},
            {"kg_h", f.kg_h},
            {"kg_h_normalizado", f.kg_h * factor},
            {"direccion", nombre_direccion(f.entrada)},
            {"equivalente_id", e != nullptr ? e->id : ""},
            {"equivalente", e != nullptr ? e->nombre : "Sin mapear"},
            {"grupo", e != nullptr ? e->grupo : "Not defined"},
            {"mapeado", e != nullptr},
        });
    }

    j["avisos"] = resultado_.avisos;

    std::ofstream salida(destino);
    if (!salida) {
        mensaje_estado_ = "No se pudo escribir el archivo JSON.";
        return;
    }
    salida << j.dump(2) << '\n';
    mensaje_estado_ = "Exportado: " + destino;
}

// ─────────────────────────────────────────────────────────────────────────────
// Importación del CSV de mapeos
// ─────────────────────────────────────────────────────────────────────────────
void PanelLCA::importar_mapeos(diagrama_flujo::Flowsheet* flowsheet) {
    if (flowsheet == nullptr) return;

    const std::string ruta = ruta_dialogo_lca(
        "--file-selection --file-filter='*.csv' --title='Importar mapeos LCA (CSV)'");
    if (ruta.empty()) return;

    std::ifstream entrada(ruta);
    if (!entrada) {
        mensaje_estado_ = "No se pudo abrir el archivo CSV.";
        return;
    }

    std::string linea;
    bool en_tabla = false;
    int aplicados = 0;
    int desconocidos = 0;

    while (std::getline(entrada, linea)) {
        if (linea.empty()) continue;
        if (linea[0] == '#') continue;
        if (!en_tabla) {
            if (linea.find("stream_id") != std::string::npos) en_tabla = true;
            continue;
        }

        const auto campos = parsear_csv(linea);
        if (campos.size() < 6) continue;

        char* fin = nullptr;
        const long long stream_id = std::strtoll(campos[0].c_str(), &fin, 10);
        if (fin == campos[0].c_str()) continue;
        const std::string& especie = campos[3];
        const std::string& id = campos[5];
        if (especie.empty() || id.empty()) continue;

        if (motor_.buscar_equivalente(id) == nullptr) {
            ++desconocidos;
            continue;
        }
        auto* flujo = flowsheet->flujo_por_id(static_cast<int64_t>(stream_id));
        if (flujo == nullptr) continue;
        flujo->lca_especies[especie] = id;
        ++aplicados;
    }

    cargar_flujos(flowsheet);
    mensaje_estado_ = "Mapeos importados: " + std::to_string(aplicados);
    if (desconocidos > 0) {
        mensaje_estado_ += " · ignorados por id desconocido: " + std::to_string(desconocidos);
    }
}

}} // namespace cma::interfaz
