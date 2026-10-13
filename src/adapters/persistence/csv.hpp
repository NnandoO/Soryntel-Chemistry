#pragma once
#include <string>
#include <vector>
#include <cmath>
#include <stdexcept>
namespace cma::adapters {
inline std::vector<std::string> csv_row(const std::string& line) {
    std::vector<std::string> result;
    std::string field;
    bool quoted = false, closed = false;
    for (size_t i = 0; i < line.size(); ++i) {
        const char c = line[i];
        if (quoted) {
            if (c == '"') {
                if (i + 1 < line.size() && line[i + 1] == '"') { field += '"'; ++i; }
                else { quoted = false; closed = true; }
            } else field += c;
        } else if (c == ',') { result.push_back(field); field.clear(); closed = false; }
        else if (c == '"' && field.empty() && !closed) quoted = true;
        else if (closed && c != ' ' && c != '\r') throw std::invalid_argument("Texto después de comillas CSV");
        else if (!closed) field += c;
    }
    if (quoted) throw std::invalid_argument("Comillas CSV sin cerrar");
    result.push_back(field);
    return result;
}
inline double csv_number(const std::string& token) {
    size_t end = 0;
    const double value = std::stod(token, &end);
    if (!std::isfinite(value) || token.find_first_not_of(" \t\r", end) != std::string::npos)
        throw std::invalid_argument("Número CSV inválido");
    return value;
}
}
