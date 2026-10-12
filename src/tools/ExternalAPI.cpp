#include "ExternalAPI.hpp"
#include <cstdio>
#include <fstream>
#include <sstream>
#include <algorithm>
#include <cstring>
#include <memory>

namespace hsc_clone {

std::string ExternalAPI::ejecutarComando(const std::string& cmd) {
    // Sanitización básica para evitar inyección de shell
    std::string safe_cmd = cmd;
    std::string bad_chars = ";|&$`\"'<>";
    safe_cmd.erase(std::remove_if(safe_cmd.begin(), safe_cmd.end(),
        [&bad_chars](char c) { return bad_chars.find(c) != std::string::npos; }), safe_cmd.end());

    std::string resultado;
    std::unique_ptr<FILE, decltype(&pclose)> pipe(popen(safe_cmd.c_str(), "r"), pclose);
    if (!pipe) {
        return "Error al ejecutar comando";
    }
    char buffer[4096];
    while (fgets(buffer, sizeof(buffer), pipe.get()) != nullptr)
        resultado += buffer;
    return resultado;
}

bool ExternalAPI::importarCSV(const std::string& ruta,
                              std::vector<std::vector<double>>& datos)
{
    std::ifstream file(ruta);
    if (!file.is_open()) return false;

    datos.clear();
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
                fila.push_back(std::stod(token));
            } catch (...) {
                fila.push_back(0.0);
            }
        }
        if (!fila.empty())
            datos.push_back(fila);
    }
    return !datos.empty();
}

bool ExternalAPI::exportarJSON(const std::string& ruta, const std::string& json) {
    std::ofstream file(ruta);
    if (!file.is_open()) return false;
    file << json;
    return file.good();
}

} // namespace hsc_clone
