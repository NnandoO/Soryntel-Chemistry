#include "interfaz/aplicacion.hpp"
#include <cstdio>
#include <cstdlib>
#include <ctime>
#include "adapters/desktop/process.hpp"
#include "adapters/persistence/MinteqImporter.hpp"
#include "adapters/scientific/PhreeqcEngine.hpp"
#include <filesystem>

int main(int argc, char* argv[]) {
    auto& repository = cma::base_datos::GestorDB::instancia();
    cma::adapters::DesktopFileDialog file_dialog;
    cma::domain::AqueousDatabase aqueous_database;
    cma::adapters::MinteqImporter importer;
    const auto component_file=cma::adapters::resource_path("data/comp_2008.vdb");
    if (component_file.empty() || !importer.load(std::filesystem::path(component_file).parent_path().string(),aqueous_database))
        fprintf(stderr,"Catálogo acuoso nativo no disponible: %s\n",importer.error().c_str());
    const auto phreeqc_file=cma::adapters::resource_path("data/reference/phreeqc/phreeqc.dat");
    cma::adapters::PhreeqcEngine aqueous_engine(std::filesystem::path(phreeqc_file).parent_path().string());
    cma::interfaz::Aplicacion app(repository, file_dialog,aqueous_database,aqueous_engine);

    if (!app.iniciar(argc, argv)) {
        fprintf(stderr, "Error: no se pudo inicializar CMA Chemistry.\n");
        return 1;
    }

    app.ejecutar();
    return 0;
}
