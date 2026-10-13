#pragma once
#include "application/modules.hpp"
#include "motores/agua.hpp"
#include <string>

namespace cma::application::modules {

struct PropiedadesAgua {
    double T;
    double P;
    double h;
    double s;
    double v;
    double cp;
    double cv;
    std::string fase;
};

class SteamTables {
public:
    PropiedadesAgua propiedades(double T, double P);
private:
    cma::motores::Agua motor_;
};

}
