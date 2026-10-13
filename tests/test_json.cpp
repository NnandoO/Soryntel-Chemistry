#include "diagrama_flujo/flowsheet.hpp"
#include <iostream>
int main() {
    cma::diagrama_flujo::Flowsheet fs;
    fs.agregar_nodo(cma::nucleo::TipoNodo::Reactor, "R1");
    fs.agregar_flujo("F1");
    std::string json = fs.a_json();
    std::string err;
    cma::diagrama_flujo::Flowsheet fs2;
    if (fs2.desde_json(json, err)) {
        std::cout << "SUCCESS" << std::endl;
    } else {
        std::cout << "ERROR: " << err << std::endl;
        return 1;
    }
    return 0;
}
