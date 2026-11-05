#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"
#include "perdida_calor.hpp"

#include <algorithm>
#include <cmath>
#include <iomanip>
#include <sstream>

namespace cma {
namespace motores {
namespace {

constexpr double kStefanBoltzmann = 5.670374419e-8; // W/(m2·K4)
constexpr double kGravity = 9.80665;                // m/s2

bool valid_layers(const std::vector<CapaAislante>& layers) {
    return std::all_of(layers.begin(), layers.end(), [](const auto& layer) {
        return std::isfinite(layer.k) && layer.k > 0 && std::isfinite(layer.espesor) && layer.espesor > 0;
    });
}

bool valid_boundary(double ti, double te, double hi, double he) {
    return std::isfinite(ti) && std::isfinite(te) && ti > 0 && te > 0 && ti < 10000 && te < 10000 &&
           std::isfinite(hi) && std::isfinite(he) && hi > 0 && he > 0;
}

double area_at_radius(domain::ThermalGeometryType geom, double r, double h_or_w, double d_or_w) {
    const double r_safe = std::max(1e-4, r);
    const double h_safe = std::max(1e-4, h_or_w);
    const double w_safe = std::max(1e-4, d_or_w);
    switch (geom) {
    case domain::ThermalGeometryType::PlaneWall:
        return h_safe * w_safe;
    case domain::ThermalGeometryType::Cube:
        return 6.0 * (2.0 * r_safe) * (2.0 * r_safe);
    case domain::ThermalGeometryType::Cylinder:
    case domain::ThermalGeometryType::HorizontalCylinder:
        return 2.0 * M_PI * r_safe * h_safe;
    case domain::ThermalGeometryType::Sphere:
        return 4.0 * M_PI * r_safe * r_safe;
    }
    return h_safe * w_safe;
}

double layer_volume_m3(domain::ThermalGeometryType geom, double r_in, double r_out, double h_or_w,
                       double d_or_w) {
    const double dr = std::max(0.0, r_out - r_in);
    if (dr <= 0.0)
        return 0.0;
    const double h_safe = std::max(1e-4, h_or_w);
    const double w_safe = std::max(1e-4, d_or_w);
    switch (geom) {
    case domain::ThermalGeometryType::PlaneWall:
        return h_safe * w_safe * dr;
    case domain::ThermalGeometryType::Cube:
        return std::pow(2.0 * r_out, 3) - std::pow(2.0 * r_in, 3);
    case domain::ThermalGeometryType::Cylinder:
    case domain::ThermalGeometryType::HorizontalCylinder:
        return M_PI * (r_out * r_out - r_in * r_in) * h_safe;
    case domain::ThermalGeometryType::Sphere:
        return (4.0 / 3.0) * M_PI * (std::pow(r_out, 3) - std::pow(r_in, 3));
    }
    return h_safe * w_safe * dr;
}

double segment_conduction_resistance_k_per_w(domain::ThermalGeometryType geom, double r_in, double r_out,
                                             double h_or_w, double d_or_w, double k_w_mk) {
    const double k_safe = std::max(1e-4, k_w_mk);
    const double dr = std::max(1e-6, r_out - r_in);
    const double h_safe = std::max(1e-4, h_or_w);
    switch (geom) {
    case domain::ThermalGeometryType::PlaneWall: {
        const double area = area_at_radius(geom, r_in, h_or_w, d_or_w);
        return dr / (k_safe * area);
    }
    case domain::ThermalGeometryType::Cube: {
        const double a_in = area_at_radius(geom, r_in, h_or_w, d_or_w);
        const double a_out = area_at_radius(geom, r_out, h_or_w, d_or_w);
        const double a_mean = std::sqrt(a_in * a_out);
        return dr / (k_safe * std::max(1e-6, a_mean));
    }
    case domain::ThermalGeometryType::Cylinder:
    case domain::ThermalGeometryType::HorizontalCylinder: {
        const double r1 = std::max(1e-4, r_in);
        const double r2 = std::max(r1 + 1e-6, r_out);
        return std::log(r2 / r1) / (2.0 * M_PI * h_safe * k_safe);
    }
    case domain::ThermalGeometryType::Sphere: {
        const double r1 = std::max(1e-4, r_in);
        const double r2 = std::max(r1 + 1e-6, r_out);
        return (1.0 / r1 - 1.0 / r2) / (4.0 * M_PI * k_safe);
    }
    }
    return dr / k_safe;
}

} // namespace

double PerdidaCalor::calcular_resistencia_conduccion(double k, double espesor, double area) {
    if (k <= 0 || area <= 0)
        return 1e12;
    return espesor / (k * area);
}

double PerdidaCalor::calcular_resistencia_conveccion(double h, double area) {
    if (h <= 0 || area <= 0)
        return 1e12;
    return 1.0 / (h * area);
}

ResultadoPerdidaCalor PerdidaCalor::pared_plana(double T_int, double T_ext, double area,
                                                const std::vector<CapaAislante>& capas, double h_int,
                                                double h_ext) {
    ResultadoPerdidaCalor res;
    error_.clear();
    if (!valid_boundary(T_int, T_ext, h_int, h_ext) || !valid_layers(capas) || !std::isfinite(area) ||
        area <= 0) {
        error_ = res.mensaje = "Temperaturas requieren Kelvin positivo y geometría/materiales válidos";
        return res;
    }
    double R_total = 0;

    // Resistencia convectiva interior
    R_total += calcular_resistencia_conveccion(h_int, area);

    // Resistencias conductivas por capa
    for (auto& c : capas)
        R_total += calcular_resistencia_conduccion(c.k, c.espesor, area);

    // Resistencia convectiva exterior
    R_total += calcular_resistencia_conveccion(h_ext, area);

    if (!std::isfinite(R_total) || R_total <= 0 || R_total > 1e10) {
        error_ = "Resistencia infinita";
        res.mensaje = error_;
        return res;
    }

    res.resistencia_total = R_total;
    res.flujo_calor = (T_int - T_ext) / R_total;

    // División segura
    double h_int_safe = std::max(h_int, 1e-6);
    double h_ext_safe = std::max(h_ext, 1e-6);
    res.T_pared_int = T_int - res.flujo_calor / (h_int_safe * area);
    res.T_pared_ext = T_ext + res.flujo_calor / (h_ext_safe * area);

    // Convection and radiation act in parallel at the outer surface. Solve
    // the same heat rate through the wall and the outer boundary (Kelvin).
    const double resistance_wall = R_total - 1 / (h_ext * area);
    double lo = std::min(T_int, T_ext), hi = std::max(T_int, T_ext);
    for (int i = 0; i < 100; ++i) {
        const double surface = (lo + hi) / 2;
        const double inward = (T_int - surface) / resistance_wall;
        const double outward = h_ext * area * (surface - T_ext) +
                               0.9 * kStefanBoltzmann * area * (std::pow(surface, 4) - std::pow(T_ext, 4));
        if (inward > outward)
            lo = surface;
        else
            hi = surface;
    }
    res.T_pared_ext = (lo + hi) / 2;
    res.flujo_calor = (T_int - res.T_pared_ext) / resistance_wall;
    res.T_pared_int = T_int - res.flujo_calor / (h_int * area);
    res.resistencia_total = std::abs(res.flujo_calor) > 1e-15 ? (T_int - T_ext) / res.flujo_calor : R_total;
    res.perfil_T = perfil_temperatura(res.T_pared_int, res.T_pared_ext, capas, area);
    res.ok = true;
    res.mensaje = "Flujo de calor: " + std::to_string(res.flujo_calor) + " W";

    return res;
}

ResultadoPerdidaCalor PerdidaCalor::tuberia(double T_int, double T_ext, double r_int, double largo,
                                            const std::vector<CapaAislante>& capas, double h_int,
                                            double h_ext) {
    ResultadoPerdidaCalor res;
    error_.clear();
    if (!valid_boundary(T_int, T_ext, h_int, h_ext) || !valid_layers(capas) || !std::isfinite(r_int) ||
        r_int <= 0 || !std::isfinite(largo) || largo <= 0) {
        error_ = res.mensaje = "Geometría o temperaturas inválidas";
        return res;
    }
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

    if (!std::isfinite(R_total) || R_total > 1e10 || R_total <= 0) {
        error_ = "Resistencia infinita o nula";
        res.mensaje = error_;
        return res;
    }

    res.resistencia_total = R_total;
    res.flujo_calor = (T_int - T_ext) / R_total;
    res.T_pared_int = T_int - res.flujo_calor / (h_int * A_int);
    res.T_pared_ext = T_ext + res.flujo_calor / (h_ext * A_ext);
    res.ok = true;
    res.mensaje = "Flujo de calor en tubería: " + std::to_string(res.flujo_calor) + " W";

    return res;
}

ResultadoPerdidaCalor PerdidaCalor::tanque_esferico(double T_int, double T_ext, double r_int,
                                                    const std::vector<CapaAislante>& capas, double h_int,
                                                    double h_ext) {
    ResultadoPerdidaCalor res;
    error_.clear();
    if (!valid_boundary(T_int, T_ext, h_int, h_ext) || !valid_layers(capas) || !std::isfinite(r_int) ||
        r_int <= 0) {
        error_ = res.mensaje = "Geometría o temperaturas inválidas";
        return res;
    }
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

    if (!std::isfinite(R_total) || R_total > 1e10 || R_total <= 0) {
        error_ = "Resistencia infinita o nula";
        res.mensaje = error_;
        return res;
    }

    res.resistencia_total = R_total;
    res.flujo_calor = (T_int - T_ext) / R_total;
    res.T_pared_int = T_int - res.flujo_calor / (h_int * A_int);
    res.T_pared_ext = T_ext + res.flujo_calor / (h_ext * A_ext);
    res.ok = true;
    res.mensaje = "Flujo de calor en tanque: " + std::to_string(res.flujo_calor) + " W";
    return res;
}

std::vector<double> PerdidaCalor::perfil_temperatura(double T_int, double T_ext,
                                                     const std::vector<CapaAislante>& capas, double area,
                                                     int puntos_por_capa) {
    if (!std::isfinite(T_int) || !std::isfinite(T_ext) || !std::isfinite(area) || area <= 0 ||
        puntos_por_capa < 1 || puntos_por_capa > 10000 || !valid_layers(capas))
        return {};
    std::vector<double> perfil = {T_int};
    double R_total = 0;
    double R_acum = 0;

    for (auto& c : capas)
        R_total += calcular_resistencia_conduccion(c.k, c.espesor, area);

    if (std::abs(R_total) < 1e-15)
        return perfil;

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

// ═══════════════════════════════════════════════════════════════════════════
// Motor Multicapa No Lineal estilo HSC Heat Loss
// ═══════════════════════════════════════════════════════════════════════════

double PerdidaCalor::calcular_conveccion_superficie(const domain::ConvectionFluidEntry* fluid,
                                                    double t_surf_c, double t_fluid_c, double char_length_m,
                                                    double velocity_m_s,
                                                    domain::ConvectionOrientation orient) {
    const double L = std::max(0.05, char_length_m);
    const double dT = std::max(0.1, std::abs(t_surf_c - t_fluid_c));
    const double t_film_k = std::max(150.0, 0.5 * (t_surf_c + t_fluid_c) + 273.15);

    // Si es un baño fundido o líquido con hc tabulado alto y sin gas ideal
    if (fluid && !fluid->is_gas && velocity_m_s <= 1e-4) {
        // Escalamiento suave por (dT)^0.25 respecto a dT_ref = 25 K
        return std::max(1.0, fluid->default_hc_natural * std::pow(dT / 25.0, 0.20));
    }

    // Propiedades de película para gases (o aire por defecto)
    double k_f = fluid ? fluid->thermal_cond_w_mk : 0.0262;
    double nu = fluid ? fluid->kinematic_visc_m2_s : 1.58e-5;
    double Pr = fluid ? fluid->prandtl : 0.71;
    double beta = (fluid && !fluid->is_gas) ? fluid->expansion_beta_1_k : (1.0 / t_film_k);

    if (!fluid || fluid->is_gas) {
        // Corrección con temperatura de película respecto a 300 K
        const double ratio = t_film_k / 300.0;
        k_f *= std::pow(ratio, 0.81);
        nu *= std::pow(ratio, 1.72) / std::max(0.1, fluid ? fluid->pressure_bar : 1.0);
    }

    // Convección natural (número de Grashof y Rayleigh)
    const double Gr = (kGravity * beta * dT * std::pow(L, 3)) / std::max(1e-14, nu * nu);
    const double Ra = std::max(1.0, Gr * Pr);

    double Nu_nat = 1.0;
    switch (orient) {
    case domain::ConvectionOrientation::VerticalSurface: {
        // Correlación de Churchill-Chu para placa/cilindro vertical (toda escala de Ra)
        const double denom = std::pow(1.0 + std::pow(0.492 / Pr, 9.0 / 16.0), 8.0 / 27.0);
        const double root_ra = 0.825 + (0.387 * std::pow(Ra, 1.0 / 6.0)) / denom;
        Nu_nat = root_ra * root_ra;
        break;
    }
    case domain::ConvectionOrientation::HorizontalHotUp: {
        // Placa horizontal caliente hacia arriba (McAdams)
        Nu_nat = (Ra < 1e7) ? 0.54 * std::pow(Ra, 0.25) : 0.15 * std::pow(Ra, 1.0 / 3.0);
        break;
    }
    case domain::ConvectionOrientation::HorizontalHotDown: {
        // Placa horizontal caliente hacia abajo
        Nu_nat = 0.27 * std::pow(Ra, 0.25);
        break;
    }
    case domain::ConvectionOrientation::HorizontalCylinder: {
        // Correlación de Churchill-Chu para cilindro horizontal
        const double denom = std::pow(1.0 + std::pow(0.559 / Pr, 9.0 / 16.0), 8.0 / 27.0);
        const double root_ra = 0.60 + (0.387 * std::pow(Ra, 1.0 / 6.0)) / denom;
        Nu_nat = root_ra * root_ra;
        break;
    }
    case domain::ConvectionOrientation::Sphere: {
        // Correlación de Churchill para esfera
        const double denom = std::pow(1.0 + std::pow(0.469 / Pr, 9.0 / 16.0), 4.0 / 9.0);
        Nu_nat = 2.0 + (0.589 * std::pow(Ra, 0.25)) / denom;
        break;
    }
    }

    double Nu_total = Nu_nat;
    if (velocity_m_s > 1e-3) {
        const double Re = (velocity_m_s * L) / std::max(1e-10, nu);
        const double Nu_forced = (Re < 5e5) ? 0.664 * std::sqrt(Re) * std::cbrt(Pr)
                                            : (0.037 * std::pow(Re, 0.8) - 871.0) * std::cbrt(Pr);
        // Convección mixta: Nu^3 = Nu_nat^3 + Nu_forced^3
        Nu_total = std::cbrt(std::pow(Nu_nat, 3.0) + std::pow(std::max(0.0, Nu_forced), 3.0));
    }

    const double hc = (Nu_total * k_f) / L;
    return std::clamp(hc, 0.5, 50000.0);
}

double PerdidaCalor::calcular_radiacion_superficie(double emissivity, double t_surf_c, double t_surr_c) {
    const double eps = std::clamp(emissivity, 0.01, 1.0);
    const double t1_k = std::max(50.0, t_surf_c + 273.15);
    const double t2_k = std::max(50.0, t_surr_c + 273.15);
    return eps * kStefanBoltzmann * (t1_k * t1_k + t2_k * t2_k) * (t1_k + t2_k);
}

domain::GasParticleRadiationResult
PerdidaCalor::calcular_radiacion_gas_particulas(const domain::GasParticleRadiationInput& in) {
    domain::GasParticleRadiationResult res;
    const double tg_k = std::max(200.0, in.gas_temp_c + 273.15);
    const double tw_k = std::max(200.0, in.wall_temp_c + 273.15);
    const double p_tot = std::max(0.1, in.total_pressure_bar);
    const double L_e = std::max(0.05, in.mean_beam_length_m);

    // Caminos ópticos presión-longitud (bar·m) para H2O, CO2 y SO2 (correlación tipo Hottel/Leckner)
    const double p_h2o = p_tot * std::clamp(in.p_h2o_vol_pct / 100.0, 0.0, 1.0);
    const double p_co2 = p_tot * std::clamp(in.p_co2_vol_pct / 100.0, 0.0, 1.0);
    const double p_so2 = p_tot * std::clamp(in.p_so2_vol_pct / 100.0, 0.0, 1.0);

    const double pl_h2o = p_h2o * L_e;
    const double pl_co2 = p_co2 * L_e;
    const double pl_so2 = p_so2 * L_e;

    const double temp_factor_g = std::pow(1000.0 / tg_k, 0.45);
    const double eps_h2o = 0.62 * (1.0 - std::exp(-1.45 * std::pow(pl_h2o, 0.65) * temp_factor_g));
    const double eps_co2 = 0.24 * (1.0 - std::exp(-2.80 * std::pow(pl_co2, 0.60) * temp_factor_g));
    const double eps_so2 = 0.31 * (1.0 - std::exp(-2.10 * std::pow(pl_so2, 0.62) * temp_factor_g));

    // Corrección por solapamiento espectral H2O-CO2
    const double overlap = (pl_h2o > 1e-4 && pl_co2 > 1e-4)
                               ? std::min(0.055, 0.03 * std::log10(1.0 + 10.0 * (pl_h2o + pl_co2)))
                               : 0.0;
    res.gas_emissivity = std::clamp(eps_h2o + eps_co2 + eps_so2 - overlap, 0.0, 0.96);

    // Emisividad de partículas/polvo en suspensión (Ley de Beer-Lambert sobre carga g/Nm3)
    const double tau_dust = 0.045 * std::max(0.0, in.dust_load_g_nm3) * L_e * (273.15 / tg_k);
    res.particle_emissivity = std::clamp(1.0 - std::exp(-tau_dust), 0.0, 0.99);

    // Emisividad combinada gas + partículas: eps_tot = eps_g + eps_p - eps_g * eps_p
    res.total_emissivity = std::clamp(res.gas_emissivity + res.particle_emissivity -
                                          res.gas_emissivity * res.particle_emissivity,
                                      0.01, 0.995);

    // Absortividad a temperatura de pared
    res.gas_absorptivity = std::clamp(res.total_emissivity * std::pow(tg_k / tw_k, 0.5), 0.01, 0.995);

    const double eps_wall_eff = 0.5 * (std::clamp(in.wall_emissivity, 0.05, 1.0) + 1.0);
    const double q_w_m2 =
        eps_wall_eff * kStefanBoltzmann *
        (res.total_emissivity * std::pow(tg_k, 4) - res.gas_absorptivity * std::pow(tw_k, 4));
    res.radiative_flux_kw_m2 = q_w_m2 / 1000.0;

    const double dT = tg_k - tw_k;
    if (std::abs(dT) > 0.1) {
        res.effective_hr_w_m2k = std::max(0.0, q_w_m2 / dT);
    } else {
        res.effective_hr_w_m2k =
            4.0 * eps_wall_eff * res.total_emissivity * kStefanBoltzmann * std::pow(tg_k, 3);
    }
    return res;
}

void PerdidaCalor::actualizar_propiedades_y_resistencias_hoja(domain::ThermalSheet& sheet,
                                                              const domain::ThermalPropertyCatalog* catalog) {
    double r = std::max(0.01, sheet.inner_diameter_or_width_m / 2.0);
    const double h_len = std::max(0.01, sheet.height_or_length_m);
    const double d_wid = std::max(0.01, sheet.inner_diameter_or_width_m);

    sheet.inner_area_m2 = area_at_radius(sheet.geometry, r, h_len, d_wid);
    sheet.total_thickness_cm = 0.0;
    sheet.total_mass_kg = 0.0;
    sheet.total_resistance_c_kw = 0.0;

    for (size_t i = 0; i < sheet.columns.size(); ++i) {
        auto& col = sheet.columns[i];
        const double t_mean_c = 0.5 * (col.temp_left_c + col.temp_right_c);
        const double thick_m = std::max(0.0, col.thickness_cm / 100.0);

        col.area_left_m2 = area_at_radius(sheet.geometry, r, h_len, d_wid);

        if (col.type == domain::ThermalColumnType::Surface) {
            col.thickness_cm = 0.0;
            col.area_right_m2 = col.area_left_m2;
            col.mass_kg = 0.0;

            if (catalog) {
                if (const auto* surf = catalog->find_surface(col.surface_material)) {
                    col.emissivity = surf->evaluate_emissivity(t_mean_c);
                }
            }

            if (!col.fix_convection) {
                const domain::ConvectionFluidEntry* fluid =
                    catalog ? catalog->find_convection(col.material) : nullptr;
                // Solo recalculamos por correlación si hay catálogo o velocidad/orientación activa
                if (fluid) {
                    const double t_surf = (i == 0) ? col.temp_right_c : col.temp_left_c;
                    const double t_fluid = (i == 0) ? col.temp_left_c : col.temp_right_c;
                    col.convection_hc_w_m2k = calcular_conveccion_superficie(
                        fluid, t_surf, t_fluid, h_len, col.velocity_m_s, sheet.orientation);
                }
            }

            if (!col.fix_radiation) {
                col.radiation_hr_w_m2k =
                    calcular_radiacion_superficie(col.emissivity, col.temp_left_c, col.temp_right_c);
            }

            if (!col.fix_resistance) {
                const double h_tot = std::max(1e-4, col.convection_hc_w_m2k + col.radiation_hr_w_m2k);
                col.resistance_c_kw = 1000.0 / (h_tot * col.area_left_m2);
            }
        } else if (col.type == domain::ThermalColumnType::SolidLayer) {
            const double r_next = r + thick_m;
            col.area_right_m2 = area_at_radius(sheet.geometry, r_next, h_len, d_wid);
            sheet.total_thickness_cm += col.thickness_cm;

            const domain::ConductionMaterialEntry* mat =
                catalog ? catalog->find_conduction(col.material) : nullptr;
            if (mat) {
                col.max_service_temp_c = mat->max_service_temp_c;
                const double vol = layer_volume_m3(sheet.geometry, r, r_next, h_len, d_wid);
                col.mass_kg = vol * mat->density_kg_m3;
            } else {
                const double vol = layer_volume_m3(sheet.geometry, r, r_next, h_len, d_wid);
                col.mass_kg = vol * 2400.0;
            }
            sheet.total_mass_kg += col.mass_kg;

            const int n_sub = std::clamp(col.grid_points, 1, 100);
            if (!col.fix_conductivity && mat) {
                // Integrar conductividad k(T) sobre los sub-intervalos de la capa
                double k_sum = 0.0;
                double r_sub_total_k_w = 0.0;
                for (int s = 0; s < n_sub; ++s) {
                    const double f0 = static_cast<double>(s) / n_sub;
                    const double f1 = static_cast<double>(s + 1) / n_sub;
                    const double fm = 0.5 * (f0 + f1);
                    const double t_sub = col.temp_left_c + fm * (col.temp_right_c - col.temp_left_c);
                    const double k_sub = mat->evaluate_k(t_sub);
                    k_sum += k_sub;
                    const double r0 = r + f0 * thick_m;
                    const double r1 = r + f1 * thick_m;
                    r_sub_total_k_w +=
                        segment_conduction_resistance_k_per_w(sheet.geometry, r0, r1, h_len, d_wid, k_sub);
                }
                col.conductivity_w_mk = k_sum / n_sub;
                if (!col.fix_resistance) {
                    col.resistance_c_kw = r_sub_total_k_w * 1000.0;
                }
            } else if (!col.fix_resistance) {
                const double r_k_w = segment_conduction_resistance_k_per_w(sheet.geometry, r, r_next, h_len,
                                                                           d_wid, col.conductivity_w_mk);
                col.resistance_c_kw = r_k_w * 1000.0;
            }

            col.exceeds_service_limit =
                (std::max(col.temp_left_c, col.temp_right_c) > col.max_service_temp_c);
            r = r_next;
        } else if (col.type == domain::ThermalColumnType::ContactLayer) {
            col.thickness_cm = 0.0;
            col.area_right_m2 = col.area_left_m2;
            col.mass_kg = 0.0;
            // Contact layer resistance is user-specified in °C/kW (default > 0)
            col.resistance_c_kw = std::max(0.0, col.resistance_c_kw);
        } else if (col.type == domain::ThermalColumnType::Enclosure) {
            const double r_next = r + thick_m;
            col.area_right_m2 = area_at_radius(sheet.geometry, r_next, h_len, d_wid);
            sheet.total_thickness_cm += col.thickness_cm;
            col.mass_kg = 0.0;

            if (!col.fix_resistance) {
                const domain::ConvectionFluidEntry* gas =
                    catalog ? catalog->find_convection(col.material) : nullptr;
                const double k_gas = gas ? gas->thermal_cond_w_mk : 0.026;
                const double hc_gap = calcular_conveccion_superficie(
                    gas, col.temp_left_c, col.temp_right_c, std::max(0.01, thick_m), 0.0, sheet.orientation);
                const double k_eff_conv = std::max(k_gas, hc_gap * std::max(0.002, thick_m));
                const double r_cond_conv_k_w = segment_conduction_resistance_k_per_w(
                    sheet.geometry, r, r_next, h_len, d_wid, k_eff_conv);

                const double eps_eff = 1.0 / (1.0 / std::clamp(col.emissivity, 0.05, 1.0) +
                                              (col.area_left_m2 / std::max(1e-6, col.area_right_m2)) *
                                                  (1.0 / std::clamp(col.emissivity, 0.05, 1.0) - 1.0));
                col.radiation_hr_w_m2k =
                    calcular_radiacion_superficie(eps_eff, col.temp_left_c, col.temp_right_c);
                col.convection_hc_w_m2k = hc_gap;
                col.conductivity_w_mk = k_eff_conv;

                const double g_cond_conv = 1.0 / std::max(1e-9, r_cond_conv_k_w);
                const double g_rad = col.radiation_hr_w_m2k * col.area_left_m2;
                col.resistance_c_kw = 1000.0 / std::max(1e-6, g_cond_conv + g_rad);
            }
            r = r_next;
        }

        sheet.total_resistance_c_kw += col.resistance_c_kw;
    }

    sheet.outer_area_m2 = area_at_radius(sheet.geometry, r, h_len, d_wid);
    if (sheet.total_resistance_c_kw > 1e-9) {
        const double r_k_w = sheet.total_resistance_c_kw / 1000.0;
        sheet.overall_u_inner_w_m2k = 1.0 / (r_k_w * std::max(1e-6, sheet.inner_area_m2));
        sheet.overall_u_outer_w_m2k = 1.0 / (r_k_w * std::max(1e-6, sheet.outer_area_m2));
    }
}

bool PerdidaCalor::resolver_flujo_calor_dos_puntos(domain::ThermalSheet& sheet,
                                                   const domain::ThermalPropertyCatalog* catalog) {
    if (sheet.columns.empty()) {
        sheet.converged = false;
        sheet.status_message = "La hoja no contiene columnas térmicas.";
        return false;
    }

    const size_t n = sheet.columns.size();

    // Determinar los dos nodos de temperatura fijados (0..n donde nodo j es el borde izquierdo de col j, y
    // nodo n es borde derecho de col n-1)
    int node_a = -1;
    double temp_a = sheet.columns.front().temp_left_c;
    int node_b = -1;
    double temp_b = sheet.columns.back().temp_right_c;

    for (size_t i = 0; i < n; ++i) {
        if (sheet.columns[i].fix_temp_left) {
            if (node_a < 0) {
                node_a = static_cast<int>(i);
                temp_a = sheet.columns[i].temp_left_c;
            } else if (node_b < 0 && static_cast<int>(i) > node_a) {
                node_b = static_cast<int>(i);
                temp_b = sheet.columns[i].temp_left_c;
            }
        }
        if (sheet.columns[i].fix_temp_right) {
            const int right_node = static_cast<int>(i) + 1;
            if (node_a < 0) {
                node_a = right_node;
                temp_a = sheet.columns[i].temp_right_c;
            } else if (node_b < 0 && right_node > node_a) {
                node_b = right_node;
                temp_b = sheet.columns[i].temp_right_c;
            }
        }
    }

    if (node_a < 0 || node_b < 0 || node_a >= node_b) {
        node_a = 0;
        temp_a = sheet.columns.front().temp_left_c;
        node_b = static_cast<int>(n);
        temp_b = sheet.columns.back().temp_right_c;
    }

    double prev_q = sheet.total_heat_flow_kw;
    bool converged = false;
    int iter = 0;

    for (iter = 1; iter <= 60; ++iter) {
        actualizar_propiedades_y_resistencias_hoja(sheet, catalog);

        double r_span_c_kw = 0.0;
        for (int idx = node_a; idx < node_b; ++idx) {
            r_span_c_kw += sheet.columns[idx].resistance_c_kw;
        }
        r_span_c_kw = std::max(1e-6, r_span_c_kw);

        const double q_new = (temp_a - temp_b) / r_span_c_kw; // kW: dT(°C) / R(°C/kW)
        const double q_damped = (iter == 1) ? q_new : (0.75 * q_new + 0.25 * prev_q);
        sheet.total_heat_flow_kw = q_damped;

        // Reconstruir temperaturas en todos los nodos 0..n a partir de node_a y q_damped
        std::vector<double> node_temps(n + 1, 0.0);
        node_temps[node_a] = temp_a;
        for (int idx = node_a; idx < static_cast<int>(n); ++idx) {
            node_temps[idx + 1] = node_temps[idx] - q_damped * sheet.columns[idx].resistance_c_kw;
        }
        for (int idx = node_a - 1; idx >= 0; --idx) {
            node_temps[idx] = node_temps[idx + 1] + q_damped * sheet.columns[idx].resistance_c_kw;
        }

        for (size_t i = 0; i < n; ++i) {
            auto& col = sheet.columns[i];
            col.temp_left_c = node_temps[i];
            col.temp_right_c = node_temps[i + 1];
            col.heat_flow_kw = q_damped;
            const double area_mean = 0.5 * (col.area_left_m2 + col.area_right_m2);
            col.heat_flux_kw_m2 = q_damped / std::max(1e-6, area_mean);
        }

        if (iter > 2 && std::abs(q_new - prev_q) <= 1e-6 * (1.0 + std::abs(q_new))) {
            converged = true;
            prev_q = q_new;
            break;
        }
        prev_q = q_damped;
    }

    actualizar_propiedades_y_resistencias_hoja(sheet, catalog);
    generar_perfil_detallado(sheet, catalog);

    sheet.inner_fluid_temp_c = sheet.columns.front().temp_left_c;
    sheet.inner_wall_temp_c = (sheet.columns.front().type == domain::ThermalColumnType::Surface)
                                  ? sheet.columns.front().temp_right_c
                                  : sheet.columns.front().temp_left_c;
    sheet.outer_shell_temp_c = (sheet.columns.back().type == domain::ThermalColumnType::Surface)
                                   ? sheet.columns.back().temp_left_c
                                   : sheet.columns.back().temp_right_c;
    sheet.ambient_temp_c = sheet.columns.back().temp_right_c;

    sheet.converged = converged || (iter <= 60);
    sheet.iterations = iter;
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(3) << "Heat Loss Solver OK (" << iter
        << " iter): Q = " << sheet.total_heat_flow_kw << " kW, R_tot = " << sheet.total_resistance_c_kw
        << " °C/kW, T_shell = " << sheet.outer_shell_temp_c << " °C";
    sheet.status_message = oss.str();
    return true;
}

bool PerdidaCalor::resolver_perfil_temperatura_q_fijo(domain::ThermalSheet& sheet,
                                                      const domain::ThermalPropertyCatalog* catalog) {
    if (sheet.columns.empty()) {
        sheet.converged = false;
        sheet.status_message = "La hoja no contiene columnas térmicas.";
        return false;
    }

    const size_t n = sheet.columns.size();
    const double Q = sheet.total_heat_flow_kw;

    // Buscar el nodo con temperatura fija (por defecto nodo 0 = temp_left de col 0)
    int anchor_node = 0;
    double anchor_temp = sheet.columns.front().temp_left_c;
    for (size_t i = 0; i < n; ++i) {
        if (sheet.columns[i].fix_temp_left) {
            anchor_node = static_cast<int>(i);
            anchor_temp = sheet.columns[i].temp_left_c;
            break;
        }
        if (sheet.columns[i].fix_temp_right) {
            anchor_node = static_cast<int>(i) + 1;
            anchor_temp = sheet.columns[i].temp_right_c;
            break;
        }
    }

    int iter = 0;
    double max_delta = 0.0;
    for (iter = 1; iter <= 50; ++iter) {
        actualizar_propiedades_y_resistencias_hoja(sheet, catalog);

        std::vector<double> node_temps(n + 1, 0.0);
        node_temps[anchor_node] = anchor_temp;
        for (int idx = anchor_node; idx < static_cast<int>(n); ++idx) {
            node_temps[idx + 1] = node_temps[idx] - Q * sheet.columns[idx].resistance_c_kw;
        }
        for (int idx = anchor_node - 1; idx >= 0; --idx) {
            node_temps[idx] = node_temps[idx + 1] + Q * sheet.columns[idx].resistance_c_kw;
        }

        max_delta = 0.0;
        for (size_t i = 0; i < n; ++i) {
            auto& col = sheet.columns[i];
            max_delta = std::max(max_delta, std::abs(col.temp_left_c - node_temps[i]));
            max_delta = std::max(max_delta, std::abs(col.temp_right_c - node_temps[i + 1]));
            col.temp_left_c = node_temps[i];
            col.temp_right_c = node_temps[i + 1];
            col.heat_flow_kw = Q;
            const double area_mean = 0.5 * (col.area_left_m2 + col.area_right_m2);
            col.heat_flux_kw_m2 = Q / std::max(1e-6, area_mean);
        }

        if (iter > 2 && max_delta < 1e-4)
            break;
    }

    actualizar_propiedades_y_resistencias_hoja(sheet, catalog);
    generar_perfil_detallado(sheet, catalog);

    sheet.inner_fluid_temp_c = sheet.columns.front().temp_left_c;
    sheet.inner_wall_temp_c = (sheet.columns.front().type == domain::ThermalColumnType::Surface)
                                  ? sheet.columns.front().temp_right_c
                                  : sheet.columns.front().temp_left_c;
    sheet.outer_shell_temp_c = (sheet.columns.back().type == domain::ThermalColumnType::Surface)
                                   ? sheet.columns.back().temp_left_c
                                   : sheet.columns.back().temp_right_c;
    sheet.ambient_temp_c = sheet.columns.back().temp_right_c;

    sheet.converged = true;
    sheet.iterations = iter;
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(3) << "Temperature Profile Solver OK (" << iter
        << " iter): Q fijo = " << Q << " kW -> T_ext = " << sheet.ambient_temp_c << " °C";
    sheet.status_message = oss.str();
    return true;
}

bool PerdidaCalor::resolver_hoja(domain::ThermalSheet& sheet, const domain::ThermalPropertyCatalog* catalog) {
    if (sheet.solver_mode == domain::HeatLossSolverMode::TemperatureProfile) {
        return resolver_perfil_temperatura_q_fijo(sheet, catalog);
    }
    return resolver_flujo_calor_dos_puntos(sheet, catalog);
}

domain::TargetThicknessResult
PerdidaCalor::optimizar_espesor_capa(domain::ThermalSheet& sheet, const domain::TargetThicknessRequest& req,
                                     const domain::ThermalPropertyCatalog* catalog) {
    domain::TargetThicknessResult res;
    if (req.target_column_index < 0 || static_cast<size_t>(req.target_column_index) >= sheet.columns.size()) {
        res.message = "Índice de columna inválido para optimización de espesor.";
        return res;
    }

    auto& target_col = sheet.columns[req.target_column_index];
    if (target_col.type != domain::ThermalColumnType::SolidLayer &&
        target_col.type != domain::ThermalColumnType::Enclosure) {
        res.message = "La columna seleccionada debe ser una capa sólida (Solid Layer) o Enclosure.";
        return res;
    }

    double lo = std::max(0.1, req.min_thickness_cm);
    double hi = std::max(lo + 1.0, req.max_thickness_cm);

    auto eval_metric = [&](double thick_cm) {
        target_col.thickness_cm = thick_cm;
        resolver_flujo_calor_dos_puntos(sheet, catalog);
        return req.target_is_outer_temp ? sheet.outer_shell_temp_c : sheet.total_heat_flow_kw;
    };

    const double f_lo = eval_metric(lo) - req.target_value;
    const double f_hi = eval_metric(hi) - req.target_value;

    // Al aumentar el espesor aislante, tanto T_shell exterior como Q_total disminuyen monótonamente
    if (f_lo * f_hi > 0.0) {
        const double best = (std::abs(f_lo) < std::abs(f_hi)) ? lo : hi;
        eval_metric(best);
        res.ok = false;
        res.optimal_thickness_cm = best;
        res.achieved_outer_temp_c = sheet.outer_shell_temp_c;
        res.achieved_heat_flow_kw = sheet.total_heat_flow_kw;
        res.message = "El valor objetivo está fuera del rango de espesores [" + std::to_string(lo) + ", " +
                      std::to_string(hi) + "] cm.";
        return res;
    }

    double mid = 0.5 * (lo + hi);
    for (int iter = 0; iter < 40; ++iter) {
        mid = 0.5 * (lo + hi);
        const double f_mid = eval_metric(mid) - req.target_value;
        if (std::abs(f_mid) < 1e-3 || (hi - lo) < 1e-3)
            break;
        if ((f_lo > 0.0 && f_mid > 0.0) || (f_lo < 0.0 && f_mid < 0.0)) {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    eval_metric(mid);
    res.ok = true;
    res.optimal_thickness_cm = mid;
    res.achieved_outer_temp_c = sheet.outer_shell_temp_c;
    res.achieved_heat_flow_kw = sheet.total_heat_flow_kw;

    std::ostringstream oss;
    oss << std::fixed << std::setprecision(2) << "Espesor óptimo = " << mid
        << " cm (T_shell = " << res.achieved_outer_temp_c << " °C, Q = " << std::setprecision(3)
        << res.achieved_heat_flow_kw << " kW)";
    res.message = oss.str();
    return res;
}

void PerdidaCalor::generar_perfil_detallado(domain::ThermalSheet& sheet,
                                            const domain::ThermalPropertyCatalog* catalog) {
    sheet.profile.clear();
    if (sheet.columns.empty())
        return;

    double x_mm = 0.0;
    double r = std::max(0.01, sheet.inner_diameter_or_width_m / 2.0);
    const double h_len = std::max(0.01, sheet.height_or_length_m);
    const double d_wid = std::max(0.01, sheet.inner_diameter_or_width_m);
    const double Q_kw = sheet.total_heat_flow_kw;

    // Punto inicial (borde izquierdo de la primera columna)
    {
        const auto& c0 = sheet.columns.front();
        sheet.profile.push_back({0.0, c0.temp_left_c, c0.conductivity_w_mk, c0.heat_flux_kw_m2,
                                 c0.max_service_temp_c, 0, c0.name.empty() ? "Col 1" : c0.name, c0.material});
    }

    for (size_t i = 0; i < sheet.columns.size(); ++i) {
        const auto& col = sheet.columns[i];
        const std::string col_label = col.name.empty() ? ("Col " + std::to_string(i + 1)) : col.name;

        if (col.type == domain::ThermalColumnType::SolidLayer ||
            col.type == domain::ThermalColumnType::Enclosure) {
            const double dx_mm = std::max(0.0, col.thickness_cm * 10.0);
            const double thick_m = dx_mm / 1000.0;
            const int steps = std::clamp(col.grid_points, 2, 100);
            const domain::ConductionMaterialEntry* mat =
                (catalog && col.type == domain::ThermalColumnType::SolidLayer)
                    ? catalog->find_conduction(col.material)
                    : nullptr;

            // Calcular resistencias parciales por sub-celda para perfil curvo exacto (cilíndrico/esférico y
            // k(T))
            std::vector<double> r_sub(steps, 0.0);
            double r_sum = 0.0;
            for (int s = 0; s < steps; ++s) {
                const double f0 = static_cast<double>(s) / steps;
                const double f1 = static_cast<double>(s + 1) / steps;
                const double fm = 0.5 * (f0 + f1);
                const double t_est = col.temp_left_c + fm * (col.temp_right_c - col.temp_left_c);
                const double k_loc = (mat && !col.fix_conductivity) ? mat->evaluate_k(t_est)
                                                                    : std::max(1e-4, col.conductivity_w_mk);
                const double r0 = r + f0 * thick_m;
                const double r1 = r + f1 * thick_m;
                r_sub[s] = segment_conduction_resistance_k_per_w(sheet.geometry, r0, r1, h_len, d_wid, k_loc);
                r_sum += r_sub[s];
            }

            double r_accum = 0.0;
            for (int s = 0; s < steps; ++s) {
                r_accum += r_sub[s];
                const double frac_r =
                    (r_sum > 1e-12) ? (r_accum / r_sum) : (static_cast<double>(s + 1) / steps);
                const double frac_x = static_cast<double>(s + 1) / steps;
                const double t_pt = col.temp_left_c + frac_r * (col.temp_right_c - col.temp_left_c);
                const double r_pt = r + frac_x * thick_m;
                const double area_pt = area_at_radius(sheet.geometry, r_pt, h_len, d_wid);
                const double k_pt =
                    (mat && !col.fix_conductivity) ? mat->evaluate_k(t_pt) : col.conductivity_w_mk;

                sheet.profile.push_back({x_mm + dx_mm * frac_x, t_pt, k_pt, Q_kw / std::max(1e-6, area_pt),
                                         col.max_service_temp_c, static_cast<int>(i), col_label,
                                         col.material});
            }
            x_mm += dx_mm;
            r += thick_m;
        } else {
            // Salto de temperatura de interfaz (Surface o Contact Layer, espesor = 0 mm)
            sheet.profile.push_back({x_mm, col.temp_right_c, col.conductivity_w_mk, col.heat_flux_kw_m2,
                                     col.max_service_temp_c, static_cast<int>(i), col_label, col.material});
        }
    }
}

} // namespace motores
} // namespace cma

#pragma GCC diagnostic pop
