#pragma once
#include "nucleo/tipos.hpp"
#include "diagrama_flujo/flowsheet.hpp"
#include <vector>
#include <map>

namespace cma { namespace motores {

struct ResultadoBalanceEnergia {
    double entalpia_entrada = 0.0; // kJ/s
    double entalpia_salida  = 0.0; // kJ/s
    double calor            = 0.0; // kJ/s
    double trabajo          = 0.0; // kJ/s
    double delta_T          = 0.0; // K
    bool   balanceado       = false;
    std::string mensaje;
};

class BalanceEnergia {
public:
    BalanceEnergia() = default;

    double entalpia_flujo(const nucleo::Flujo& flujo, const std::map<int64_t, nucleo::Especie>& db);

    ResultadoBalanceEnergia resolver_nodo(
        const nucleo::Nodo& nodo,
        const std::vector<nucleo::Flujo>& entradas,
        const std::vector<nucleo::Flujo>& salidas,
        const std::map<int64_t, nucleo::Especie>& db
    );

    double calor_reaccion(const nucleo::Reaccion& rxn, double T_K,
                          const std::map<int64_t, nucleo::Especie>& db);

    double temperatura_adiabatica(
        const nucleo::Flujo& entrada,
        const nucleo::Reaccion& rxn,
        double conversion,
        const std::map<int64_t, nucleo::Especie>& db,
        double T_inicial = 1000.0
    );

    double capacidad_calorifica_flujo(const nucleo::Flujo& flujo,
                                      const std::map<int64_t, nucleo::Especie>& db) const;

private:
    static constexpr int MAX_ITER = 200;
    static constexpr double TOL = 1e-10;
};

}} // namespace
