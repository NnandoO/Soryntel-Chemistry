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

    // Valores iniciales: warm start desde el punto anterior si está disponible
    double total_b = b.sum();
    Eigen::VectorXd n = Eigen::VectorXd::Ones((int)NS) * (total_b / (double)NS);
    if (n_inout && !n_inout->empty()) {
        double floor = std::max(1e-9 * total_b, 1e-12);
        for (size_t j = 0; j < NS; ++j) {
            auto it = n_inout->find(activas[j]);
            if (it != n_inout->end() && it->second > 0.0)
                n((int)j) = std::max(it->second, floor);
        }
        // Evitar warm starts con norma excesivamente distinta
        double escala = (n.sum() > 1e-30) ? total_b / n.sum() : 1.0;
        n *= escala;
        for (size_t j = 0; j < NS; ++j) n((int)j) = std::max(n((int)j), 1e-12);
    }
    if (T_K <= 0.0) { res.mensaje = "Temperatura inválida."; return res; }
    double RT = nucleo::R_GAS * T_K / 1000.0; // kJ/mol

    const double N_FLOOR = 1e-13;                       // piso de composición
    const double b_scale = std::max(1.0, b.cwiseAbs().maxCoeff());

    // Reparar factibilidad con conjunto activo: la corrección de mínima norma
    // solo se reparte sobre especies "libres" (las que tocan el piso se
    // congelan). Así se conserva la masa incluso con especies traza.
    auto reparar_factibilidad = [&](Eigen::VectorXd& nv) {
        for (size_t j = 0; j < NS; ++j) if (nv((int)j) < N_FLOOR) nv((int)j) = N_FLOOR;
        std::vector<bool> libre(NS, true);
        for (int rep = 0; rep < 25; ++rep) {
            Eigen::VectorXd r = b - A * nv;
            if (r.cwiseAbs().maxCoeff() < 1e-10 * b_scale) return;
            // Construir matriz solo con columnas libres
            std::vector<int> cols;
            for (size_t j = 0; j < NS; ++j) if (libre[j]) cols.push_back((int)j);
            if (cols.empty()) return;
            Eigen::MatrixXd Af((int)NE_t, (int)cols.size());
            for (size_t k = 0; k < cols.size(); ++k) Af.col((int)k) = A.col(cols[k]);
            Eigen::BDCSVD<Eigen::MatrixXd> svd(Af, Eigen::ComputeThinU | Eigen::ComputeThinV);
            Eigen::VectorXd corr = svd.solve(r);
            bool recorto = false;
            for (size_t k = 0; k < cols.size(); ++k) {
                int j = cols[k];
                double nvj = nv(j) + corr((int)k);
                if (nvj < N_FLOOR) { nv(j) = N_FLOOR; libre[j] = false; recorto = true; }
                else nv(j) = nvj;
            }
            (void)recorto;
        }
    };
    reparar_factibilidad(n);

    // ════════════════════════════════════════════════════════════════════════
    // Iteración de Gibbs (RAND, NASA RP-1311):
    //   desconocidas: Δπ (potenciales elementales), Δn de condensadas activas
    //   y Δln N (total de moles gas). Los gases se actualizan en forma
    //   exponencial (positividad garantizada).
    // ════════════════════════════════════════════════════════════════════════

    // Clasificación de fases y potencial estándar adimensional c_j
    std::vector<bool> es_gas(NS);
    Eigen::VectorXd cvec = Eigen::VectorXd::Zero((int)NS);
    bool hay_gas = false;
    for (size_t j = 0; j < NS; ++j) {
        auto it = db.find(activas[j]);
        if (it == db.end()) { es_gas[j] = true; hay_gas = true; continue; }
        bool condensado = (it->second.fase_std == nucleo::Fase::Solido ||
                           it->second.fase_std == nucleo::Fase::Liquido);
        es_gas[j] = !condensado;
        if (es_gas[j]) hay_gas = true;
        cvec((int)j) = it->second.energia_gibbs(T_K) / RT;
        if (es_gas[j]) cvec((int)j) += std::log(std::max(P_atm, 1e-30));
    }

    // Fases condensadas activas inicialmente: las del feed con cantidad > 0
    std::vector<bool> cond_activa(NS, false);
    for (size_t j = 0; j < NS; ++j) {
        if (es_gas[j]) continue;
        auto itf = alimentacion.find(activas[j]);
        if (itf != alimentacion.end() && itf->second > 0.0) {
            cond_activa[j] = true;
            n((int)j) = std::max(itf->second, N_FLOOR);
        }
    }

    Eigen::VectorXd pi = Eigen::VectorXd::Zero((int)NE_t);
    reparar_factibilidad(n);

    bool cerca_convergencia = false;

    for (int iter = 0; iter < MAX_ITER; ++iter) {
        double N = 0.0;
        for (size_t j = 0; j < NS; ++j) if (es_gas[j]) N += n((int)j);
        N = std::max(N, (double)NS * N_FLOOR + 1e-30);

        // residuales de equilibrio: r = c - Aᵀπ  (+ ln(n/N) para gases)
        Eigen::VectorXd r_g = Eigen::VectorXd::Zero((int)NS);
        Eigen::VectorXd r_c = Eigen::VectorXd::Zero((int)NS);
        for (size_t j = 0; j < NS; ++j) {
            double atp = 0.0;
            for (size_t i = 0; i < NE_t; ++i) atp += A((int)i, (int)j) * pi((int)i);
            if (es_gas[j])
                r_g((int)j) = cvec((int)j) + std::log(std::max(n((int)j), 1e-300) / N) - atp;
            else
                r_c((int)j) = cvec((int)j) - atp;
        }

        // ── Test de fases condensadas: solo cuando casi convergido ──
        bool cambio_fase = false;
        if (cerca_convergencia) {
            for (size_t j = 0; j < NS; ++j) {
                if (es_gas[j]) continue;
                if (!cond_activa[j] && r_c((int)j) < -1e-7) {
                    cond_activa[j] = true;
                    n((int)j) = std::max(n((int)j), 1e-8 * b_scale);
                    cambio_fase = true;
                } else if (cond_activa[j] && n((int)j) <= N_FLOOR * 10.0 && r_c((int)j) > 1e-7) {
                    cond_activa[j] = false;
                    n((int)j) = N_FLOOR;
                    cambio_fase = true;
                }
            }
            if (cambio_fase) cerca_convergencia = false;
        }

        // ── Sistema reducido ──
        std::vector<int> cond_cols;
        for (size_t j = 0; j < NS; ++j) if (!es_gas[j] && cond_activa[j]) cond_cols.push_back((int)j);
        int NCa = (int)cond_cols.size();
        int idxLN = (int)NE_t + NCa;              // columna de Δln N si hay gas
        int dim = (int)NE_t + NCa + (hay_gas ? 1 : 0);
        Eigen::MatrixXd M = Eigen::MatrixXd::Zero(dim, dim);
        Eigen::VectorXd y = Eigen::VectorXd::Zero(dim);

        Eigen::VectorXd resid = b - A * n;
        for (int i = 0; i < (int)NE_t; ++i) {
            double rhs_i = resid(i);
            double rowlnN = 0.0;
            for (size_t j = 0; j < NS; ++j) {
                if (!es_gas[j]) continue;
                double nj = n((int)j), aij = A(i, (int)j);
                for (int k = 0; k < (int)NE_t; ++k)
                    M(i, k) += aij * nj * A(k, (int)j);
                rhs_i += aij * nj * r_g((int)j);
                rowlnN += aij * nj;
            }
            if (hay_gas) M(i, idxLN) = rowlnN;
            for (int cix = 0; cix < NCa; ++cix)
                M(i, (int)NE_t + cix) = A(i, cond_cols[cix]);
            y(i) = rhs_i;
        }
        if (hay_gas) {
            // definición de Δln N a partir de los gases (N' = Σ n_j')
            for (int k = 0; k < (int)NE_t; ++k) {
                double acc = 0.0;
                for (size_t j = 0; j < NS; ++j)
                    if (es_gas[j]) acc += n((int)j) * A(k, (int)j);
                M(idxLN, k) = acc;
            }
            double rhs = 0.0;
            for (size_t j = 0; j < NS; ++j)
                if (es_gas[j]) rhs += n((int)j) * r_g((int)j);
            y(idxLN) = rhs;
        }
        for (int cix = 0; cix < NCa; ++cix) {
            int row = (int)NE_t + cix;
            for (int i = 0; i < (int)NE_t; ++i)
                M(row, i) = A(i, cond_cols[cix]);
            y(row) = r_c(cond_cols[cix]);
        }

        Eigen::FullPivLU<Eigen::MatrixXd> lu(M);
        if (!lu.isInvertible()) {
            if (NCa > 0) { // retirar la última condensada y reintentar
                int jc = cond_cols.back();
                cond_activa[jc] = false;
                n(jc) = N_FLOOR;
                continue;
            }
            res.mensaje = "Sistema singular en iteración " + std::to_string(iter);
            break;
        }
        Eigen::VectorXd xsol = lu.solve(y);
        if (!xsol.allFinite()) {
            res.mensaje = "Paso no finito en iteración " + std::to_string(iter);
            break;
        }

        Eigen::VectorXd dpi = xsol.head((int)NE_t);
        double dlnN = hay_gas ? xsol(idxLN) : 0.0;

        // sub-relajación: limitar Δln n a ±2 por iteración
        double alpha = 1.0;
        {
            double maxdln = 0.0;
            for (size_t j = 0; j < NS; ++j) {
                if (!es_gas[j]) continue;
                double d = dlnN - r_g((int)j);
                for (int i = 0; i < (int)NE_t; ++i) d += A((int)i, (int)j) * dpi(i);
                maxdln = std::max(maxdln, std::abs(d));
            }
            if (maxdln > 2.0) alpha = 2.0 / maxdln;
        }

        for (int i = 0; i < (int)NE_t; ++i) pi(i) += alpha * dpi(i);
        for (size_t j = 0; j < NS; ++j) {
            if (!es_gas[j]) continue;
            double dln = dlnN - r_g((int)j);
            for (int i = 0; i < (int)NE_t; ++i) dln += A((int)i, (int)j) * dpi(i);
            n((int)j) = std::max(n((int)j) * std::exp(alpha * dln), N_FLOOR);
        }
        for (int cix = 0; cix < NCa; ++cix) {
            int j = cond_cols[cix];
            n(j) = std::max(n(j) + alpha * xsol((int)NE_t + cix), N_FLOOR);
        }
        reparar_factibilidad(n);
        res.iteraciones = iter + 1;

        // ── Convergencia: masa + estacionariedad + fases estables ──
        double res_norm = (A * n - b).cwiseAbs().maxCoeff() / b_scale;
        double rg_max = 0.0;
        for (size_t j = 0; j < NS; ++j)
            if (es_gas[j] && n((int)j) > 1e-9 * N)
                rg_max = std::max(rg_max, std::abs(r_g((int)j)));
        cerca_convergencia = (res_norm < 1e-6 && rg_max < 1e-3);
        double rc_max = 0.0;
        for (size_t j = 0; j < NS; ++j)
            if (!es_gas[j] && cond_activa[j] && n((int)j) > N_FLOOR * 10)
                rc_max = std::max(rc_max, std::abs(r_c((int)j)));
        bool fases_ok = !cambio_fase;
        if (fases_ok) {
            for (size_t j = 0; j < NS; ++j)
                if (!es_gas[j] && !cond_activa[j] && r_c((int)j) < -1e-6) fases_ok = false;
        }
        if (res_norm < 1e-8 && rg_max < 1e-6 && rc_max < 1e-6 && fases_ok) {
            res.convergio = true;
            break;
        }
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
    res.mensaje = res.convergio
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
