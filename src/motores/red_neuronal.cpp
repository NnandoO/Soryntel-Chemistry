#include "motores/red_neuronal.hpp"
#include <cmath>
#include <random>
#include <algorithm>
#include <sstream>
#include <iomanip>

namespace cma { namespace motores {

double RedNeuronal::activar(double v, int tipo) const {
    switch (tipo) {
        case 0: return std::tanh(std::clamp(v, -20.0, 20.0));              // TanH
        case 1: return 1.0 / (1.0 + std::exp(-std::clamp(v, -40.0, 40.0))); // Sigmoid
        case 2: return v > 0.0 ? v : 0.0;                                  // ReLU
        default: return v;                                                 // Linear
    }
}

double RedNeuronal::activar_deriv(double v, double y, int tipo) const {
    (void)v;
    switch (tipo) {
        case 0: return 1.0 - y * y;                     // tanh'
        case 1: return y * (1.0 - y);                   // sigmoid'
        case 2: return y > 0.0 ? 1.0 : 0.0;             // relu' (y = salida relu)
        default: return 1.0;                            // linear
    }
}

void RedNeuronal::inicializar(int n_entrada, const ConfigANN& cfg, int n_salida) {
    cfg_ = cfg;
    n_ent_ = n_entrada;
    n_sal_ = n_salida;
    W_.clear();
    B_.clear();
    normalizado_ = false;
    x_min_.assign(n_ent_, -0.9);
    x_max_.assign(n_ent_, 0.9);
    y_min_.assign(n_sal_, -0.9);
    y_max_.assign(n_sal_, 0.9);

    std::mt19937 rng(static_cast<unsigned>(cfg.semilla));
    auto xavier = [&rng](int fin, int fout) {
        double r = std::sqrt(6.0 / (fin + fout));
        std::uniform_real_distribution<double> d(-r, r);
        return d(rng);
    };

    int prev = n_ent_;
    for (int h : cfg.ocultas) {
        if (h <= 0) continue;
        std::vector<std::vector<double>> w(h, std::vector<double>(prev));
        for (auto& fila : w) for (auto& v : fila) v = xavier(prev, h);
        W_.push_back(std::move(w));
        B_.push_back(std::vector<double>(h, 0.0));
        prev = h;
    }
    std::vector<std::vector<double>> w(n_sal_, std::vector<double>(prev));
    for (auto& fila : w) for (auto& v : fila) v = xavier(prev, n_sal_);
    W_.push_back(std::move(w));
    B_.push_back(std::vector<double>(n_sal_, 0.0));
}

std::vector<double> RedNeuronal::normalizar_x(const std::vector<double>& x_raw) const {
    if (!normalizado_ || x_min_.size() != x_raw.size()) return x_raw;
    std::vector<double> xn(x_raw.size());
    for (size_t i = 0; i < x_raw.size(); ++i) {
        double span = x_max_[i] - x_min_[i];
        xn[i] = (span > 1e-15) ? (-0.9 + 1.8 * (x_raw[i] - x_min_[i]) / span) : 0.0;
    }
    return xn;
}

std::vector<double> RedNeuronal::normalizar_y(const std::vector<double>& y_raw) const {
    if (!normalizado_ || y_min_.size() != y_raw.size()) return y_raw;
    std::vector<double> yn(y_raw.size());
    for (size_t i = 0; i < y_raw.size(); ++i) {
        double span = y_max_[i] - y_min_[i];
        yn[i] = (span > 1e-15) ? (-0.9 + 1.8 * (y_raw[i] - y_min_[i]) / span) : 0.0;
    }
    return yn;
}

std::vector<double> RedNeuronal::desnormalizar_y(const std::vector<double>& y_norm) const {
    if (!normalizado_ || y_min_.size() != y_norm.size()) return y_norm;
    std::vector<double> yr(y_norm.size());
    for (size_t i = 0; i < y_norm.size(); ++i) {
        double span = y_max_[i] - y_min_[i];
        yr[i] = (span > 1e-15) ? (y_min_[i] + (y_norm[i] + 0.9) * span / 1.8) : y_min_[i];
    }
    return yr;
}

void RedNeuronal::forward(const std::vector<double>& x,
                          std::vector<std::vector<double>>& a) const {
    a[0] = x;
    for (size_t l = 0; l < W_.size(); ++l) {
        int tipo = (l + 1 == W_.size()) ? cfg_.act_salida : cfg_.act_oculta;
        for (size_t j = 0; j < W_[l].size(); ++j) {
            double s = B_[l][j];
            for (size_t i = 0; i < W_[l][j].size(); ++i) s += W_[l][j][i] * a[l][i];
            a[l + 1][j] = activar(s, tipo);
        }
    }
}

std::vector<double> RedNeuronal::predecir_normalizado(const std::vector<double>& entrada) const {
    if (W_.empty()) return {};
    std::vector<std::vector<double>> a(W_.size() + 1);
    a[0].resize(n_ent_);
    for (size_t l = 0; l < W_.size(); ++l) a[l + 1].resize(W_[l].size());
    forward(normalizar_x(entrada), a);
    return a.back();
}

std::vector<double> RedNeuronal::predecir(const std::vector<double>& entrada) const {
    if (W_.empty()) return {};
    return desnormalizar_y(predecir_normalizado(entrada));
}

size_t RedNeuronal::total_params() const {
    size_t n = 0;
    for (auto& capa : W_) for (auto& fila : capa) n += fila.size();
    for (auto& b : B_) n += b.size();
    return n;
}

std::vector<double> RedNeuronal::obtener_params() const {
    std::vector<double> p;
    p.reserve(total_params());
    for (auto& capa : W_) for (auto& fila : capa) for (double v : fila) p.push_back(v);
    for (auto& b : B_) for (double v : b) p.push_back(v);
    return p;
}

void RedNeuronal::fijar_params(const std::vector<double>& p) {
    size_t k = 0;
    for (auto& capa : W_) for (auto& fila : capa) for (double& v : fila)
        v = (k < p.size()) ? p[k++] : 0.0;
    for (auto& b : B_) for (double& v : b) v = (k < p.size()) ? p[k++] : 0.0;
}

std::vector<int> RedNeuronal::topologia() const {
    std::vector<int> t;
    if (W_.empty()) return t;
    t.push_back(n_ent_);
    for (size_t l = 0; l + 1 < W_.size(); ++l) t.push_back((int)W_[l].size());
    t.push_back(n_sal_);
    return t;
}

ResultadoANN RedNeuronal::entrenar(const std::vector<std::vector<double>>& X_raw,
                                   const std::vector<std::vector<double>>& Y_raw,
                                   const std::vector<int>& idx_ent,
                                   const std::vector<int>& idx_val,
                                   double mse_objetivo,
                                   int max_epocas,
                                   const std::function<void(float)>& progreso,
                                   const std::atomic<bool>& cancelar) {
    ResultadoANN res;
    if (X_raw.empty() || Y_raw.empty() || idx_ent.empty() || W_.empty()) {
        res.mensaje = "Datos o red no inicializados.";
        return res;
    }

    // ── Escalado Min-Max a [-0.9, 0.9] como en HSC Data ──
    std::vector<std::vector<double>> X = X_raw;
    std::vector<std::vector<double>> Y = Y_raw;
    if (cfg_.normalizar) {
        x_min_.assign(n_ent_, 1e300);
        x_max_.assign(n_ent_, -1e300);
        y_min_.assign(n_sal_, 1e300);
        y_max_.assign(n_sal_, -1e300);
        for (size_t r = 0; r < X_raw.size(); ++r) {
            for (int d = 0; d < n_ent_ && d < (int)X_raw[r].size(); ++d) {
                x_min_[d] = std::min(x_min_[d], X_raw[r][d]);
                x_max_[d] = std::max(x_max_[d], X_raw[r][d]);
            }
            for (int m = 0; m < n_sal_ && m < (int)Y_raw[r].size(); ++m) {
                y_min_[m] = std::min(y_min_[m], Y_raw[r][m]);
                y_max_[m] = std::max(y_max_[m], Y_raw[r][m]);
            }
        }
        normalizado_ = true;
        for (size_t r = 0; r < X_raw.size(); ++r) {
            X[r] = normalizar_x(X_raw[r]);
            Y[r] = normalizar_y(Y_raw[r]);
        }
    } else {
        normalizado_ = false;
    }

    std::vector<std::vector<double>> eval_a(W_.size() + 1);
    eval_a[0].resize(n_ent_);
    for (size_t l = 0; l < W_.size(); ++l) eval_a[l + 1].resize(W_[l].size());

    auto mse_de = [&](const std::vector<int>& idx) {
        if (idx.empty()) return 0.0;
        double s = 0.0;
        size_t cuenta = 0;
        for (int i : idx) {
            forward(X[i], eval_a);
            const auto& y = eval_a.back();
            for (size_t j = 0; j < y.size() && j < Y[i].size(); ++j) {
                double d = y[j] - Y[i][j];
                s += d * d;
                ++cuenta;
            }
        }
        return cuenta ? (s / (double)cuenta) : 0.0;
    };

    auto registrar_punto = [&](int epoca_num, double mse_e, double mse_v) {
        res.epoca_hist.push_back((double)epoca_num);
        res.mse_ent_hist.push_back(mse_e);
        res.mse_val_hist.push_back(mse_v);
        res.rmse_ent_hist.push_back(std::sqrt(std::max(0.0, mse_e)));
        res.rmse_val_hist.push_back(std::sqrt(std::max(0.0, mse_v)));
    };

    if (cfg_.tipo == 1) {
        // ── PSO (Particle Swarm Optimizer) sobre el vector de pesos ──
        std::mt19937 rng(static_cast<unsigned>(cfg_.semilla));
        std::uniform_real_distribution<double> u(0.0, 1.0);
        std::uniform_real_distribution<double> u_sym(-0.8, 0.8);
        const size_t NP = total_params();
        const int S = std::max(10, cfg_.particulas_pso);
        const double w_inercia = 0.72, c1 = 1.49, c2 = 1.49;
        const double v_max = 0.5;

        std::vector<std::vector<double>> pos(S), vel(S), pbest(S);
        std::vector<double> fit(S, 1e300), pbest_fit(S, 1e300);
        auto evaluar = [&](const std::vector<double>& p) {
            fijar_params(p);
            return mse_de(idx_ent);
        };
        auto base = obtener_params();
        for (int s = 0; s < S; ++s) {
            pos[s] = base;
            if (s > 0) {
                for (auto& v : pos[s]) v += u_sym(rng); // perturba pesos Y sesgos con cambio de signo
            }
            vel[s].assign(NP, 0.0);
            fit[s] = evaluar(pos[s]);
            pbest[s] = pos[s];
            pbest_fit[s] = fit[s];
        }
        int gbest = 0;
        for (int s = 1; s < S; ++s) if (fit[s] < fit[gbest]) gbest = s;
        std::vector<double> gbest_pos = pos[gbest];
        double gbest_fit = fit[gbest];

        const int registrar_cada = std::max(1, max_epocas / 500);
        for (int it = 0; it < max_epocas; ++it) {
            if (cancelar.load()) { res.cancelado = true; break; }
            for (int s = 0; s < S; ++s) {
                for (size_t k = 0; k < NP; ++k) {
                    vel[s][k] = w_inercia * vel[s][k]
                        + c1 * u(rng) * (pbest[s][k] - pos[s][k])
                        + c2 * u(rng) * (gbest_pos[k] - pos[s][k]);
                    vel[s][k] = std::clamp(vel[s][k], -v_max, v_max);
                    pos[s][k] += vel[s][k];
                }
                fit[s] = evaluar(pos[s]);
                if (fit[s] < pbest_fit[s]) { pbest_fit[s] = fit[s]; pbest[s] = pos[s]; }
                if (fit[s] < gbest_fit) { gbest_fit = fit[s]; gbest_pos = pos[s]; }
            }
            fijar_params(gbest_pos);
            res.epocas = it + 1;
            if (it % registrar_cada == 0 || it == max_epocas - 1 || gbest_fit < mse_objetivo) {
                registrar_punto(res.epocas, gbest_fit, idx_val.empty() ? gbest_fit : mse_de(idx_val));
            }
            if (progreso) progreso((float)it / (float)max_epocas);
            if (gbest_fit < mse_objetivo) break;
        }
        fijar_params(gbest_pos);
        res.mse_ent = gbest_fit;
        res.mse_val = idx_val.empty() ? gbest_fit : mse_de(idx_val);
        res.rmse_ent = std::sqrt(std::max(0.0, res.mse_ent));
        res.rmse_val = std::sqrt(std::max(0.0, res.mse_val));
        res.ok = true;
        res.mensaje = "PSO: " + std::to_string(res.epocas) + " iteraciones.";
        return res;
    }

    if (cfg_.tipo == 2) {
        // ── RNN (Random Neural Network / Adaptive Random Weight Search) ──
        std::mt19937 rng(static_cast<unsigned>(cfg_.semilla));
        std::normal_distribution<double> gauss(0.0, 1.0);
        std::uniform_real_distribution<double> u01(0.0, 1.0);

        std::vector<double> mejor_p = obtener_params();
        double mejor = mse_de(idx_ent);
        double radio = 0.25;
        const size_t NP = mejor_p.size();
        const int registrar_cada = std::max(1, max_epocas / 500);

        for (int it = 0; it < max_epocas; ++it) {
            if (cancelar.load()) { res.cancelado = true; break; }
            std::vector<double> p = mejor_p;
            // Perturbar un subconjunto aleatorio de pesos (~25% o al menos 4 parámetros)
            double prob = std::max(4.0 / std::max<size_t>(1, NP), 0.25);
            for (size_t k = 0; k < NP; ++k) {
                if (u01(rng) <= prob) {
                    p[k] += gauss(rng) * radio;
                }
            }
            fijar_params(p);
            double e = mse_de(idx_ent);
            if (e < mejor) {
                mejor = e;
                mejor_p = std::move(p);
                radio = std::min(0.5, radio * 1.05);
            } else {
                fijar_params(mejor_p);
                radio = std::max(1e-4, radio * 0.995);
            }

            res.epocas = it + 1;
            if (it % registrar_cada == 0 || it == max_epocas - 1 || mejor < mse_objetivo) {
                registrar_punto(res.epocas, mejor, idx_val.empty() ? mejor : mse_de(idx_val));
            }
            if (progreso) progreso((float)it / (float)max_epocas);
            if (mejor < mse_objetivo) break;
        }
        fijar_params(mejor_p);
        res.mse_ent = mejor;
        res.mse_val = idx_val.empty() ? mejor : mse_de(idx_val);
        res.rmse_ent = std::sqrt(std::max(0.0, res.mse_ent));
        res.rmse_val = std::sqrt(std::max(0.0, res.mse_val));
        res.ok = true;
        res.mensaje = "RNN aleatoria: " + std::to_string(res.epocas) + " rondas.";
        return res;
    }

    // ── FFNN backpropagation con mini-batch SGD, momento y decaimiento ──
    const int L = (int)W_.size();
    std::vector<std::vector<std::vector<double>>> gW(L), mW(L);
    std::vector<std::vector<double>> gB(L), mB(L);
    for (int l = 0; l < L; ++l) {
        gW[l].assign(W_[l].size(), std::vector<double>(W_[l][0].size(), 0.0));
        mW[l] = gW[l];
        gB[l].assign(B_[l].size(), 0.0);
        mB[l] = gB[l];
    }

    std::vector<int> orden = idx_ent;
    std::mt19937 rng(static_cast<unsigned>(cfg_.semilla));
    std::vector<std::vector<double>> a(L + 1), delta(L + 1);
    for (int l = 0; l <= L; ++l) {
        a[l].resize(l == 0 ? n_ent_ : (int)W_[l - 1].size());
        delta[l].resize(a[l].size());
    }

    const int batch_sz = std::clamp(cfg_.batch_size, 1, (int)orden.size());
    const int registrar_cada = std::max(1, std::min(max_epocas / 400, 10));
    int step_count = 0;

    for (int ep = 0; ep < max_epocas; ++ep) {
        if (cancelar.load()) { res.cancelado = true; break; }
        std::shuffle(orden.begin(), orden.end(), rng);

        int b_count = 0;
        for (auto& gw : gW) for (auto& f : gw) std::fill(f.begin(), f.end(), 0.0);
        for (auto& gb : gB) std::fill(gb.begin(), gb.end(), 0.0);

        for (size_t pos_idx = 0; pos_idx < orden.size(); ++pos_idx) {
            int idx = orden[pos_idx];
            forward(X[idx], a);
            // delta de salida (MSE): dE/dz = (y_pred - y_true) * f'(z)
            for (size_t j = 0; j < delta[L].size(); ++j) {
                double t = (j < Y[idx].size()) ? Y[idx][j] : 0.0;
                delta[L][j] = (a[L][j] - t) * activar_deriv(0.0, a[L][j], cfg_.act_salida);
            }

            for (int l = L - 1; l >= 0; --l) {
                for (size_t j = 0; j < delta[l + 1].size(); ++j) {
                    gB[l][j] += delta[l + 1][j];
                    for (size_t i = 0; i < W_[l][j].size(); ++i)
                        gW[l][j][i] += delta[l + 1][j] * a[l][i];
                }
                if (l > 0) {
                    int tipo = cfg_.act_oculta;
                    for (size_t i = 0; i < delta[l].size(); ++i) {
                        double s = 0.0;
                        for (size_t j = 0; j < W_[l].size(); ++j)
                            s += W_[l][j][i] * delta[l + 1][j];
                        double y = a[l][i];
                        delta[l][i] = s * activar_deriv(0.0, y, tipo);
                    }
                }
            }

            ++b_count;
            if (b_count >= batch_sz || pos_idx + 1 == orden.size()) {
                double lr = cfg_.tasa / (1.0 + cfg_.decaimiento * (double)step_count);
                ++step_count;
                double inv_b = 1.0 / (double)b_count;
                for (int l = 0; l < L; ++l) {
                    for (size_t j = 0; j < W_[l].size(); ++j) {
                        for (size_t i = 0; i < W_[l][j].size(); ++i) {
                            double g = std::clamp(gW[l][j][i] * inv_b, -10.0, 10.0);
                            mW[l][j][i] = cfg_.momento * mW[l][j][i] - lr * g;
                            W_[l][j][i] += mW[l][j][i];
                            gW[l][j][i] = 0.0;
                        }
                        double g = std::clamp(gB[l][j] * inv_b, -10.0, 10.0);
                        mB[l][j] = cfg_.momento * mB[l][j] - lr * g;
                        B_[l][j] += mB[l][j];
                        gB[l][j] = 0.0;
                    }
                }
                b_count = 0;
            }
        }

        res.epocas = ep + 1;
        double mse_e_actual = mse_de(idx_ent);
        if (ep % registrar_cada == 0 || ep == max_epocas - 1 || mse_e_actual < mse_objetivo) {
            double mse_v_actual = idx_val.empty() ? mse_e_actual : mse_de(idx_val);
            registrar_punto(res.epocas, mse_e_actual, mse_v_actual);
        }
        if (progreso) progreso((float)ep / (float)max_epocas);
        if (mse_e_actual < mse_objetivo) break;
    }

    res.mse_ent = mse_de(idx_ent);
    res.mse_val = idx_val.empty() ? res.mse_ent : mse_de(idx_val);
    res.rmse_ent = std::sqrt(std::max(0.0, res.mse_ent));
    res.rmse_val = std::sqrt(std::max(0.0, res.mse_val));
    res.ok = true;
    res.mensaje = "FFNN: " + std::to_string(res.epocas) + " épocas.";
    return res;
}

std::string RedNeuronal::a_json() const {
    std::ostringstream os;
    os << std::setprecision(17);  // roundtrip exacto de doubles
    os << "{\"topologia\":[";
    auto topo = topologia();
    for (size_t i = 0; i < topo.size(); ++i) { if (i) os << ","; os << topo[i]; }
    os << "],\"meta\":[" << cfg_.act_oculta << "," << cfg_.act_salida << "," << cfg_.tipo << "," << cfg_.batch_size;
    os << "],\"w\":[";
    bool primero = true;
    for (auto& capa : W_) for (auto& fila : capa) for (double v : fila) {
        if (!primero) os << ",";
        os << v;
        primero = false;
    }
    os << "],\"b\":[";
    primero = true;
    for (auto& b : B_) for (double v : b) { if (!primero) os << ","; os << v; primero = false; }
    os << "]";
    if (normalizado_) {
        auto write_arr = [&](const char* key, const std::vector<double>& arr) {
            os << ",\"" << key << "\":[";
            for (size_t i = 0; i < arr.size(); ++i) { if (i) os << ","; os << arr[i]; }
            os << "]";
        };
        write_arr("xmin", x_min_);
        write_arr("xmax", x_max_);
        write_arr("ymin", y_min_);
        write_arr("ymax", y_max_);
    }
    os << "}";
    return os.str();
}

bool RedNeuronal::desde_json(const std::string& texto) {
    // parseo mínimo sin dependencias: extrae arrays numéricos
    auto extraer = [&](const std::string& clave) {
        std::vector<double> out;
        size_t p = texto.find("\"" + clave + "\":[");
        if (p == std::string::npos) return out;
        p += clave.size() + 4;  // saltar: comilla + clave + comilla + ':' + '['
        size_t fin = texto.find(']', p);
        if (fin == std::string::npos) return out;
        std::string cuerpo = texto.substr(p, fin - p);
        size_t i = 0;
        while (i < cuerpo.size()) {
            size_t coma = cuerpo.find(',', i);
            if (coma == std::string::npos) coma = cuerpo.size();
            try { out.push_back(std::stod(cuerpo.substr(i, coma - i))); } catch (...) {}
            i = coma + 1;
        }
        return out;
    };
    auto topo_v = extraer("topologia");
    auto meta_v = extraer("meta");
    auto w = extraer("w");
    auto b = extraer("b");
    if (topo_v.size() < 2) return false;
    n_ent_ = (int)topo_v.front();
    n_sal_ = (int)topo_v.back();
    cfg_.ocultas.clear();
    for (size_t i = 1; i + 1 < topo_v.size(); ++i)
        cfg_.ocultas.push_back((int)topo_v[i]);
    if (meta_v.size() >= 2) {
        cfg_.act_oculta = (int)meta_v[0];
        cfg_.act_salida = (int)meta_v[1];
    }
    if (meta_v.size() >= 3) cfg_.tipo = (int)meta_v[2];
    if (meta_v.size() >= 4) cfg_.batch_size = std::max(1, (int)meta_v[3]);
    W_.clear();
    B_.clear();
    size_t k = 0, kb = 0;   // índices independientes para pesos (w) y sesgos (b)
    for (size_t l = 0; l + 1 < topo_v.size(); ++l) {
        int fout = (int)topo_v[l + 1], fin = (int)topo_v[l];
        std::vector<std::vector<double>> capa(fout, std::vector<double>(fin));
        for (auto& fila : capa) for (auto& v : fila) v = (k < w.size()) ? w[k++] : 0.0;
        W_.push_back(capa);
        std::vector<double> ses(fout);
        for (auto& v : ses) v = (kb < b.size()) ? b[kb++] : 0.0;
        B_.push_back(ses);
    }
    auto xmin = extraer("xmin");
    auto xmax = extraer("xmax");
    auto ymin = extraer("ymin");
    auto ymax = extraer("ymax");
    if (xmin.size() == (size_t)n_ent_ && xmax.size() == (size_t)n_ent_ &&
        ymin.size() == (size_t)n_sal_ && ymax.size() == (size_t)n_sal_) {
        x_min_ = std::move(xmin);
        x_max_ = std::move(xmax);
        y_min_ = std::move(ymin);
        y_max_ = std::move(ymax);
        normalizado_ = true;
    } else {
        normalizado_ = false;
    }
    return !W_.empty();
}

std::string RedNeuronal::a_xml() const {
    auto act_nombre = [](int a) {
        switch (a) {
            case 0: return "TanH";
            case 1: return "Sigmoid";
            case 2: return "ReLU";
            default: return "Linear";
        }
    };
    std::ostringstream os;
    os << std::setprecision(8);
    os << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
    os << "<NeuralNetwork type=\"" << (cfg_.tipo == 0 ? "FFNN" : cfg_.tipo == 1 ? "PSO" : "RNN") << "\">\n";
    os << "  <Hyperparameters learningRate=\"" << cfg_.tasa
       << "\" decay=\"" << cfg_.decaimiento
       << "\" momentum=\"" << cfg_.momento
       << "\" batchSize=\"" << cfg_.batch_size
       << "\" hiddenActivation=\"" << act_nombre(cfg_.act_oculta)
       << "\" outputActivation=\"" << act_nombre(cfg_.act_salida) << "\"/>\n";
    auto topo = topologia();
    os << "  <Structure inputs=\"" << n_ent_ << "\" outputs=\"" << n_sal_ << "\" layers=\"" << topo.size() << "\">\n";
    for (size_t l = 0; l < W_.size(); ++l) {
        os << "    <Layer index=\"" << (l + 1) << "\" neurons=\"" << W_[l].size()
           << "\" activation=\"" << act_nombre(l + 1 == W_.size() ? cfg_.act_salida : cfg_.act_oculta) << "\">\n";
        for (size_t j = 0; j < W_[l].size(); ++j) {
            os << "      <Neuron index=\"" << j << "\" bias=\"" << B_[l][j] << "\">";
            for (size_t i = 0; i < W_[l][j].size(); ++i) {
                if (i) os << " ";
                os << W_[l][j][i];
            }
            os << "</Neuron>\n";
        }
        os << "    </Layer>\n";
    }
    os << "  </Structure>\n";
    if (normalizado_) {
        os << "  <Normalization range=\"[-0.9, 0.9]\">\n";
        for (size_t i = 0; i < x_min_.size(); ++i)
            os << "    <Input index=\"" << i << "\" min=\"" << x_min_[i] << "\" max=\"" << x_max_[i] << "\"/>\n";
        for (size_t i = 0; i < y_min_.size(); ++i)
            os << "    <Output index=\"" << i << "\" min=\"" << y_min_[i] << "\" max=\"" << y_max_[i] << "\"/>\n";
        os << "  </Normalization>\n";
    }
    os << "</NeuralNetwork>\n";
    return os.str();
}

}} // namespace
