#pragma once
#include "nucleo/tipos.hpp"
#include "motores/equilibrio_fases.hpp"
#include <vector>
#include <map>

namespace cma { namespace motores {

struct ResultadoDestilacion {
    double destilado = 0.0;      // kmol/s
    double residuo   = 0.0;      // kmol/s
    double R_min     = 0.0;      // relación de reflujo mínima
    double R_operacion = 0.0;    // relación de reflujo de operación
    int    N_min     = 0;         // etapas mínimas (Fenske)
    int    N_etapas  = 0;         // etapas reales
    std::map<int64_t, double> x_destilado;
    std::map<int64_t, double> x_residuo;
    std::vector<double> perfil_temperatura; // T en cada etapa
    std::string mensaje;
};

class Destilacion {
public:
    Destilacion() = default;

    // Fenske: número mínimo de etapas (separación binaria)
    double fenske(double xD_volatil, double xB_volatil, double alfa_promedio) const;

    // Underwood: reflujo mínimo
    double underwood_min_reflujo(double alfa, double xF, double xD, double q = 1.0) const;

    // Gilliland: etapas reales desde R_min y N_min
    int gilliland(int N_min, double R_min, double R_op) const;

    // McCabe-Thiele simplificado para destilación binaria
    ResultadoDestilacion mccabe_thiele(
        double zF,              // fracción molar del volátil en alimentación
        double xD,              // fracción molar en destilado
        double xB,              // fracción molar en residuo
        double alfa,            // volatilidad relativa
        double q,               // calidad de alimentación
        double R_op             // relación de reflujo de operación
    ) const;

    // Destilación multicomponente (método shortcut Fenske-Underwood-Gilliland)
    ResultadoDestilacion fenske_underwood_gilliland(
        const nucleo::Flujo& alimentacion,
        const std::vector<int64_t>& clave_liviana,
        const std::vector<int64_t>& clave_pesada,
        double recuperacion_liviano_dest,
        double recuperacion_pesado_res,
        double R_op,
        const std::map<int64_t, nucleo::Especie>& db
    ) const;

private:
    // Volatilidad relativa entre dos especies
    double volatilidad_relativa(double K1, double K2) const;
};

}} // namespace
