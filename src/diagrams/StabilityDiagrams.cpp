#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"
#include "StabilityDiagrams.hpp"

#include "nucleo/tabla_periodica.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <fstream>
#include <functional>
#include <iomanip>
#include <map>
#include <set>
#include <sstream>

namespace cma::application::modules {

namespace {

constexpr double kR_kJ = 0.008314462618;
constexpr double kLn10 = 2.302585092994046;

struct FaseCondensada {
    std::string nombre;
    double nu_metal = 1.0;
    double nu_O2 = 0.0;
    double nu_SO2 = 0.0;
    std::function<double(double)> delta_G_rxn;
};

std::vector<FaseCondensada> fases_para_metal(const std::string& metal) {
    std::string m = metal;
    for (char& c : m)
        c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));

    if (m == "cu" || m == "cobre" || m == "copper") {
        return {{"Cu", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
                {"Cu2O", 2.0, 0.5, 0.0, [](double T) { return -169.5 + 0.0712 * T; }},
                {"CuO", 1.0, 0.5, 0.0, [](double T) { return -155.2 + 0.0874 * T; }},
                {"Cu2S", 2.0, -1.0, 1.0, [](double T) { return 230.8 - 0.0435 * T; }},
                {"CuSO4", 1.0, 1.0, 1.0, [](double T) { return -771.4 + 0.2862 * T; }},
                {"CuO*CuSO4", 2.0, 1.5, 1.0, [](double T) { return -935.5 + 0.3805 * T; }}};
    }
    if (m == "fe" || m == "hierro" || m == "iron") {
        return {{"Fe", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
                {"FeO", 1.0, 0.5, 0.0, [](double T) { return -263.7 + 0.0644 * T; }},
                {"Fe3O4", 3.0, 2.0, 0.0, [](double T) { return -1102.5 + 0.3075 * T; }},
                {"Fe2O3", 2.0, 1.5, 0.0, [](double T) { return -813.0 + 0.2470 * T; }},
                {"FeS", 1.0, -1.0, 1.0, [](double T) { return 212.0 - 0.0402 * T; }},
                {"FeSO4", 1.0, 1.0, 1.0, [](double T) { return -928.0 + 0.3450 * T; }}};
    }
    if (m == "ni" || m == "niquel" || m == "nickel") {
        return {{"Ni", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
                {"NiO", 1.0, 0.5, 0.0, [](double T) { return -239.7 + 0.0845 * T; }},
                {"Ni3S2", 3.0, -2.0, 2.0, [](double T) { return 442.0 - 0.0850 * T; }},
                {"NiSO4", 1.0, 1.0, 1.0, [](double T) { return -872.4 + 0.3485 * T; }}};
    }
    if (m == "pb" || m == "plomo" || m == "lead") {
        return {{"Pb", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
                {"PbO", 1.0, 0.5, 0.0, [](double T) { return -217.3 + 0.0988 * T; }},
                {"PbS", 1.0, -1.0, 1.0, [](double T) { return 264.0 - 0.0520 * T; }},
                {"PbSO4", 1.0, 1.0, 1.0, [](double T) { return -918.4 + 0.3640 * T; }},
                {"PbO*PbSO4", 2.0, 1.5, 1.0, [](double T) { return -1141.0 + 0.4680 * T; }}};
    }
    if (m == "zn" || m == "zinc") {
        return {{"Zn", 1.0, 0.0, 0.0, [](double) { return 0.0; }},
                {"ZnO", 1.0, 0.5, 0.0, [](double T) { return -348.3 + 0.1005 * T; }},
                {"ZnS", 1.0, -1.0, 1.0, [](double T) { return 157.0 - 0.0380 * T; }},
                {"ZnSO4", 1.0, 1.0, 1.0, [](double T) { return -978.0 + 0.3620 * T; }},
                {"ZnO*2ZnSO4", 3.0, 2.5, 2.0, [](double T) { return -2310.0 + 0.8250 * T; }}};
    }
    return {};
}

std::map<std::string, double> parse_atom_counts(const std::string& formula) {
    std::map<std::string, double> counts;
    auto atoms = cma::nucleo::TablaPeriodica::instancia().analizar_formula(formula);
    for (const auto& a : atoms) {
        if (!a.elem.empty() && a.cuenta > 0) {
            counts[a.elem] += static_cast<double>(a.cuenta);
        }
    }
    return counts;
}

// Resuelve el sistema estequiométrico 2x2 para expresar los elementos no metálicos (E1, E2)
// de una especie en términos de los dos gases de referencia (Gas 1, Gas 2):
// [ g1_e1  g2_e1 ] [ nu_g1 ] = [ sp_e1 ]
// [ g1_e2  g2_e2 ] [ nu_g2 ]   [ sp_e2 ]
struct GasBasisSystem {
    std::string main_elem;
    std::string elem1;
    std::string elem2;
    double g1_e1 = 0.0, g1_e2 = 0.0;
    double g2_e1 = 0.0, g2_e2 = 0.0;
    double det = 0.0;
    bool valid = false;

    bool solve_gas_stoich(double sp_e1, double sp_e2, double& nu_g1, double& nu_g2) const {
        if (!valid || std::abs(det) < 1e-12)
            return false;
        nu_g1 = (sp_e1 * g2_e2 - sp_e2 * g2_e1) / det;
        nu_g2 = (g1_e1 * sp_e2 - g1_e2 * sp_e1) / det;
        return true;
    }
};

GasBasisSystem build_gas_basis(const std::string& main_formula, const std::string& gas1_formula,
                               const std::string& gas2_formula) {
    GasBasisSystem sys;
    const auto m_map = parse_atom_counts(main_formula);
    sys.main_elem = m_map.empty() ? main_formula : m_map.begin()->first;

    const auto g1_map = parse_atom_counts(gas1_formula);
    const auto g2_map = parse_atom_counts(gas2_formula);

    std::vector<std::string> non_metals;
    for (const auto& [el, c] : g1_map) {
        if (el != sys.main_elem && std::find(non_metals.begin(), non_metals.end(), el) == non_metals.end()) {
            non_metals.push_back(el);
        }
    }
    for (const auto& [el, c] : g2_map) {
        if (el != sys.main_elem && std::find(non_metals.begin(), non_metals.end(), el) == non_metals.end()) {
            non_metals.push_back(el);
        }
    }

    if (non_metals.empty())
        return sys;
    sys.elem1 = non_metals[0];
    sys.elem2 = (non_metals.size() > 1) ? non_metals[1] : "";

    sys.g1_e1 = g1_map.count(sys.elem1) ? g1_map.at(sys.elem1) : 0.0;
    sys.g1_e2 = (!sys.elem2.empty() && g1_map.count(sys.elem2)) ? g1_map.at(sys.elem2) : 0.0;
    sys.g2_e1 = g2_map.count(sys.elem1) ? g2_map.at(sys.elem1) : 0.0;
    sys.g2_e2 = (!sys.elem2.empty() && g2_map.count(sys.elem2)) ? g2_map.at(sys.elem2) : 0.0;

    if (sys.elem2.empty()) {
        // Caso degenerado de 1 solo elemento gaseoso
        sys.g2_e2 = 1.0;
    }

    sys.det = sys.g1_e1 * sys.g2_e2 - sys.g1_e2 * sys.g2_e1;
    sys.valid = std::abs(sys.det) > 1e-9;
    return sys;
}

const cma::nucleo::Especie* find_gas_species(const domain::ThermoRepository* repo,
                                             const std::string& gas_formula) {
    if (!repo)
        return nullptr;
    const auto all = repo->todas_especies();
    for (const auto* e : all) {
        if (e && e->formula == gas_formula && e->fase_std == cma::nucleo::Fase::Gas)
            return e;
    }
    for (const auto* e : all) {
        if (e && e->formula == gas_formula)
            return e;
    }
    return nullptr;
}

std::string format_reaction(const std::string& main_el, double n_main, const std::string& gas1, double nu_g1,
                            const std::string& gas2, double nu_g2, const std::string& product) {
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(2);
    oss << n_main << " " << main_el;
    auto append_side = [&](double coeff, const std::string& f) {
        if (coeff > 1e-6)
            oss << " + " << coeff << " " << f << "(g)";
    };
    append_side(nu_g1, gas1);
    append_side(nu_g2, gas2);
    oss << " = " << product;
    if (nu_g1 < -1e-6)
        oss << " + " << (-nu_g1) << " " << gas1 << "(g)";
    if (nu_g2 < -1e-6)
        oss << " + " << (-nu_g2) << " " << gas2 << "(g)";
    return oss.str();
}

struct Vec2d {
    double x = 0.0;
    double y = 0.0;
};

// Recorta un polígono convexo contra el semiplano A*x + B*y + C <= 0 (Sutherland-Hodgman)
std::vector<Vec2d> clip_polygon_halfplane(const std::vector<Vec2d>& poly, double A, double B, double C) {
    if (poly.empty())
        return poly;
    std::vector<Vec2d> res;
    auto eval = [=](const Vec2d& pt) { return A * pt.x + B * pt.y + C; };
    constexpr double kEps = 1e-11;

    for (size_t i = 0; i < poly.size(); ++i) {
        const Vec2d cur = poly[i];
        const Vec2d prev = poly[(i == 0) ? (poly.size() - 1) : (i - 1)];
        const double d_cur = eval(cur);
        const double d_prev = eval(prev);

        if (d_cur <= kEps) {
            if (d_prev > kEps) {
                const double t = d_prev / (d_prev - d_cur);
                res.push_back({prev.x + t * (cur.x - prev.x), prev.y + t * (cur.y - prev.y)});
            }
            res.push_back(cur);
        } else if (d_prev <= kEps) {
            const double t = d_prev / (d_prev - d_cur);
            res.push_back({prev.x + t * (cur.x - prev.x), prev.y + t * (cur.y - prev.y)});
        }
    }
    return res;
}

} // namespace

// ═══════════════════════════════════════════════════════════════════════════
// API Clásica (Compatibilidad con Tests)
// ═══════════════════════════════════════════════════════════════════════════

std::vector<PuntoEstabilidad> StabilityDiagrams::generar(const std::string& metal, double T) const {
    return generar(metal, T, -30.0, 0.0, 60, -15.0, 5.0, 40);
}

std::vector<PuntoEstabilidad> StabilityDiagrams::generar(const std::string& metal, double T,
                                                         double log_pO2_min, double log_pO2_max, int steps_O2,
                                                         double log_pSO2_min, double log_pSO2_max,
                                                         int steps_SO2) const {
    std::vector<PuntoEstabilidad> puntos;
    if (!std::isfinite(T) || T < 300.0 || T > 2500.0)
        return puntos;
    if (steps_O2 < 2 || steps_SO2 < 2 || log_pO2_min >= log_pO2_max || log_pSO2_min >= log_pSO2_max)
        return puntos;

    auto fases = fases_para_metal(metal);
    if (fases.empty())
        return puntos;

    const double RT_ln10 = kR_kJ * T * kLn10;
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
                double g_norm =
                    (dG - (fase.nu_O2 * RT_ln10 * lp_O2) - (fase.nu_SO2 * RT_ln10 * lp_SO2)) / fase.nu_metal;
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

// ═══════════════════════════════════════════════════════════════════════════
// Filtrado y Balance Estequiométrico 2x2 de Especies del Sistema
// ═══════════════════════════════════════════════════════════════════════════

std::vector<domain::StabilitySpeciesSelection>
StabilityDiagrams::filtrar_especies_sistema(const std::string& main_element, const std::string& gas1_formula,
                                            const std::string& gas2_formula, bool solo_condensadas) const {
    std::vector<domain::StabilitySpeciesSelection> out;
    const auto basis = build_gas_basis(main_element, gas1_formula, gas2_formula);
    if (basis.main_elem.empty())
        return out;

    if (repo_) {
        const auto all = repo_->todas_especies();
        std::set<std::string> seen_formulas;

        for (const auto* esp : all) {
            if (!esp || esp->id > 445 || std::abs(esp->cp_c) > 1.0)
                continue;
            if (solo_condensadas &&
                (esp->fase_std == cma::nucleo::Fase::Gas || esp->fase_std == cma::nucleo::Fase::Mixto ||
                 esp->formula.find("(+") != std::string::npos ||
                 esp->formula.find("(-") != std::string::npos ||
                 esp->formula.find("(a)") != std::string::npos ||
                 esp->formula.find("(aq)") != std::string::npos)) {
                continue;
            }
            const auto cmap = parse_atom_counts(esp->formula);
            if (cmap.empty() || !cmap.count(basis.main_elem))
                continue;

            bool alien_element = false;
            for (const auto& [el, cnt] : cmap) {
                if (el != basis.main_elem && el != basis.elem1 && el != basis.elem2) {
                    alien_element = true;
                    break;
                }
            }
            if (alien_element)
                continue;

            // Evitar duplicados polimórficos con misma fórmula salvo que aporten la fase base
            if (seen_formulas.count(esp->formula))
                continue;
            seen_formulas.insert(esp->formula);

            const double n_m = cmap.at(basis.main_elem);
            const double n_e1 = cmap.count(basis.elem1) ? cmap.at(basis.elem1) : 0.0;
            const double n_e2 =
                (!basis.elem2.empty() && cmap.count(basis.elem2)) ? cmap.at(basis.elem2) : 0.0;

            double nu_g1 = 0.0, nu_g2 = 0.0;
            if (basis.valid) {
                basis.solve_gas_stoich(n_e1, n_e2, nu_g1, nu_g2);
            }

            domain::StabilitySpeciesSelection item;
            item.id = esp->id;
            item.formula = esp->formula;
            item.name = esp->nombre;
            item.phase_label = cma::nucleo::nombre_fase(esp->fase_std);
            item.n_main = n_m;
            item.n_elem1 = n_e1;
            item.n_elem2 = n_e2;
            item.nu_gas1 = nu_g1;
            item.nu_gas2 = nu_g2;
            item.activity = 1.0;
            item.selected = true;
            item.is_gas = (esp->fase_std == cma::nucleo::Fase::Gas);
            out.push_back(item);
        }
    }

    // Complementar con las fases analíticas de referencia (p.ej. oxisulfatos CuO*CuSO4, PbO*PbSO4,
    // ZnO*2ZnSO4)
    if (basis.elem1 == "O" && basis.elem2 == "S" && gas1_formula == "O2" && gas2_formula == "SO2") {
        auto ref_fases = fases_para_metal(basis.main_elem);
        int64_t synth_id = -100;
        for (const auto& rf : ref_fases) {
            bool exists =
                std::any_of(out.begin(), out.end(), [&](const auto& s) { return s.formula == rf.nombre; });
            if (!exists) {
                domain::StabilitySpeciesSelection item;
                item.id = synth_id--;
                item.formula = rf.nombre;
                item.name = rf.nombre + " (Kellogg Ref.)";
                item.phase_label = "Sólido";
                item.n_main = rf.nu_metal;
                item.nu_gas1 = rf.nu_O2;
                item.nu_gas2 = rf.nu_SO2;
                item.activity = 1.0;
                item.selected = true;
                out.push_back(item);
            }
        }
    }

    return out;
}

// ═══════════════════════════════════════════════════════════════════════════
// Motor Lpp (Isotérmico Vectorial de Kellogg)
// ═══════════════════════════════════════════════════════════════════════════

domain::LppDiagramResult StabilityDiagrams::calcular_lpp(const domain::LppDiagramRequest& req) const {
    domain::LppDiagramResult res;
    if (!std::isfinite(req.temperature_k) || req.temperature_k < 100.0 || req.temperature_k > 4000.0) {
        res.message = "Temperatura fuera del rango termodinámico válido (100..4000 K).";
        return res;
    }
    if (req.x_min >= req.x_max || req.y_min >= req.y_max) {
        res.message = "Los límites de los ejes log(p) son inválidos (min >= max).";
        return res;
    }

    const auto basis = build_gas_basis(req.main_element, req.gas1_formula, req.gas2_formula);
    if (!basis.valid) {
        res.message = "Los dos gases seleccionados no son linealmente independientes en su estequiometría.";
        return res;
    }

    auto candidates = req.candidates;
    if (candidates.empty()) {
        candidates = filtrar_especies_sistema(req.main_element, req.gas1_formula, req.gas2_formula, true);
    }

    const double T = req.temperature_k;
    const double RT = kR_kJ * T;
    const double RT_ln10 = RT * kLn10;

    const auto* esp_g1 = find_gas_species(repo_, req.gas1_formula);
    const auto* esp_g2 = find_gas_species(repo_, req.gas2_formula);
    const double mu0_g1 = esp_g1 ? esp_g1->energia_gibbs(T) : 0.0;
    const double mu0_g2 = esp_g2 ? esp_g2->energia_gibbs(T) : 0.0;

    // También buscamos la especie del elemento metálico puro para referencia de fases sintéticas
    const cma::nucleo::Especie* esp_metal_ref = nullptr;
    if (repo_) {
        for (const auto* e : repo_->todas_especies()) {
            if (e && e->formula == basis.main_elem && e->fase_std != cma::nucleo::Fase::Gas) {
                esp_metal_ref = e;
                break;
            }
        }
    }
    const double mu0_metal = esp_metal_ref ? esp_metal_ref->energia_gibbs(T) : 0.0;
    const auto ref_fases = fases_para_metal(basis.main_elem);

    for (const auto& cand : candidates) {
        if (!cand.selected || cand.n_main <= 0.0)
            continue;

        double nu_g1 = cand.nu_gas1;
        double nu_g2 = cand.nu_gas2;
        if (std::abs(nu_g1) < 1e-12 && std::abs(nu_g2) < 1e-12 &&
            (cand.n_elem1 > 0.0 || cand.n_elem2 > 0.0)) {
            basis.solve_gas_stoich(cand.n_elem1, cand.n_elem2, nu_g1, nu_g2);
        }

        double g_species = 0.0;
        bool has_g = false;
        if (repo_ && cand.id >= 0) {
            if (const auto* e = repo_->especie_por_id(cand.id)) {
                g_species = e->energia_gibbs(T);
                has_g = true;
            }
        }
        if (!has_g) {
            for (const auto& rf : ref_fases) {
                if (rf.nombre == cand.formula) {
                    // dG_rxn = G_prod - nu_M * G_M - nu_O2 * G_O2 - nu_SO2 * G_SO2
                    g_species =
                        rf.delta_G_rxn(T) + rf.nu_metal * mu0_metal + rf.nu_O2 * mu0_g1 + rf.nu_SO2 * mu0_g2;
                    nu_g1 = rf.nu_O2;
                    nu_g2 = rf.nu_SO2;
                    has_g = true;
                    break;
                }
            }
        }
        if (!has_g)
            continue;

        const double act = std::clamp(cand.activity, 1e-12, 10.0);
        const double g_corr = g_species + RT * std::log(act);

        domain::BalancedPhasePlane plane;
        plane.species_id = cand.id;
        plane.formula = cand.formula;
        plane.n_main = cand.n_main;
        plane.nu_gas1_per_metal = nu_g1 / cand.n_main;
        plane.nu_gas2_per_metal = nu_g2 / cand.n_main;
        plane.activity = act;
        plane.g_std_kj_mol = g_corr;
        plane.Z = (g_corr - nu_g1 * mu0_g1 - nu_g2 * mu0_g2) / cand.n_main;
        plane.A = -plane.nu_gas1_per_metal * RT_ln10;
        plane.B = -plane.nu_gas2_per_metal * RT_ln10;
        plane.formation_reaction = format_reaction(basis.main_elem, cand.n_main, req.gas1_formula, nu_g1,
                                                   req.gas2_formula, nu_g2, cand.formula);

        res.planes.push_back(plane);
    }

    if (res.planes.empty()) {
        res.message = "No hay especies activas seleccionadas para construir el diagrama Lpp.";
        return res;
    }

    // Recorte de polígonos de predominancia exactamente en la ventana [x_min, x_max] x [y_min, y_max]
    const std::vector<Vec2d> bbox = {
        {req.x_min, req.y_min}, {req.x_max, req.y_min}, {req.x_max, req.y_max}, {req.x_min, req.y_max}};

    for (size_t i = 0; i < res.planes.size(); ++i) {
        auto poly = bbox;
        const auto& pi = res.planes[i];
        for (size_t j = 0; j < res.planes.size(); ++j) {
            if (i == j)
                continue;
            const auto& pj = res.planes[j];
            poly = clip_polygon_halfplane(poly, pi.A - pj.A, pi.B - pj.B, pi.Z - pj.Z);
            if (poly.empty())
                break;
        }

        if (poly.size() >= 3) {
            double area2 = 0.0;
            double cx = 0.0, cy = 0.0;
            for (size_t k = 0; k < poly.size(); ++k) {
                const size_t nxt = (k + 1) % poly.size();
                const double cross = poly[k].x * poly[nxt].y - poly[nxt].x * poly[k].y;
                area2 += cross;
                cx += (poly[k].x + poly[nxt].x) * cross;
                cy += (poly[k].y + poly[nxt].y) * cross;
            }
            const double area = 0.5 * std::abs(area2);
            if (area < 1e-6)
                continue;

            domain::PhasePolygonLpp pf;
            pf.species_id = pi.species_id;
            pf.formula = pi.formula;
            pf.formation_reaction = pi.formation_reaction;
            pf.activity = pi.activity;
            pf.area_log2 = area;
            for (const auto& v : poly) {
                pf.vertices.push_back({v.x, v.y});
            }
            if (std::abs(area2) > 1e-9) {
                pf.centroid_x = cx / (3.0 * area2);
                pf.centroid_y = cy / (3.0 * area2);
            } else {
                pf.centroid_x = poly[0].x;
                pf.centroid_y = poly[0].y;
            }
            res.polygons.push_back(pf);
        }
    }

    // Extraer líneas de equilibrio exactas entre pares de polígonos adyacentes
    for (size_t i = 0; i < res.polygons.size(); ++i) {
        for (size_t j = i + 1; j < res.polygons.size(); ++j) {
            const auto& pa = res.polygons[i];
            const auto& pb = res.polygons[j];

            // Buscar vértices compartidos entre pa y pb
            std::vector<Vec2d> shared;
            for (const auto& va : pa.vertices) {
                for (const auto& vb : pb.vertices) {
                    if (std::hypot(va.first - vb.first, va.second - vb.second) < 1e-4) {
                        bool dup = false;
                        for (const auto& s : shared) {
                            if (std::hypot(s.x - va.first, s.y - va.second) < 1e-4)
                                dup = true;
                        }
                        if (!dup)
                            shared.push_back({va.first, va.second});
                    }
                }
            }

            if (shared.size() >= 2) {
                domain::BoundaryLineLpp bl;
                bl.phase_a = pa.formula;
                bl.phase_b = pb.formula;
                bl.x1 = shared[0].x;
                bl.y1 = shared[0].y;
                bl.x2 = shared[1].x;
                bl.y2 = shared[1].y;
                const double dx = bl.x2 - bl.x1;
                const double dy = bl.y2 - bl.y1;
                bl.slope_dy_dx = (std::abs(dx) > 1e-8) ? (dy / dx) : 1e9;

                // Encontrar planos correspondientes para log K de equilibrio
                for (const auto& pl_a : res.planes) {
                    if (pl_a.species_id == pa.species_id) {
                        for (const auto& pl_b : res.planes) {
                            if (pl_b.species_id == pb.species_id) {
                                bl.log_k = -(pl_b.Z - pl_a.Z) / RT_ln10;
                                break;
                            }
                        }
                        break;
                    }
                }
                bl.reaction_equation = pa.formula + " <=> " + pb.formula;
                res.boundaries.push_back(bl);
            }
        }
    }

    // Detectar puntos triples invariantes (coexistencia de 3 fases condensadas dentro del dominio)
    for (size_t i = 0; i < res.polygons.size(); ++i) {
        for (size_t j = i + 1; j < res.polygons.size(); ++j) {
            for (size_t k = j + 1; k < res.polygons.size(); ++k) {
                const auto& pa = res.polygons[i];
                const auto& pb = res.polygons[j];
                const auto& pc = res.polygons[k];
                for (const auto& va : pa.vertices) {
                    bool in_b = false, in_c = false;
                    for (const auto& vb : pb.vertices) {
                        if (std::hypot(va.first - vb.first, va.second - vb.second) < 1e-3)
                            in_b = true;
                    }
                    for (const auto& vc : pc.vertices) {
                        if (std::hypot(va.first - vc.first, va.second - vc.second) < 1e-3)
                            in_c = true;
                    }
                    if (in_b && in_c) {
                        res.triple_points.push_back(
                            {pa.formula, pb.formula, pc.formula, va.first, va.second});
                        break;
                    }
                }
            }
        }
    }

    // Evaluar punto de sonda (probe)
    double best_g = 1e30;
    for (const auto& pl : res.planes) {
        const double g_val = pl.Z + pl.A * req.probe_log_p1 + pl.B * req.probe_log_p2;
        if (g_val < best_g) {
            best_g = g_val;
            res.probe_predominant_phase = pl.formula;
            res.probe_min_potential_kj = g_val;
        }
    }

    std::ostringstream title;
    title << req.main_element << " - " << req.gas1_formula << " - " << req.gas2_formula << " System at "
          << std::fixed << std::setprecision(1) << (T - 273.15) << " °C (" << T << " K)";
    res.system_title = title.str();
    res.temperature_k = T;
    res.ok = !res.polygons.empty();
    res.message = "Diagrama Lpp calculado: " + std::to_string(res.polygons.size()) + " dominios de fase, " +
                  std::to_string(res.boundaries.size()) + " fronteras y " +
                  std::to_string(res.triple_points.size()) + " puntos triples.";
    return res;
}

// ═══════════════════════════════════════════════════════════════════════════
// Motor Tpp (T-p y p-p por Minimización de Gibbs en Malla + Contornos)
// ═══════════════════════════════════════════════════════════════════════════

domain::TppDiagramResult StabilityDiagrams::calcular_tpp(const domain::TppDiagramRequest& req) const {
    domain::TppDiagramResult res;
    if (req.x_var == req.y_var) {
        res.message = "Los ejes X e Y deben representar variables termodinámicas distintas.";
        return res;
    }
    if (req.x_min >= req.x_max || req.y_min >= req.y_max || req.rows < 4 || req.cols < 4) {
        res.message = "Rango de ejes o resolución de malla inválidos.";
        return res;
    }

    const auto basis = build_gas_basis(req.main_element, req.gas1_formula, req.gas2_formula);
    if (!basis.valid) {
        res.message = "Los dos gases seleccionados no forman una base estequiométrica independiente.";
        return res;
    }

    auto candidates = req.candidates;
    if (candidates.empty()) {
        candidates = filtrar_especies_sistema(req.main_element, req.gas1_formula, req.gas2_formula, true);
    }

    std::vector<domain::StabilitySpeciesSelection> active;
    for (auto c : candidates) {
        if (!c.selected || c.n_main <= 0.0)
            continue;
        if (std::abs(c.nu_gas1) < 1e-12 && std::abs(c.nu_gas2) < 1e-12 &&
            (c.n_elem1 > 0.0 || c.n_elem2 > 0.0)) {
            basis.solve_gas_stoich(c.n_elem1, c.n_elem2, c.nu_gas1, c.nu_gas2);
        }
        active.push_back(c);
    }

    if (active.empty()) {
        res.message = "No hay especies activas seleccionadas para el diagrama Tpp.";
        return res;
    }

    const auto* esp_g1 = find_gas_species(repo_, req.gas1_formula);
    const auto* esp_g2 = find_gas_species(repo_, req.gas2_formula);
    const auto ref_fases = fases_para_metal(basis.main_elem);

    const cma::nucleo::Especie* esp_metal_ref = nullptr;
    if (repo_) {
        for (const auto* e : repo_->todas_especies()) {
            if (e && e->formula == basis.main_elem && e->fase_std != cma::nucleo::Fase::Gas) {
                esp_metal_ref = e;
                break;
            }
        }
    }

    auto eval_coords = [&](double x, double y, double& T_k, double& lp1, double& lp2) {
        T_k = 973.15;
        lp1 = -10.0;
        lp2 = -2.0;
        if (req.x_var != domain::StabilityAxisVar::TemperatureK &&
            req.y_var != domain::StabilityAxisVar::TemperatureK) {
            T_k = req.fixed_value;
        } else if (req.x_var != domain::StabilityAxisVar::LogPressureGas1 &&
                   req.y_var != domain::StabilityAxisVar::LogPressureGas1) {
            lp1 = req.fixed_value;
        } else {
            lp2 = req.fixed_value;
        }

        if (req.x_var == domain::StabilityAxisVar::TemperatureK)
            T_k = x;
        else if (req.x_var == domain::StabilityAxisVar::LogPressureGas1)
            lp1 = x;
        else
            lp2 = x;

        if (req.y_var == domain::StabilityAxisVar::TemperatureK)
            T_k = y;
        else if (req.y_var == domain::StabilityAxisVar::LogPressureGas1)
            lp1 = y;
        else
            lp2 = y;

        T_k = std::clamp(T_k, 200.0, 3500.0);
    };

    auto eval_best_species = [&](double T_k, double lp1, double lp2, double& out_min_g) -> int {
        const double mu0_g1 = esp_g1 ? esp_g1->energia_gibbs(T_k) : 0.0;
        const double mu0_g2 = esp_g2 ? esp_g2->energia_gibbs(T_k) : 0.0;
        const double mu0_m = esp_metal_ref ? esp_metal_ref->energia_gibbs(T_k) : 0.0;
        const double rt = kR_kJ * T_k;
        const double mu_g1 = mu0_g1 + rt * kLn10 * lp1;
        const double mu_g2 = mu0_g2 + rt * kLn10 * lp2;

        out_min_g = 1e30;
        int best_k = 0;

        for (int k = 0; k < static_cast<int>(active.size()); ++k) {
            const auto& sp = active[k];
            double g_sp = 0.0;
            bool ok_sp = false;
            if (repo_ && sp.id >= 0) {
                if (const auto* e = repo_->especie_por_id(sp.id)) {
                    g_sp = e->energia_gibbs(T_k);
                    ok_sp = true;
                }
            }
            double nu1 = sp.nu_gas1;
            double nu2 = sp.nu_gas2;
            if (!ok_sp) {
                for (const auto& rf : ref_fases) {
                    if (rf.nombre == sp.formula) {
                        g_sp = rf.delta_G_rxn(T_k) + rf.nu_metal * mu0_m + rf.nu_O2 * mu0_g1 +
                               rf.nu_SO2 * mu0_g2;
                        nu1 = rf.nu_O2;
                        nu2 = rf.nu_SO2;
                        ok_sp = true;
                        break;
                    }
                }
            }
            if (!ok_sp)
                continue;

            const double act = std::clamp(sp.activity, 1e-12, 10.0);
            const double g_corr = g_sp + rt * std::log(act);
            const double g_norm = (g_corr - nu1 * mu_g1 - nu2 * mu_g2) / sp.n_main;
            if (g_norm < out_min_g) {
                out_min_g = g_norm;
                best_k = k;
            }
        }
        return best_k;
    };

    res.rows = req.rows;
    res.cols = req.cols;
    std::vector<int> raw_grid(res.rows * res.cols, 0);
    std::map<int, int> counts_per_active;

    const double dx = (req.x_max - req.x_min) / std::max(1, res.cols - 1);
    const double dy = (req.y_max - req.y_min) / std::max(1, res.rows - 1);

    for (int r = 0; r < res.rows; ++r) {
        // Fila 0 en la parte superior para ImPlot::PlotHeatmap
        const double y = req.y_max - r * dy;
        for (int c = 0; c < res.cols; ++c) {
            const double x = req.x_min + c * dx;
            double T_k = 0.0, lp1 = 0.0, lp2 = 0.0, min_g = 0.0;
            eval_coords(x, y, T_k, lp1, lp2);
            const int best_k = eval_best_species(T_k, lp1, lp2, min_g);
            raw_grid[r * res.cols + c] = best_k;
            counts_per_active[best_k]++;
        }
    }

    // Construir leyenda compacta con las fases que realmente aparecen en el diagrama
    std::map<int, int> active_to_legend;
    const double total_cells = static_cast<double>(res.rows * res.cols);
    int legend_idx = 0;
    for (const auto& [act_k, cnt] : counts_per_active) {
        domain::TppLegendEntry entry;
        entry.species_id = active[act_k].id;
        entry.color_index = legend_idx;
        entry.formula = active[act_k].formula;
        entry.area_fraction_pct = (100.0 * cnt) / total_cells;
        res.legend.push_back(entry);
        active_to_legend[act_k] = legend_idx++;
    }

    res.phase_index_grid.resize(raw_grid.size(), 0);
    for (size_t i = 0; i < raw_grid.size(); ++i) {
        res.phase_index_grid[i] = active_to_legend[raw_grid[i]];
    }

    // Extraer segmentos de contorno entre celdas vecinas de distinta fase predominante
    for (int r = 0; r + 1 < res.rows; ++r) {
        const double y_top = req.y_max - r * dy;
        const double y_bot = req.y_max - (r + 1) * dy;
        const double y_mid = 0.5 * (y_top + y_bot);
        for (int c = 0; c + 1 < res.cols; ++c) {
            const double x_left = req.x_min + c * dx;
            const double x_right = req.x_min + (c + 1) * dx;
            const double x_mid = 0.5 * (x_left + x_right);

            const int p00 = res.phase_index_grid[r * res.cols + c];
            const int p01 = res.phase_index_grid[r * res.cols + (c + 1)];
            const int p10 = res.phase_index_grid[(r + 1) * res.cols + c];

            if (p00 != p01) {
                res.contours.push_back(
                    {x_mid, y_top, x_mid, y_bot, res.legend[p00].formula, res.legend[p01].formula});
            }
            if (p00 != p10) {
                res.contours.push_back(
                    {x_left, y_mid, x_right, y_mid, res.legend[p00].formula, res.legend[p10].formula});
            }
        }
    }

    // Evaluar sonda (probe)
    {
        double T_k = 0.0, lp1 = 0.0, lp2 = 0.0, min_g = 0.0;
        eval_coords(req.probe_x, req.probe_y, T_k, lp1, lp2);
        const int best_k = eval_best_species(T_k, lp1, lp2, min_g);
        res.probe_predominant_phase = active[best_k].formula;
        res.probe_min_potential_kj = min_g;
    }

    res.system_title =
        req.main_element + " - " + req.gas1_formula + " - " + req.gas2_formula + " (Tpp Diagram)";
    res.ok = true;
    res.message = "Diagrama Tpp calculado: " + std::to_string(res.legend.size()) + " fases estables.";
    return res;
}

// ═══════════════════════════════════════════════════════════════════════════
// Exportación CSV
// ═══════════════════════════════════════════════════════════════════════════

bool StabilityDiagrams::exportar_lpp_csv(const std::string& ruta, const domain::LppDiagramResult& res) const {
    std::ofstream out(ruta);
    if (!out.is_open())
        return false;
    out << "# HSC Lpp Phase Stability Diagram Export - CMA Chemistry\n";
    out << "# System," << res.system_title << ",Temperature_K," << res.temperature_k << "\n";
    out << "Section,PhaseA,PhaseB_or_Reaction,X1_or_CentroidX,Y1_or_CentroidY,X2_or_Area,Y2_or_Slope,LogK\n";
    out << std::fixed << std::setprecision(4);

    for (const auto& p : res.polygons) {
        out << "POLYGON,\"" << p.formula << "\",\"" << p.formation_reaction << "\"," << p.centroid_x << ","
            << p.centroid_y << "," << p.area_log2 << ",0,0\n";
    }
    for (const auto& b : res.boundaries) {
        out << "BOUNDARY,\"" << b.phase_a << "\",\"" << b.phase_b << "\"," << b.x1 << "," << b.y1 << ","
            << b.x2 << "," << b.y2 << "," << b.log_k << "\n";
    }
    for (const auto& tp : res.triple_points) {
        out << "TRIPLE_POINT,\"" << tp.phase_a << " + " << tp.phase_b << "\",\"" << tp.phase_c << "\","
            << tp.log_p_gas1 << "," << tp.log_p_gas2 << ",0,0,0\n";
    }
    return true;
}

bool StabilityDiagrams::exportar_tpp_csv(const std::string& ruta, const domain::TppDiagramRequest& req,
                                         const domain::TppDiagramResult& res) const {
    std::ofstream out(ruta);
    if (!out.is_open())
        return false;
    out << "# HSC Tpp Phase Stability Diagram Export - CMA Chemistry\n";
    out << "# System," << res.system_title << "\n";
    out << "Phase,AreaFraction_pct\n";
    out << std::fixed << std::setprecision(3);
    for (const auto& l : res.legend) {
        out << "\"" << l.formula << "\"," << l.area_fraction_pct << "\n";
    }
    out << "\n# Sampled Grid (X, Y, PredominantPhase)\n";
    out << "X,Y,Phase\n";
    const double dx = (req.x_max - req.x_min) / std::max(1, res.cols - 1);
    const double dy = (req.y_max - req.y_min) / std::max(1, res.rows - 1);
    const int step_r = std::max(1, res.rows / 25);
    const int step_c = std::max(1, res.cols / 25);
    for (int r = 0; r < res.rows; r += step_r) {
        for (int c = 0; c < res.cols; c += step_c) {
            const int idx = res.phase_index_grid[r * res.cols + c];
            out << (req.x_min + c * dx) << "," << (req.y_max - r * dy) << ",\"" << res.legend[idx].formula
                << "\"\n";
        }
    }
    return true;
}

} // namespace cma::application::modules

#pragma GCC diagnostic pop
