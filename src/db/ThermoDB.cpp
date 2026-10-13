#include "ThermoDB.hpp"

namespace cma::application::modules {

ThermoDB::ThermoDB(const cma::domain::ThermoRepository& repository) : motor_(repository) {}

const cma::nucleo::Especie* ThermoDB::especiePorNombre(const std::string& nombre) {
    return motor_.especie_por_nombre(nombre);
}

const cma::nucleo::Especie* ThermoDB::especiePorId(int64_t id) {
    return motor_.especie_por_id(id);
}

}
