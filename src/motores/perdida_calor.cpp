#include "perdida_calor.hpp"
#include <cmath>
#include <algorithm>
#include <sstream>

namespace cma { namespace motores {

double PerdidaCalor::calcular_resistencia_conduccion(double k, double espesor, double area) {
    if (k <= 0 || area <= 0) return 1e12;
    return espesor / (k * area);
}

double PerdidaCalor::calcular_resistencia_conveccion(double h, double area) {
    if (h <= 0 || area <= 0) return 1e12;
    return 1.0 / (h * area);
}

ResultadoPerdidaCalor PerdidaCalor::pared_plana(
    double T_int, double T_ext, double area,
    const std::vector<CapaAislante>& capas,
    double h_int, double h_ext)
{
    ResultadoPerdidaCalor res;
    double R_total = 0;

    // Resistencia convectiva interior
    R_total += calcular_resistencia_conveccion(h_int, area);

    // Resistencias conductivas por capa
    for (auto& c : capas)
        R_total += calcular_resistencia_conduccion(c.k, c.espesor, area);

    // Resistencia convectiva exterior
    R_total += calcular_resistencia_conveccion(h_ext, area);

    if (R_total > 1e10) {
        error_ = "Resistencia infinita";
        return res;
    }

    res.resistencia_total = R_total;
    res.flujo_calor = (T_int - T_ext) / R_total;
    
    // División segura
    double h_int_safe = std::max(h_int, 1e-6);
    double h_ext_safe = std::max(h_ext, 1e-6);
    res.T_pared_int = T_int - res.flujo_calor / (h_int_safe * area);
    res.T_pared_ext = T_ext + res.flujo_calor / (h_ext_safe * area);

    // Radiación exterior (asumiendo emisividad 0.9)
    double emisividad = 0.9;
    double sigma = 5.670374419e-8;
    double Q_rad = emisividad * sigma * area * (std::pow(res.T_pared_ext, 4) - std::pow(T_ext, 4));
    res.flujo_calor += Q_rad;
    res.perfil_T = perfil_temperatura(T_int, T_ext, capas, area);
    res.ok = true;
    res.mensaje = "Flujo de calor: " + std::to_string(res.flujo_calor) + " W";

    return res;
}

ResultadoPerdidaCalor PerdidaCalor::tuberia(
    double T_int, double T_ext, double r_int, double largo,
    const std::vector<CapaAislante>& capas,
    double h_int, double h_ext)
{
    ResultadoPerdidaCalor res;
    double R_total = 0;

    // Convección interior
    double A_int = 2 * M_PI * r_int * largo;
    R_total += calcular_resistencia_conveccion(h_int, A_int);

    // Conducción radial por capa
    double r = r_int;
    for (auto& c : capas) {
        double r_ext = r + c.espesor;
        if (r > 0 && c.k > 0)
            R_total += std::log(r_ext / r) / (2 * M_PI * c.k * largo);
        r = r_ext;
    }

    // Convección exterior
    double A_ext = 2 * M_PI * r * largo;
    R_total += calcular_resistencia_conveccion(h_ext, A_ext);

    if (R_total > 1e10 || std::abs(R_total) < 1e-15) {
        error_ = "Resistencia infinita o nula";
        return res;
    }

    res.resistencia_total = R_total;
    res.flujo_calor = (T_int - T_ext) / R_total;
    res.ok = true;
    res.mensaje = "Flujo de calor en tubería: " + std::to_string(res.flujo_calor) + " W";

    return res;
}

ResultadoPerdidaCalor PerdidaCalor::tanque_esferico(
    double T_int, double T_ext, double r_int,
    const std::vector<CapaAislante>& capas,
    double h_int, double h_ext)
{
    ResultadoPerdidaCalor res;
    double R_total = 0;

    double A_int = 4 * M_PI * r_int * r_int;
    R_total += calcular_resistencia_conveccion(h_int, A_int);

    double r = r_int;
    for (auto& c : capas) {
        double r_ext = r + c.espesor;
        if (r > 0 && c.k > 0)
            R_total += (1.0 / r - 1.0 / r_ext) / (4 * M_PI * c.k);
        r = r_ext;
    }

    double A_ext = 4 * M_PI * r * r;
    R_total += calcular_resistencia_conveccion(h_ext, A_ext);

    if (R_total > 1e10 || std::abs(R_total) < 1e-15) {
        error_ = "Resistencia infinita o nula";
        return res;
    }

    res.flujo_calor = (T_int - T_ext) / R_total;
    res.ok = true;
    res.mensaje = "Flujo de calor en tanque: " + std::to_string(res.flujo_calor) + " W";
    return res;
}

std::vector<double> PerdidaCalor::perfil_temperatura(
    double T_int, double T_ext,
    const std::vector<CapaAislante>& capas,
    double area, int puntos_por_capa)
{
    std::vector<double> perfil = { T_int };
    double R_total = 0;
    double R_acum = 0;

    for (auto& c : capas)
        R_total += calcular_resistencia_conduccion(c.k, c.espesor, area);

    if (std::abs(R_total) < 1e-15) return perfil;

    double Q = (T_int - T_ext) / R_total;

    for (auto& c : capas) {
        double R_capa = calcular_resistencia_conduccion(c.k, c.espesor, area);
        for (int i = 1; i <= puntos_por_capa; ++i) {
            double f = (double)i / puntos_por_capa;
            double T = T_int - Q * (R_acum + f * R_capa);
            perfil.push_back(T);
        }
        R_acum += R_capa;
    }

    return perfil;
}

}} // namespace
