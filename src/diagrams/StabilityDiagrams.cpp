#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"

#include "StabilityDiagrams.hpp"
#include "StabilityCatalogHelpers.hpp"
#include "nucleo/tabla_periodica.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <functional>
#include <iomanip>
#include <map>
#include <set>
#include <sstream>

namespace cma::application::modules {

using namespace detail;

// ── Compatibilidad con regresión previa ──────────────────────────────────────
namespace {
struct FaseCondensadaLegado {
    std::string nombre;
    double nu_metal = 1.0;
    double nu_O2 = 0.0;
    double nu_SO2 = 0.0;
    std::function<double(double)> delta_G_rxn;
};

std::vector<FaseCondensadaLegado> fases_para_metal_legado(const std::string& metal) {
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

std::vector<PuntoEstabilidad> StabilityDiagrams::generar(const std::string& metal, double T) const {
    return generar(metal, T, -30.0, 0.0, 60, -15.0, 5.0, 40);
}

std::vector<PuntoEstabilidad> StabilityDiagrams::generar(const std::string& metal, double T,
                                                         double log_pO2_min, double log_pO2_max, int steps_O2,
                                                         double log_pSO2_min, double log_pSO2_max, int steps_SO2) const {
    std::vector<PuntoEstabilidad> puntos;
    if (!std::isfinite(T) || T < 300.0 || T > 2500.0) return puntos;
    if (steps_O2 < 2 || steps_SO2 < 2 || log_pO2_min >= log_pO2_max || log_pSO2_min >= log_pSO2_max) return puntos;

    auto fases = fases_para_metal_legado(metal);
    if (fases.empty()) return puntos;

    const double RT_ln10 = kR_kJ * T * kLn10;
    const double dO2 = (log_pO2_max - log_pO2_min) / (steps_O2 - 1);
    const double dSO2 = (log_pSO2_max - log_pSO2_min) / (steps_SO2 - 1);

    puntos.reserve(steps_O2 * steps_SO2);
    for (int i = 0; i < steps_O2; ++i) {
        const double lp_O2 = log_pO2_min + i * dO2;
        for (int j = 0; j < steps_SO2; ++j) {
            const double lp_SO2 = log_pSO2_min + j * dSO2;
            double min_G = 1e30;
            std::string mejor_fase = fases[0].nombre;
            for (const auto& fase : fases) {
                const double dG = fase.delta_G_rxn(T);
                const double g_norm = (dG - (fase.nu_O2 * RT_ln10 * lp_O2) - (fase.nu_SO2 * RT_ln10 * lp_SO2)) / fase.nu_metal;
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

// ── Implementación Hexagonal de Tpp y Lpp ───────────────────────────────────

domain::SistemaTernarioEstabilidad StabilityDiagrams::analizar_sistema_ternario(
    const std::vector<std::string>& elementos,
    bool incluir_liquidos,
    bool incluir_organicos,
    const std::string& forzar_elemento_principal,
    bool incluir_secundarias_sin_principal) const {

    domain::SistemaTernarioEstabilidad sis;
    std::set<std::string> set_el;
    for (const auto& e : elementos) {
        if (!e.empty()) set_el.insert(e);
    }
    sis.elementos.assign(set_el.begin(), set_el.end());
    if (sis.elementos.size() != 3) return sis;

    // Determinar elemento principal (el metal/elemento con mayor prioridad metálica)
    std::vector<std::string> ordenados = sis.elementos;
    std::sort(ordenados.begin(), ordenados.end(), [](const std::string& a, const std::string& b) {
        return prioridad_elemento_principal(a) > prioridad_elemento_principal(b);
    });

    if (!forzar_elemento_principal.empty() && set_el.count(forzar_elemento_principal)) {
        sis.elemento_principal = forzar_elemento_principal;
    } else {
        sis.elemento_principal = ordenados[0];
    }

    std::vector<std::string> aux;
    for (const auto& el : sis.elementos) {
        if (el != sis.elemento_principal) aux.push_back(el);
    }
    // Ordenar elementos auxiliares de forma determinista (ej. Cl antes de O, S antes de O)
    std::sort(aux.begin(), aux.end(), [](const std::string& a, const std::string& b) {
        return prioridad_elemento_principal(a) > prioridad_elemento_principal(b);
    });
    if (aux.size() >= 2) {
        sis.elem_aux1 = aux[0];
        sis.elem_aux2 = aux[1];
    }

    const auto catalogo = recolectar_catalogo_combinado(repo_);
    for (const auto& sp : catalogo) {
        const auto comp = parsear_composicion_fraccionaria(sp.formula);
        if (comp.empty()) continue;

        bool fuera_de_sistema = false;
        for (const auto& [el, cnt] : comp) {
            if (!set_el.count(el)) {
                fuera_de_sistema = true;
                break;
            }
        }
        if (fuera_de_sistema) continue;

        const bool tiene_c = comp.count("C") > 0;
        const bool tiene_h = comp.count("H") > 0;
        const bool es_org = (tiene_c && tiene_h && comp.at("C") >= 2.0);
        if (es_org && !incluir_organicos) continue;

        const bool tiene_main = (comp.count(sis.elemento_principal) > 0 && comp.at(sis.elemento_principal) > 0.0);

        domain::EspecieCandidataEstabilidad cand;
        cand.id = sp.id;
        cand.formula = sp.formula;
        cand.nombre = sp.nombre;
        cand.fase = sp.fase_std;
        cand.masa_molar = sp.masa_molar;
        cand.h_formacion_298 = sp.h_formacion;
        cand.s_298 = sp.s_formacion;
        cand.cp_298 = sp.cp(298.15);
        cand.T_min_K = sp.T_min;
        cand.T_max_K = sp.T_max;
        cand.n_main = tiene_main ? comp.at(sis.elemento_principal) : 0.0;
        cand.composicion = comp;
        cand.es_gas = (sp.fase_std == nucleo::Fase::Gas);
        cand.es_liquido = (sp.fase_std == nucleo::Fase::Liquido);
        cand.es_organico = es_org;
        cand.contiene_elemento_principal = tiene_main;

        if (cand.es_gas && !tiene_main) {
            cand.formula = format_gas_formula(cand.formula);
            sis.especies_gas_ejes.push_back(cand);
        } else if (!cand.es_gas) {
            if (cand.es_liquido && !incluir_liquidos) continue;
            if (tiene_main || incluir_secundarias_sin_principal) {
                // En V-Cl-O Imagen 1, V6O11 aparece desmarcada por defecto para mostrar VO2-V3O5 limpio
                if (cand.formula == "V6O11") cand.seleccionada_por_defecto = false;
                sis.especies_sistema.push_back(cand);
            }
        }
    }

    return sis;
}

std::optional<domain::EspecieCandidataEstabilidad> StabilityDiagrams::buscar_info_especie(
    const std::string& formula) const {
    const auto catalogo = recolectar_catalogo_combinado(repo_);
    const auto* sp = buscar_especie_catalogo(catalogo, formula, false);
    if (!sp) return std::nullopt;

    domain::EspecieCandidataEstabilidad cand;
    cand.id = sp->id;
    cand.formula = sp->formula;
    cand.nombre = sp->nombre;
    cand.fase = sp->fase_std;
    cand.masa_molar = sp->masa_molar;
    cand.h_formacion_298 = sp->h_formacion;
    cand.s_298 = sp->s_formacion;
    cand.cp_298 = sp->cp(298.15);
    cand.T_min_K = sp->T_min;
    cand.T_max_K = sp->T_max;
    cand.composicion = parsear_composicion_fraccionaria(sp->formula);
    cand.es_gas = (sp->fase_std == nucleo::Fase::Gas);
    cand.es_liquido = (sp->fase_std == nucleo::Fase::Liquido);
    return cand;
}

domain::ResultadoDiagramaLpp StabilityDiagrams::calcular_lpp(
    const domain::ParametrosCalculoLpp& params) const {

    domain::ResultadoDiagramaLpp res;
    if (params.elementos.size() != 3) {
        res.mensaje_error = "Seleccione exactamente 3 elementos para el cálculo del diagrama Lpp.";
        return res;
    }
    if (!std::isfinite(params.T_K) || params.T_K < 100.0 || params.T_K > 4000.0) {
        res.mensaje_error = "Temperatura fuera del dominio termodinámico válido (100 - 4000 K).";
        return res;
    }

    const auto sis = analizar_sistema_ternario(params.elementos, params.incluir_liquidos,
                                               params.incluir_organicos, params.elemento_principal, false);
    if (sis.elemento_principal.empty()) {
        res.mensaje_error = "No se pudo determinar el elemento principal del sistema ternario.";
        return res;
    }

    res.elemento_principal = sis.elemento_principal;
    res.titulo_sistema = sis.elemento_principal + "-" + sis.elem_aux1 + "-" + sis.elem_aux2;
    res.gas_x = format_gas_formula(params.gas_x);
    res.gas_y = format_gas_formula(params.gas_y);
    res.T_K = params.T_K;

    const auto comp_gx = parsear_composicion_fraccionaria(res.gas_x);
    const auto comp_gy = parsear_composicion_fraccionaria(res.gas_y);
    if (comp_gx.count(sis.elemento_principal) || comp_gy.count(sis.elemento_principal)) {
        res.mensaje_error = "Las especies de gas en los ejes X e Y no deben contener el elemento principal (" +
                            sis.elemento_principal + ").";
        return res;
    }

    double test_nx = 0.0, test_ny = 0.0;
    std::map<std::string, double> test_sp = {{sis.elem_aux1, 1.0}, {sis.elem_aux2, 1.0}};
    if (!resolver_estequiometria_gases(sis.elem_aux1, sis.elem_aux2, comp_gx, comp_gy, test_sp, test_nx, test_ny)) {
        res.mensaje_error = "Los gases seleccionados para el eje X (" + res.gas_x + ") y eje Y (" +
                            res.gas_y + ") son linealmente dependientes.";
        return res;
    }

    const auto catalogo = recolectar_catalogo_combinado(repo_);
    const auto* sp_gx = buscar_especie_catalogo(catalogo, res.gas_x, true);
    const auto* sp_gy = buscar_especie_catalogo(catalogo, res.gas_y, true);
    const auto* sp_main = buscar_especie_catalogo(catalogo, sis.elemento_principal, false);

    const double G_gx = sp_gx ? sp_gx->energia_gibbs(params.T_K) : 0.0;
    const double G_gy = sp_gy ? sp_gy->energia_gibbs(params.T_K) : 0.0;
    const double G_main = sp_main ? sp_main->energia_gibbs(params.T_K) : 0.0;
    const double RT_ln10 = kR_kJ * params.T_K * kLn10;

    std::set<std::string> sel_set(params.formulas_seleccionadas.begin(), params.formulas_seleccionadas.end());
    const bool usar_todas = sel_set.empty();

    struct CandidatoPlano {
        int64_t id = 0;
        std::string formula;
        std::string nombre;
        double n_m = 1.0;
        double nu_x = 0.0;
        double nu_y = 0.0;
        double nu_x_norm = 0.0;
        double nu_y_norm = 0.0;
        double Z = 0.0;
        double A = 0.0;
        double B = 0.0;
    };

    std::vector<CandidatoPlano> candidatos;
    for (const auto& cand : sis.especies_sistema) {
        if (!cand.contiene_elemento_principal || cand.n_main <= 0.0) continue;
        if (usar_todas ? !cand.seleccionada_por_defecto : !sel_set.count(cand.formula)) continue;

        const auto* sp = buscar_especie_catalogo(catalogo, cand.formula, false);
        if (!sp) continue;

        double nu_x = 0.0, nu_y = 0.0;
        if (!resolver_estequiometria_gases(sis.elem_aux1, sis.elem_aux2, comp_gx, comp_gy,
                                           cand.composicion, nu_x, nu_y)) {
            continue;
        }

        const double G_sp = sp->energia_gibbs(params.T_K);
        if (!sp->en_rango_temperatura(params.T_K)) {
            res.advertencias.push_back(sp->comprobar_rango(params.T_K).advertencia);
        }

        CandidatoPlano cp;
        cp.id = sp->id;
        cp.formula = cand.formula;
        cp.nombre = cand.nombre;
        cp.n_m = cand.n_main;
        cp.nu_x = nu_x;
        cp.nu_y = nu_y;
        cp.nu_x_norm = nu_x / cand.n_main;
        cp.nu_y_norm = nu_y / cand.n_main;
        cp.Z = (G_sp - cand.n_main * G_main - nu_x * G_gx - nu_y * G_gy) / cand.n_main;
        cp.A = -cp.nu_x_norm * RT_ln10;
        cp.B = -cp.nu_y_norm * RT_ln10;
        candidatos.push_back(cp);
    }

    if (candidatos.empty()) {
        res.mensaje_error = "Seleccione al menos una especie que contenga el elemento principal (" +
                            sis.elemento_principal + ").";
        return res;
    }

    // Omitir automáticamente sustancias con estequiometría idéntica conservando la más estable a T_K
    std::vector<CandidatoPlano> planos;
    if (params.omitir_estequiometria_identica) {
        for (const auto& cp : candidatos) {
            bool encontrado = false;
            for (auto& ex : planos) {
                if (std::abs(ex.nu_x_norm - cp.nu_x_norm) < 1e-6 &&
                    std::abs(ex.nu_y_norm - cp.nu_y_norm) < 1e-6) {
                    encontrado = true;
                    if (cp.Z < ex.Z) {
                        res.especies_omitidas_duplicadas.push_back(ex.formula);
                        ex = cp;
                    } else {
                        res.especies_omitidas_duplicadas.push_back(cp.formula);
                    }
                    break;
                }
            }
            if (!encontrado) planos.push_back(cp);
        }
    } else {
        planos = candidatos;
    }

    // Cálculo de auto-escala basado en las intersecciones de fronteras y puntos triples
    double x_min = params.x_min;
    double x_max = params.x_max;
    double y_min = params.y_min;
    double y_max = params.y_max;

    if (params.auto_escala || x_min >= x_max || y_min >= y_max) {
        std::vector<double> xs_crit, ys_crit;
        for (size_t i = 0; i < planos.size(); ++i) {
            for (size_t j = i + 1; j < planos.size(); ++j) {
                const double dA = planos[i].A - planos[j].A;
                const double dB = planos[i].B - planos[j].B;
                const double dZ = planos[i].Z - planos[j].Z;
                if (std::abs(dB) < 1e-9 && std::abs(dA) > 1e-9) {
                    const double xv = -dZ / dA;
                    // Comprobar si i y j son el mínimo en (xv, -40) o similar
                    xs_crit.push_back(xv);
                } else if (std::abs(dA) < 1e-9 && std::abs(dB) > 1e-9) {
                    const double yv = -dZ / dB;
                    ys_crit.push_back(yv);
                }
                for (size_t k = j + 1; k < planos.size(); ++k) {
                    const double a1 = planos[i].A - planos[j].A;
                    const double b1 = planos[i].B - planos[j].B;
                    const double c1 = -(planos[i].Z - planos[j].Z);
                    const double a2 = planos[i].A - planos[k].A;
                    const double b2 = planos[i].B - planos[k].B;
                    const double c2 = -(planos[i].Z - planos[k].Z);
                    const double det = a1 * b2 - a2 * b1;
                    if (std::abs(det) > 1e-9) {
                        const double xt = (c1 * b2 - c2 * b1) / det;
                        const double yt = (a1 * c2 - a2 * c1) / det;
                        const double g_trip = planos[i].Z + planos[i].A * xt + planos[i].B * yt;
                        bool estable = true;
                        for (size_t m = 0; m < planos.size(); ++m) {
                            const double gm = planos[m].Z + planos[m].A * xt + planos[m].B * yt;
                            if (gm < g_trip - 1e-5) {
                                estable = false;
                                break;
                            }
                        }
                        if (estable) {
                            xs_crit.push_back(xt);
                            ys_crit.push_back(yt);
                        }
                    }
                }
            }
        }
        if (!xs_crit.empty()) {
            auto [it_xmin, it_xmax] = std::minmax_element(xs_crit.begin(), xs_crit.end());
            x_min = std::floor((*it_xmin - 6.0) / 5.0) * 5.0;
            x_max = std::max(0.0, std::ceil((*it_xmax + 6.0) / 5.0) * 5.0);
        } else {
            x_min = -40.0;
            x_max = 0.0;
        }
        if (!ys_crit.empty()) {
            auto [it_ymin, it_ymax] = std::minmax_element(ys_crit.begin(), ys_crit.end());
            y_min = std::floor((*it_ymin - 6.0) / 5.0) * 5.0;
            y_max = std::max(0.0, std::ceil((*it_ymax + 6.0) / 5.0) * 5.0);
        } else {
            y_min = -40.0;
            y_max = 0.0;
        }
    }

    res.x_min_usado = x_min;
    res.x_max_usado = x_max;
    res.y_min_usado = y_min;
    res.y_max_usado = y_max;

    // Caja envolvente exacta del gráfico visible para que los centroides queden dentro del área visible
    const std::vector<Vec2d> bbox = {
        {x_min, y_min}, {x_max, y_min}, {x_max, y_max}, {x_min, y_max}
    };

    for (size_t i = 0; i < planos.size(); ++i) {
        auto poly = bbox;
        const auto& pi = planos[i];
        for (size_t j = 0; j < planos.size(); ++j) {
            if (i == j) continue;
            const auto& pj = planos[j];
            poly = clip_polygon_halfplane(poly, pi.A - pj.A, pi.B - pj.B, pi.Z - pj.Z);
            if (poly.empty()) break;
        }
        if (poly.size() >= 3) {
            domain::PoligonoPredominancia pf;
            pf.id = pi.id;
            pf.formula = pi.formula;
            pf.etiqueta_grafico = format_clean_plot_label(pi.formula);
            pf.nombre = pi.nombre;
            pf.color_index = static_cast<int>(i);
            pf.nu_x_norm = pi.nu_x_norm;
            pf.nu_y_norm = pi.nu_y_norm;
            pf.Z = pi.Z;
            pf.A = pi.A;
            pf.B = pi.B;
            for (const auto& v : poly) {
                pf.vertices.push_back({v.x, v.y});
            }
            calcular_centroide_y_area(pf);
            if (pf.area > 1e-6) {
                res.poligonos.push_back(pf);
            }
        }
    }

    // Extraer segmentos exactos de frontera entre cada par de fases activas y puntos triples
    for (size_t i = 0; i < res.poligonos.size(); ++i) {
        for (size_t j = i + 1; j < res.poligonos.size(); ++j) {
            const auto& pa = res.poligonos[i];
            const auto& pb = res.poligonos[j];
            const double dA = pa.A - pb.A;
            const double dB = pa.B - pb.B;
            const double dZ = pa.Z - pb.Z;
            const double norm = std::hypot(dA, dB);
            if (norm < 1e-9) continue;

            // Buscar vértices compartidos sobre la recta dA*x + dB*y + dZ = 0
            std::vector<Vec2d> pts_en_recta;
            for (const auto& v : pa.vertices) {
                if (std::abs(dA * v.x + dB * v.y + dZ) / norm < 1e-4) {
                    pts_en_recta.push_back({v.x, v.y});
                }
            }
            if (pts_en_recta.size() >= 2) {
                // Ordenar a lo largo de la recta y tomar los dos extremos
                std::sort(pts_en_recta.begin(), pts_en_recta.end(), [](const Vec2d& u, const Vec2d& v) {
                    return (std::abs(u.x - v.x) > 1e-5) ? (u.x < v.x) : (u.y < v.y);
                });
                const Vec2d p1 = pts_en_recta.front();
                const Vec2d p2 = pts_en_recta.back();
                if (std::hypot(p2.x - p1.x, p2.y - p1.y) > 1e-3) {
                    domain::LineaFronteraLpp lf;
                    lf.fase_a = pa.formula;
                    lf.fase_b = pb.formula;
                    const auto* cpa = &planos[pa.color_index];
                    const auto* cpb = &planos[pb.color_index];
                    lf.ecuacion_reaccion = construir_ecuacion_frontera(
                        cpa->formula, cpa->n_m, cpa->nu_x, cpa->nu_y,
                        cpb->formula, cpb->n_m, cpb->nu_x, cpb->nu_y,
                        res.gas_x, res.gas_y);
                    lf.delta_G_rxn_kJ = cpa->n_m * cpb->n_m * (pb.Z - pa.Z);
                    lf.log10_K = -lf.delta_G_rxn_kJ / RT_ln10;
                    lf.p1 = {p1.x, p1.y};
                    lf.p2 = {p2.x, p2.y};
                    lf.pendiente = (std::abs(p2.x - p1.x) > 1e-6) ? ((p2.y - p1.y) / (p2.x - p1.x)) : 1e9;
                    res.fronteras.push_back(lf);
                }
            }
        }
    }

    // Puntos triples dentro de la ventana visible
    for (size_t i = 0; i < res.poligonos.size(); ++i) {
        for (size_t j = i + 1; j < res.poligonos.size(); ++j) {
            for (size_t k = j + 1; k < res.poligonos.size(); ++k) {
                const auto& pi = res.poligonos[i];
                const auto& pj = res.poligonos[j];
                const auto& pk = res.poligonos[k];
                const double a1 = pi.A - pj.A;
                const double b1 = pi.B - pj.B;
                const double c1 = -(pi.Z - pj.Z);
                const double a2 = pi.A - pk.A;
                const double b2 = pi.B - pk.B;
                const double c2 = -(pi.Z - pk.Z);
                const double det = a1 * b2 - a2 * b1;
                if (std::abs(det) > 1e-9) {
                    const double xt = (c1 * b2 - c2 * b1) / det;
                    const double yt = (a1 * c2 - a2 * c1) / det;
                    if (xt >= x_min - 1e-4 && xt <= x_max + 1e-4 &&
                        yt >= y_min - 1e-4 && yt <= y_max + 1e-4) {
                        const double gt = pi.Z + pi.A * xt + pi.B * yt;
                        bool estable = true;
                        for (const auto& pm : res.poligonos) {
                            if (pm.Z + pm.A * xt + pm.B * yt < gt - 1e-4) {
                                estable = false;
                                break;
                            }
                        }
                        if (estable) {
                            res.puntos_triples.push_back({pi.formula, pj.formula, pk.formula, xt, yt});
                        }
                    }
                }
            }
        }
    }

    res.ok = !res.poligonos.empty();
    if (!res.ok) {
        res.mensaje_error = "No se encontraron regiones de estabilidad dentro del rango especificado.";
    }
    return res;
}

domain::ResultadoDiagramaTpp StabilityDiagrams::calcular_tpp(
    const domain::ParametrosCalculoTpp& params) const {

    domain::ResultadoDiagramaTpp res;
    if (params.elementos.size() != 3) {
        res.mensaje_error = "Seleccione 3 elementos para el diagrama de predominancia Tpp.";
        return res;
    }
    if (params.x_min >= params.x_max || params.y_min >= params.y_max) {
        res.mensaje_error = "El valor mínimo del eje debe ser estrictamente menor que el máximo.";
        return res;
    }

    const auto sis = analizar_sistema_ternario(params.elementos, true, params.incluir_organicos,
                                               params.elemento_principal, true);
    if (sis.elemento_principal.empty()) {
        res.mensaje_error = "No se pudo identificar el elemento principal del sistema.";
        return res;
    }
    res.elemento_principal = sis.elemento_principal;
    res.x_min = params.x_min;
    res.x_max = params.x_max;
    res.y_min = params.y_min;
    res.y_max = params.y_max;

    // ── Caso 1: Modo isotérmico p-p (log pX vs log pY a Temperatura fija) ────
    if (!params.eje_x_es_temperatura) {
        res.modo = domain::StabilityDiagramMode::Tpp_PartialPressure;
        const double T_K = params.temp_en_celsius ? (params.valor_fijo + 273.15) : params.valor_fijo;

        domain::ParametrosCalculoLpp lpp_p;
        lpp_p.elementos = params.elementos;
        lpp_p.elemento_principal = sis.elemento_principal;
        lpp_p.gas_x = params.gas_x;
        lpp_p.gas_y = params.gas_y;
        lpp_p.formulas_seleccionadas = params.formulas_seleccionadas;
        lpp_p.T_K = T_K;
        lpp_p.auto_escala = false;
        lpp_p.x_min = params.x_min;
        lpp_p.x_max = params.x_max;
        lpp_p.y_min = params.y_min;
        lpp_p.y_max = params.y_max;
        lpp_p.incluir_liquidos = true;
        lpp_p.incluir_organicos = params.incluir_organicos;
        lpp_p.omitir_estequiometria_identica = true;

        auto lpp_res = calcular_lpp(lpp_p);
        if (!lpp_res.ok) {
            res.mensaje_error = lpp_res.mensaje_error;
            return res;
        }

        res.titulo_diagrama = "Predominance Diagram for " + sis.elemento_principal + "-" +
                              sis.elem_aux1 + "-" + sis.elem_aux2 + " System";
        std::ostringstream sub;
        sub << "Constant value: Temperature = " << std::fixed << std::setprecision(0)
            << params.valor_fijo << (params.temp_en_celsius ? " °C" : " K");
        res.subtitulo_constante = sub.str();
        res.etiqueta_eje_x = "log p" + format_gas_formula(params.gas_x);
        res.etiqueta_eje_y = "log p" + format_gas_formula(params.gas_y);
        res.poligonos = lpp_res.poligonos;
        res.fronteras = lpp_res.fronteras;
        res.advertencias = lpp_res.advertencias;

        // Poblar malla mediante el algoritmo recursivo quadtree de energía de Gibbs mínima
        const int N = 1 << std::clamp(params.profundidad_recursiva, 4, 8);
        res.grid_cols = N;
        res.grid_rows = N;
        res.grid_phase_idx.assign(N * N, -1);

        auto eval_cell = [&](int c, int r) -> int {
            ++res.evaluaciones_gibbs;
            const double x = params.x_min + (c + 0.5) * (params.x_max - params.x_min) / N;
            const double y = params.y_min + (r + 0.5) * (params.y_max - params.y_min) / N;
            double best_g = 1e30;
            int best_idx = 0;
            for (size_t k = 0; k < res.poligonos.size(); ++k) {
                const double g = res.poligonos[k].Z + res.poligonos[k].A * x + res.poligonos[k].B * y;
                if (g < best_g) {
                    best_g = g;
                    best_idx = static_cast<int>(k);
                }
            }
            return best_idx;
        };

        auto subdividir = [&](auto& self, int c0, int r0, int c1, int r1) -> void {
            if (c0 > c1 || r0 > r1) return;
            if (c0 == c1 && r0 == r1) {
                res.grid_phase_idx[r0 * N + c0] = eval_cell(c0, r0);
                return;
            }
            const int cm = (c0 + c1) / 2;
            const int rm = (r0 + r1) / 2;
            const int p00 = eval_cell(c0, r0);
            const int p10 = eval_cell(c1, r0);
            const int p01 = eval_cell(c0, r1);
            const int p11 = eval_cell(c1, r1);
            const int pmm = eval_cell(cm, rm);
            if (p00 == p10 && p00 == p01 && p00 == p11 && p00 == pmm && (c1 - c0) <= 8) {
                for (int r = r0; r <= r1; ++r)
                    for (int c = c0; c <= c1; ++c)
                        res.grid_phase_idx[r * N + c] = p00;
                return;
            }
            self(self, c0, r0, cm, rm);
            self(self, cm + 1, r0, c1, rm);
            self(self, c0, rm + 1, cm, r1);
            self(self, cm + 1, rm + 1, c1, r1);
        };

        subdividir(subdividir, 0, 0, N - 1, N - 1);
        res.ok = true;
        return res;
    }

    // ── Caso 2: Modo T-p (Temperatura en eje X, log p_Y en eje Y, p_fijo constante) ──
    res.modo = domain::StabilityDiagramMode::Tpp_TempPressure;
    const std::string gas_y_fmt = format_gas_formula(params.gas_y);
    const std::string gas_fix_fmt = format_gas_formula(params.gas_fijo);

    const auto comp_gy = parsear_composicion_fraccionaria(gas_y_fmt);
    const auto comp_gfix = parsear_composicion_fraccionaria(gas_fix_fmt);
    if (comp_gy.count(sis.elemento_principal) || comp_gfix.count(sis.elemento_principal)) {
        res.mensaje_error = "Las especies gaseosas no pueden contener el elemento principal (" +
                            sis.elemento_principal + ").";
        return res;
    }

    // Validación HSC: el gas de presión fija no debe tener más elementos que el gas del eje Y
    if (comp_gfix.size() > comp_gy.size()) {
        res.advertencias.push_back("HSC T-p note: fixed gas (" + gas_fix_fmt +
                                   ") contains more elements than Y-axis gas (" + gas_y_fmt + ").");
    }

    double test_ny = 0.0, test_nfix = 0.0;
    std::map<std::string, double> test_sp = {{sis.elem_aux1, 1.0}, {sis.elem_aux2, 1.0}};
    if (!resolver_estequiometria_gases(sis.elem_aux1, sis.elem_aux2, comp_gy, comp_gfix,
                                       test_sp, test_ny, test_nfix)) {
        res.mensaje_error = "El gas del eje Y (" + gas_y_fmt + ") y el gas fijo (" +
                            gas_fix_fmt + ") deben ser linealmente independientes.";
        return res;
    }

    const double log10_p_fix = (params.presion_fija_en_bar && params.valor_fijo > 0.0)
                                   ? std::log10(params.valor_fijo)
                                   : params.valor_fijo;
    const double p_fix_bar = std::pow(10.0, log10_p_fix);

    res.titulo_diagrama = "Predominance Diagram for " + sis.elem_aux1 + "-" +
                          sis.elemento_principal + "-" + sis.elem_aux2 + " System";
    {
        std::ostringstream sub;
        sub << "Constant value: p" << gas_fix_fmt << " = ";
        if (p_fix_bar >= 1e-3 && p_fix_bar <= 1e4) {
            sub << std::defaultfloat << p_fix_bar << " bar";
        } else {
            sub << std::scientific << std::setprecision(1) << p_fix_bar << " bar";
        }
        res.subtitulo_constante = sub.str();
    }
    res.etiqueta_eje_x = params.temp_en_celsius ? "T / °C" : "T / K";
    res.etiqueta_eje_y = "log p" + gas_y_fmt;

    const auto catalogo = recolectar_catalogo_combinado(repo_);
    const auto* sp_gy = buscar_especie_catalogo(catalogo, gas_y_fmt, true);
    const auto* sp_gfix = buscar_especie_catalogo(catalogo, gas_fix_fmt, true);
    const auto* sp_main = buscar_especie_catalogo(catalogo, sis.elemento_principal, false);

    std::set<std::string> sel_set(params.formulas_seleccionadas.begin(), params.formulas_seleccionadas.end());
    const bool usar_todas = sel_set.empty();

    // Agrupar polimorfos con la misma fórmula limpia (ej. Cu, Cu(l), Cu(FCC) -> "Cu")
    struct GrupoFaseTP {
        std::string etiqueta_base;
        double nu_y_norm = 0.0;
        double nu_fix_norm = 0.0;
        std::vector<std::pair<const nucleo::Especie*, double>> polimorfos; // (especie, n_main)
    };
    std::vector<GrupoFaseTP> grupos;

    for (const auto& cand : sis.especies_sistema) {
        if (!cand.contiene_elemento_principal || cand.n_main <= 0.0) continue;
        if (usar_todas ? !cand.seleccionada_por_defecto : !sel_set.count(cand.formula)) continue;
        const auto* sp = buscar_especie_catalogo(catalogo, cand.formula, false);
        if (!sp) continue;

        double nu_y = 0.0, nu_fix = 0.0;
        if (!resolver_estequiometria_gases(sis.elem_aux1, sis.elem_aux2, comp_gy, comp_gfix,
                                           cand.composicion, nu_y, nu_fix)) {
            continue;
        }
        const double ny_n = nu_y / cand.n_main;
        const double nfix_n = nu_fix / cand.n_main;
        const std::string base_lbl = strip_phase_or_polymorph_tag(cand.formula);

        bool agregado = false;
        for (auto& g : grupos) {
            if (std::abs(g.nu_y_norm - ny_n) < 1e-6 && std::abs(g.nu_fix_norm - nfix_n) < 1e-6) {
                g.polimorfos.push_back({sp, cand.n_main});
                agregado = true;
                break;
            }
        }
        if (!agregado) {
            GrupoFaseTP ng;
            ng.etiqueta_base = base_lbl;
            ng.nu_y_norm = ny_n;
            ng.nu_fix_norm = nfix_n;
            ng.polimorfos.push_back({sp, cand.n_main});
            grupos.push_back(ng);
        }
    }

    if (grupos.empty()) {
        res.mensaje_error = "Seleccione al menos una especie con el elemento principal (" +
                            sis.elemento_principal + ").";
        return res;
    }

    // En cualquier temperatura T_K, G_norm_k(T_K, y) = Z_k(T_K) + B_k(T_K) * y es lineal en y = log10(p_Y)
    auto evaluar_ZB_en_T = [&](double x_val, std::vector<double>& Z_out, std::vector<double>& B_out) {
        const double T_K = std::max(100.0, params.temp_en_celsius ? (x_val + 273.15) : x_val);
        const double RT_ln10 = kR_kJ * T_K * kLn10;
        const double mu_gy0 = sp_gy ? sp_gy->energia_gibbs(T_K) : 0.0;
        const double mu_gfix = (sp_gfix ? sp_gfix->energia_gibbs(T_K) : 0.0) + RT_ln10 * log10_p_fix;
        const double g_main = sp_main ? sp_main->energia_gibbs(T_K) : 0.0;

        Z_out.resize(grupos.size());
        B_out.resize(grupos.size());
        for (size_t k = 0; k < grupos.size(); ++k) {
            double best_g_sp = 1e30;
            for (const auto& [sp_ptr, n_m] : grupos[k].polimorfos) {
                const double g_val = sp_ptr->energia_gibbs(T_K) / n_m;
                if (g_val < best_g_sp) best_g_sp = g_val;
            }
            Z_out[k] = best_g_sp - g_main - grupos[k].nu_fix_norm * mu_gfix - grupos[k].nu_y_norm * mu_gy0;
            B_out[k] = -grupos[k].nu_y_norm * RT_ln10;
        }
    };

    // Envolvente inferior 1D exacta en y in [y_min, y_max] para cada corte de temperatura x_m
    const int n_slices = 160;
    struct IntervaloY {
        double y_bot = 0.0;
        double y_top = 0.0;
        bool activo = false;
    };
    std::vector<double> xs(n_slices + 1);
    std::vector<std::vector<IntervaloY>> slices(n_slices + 1, std::vector<IntervaloY>(grupos.size()));

    for (int m = 0; m <= n_slices; ++m) {
        const double x_val = params.x_min + m * (params.x_max - params.x_min) / n_slices;
        xs[m] = x_val;
        std::vector<double> Zs, Bs;
        evaluar_ZB_en_T(x_val, Zs, Bs);

        for (size_t k = 0; k < grupos.size(); ++k) {
            double low = params.y_min;
            double high = params.y_max;
            for (size_t j = 0; j < grupos.size(); ++j) {
                if (k == j) continue;
                // Condición: (B_k - B_j)*y + (Z_k - Z_j) <= 0
                const double dB = Bs[k] - Bs[j];
                const double dZ = Zs[k] - Zs[j];
                if (std::abs(dB) < 1e-11) {
                    if (dZ > 0.0) {
                        low = 1.0;
                        high = 0.0;
                        break;
                    }
                } else {
                    const double y_int = -dZ / dB;
                    if (dB > 0.0) {
                        high = std::min(high, y_int);
                    } else {
                        low = std::max(low, y_int);
                    }
                }
            }
            if (high > low + 1e-6) {
                slices[m][k] = {low, high, true};
            }
        }
    }

    // Construir polígonos, trapecios convexos y curvas de frontera para cada fase activa
    for (size_t k = 0; k < grupos.size(); ++k) {
        domain::PoligonoPredominancia pf;
        pf.id = static_cast<int64_t>(k + 1);
        pf.formula = grupos[k].etiqueta_base;
        pf.etiqueta_grafico = grupos[k].etiqueta_base;
        pf.nombre = grupos[k].polimorfos.front().first->nombre;
        pf.color_index = static_cast<int>(k);

        std::vector<domain::VerticeEstabilidad2D> lower_pts, upper_pts;
        double sum_area = 0.0;

        for (int m = 0; m < n_slices; ++m) {
            const auto& s0 = slices[m][k];
            const auto& s1 = slices[m + 1][k];
            if (s0.activo && s1.activo) {
                domain::TrapecioEstabilidad2D trap{xs[m], xs[m + 1], s0.y_bot, s0.y_top, s1.y_bot, s1.y_top};
                pf.trapecios.push_back(trap);
                const double dx = xs[m + 1] - xs[m];
                const double h0 = s0.y_top - s0.y_bot;
                const double h1 = s1.y_top - s1.y_bot;
                sum_area += 0.5 * (h0 + h1) * dx;
            }
        }
        for (int m = 0; m <= n_slices; ++m) {
            if (slices[m][k].activo) {
                lower_pts.push_back({xs[m], slices[m][k].y_bot});
                upper_pts.push_back({xs[m], slices[m][k].y_top});
            }
        }
        if (!lower_pts.empty() && sum_area > 1e-5) {
            pf.vertices = lower_pts;
            for (auto it = upper_pts.rbegin(); it != upper_pts.rend(); ++it) {
                pf.vertices.push_back(*it);
            }
            pf.area = sum_area;
            // Ubicar la etiqueta sobre la lonja central más representativa de la banda
            const size_t mid_idx = lower_pts.size() / 2;
            pf.centroide = {
                lower_pts[mid_idx].x,
                0.5 * (lower_pts[mid_idx].y + upper_pts[mid_idx].y)
            };
            res.poligonos.push_back(pf);
        }
    }

    // Malla recursiva quadtree para diagnóstico y consulta rápida
    const int N = 1 << std::clamp(params.profundidad_recursiva, 4, 8);
    res.grid_cols = N;
    res.grid_rows = N;
    res.grid_phase_idx.assign(N * N, 0);
    res.evaluaciones_gibbs = static_cast<int>((n_slices + 1) * grupos.size());
    res.ok = !res.poligonos.empty();
    return res;
}

std::string StabilityDiagrams::evaluar_punto_cursor_tpp(
    const domain::ParametrosCalculoTpp& params,
    double x, double y,
    double* out_g_norm_kJ) const {

    const auto sis = analizar_sistema_ternario(params.elementos, true, params.incluir_organicos,
                                               params.elemento_principal, false);
    if (sis.elemento_principal.empty()) return "";

    const auto catalogo = recolectar_catalogo_combinado(repo_);
    double T_K = 298.15;
    std::string gas1_fmt, gas2_fmt;
    double logp1 = 0.0, logp2 = 0.0;

    if (!params.eje_x_es_temperatura) {
        T_K = params.temp_en_celsius ? (params.valor_fijo + 273.15) : params.valor_fijo;
        gas1_fmt = format_gas_formula(params.gas_x);
        gas2_fmt = format_gas_formula(params.gas_y);
        logp1 = x;
        logp2 = y;
    } else {
        T_K = params.temp_en_celsius ? (x + 273.15) : x;
        gas1_fmt = format_gas_formula(params.gas_y);
        gas2_fmt = format_gas_formula(params.gas_fijo);
        logp1 = y;
        logp2 = (params.presion_fija_en_bar && params.valor_fijo > 0.0)
                    ? std::log10(params.valor_fijo)
                    : params.valor_fijo;
    }
    T_K = std::max(100.0, T_K);
    const double RT_ln10 = kR_kJ * T_K * kLn10;

    const auto comp_g1 = parsear_composicion_fraccionaria(gas1_fmt);
    const auto comp_g2 = parsear_composicion_fraccionaria(gas2_fmt);
    const auto* sp_g1 = buscar_especie_catalogo(catalogo, gas1_fmt, true);
    const auto* sp_g2 = buscar_especie_catalogo(catalogo, gas2_fmt, true);
    const auto* sp_main = buscar_especie_catalogo(catalogo, sis.elemento_principal, false);

    const double mu_g1 = (sp_g1 ? sp_g1->energia_gibbs(T_K) : 0.0) + RT_ln10 * logp1;
    const double mu_g2 = (sp_g2 ? sp_g2->energia_gibbs(T_K) : 0.0) + RT_ln10 * logp2;
    const double g_main = sp_main ? sp_main->energia_gibbs(T_K) : 0.0;

    std::set<std::string> sel_set(params.formulas_seleccionadas.begin(), params.formulas_seleccionadas.end());
    const bool usar_todas = sel_set.empty();

    double best_g = 1e30;
    std::string best_formula;
    for (const auto& cand : sis.especies_sistema) {
        if (!cand.contiene_elemento_principal || cand.n_main <= 0.0) continue;
        if (usar_todas ? !cand.seleccionada_por_defecto : !sel_set.count(cand.formula)) continue;
        const auto* sp = buscar_especie_catalogo(catalogo, cand.formula, false);
        if (!sp) continue;
        double nu1 = 0.0, nu2 = 0.0;
        if (!resolver_estequiometria_gases(sis.elem_aux1, sis.elem_aux2, comp_g1, comp_g2,
                                           cand.composicion, nu1, nu2)) {
            continue;
        }
        const double g_norm = (sp->energia_gibbs(T_K) - cand.n_main * g_main - nu1 * mu_g1 - nu2 * mu_g2) / cand.n_main;
        if (g_norm < best_g) {
            best_g = g_norm;
            best_formula = cand.formula;
        }
    }
    if (out_g_norm_kJ) *out_g_norm_kJ = best_g;
    return best_formula;
}

std::string StabilityDiagrams::exportar_tabla_lpp_csv(const domain::ResultadoDiagramaLpp& res) const {
    std::ostringstream os;
    os << "# HSC Lpp Phase Stability Diagram - Vector Boundary & Invariant Point Report\n";
    os << "# System: " << res.titulo_sistema << " | T = " << std::fixed << std::setprecision(2)
       << (res.T_K - 273.15) << " C (" << res.T_K << " K)\n";
    os << "# X-Axis: log p" << res.gas_x << " | Y-Axis: log p" << res.gas_y << "\n\n";
    os << "Phase A,Phase B,Equilibrium Reaction,Delta G0 (kJ),log10 K,X1,Y1,X2,Y2,Slope (dY/dX)\n";
    for (const auto& f : res.fronteras) {
        os << f.fase_a << "," << f.fase_b << ",\"" << f.ecuacion_reaccion << "\","
           << std::fixed << std::setprecision(3) << f.delta_G_rxn_kJ << ","
           << f.log10_K << ","
           << f.p1.x << "," << f.p1.y << ","
           << f.p2.x << "," << f.p2.y << ","
           << f.pendiente << "\n";
    }
    if (!res.puntos_triples.empty()) {
        os << "\n# Invariant Triple Points\n";
        os << "Phase 1,Phase 2,Phase 3,log p" << res.gas_x << ",log p" << res.gas_y << "\n";
        for (const auto& pt : res.puntos_triples) {
            os << pt.fase_1 << "," << pt.fase_2 << "," << pt.fase_3 << ","
               << std::fixed << std::setprecision(3) << pt.x << "," << pt.y << "\n";
        }
    }
    return os.str();
}

std::string StabilityDiagrams::serializar_tpp(const domain::ParametrosCalculoTpp& p) const {
    std::ostringstream os;
    os << "HSC_TPP8_SPEC\n";
    os << "ELEMENTS=" << (p.elementos.size() > 0 ? p.elementos[0] : "") << ","
       << (p.elementos.size() > 1 ? p.elementos[1] : "") << ","
       << (p.elementos.size() > 2 ? p.elementos[2] : "") << "\n";
    os << "MAIN=" << p.elemento_principal << "\n";
    os << "X_TEMP=" << (p.eje_x_es_temperatura ? 1 : 0) << "\n";
    os << "CELSIUS=" << (p.temp_en_celsius ? 1 : 0) << "\n";
    os << "GAS_X=" << p.gas_x << "\n";
    os << "GAS_Y=" << p.gas_y << "\n";
    os << "GAS_FIXED=" << p.gas_fijo << "\n";
    os << "X_MIN=" << p.x_min << "\n";
    os << "X_MAX=" << p.x_max << "\n";
    os << "Y_MIN=" << p.y_min << "\n";
    os << "Y_MAX=" << p.y_max << "\n";
    os << "FIXED_VAL=" << p.valor_fijo << "\n";
    os << "ORGANICS=" << (p.incluir_organicos ? 1 : 0) << "\n";
    os << "SPECIES=";
    for (size_t i = 0; i < p.formulas_seleccionadas.size(); ++i) {
        if (i > 0) os << ";";
        os << p.formulas_seleccionadas[i];
    }
    os << "\n";
    return os.str();
}

bool StabilityDiagrams::deserializar_tpp(const std::string& contenido, domain::ParametrosCalculoTpp& out) const {
    if (contenido.find("HSC_TPP8_SPEC") == std::string::npos) return false;
    std::istringstream is(contenido);
    std::string line;
    while (std::getline(is, line)) {
        const size_t eq = line.find('=');
        if (eq == std::string::npos) continue;
        const std::string k = line.substr(0, eq);
        const std::string v = line.substr(eq + 1);
        if (k == "ELEMENTS") {
            out.elementos.clear();
            std::istringstream es(v);
            std::string tok;
            while (std::getline(es, tok, ',')) if (!tok.empty()) out.elementos.push_back(tok);
        } else if (k == "MAIN") out.elemento_principal = v;
        else if (k == "X_TEMP") out.eje_x_es_temperatura = (v == "1");
        else if (k == "CELSIUS") out.temp_en_celsius = (v == "1");
        else if (k == "GAS_X") out.gas_x = v;
        else if (k == "GAS_Y") out.gas_y = v;
        else if (k == "GAS_FIXED") out.gas_fijo = v;
        else if (k == "X_MIN") out.x_min = std::stod(v);
        else if (k == "X_MAX") out.x_max = std::stod(v);
        else if (k == "Y_MIN") out.y_min = std::stod(v);
        else if (k == "Y_MAX") out.y_max = std::stod(v);
        else if (k == "FIXED_VAL") out.valor_fijo = std::stod(v);
        else if (k == "ORGANICS") out.incluir_organicos = (v == "1");
        else if (k == "SPECIES") {
            out.formulas_seleccionadas.clear();
            std::istringstream ss(v);
            std::string tok;
            while (std::getline(ss, tok, ';')) if (!tok.empty()) out.formulas_seleccionadas.push_back(tok);
        }
    }
    return out.elementos.size() == 3;
}

std::string StabilityDiagrams::serializar_lpp(const domain::ParametrosCalculoLpp& p) const {
    std::ostringstream os;
    os << "HSC_LPP8_SPEC\n";
    os << "ELEMENTS=" << (p.elementos.size() > 0 ? p.elementos[0] : "") << ","
       << (p.elementos.size() > 1 ? p.elementos[1] : "") << ","
       << (p.elementos.size() > 2 ? p.elementos[2] : "") << "\n";
    os << "MAIN=" << p.elemento_principal << "\n";
    os << "GAS_X=" << p.gas_x << "\n";
    os << "GAS_Y=" << p.gas_y << "\n";
    os << "T_K=" << p.T_K << "\n";
    os << "AUTO_SCALE=" << (p.auto_escala ? 1 : 0) << "\n";
    os << "X_MIN=" << p.x_min << "\n";
    os << "X_MAX=" << p.x_max << "\n";
    os << "Y_MIN=" << p.y_min << "\n";
    os << "Y_MAX=" << p.y_max << "\n";
    os << "LIQUIDS=" << (p.incluir_liquidos ? 1 : 0) << "\n";
    os << "ORGANICS=" << (p.incluir_organicos ? 1 : 0) << "\n";
    os << "SPECIES=";
    for (size_t i = 0; i < p.formulas_seleccionadas.size(); ++i) {
        if (i > 0) os << ";";
        os << p.formulas_seleccionadas[i];
    }
    os << "\n";
    return os.str();
}

bool StabilityDiagrams::deserializar_lpp(const std::string& contenido, domain::ParametrosCalculoLpp& out) const {
    if (contenido.find("HSC_LPP8_SPEC") == std::string::npos) return false;
    std::istringstream is(contenido);
    std::string line;
    while (std::getline(is, line)) {
        const size_t eq = line.find('=');
        if (eq == std::string::npos) continue;
        const std::string k = line.substr(0, eq);
        const std::string v = line.substr(eq + 1);
        if (k == "ELEMENTS") {
            out.elementos.clear();
            std::istringstream es(v);
            std::string tok;
            while (std::getline(es, tok, ',')) if (!tok.empty()) out.elementos.push_back(tok);
        } else if (k == "MAIN") out.elemento_principal = v;
        else if (k == "GAS_X") out.gas_x = v;
        else if (k == "GAS_Y") out.gas_y = v;
        else if (k == "T_K") out.T_K = std::stod(v);
        else if (k == "AUTO_SCALE") out.auto_escala = (v == "1");
        else if (k == "X_MIN") out.x_min = std::stod(v);
        else if (k == "X_MAX") out.x_max = std::stod(v);
        else if (k == "Y_MIN") out.y_min = std::stod(v);
        else if (k == "Y_MAX") out.y_max = std::stod(v);
        else if (k == "LIQUIDS") out.incluir_liquidos = (v == "1");
        else if (k == "ORGANICS") out.incluir_organicos = (v == "1");
        else if (k == "SPECIES") {
            out.formulas_seleccionadas.clear();
            std::istringstream ss(v);
            std::string tok;
            while (std::getline(ss, tok, ';')) if (!tok.empty()) out.formulas_seleccionadas.push_back(tok);
        }
    }
    return out.elementos.size() == 3;
}

} // namespace cma::application::modules

#pragma GCC diagnostic pop
