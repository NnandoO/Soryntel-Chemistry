#pragma once
#include <string>
#include <vector>
#include "domain/PourbaixTypes.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "motores/pourbaix.hpp"

namespace cma::interfaz {

class PanelPourbaix {
public:
    explicit PanelPourbaix(const domain::ThermoRepository& repository);
    void renderizar();

private:
    std::string elemento_actual_      = "Fe";
    double      temperatura_actual_   = 298.15;
    double      presion_actual_       = 1.0;
    double      concentracion_actual_ = 1e-6;
    int         escala_idx_           = 0; // 0=SHE, 1=SCE, 2=Ag/AgCl
    int         modo_color_           = 0; // 0=Por Especie, 1=Zonas de Corrosión (Inmunidad/Pasivación/Corrosión)
    bool        mostrar_agua_         = true;
    bool        mostrar_etiquetas_    = true;
    bool        mostrar_puntos_triples_ = true;
    bool        mostrar_mapa_color_   = true;

    // Ligandos secundarios activos (S, Cl, C, N)
    bool        lig_S_activo_  = false;
    double      lig_S_molal_   = 1e-2;
    bool        lig_Cl_activo_ = false;
    double      lig_Cl_molal_  = 1e-1;
    bool        lig_C_activo_  = false;
    double      lig_C_molal_   = 1e-3;
    bool        lig_N_activo_  = false;
    double      lig_N_molal_   = 1e-1;

    // Sonda operativa interactiva (pH, Eh)
    double      probe_pH_      = 7.0;
    double      probe_Eh_      = 0.0;
    char        csv_path_[256] = "hsc_pourbaix_eph.csv";
    std::string estado_msg_;

    cma::motores::CalculadorPourbaix calc_;
    bool calculado_ = false;

    void sincronizar_y_calcular();
};

} // namespace cma::interfaz
