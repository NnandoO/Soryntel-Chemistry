#include "DataFitter.hpp"
#include <Eigen/Dense>
#include <cmath>

namespace hsc_clone {

ResultadoFit DataFitter::lineal(const std::vector<double>& x, const std::vector<double>& y) {
    ResultadoFit res;
    if (x.size() != y.size() || x.size() < 2) {
        res.mensaje = "Los datos deben tener el mismo tamano y al menos 2 puntos";
        return res;
    }

    size_t n = x.size();
    Eigen::MatrixXd X(n, 2);
    Eigen::VectorXd Y(n);
    for (size_t i = 0; i < n; ++i) {
        X(i, 0) = 1.0;
        X(i, 1) = x[i];
        Y(i) = y[i];
    }

    auto ldlt = (X.transpose() * X).ldlt();
    if (ldlt.info() != Eigen::Success) {
        res.mensaje = "Error: Matriz singular";
        return res;
    }
    Eigen::VectorXd coef = ldlt.solve(X.transpose() * Y);

    double y_mean = Y.mean();
    double ss_tot = 0.0, ss_res = 0.0;
    for (size_t i = 0; i < n; ++i) {
        double y_pred = coef(0) + coef(1) * x[i];
        ss_tot += (y[i] - y_mean) * (y[i] - y_mean);
        ss_res += (y[i] - y_pred) * (y[i] - y_pred);
    }

    res.coeficientes = {coef(0), coef(1)};
    res.r_cuadrado = (ss_tot > 0) ? (1.0 - ss_res / ss_tot) : 0.0;
    res.ok = true;
    return res;
}

ResultadoFit DataFitter::polinomial(const std::vector<double>& x, const std::vector<double>& y, int grado) {
    ResultadoFit res;
    if (x.size() != y.size() || x.size() < static_cast<size_t>(grado + 1)) {
        res.mensaje = "Se necesitan al menos " + std::to_string(grado + 1) + " puntos";
        return res;
    }

    size_t n = x.size();
    int m = grado + 1;
    Eigen::MatrixXd X(n, m);
    Eigen::VectorXd Y(n);
    for (size_t i = 0; i < n; ++i) {
        double xp = 1.0;
        for (int j = 0; j < m; ++j) {
            X(i, j) = xp;
            xp *= x[i];
        }
        Y(i) = y[i];
    }

    auto ldlt = (X.transpose() * X).ldlt();
    if (ldlt.info() != Eigen::Success) {
        res.mensaje = "Error: Matriz singular";
        return res;
    }
    Eigen::VectorXd coef = ldlt.solve(X.transpose() * Y);

    double y_mean = Y.mean();
    double ss_tot = 0.0, ss_res = 0.0;
    for (size_t i = 0; i < n; ++i) {
        double y_pred = 0.0;
        double xp = 1.0;
        for (int j = 0; j < m; ++j) {
            y_pred += coef(j) * xp;
            xp *= x[i];
        }
        ss_tot += (y[i] - y_mean) * (y[i] - y_mean);
        ss_res += (y[i] - y_pred) * (y[i] - y_pred);
    }

    res.coeficientes.resize(m);
    for (int j = 0; j < m; ++j)
        res.coeficientes[j] = coef(j);
    res.r_cuadrado = (ss_tot > 0) ? (1.0 - ss_res / ss_tot) : 0.0;
    res.ok = true;
    return res;
}

} // namespace hsc_clone
