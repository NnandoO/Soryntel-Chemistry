#pragma once
#include "application/modules.hpp"
#include <string>
#include <vector>

namespace cma::application::modules {

class ExternalAPI {
public:
    bool importarCSV(const std::string& ruta, std::vector<std::vector<double>>& datos);
    bool exportarJSON(const std::string& ruta, const std::string& json);
};

} // namespace cma::application::modules
