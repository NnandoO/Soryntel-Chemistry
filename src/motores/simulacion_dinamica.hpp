#pragma once
#include <vector>
#include <string>
#include <map>
#include <functional>

namespace cma { namespace motores {

struct ResultadoODEs {
    std::vector<double> tiempos;
    std::map<std::string, std::vector<double>> variables;
    std::string mensaje;
    bool ok = false;
};

class SimulacionDinamica {
public:
    // Resuelve sistema de ODEs con RK4
    ResultadoODEs resolver(
        double t_inicial, double t_final, double paso,
        const std::map<std::string, double>& condiciones_iniciales,
        std::function<void(double t, const std::map<std::string,double>& y,
                          std::map<std::string,double>& dydt)> f);

    // Tanque con vaciado gravitacional
    double tiempo_vaciado_tanque(double area_tanque, double area_orificio,
                                  double altura_inicial, double Cd = 0.62);

    // Calentamiento de un horno (modelo simplificado)
    std::vector<double> perfil_temperatura_horno(double T0, double T_ambiente,
                                                  double masa, double cp,
                                                  double hA, double t_max, double dt);

    // Reactor batch dinámico
    ResultadoODEs reactor_batch_dinamico(
        double T, double V, double k0, double Ea,
        const std::vector<double>& C0, int orden, double t_max, double dt);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;
};

}} // namespace
