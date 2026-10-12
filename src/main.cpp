#include "interfaz/aplicacion.hpp"
#include <cstdio>
#include <cstdlib>
#include <ctime>

int main(int argc, char* argv[]) {
    cma::interfaz::Aplicacion app;

    if (!app.iniciar(argc, argv)) {
        fprintf(stderr, "Error: no se pudo inicializar CMA Chemistry.\n");
        return 1;
    }

    app.ejecutar();
    return 0;
}
