#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"

#include "pourbaix.hpp"
#include "PourbaixCatalogHelpers.hpp"

#include <algorithm>
#include <cmath>
#include <iomanip>
#include <map>
#include <set>
#include <sstream>

namespace cma { namespace motores {

using namespace detail_eph;

namespace {

struct PlanoLinealEpH {
    int64_t id = 0;
    std::string formula;
    std::string nombre;
    int estado = 2;
    int carga = 0;
    double n_main = 1.0;
    double n_O_norm = 0.0;
    double n_H_norm = 0.0;
    double n_Hplus_norm = 0.0;
    double n_e_norm = 0.0;
    double Z = 0.0; // kJ / mol M
    double A = 0.0; // kJ / (mol M · pH)
    double B = 0.0; // kJ / (mol M · V)
};

domain::PourbaixCorrosionState clasificar_estado_corrosion(const domain::EpHCandidateSpecies& sp) {
    if (sp.estado == 3 || sp.estado == 4 || sp.estado == 5) {
        return domain::PourbaixCorrosionState::Corrosion;
    }
    if (sp.composicion.size() == 1 && sp.carga == 0 && sp.estado == 2) {
        return domain::PourbaixCorrosionState::Immunity;
    }
    return domain::PourbaixCorrosionState::Passivation;
}

std::string construir_ecuacion_redox(const std::string& fa, const std::string& fb,
                                     double dn_Hplus, double dn_e) {
    std::ostringstream os;
    os << fa;
    if (dn_Hplus > 1e-3) {
        os << " + " << std::fixed << std::setprecision(2) << dn_Hplus << " H(+a)";
    }
    if (dn_e > 1e-3) {
        os << " + " << std::fixed << std::setprecision(2) << dn_e << " e-";
    }
    os << " = " << fb;
    if (dn_Hplus < -1e-3) {
        os << " + " << std::fixed << std::setprecision(2) << (-dn_Hplus) << " H(+a)";
    }
    if (dn_e < -1e-3) {
        os << " + " << std::fixed << std::setprecision(2) << (-dn_e) << " e-";
    }
    return os.str();
}

void extraer_poligonos_y_fronteras_planos(
    const std::vector<PlanoLinealEpH>& planos,
    const std::vector<domain::EpHCandidateSpecies>& catalogo_ref,
    double pH_min, double pH_max,
    double Eh_min, double Eh_max,
    bool es_solo_iones,
    std::vector<domain::EpHPoligonoPredominancia>& out_poligonos,
    std::vector<domain::EpHSegmentoFrontera>& out_fronteras,
    std::vector<domain::EpHPuntoTriple>* out_triples = nullptr) {

    const std::vector<domain::EpHVertice2D> bbox = {
        {pH_min, Eh_min}, {pH_max, Eh_min}, {pH_max, Eh_max}, {pH_min, Eh_max}
    };

    // Filtrar polimorfos con idéntica pendiente (A, B) conservando el de menor Z
    std::vector<PlanoLinealEpH> unicos;
    for (const auto& p : planos) {
        bool dup = false;
        for (auto& ex : unicos) {
            if (std::abs(ex.A - p.A) < 1e-5 && std::abs(ex.B - p.B) < 1e-5) {
                dup = true;
                if (p.Z < ex.Z) ex = p;
                break;
            }
        }
        if (!dup) unicos.push_back(p);
    }

    int area_counter = static_cast<int>(out_poligonos.size()) + 1;
    std::vector<size_t> indices_planos_activos;

    for (size_t i = 0; i < unicos.size(); ++i) {
        auto poly = bbox;
        const auto& pi = unicos[i];
        for (size_t j = 0; j < unicos.size(); ++j) {
            if (i == j) continue;
            const auto& pj = unicos[j];
            poly = clip_polygon_eph(poly, pi.A - pj.A, pi.B - pj.B, pi.Z - pj.Z);
            if (poly.empty()) break;
        }
        if (poly.size() >= 3) {
            domain::EpHPoligonoPredominancia pf;
            pf.id = pi.id;
            pf.area_no = area_counter;
            pf.formula = pi.formula;
            pf.etiqueta = pi.formula;
            pf.nombre = pi.nombre;
            pf.es_acuoso = (pi.estado == 3 || pi.estado == 4);
            pf.carga = pi.carga;
            pf.color_index = static_cast<int>(out_poligonos.size());
            pf.Z = pi.Z;
            pf.A = pi.A;
            pf.B = pi.B;
            pf.vertices = poly;

            for (const auto& sp : catalogo_ref) {
                if (sp.formula == pi.formula) {
                    pf.estado_corrosion = clasificar_estado_corrosion(sp);
                    break;
                }
            }
            calcular_centroide_mas_ancho_eph(pf);
            if (pf.area > 1e-5) {
                out_poligonos.push_back(pf);
                indices_planos_activos.push_back(i);
                ++area_counter;
            }
        }
    }

    // Extraer segmentos exactos de frontera entre pares de polígonos activos
    for (size_t a = 0; a < indices_planos_activos.size(); ++a) {
        for (size_t b = a + 1; b < indices_planos_activos.size(); ++b) {
            const auto& pa = unicos[indices_planos_activos[a]];
            const auto& pb = unicos[indices_planos_activos[b]];
            const auto& poly_a = out_poligonos[out_poligonos.size() - indices_planos_activos.size() + a];

            const double dA = pa.A - pb.A;
            const double dB = pa.B - pb.B;
            const double dZ = pa.Z - pb.Z;
            const double norm = std::hypot(dA, dB);
            if (norm < 1e-8) continue;

            std::vector<domain::EpHVertice2D> pts;
            for (const auto& v : poly_a.vertices) {
                if (std::abs(dA * v.pH + dB * v.Eh + dZ) / norm < 1e-3) {
                    pts.push_back(v);
                }
            }
            if (pts.size() >= 2) {
                std::sort(pts.begin(), pts.end(), [](const domain::EpHVertice2D& u, const domain::EpHVertice2D& v) {
                    return (std::abs(u.pH - v.pH) > 1e-4) ? (u.pH < v.pH) : (u.Eh < v.Eh);
                });
                const auto p1 = pts.front();
                const auto p2 = pts.back();
                if (std::hypot(p2.pH - p1.pH, p2.Eh - p1.Eh) > 1e-3) {
                    domain::EpHSegmentoFrontera seg;
                    seg.area_no = poly_a.area_no;
                    seg.fase_a = pa.formula;
                    seg.fase_b = pb.formula;
                    seg.es_agua = false;
                    seg.es_solo_iones = es_solo_iones;
                    seg.p1 = p1;
                    seg.p2 = p2;
                    seg.dG_rxn_kJ = pb.Z - pa.Z;
                    seg.ecuacion_reaccion = construir_ecuacion_redox(
                        pa.formula, pb.formula, pb.n_Hplus_norm - pa.n_Hplus_norm, pb.n_e_norm - pa.n_e_norm);

                    if (std::abs(dB) < 1e-5) {
                        seg.tipo_linea = 2; // Vertical (solo H+)
                        seg.pendiente = 1e9;
                        seg.intercepto_eh = p1.pH;
                    } else if (std::abs(dA) < 1e-5) {
                        seg.tipo_linea = 0; // Horizontal (solo e-)
                        seg.pendiente = 0.0;
                        seg.intercepto_eh = -dZ / dB;
                    } else {
                        seg.tipo_linea = 1; // Diagonal (H+ y e-)
                        seg.pendiente = -dA / dB;
                        seg.intercepto_eh = -dZ / dB;
                    }
                    out_fronteras.push_back(seg);
                }
            }
        }
    }

    // Extraer puntos triples invariantes
    if (out_triples) {
        const size_t offset = out_poligonos.size() - indices_planos_activos.size();
        for (size_t i = 0; i < indices_planos_activos.size(); ++i) {
            for (size_t j = i + 1; j < indices_planos_activos.size(); ++j) {
                for (size_t k = j + 1; k < indices_planos_activos.size(); ++k) {
                    const auto& pi = out_poligonos[offset + i];
                    const auto& pj = out_poligonos[offset + j];
                    const auto& pk = out_poligonos[offset + k];
                    const double a1 = pi.A - pj.A, b1 = pi.B - pj.B, c1 = -(pi.Z - pj.Z);
                    const double a2 = pi.A - pk.A, b2 = pi.B - pk.B, c2 = -(pi.Z - pk.Z);
                    const double det = a1 * b2 - a2 * b1;
                    if (std::abs(det) > 1e-8) {
                        const double xp = (c1 * b2 - c2 * b1) / det;
                        const double ye = (a1 * c2 - a2 * c1) / det;
                        if (xp >= pH_min - 1e-4 && xp <= pH_max + 1e-4 &&
                            ye >= Eh_min - 1e-4 && ye <= Eh_max + 1e-4) {
                            const double gt = pi.Z + pi.A * xp + pi.B * ye;
                            bool estable = true;
                            for (size_t m = 0; m < indices_planos_activos.size(); ++m) {
                                const auto& pm = out_poligonos[offset + m];
                                if (pm.Z + pm.A * xp + pm.B * ye < gt - 1e-3) {
                                    estable = false;
                                    break;
                                }
                            }
                            if (estable) {
                                out_triples->push_back({pi.formula, pj.formula, pk.formula, xp, ye});
                            }
                        }
                    }
                }
            }
        }
    }
}

} // namespace

void CalculadorPourbaix::establecer_elemento(const std::string& elemento) {
    elemento_ = elemento;
    params_actuales_.elemento_principal = elemento;
    if (params_actuales_.elementos.empty()) {
        params_actuales_.elementos.push_back({elemento, conc_, 1.0});
    } else {
        params_actuales_.elementos[0].symbol = elemento;
    }
}

void CalculadorPourbaix::establecer_temperatura(double T_K) {
    T_K_ = std::clamp(T_K, 250.0, 1000.0);
    params_actuales_.T_K = T_K_;
}

void CalculadorPourbaix::establecer_concentracion(double conc) {
    conc_ = std::max(conc, 1e-20);
    for (auto& el : params_actuales_.elementos) {
        if (el.symbol == elemento_) el.molality = conc_;
    }
}

void CalculadorPourbaix::establecer_escala_potencial(domain::PourbaixPotentialScale escala) {
    escala_ = escala;
    params_actuales_.escala_potencial = escala;
}

double CalculadorPourbaix::pendiente_nernst() const {
    return -(kR_J * T_K_ * kLn10) / kFaraday_C;
}

double CalculadorPourbaix::eh_agua_oxidacion(double pH, double p_O2_bar) const {
    const double dG_H2O_kJ = calcular_dG_agua_kcal(T_K_, 1.0) * kCalToKJ;
    const double E0_ox = -dG_H2O_kJ / (2.0 * kFaraday_kJ_V);
    const double slope = pendiente_nernst();
    const double term_p = -0.25 * slope * std::log10(std::max(p_O2_bar, 1e-20));
    return E0_ox + slope * pH + term_p - offset_escala_potencial(escala_);
}

double CalculadorPourbaix::eh_agua_reduccion(double pH, double p_H2_bar) const {
    const double slope = pendiente_nernst();
    const double term_p = 0.5 * slope * std::log10(std::max(p_H2_bar, 1e-20));
    return 0.0000 + slope * pH + term_p - offset_escala_potencial(escala_);
}

void CalculadorPourbaix::cargar_preset_hsc(int preset_idx) {
    domain::EpHSystemParameters p;
    p.T_K = 298.15;
    p.escala_potencial = escala_;
    p.pH_min = 0.0; p.pH_max = 14.0;
    p.Eh_min = -2.0; p.Eh_max = 2.0;
    p.modos_busqueda.condensed = true;
    p.modos_busqueda.aqueous_ions = true;
    p.modos_busqueda.aqueous_neutrals = true;

    switch (preset_idx) {
        case 0: // Fe - H2O (25 °C, [Fe] = 1e-6 m)
            p.elementos = {{"Fe", 1e-6, 1.0}};
            p.elemento_principal = "Fe";
            break;
        case 1: // Cu - H2O (25 °C, [Cu] = 1e-6 m)
            p.elementos = {{"Cu", 1e-6, 1.0}};
            p.elemento_principal = "Cu";
            break;
        case 2: // Cu - S - H2O (Fig. 1, 2, 3 HSC EpH: Cu principal, 1.0 m)
            p.elementos = {{"Cu", 1.0, 1.0}, {"S", 1.0, 1.0}};
            p.elemento_principal = "Cu";
            p.modos_busqueda.aqueous_neutrals = false;
            break;
        case 3: // S - Cu - H2O (Fig. 4 HSC EpH: S principal, 1.0 m)
            p.elementos = {{"Cu", 1.0, 1.0}, {"S", 1.0, 1.0}};
            p.elemento_principal = "S";
            p.modos_busqueda.aqueous_neutrals = false;
            break;
        case 4: // Au - Cl - H2O (Cloruración ácida: [Au] = 1e-4 m, [Cl] = 1.0 m)
            p.elementos = {{"Au", 1e-4, 1.0}, {"Cl", 1.0, 1.0}};
            p.elemento_principal = "Au";
            break;
        case 5: // Mn - H2O (Sección 17.1 Manual HSC: [Mn] = 1e-4 m)
            p.elementos = {{"Mn", 1e-4, 1.0}};
            p.elemento_principal = "Mn";
            break;
        case 6: // Zn - S - H2O ([Zn] = 1e-3 m, [S] = 1e-2 m)
        default:
            p.elementos = {{"Zn", 1e-3, 1.0}, {"S", 1e-2, 1.0}};
            p.elemento_principal = "Zn";
            break;
    }

    elemento_ = p.elemento_principal;
    T_K_ = p.T_K;
    conc_ = p.elementos.front().molality;
    params_actuales_ = p;
    calcular_diagrama_eph(params_actuales_);
}

std::vector<domain::EpHCandidateSpecies> CalculadorPourbaix::recolectar_especies_sistema(
    const domain::EpHSystemParameters& params) const {

    std::set<std::string> elems_permitidos = {"H", "O"};
    std::map<std::string, double> molalidades;
    double p_max = 1.0;

    for (const auto& el : params.elementos) {
        if (!el.symbol.empty()) {
            elems_permitidos.insert(el.symbol);
            molalidades[el.symbol] = std::max(el.molality, 1e-20);
            p_max = std::max(p_max, el.pressure_bar);
        }
    }
    if (elems_permitidos.size() <= 2 && !params.elemento_principal.empty()) {
        elems_permitidos.insert(params.elemento_principal);
        molalidades[params.elemento_principal] = std::max(conc_, 1e-20);
    }

    const double T_K = std::clamp(params.T_K, 250.0, 1000.0);
    const double dT = T_K - 298.15;
    const double RT_kJ = kR_kJ * T_K;
    const double eps_w = params.override_constante_dielectrica
                             ? params.constante_dielectrica_manual
                             : calcular_constante_dielectrica_agua(T_K, p_max);
    const double gamma_davies = params.usar_davies
                                    ? calcular_factor_davies(params.fuerza_ionica, eps_w, T_K)
                                    : 1.0;

    std::vector<domain::EpHCandidateSpecies> resultado;
    std::set<std::string> vistas;
    int64_t next_id = 800001;

    auto evaluar_y_agregar = [&](const std::string& formula, const std::string& nombre,
                                 int estado_cat, double dG298_kcal, double s298_J_K,
                                 double cp298_J_K, bool def_sel) {
        if (vistas.count(formula)) return;
        std::map<std::string, double> comp;
        int carga = 0;
        int est_tag = -1;
        if (!parsear_especie_eph(formula, comp, carga, est_tag)) return;

        // Excluir especies puras de H y O (H2O, O2, H2, OH-, H+) de la lista de especies del elemento
        bool tiene_no_ho = false;
        for (const auto& [el, cnt] : comp) {
            if (!elems_permitidos.count(el)) return;
            if (el != "H" && el != "O" && cnt > 0.0) tiene_no_ho = true;
        }
        if (!tiene_no_ho) return;

        const bool es_org = (comp.count("C") && comp.count("H") && comp.at("C") >= 2.0);
        if (es_org && !params.modos_busqueda.organics) return;

        const int estado = (est_tag >= 0) ? est_tag : estado_cat;
        // Filtrar según Search Mode (Sección 17.2.2)
        if (estado == 0 && !params.modos_busqueda.gases) return;
        if (estado == 5 && !params.modos_busqueda.gas_ions) return;
        if (estado == 1 && !params.modos_busqueda.liquids) return;
        if (estado == 2 && !params.modos_busqueda.condensed) return;
        if (estado == 4 && !params.modos_busqueda.aqueous_neutrals) return;
        if (estado == 3 && !params.modos_busqueda.aqueous_ions) return;

        double cp_eff_J = cp298_J_K;
        if (params.usar_criss_cobble && (estado == 3 || estado == 4) && std::abs(cp_eff_J) < 1e-3) {
            cp_eff_J = calcular_cp_criss_cobble_J(carga, s298_J_K, T_K);
        }

        const double dG298_kJ = dG298_kcal * kCalToKJ;
        const double term_S_kJ = -(s298_J_K / 1000.0) * dT;
        const double term_Cp_kJ = (cp_eff_J / 1000.0) * (dT - T_K * std::log(T_K / 298.15));
        const double dG_T_kJ = dG298_kJ + term_S_kJ + term_Cp_kJ;

        // Determinar el elemento no-H/O primario de esta especie para su molalidad
        std::string elem_sp = params.elemento_principal;
        double coef_sp = 1.0;
        if (comp.count(params.elemento_principal) && comp.at(params.elemento_principal) > 0.0) {
            elem_sp = params.elemento_principal;
            coef_sp = comp.at(params.elemento_principal);
        } else {
            for (const auto& [el, cnt] : comp) {
                if (el != "H" && el != "O") {
                    elem_sp = el;
                    coef_sp = cnt;
                    break;
                }
            }
        }

        double dG_app_kJ = dG_T_kJ;
        if (estado == 3 || estado == 4) {
            const double molal = molalidades.count(elem_sp) ? molalidades.at(elem_sp) : 1.0;
            const double a_ion = std::max((molal / std::max(coef_sp, 1.0)) *
                                          std::pow(gamma_davies, carga * carga), 1e-30);
            dG_app_kJ += RT_kJ * std::log(a_ion);
        } else if (estado == 0) {
            dG_app_kJ += RT_kJ * std::log(std::max(p_max, 1e-20));
        }

        domain::EpHCandidateSpecies cand;
        cand.id = next_id++;
        cand.formula = formula;
        cand.clean_label = formula;
        cand.nombre = nombre;
        cand.elemento_grupo = elem_sp;
        cand.estado = estado;
        cand.fase = (estado == 0) ? nucleo::Fase::Gas
                  : (estado == 1) ? nucleo::Fase::Liquido
                                  : nucleo::Fase::Solido;
        cand.carga = carga;
        cand.dG0_T_kJ = dG_T_kJ;
        cand.dG0_T_kcal = dG_T_kJ * kKJToKcal;
        cand.dG_app_base_kJ = dG_app_kJ;
        cand.s_298_J_K = s298_J_K;
        cand.cp_298_J_K = cp298_J_K;
        cand.h_298_kJ = dG298_kJ + 298.15 * (s298_J_K / 1000.0);
        cand.composicion = comp;
        cand.seleccionada_por_defecto = def_sel;
        resultado.push_back(cand);
        vistas.insert(formula);
    };

    for (const auto& r : catalogo_eph_hsc()) {
        evaluar_y_agregar(r.formula, r.nombre, r.estado, r.dG298_kcal, r.s298_J_K, r.cp298_J_K, r.default_selected);
    }

    // Si el usuario selecciona un elemento que no está en el catálogo curado EpH, consultar repo_
    if (repository_ && resultado.empty()) {
        for (const auto* sp : repository_->todas_especies()) {
            if (!sp) continue;
            // Rechazar especies con sufijo (ia) según Sección 17.2.3
            if (sp->formula.find("(ia)") != std::string::npos) continue;
            int est = (sp->fase_std == nucleo::Fase::Gas) ? 0
                    : (sp->fase_std == nucleo::Fase::Liquido) ? 1 : 2;
            const double dG298_kJ = sp->h_formacion - 298.15 * (sp->s_formacion / 1000.0);
            evaluar_y_agregar(sp->formula, sp->nombre, est, dG298_kJ * kKJToKcal,
                              sp->s_formacion, sp->cp(298.15), true);
        }
    }

    return resultado;
}

domain::EpHDiagramResult CalculadorPourbaix::calcular_diagrama_eph(
    const domain::EpHSystemParameters& params) {

    params_actuales_ = params;
    domain::EpHDiagramResult res;

    if (params.pH_max - params.pH_min < 0.2 || params.Eh_max - params.Eh_min < 0.2) {
        res.mensaje_error = "La diferencia mínima entre los límites Min y Max de pH y Eh debe ser de al menos 0.2.";
        return res;
    }

    res.T_K = std::clamp(params.T_K, 250.0, 1000.0);
    res.T_C = res.T_K - 273.15;
    res.pH_min = params.pH_min;
    res.pH_max = params.pH_max;
    res.Eh_min = params.Eh_min;
    res.Eh_max = params.Eh_max;
    res.escala_potencial = params.escala_potencial;
    res.elementos_usados = params.elementos;
    if (res.elementos_usados.empty()) {
        res.elementos_usados.push_back({params.elemento_principal.empty() ? "Cu" : params.elemento_principal, conc_, 1.0});
    }

    res.elemento_principal = params.elemento_principal.empty()
                                 ? res.elementos_usados.front().symbol
                                 : params.elemento_principal;
    for (const auto& el : res.elementos_usados) {
        if (el.symbol != res.elemento_principal && !el.symbol.empty()) {
            res.elementos_secundarios.push_back(el.symbol);
        }
    }

    // Presión total del sistema >= presión de vapor del agua a T (Sección 17.3.6)
    res.presion_vapor_agua_bar = calcular_presion_vapor_agua_bar(res.T_K);
    double max_p = 1.0;
    for (const auto& el : res.elementos_usados) max_p = std::max(max_p, el.pressure_bar);
    if (max_p < res.presion_vapor_agua_bar) {
        res.advertencias.push_back("System pressure raised to water saturation vapor pressure at T.");
        max_p = res.presion_vapor_agua_bar;
    }
    res.presion_total_bar = max_p;

    res.constante_dielectrica = params.override_constante_dielectrica
                                    ? params.constante_dielectrica_manual
                                    : calcular_constante_dielectrica_agua(res.T_K, res.presion_total_bar);
    res.dG_H2O_kcal = params.override_dG_agua
                          ? params.dG_agua_manual_kcal
                          : calcular_dG_agua_kcal(res.T_K, res.presion_total_bar);
    res.dG_H2O_kJ = res.dG_H2O_kcal * kCalToKJ;
    dGf_H2O_l_ = res.dG_H2O_kJ;

    res.fuerza_ionica = params.fuerza_ionica;
    res.factor_correccion_davies = params.usar_davies
                                       ? calcular_factor_davies(params.fuerza_ionica, res.constante_dielectrica, res.T_K)
                                       : 1.0;

    // Título del diagrama estilo HSC EpH Fig. 3 y Fig. 4: "Cu - S - System at 25.00 °C"
    {
        std::ostringstream ts;
        ts << res.elemento_principal;
        for (const auto& sec : res.elementos_secundarios) ts << " - " << sec;
        ts << " - System at " << std::fixed << std::setprecision(2) << res.T_C << " °C";
        res.titulo_diagrama = ts.str();
    }

    // Líneas de estabilidad del agua (Sección 17.1)
    T_K_ = res.T_K;
    escala_ = res.escala_potencial;
    res.eh_agua_sup_pHmin = eh_agua_oxidacion(res.pH_min, res.presion_total_bar);
    res.eh_agua_sup_pHmax = eh_agua_oxidacion(res.pH_max, res.presion_total_bar);
    res.eh_agua_inf_pHmin = eh_agua_reduccion(res.pH_min, res.presion_total_bar);
    res.eh_agua_inf_pHmax = eh_agua_reduccion(res.pH_max, res.presion_total_bar);

    res.especies_tabla = recolectar_especies_sistema(params);
    if (res.especies_tabla.empty()) {
        res.mensaje_error = "No se encontraron especies para los elementos y modos de búsqueda seleccionados.";
        return res;
    }

    std::set<std::string> sel_set(params.formulas_seleccionadas.begin(), params.formulas_seleccionadas.end());
    const bool usar_defecto = sel_set.empty();

    std::vector<domain::EpHCandidateSpecies> activas;
    for (const auto& sp : res.especies_tabla) {
        if (usar_defecto ? sp.seleccionada_por_defecto : (sel_set.count(sp.formula) > 0)) {
            activas.push_back(sp);
        }
    }
    if (activas.empty()) activas = res.especies_tabla;

    const double RT_ln10_kJ = kR_kJ * res.T_K * kLn10;
    const double E_offset = offset_escala_potencial(res.escala_potencial);

    auto construir_plano_puro = [&](const domain::EpHCandidateSpecies& sp, const std::string& elem_norm) {
        const double n_m = sp.composicion.count(elem_norm) ? sp.composicion.at(elem_norm) : 1.0;
        const double n_O = sp.composicion.count("O") ? sp.composicion.at("O") : 0.0;
        const double n_H = sp.composicion.count("H") ? sp.composicion.at("H") : 0.0;
        const double n_Hp_norm = (2.0 * n_O - n_H) / n_m;
        const double n_e_norm = (2.0 * n_O - n_H + sp.carga) / n_m;

        PlanoLinealEpH pl;
        pl.id = sp.id;
        pl.formula = sp.formula;
        pl.nombre = sp.nombre;
        pl.estado = sp.estado;
        pl.carga = sp.carga;
        pl.n_main = n_m;
        pl.n_O_norm = n_O / n_m;
        pl.n_H_norm = n_H / n_m;
        pl.n_Hplus_norm = n_Hp_norm;
        pl.n_e_norm = n_e_norm;
        pl.A = -n_Hp_norm * RT_ln10_kJ;
        pl.B = -n_e_norm * kFaraday_kJ_V;
        pl.Z = (sp.dG_app_base_kJ - n_O * res.dG_H2O_kJ) / n_m + pl.B * E_offset;
        return pl;
    };

    // Clasificar especies activas en Puras-M, Puras-L y Mixtas M-L
    std::vector<PlanoLinealEpH> planos_puros_M;
    std::vector<PlanoLinealEpH> planos_puros_L;
    std::vector<domain::EpHCandidateSpecies> mixtas_ML;

    for (const auto& sp : activas) {
        const bool tiene_M = sp.composicion.count(res.elemento_principal) && sp.composicion.at(res.elemento_principal) > 0.0;
        bool tiene_L = false;
        for (const auto& sec : res.elementos_secundarios) {
            if (sp.composicion.count(sec) && sp.composicion.at(sec) > 0.0) {
                tiene_L = true;
                break;
            }
        }
        if (tiene_M && !tiene_L) {
            planos_puros_M.push_back(construir_plano_puro(sp, res.elemento_principal));
        } else if (!tiene_M && tiene_L) {
            planos_puros_L.push_back(construir_plano_puro(sp, res.elementos_secundarios.front()));
        } else if (tiene_M && tiene_L) {
            mixtas_ML.push_back(sp);
        }
    }

    // Caso A: Si M es el ligando (ej. S - Cu en Fig. 4) o una especie mixta acopla con el estado basal de L (ej. Cu(s) o Cl-)
    // Para especies mixtas cuya contraparte en L tiene un único estado basal reducido (como Cu(s) cuando M=S, o Cl-(a) cuando M=Au):
    for (const auto& m_sp : mixtas_ML) {
        const double n_M = m_sp.composicion.at(res.elemento_principal);
        const std::string elem_L = res.elementos_secundarios.front();
        const double n_L = m_sp.composicion.at(elem_L);
        const double n_O = m_sp.composicion.count("O") ? m_sp.composicion.at("O") : 0.0;
        const double n_H = m_sp.composicion.count("H") ? m_sp.composicion.at("H") : 0.0;

        if (res.elemento_principal == "S" || elem_L == "Cl") {
            // Acoplar con la referencia basal de L (Cu metálico o Cl-)
            PlanoLinealEpH ref_L;
            bool found_ref = false;
            for (const auto& pl_L : planos_puros_L) {
                if (pl_L.formula == "Cu" || pl_L.formula == "Cl(-a)") {
                    ref_L = pl_L;
                    found_ref = true;
                    break;
                }
            }
            if (!found_ref && !planos_puros_L.empty()) {
                ref_L = planos_puros_L.front();
                found_ref = true;
            }
            if (found_ref) {
                PlanoLinealEpH pl_m = construir_plano_puro(m_sp, res.elemento_principal);
                const double ratio = n_L / n_M;
                pl_m.Z -= ratio * ref_L.Z;
                pl_m.A -= ratio * ref_L.A;
                pl_m.B -= ratio * ref_L.B;
                pl_m.n_Hplus_norm -= ratio * ref_L.n_Hplus_norm;
                pl_m.n_e_norm -= ratio * ref_L.n_e_norm;
                planos_puros_M.push_back(pl_m);
            }
        }
        (void)n_O;
        (void)n_H;
    }

    // Extraer polígonos y fronteras del envolvente inferior principal
    extraer_poligonos_y_fronteras_planos(
        planos_puros_M, activas, res.pH_min, res.pH_max, res.Eh_min, res.Eh_max,
        false, res.poligonos, res.fronteras, &res.puntos_triples);

    // Caso B: Para sulfuros metálicos cuando el metal es el elemento principal (ej. Cu - S en Fig. 3, Zn - S, Pb - S)
    // Calcular el polígono convexo exacto de cada sulfuro acoplado con las especies reducidas y la primera oxidada de L
    if (res.elemento_principal != "S" && !res.elementos_secundarios.empty() &&
        res.elementos_secundarios.front() != "Cl" && !mixtas_ML.empty() && !planos_puros_L.empty()) {

        // Identificar las especies reducidas de L (n_e <= 0) y la especie oxidada de menor n_e positivo
        std::vector<PlanoLinealEpH> refs_L;
        const PlanoLinealEpH* mejor_ox_L = nullptr;
        for (const auto& pl_L : planos_puros_L) {
            if (pl_L.n_e_norm <= 1e-4) {
                refs_L.push_back(pl_L);
            } else {
                if (!mejor_ox_L || pl_L.n_e_norm < mejor_ox_L->n_e_norm - 1e-4 ||
                    (std::abs(pl_L.n_e_norm - mejor_ox_L->n_e_norm) <= 1e-4 && pl_L.Z < mejor_ox_L->Z)) {
                    mejor_ox_L = &pl_L;
                }
            }
        }
        if (mejor_ox_L) refs_L.push_back(*mejor_ox_L);

        const std::vector<domain::EpHVertice2D> bbox = {
            {res.pH_min, res.Eh_min}, {res.pH_max, res.Eh_min},
            {res.pH_max, res.Eh_max}, {res.pH_min, res.Eh_max}
        };

        for (const auto& m_sp : mixtas_ML) {
            const double n_M = m_sp.composicion.at(res.elemento_principal);
            const std::string elem_L = res.elementos_secundarios.front();
            const double n_L = m_sp.composicion.at(elem_L);
            const double ratio = n_L / n_M;
            const PlanoLinealEpH base_m = construir_plano_puro(m_sp, res.elemento_principal);

            auto poly_m = bbox;
            struct HalfPlaneRef {
                double dA, dB, dZ;
                std::string otra_fase;
                double dn_Hp, dn_e;
            };
            std::vector<HalfPlaneRef> hps;

            for (const auto& rl : refs_L) {
                const double Zm = base_m.Z - ratio * rl.Z;
                const double Am = base_m.A - ratio * rl.A;
                const double Bm = base_m.B - ratio * rl.B;
                const double nHp_m = base_m.n_Hplus_norm - ratio * rl.n_Hplus_norm;
                const double ne_m = base_m.n_e_norm - ratio * rl.n_e_norm;

                for (const auto& pm : planos_puros_M) {
                    const double dA = Am - pm.A;
                    const double dB = Bm - pm.B;
                    const double dZ = Zm - pm.Z;
                    poly_m = clip_polygon_eph(poly_m, dA, dB, dZ);
                    hps.push_back({dA, dB, dZ, pm.formula + " + " + rl.formula, pm.n_Hplus_norm - nHp_m, pm.n_e_norm - ne_m});
                    if (poly_m.empty()) break;
                }
                if (poly_m.empty()) break;
            }

            if (poly_m.size() >= 3) {
                domain::EpHPoligonoPredominancia pf;
                pf.id = m_sp.id;
                pf.area_no = static_cast<int>(res.poligonos.size()) + 1;
                pf.formula = m_sp.formula;
                pf.etiqueta = m_sp.formula;
                pf.nombre = m_sp.nombre;
                pf.es_acuoso = false;
                pf.carga = m_sp.carga;
                pf.color_index = static_cast<int>(res.poligonos.size());
                pf.estado_corrosion = domain::PourbaixCorrosionState::Passivation;
                pf.vertices = poly_m;
                calcular_centroide_mas_ancho_eph(pf);

                if (pf.area > 1e-4) {
                    res.poligonos.push_back(pf);
                    for (const auto& hp : hps) {
                        const double norm = std::hypot(hp.dA, hp.dB);
                        if (norm < 1e-8) continue;
                        std::vector<domain::EpHVertice2D> pts;
                        for (const auto& v : poly_m) {
                            if (std::abs(hp.dA * v.pH + hp.dB * v.Eh + hp.dZ) / norm < 1e-3) {
                                pts.push_back(v);
                            }
                        }
                        if (pts.size() >= 2) {
                            std::sort(pts.begin(), pts.end(), [](const domain::EpHVertice2D& u, const domain::EpHVertice2D& v) {
                                return (std::abs(u.pH - v.pH) > 1e-4) ? (u.pH < v.pH) : (u.Eh < v.Eh);
                            });
                            const auto p1 = pts.front();
                            const auto p2 = pts.back();
                            if (std::hypot(p2.pH - p1.pH, p2.Eh - p1.Eh) > 1e-3) {
                                domain::EpHSegmentoFrontera seg;
                                seg.area_no = pf.area_no;
                                seg.fase_a = m_sp.formula;
                                seg.fase_b = hp.otra_fase;
                                seg.p1 = p1;
                                seg.p2 = p2;
                                seg.dG_rxn_kJ = -hp.dZ;
                                seg.ecuacion_reaccion = construir_ecuacion_redox(m_sp.formula, hp.otra_fase, hp.dn_Hp, hp.dn_e);
                                seg.tipo_linea = (std::abs(hp.dB) < 1e-5) ? 2 : (std::abs(hp.dA) < 1e-5 ? 0 : 1);
                                seg.pendiente = (std::abs(hp.dB) > 1e-5) ? (-hp.dA / hp.dB) : 1e9;
                                seg.intercepto_eh = (std::abs(hp.dB) > 1e-5) ? (-hp.dZ / hp.dB) : p1.pH;
                                res.fronteras.push_back(seg);
                            }
                        }
                    }
                }
            }
        }
    }

    // Segundo diagrama opcional: "Show predominance areas of ions" (Sección 17.3.7, líneas azules punteadas)
    if (params.mostrar_areas_iones) {
        std::vector<PlanoLinealEpH> planos_iones;
        for (const auto& pl : planos_puros_M) {
            if (pl.estado == 3 || pl.estado == 4) {
                planos_iones.push_back(pl);
            }
        }
        if (planos_iones.size() >= 2) {
            extraer_poligonos_y_fronteras_planos(
                planos_iones, activas, res.pH_min, res.pH_max, res.Eh_min, res.Eh_max,
                true, res.poligonos_iones, res.fronteras_iones, nullptr);
        }
    }

    res.ok = !res.poligonos.empty();
    ultimo_resultado_ = res;
    sincronizar_legado_desde_resultado();
    return res;
}

void CalculadorPourbaix::sincronizar_legado_desde_resultado() {
    especies_activas_.clear();
    for (const auto& sp : ultimo_resultado_.especies_tabla) {
        if (sp.elemento_grupo != ultimo_resultado_.elemento_principal) continue;
        const double coef = sp.composicion.count(ultimo_resultado_.elemento_principal)
                                ? sp.composicion.at(ultimo_resultado_.elemento_principal)
                                : 1.0;
        EspeciePourbaix ep;
        ep.formula = sp.formula;
        ep.nombre = sp.nombre;
        ep.dG_f = sp.dG_app_base_kJ;
        ep.carga = sp.carga;
        ep.coef_elemento = static_cast<int>(std::round(coef));
        ep.estado = (sp.estado == 4) ? 3 : sp.estado;
        ep.n_H = (sp.composicion.count("H") ? sp.composicion.at("H") : 0.0) / coef;
        ep.n_O = (sp.composicion.count("O") ? sp.composicion.at("O") : 0.0) / coef;
        especies_activas_.push_back(ep);
    }

    lineas_.clear();
    // 2 líneas de estabilidad del agua
    {
        const double slope = pendiente_nernst();
        LineaPourbaix l_ox{"O2(g)", "H2O", slope, ultimo_resultado_.eh_agua_sup_pHmin - slope * ultimo_resultado_.pH_min,
                           false, 0.0, true, ultimo_resultado_.pH_min, ultimo_resultado_.eh_agua_sup_pHmin,
                           ultimo_resultado_.pH_max, ultimo_resultado_.eh_agua_sup_pHmax, "2 H2O = O2(g) + 4 H(+a) + 4 e-"};
        LineaPourbaix l_red{"H(+a)", "H2(g)", slope, ultimo_resultado_.eh_agua_inf_pHmin - slope * ultimo_resultado_.pH_min,
                            false, 0.0, true, ultimo_resultado_.pH_min, ultimo_resultado_.eh_agua_inf_pHmin,
                            ultimo_resultado_.pH_max, ultimo_resultado_.eh_agua_inf_pHmax, "2 H(+a) + 2 e- = H2(g)"};
        lineas_.push_back(l_ox);
        lineas_.push_back(l_red);
    }
    for (const auto& f : ultimo_resultado_.fronteras) {
        LineaPourbaix lp;
        lp.especie_a = f.fase_a;
        lp.especie_b = f.fase_b;
        lp.vertical = (f.tipo_linea == 2);
        lp.x_val = lp.vertical ? f.p1.pH : 0.0;
        lp.m = lp.vertical ? 0.0 : f.pendiente;
        lp.b = lp.vertical ? 0.0 : f.intercepto_eh;
        lp.es_agua = false;
        lp.x1 = f.p1.pH; lp.y1 = f.p1.Eh;
        lp.x2 = f.p2.pH; lp.y2 = f.p2.Eh;
        lp.ecuacion = f.ecuacion_reaccion;
        lineas_.push_back(lp);
    }

    // Poblar grilla de dominancia 80x80 para compatibilidad
    const int NP = 80, NE = 80;
    dominancia_.n_pH = NP;
    dominancia_.n_Eh = NE;
    dominancia_.pH_min = ultimo_resultado_.pH_min;
    dominancia_.pH_max = ultimo_resultado_.pH_max;
    dominancia_.Eh_min = ultimo_resultado_.Eh_min;
    dominancia_.Eh_max = ultimo_resultado_.Eh_max;
    dominancia_.grid.assign(NP * NE, 0);

    for (int ip = 0; ip < NP; ++ip) {
        const double pH = dominancia_.pH_min + (dominancia_.pH_max - dominancia_.pH_min) * ip / (NP - 1.0);
        for (int ie = 0; ie < NE; ++ie) {
            const double Eh = dominancia_.Eh_min + (dominancia_.Eh_max - dominancia_.Eh_min) * ie / (NE - 1.0);
            auto ev = evaluar_punto(pH, Eh);
            int idx = 0;
            for (size_t k = 0; k < especies_activas_.size(); ++k) {
                if (especies_activas_[k].formula == ev.predominant_formula) {
                    idx = static_cast<int>(k);
                    break;
                }
            }
            dominancia_.grid[ip * NE + ie] = idx;
        }
    }
}

bool CalculadorPourbaix::calcular() {
    if (params_actuales_.elementos.empty()) {
        params_actuales_.elementos = {{elemento_, conc_, 1.0}};
        params_actuales_.elemento_principal = elemento_;
    }
    params_actuales_.T_K = T_K_;
    params_actuales_.escala_potencial = escala_;
    auto res = calcular_diagrama_eph(params_actuales_);
    return res.ok;
}

domain::PourbaixPointEvaluation CalculadorPourbaix::evaluar_punto(double pH, double Eh) const {
    domain::PourbaixPointEvaluation ev;
    if (!ultimo_resultado_.ok || ultimo_resultado_.poligonos.empty()) return ev;

    ev.valid = true;
    ev.pH = pH;
    ev.Eh = Eh;
    const double eh_ox = eh_agua_oxidacion(pH, ultimo_resultado_.presion_total_bar);
    const double eh_red = eh_agua_reduccion(pH, ultimo_resultado_.presion_total_bar);
    ev.o2_evolution = (Eh > eh_ox);
    ev.h2_evolution = (Eh < eh_red);
    ev.water_stable = !ev.o2_evolution && !ev.h2_evolution;

    // 1. Verificar si el punto cae dentro de algún polígono de sulfuro mixto (como Cu2S) que se superpone a Cu(s)
    auto punto_en_poligono = [](const std::vector<domain::EpHVertice2D>& verts, double x, double y) {
        bool inside = false;
        const size_t n = verts.size();
        for (size_t i = 0, j = n - 1; i < n; j = i++) {
            const double xi = verts[i].pH, yi = verts[i].Eh;
            const double xj = verts[j].pH, yj = verts[j].Eh;
            if (((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi + 1e-30) + xi)) {
                inside = !inside;
            }
        }
        return inside;
    };

    // Buscar el polígono con menor G aparente o polígono mixto activo que contiene a (pH, Eh)
    double best_g = 1e30;
    const domain::EpHPoligonoPredominancia* best_poly = &ultimo_resultado_.poligonos.front();

    for (const auto& pol : ultimo_resultado_.poligonos) {
        if (std::abs(pol.A) < 1e-9 && std::abs(pol.B) < 1e-9 && std::abs(pol.Z) < 1e-9 && pol.formula != ultimo_resultado_.elemento_principal) {
            // Polígono mixto recortado (ej. Cu2S en modo Cu-S)
            if (punto_en_poligono(pol.vertices, pH, Eh)) {
                best_poly = &pol;
                best_g = -1e9;
                break;
            }
            continue;
        }
        const double g = pol.Z + pol.A * pH + pol.B * Eh;
        if (g < best_g) {
            best_g = g;
            best_poly = &pol;
        }
    }

    ev.predominant_formula = best_poly->formula;
    ev.dG_apparent_kJ = best_g;
    ev.corrosion_state = best_poly->estado_corrosion;

    if (!ultimo_resultado_.poligonos_iones.empty()) {
        double best_ion_g = 1e30;
        for (const auto& ipol : ultimo_resultado_.poligonos_iones) {
            const double g = ipol.Z + ipol.A * pH + ipol.B * Eh;
            if (g < best_ion_g) {
                best_ion_g = g;
                ev.predominant_ion_formula = ipol.formula;
            }
        }
    }
    return ev;
}

std::vector<domain::EpHCombinerDataset> CalculadorPourbaix::calcular_combiner_eph(
    const domain::EpHSystemParameters& base_params,
    const std::vector<double>& temperaturas_C) const {

    std::vector<domain::EpHCombinerDataset> datasets;
    CalculadorPourbaix worker(repository_);

    for (double t_c : temperaturas_C) {
        domain::EpHSystemParameters p = base_params;
        p.T_K = t_c + 273.15;
        domain::EpHCombinerDataset ds;
        std::ostringstream lbl;
        lbl << std::fixed << std::setprecision(0) << t_c << " °C";
        ds.etiqueta = lbl.str();
        ds.temperatura_C = t_c;
        ds.molalidad_principal = p.elementos.empty() ? 1.0 : p.elementos.front().molality;
        ds.activo = true;
        ds.resultado = worker.calcular_diagrama_eph(p);
        datasets.push_back(ds);
    }
    return datasets;
}

std::string CalculadorPourbaix::exportar_csv() const {
    return exportar_tabla_eph_csv(ultimo_resultado_);
}

std::string CalculadorPourbaix::exportar_tabla_eph_csv(const domain::EpHDiagramResult& res) const {
    std::ostringstream os;
    os << "# HSC EpH - Pourbaix Diagram Basic Settings & Boundary Coordinates\n";
    os << "# Title: " << res.titulo_diagrama << "\n";
    os << "# Dielectric Constant: " << std::fixed << std::setprecision(3) << res.constante_dielectrica
       << " | DeltaG H2O: " << res.dG_H2O_kcal << " kcal/mol\n\n";

    os << "[SPECIES]\n";
    os << "Species,DeltaG (kcal/mol),DeltaG (kJ/mol),State,Charge\n";
    for (const auto& sp : res.especies_tabla) {
        os << sp.formula << "," << std::fixed << std::setprecision(3)
           << sp.dG0_T_kcal << "," << sp.dG0_T_kJ << ","
           << sp.estado << "," << sp.carga << "\n";
    }

    os << "\n[NERNST_BOUNDARIES]\n";
    os << "Area no,Species A,Species B,X1 (pH),Y1 (V),X2 (pH),Y2 (V),Slope (V/pH),Reaction\n";
    for (const auto& f : res.fronteras) {
        os << f.area_no << "," << f.fase_a << "," << f.fase_b << ","
           << std::fixed << std::setprecision(3)
           << f.p1.pH << "," << f.p1.Eh << ","
           << f.p2.pH << "," << f.p2.Eh << ","
           << f.pendiente << ",\"" << f.ecuacion_reaccion << "\"\n";
    }

    os << "\n[TRIPLE_POINTS]\n";
    os << "Phase 1,Phase 2,Phase 3,pH,Eh (V)\n";
    for (const auto& pt : res.puntos_triples) {
        os << pt.fase_1 << "," << pt.fase_2 << "," << pt.fase_3 << ","
           << std::fixed << std::setprecision(3) << pt.pH << "," << pt.Eh << "\n";
    }
    return os.str();
}

std::string CalculadorPourbaix::serializar_eph8(const domain::EpHSystemParameters& p) const {
    std::ostringstream os;
    os << "HSC_EPH8_SPEC\n";
    os << "MAIN=" << p.elemento_principal << "\n";
    os << "T_K=" << p.T_K << "\n";
    os << "SCALE=" << static_cast<int>(p.escala_potencial) << "\n";
    os << "PH_MIN=" << p.pH_min << "\n";
    os << "PH_MAX=" << p.pH_max << "\n";
    os << "EH_MIN=" << p.Eh_min << "\n";
    os << "EH_MAX=" << p.Eh_max << "\n";
    os << "DAVIES=" << (p.usar_davies ? 1 : 0) << "\n";
    os << "ION_STRENGTH=" << p.fuerza_ionica << "\n";
    os << "CRISS_COBBLE=" << (p.usar_criss_cobble ? 1 : 0) << "\n";
    os << "SHOW_IONS=" << (p.mostrar_areas_iones ? 1 : 0) << "\n";
    os << "SHOW_COLORS=" << (p.mostrar_colores ? 1 : 0) << "\n";
    os << "ELEMENTS=";
    for (size_t i = 0; i < p.elementos.size(); ++i) {
        if (i > 0) os << ";";
        os << p.elementos[i].symbol << ":" << p.elementos[i].molality << ":" << p.elementos[i].pressure_bar;
    }
    os << "\n";
    return os.str();
}

bool CalculadorPourbaix::deserializar_eph8(const std::string& contenido, domain::EpHSystemParameters& out) const {
    if (contenido.find("HSC_EPH8_SPEC") == std::string::npos) return false;
    std::istringstream is(contenido);
    std::string line;
    while (std::getline(is, line)) {
        const size_t eq = line.find('=');
        if (eq == std::string::npos) continue;
        const std::string k = line.substr(0, eq);
        const std::string v = line.substr(eq + 1);
        if (k == "MAIN") out.elemento_principal = v;
        else if (k == "T_K") out.T_K = std::stod(v);
        else if (k == "SCALE") out.escala_potencial = static_cast<domain::PourbaixPotentialScale>(std::stoi(v));
        else if (k == "PH_MIN") out.pH_min = std::stod(v);
        else if (k == "PH_MAX") out.pH_max = std::stod(v);
        else if (k == "EH_MIN") out.Eh_min = std::stod(v);
        else if (k == "EH_MAX") out.Eh_max = std::stod(v);
        else if (k == "DAVIES") out.usar_davies = (v == "1");
        else if (k == "ION_STRENGTH") out.fuerza_ionica = std::stod(v);
        else if (k == "CRISS_COBBLE") out.usar_criss_cobble = (v == "1");
        else if (k == "SHOW_IONS") out.mostrar_areas_iones = (v == "1");
        else if (k == "SHOW_COLORS") out.mostrar_colores = (v == "1");
        else if (k == "ELEMENTS") {
            out.elementos.clear();
            std::istringstream es(v);
            std::string tok;
            while (std::getline(es, tok, ';')) {
                const size_t c1 = tok.find(':');
                const size_t c2 = (c1 != std::string::npos) ? tok.find(':', c1 + 1) : std::string::npos;
                if (c1 != std::string::npos && c2 != std::string::npos) {
                    out.elementos.push_back({
                        tok.substr(0, c1),
                        std::stod(tok.substr(c1 + 1, c2 - c1 - 1)),
                        std::stod(tok.substr(c2 + 1))
                    });
                }
            }
        }
    }
    return !out.elementos.empty();
}

}} // namespace

#pragma GCC diagnostic pop
