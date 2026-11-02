#include "shared.hpp"
#include <algorithm>
#include <cstdio>
#include <sstream>

namespace cma { namespace interfaz {

namespace {

std::string ruta_dialogo_hmb(const char* argumentos) {
    std::string ruta;
    FILE* zen = adapters::dialog_pipe(std::string("zenity ") + argumentos + " 2>/dev/null");
    if (zen) {
        char buf[1024] = {0};
        if (std::fgets(buf, sizeof(buf), zen)) {
            ruta = trim_str(buf);
        }
        std::fclose(zen);
    }
    return ruta;
}

} // namespace

// ─────────────────────────────────────────────────────────────────────────────
// E / S de archivos
// ─────────────────────────────────────────────────────────────────────────────

void PanelBalanceHoja::abrir_txt() {
    const std::string ruta = ruta_dialogo_hmb(
        "--file-selection --file-filter='*.txt' --title='Abrir hoja de balance (HMB)'");
    if (ruta.empty()) return;

    FILE* fp = std::fopen(ruta.c_str(), "r");
    if (!fp) return;

    std::vector<CorrienteUI> nuevas_entradas, nuevas_salidas;
    bool seccion_salidas = false;
    char linea[1024];
    while (std::fgets(linea, sizeof(linea), fp)) {
        const std::string l = trim_str(linea);
        if (l.empty()) continue;
        if (l[0] == '#') {
            if (l.find("Outputs") != std::string::npos ||
                l.find("SALIDAS") != std::string::npos) {
                seccion_salidas = true;
            }
            continue;
        }
        auto partes = split_str(l, ",");
        if (partes.empty()) continue;

        if (partes[0] == "UNITS") {
            for (auto& p : partes) {
                const std::string t = trim_str(p);
                if (t == "temp=K") temp_unit_ = 1;
                else if (t == "temp=C") temp_unit_ = 0;
                else if (t == "energy=Mcal") energy_unit_ = 0;
                else if (t == "energy=MJ") energy_unit_ = 1;
                else if (t == "energy=kWh") energy_unit_ = 2;
                else if (t == "energy=kJ") energy_unit_ = 3;
                else if (t == "energy=kcal") energy_unit_ = 4;
            }
            continue;
        }

        if (partes[0] == "STREAM" && partes.size() >= 2) {
            bool salida = seccion_salidas;
            size_t base = 1;
            if (partes.size() >= 4 && (partes[1] == "IN" || partes[1] == "OUT")) {
                salida = (partes[1] == "OUT");
                base = 2;
            }
            CorrienteUI c;
            copiar_texto(c.nombre, sizeof(c.nombre), trim_str(partes[base]));
            if (partes.size() > base + 1) {
                try { c.calor = std::stod(partes[base + 1]); } catch (...) { c.calor = 0.0; }
            }
            if (partes.size() > base + 2) {
                try { c.temperatura_fija = (std::stoi(partes[base + 2]) != 0); } catch (...) {}
            }
            (salida ? nuevas_salidas : nuevas_entradas).push_back(c);
        } else if (partes[0] == "ROW" && partes.size() >= 5) {
            FilaUI f;
            copiar_texto(f.formula, sizeof(f.formula), trim_str(partes[1]));
            try {
                f.temper = std::stod(partes[2]);
                f.presion = std::stod(partes[3]);
                f.kmol = std::stod(partes[4]);
            } catch (...) {
                continue;
            }
            auto& destino = seccion_salidas ? nuevas_salidas : nuevas_entradas;
            if (!destino.empty()) destino.back().filas.push_back(f);
        }
    }
    std::fclose(fp);

    if (!nuevas_entradas.empty() || !nuevas_salidas.empty()) {
        entradas_ = std::move(nuevas_entradas);
        salidas_ = std::move(nuevas_salidas);
        corriente_sel_ = 0;
        fila_sel_ = -1;
        ultimo_solver_ = motores::BalanceHoja::ResultadoTemperatura{};
        recalcular(true);
    }
}

void PanelBalanceHoja::guardar_txt() {
    std::string ruta = ruta_dialogo_hmb(
        "--file-selection --save --confirm-overwrite --file-filter='*.txt' "
        "--title='Guardar hoja de balance (HMB)'");
    if (ruta.empty()) return;
    if (ruta.find(".txt") == std::string::npos) ruta += ".txt";

    FILE* fp = std::fopen(ruta.c_str(), "w");
    if (!fp) return;

    std::fprintf(fp, "# CMA HMB v2\n");
    std::fprintf(fp, "UNITS,temp=%s,energy=%s\n",
                 temp_unit_ == 0 ? "C" : "K", etiqueta_energia());

    auto escribir = [&](const std::vector<CorrienteUI>& grupos, const char* direccion) {
        for (const auto& c : grupos) {
            std::string nombre = c.nombre;
            std::replace(nombre.begin(), nombre.end(), ',', ';');
            std::fprintf(fp, "STREAM,%s,%s,%.10g,%d\n",
                         direccion, nombre.c_str(), c.calor, c.temperatura_fija ? 1 : 0);
            for (const auto& f : c.filas) {
                std::fprintf(fp, "ROW,%s,%.10g,%.10g,%.10g\n",
                             f.formula, f.temper, f.presion, f.kmol);
            }
        }
    };
    escribir(entradas_, "IN");
    escribir(salidas_, "OUT");

    std::fprintf(fp, "\n# === RESULTADOS DEL BALANCE (HSC / CMA) ===\n");
    std::istringstream iss(generar_informe());
    std::string linea_inf;
    while (std::getline(iss, linea_inf)) {
        std::fprintf(fp, "# %s\n", linea_inf.c_str());
    }
    std::fclose(fp);
}

void PanelBalanceHoja::exportar_csv() {
    std::string ruta = ruta_dialogo_hmb(
        "--file-selection --save --confirm-overwrite --file-filter='*.csv' "
        "--title='Exportar resultados (CSV)'");
    if (ruta.empty()) return;
    if (ruta.find(".csv") == std::string::npos) ruta += ".csv";

    FILE* fp = std::fopen(ruta.c_str(), "w");
    if (!fp) return;

    std::fprintf(fp,
        "Tipo,Corriente,Formula,Temperatura(%s),Presion(bar),kmol,kg,Nm3,Cp(kJ/K),"
        "Hformacion(%s),Hsensible(%s),Htotal(%s),Exergia(%s),Calor_externo(%s)\n",
        etiqueta_temperatura(), etiqueta_energia(), etiqueta_energia(),
        etiqueta_energia(), etiqueta_energia(), etiqueta_energia());

    auto escribir = [&](const std::vector<motores::CorrienteBalance>& grupos, const char* tipo) {
        for (const auto& c : grupos) {
            const double calor = motores::energia_desde_kJ(c.calor_kJ, unidad_energia());
            for (const auto& f : c.filas) {
                const double T = motores::temperatura_desde_K(f.temperatura_K,
                                                              unidad_temperatura());
                const double ue = motores::factor_energia_kJ(unidad_energia());
                std::fprintf(fp, "%s,%s,%s,%.6g,%.6g,%.6g,%.6g,%.6g,%.6g,%.6g,%.6g,%.6g,%.6g,%.6g\n",
                             tipo, c.nombre.c_str(), f.formula.c_str(), T, f.presion_bar,
                             f.kmol, f.masa_kg, f.volumen_nm3, f.cp_kJ_K,
                             f.h_formacion_kJ / ue, f.h_sensible_kJ / ue,
                             f.h_total_kJ / ue, f.exergia_total_kJ / ue, calor);
            }
        }
    };
    escribir(area_.entradas, "ENTRADA");
    escribir(area_.salidas, "SALIDA");
    std::fprintf(fp, "\nRESUMEN\n");
    std::fprintf(fp, "kmol_in,%.6g\nkg_in,%.6g\nH_in_kJ,%.6g\n", resultado_.kmol_entrada,
                 resultado_.masa_entrada_kg, resultado_.h_entrada_kJ);
    std::fprintf(fp, "kmol_out,%.6g\nkg_out,%.6g\nH_out_kJ,%.6g\n", resultado_.kmol_salida,
                 resultado_.masa_salida_kg, resultado_.h_salida_kJ);
    std::fprintf(fp, "calor_externo_kJ,%.6g\nresidual_energia_kJ,%.6g\n",
                 resultado_.calor_externo_kJ, resultado_.residual_energia_kJ);
    std::fclose(fp);
}

void PanelBalanceHoja::copiar_informe() {
    const std::string informe = generar_informe();
    ImGui::SetClipboardText(informe.c_str());
}

std::string PanelBalanceHoja::generar_informe() const {
    std::ostringstream os;
    os << "=== BALANCE DE CALOR Y MATERIALES - CMA Chemistry ===\n";
    os << "Unidades: Temperatura = " << etiqueta_temperatura()
       << " | Energia = " << etiqueta_energia() << "\n\n";

    auto corriente = [&](const motores::CorrienteBalance& c, const char* tipo) {
        os << "[" << tipo << "] " << c.nombre
           << "   (calor externo: "
           << motores::energia_desde_kJ(c.calor_kJ, unidad_energia()) << " "
           << etiqueta_energia() << ")\n";
        for (const auto& f : c.filas) {
            os << "    " << f.formula
               << "  T=" << motores::temperatura_desde_K(f.temperatura_K, unidad_temperatura())
               << " " << etiqueta_temperatura()
               << "  P=" << f.presion_bar << " bar"
               << "  n=" << f.kmol << " kmol"
               << "  m=" << f.masa_kg << " kg";
            if (f.especie != nullptr) {
                os << "  H=" << motores::energia_desde_kJ(f.h_total_kJ, unidad_energia())
                   << " " << etiqueta_energia();
            } else {
                os << "  [sin datos termodinamicos]";
            }
            os << "\n";
        }
    };

    for (const auto& c : area_.entradas) corriente(c, "ENTRADA");
    for (const auto& c : area_.salidas) corriente(c, "SALIDA");

    os << "\n--- RESUMEN ---\n";
    os << "Entradas: kmol=" << resultado_.kmol_entrada
       << "  kg=" << resultado_.masa_entrada_kg
       << "  H=" << motores::energia_desde_kJ(resultado_.h_entrada_kJ, unidad_energia())
       << " " << etiqueta_energia()
       << "  Exergia=" << motores::energia_desde_kJ(resultado_.exergia_entrada_kJ, unidad_energia())
       << " " << etiqueta_energia() << "\n";
    os << "Salidas:  kmol=" << resultado_.kmol_salida
       << "  kg=" << resultado_.masa_salida_kg
       << "  H=" << motores::energia_desde_kJ(resultado_.h_salida_kJ, unidad_energia())
       << " " << etiqueta_energia()
       << "  Exergia=" << motores::energia_desde_kJ(resultado_.exergia_salida_kJ, unidad_energia())
       << " " << etiqueta_energia() << "\n";
    os << "Residual masa: " << resultado_.residual_masa_kg << " kg ("
       << resultado_.error_masa_pct << " %)\n";
    os << "Residual energia: "
       << motores::energia_desde_kJ(resultado_.residual_energia_kJ, unidad_energia())
       << " " << etiqueta_energia() << " (" << resultado_.error_energia_pct << " %)\n";
    os << "Calor externo neto: "
       << motores::energia_desde_kJ(resultado_.calor_externo_kJ, unidad_energia())
       << " " << etiqueta_energia() << "\n";
    os << "Exergia destruida: "
       << motores::energia_desde_kJ(resultado_.exergia_destruida_kJ, unidad_energia())
       << " " << etiqueta_energia()
       << "  Eficiencia exergetica: " << resultado_.eficiencia_exergetica_pct << " %\n";

    if (!resultado_.avisos.empty()) {
        os << "\n--- AVISOS ---\n";
        for (const auto& a : resultado_.avisos) os << "  * " << a << "\n";
    }
    return os.str();
}

}} // namespace cma::interfaz
