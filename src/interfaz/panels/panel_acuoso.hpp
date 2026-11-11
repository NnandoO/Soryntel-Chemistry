#pragma once
#include <cstdint>
#include <map>
#include <string>
#include <vector>
#include "../diagrams/PropertyDiagrams.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "motores/agua.hpp"
#include "motores/cinetica.hpp"
#include "motores/electroquimica.hpp"
#include "motores/exergia.hpp"
#include "motores/simulacion_dinamica.hpp"
#include "motores/soluciones_acuosas.hpp"

namespace cma { namespace interfaz {

// ── Panel de Electroquímica ─────────────────────────────────────────────────
class PanelElectroquimica {
public:
    void renderizar();

private:
    char anodo_[128] = "Zn2+ + 2e- -> Zn";
    char catodo_[128] = "Cu2+ + 2e- -> Cu";
    double T_ = 298.15;
    double E_celda_ = 0.0;
    double delta_G_ = 0.0;
    double K_eq_ = 0.0;
    bool calculado_ = false;
};

// ── Panel de Cinética ──────────────────────────────────────────────────────
class PanelCinetica {
public:
    void renderizar();

private:
    double C0_ = 1.0;
    double k_ = 0.1;
    double tiempo_ = 10.0;
    int orden_ = 1;
    int reactor_ = 0; // 0=batch, 1=CSTR, 2=PFR
    double Cb0_ = 0.0;
    double Cf_ = 0.0;
    double conversion_ = 0.0;
    bool calculado_ = false;
};

// ── Panel de Simulación Dinámica (HSC-Sim Dynamic Dialog) ───────────────────
struct EventoDinamicoTanques {
    double tiempo_s = 0.0;
    double feed_kg_h = 36000.0;
    double valvula1_pct = 100.0;
    double valvula2_pct = 100.0;
    double valvula3_pct = 100.0;
};

struct ConfigTanqueCascada {
    std::string nombre;
    double area_m2 = 2.0;
    double altura_max_m = 3.0;
    double area_orificio_m2 = 0.008;
    double nivel_inicial_pct = 50.0;
};

class PanelSimulacionDinamica {
public:
    PanelSimulacionDinamica();
    void renderizar();
private:
    double t_inicial_ = 0, t_final_ = 120, paso_ = 0.5;
    double A_tanque_ = 1.0, A_orificio_ = 0.01, h0_ = 2.0;
    double T0_horno_ = 300, T_amb_ = 298, masa_ = 100, cp_ = 500, hA_ = 50;
    double C0_ = 1.0, k0_ = 0.1, Ea_ = 50000, T_react_ = 500;
    int modo_ = 0; // 0=Cascada 3 Tanques (HSC-Sim), 1=Vaciado Torricelli, 2=Horno, 3=Reactor Batch
    bool calculado_ = false;
    double cursor_tiempo_ = 30.0;
    std::vector<ConfigTanqueCascada> tanques_;
    std::vector<EventoDinamicoTanques> agenda_;
    motores::ResultadoODEs resultado_;
    motores::SimulacionDinamica motor_;
    void ejecutar_cascada_tanques();
};

// ── Panel de Exergía ────────────────────────────────────────────────────
class PanelExergia {
public:
    void renderizar();
private:
    double h_ent_ = 100, h_sal_ = 80, h0_ = 50;
    double T_ent_ = 600, T_sal_ = 400, T0_ = 298;
    double s_ent_ = 0.5, s_sal_ = 0.4, s0_ = 0.3;
    double W_util_ = 10, Q_aporte_ = 50;
    motores::Exergia motor_;
    motores::ResultadoExergia resultado_;
    bool calculado_ = false;
};

// ── Panel de Diagramas Termoquímicos y Ellingham (HSC H, S, Cp, G & Ellingham) ──
class PanelDiagramasTermo {
public:
    struct EntradaEspecieDiagrama {
        std::string formula;
        double multiplicador = 1.0;
        bool activa = true;
    };

    struct EcuacionPersonalizada {
        std::string ecuacion;
        bool visible = true;
    };

    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void renderizar();

private:
    const domain::ThermoRepository* repo_ = nullptr;
    cma::application::modules::PropertyDiagrams motor_;

    domain::PropertyDiagramMode modo_ = domain::PropertyDiagramMode::EllinghamReactions;
    domain::PropertyDiagramType tipo_propiedad_ = domain::PropertyDiagramType::DeltaG_Ellingham;
    domain::EllinghamNormalizationBasis base_norm_ = domain::EllinghamNormalizationBasis::PerMoleO2;
    int familia_preset_ = 0; // 0=Oxidos, 1=Sulfuros, 2=Cloruros, 3=Carbonatos, 4=Sulfatos
    bool especie_usar_dHf_ = false;

    double T_min_ = 298.15;
    double T_max_ = 2273.15;
    int num_pasos_ = 140;
    bool usar_envolvente_fases_ = true;
    bool temp_en_celsius_ = false;
    bool energia_en_kcal_ = false;

    bool mostrar_iso_pO2_ = true;
    bool mostrar_iso_CO_CO2_ = false;
    bool mostrar_iso_H2_H2O_ = false;
    bool mostrar_marcadores_ = true;

    // Constructor de reacciones y visibilidad
    std::map<std::string, bool> visibilidad_curvas_;
    std::vector<std::string> curvas_eliminadas_;
    std::vector<EcuacionPersonalizada> ecuaciones_personalizadas_;
    char nueva_ecuacion_buf_[256] = "2 Ni + O2(g) = 2 NiO";
    std::string mensaje_estado_;

    // Selector de especies (modo SpeciesProperties)
    std::vector<EntradaEspecieDiagrama> especies_activas_;
    char nueva_especie_buf_[128] = "Al2O3";
    double nuevo_mult_especie_ = 1.0;
    bool show_select_species_modal_ = false;
    char search_buffer_[256] = "";

    // Resultado del motor y estado nomográfico
    domain::ResultadoDiagramaPropiedades resultado_;
    bool inicializado_ = false;
    bool calculado_ = false;

    char filtro_intersecciones_[128] = "";
    double nomo_T_K_ = 1273.15;
    double nomo_dG_kJ_ = -380.0;
    domain::EscalaNomograficaPunto nomo_resultado_;
    int curva_tabla_idx_ = 0;
    std::string csv_exportado_;
    char ruta_csv_buf_[256] = "diagrama_ellingham_hsc.csv";

    void cargar_familia_preset(domain::EllinghamPresetFamily familia);
    void recalcular();
    void actualizar_intersecciones();
};

// ── Panel de Agua (Tablas de Vapor) ─────────────────────────────────────
class PanelAgua {
public:
    void renderizar();
private:
    double T_ = 373.15, P_ = 0.101325;
    double T_min_sat_ = 298.15, T_max_sat_ = 647.0, T_step_sat_ = 10.0;
    motores::Agua motor_;
    motores::PropiedadesAgua resultado_;
    std::vector<motores::PropiedadesAgua> tabla_sat_;
    bool calculado_ = false;
    int modo_ = 0; // 0=propiedades, 1=saturación
};

// ── Panel de Soluciones Acuosas ─────────────────────────────────────────
class PanelSolucionesAcuosas {
public:
    void renderizar();
private:
    struct FilaAqua {
        char especie[128] = {};
        double temp = 40.0;
        double amount_kmol = 0.0;
        double amount_mol_pct = 0.0;
        double amount_g = 0.0;
        double h_ideal = 0.0;
        double h_estimate = 0.0;
        double cp_ideal = 0.0;
        double cp_estimate = 0.0;
        double ac_molar = 1.0;
        double ac_molal = 1.0;
        int carga = 0;
        bool calculo_ok = false;
    };

    std::vector<FilaAqua> filas_;
    bool usar_davies_ = true;
    bool usar_pitzer_ = false;
    bool usar_harvie_ = false;
    bool db_inicializada_ = false;

    motores::SolucionesAcuosas motor_;
    void calcular_hoja();
};

}} // namespace cma::interfaz
