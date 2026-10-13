#pragma once
#include "application/modules.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "nucleo/tipos.hpp"
#include <string>

namespace cma::application::modules {

class ThermoDB {
public:
    explicit ThermoDB(const cma::domain::ThermoRepository& repository);
    const cma::nucleo::Especie* especiePorNombre(const std::string& nombre);
    const cma::nucleo::Especie* especiePorId(int64_t id);
private:
    const cma::domain::ThermoRepository& motor_;
};

}
