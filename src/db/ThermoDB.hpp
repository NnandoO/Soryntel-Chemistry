#pragma once
#include "base_datos/gestor_db.hpp"
#include "nucleo/tipos.hpp"
#include <string>

namespace hsc_clone {

class ThermoDB {
public:
    ThermoDB();
    const cma::nucleo::Especie* especiePorNombre(const std::string& nombre);
    const cma::nucleo::Especie* especiePorId(int64_t id);
private:
    cma::base_datos::GestorDB& motor_;
};

}
