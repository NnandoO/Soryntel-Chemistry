#pragma once
#include <vector>
#include <string>
#include <map>

namespace cma { namespace motores {

struct CapaAislante {
    std::string material;
    double espesor;  // m
    double k;        // W/(m·K) conductividad
};

struct ResultadoPerdidaCalor {
    double flujo_calor = 0;      // W
    double T_pared_int = 0;      // K
    double T_pared_ext = 0;      // K
    double resistencia_total = 0; // K/W
    std::vector<double> perfil_T;
    std::string mensaje;
    bool ok = false;
};

class PerdidaCalor {
public:
    // Pared plana multicapa
    ResultadoPerdidaCalor pared_plana(
        double T_interior, double T_exterior,
        double area, const std::vector<CapaAislante>& capas,
        double h_int = 10.0, double h_ext = 10.0);

    // Tubería cilíndrica con aislamiento
    ResultadoPerdidaCalor tuberia(
        double T_interior, double T_exterior,
        double radio_int, double largo,
        const std::vector<CapaAislante>& capas,
        double h_int = 50.0, double h_ext = 10.0);

    // Tanque esférico
    ResultadoPerdidaCalor tanque_esferico(
        double T_interior, double T_exterior,
        double radio_int,
        const std::vector<CapaAislante>& capas,
        double h_int = 10.0, double h_ext = 10.0);

    // Temperatura en pared compuesta (1D estacionario)
    std::vector<double> perfil_temperatura(
        double T_int, double T_ext,
        const std::vector<CapaAislante>& capas,
        double area, int puntos_por_capa = 5);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;
    double calcular_resistencia_conduccion(double k, double espesor, double area);
    double calcular_resistencia_conveccion(double h, double area);
};

}} // namespace
