#include "agua.hpp"
#define REGION3_ITERATE
#include <IF97.h>
#include <cmath>
#include <limits>
#include <stdexcept>
#include <algorithm>

namespace cma::motores {
namespace {
constexpr double critical_temperature = 647.096;
constexpr double critical_pressure = 22.064;
PropiedadesAgua saturated(double temperature, double pressure, bool vapor) {
    const double pascal = pressure * 1e6;
    PropiedadesAgua state;
    state.T = temperature; state.P = pressure;
    state.h = (vapor ? IF97::hvap_p(pascal) : IF97::hliq_p(pascal)) / 1000;
    state.s = (vapor ? IF97::svap_p(pascal) : IF97::sliq_p(pascal)) / 1000;
    state.v = 1 / (vapor ? IF97::rhovap_p(pascal) : IF97::rholiq_p(pascal));
    state.cp = (vapor ? IF97::cpvap_p(pascal) : IF97::cpliq_p(pascal)) / 1000;
    state.cv = (vapor ? IF97::cvvap_p(pascal) : IF97::cvliq_p(pascal)) / 1000;
    state.mu = vapor ? IF97::viscvap_p(pascal) : IF97::viscliq_p(pascal);
    state.k = vapor ? IF97::tcondvap_p(pascal) : IF97::tcondliq_p(pascal);
    state.fase = vapor ? "vapor saturado" : "líquido saturado";
    state.ok = true;
    return state;
}
}
PropiedadesAgua Agua::propiedades(double temperature, double pressure) {
    PropiedadesAgua state;
    state.T = temperature; state.P = pressure;
    error_.clear();
    try {
        if (!std::isfinite(temperature) || !std::isfinite(pressure) || pressure <= 0)
            throw std::invalid_argument("Temperatura/presión no finita o no positiva");
        if (temperature < critical_temperature && temperature >= 273.15) {
            const double saturation = IF97::psat97(temperature) / 1e6;
            if (std::abs(pressure - saturation) <= 1e-9 * std::max(1.0,saturation))
                throw std::invalid_argument("Sobre saturación: especifique calidad de vapor o elija líquido/vapor saturado");
        }
        const double pascal = pressure * 1e6;
        state.h = IF97::hmass_Tp(temperature,pascal) / 1000;
        state.s = IF97::smass_Tp(temperature,pascal) / 1000;
        state.v = 1 / IF97::rhomass_Tp(temperature,pascal);
        state.cp = IF97::cpmass_Tp(temperature,pascal) / 1000;
        state.cv = IF97::cvmass_Tp(temperature,pascal) / 1000;
        state.mu = IF97::visc_Tp(temperature,pascal);
        state.k = IF97::tcond_Tp(temperature,pascal);
        if (temperature >= critical_temperature)
            state.fase = pressure >= critical_pressure ? "fluido supercrítico" : "vapor";
        else state.fase = pressure > IF97::psat97(temperature)/1e6 ? "líquido comprimido" : "vapor sobrecalentado";
        state.ok = true;
    } catch (const std::exception& e) {
        error_ = state.mensaje = std::string("Fuera de cálculo IF97: ") + e.what();
        state.fase = state.mensaje;
    }
    return state;
}
std::pair<PropiedadesAgua,PropiedadesAgua> Agua::saturacion_T(double temperature) {
    error_.clear();
    try {
        if (!std::isfinite(temperature) || temperature < 273.15 || temperature >= critical_temperature)
            throw std::invalid_argument("Saturación requiere 273.15 <= T < 647.096 K");
        const double pressure = IF97::psat97(temperature) / 1e6;
        return {saturated(temperature,pressure,false),saturated(temperature,pressure,true)};
    } catch (const std::exception& e) {
        PropiedadesAgua invalid;
        error_ = invalid.fase = invalid.mensaje = e.what();
        return {invalid,invalid};
    }
}
PropiedadesAgua Agua::mezcla_saturada(double pressure, double quality) {
    error_.clear();
    try {
        if (!std::isfinite(pressure) || !std::isfinite(quality) || quality < 0 || quality > 1)
            throw std::invalid_argument("Calidad de vapor requiere 0 <= x <= 1");
        const double temperature = IF97::Tsat97(pressure * 1e6);
        const auto liquid = saturated(temperature,pressure,false);
        const auto vapor = saturated(temperature,pressure,true);
        if (quality == 0) return liquid;
        if (quality == 1) return vapor;
        PropiedadesAgua state;
        state.T = temperature; state.P = pressure;
        state.h = liquid.h + quality * (vapor.h - liquid.h);
        state.s = liquid.s + quality * (vapor.s - liquid.s);
        state.v = liquid.v + quality * (vapor.v - liquid.v);
        // Cp, Cv and transport properties of a two-phase mixture require an
        // additional physical model. Do not interpolate them as if single-phase.
        state.cp = state.cv = state.mu = state.k = std::numeric_limits<double>::quiet_NaN();
        state.fase = "mezcla saturada";
        state.mensaje = "Cp/Cv/transporte no definidos para la mezcla bifásica";
        state.ok = true;
        return state;
    } catch (const std::exception& e) {
        PropiedadesAgua invalid; error_ = invalid.fase = invalid.mensaje = e.what(); return invalid;
    }
}
double Agua::temperatura_saturacion(double pressure) {
    error_.clear();
    try {
        if (!std::isfinite(pressure)) throw std::invalid_argument("Presión no finita");
        return IF97::Tsat97(pressure * 1e6);
    } catch (const std::exception& e) { error_ = e.what(); return std::numeric_limits<double>::quiet_NaN(); }
}
double Agua::presion_saturacion(double temperature) {
    error_.clear();
    try {
        if (!std::isfinite(temperature)) throw std::invalid_argument("Temperatura no finita");
        return IF97::psat97(temperature) / 1e6;
    } catch (const std::exception& e) { error_ = e.what(); return std::numeric_limits<double>::quiet_NaN(); }
}
double Agua::calor_latente(double temperature) {
    const auto states = saturacion_T(temperature);
    return states.first.ok && states.second.ok ? states.second.h - states.first.h : std::numeric_limits<double>::quiet_NaN();
}
double Agua::viscosidad_liquida(double temperature) {
    const auto states = saturacion_T(temperature);
    return states.first.ok ? states.first.mu : std::numeric_limits<double>::quiet_NaN();
}
double Agua::conductividad_termica(double temperature,double pressure) {
    const auto state = propiedades(temperature,pressure);
    return state.ok ? state.k : std::numeric_limits<double>::quiet_NaN();
}
}
