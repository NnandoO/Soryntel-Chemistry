#pragma once
#include "base_datos/gestor_db.hpp"
#include "nucleo/tipos.hpp"
#include <vector>

namespace hsc_clone {

class MineralDB {
public:
    MineralDB();
    std::vector<const cma::nucleo::Especie*> minerales();
private:
    cma::base_datos::GestorDB& motor_;
};

}
