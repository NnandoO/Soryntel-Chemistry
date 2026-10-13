#pragma once
#include <Eigen/Dense>
#include <vector>
#include <cmath>
namespace cma::application {
inline std::vector<double> explained_variance(const std::vector<std::vector<double>>& observations) {
    if (observations.size() < 2 || observations.front().empty()) return {};
    const auto columns = observations.front().size();
    Eigen::MatrixXd data(static_cast<Eigen::Index>(observations.size()), static_cast<Eigen::Index>(columns));
    for (size_t row = 0; row < observations.size(); ++row) {
        if (observations[row].size() != columns) return {};
        for (size_t col = 0; col < columns; ++col) {
            if (!std::isfinite(observations[row][col])) return {};
            data(static_cast<Eigen::Index>(row),static_cast<Eigen::Index>(col)) = observations[row][col];
        }
    }
    data.rowwise() -= data.colwise().mean().eval();
    for (Eigen::Index col = 0; col < data.cols(); ++col) {
        const double scale = std::sqrt(data.col(col).squaredNorm() / (data.rows() - 1));
        if (scale > 1e-15) data.col(col) /= scale;
    }
    const Eigen::MatrixXd covariance = data.transpose() * data / (data.rows() - 1);
    Eigen::SelfAdjointEigenSolver<Eigen::MatrixXd> solver(covariance);
    if (solver.info() != Eigen::Success) return {};
    const double total = solver.eigenvalues().sum();
    if (total <= 1e-15) return {};
    std::vector<double> result;
    for (Eigen::Index i = static_cast<Eigen::Index>(columns); i-- > 0;)
        result.push_back(100 * std::max(0.0, solver.eigenvalues()(i)) / total);
    return result;
}
}
