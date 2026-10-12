#include "motores/balance_energia.hpp"
#include <algorithm>
#include <cmath>

namespace cma { namespace motores {

double BalanceEnergia::capacidad_calorifica_flujo(const nucleo::Flujo& flujo,
                                                    const std::map<int64_t, nucleo::Especie>& db) const
{
    // cp() retorna J/(mol·K), n en kmol/s → resultado en J·kmol/(mol·K·s) = kJ/(K·s)
    double Cp_total = 0;
    for (auto& [sid, n] : flujo.flujo_molar) {
        auto it = db.find(sid);
        if (it != db.end())
            Cp_total += n * it->second.cp(flujo.temperatura); // kJ/(K·s)
    }
    return Cp_total;
}

double BalanceEnergia::entalpia_flujo(const nucleo::Flujo& flujo,
                                       const std::map<int64_t, nucleo::Especie>& db)
{
    double H = 0;
    for (auto& [sid, n] : flujo.flujo_molar) {
        auto it = db.find(sid);
        if (it != db.end())
            H += n * it->second.entalpia_abs(flujo.temperatura);
        else {
            // Especie desconocida: usar Cp genérico neutro (gas diatómico ~29.1 J/molK)
            // Objeto local (no static) para thread-safety
            nucleo::Especie desconocida;
            desconocida.h_formacion = 0.0;
            desconocida.cp_a = 29.1;
            H += n * desconocida.entalpia_abs(flujo.temperatura);
        }
    }
    return H;
}

double BalanceEnergia::calor_reaccion(const nucleo::Reaccion& rxn, double T_K,
                                       const std::map<int64_t, nucleo::Especie>& db)
{
    double dH = 0;
    for (auto& [sid, coeff] : rxn.estequiometria) {
        auto it = db.find(sid);
        if (it != db.end())
            dH += coeff * it->second.entalpia_ref(T_K);
    }
    return rxn.delta_h_298 + dH;
}

ResultadoBalanceEnergia BalanceEnergia::resolver_nodo(
    const nucleo::Nodo& nodo,
    const std::vector<nucleo::Flujo>& entradas,
    const std::vector<nucleo::Flujo>& salidas,
    const std::map<int64_t, nucleo::Especie>& db)
{
    ResultadoBalanceEnergia res;

    double H_in = 0, H_out = 0;
    for (auto& e : entradas) H_in  += entalpia_flujo(e, db);
    for (auto& s : salidas)  H_out += entalpia_flujo(s, db);

    res.entalpia_entrada = H_in;
    res.entalpia_salida  = H_out;
    res.calor = H_out - H_in;

    if (nodo.params.count("T_salida"))
        res.delta_T = nodo.params.at("T_salida") - (entradas.empty() ? nucleo::STP_TEMP : entradas[0].temperatura);

    res.balanceado = std::abs(res.calor) < 1.0 || 
                     (std::abs(H_in) > 0.0 && std::abs(res.calor / H_in) < 1e-4) ||
                     nodo.tipo == nucleo::TipoNodo::IntercambiadorCalor ||
                     nodo.tipo == nucleo::TipoNodo::Fuente ||
                     nodo.tipo == nucleo::TipoNodo::Sumidero;
    res.mensaje = res.balanceado
        ? "Balance de energía satisfecho."
        : "Desequilibrio energético: Q = " + std::to_string(res.calor) + " kJ/s";
    return res;
}

double BalanceEnergia::temperatura_adiabatica(
    const nucleo::Flujo& entrada,
    const nucleo::Reaccion& rxn,
    double conversion,
    const std::map<int64_t, nucleo::Especie>& db,
    double T_inicial)
{
    // Reactivo limitante
    double ext_max = 1e30;
    for (auto& [sid, coeff] : rxn.estequiometria) {
        if (coeff < 0) {
            double disp = entrada.flujo_especie(sid);
            double ext = disp / (-coeff);
            if (ext < ext_max) ext_max = ext;
        }
    }
    if (ext_max > 1e29) ext_max = 0; // ningún reactivo presente
    double extension = ext_max * conversion;

    // Calor de reacción a temperatura de entrada
    double H_rxn = calor_reaccion(rxn, entrada.temperatura, db);
    double Q_rxn = -extension * H_rxn;

    // Productos a Temp de entrada
    nucleo::Flujo productos = entrada;
    for (auto& [sid, coeff] : rxn.estequiometria)
        productos.flujo_molar[sid] += coeff * extension;

    double H_prod_Tin = 0;
    for (auto& [sid, n] : productos.flujo_molar) {
        auto it = db.find(sid);
        if (it != db.end())
            H_prod_Tin += n * it->second.entalpia_abs(entrada.temperatura);
    }

    // Newton-Raphson
    double T = T_inicial;
    for (int iter = 0; iter < 200; ++iter) {
        double H_T = 0, Cp_T = 0;
        for (auto& [sid, n] : productos.flujo_molar) {
            auto it = db.find(sid);
            if (it != db.end()) {
                H_T  += n * it->second.entalpia_abs(T);
                Cp_T += n * it->second.cp(T);
            }
        }

        double objetivo = H_prod_Tin + Q_rxn;
        double f = H_T - objetivo;
        if (std::abs(f) < 1e-8) break;

        double df = Cp_T / 1000.0;
        if (std::abs(df) < 1e-15) break;
        T -= f / df;
        T = std::clamp(T, 100.0, 5000.0);
    }

    return T;
}

}} // namespace
