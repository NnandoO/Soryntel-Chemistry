#pragma once
#ifndef _USE_MATH_DEFINES
#define _USE_MATH_DEFINES
#endif
#include "interfaz/paneles.hpp"
#include "utils/i18n.h"
#include <imgui.h>
#include <implot.h>
#include <implot3d.h>
#include "interfaz/paneles.hpp"
#include <nlohmann/json.hpp>
#include <fstream>
#include <cstring>
#include <cstdlib>
#include "motores/perdida_calor.hpp"
#include "base_datos/gestor_db.hpp"
#include "base_datos/gestor_elementos.hpp"
#include "base_datos/gestor_unidades.hpp"
#include <iostream>
#include <iomanip>
#include <set>
#include "thermo/GibbsMinimizer.hpp"
#include "adapters/desktop/process.hpp"


namespace cma { namespace interfaz {
bool parse_chemical_equation(const std::string&, nucleo::Reaccion&, std::string&);
std::map<std::string, int> parse_formula_elements(const std::string&, std::string&);

inline ImVec4 AdaptiveColor(float r, float g, float b, float a) {
    if (ImGui::GetStyle().Colors[ImGuiCol_WindowBg].x > 0.5f) {
        if (r > 0.8f && g > 0.8f && b > 0.8f) return ImVec4(0.1f, 0.1f, 0.1f, a);
        return ImVec4(r * 0.4f, g * 0.4f, b * 0.4f, a);
    }
    return ImVec4(r, g, b, a);
}


inline std::string trim_str(const std::string& str) {
    size_t first = str.find_first_not_of(" \t\r\n");
    if (first == std::string::npos) return "";
    size_t last = str.find_last_not_of(" \t\r\n");
    return str.substr(first, (last - first + 1));
}

inline std::string convert_to_subscript(const std::string& s) {
    std::string res;
    for (size_t i = 0; i < s.length(); ++i) {
        char c = s[i];
        if (c >= '0' && c <= '9') {
            res.push_back((char)0xE2);
            res.push_back((char)0x82);
            res.push_back((char)(0x80 + (c - '0')));
        } else {
            res.push_back(c);
        }
    }
    return res;
}


inline const cma::nucleo::Especie* buscar_especie_por_nombre_o_formula(const std::string& query_raw) {
    std::string query = trim_str(query_raw);
    if (query.empty()) return nullptr;

    auto& gestor = cma::base_datos::GestorDB::instancia();

    // Strip state tags like (g), (s), (l), (a), (+a), (-a), (+3a), etc.
    std::string formula_clean;
    size_t len = query.length();
    for (size_t k = 0; k < len; ++k) {
        if (query[k] == '(') {
            size_t close = query.find(')', k);
            if (close != std::string::npos) {
                std::string tag = query.substr(k + 1, close - k - 1);
                if (tag == "g" || tag == "s" || tag == "l" || tag == "a" || tag == "aq" || tag == "α" || tag == "+a" || tag == "-a" || tag.find("a") != std::string::npos || tag.find("+") != std::string::npos || tag.find("-") != std::string::npos) {
                    k = close;
                    continue;
                }
            }
        }
        formula_clean += query[k];
    }

    // 1) Coincidencia EXACTA incluyendo el tag de fase/carga (como HSC):
    //    "Mg(g)" != "Mg"  y  "Ag(+a)" != "Ag"
    if (const auto* esp = gestor.especie_por_nombre(query)) return esp;
    std::string q_sub_full = convert_to_subscript(query);
    for (const auto* e : gestor.todas_especies()) {
        if (e->formula == query || e->formula == q_sub_full) return e;
    }

    // 2) Nombre con/sin tag y fórmula sin tag (fallback genérico)
    if (const auto* esp = gestor.especie_por_nombre(formula_clean)) return esp;
    std::string q_sub = convert_to_subscript(formula_clean);
    for (const auto* e : gestor.todas_especies()) {
        if (e->formula == q_sub || e->formula == formula_clean) {
            return e;
        }
    }
    return nullptr;
}

inline std::vector<std::string> split_str(const std::string& s, const std::string& delim) {
    std::vector<std::string> parts;
    size_t start = 0;
    size_t end = s.find(delim);
    while (end != std::string::npos) {
        parts.push_back(s.substr(start, end - start));
        start = end + delim.length();
        end = s.find(delim, start);
    }
    parts.push_back(s.substr(start));
    return parts;
}

}} // namespace
