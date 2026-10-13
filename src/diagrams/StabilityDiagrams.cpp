#include "StabilityDiagrams.hpp"
#include <cmath>
#include <algorithm>
#include <functional>

namespace cma::application::modules {

namespace {

struct FaseCondensada {
    std::string nombre;
    double nu_metal = 1.0;
    double nu_O2 = 0.0;
    double nu_SO2 = 0.0;
    std::function<double(double)> delta_G_rxn;
};

std::vector<FaseCondensada> fases_para_metal(const std::string& metal) {
    std::string m = metal;
    for (char& c : m) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));

    if (m == "cu" || m == "cobre" || m == "copper") {
        return {
            {"Cu", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
            {"Cu2O", 2.0, 0.5, 0.0, [](double T) { return -169.5 + 0.0712 * T; }},
            {"CuO", 1.0, 0.5, 0.0, [](double T) { return -155.2 + 0.0874 * T; }},
            {"Cu2S", 2.0, -1.0, 1.0, [](double T) { return 230.8 - 0.0435 * T; }},
            {"CuSO4", 1.0, 1.0, 1.0, [](double T) { return -771.4 + 0.2862 * T; }},
            {"CuO*CuSO4", 2.0, 1.5, 1.0, [](double T) { return -935.5 + 0.3805 * T; }}
        };
    }
    if (m == "fe" || m == "hierro" || m == "iron") {
        return {
            {"Fe", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
            {"FeO", 1.0, 0.5, 0.0, [](double T) { return -263.7 + 0.0644 * T; }},
            {"Fe3O4", 3.0, 2.0, 0.0, [](double T) { return -1102.5 + 0.3075 * T; }},
            {"Fe2O3", 2.0, 1.5, 0.0, [](double T) { return -813.0 + 0.2470 * T; }},
            {"FeS", 1.0, -1.0, 1.0, [](double T) { return 212.0 - 0.0402 * T; }},
            {"FeSO4", 1.0, 1.0, 1.0, [](double T) { return -928.0 + 0.3450 * T; }}
        };
    }
    if (m == "ni" || m == "niquel" || m == "nickel") {
        return {
            {"Ni", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
            {"NiO", 1.0, 0.5, 0.0, [](double T) { return -239.7 + 0.0845 * T; }},
            {"Ni3S2", 3.0, -2.0, 2.0, [](double T) { return 442.0 - 0.0850 * T; }},
            {"NiSO4", 1.0, 1.0, 1.0, [](double T) { return -872.4 + 0.3485 * T; }}
        };
    }
    if (m == "pb" || m == "plomo" || m == "lead") {
        return {
            {"Pb", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
            {"PbO", 1.0, 0.5, 0.0, [](double T) { return -217.3 + 0.0988 * T; }},
            {"PbS", 1.0, -1.0, 1.0, [](double T) { return 264.0 - 0.0520 * T; }},
            {"PbSO4", 1.0, 1.0, 1.0, [](double T) { return -918.4 + 0.3640 * T; }},
            {"PbO*PbSO4", 2.0, 1.5, 1.0, [](double T) { return -1141.0 + 0.4680 * T; }}
        };
    }
    if (m == "zn" || m == "zinc") {
        return {
            {"Zn", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
            {"ZnO", 1.0, 0.5, 0.0, [](double T) { return -348.3 + 0.1005 * T; }},
            {"ZnS", 1.0, -1.0, 1.0, [](double T) { return 157.0 - 0.0380 * T; }},
            {"ZnSO4", 1.0, 1.0, 1.0, [](double T) { return -978.0 + 0.3620 * T; }},
            {"ZnO*2ZnSO4", 3.0, 2.5, 2.0, [](double T) { return -2310.0 + 0.8250 * T; }}
        };
    }
    return {};
}

} // namespace

std::vector<PuntoEstabilidad> StabilityDiagrams::generar(const std::string& metal,
                                                          double T) const
{
    return generar(metal, T, -30.0, 0.0, 60, -15.0, 5.0, 40);
}

std::vector<PuntoEstabilidad> StabilityDiagrams::generar(const std::string& metal, double T,
                                                          double log_pO2_min, double log_pO2_max, int steps_O2,
                                                          double log_pSO2_min, double log_pSO2_max, int steps_SO2) const
{
    std::vector<PuntoEstabilidad> puntos;
    if (!std::isfinite(T) || T < 300.0 || T > 2500.0) return puntos;
    if (steps_O2 < 2 || steps_SO2 < 2 || log_pO2_min >= log_pO2_max || log_pSO2_min >= log_pSO2_max) return puntos;

    auto fases = fases_para_metal(metal);
    if (fases.empty()) return puntos;

    const double R_kJ = 0.008314462618;
    const double ln10 = 2.302585092994046;
    const double RT_ln10 = R_kJ * T * ln10;

    const double dO2 = (log_pO2_max - log_pO2_min) / (steps_O2 - 1);
    const double dSO2 = (log_pSO2_max - log_pSO2_min) / (steps_SO2 - 1);

    puntos.reserve(steps_O2 * steps_SO2);

    for (int i = 0; i < steps_O2; ++i) {
        double lp_O2 = log_pO2_min + i * dO2;
        for (int j = 0; j < steps_SO2; ++j) {
            double lp_SO2 = log_pSO2_min + j * dSO2;

            double min_G = 1e30;
            std::string mejor_fase = fases[0].nombre;

            for (const auto& fase : fases) {
                double dG = fase.delta_G_rxn(T);
                double g_norm = (dG - (fase.nu_O2 * RT_ln10 * lp_O2) - (fase.nu_SO2 * RT_ln10 * lp_SO2)) / fase.nu_metal;
                if (g_norm < min_G) {
                    min_G = g_norm;
                    mejor_fase = fase.nombre;
                }
            }

            puntos.push_back({lp_O2, lp_SO2, mejor_fase});
        }
    }

    return puntos;
}

} // namespace cma::application::modules
