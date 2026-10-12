#pragma once
#include <string>
#include <vector>

namespace hsc_clone {

class ExternalAPI {
public:
    std::string ejecutarComando(const std::string& cmd);
    bool importarCSV(const std::string& ruta, std::vector<std::vector<double>>& datos);
    bool exportarJSON(const std::string& ruta, const std::string& json);
};

} // namespace hsc_clone
