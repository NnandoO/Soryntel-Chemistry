#include "simulacion_dinamica.hpp"
#include <cmath>
#include <algorithm>
#include <sstream>

namespace cma { namespace motores {

ResultadoODEs SimulacionDinamica::resolver(
    double t_inicial, double t_final, double paso,
    const std::map<std::string, double>& condiciones_iniciales,
    std::function<void(double t, const std::map<std::string,double>& y,
                      std::map<std::string,double>& dydt)> f)
{
    ResultadoODEs res;
    if (condiciones_iniciales.empty()) {
        error_ = "Sin condiciones iniciales";
        return res;
    }
    if (t_final <= t_inicial || paso <= 0) {
        error_ = "t_final debe ser > t_inicial y paso > 0";
        return res;
    }

    std::vector<std::string> vars;
    for (auto& [k,_] : condiciones_iniciales) vars.push_back(k);

    std::map<std::string, double> y = condiciones_iniciales;
    double t = t_inicial;
    int n = vars.size();

    auto guardar = [&]() {
        res.tiempos.push_back(t);
        for (auto& v : vars)
            res.variables[v].push_back(y[v]);
    };

    guardar();

    int pasos = (int)((t_final - t_inicial) / paso);
    for (int i = 0; i < pasos; ++i) {
        // RK4
        std::map<std::string, double> k1, k2, k3, k4, yt;

        auto f_eval = [&](const std::map<std::string,double>& yy,
                          std::map<std::string,double>& kk) {
            std::map<std::string,double> dydt;
            f(t, yy, dydt);
            for (auto& v : vars) kk[v] = dydt[v] * paso;
        };

        f_eval(y, k1);

        yt = y;
        for (auto& v : vars) yt[v] = y[v] + 0.5 * k1[v];
        f_eval(yt, k2);

        yt = y;
        for (auto& v : vars) yt[v] = y[v] + 0.5 * k2[v];
        f_eval(yt, k3);

        yt = y;
        for (auto& v : vars) yt[v] = y[v] + k3[v];
        f_eval(yt, k4);

        for (auto& v : vars)
            y[v] = y[v] + (k1[v] + 2*k2[v] + 2*k3[v] + k4[v]) / 6.0;

        t += paso;
        guardar();
    }

    res.ok = true;
    res.mensaje = "Simulación completada: " + std::to_string(pasos) + " pasos, "
                  + std::to_string(vars.size()) + " variables";
    return res;
}

double SimulacionDinamica::tiempo_vaciado_tanque(
    double A_tanque, double A_orificio, double h0, double Cd)
{
    // t = (A_tanque / (Cd * A_orificio)) * sqrt(2*h0/g)
    double g = 9.81;
    if (A_orificio <= 0 || h0 <= 0 || Cd <= 0) return 0;
    return (A_tanque / (Cd * A_orificio)) * std::sqrt(2 * h0 / g);
}

std::vector<double> SimulacionDinamica::perfil_temperatura_horno(
    double T0, double T_amb, double masa, double cp, double hA,
    double t_max, double dt)
{
    std::vector<double> perfil;
    double T = T0;
    double denom = masa * cp;
    if (std::abs(denom) < 1e-15) { perfil.push_back(T); return perfil; }
    for (double t = 0; t <= t_max; t += dt) {
        perfil.push_back(T);
        double dTdt = -hA * (T - T_amb) / denom;
        T += dTdt * dt;
    }
    return perfil;
}

ResultadoODEs SimulacionDinamica::reactor_batch_dinamico(
    double T, double V, double k0, double Ea,
    const std::vector<double>& C0, int orden, double t_max, double dt)
{
    // Arrhenius: k = k0 * exp(-Ea/(R*T)) — Ea en kJ/mol, R en J/(mol·K)
    double R = 8.314462618;
    double k = k0 * std::exp(-Ea * 1000.0 / (R * T));

    std::map<std::string, double> ci;
    for (size_t i = 0; i < C0.size(); ++i)
        ci["C[" + std::to_string(i) + "]"] = C0[i];

    auto f = [&](double t, const std::map<std::string,double>& y,
                 std::map<std::string,double>& dydt) {
        double c0 = y.at("C[0]");
        double rate = k * std::pow(std::max(c0, 0.0), orden);
        dydt["C[0]"] = -rate;
        // Productos: estequiometría A → P (1:1)
        if (C0.size() > 1)
            dydt["C[1]"] = rate;
        for (size_t i = 2; i < C0.size(); ++i)
            dydt["C[" + std::to_string(i) + "]"] = 0;
    };

    return resolver(0, t_max, dt, ci, f);
}

}} // namespace
