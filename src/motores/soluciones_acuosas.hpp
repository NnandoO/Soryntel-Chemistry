#pragma once
#include <vector>
#include <string>
#include <map>

namespace cma { namespace motores {

class SolucionesAcuosas {
public:
    // Coeficiente osmótico (modelo de Pitzer simplificado)
    double coeficiente_osmótico(double fuerza_ionica, double T = 298.15);

    // Coeficientes de actividad (distintos modelos)
    double coeficiente_actividad_davies(double carga, double fuerza_ionica, double T = 298.15);
    double coeficiente_actividad_pitzer(double carga, double fuerza_ionica, double T = 298.15);
    double coeficiente_actividad_harvie(double carga, double fuerza_ionica, double T = 298.15);

    // Fuerza iónica
    double fuerza_ionica(const std::map<std::string, double>& concentraciones,
                          const std::map<std::string, int>& cargas);

    // pH de una solución
    double ph(double concentracion_H, double T = 298.15);

    // Densidad de solución electrolítica (aproximación)
    double densidad_solucion(double densidad_agua, const std::vector<double>& concentraciones,
                              const std::vector<double>& masas_molares);

    // Conductividad eléctrica de solución
    double conductividad_electrica(double T, const std::map<std::string, double>& iones,
                                    const std::map<std::string, double>& movilidades);

    std::string ultimo_error() const { return error_; }

private:
    std::string error_;
};

}} // namespace
