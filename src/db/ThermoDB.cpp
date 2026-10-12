#include "ThermoDB.hpp"

namespace hsc_clone {

ThermoDB::ThermoDB() : motor_(cma::base_datos::GestorDB::instancia()) {}

const cma::nucleo::Especie* ThermoDB::especiePorNombre(const std::string& nombre) {
    return motor_.especie_por_nombre(nombre);
}

const cma::nucleo::Especie* ThermoDB::especiePorId(int64_t id) {
    return motor_.especie_por_id(id);
}

}
