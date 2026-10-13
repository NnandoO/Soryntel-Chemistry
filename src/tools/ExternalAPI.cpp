#include "ExternalAPI.hpp"
#include <cstdio>
#include <fstream>
#include <sstream>
#include <algorithm>
#include <cstring>
#include <memory>
#include <cmath>
#include <cctype>

namespace cma::application::modules {

bool ExternalAPI::importarCSV(const std::string& ruta,
                              std::vector<std::vector<double>>& datos)
{
    std::ifstream file(ruta);
    if (!file.is_open()) return false;

    std::vector<std::vector<double>> parsed;
    size_t width = 0;
    std::string linea;
    while (std::getline(file, linea)) {
        if (linea.empty()) continue;
        std::vector<double> fila;
        std::stringstream ss(linea);
        std::string token;
        while (std::getline(ss, token, ',')) {
            token.erase(std::remove_if(token.begin(), token.end(),
                [](unsigned char c) { return std::isspace(c); }), token.end());
            try {
                size_t consumed = 0;
                double value = std::stod(token, &consumed);
                if (consumed != token.size() || !std::isfinite(value)) return false;
                fila.push_back(value);
            } catch (...) {
                return false;
            }
        }
        if (!linea.empty() && linea.back() == ',') return false;
        if (fila.empty()) return false;
        if (width == 0) width = fila.size();
        if (fila.size() != width) return false;
        parsed.push_back(std::move(fila));
    }
    if (file.bad() || parsed.empty()) return false;
    datos = std::move(parsed);
    return true;
}

bool ExternalAPI::exportarJSON(const std::string& ruta, const std::string& json) {
    std::ofstream file(ruta);
    if (!file.is_open()) return false;
    file << json;
    return file.good();
}

} // namespace cma::application::modules
