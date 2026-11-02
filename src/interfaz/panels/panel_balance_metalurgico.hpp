#pragma once

#include "diagrama_flujo/flowsheet.hpp"
#include "motores/balance_metalurgico.hpp"
#include <string>
#include <vector>

namespace cma {
namespace interfaz {

class PanelReconciliacion {
public:
    void renderizar(diagrama_flujo::Flowsheet* flowsheet = nullptr);
    void reset();

private:
    motores::ConfigBalanceMetalurgico config_;
    motores::ResultadoBalanceMetalurgico resultado_;
    std::vector<std::string> elementos_;
    diagrama_flujo::Flowsheet* fs_ = nullptr;
    int modo_ = 0;
    int fila_seleccionada_ = -1;
    bool calculado_ = false;
    bool cargado_inicial_ = false;
    std::string mensaje_;

    void inicializar_datos(diagrama_flujo::Flowsheet* flowsheet);
    void ejecutar_balance();
    void guardar_configuracion(bool usar_reconciliados);
    void importar_csv();
    void exportar_csv();
    void dibujar_barra_herramientas();
    void dibujar_modo();
    void dibujar_corrientes();
    void dibujar_unidades();
    void dibujar_ensayos();
    void dibujar_solver();
    void dibujar_reporte();
    void dibujar_ayuda();
};

} // namespace interfaz
} // namespace cma
