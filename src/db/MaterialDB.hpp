#pragma once
#include "application/modules.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "nucleo/tipos.hpp"
#include <vector>

namespace cma::application::modules {

class MaterialDB {
public:
    explicit MaterialDB(const cma::domain::ThermoRepository& repository);
    std::vector<const cma::nucleo::Especie*> materiales();
private:
    const cma::domain::ThermoRepository& motor_;
};

}
