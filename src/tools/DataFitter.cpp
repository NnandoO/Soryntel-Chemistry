#include "DataFitter.hpp"
#include <Eigen/Dense>
#include <cmath>
#include <algorithm>

namespace cma::application::modules {
ResultadoFit DataFitter::lineal(const std::vector<double>& x, const std::vector<double>& y) {
    return polinomial(x, y, 1);
}
ResultadoFit DataFitter::polinomial(const std::vector<double>& x, const std::vector<double>& y, int grado) {
    ResultadoFit result;
    if (grado < 0 || grado > 12 || x.size() != y.size() || x.size() < static_cast<size_t>(grado + 1) ||
        x.size() > 1000000 || std::any_of(x.begin(),x.end(),[](double v){ return !std::isfinite(v); }) ||
        std::any_of(y.begin(),y.end(),[](double v){ return !std::isfinite(v); })) {
        result.mensaje = "Datos no finitos, grado inválido o puntos insuficientes"; return result;
    }
    const auto bounds = std::minmax_element(x.begin(), x.end());
    const double center = (*bounds.first + *bounds.second) / 2;
    const double scale = (*bounds.second - *bounds.first) / 2;
    if (grado > 0 && scale <= 0) { result.mensaje = "Valores X no identifican un polinomio"; return result; }
    Eigen::MatrixXd design(static_cast<Eigen::Index>(x.size()), grado + 1);
    Eigen::VectorXd target(static_cast<Eigen::Index>(x.size()));
    for (size_t i = 0; i < x.size(); ++i) {
        const double u = scale > 0 ? (x[i] - center) / scale : 0;
        double power = 1;
        for (int j = 0; j <= grado; ++j) { design(static_cast<Eigen::Index>(i),j) = power; power *= u; }
        target(static_cast<Eigen::Index>(i)) = y[i];
    }
    Eigen::ColPivHouseholderQR<Eigen::MatrixXd> solver(design);
    if (solver.rank() != grado + 1) { result.mensaje = "Polinomio no identificable (matriz singular)"; return result; }
    const Eigen::VectorXd coefficients = solver.solve(target);
    if (!coefficients.allFinite()) { result.mensaje = "Ajuste no finito"; return result; }
    result.coeficientes.assign(static_cast<size_t>(grado + 1), 0);
    // Transform the centered polynomial back to coefficients of powers of x.
    const double divisor = scale > 0 ? scale : 1;
    for (int j = 0; j <= grado; ++j) {
        double binomial = 1;
        for (int k = 0; k <= j; ++k) {
            result.coeficientes[static_cast<size_t>(k)] += coefficients(j) * binomial * std::pow(-center, j-k) / std::pow(divisor,j);
            if (k < j) binomial *= static_cast<double>(j-k) / (k+1);
        }
    }
    if (std::any_of(result.coeficientes.begin(),result.coeficientes.end(),[](double v){return !std::isfinite(v);})) {
        result.mensaje = "Coeficientes fuera de rango"; return result;
    }
    const double residual = (design * coefficients - target).squaredNorm();
    const double total = (target.array() - target.mean()).matrix().squaredNorm();
    result.r_cuadrado = total > 0 ? 1 - residual / total : (residual < 1e-20 ? 1 : 0);
    result.ok = true; result.mensaje = "Ajuste por QR con X centrada y escalada";
    return result;
}
}
