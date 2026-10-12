#pragma once
#include "base_datos/gestor_db.hpp"
#include "nucleo/tipos.hpp"
#include <vector>

namespace hsc_clone {

class MaterialDB {
public:
    MaterialDB();
    std::vector<const cma::nucleo::Especie*> materiales();
private:
    cma::base_datos::GestorDB& motor_;
};

}
