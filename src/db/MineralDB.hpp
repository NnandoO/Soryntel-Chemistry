#pragma once
#include "application/modules.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "nucleo/tipos.hpp"
#include <vector>

namespace cma::application::modules {

class MineralDB {
public:
    explicit MineralDB(const cma::domain::ThermoRepository& repository);
    std::vector<const cma::nucleo::Especie*> minerales();
private:
    const cma::domain::ThermoRepository& motor_;
};

}
