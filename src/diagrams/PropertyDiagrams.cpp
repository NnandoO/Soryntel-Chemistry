#include "PropertyDiagrams.hpp"
#include "PropertyCatalogHelpers.hpp"

#include "nucleo/tabla_periodica.hpp"
#include "nucleo/tipos.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <iomanip>
#include <map>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

namespace cma::application::modules {

namespace {

using detail::kR_J;
using detail::kR_kJ;
using detail::kLn10;
using detail::kTref;
using detail::trim_str;
using detail::clean_formula_str;
using detail::es_especie_acuosa;
using detail::make_sp;
using detail::make_transition_polymorph;
using detail::tabla_transiciones;
using detail::catalogo_respaldo;
using detail::format_phase_label;
using detail::resolver_polimorfos;

const nucleo::Especie& seleccionar_polimorfo_activo(const std::vector<nucleo::Especie>& polimorfos,
                                                    double T_K) {
    auto it = std::min_element(polimorfos.begin(), polimorfos.end(), [T_K](const auto& a, const auto& b) {
        return a.energia_gibbs(T_K) < b.energia_gibbs(T_K);
    });
    return *it;
}

double refinar_temperatura_transicion(const nucleo::Especie& a, const nucleo::Especie& b,
                                      double T_low, double T_high) {
    double lo = T_low;
    double hi = T_high;
    double d_lo = a.energia_gibbs(lo) - b.energia_gibbs(lo);
    for (int iter = 0; iter < 32; ++iter) {
        double mid = 0.5 * (lo + hi);
        double d_mid = a.energia_gibbs(mid) - b.energia_gibbs(mid);
        if ((d_lo <= 0.0 && d_mid <= 0.0) || (d_lo >= 0.0 && d_mid >= 0.0)) {
            lo = mid;
            d_lo = d_mid;
        } else {
            hi = mid;
        }
    }
    return 0.5 * (lo + hi);
}

// Evalua H°(T), S°(T), G°(T) por 1 mol de atomos de un elemento en su estado estandar de referencia a T
void evaluar_elemento_referencia(const domain::ThermoRepository& repo, const std::string& elem_sym,
                                 double T_K, double& H_out, double& S_out, double& G_out) {
    if (elem_sym == "O" || elem_sym == "H" || elem_sym == "N" || elem_sym == "Cl" || elem_sym == "F") {
        std::string gas_f = elem_sym + "2";
        auto polys = resolver_polimorfos(repo, gas_f, false);
        if (!polys.empty()) {
            const auto& sp = polys.front();
            H_out = 0.5 * sp.entalpia_abs(T_K);
            S_out = 0.5 * sp.entropia_abs(T_K);
            G_out = 0.5 * sp.energia_gibbs(T_K);
            return;
        }
    }
    if (elem_sym == "S") {
        auto s_sol = resolver_polimorfos(repo, "S", false);
        auto s2_gas = resolver_polimorfos(repo, "S2", false);
        if (!s_sol.empty() && !s2_gas.empty()) {
            const double g_s = s_sol.front().energia_gibbs(T_K);
            const double g_s2 = 0.5 * s2_gas.front().energia_gibbs(T_K);
            if (T_K >= 717.8 || g_s2 < g_s) {
                H_out = 0.5 * s2_gas.front().entalpia_abs(T_K);
                S_out = 0.5 * s2_gas.front().entropia_abs(T_K);
                G_out = g_s2;
            } else {
                H_out = s_sol.front().entalpia_abs(T_K);
                S_out = s_sol.front().entropia_abs(T_K);
                G_out = g_s;
            }
            return;
        }
    }
    auto polys = resolver_polimorfos(repo, elem_sym, true);
    if (!polys.empty()) {
        const auto& act = seleccionar_polimorfo_activo(polys, T_K);
        H_out = act.entalpia_abs(T_K);
        S_out = act.entropia_abs(T_K);
        G_out = act.energia_gibbs(T_K);
        return;
    }
    H_out = 0.0;
    S_out = 0.0;
    G_out = 0.0;
}

bool es_elemento_referencia_puro(const std::string& clean_f) {
    static const std::vector<std::string> refs = {
        "O2", "H2", "N2", "Cl2", "F2", "S", "C", "Fe", "Cu", "Zn", "Al", "Si",
        "Mg", "Ca", "Pb", "Ni", "Ti", "Cr", "Mn", "Mo", "Co", "V", "W", "Ag",
        "Au", "Pt", "Na", "K", "Sn", "He", "Ne", "Ar"
    };
    return std::find(refs.begin(), refs.end(), clean_f) != refs.end();
}

std::string format_coef(double c) {
    double ac = std::abs(c);
    if (std::abs(ac - 1.0) < 1e-6) {
        return "";
    }
    // Fracciones metalurgicas comunes (4/3, 2/3, 3/2, 1/2, 1/3, 1/4, 3/4)
    static const std::pair<int, int> fracs[] = {
        {1, 2}, {3, 2}, {1, 3}, {2, 3}, {4, 3}, {1, 4}, {3, 4}
    };
    for (const auto& [num, den] : fracs) {
        if (std::abs(ac - static_cast<double>(num) / static_cast<double>(den)) < 1e-4) {
            return std::to_string(num) + "/" + std::to_string(den) + " ";
        }
    }
    if (std::abs(ac - std::round(ac)) < 1e-5) {
        return std::to_string(static_cast<int>(std::round(ac))) + " ";
    }
    std::ostringstream os;
    os << std::fixed << std::setprecision(3) << ac << " ";
    return os.str();
}

std::string formatear_ecuacion_normalizada(const std::vector<TerminoReaccionDiagrama>& terminos) {
    std::ostringstream lhs;
    std::ostringstream rhs;
    bool first_l = true;
    bool first_r = true;
    for (const auto& t : terminos) {
        if (t.coef < -1e-12) {
            if (!first_l) {
                lhs << " + ";
            }
            lhs << format_coef(t.coef) << t.formula;
            first_l = false;
        } else if (t.coef > 1e-12) {
            if (!first_r) {
                rhs << " + ";
            }
            rhs << format_coef(t.coef) << t.formula;
            first_r = false;
        }
    }
    return lhs.str() + " = " + rhs.str();
}

// Parsea un termino individual como "4/3 Al", "2 FeO", "O2(g)", "0.5 Fe3O4"
bool parse_termino_quimico(const std::string& raw_term, double sign, TerminoReaccionDiagrama& out) {
    std::string s = trim_str(raw_term);
    if (s.empty()) {
        return false;
    }
    size_t idx = 0;
    double coef = 1.0;
    if (std::isdigit(static_cast<unsigned char>(s[0])) != 0 || s[0] == '.') {
        size_t num_end = 0;
        double num = std::stod(s, &num_end);
        idx = num_end;
        while (idx < s.size() && s[idx] == ' ') {
            ++idx;
        }
        if (idx < s.size() && s[idx] == '/') {
            ++idx;
            while (idx < s.size() && s[idx] == ' ') {
                ++idx;
            }
            size_t den_end = 0;
            double den = std::stod(s.substr(idx), &den_end);
            if (std::abs(den) > 1e-12) {
                coef = num / den;
            }
            idx += den_end;
        } else {
            coef = num;
        }
    }
    std::string formula = trim_str(s.substr(idx));
    if (formula.empty()) {
        return false;
    }
    out.formula = formula;
    out.coef = sign * coef;
    return true;
}

std::vector<std::string> split_reaction_side(const std::string& side) {
    std::vector<std::string> parts;
    std::string current;
    int paren_depth = 0;
    for (size_t i = 0; i < side.size(); ++i) {
        char c = side[i];
        if (c == '(' || c == '[') {
            ++paren_depth;
        } else if (c == ')' || c == ']') {
            paren_depth = std::max(0, paren_depth - 1);
        }
        if (c == '+' && paren_depth == 0) {
            parts.push_back(current);
            current.clear();
        } else {
            current.push_back(c);
        }
    }
    if (!current.empty()) {
        parts.push_back(current);
    }
    return parts;
}

std::vector<TerminoReaccionDiagrama> parsear_ecuacion_quimica(const std::string& eq) {
    std::vector<TerminoReaccionDiagrama> terminos;
    std::string lhs_str;
    std::string rhs_str;

    static const char* delimiters[] = {"<=>", "<->", "->", "=", "→", "⇌"};
    size_t split_pos = std::string::npos;
    size_t delim_len = 0;
    for (const char* d : delimiters) {
        size_t p = eq.find(d);
        if (p != std::string::npos) {
            split_pos = p;
            delim_len = std::char_traits<char>::length(d);
            break;
        }
    }
    if (split_pos == std::string::npos) {
        return terminos;
    }
    lhs_str = eq.substr(0, split_pos);
    rhs_str = eq.substr(split_pos + delim_len);

    for (const auto& part : split_reaction_side(lhs_str)) {
        TerminoReaccionDiagrama t;
        if (parse_termino_quimico(part, -1.0, t)) {
            terminos.push_back(t);
        }
    }
    for (const auto& part : split_reaction_side(rhs_str)) {
        TerminoReaccionDiagrama t;
        if (parse_termino_quimico(part, +1.0, t)) {
            terminos.push_back(t);
        }
    }
    return terminos;
}

std::string gas_objetivo_para_base(domain::EllinghamNormalizationBasis basis) {
    switch (basis) {
    case domain::EllinghamNormalizationBasis::PerMoleO2:
        return "O2";
    case domain::EllinghamNormalizationBasis::PerMoleS2:
        return "S2";
    case domain::EllinghamNormalizationBasis::PerMoleCl2:
        return "Cl2";
    case domain::EllinghamNormalizationBasis::PerMoleCO2:
        return "CO2";
    case domain::EllinghamNormalizationBasis::PerMoleSO3:
        return "SO3";
    case domain::EllinghamNormalizationBasis::PerMoleH2O:
        return "H2O";
    case domain::EllinghamNormalizationBasis::PerMoleReaction:
        return "";
    }
    return "";
}

// Evalua dG°(T) de 2 CO(g) + O2(g) = 2 CO2(g) y 2 H2(g) + O2(g) = 2 H2O(g)
void evaluar_reacciones_escala_gases(const domain::ThermoRepository& repo, double T_K,
                                     double& dG_CO_CO2_kJ, double& dG_H2_H2O_kJ) {
    auto o2 = resolver_polimorfos(repo, "O2(g)", false);
    auto co = resolver_polimorfos(repo, "CO(g)", false);
    auto co2 = resolver_polimorfos(repo, "CO2(g)", false);
    auto h2 = resolver_polimorfos(repo, "H2(g)", false);
    auto h2o = resolver_polimorfos(repo, "H2O(g)", false);

    if (!o2.empty() && !co.empty() && !co2.empty()) {
        dG_CO_CO2_kJ = 2.0 * co2.front().energia_gibbs(T_K) -
                       2.0 * co.front().energia_gibbs(T_K) -
                       o2.front().energia_gibbs(T_K);
    } else {
        dG_CO_CO2_kJ = -565.0 + 0.173 * T_K;
    }

    if (!o2.empty() && !h2.empty() && !h2o.empty()) {
        dG_H2_H2O_kJ = 2.0 * h2o.front().energia_gibbs(T_K) -
                       2.0 * h2.front().energia_gibbs(T_K) -
                       o2.front().energia_gibbs(T_K);
    } else {
        dG_H2_H2O_kJ = -492.0 + 0.110 * T_K;
    }
}

} // namespace

std::vector<PuntoPropiedad> PropertyDiagrams::generarCurvas(double h_ref, double s_ref, double cp_a,
                                                            double cp_b, double cp_c, double cp_d,
                                                            double T_min, double T_max) {
    std::vector<PuntoPropiedad> out;
    const int steps = 100;
    const double dt = (T_max - T_min) / static_cast<double>(steps);

    nucleo::Especie sp;
    sp.h_formacion = h_ref;
    sp.s_formacion = s_ref;
    sp.cp_a = cp_a;
    sp.cp_b = cp_b;
    sp.cp_c = cp_c;
    sp.cp_d = cp_d;
    if (std::abs(cp_c) > 1.0) sp.modelo_cp = nucleo::ModeloCp::MAIER_KELLEY;

    out.reserve(static_cast<size_t>(steps) + 1);
    for (int i = 0; i <= steps; ++i) {
        const double T = std::max(T_min + static_cast<double>(i) * dt, 1e-10);
        const double cp = sp.cp(T);
        const double H = sp.entalpia_abs(T);
        const double S = sp.entropia_abs(T);
        const double G = sp.energia_gibbs(T);
        out.push_back({T, H, S, cp, G});
    }
    return out;
}

domain::CurvaDiagramaTermo PropertyDiagrams::calcular_curva_especie(
    const domain::ThermoRepository& repo,
    const std::string& formula_o_nombre,
    double T_min_K,
    double T_max_K,
    int steps,
    bool usar_envolvente_fases,
    double multiplicador) const {
    domain::CurvaDiagramaTermo curva;
    curva.label = trim_str(formula_o_nombre);
    curva.equation_normalized = curva.label;
    curva.norm_factor = multiplicador;

    auto polimorfos = resolver_polimorfos(repo, formula_o_nombre, usar_envolvente_fases);
    if (polimorfos.empty()) {
        return curva;
    }

    std::string phase_tag;
    const std::string clean_f = clean_formula_str(formula_o_nombre, &phase_tag);
    const auto atomos = nucleo::TablaPeriodica::instancia().analizar_formula(clean_f);
    const bool es_elem_puro = es_elemento_referencia_puro(clean_f) &&
                              (usar_envolvente_fases || phase_tag.empty() || phase_tag == "s" ||
                               (clean_f == "O2" || clean_f == "H2" || clean_f == "N2" ||
                                clean_f == "Cl2" || clean_f == "F2"));

    const int n_steps = std::max(2, steps);
    const double t_lo = std::max(50.0, std::min(T_min_K, T_max_K));
    const double t_hi = std::max(t_lo + 1.0, std::max(T_min_K, T_max_K));
    const double dt = (t_hi - t_lo) / static_cast<double>(n_steps);

    const auto& ref_298 = seleccionar_polimorfo_activo(polimorfos, kTref);
    const double H_298 = ref_298.entalpia_abs(kTref);
    const double S_298 = ref_298.entropia_abs(kTref);
    curva.molar_mass_g = (ref_298.masa_molar > 1e-3) ? ref_298.masa_molar : 100.0;
    curva.T_min_db_C = polimorfos.front().T_min - 273.15;
    curva.T_max_db_C = polimorfos.back().T_max - 273.15;

    curva.points.reserve(static_cast<size_t>(n_steps) + 1);
    const nucleo::Especie* prev_sp = nullptr;
    double prev_T = t_lo;

    for (int i = 0; i <= n_steps; ++i) {
        const double T = (i == 0) ? t_lo : (t_lo + static_cast<double>(i) * dt);
        const auto& act = seleccionar_polimorfo_activo(polimorfos, T);

        if (prev_sp != nullptr && prev_sp->formula != act.formula) {
            const double T_tr = refinar_temperatura_transicion(*prev_sp, act, prev_T, T);
            domain::TransicionFasePunto tp;
            tp.T_K = T_tr;
            tp.delta_H_kJ = (act.entalpia_abs(T_tr) - prev_sp->entalpia_abs(T_tr)) * multiplicador;
            tp.delta_S_J_K = (act.entropia_abs(T_tr) - prev_sp->entropia_abs(T_tr)) * multiplicador;
            tp.from_phase = prev_sp->formula;
            tp.to_phase = act.formula;
            curva.phase_transitions.push_back(tp);
        }
        prev_sp = &act;
        prev_T = T;

        domain::PuntoPropiedadTermo pt;
        pt.T = T;
        pt.H = act.entalpia_abs(T) * multiplicador;
        pt.dH298 = (act.entalpia_abs(T) - H_298) * multiplicador;
        pt.S = act.entropia_abs(T) * multiplicador;
        pt.Cp = act.cp(T) * multiplicador;
        pt.G = act.energia_gibbs(T) * multiplicador;
        pt.Ex = ((act.entalpia_abs(T) - H_298) - kTref * (act.entropia_abs(T) - S_298) / 1000.0) * multiplicador;
        pt.active_phase = act.formula;
        pt.en_rango = act.en_rango_temperatura(T);
        if (!pt.en_rango && curva.warnings.size() < 10) {
            curva.warnings.push_back(act.comprobar_rango(T).advertencia);
        }

        if (es_elem_puro) {
            pt.dHf = 0.0;
            pt.dSf = 0.0;
            pt.dGf = 0.0;
            pt.log10K = 0.0;
        } else {
            double H_elems = 0.0;
            double S_elems = 0.0;
            double G_elems = 0.0;
            for (const auto& a : atomos) {
                double h_e = 0.0;
                double s_e = 0.0;
                double g_e = 0.0;
                evaluar_elemento_referencia(repo, a.elem, T, h_e, s_e, g_e);
                H_elems += static_cast<double>(a.cuenta) * h_e;
                S_elems += static_cast<double>(a.cuenta) * s_e;
                G_elems += static_cast<double>(a.cuenta) * g_e;
            }
            pt.dHf = (act.entalpia_abs(T) - H_elems) * multiplicador;
            pt.dSf = (act.entropia_abs(T) - S_elems) * multiplicador;
            pt.dGf = (act.energia_gibbs(T) - G_elems) * multiplicador;
            if (std::abs(pt.dGf) < 1e-11) {
                pt.dGf = 0.0;
            }
            pt.log10K = -pt.dGf * 1000.0 / (kR_J * T * kLn10);
        }
        curva.points.push_back(pt);
    }

    return curva;
}

domain::CurvaDiagramaTermo PropertyDiagrams::calcular_curva_reaccion(
    const domain::ThermoRepository& repo,
    const std::string& etiqueta,
    const std::vector<TerminoReaccionDiagrama>& terminos,
    domain::EllinghamNormalizationBasis basis,
    double T_min_K,
    double T_max_K,
    int steps,
    bool usar_envolvente_fases) const {
    domain::CurvaDiagramaTermo curva;
    curva.label = etiqueta;
    if (terminos.empty()) {
        return curva;
    }

    std::vector<TerminoReaccionDiagrama> norm_terms = terminos;
    double norm_factor = 1.0;
    const std::string target_gas = gas_objetivo_para_base(basis);

    if (!target_gas.empty()) {
        double nu_gas = 0.0;
        for (const auto& t : terminos) {
            if (clean_formula_str(t.formula) == target_gas) {
                nu_gas += t.coef;
            }
        }
        if (std::abs(nu_gas) > 1e-12) {
            // Convencion Ellingham: el gas reactivo (O2, S2, Cl2, CO2, SO3, H2O) queda con nu = -1
            norm_factor = -1.0 / nu_gas;
            for (auto& t : norm_terms) {
                t.coef *= norm_factor;
            }
        }
    }

    curva.norm_factor = std::abs(norm_factor);
    curva.equation_normalized = formatear_ecuacion_normalizada(norm_terms);
    if (curva.label.empty()) {
        curva.label = curva.equation_normalized;
    }

    struct TermResolved {
        TerminoReaccionDiagrama term;
        std::vector<nucleo::Especie> polys;
        double H_298 = 0.0;
        const nucleo::Especie* prev_sp = nullptr;
    };

    std::vector<TermResolved> resolved;
    resolved.reserve(norm_terms.size());
    for (const auto& t : norm_terms) {
        auto polys = resolver_polimorfos(repo, t.formula, usar_envolvente_fases);
        if (polys.empty()) {
            return domain::CurvaDiagramaTermo{};
        }
        const auto& sp298 = seleccionar_polimorfo_activo(polys, kTref);
        resolved.push_back({t, std::move(polys), sp298.entalpia_abs(kTref), nullptr});
    }

    const int n_steps = std::max(2, steps);
    const double t_lo = std::max(50.0, std::min(T_min_K, T_max_K));
    const double t_hi = std::max(t_lo + 1.0, std::max(T_min_K, T_max_K));
    const double dt = (t_hi - t_lo) / static_cast<double>(n_steps);

    curva.points.reserve(static_cast<size_t>(n_steps) + 1);
    double prev_T = t_lo;

    for (int i = 0; i <= n_steps; ++i) {
        const double T = (i == 0) ? t_lo : (t_lo + static_cast<double>(i) * dt);
        double dH = 0.0;
        double dH298 = 0.0;
        double dS = 0.0;
        double dCp = 0.0;
        double dG = 0.0;
        std::string active_desc;

        for (auto& r : resolved) {
            const auto& act = seleccionar_polimorfo_activo(r.polys, T);
            if (r.prev_sp != nullptr && r.prev_sp->formula != act.formula) {
                const double T_tr = refinar_temperatura_transicion(*r.prev_sp, act, prev_T, T);
                domain::TransicionFasePunto tp;
                tp.T_K = T_tr;
                tp.delta_H_kJ = std::abs(r.term.coef) *
                                (act.entalpia_abs(T_tr) - r.prev_sp->entalpia_abs(T_tr));
                tp.delta_S_J_K = std::abs(r.term.coef) *
                                 (act.entropia_abs(T_tr) - r.prev_sp->entropia_abs(T_tr));
                tp.from_phase = r.prev_sp->formula;
                tp.to_phase = act.formula;
                curva.phase_transitions.push_back(tp);
            }
            r.prev_sp = &act;

            dH += r.term.coef * act.entalpia_abs(T);
            dH298 += r.term.coef * (act.entalpia_abs(T) - r.H_298);
            dS += r.term.coef * act.entropia_abs(T);
            dCp += r.term.coef * act.cp(T);
            dG += r.term.coef * act.energia_gibbs(T);
            if (!active_desc.empty()) {
                active_desc += ", ";
            }
            active_desc += act.formula;
        }
        prev_T = T;

        domain::PuntoPropiedadTermo pt;
        pt.T = T;
        pt.H = dH;
        pt.dH298 = dH298;
        pt.S = dS;
        pt.Cp = dCp;
        pt.G = dG;
        pt.dGf = dG;
        pt.dHf = dH;
        pt.dSf = dS;
        pt.log10K = -dG * 1000.0 / (kR_J * T * kLn10);
        pt.active_phase = active_desc;
        curva.points.push_back(pt);
    }

    return curva;
}

domain::CurvaDiagramaTermo PropertyDiagrams::calcular_curva_ecuacion_texto(
    const domain::ThermoRepository& repo,
    const std::string& ecuacion_texto,
    domain::EllinghamNormalizationBasis basis,
    double T_min_K,
    double T_max_K,
    int steps,
    bool usar_envolvente_fases) const {
    auto terminos = parsear_ecuacion_quimica(ecuacion_texto);
    if (terminos.empty()) {
        return domain::CurvaDiagramaTermo{};
    }
    return calcular_curva_reaccion(repo, trim_str(ecuacion_texto), terminos, basis,
                                   T_min_K, T_max_K, steps, usar_envolvente_fases);
}

domain::ResultadoDiagramaPropiedades PropertyDiagrams::generar_familia_ellingham(
    const domain::ThermoRepository& repo,
    domain::EllinghamPresetFamily familia,
    double T_min_K,
    double T_max_K,
    int steps,
    bool usar_envolvente_fases) const {
    domain::ResultadoDiagramaPropiedades res;
    res.mode = domain::PropertyDiagramMode::EllinghamReactions;
    res.diagram_type = domain::PropertyDiagramType::DeltaG_Ellingham;
    res.T_min_K = T_min_K;
    res.T_max_K = T_max_K;

    std::vector<std::string> ecuaciones;
    switch (familia) {
    case domain::EllinghamPresetFamily::MetallurgicalOxides:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleO2;
        ecuaciones = {
            "2 Ca + O2(g) = 2 CaO",
            "2 Mg + O2(g) = 2 MgO",
            "4/3 Al + O2(g) = 2/3 Al2O3",
            "Ti + O2(g) = TiO2",
            "Si + O2(g) = SiO2",
            "2 Mn + O2(g) = 2 MnO",
            "4/3 Cr + O2(g) = 2/3 Cr2O3",
            "2 Zn + O2(g) = 2 ZnO",
            "2 Fe + O2(g) = 2 FeO",
            "3/2 Fe + O2(g) = 1/2 Fe3O4",
            "2 Ni + O2(g) = 2 NiO",
            "2 Pb + O2(g) = 2 PbO",
            "4 Cu + O2(g) = 2 Cu2O",
            "2 C + O2(g) = 2 CO(g)",
            "C + O2(g) = CO2(g)",
            "2 CO(g) + O2(g) = 2 CO2(g)",
            "2 H2(g) + O2(g) = 2 H2O(g)"
        };
        break;
    case domain::EllinghamPresetFamily::SulfideRoasting:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleS2;
        ecuaciones = {
            "4 Cu + S2(g) = 2 Cu2S",
            "2 Fe + S2(g) = 2 FeS",
            "2 Zn + S2(g) = 2 ZnS",
            "2 Pb + S2(g) = 2 PbS",
            "Mo + S2(g) = MoS2",
            "2 H2(g) + S2(g) = 2 H2S(g)",
            "S2(g) + 2 O2(g) = 2 SO2(g)"
        };
        break;
    case domain::EllinghamPresetFamily::ChlorinationMetallurgy:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleCl2;
        ecuaciones = {
            "Ca + Cl2(g) = CaCl2",
            "Mg + Cl2(g) = MgCl2",
            "2/3 Al + Cl2(g) = 2/3 AlCl3",
            "1/2 Ti + Cl2(g) = 1/2 TiCl4",
            "Zn + Cl2(g) = ZnCl2",
            "Fe + Cl2(g) = FeCl2",
            "Pb + Cl2(g) = PbCl2",
            "H2(g) + Cl2(g) = 2 HCl(g)"
        };
        break;
    case domain::EllinghamPresetFamily::CarbonateCalcination:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleCO2;
        ecuaciones = {
            "CaO + CO2(g) = CaCO3",
            "MgO + CO2(g) = MgCO3",
            "FeO + CO2(g) = FeCO3",
            "ZnO + CO2(g) = ZnCO3"
        };
        break;
    case domain::EllinghamPresetFamily::SulfateDecomposition:
        res.basis = domain::EllinghamNormalizationBasis::PerMoleSO3;
        ecuaciones = {
            "CaO + SO3(g) = CaSO4",
            "MgO + SO3(g) = MgSO4",
            "ZnO + SO3(g) = ZnSO4",
            "CuO + SO3(g) = CuSO4"
        };
        break;
    }

    for (const auto& eq : ecuaciones) {
        auto c = calcular_curva_ecuacion_texto(repo, eq, res.basis, T_min_K, T_max_K,
                                               steps, usar_envolvente_fases);
        if (!c.points.empty()) {
            res.curves.push_back(std::move(c));
        }
    }

    res.intersections = encontrar_intersecciones(res.curves, res.diagram_type);
    if (res.basis == domain::EllinghamNormalizationBasis::PerMoleO2) {
        generar_lineas_isopotenciales(repo, T_min_K, T_max_K,
                                      res.iso_pO2_lines, res.iso_CO_CO2_lines, res.iso_H2_H2O_lines);
    }

    std::ostringstream log;
    log << "Diagrama Ellingham generado: " << res.curves.size() << " curvas, "
        << res.intersections.size() << " intersecciones en [" << std::fixed << std::setprecision(1)
        << T_min_K << " K, " << T_max_K << " K].";
    res.summary_log = log.str();
    return res;
}

std::vector<domain::InterseccionEllingham> PropertyDiagrams::encontrar_intersecciones(
    const std::vector<domain::CurvaDiagramaTermo>& curvas,
    domain::PropertyDiagramType tipo) const {
    std::vector<domain::InterseccionEllingham> out;
    if (curvas.size() < 2) {
        return out;
    }

    for (size_t a = 0; a < curvas.size(); ++a) {
        if (!curvas[a].visible || curvas[a].points.size() < 2) {
            continue;
        }
        for (size_t b = a + 1; b < curvas.size(); ++b) {
            if (!curvas[b].visible || curvas[b].points.size() != curvas[a].points.size()) {
                continue;
            }
            const size_t n = curvas[a].points.size();
            for (size_t i = 1; i < n; ++i) {
                const double ya0 = curvas[a].evaluar_propiedad(tipo, i - 1, false);
                const double ya1 = curvas[a].evaluar_propiedad(tipo, i, false);
                const double yb0 = curvas[b].evaluar_propiedad(tipo, i - 1, false);
                const double yb1 = curvas[b].evaluar_propiedad(tipo, i, false);
                const double d0 = ya0 - yb0;
                const double d1 = ya1 - yb1;

                if (((d0 <= 0.0 && d1 > 0.0) || (d0 >= 0.0 && d1 < 0.0)) &&
                    std::abs(d1 - d0) > 1e-12) {
                    const double alpha = -d0 / (d1 - d0);
                    const double t0 = curvas[a].points[i - 1].T;
                    const double t1 = curvas[a].points[i].T;
                    const double T_K = t0 + alpha * (t1 - t0);
                    const double val_Y = ya0 + alpha * (ya1 - ya0);
                    const double dG_a = curvas[a].points[i - 1].dGf +
                                        alpha * (curvas[a].points[i].dGf - curvas[a].points[i - 1].dGf);

                    domain::InterseccionEllingham inter;
                    inter.curve_a = curvas[a].label;
                    inter.curve_b = curvas[b].label;
                    inter.T_K = T_K;
                    inter.T_C = T_K - 273.15;
                    inter.value_Y = val_Y;
                    inter.log10_pO2 = (dG_a * 1000.0) / (kR_J * T_K * kLn10);

                    const double dG_CO_CO2 = -565.0 + 0.173 * T_K;
                    const double dG_H2_H2O = -492.0 + 0.110 * T_K;
                    inter.log10_CO_CO2 = ((dG_CO_CO2 - dG_a) * 1000.0) / (2.0 * kR_J * T_K * kLn10);
                    inter.log10_H2_H2O = ((dG_H2_H2O - dG_a) * 1000.0) / (2.0 * kR_J * T_K * kLn10);

                    const std::string& lower_above = (d1 < 0.0) ? curvas[a].label : curvas[b].label;
                    const std::string& higher_above = (d1 < 0.0) ? curvas[b].label : curvas[a].label;
                    std::ostringstream interp;
                    interp << "Por encima de T = " << std::fixed << std::setprecision(1) << T_K
                           << " K (" << inter.T_C << " °C), la curva [" << lower_above
                           << "] reduce termodinámicamente a [" << higher_above << "].";
                    inter.interpretation = interp.str();
                    out.push_back(std::move(inter));
                }
            }
        }
    }

    std::sort(out.begin(), out.end(), [](const auto& x, const auto& y) {
        return x.T_K < y.T_K;
    });
    return out;
}

domain::EscalaNomograficaPunto PropertyDiagrams::evaluar_escala_nomografica(
    const domain::ThermoRepository& repo,
    double T_K,
    double delta_G_kJ_per_O2) const {
    domain::EscalaNomograficaPunto pt;
    const double T = std::max(50.0, T_K);
    pt.T_K = T;
    pt.delta_G_kJ_per_O2 = delta_G_kJ_per_O2;

    const double rt_ln10_kJ = kR_kJ * T * kLn10;
    pt.log10_pO2 = delta_G_kJ_per_O2 / rt_ln10_kJ;
    pt.pO2_atm = std::pow(10.0, std::clamp(pt.log10_pO2, -100.0, 100.0));

    double dG_CO_CO2 = 0.0;
    double dG_H2_H2O = 0.0;
    evaluar_reacciones_escala_gases(repo, T, dG_CO_CO2, dG_H2_H2O);

    pt.log10_CO_CO2 = (dG_CO_CO2 - delta_G_kJ_per_O2) / (2.0 * rt_ln10_kJ);
    pt.ratio_CO_CO2 = std::pow(10.0, std::clamp(pt.log10_CO_CO2, -50.0, 50.0));

    pt.log10_H2_H2O = (dG_H2_H2O - delta_G_kJ_per_O2) / (2.0 * rt_ln10_kJ);
    pt.ratio_H2_H2O = std::pow(10.0, std::clamp(pt.log10_H2_H2O, -50.0, 50.0));
    return pt;
}

void PropertyDiagrams::generar_lineas_isopotenciales(
    const domain::ThermoRepository& repo,
    double T_min_K,
    double T_max_K,
    std::vector<domain::LineaIsoPotencial>& iso_pO2,
    std::vector<domain::LineaIsoPotencial>& iso_CO_CO2,
    std::vector<domain::LineaIsoPotencial>& iso_H2_H2O) const {
    iso_pO2.clear();
    iso_CO_CO2.clear();
    iso_H2_H2O.clear();

    const double t0 = std::max(50.0, std::min(T_min_K, T_max_K));
    const double t1 = std::max(t0 + 1.0, std::max(T_min_K, T_max_K));

    static const double log_pO2_vals[] = {-2, -4, -6, -8, -10, -12, -14, -16, -20, -24, -28};
    for (double lp : log_pO2_vals) {
        domain::LineaIsoPotencial line;
        std::ostringstream lbl;
        lbl << "log10(pO2) = " << static_cast<int>(lp);
        line.label = lbl.str();
        line.parameter_value = lp;
        line.T_K = {t0, t1};
        line.delta_G_kJ = {kR_kJ * t0 * kLn10 * lp, kR_kJ * t1 * kLn10 * lp};
        iso_pO2.push_back(std::move(line));
    }

    double dG_CO_0 = 0.0;
    double dG_H2_0 = 0.0;
    double dG_CO_1 = 0.0;
    double dG_H2_1 = 0.0;
    evaluar_reacciones_escala_gases(repo, t0, dG_CO_0, dG_H2_0);
    evaluar_reacciones_escala_gases(repo, t1, dG_CO_1, dG_H2_1);

    static const double log_ratio_vals[] = {-4, -3, -2, -1, 0, 1, 2, 3, 4};
    for (double lr : log_ratio_vals) {
        domain::LineaIsoPotencial l_co;
        std::ostringstream lbl_co;
        lbl_co << "log10(CO/CO2) = " << static_cast<int>(lr);
        l_co.label = lbl_co.str();
        l_co.parameter_value = lr;
        l_co.T_K = {t0, t1};
        l_co.delta_G_kJ = {dG_CO_0 - 2.0 * kR_kJ * t0 * kLn10 * lr,
                           dG_CO_1 - 2.0 * kR_kJ * t1 * kLn10 * lr};
        iso_CO_CO2.push_back(std::move(l_co));

        domain::LineaIsoPotencial l_h2;
        std::ostringstream lbl_h2;
        lbl_h2 << "log10(H2/H2O) = " << static_cast<int>(lr);
        l_h2.label = lbl_h2.str();
        l_h2.parameter_value = lr;
        l_h2.T_K = {t0, t1};
        l_h2.delta_G_kJ = {dG_H2_0 - 2.0 * kR_kJ * t0 * kLn10 * lr,
                           dG_H2_1 - 2.0 * kR_kJ * t1 * kLn10 * lr};
        iso_H2_H2O.push_back(std::move(l_h2));
    }
}

double PropertyDiagrams::balancear_cantidad_elemento(
    const std::string& formula,
    const std::string& elemento,
    double cantidad_objetivo) const {
    const std::string clean_f = clean_formula_str(formula);
    const std::string elem_trim = trim_str(elemento);
    if (clean_f.empty() || elem_trim.empty() || cantidad_objetivo <= 0.0) {
        return 1.0;
    }
    const auto atomos = nucleo::TablaPeriodica::instancia().analizar_formula(clean_f);
    for (const auto& a : atomos) {
        if (a.elem == elem_trim && a.cuenta > 1e-9) {
            return cantidad_objetivo / static_cast<double>(a.cuenta);
        }
    }
    return 1.0;
}

std::string PropertyDiagrams::exportar_csv(
    const std::vector<domain::CurvaDiagramaTermo>& curvas,
    const std::vector<domain::InterseccionEllingham>& intersecciones,
    domain::PropertyDiagramType tipo,
    bool temp_en_celsius,
    bool energia_en_kcal) const {
    std::ostringstream os;
    os << std::fixed << std::setprecision(4);
    os << "[CURVES]\n";
    os << (temp_en_celsius ? "T_C" : "T_K");
    for (const auto& c : curvas) {
        if (c.visible) {
            os << ",\"" << c.label << "\"";
        }
    }
    os << "\n";

    size_t max_pts = 0;
    for (const auto& c : curvas) {
        if (c.visible) {
            max_pts = std::max(max_pts, c.points.size());
        }
    }

    for (size_t i = 0; i < max_pts; ++i) {
        double T_val = 0.0;
        for (const auto& c : curvas) {
            if (c.visible && i < c.points.size()) {
                T_val = temp_en_celsius ? (c.points[i].T - 273.15) : c.points[i].T;
                break;
            }
        }
        os << T_val;
        for (const auto& c : curvas) {
            if (!c.visible) {
                continue;
            }
            if (i < c.points.size()) {
                os << "," << c.evaluar_propiedad(tipo, i, energia_en_kcal);
            } else {
                os << ",";
            }
        }
        os << "\n";
    }

    os << "\n[INTERSECTIONS]\n";
    os << "Curve_A,Curve_B,T_K,T_C,Value_Y,log10_pO2,log10_CO_CO2,log10_H2_H2O,Interpretation\n";
    const double k_faktor = (energia_en_kcal && tipo != domain::PropertyDiagramType::Log10K)
                                ? (1.0 / 4.184)
                                : 1.0;
    for (const auto& inter : intersecciones) {
        os << "\"" << inter.curve_a << "\",\"" << inter.curve_b << "\","
           << inter.T_K << "," << inter.T_C << "," << (inter.value_Y * k_faktor) << ","
           << inter.log10_pO2 << "," << inter.log10_CO_CO2 << "," << inter.log10_H2_H2O << ",\""
           << inter.interpretation << "\"\n";
    }
    return os.str();
}

} // namespace cma::application::modules
