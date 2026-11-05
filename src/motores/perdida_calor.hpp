#pragma once
#include "domain/HeatLossTypes.hpp"
#include "domain/ports/ThermalPropertyCatalog.hpp"

#include <map>
#include <string>
#include <vector>

namespace cma {
namespace motores {

struct CapaAislante {
    std::string material;
    double espesor; // m
    double k;       // W/(m·K) conductividad
};

struct ResultadoPerdidaCalor {
    double flujo_calor = 0;       // W
    double T_pared_int = 0;       // K
    double T_pared_ext = 0;       // K
    double resistencia_total = 0; // K/W
    std::vector<double> perfil_T;
    std::string mensaje;
    bool ok = false;
};

class PerdidaCalor {
  public:
    // ── Métodos clásicos de transferencia 1D (compatibilidad) ───────────────
    ResultadoPerdidaCalor pared_plana(double T_interior, double T_exterior, double area,
                                      const std::vector<CapaAislante>& capas, double h_int = 10.0,
                                      double h_ext = 10.0);

    ResultadoPerdidaCalor tuberia(double T_interior, double T_exterior, double radio_int, double largo,
                                  const std::vector<CapaAislante>& capas, double h_int = 50.0,
                                  double h_ext = 10.0);

    ResultadoPerdidaCalor tanque_esferico(double T_interior, double T_exterior, double radio_int,
                                          const std::vector<CapaAislante>& capas, double h_int = 10.0,
                                          double h_ext = 10.0);

    std::vector<double> perfil_temperatura(double T_int, double T_ext, const std::vector<CapaAislante>& capas,
                                           double area, int puntos_por_capa = 5);

    // ── Motor multicapa no lineal estilo HSC Heat Loss Module ───────────────
    bool resolver_hoja(domain::ThermalSheet& sheet, const domain::ThermalPropertyCatalog* catalog = nullptr);

    bool resolver_flujo_calor_dos_puntos(domain::ThermalSheet& sheet,
                                         const domain::ThermalPropertyCatalog* catalog = nullptr);

    bool resolver_perfil_temperatura_q_fijo(domain::ThermalSheet& sheet,
                                            const domain::ThermalPropertyCatalog* catalog = nullptr);

    domain::TargetThicknessResult
    optimizar_espesor_capa(domain::ThermalSheet& sheet, const domain::TargetThicknessRequest& req,
                           const domain::ThermalPropertyCatalog* catalog = nullptr);

    static double calcular_conveccion_superficie(const domain::ConvectionFluidEntry* fluid, double t_surf_c,
                                                 double t_fluid_c, double char_length_m, double velocity_m_s,
                                                 domain::ConvectionOrientation orient);

    static double calcular_radiacion_superficie(double emissivity, double t_surf_c, double t_surr_c);

    static domain::GasParticleRadiationResult
    calcular_radiacion_gas_particulas(const domain::GasParticleRadiationInput& in);

    static void generar_perfil_detallado(domain::ThermalSheet& sheet,
                                         const domain::ThermalPropertyCatalog* catalog = nullptr);

    std::string ultimo_error() const { return error_; }

  private:
    std::string error_;
    double calcular_resistencia_conduccion(double k, double espesor, double area);
    double calcular_resistencia_conveccion(double h, double area);

    static void actualizar_propiedades_y_resistencias_hoja(domain::ThermalSheet& sheet,
                                                           const domain::ThermalPropertyCatalog* catalog);
};

} // namespace motores
} // namespace cma
