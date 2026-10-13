#include "motores/minimizador_gibbs.hpp"
#include "nucleo/tabla_periodica.hpp"
#include <cmath>
#include <algorithm>
#include <set>

namespace cma { namespace motores {

// Carga formal a partir del tag de la fórmula: "(+3a)"→+3, "(-a)"→-1, "e-"→-1
static double carga_especie(const nucleo::Especie& sp) {
    if (sp.formula == "e-" || sp.formula == "e") return -1.0;
    size_t open = sp.formula.rfind('(');
    if (open == std::string::npos) return 0.0;
    size_t close = sp.formula.find(')', open);
    if (close == std::string::npos) return 0.0;
    std::string tag = sp.formula.substr(open + 1, close - open - 1);
    if (tag.empty() || (tag[0] != '+' && tag[0] != '-')) return 0.0;
    int sign = (tag[0] == '+') ? 1 : -1;
    double val = 0.0;
    for (size_t i = 1; i < tag.size() && std::isdigit((unsigned char)tag[i]); ++i)
        val = val * 10.0 + (tag[i] - '0');
    if (val == 0.0) val = 1.0;
    return sign * val;
}

ResultadoGibbs MinimizadorGibbs::minimizar(
    const std::map<int64_t, double>& alimentacion,
    double T_K,
    double P_atm,
    const std::map<int64_t, nucleo::Especie>& db,
    const std::vector<int64_t>& especies_posibles,
    std::map<int64_t, double>* n_inout,
    bool modo_celda)
{
    ResultadoGibbs res;
    if (!std::isfinite(T_K) || !std::isfinite(P_atm) || T_K <= 0 || P_atm <= 0) {
        res.mensaje = "Temperatura y presión deben ser finitas y positivas"; return res;
    }
    for (const auto& [sid, cantidad] : alimentacion) {
        if (!db.count(sid) || !std::isfinite(cantidad) || cantidad < 0) {
            res.mensaje = "Alimentación inválida o especie desconocida"; return res;
        }
    }
    for (auto sid : especies_posibles) {
        if (!db.count(sid)) { res.mensaje = "Especie candidata desconocida"; return res; }
    }

    // Determinar especies activas
    std::vector<int64_t> activas;
    std::set<int64_t> vistas;
    for (auto& [sid, _] : alimentacion) {
        if (vistas.insert(sid).second) activas.push_back(sid);
    }
    for (int64_t sid : especies_posibles) {
        if (vistas.insert(sid).second) activas.push_back(sid);
    }
    if (especies_posibles.empty()) {
        activas.clear();
        for (auto& [sid, _] : alimentacion) activas.push_back(sid);
    }

    // Modo celda (HSC): añadir el electrón como especie para equilibrios redox
    if (modo_celda) {
        for (auto& [sid, sp] : db) {
            if (sp.formula == "e-" && vistas.insert(sid).second) {
                activas.push_back(sid);
                break;
            }
        }
    }

    // Cargas por especie; decidir si hace falta la restricción de electroneutralidad
    std::map<int64_t, double> carga_sp;
    bool necesita_carga = false;
    for (int64_t sid : activas) {
        auto it = db.find(sid);
        if (it == db.end()) continue;
        double q = carga_especie(it->second);
        carga_sp[sid] = q;
        if (std::abs(q) > 1e-12) necesita_carga = true;
    }

    if (activas.size() > 250) { res.mensaje = "Demasiadas especies (máx. 250)"; return res; }
    // Species containing an element absent from the feed must have exactly zero
    // amount. Keeping them inside a strictly positive barrier makes it infeasible.
    const auto& periodic = nucleo::TablaPeriodica::instancia();
    std::set<std::string> feed_elements;
    for (const auto& [sid, amount] : alimentacion) if (amount > 0)
        for (const auto& atom : periodic.analizar_formula(db.at(sid).formula)) feed_elements.insert(atom.elem);
    activas.erase(std::remove_if(activas.begin(), activas.end(), [&](int64_t sid) {
        const auto atoms = periodic.analizar_formula(db.at(sid).formula);
        return std::any_of(atoms.begin(), atoms.end(), [&](const auto& atom) { return !feed_elements.count(atom.elem); });
    }), activas.end());
    double feed_charge = 0;
    for (const auto& [sid, amount] : alimentacion) feed_charge += amount * carga_sp[sid];
    bool positive_charge = false, negative_charge = false;
    for (auto sid : activas) {
        positive_charge |= carga_sp[sid] > 0;
        negative_charge |= carga_sp[sid] < 0;
    }
    if (std::abs(feed_charge) < 1e-14 && (!positive_charge || !negative_charge)) {
        // Neutral feed plus ions of only one sign forces every ion to zero.
        activas.erase(std::remove_if(activas.begin(), activas.end(), [&](int64_t sid) {
            return std::abs(carga_sp[sid]) > 1e-14;
        }), activas.end());
        necesita_carga = false;
    }
    size_t NS = activas.size();
    if (NS == 0) { res.mensaje = "No hay especies."; return res; }
    if (NS > 250) {
        res.mensaje = "Demasiadas especies (" + std::to_string(NS) +
                      "). Limite la selección de elementos (máx. 250).";
        return res;
    }

    // Construir matriz de elementos
    auto& tp = nucleo::TablaPeriodica::instancia();
    std::map<std::string, int> indice_elemento;
    std::vector<std::string> elementos;

    for (int64_t sid : activas) {
        auto it = db.find(sid);
        if (it == db.end()) continue;
        auto atomos = tp.analizar_formula(it->second.formula);
        for (auto& ca : atomos) {
            if (indice_elemento.find(ca.elem) == indice_elemento.end()) {
                indice_elemento[ca.elem] = (int)elementos.size();
                elementos.push_back(ca.elem);
            }
        }
    }

    size_t NE = elementos.size();
    if (NE == 0) { res.mensaje = "No se encontraron elementos."; return res; }

    // Fila extra: conservación de carga cuando hay especies cargadas
    size_t NE_t = NE + (necesita_carga ? 1 : 0);

    // Matriz A (NE_t x NS): átomos del elemento i en especie j (+ fila de carga)
    Eigen::MatrixXd A = Eigen::MatrixXd::Zero((int)NE_t, (int)NS);
    std::map<int64_t, int> columna_especie;
    for (size_t j = 0; j < NS; ++j) columna_especie[activas[j]] = (int)j;

    for (size_t i = 0; i < NE; ++i) {
        for (int64_t sid : activas) {
            auto it = db.find(sid);
            if (it == db.end()) continue;
            auto atomos = tp.analizar_formula(it->second.formula);
            for (auto& ca : atomos) {
                if (ca.elem == elementos[i]) {
                    A((int)i, columna_especie[sid]) = ca.cuenta;
                }
            }
        }
    }
    if (necesita_carga) {
        for (size_t j = 0; j < NS; ++j) A((int)NE, (int)j) = carga_sp[activas[j]];
    }

    // Vector b: abundancia de elementos desde alimentación (+ carga total)
    Eigen::VectorXd b = Eigen::VectorXd::Zero((int)NE_t);
    for (auto& [sid, n] : alimentacion) {
        auto it = db.find(sid);
        if (it == db.end()) continue;
        auto atomos = tp.analizar_formula(it->second.formula);
        for (auto& ca : atomos) {
            auto idx = indice_elemento.find(ca.elem);
            if (idx != indice_elemento.end())
                b(idx->second) += n * ca.cuenta;
        }
    }
    if (necesita_carga) {
        double qtot = 0.0;
        for (auto& [sid, m] : alimentacion) qtot += m * carga_sp[sid];
        b((int)NE) = qtot;
    }

    if (b.norm() < 1e-30) { res.mensaje = "Alimentación vacía."; return res; }

    // Remove linearly dependent conservation rows (e.g. charge rows proportional
    // to an elemental balance). KKT systems must use independent constraints.
    Eigen::ColPivHouseholderQR<Eigen::MatrixXd> row_qr(A.transpose());
    const int rank = static_cast<int>(row_qr.rank());
    if (rank == 0) { res.mensaje = "Sin restricciones independientes"; return res; }
    Eigen::MatrixXd Ar(rank, static_cast<int>(NS));
    Eigen::VectorXd br(rank);
    for (int i = 0; i < rank; ++i) {
        const int row = row_qr.colsPermutation().indices()(i);
        Ar.row(i) = A.row(row); br(i) = b(row);
    }
    const double RT = nucleo::R_GAS * T_K / 1000;
    const double b_scale = std::max(1.0, b.cwiseAbs().maxCoeff());
    const double amount_scale = std::max(1.0, b.head(static_cast<int>(NE)).sum());
    const double N_FLOOR = 1e-30;
    std::vector<bool> es_gas(NS);
    Eigen::VectorXd cvec(static_cast<int>(NS));
    for (size_t j = 0; j < NS; ++j) {
        const auto& species = db.at(activas[j]);
        es_gas[j] = species.fase_std == nucleo::Fase::Gas || species.fase_std == nucleo::Fase::Mixto;
        cvec(static_cast<int>(j)) = species.energia_gibbs(T_K) / RT;
        if (es_gas[j]) cvec(static_cast<int>(j)) += std::log(P_atm);
    }
    if (!cvec.allFinite()) { res.mensaje = "Datos termodinámicos no finitos"; return res; }
    // Positive, infeasible-start primal-dual barrier method for the convex
    // ideal-mixture Gibbs objective. Condensed species have unit activity.
    Eigen::VectorXd n = Eigen::VectorXd::Constant(static_cast<int>(NS), amount_scale / static_cast<double>(NS));
    if (n_inout) for (size_t j = 0; j < NS; ++j) {
        const auto it = n_inout->find(activas[j]);
        if (it != n_inout->end() && std::isfinite(it->second) && it->second > 0)
            n(static_cast<int>(j)) = 0.9 * it->second + 0.1 * amount_scale / static_cast<double>(NS);
    }
    const double cost_scale = std::max(1.0, cvec.cwiseAbs().maxCoeff());
    Eigen::VectorXd pi = Eigen::VectorXd::Zero(rank);
    auto gradient = [&](const Eigen::VectorXd& amounts) {
        Eigen::VectorXd grad = cvec;
        double gas_total = 0;
        for (size_t j = 0; j < NS; ++j) if (es_gas[j]) gas_total += amounts(static_cast<int>(j));
        for (size_t j = 0; j < NS; ++j) if (es_gas[j])
            grad(static_cast<int>(j)) += std::log(std::max(amounts(static_cast<int>(j)), N_FLOOR) / std::max(gas_total, N_FLOOR));
        return Eigen::VectorXd(grad / cost_scale);
    };
    double mu = 0.1;
    bool solved = false;
    for (int stage = 0; stage < 15; ++stage) {
        for (int iter = 0; iter < 300; ++iter) {
            ++res.iteraciones;
            const Eigen::VectorXd grad = gradient(n);
            const Eigen::VectorXd dual = grad - Ar.transpose() * pi - (mu * n.cwiseInverse()).eval();
            const Eigen::VectorXd primal = br - Ar * n;
            const double dual_norm = dual.cwiseProduct(n).cwiseAbs().maxCoeff() / amount_scale;
            const double primal_norm = primal.cwiseAbs().maxCoeff() / b_scale;
            if (dual_norm < 1e-8 && primal_norm < 1e-10) { solved = true; break; }
            solved = false;
            Eigen::MatrixXd H = Eigen::MatrixXd::Zero(static_cast<int>(NS), static_cast<int>(NS));
            double gas_total = 0;
            for (size_t j = 0; j < NS; ++j) if (es_gas[j]) gas_total += n(static_cast<int>(j));
            for (size_t j = 0; j < NS; ++j) {
                const int jj = static_cast<int>(j);
                H(jj,jj) = mu / (n(jj) * n(jj));
                if (es_gas[j]) {
                    H(jj,jj) += 1 / (cost_scale * n(jj));
                    for (size_t k = 0; k < NS; ++k) if (es_gas[k])
                        H(jj,static_cast<int>(k)) -= 1 / (cost_scale * gas_total);
                }
            }
            // Eliminate dn using a positive definite Hessian. Row equilibration
            // avoids losing conservation constraints for trace components.
            // Solve in relative amounts (dn = diag(n) ds) so trace species
            // do not make the Hessian ill-conditioned by factors of 1/n².
            const Eigen::MatrixXd scaled_h = n.asDiagonal() * H * n.asDiagonal();
            const Eigen::MatrixXd scaled_a = Ar * n.asDiagonal();
            Eigen::LDLT<Eigen::MatrixXd> hsolve(scaled_h);
            if (hsolve.info() != Eigen::Success) break;
            const Eigen::MatrixXd hat = hsolve.solve(scaled_a.transpose());
            const Eigen::VectorXd hd = hsolve.solve(n.cwiseProduct(dual));
            const Eigen::MatrixXd schur = scaled_a * hat;
            Eigen::VectorXd scaling(rank);
            for (int row = 0; row < rank; ++row) scaling(row) = 1 / std::sqrt(std::max(schur(row,row),1e-300));
            const Eigen::MatrixXd normalized = scaling.asDiagonal() * schur * scaling.asDiagonal();
            const Eigen::VectorXd rhs = scaling.cwiseProduct(primal + scaled_a * hd);
            const Eigen::VectorXd dpi = scaling.cwiseProduct(normalized.completeOrthogonalDecomposition().solve(rhs));
            const Eigen::VectorXd dn = n.cwiseProduct(hat * dpi - hd);
            if (!dn.allFinite() || !dpi.allFinite()) break;
            double step = 1;
            for (int j = 0; j < n.size(); ++j) if (dn(j) < 0)
                step = std::min(step, -0.99 * n(j) / dn(j));
            const double merit = dual.cwiseProduct(n).norm() / amount_scale + primal.norm() / b_scale;
            bool accepted = false;
            for (int search = 0; search < 60; ++search) {
                const Eigen::VectorXd next = n + step * dn;
                const Eigen::VectorXd next_pi = pi + step * dpi;
                const Eigen::VectorXd next_dual = gradient(next) - Ar.transpose() * next_pi - (mu * next.cwiseInverse()).eval();
                const double next_merit = next_dual.cwiseProduct(next).norm() / amount_scale + (br - Ar * next).norm() / b_scale;
                if (next.allFinite() && next.minCoeff() > 0 && std::isfinite(next_merit) &&
                    next_merit <= (1 - 1e-4 * step) * merit) {
                    n = next; pi = next_pi; accepted = true; break;
                }
                step *= 0.5;
            }
            if (!accepted) break;
        }
        if (!solved) {
            res.mensaje = "Gibbs: residual primal=" + std::to_string((Ar * n - br).norm()) +
                " dual=" + std::to_string((gradient(n) - Ar.transpose() * pi - (mu * n.cwiseInverse()).eval()).norm()) +
                " barrera=" + std::to_string(mu);
            break;
        }
        mu *= 0.1;
    }
    // Return success only with actual conservation and finite objective/states.
    const double mass_error = (A * n - b).cwiseAbs().maxCoeff() / b_scale;
    res.convergio = solved && mass_error < 1e-8 && n.allFinite() && mu < 1e-12;
    if (!res.convergio && n_inout && !n_inout->empty()) {
        // A previous optimum can be too close to a face of the feasible region
        // for the initial barrier. Restart independently, retaining the same
        // thermodynamic model and convergence criteria.
        auto restarted = minimizar(alimentacion, T_K, P_atm, db, especies_posibles, nullptr, modo_celda);
        restarted.iteraciones += res.iteraciones;
        if (restarted.convergio) {
            *n_inout = restarted.flujo_molar;
            restarted.mensaje += " Reinicio desde interior (warm start inestable).";
        }
        return restarted;
    }

    double total_G = 0;
    double total_gas = 0.0;
    for (size_t j = 0; j < NS; ++j) {
        if (es_gas[j] && n((int)j) > 1e-15) total_gas += n((int)j);
    }
    total_gas = std::max(total_gas, (double)NS * N_FLOOR + 1e-30);
    for (size_t j = 0; j < NS; ++j) {
        if (n((int)j) > 1e-12) {
            int64_t sid = activas[j];
            res.flujo_molar[sid] = n((int)j);
            auto it = db.find(sid);
            if (it != db.end()) {
                double G0 = it->second.energia_gibbs(T_K);
                if (es_gas[j]) {
                    double yjP = std::max(n((int)j) / total_gas * P_atm, 1e-30);
                    total_G += n((int)j) * (G0 + RT * std::log(yjP));
                } else {
                    total_G += n((int)j) * G0;
                }
            }
        }
    }

    res.energia_gibbs_total = total_G;
    if (res.mensaje.empty()) res.mensaje = res.convergio
        ? "Minimización Gibbs convergió en " + std::to_string(res.iteraciones) + " iteraciones."
        : "No convergió completamente tras " + std::to_string(res.iteraciones) + " iteraciones.";

    // Exportar la solución como semilla para warm starts
    if (n_inout && res.convergio) {
        n_inout->clear();
        for (size_t j = 0; j < NS; ++j)
            if (n((int)j) > 1e-15) (*n_inout)[activas[j]] = n((int)j);
    }

    return res;
}

}} // namespace
