#include "UnitConverter.hpp"
#include <map>
#include <cmath>
#include <algorithm>

namespace hsc_clone {

static double to_base_temperature(double v, const std::string& u) {
    if (u == "K") return v;
    if (u == "C") return v + 273.15;
    if (u == "F") return (v - 32.0) * 5.0 / 9.0 + 273.15;
    return v;
}

static double from_base_temperature(double v, const std::string& u) {
    if (u == "K") return v;
    if (u == "C") return v - 273.15;
    if (u == "F") return (v - 273.15) * 9.0 / 5.0 + 32.0;
    return v;
}

double UnitConverter::convertir(double valor, const std::string& de, const std::string& a) {
    if (de == a) return valor;

    // Temperature
    {
        static const char* temp_units[] = {"K", "C", "F"};
        bool is_temp = false;
        for (auto* tu : temp_units)
            if (de == tu || a == tu) is_temp = true;
        if (is_temp) {
            double base = to_base_temperature(valor, de);
            return from_base_temperature(base, a);
        }
    }

    // Pressure (base: Pa)
    static const std::map<std::string, double> pres_factors = {
        {"Pa", 1.0}, {"bar", 1e5}, {"atm", 101325.0},
        {"psi", 6894.757}, {"mmHg", 133.322}
    };
    {
        auto it_de = pres_factors.find(de);
        auto it_a  = pres_factors.find(a);
        if (it_de != pres_factors.end() && it_a != pres_factors.end())
            return valor * it_de->second / it_a->second;
    }

    // Energy (base: J)
    static const std::map<std::string, double> ener_factors = {
        {"J", 1.0}, {"cal", 4.184}, {"BTU", 1055.06}, {"kWh", 3.6e6}
    };
    {
        auto it_de = ener_factors.find(de);
        auto it_a  = ener_factors.find(a);
        if (it_de != ener_factors.end() && it_a != ener_factors.end())
            return valor * it_de->second / it_a->second;
    }

    // Mass (base: kg)
    static const std::map<std::string, double> mass_factors = {
        {"kg", 1.0}, {"g", 1e-3}, {"lb", 0.453592}, {"oz", 0.0283495}
    };
    {
        auto it_de = mass_factors.find(de);
        auto it_a  = mass_factors.find(a);
        if (it_de != mass_factors.end() && it_a != mass_factors.end())
            return valor * it_de->second / it_a->second;
    }

    // Volume (base: m3)
    static const std::map<std::string, double> vol_factors = {
        {"m3", 1.0}, {"L", 1e-3}, {"gal", 0.00378541}, {"ft3", 0.0283168}
    };
    {
        auto it_de = vol_factors.find(de);
        auto it_a  = vol_factors.find(a);
        if (it_de != vol_factors.end() && it_a != vol_factors.end())
            return valor * it_de->second / it_a->second;
    }

    return valor;
}

} // namespace hsc_clone
