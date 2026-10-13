#include "SpeciesConverter.hpp"
#include "nucleo/tabla_periodica.hpp"
#include <Eigen/Dense>
#include <algorithm>
#include <cmath>

namespace cma::application::modules {
std::map<std::string,double> SpeciesConverter::composicion(const std::string& formula) {
    const auto& table = cma::nucleo::TablaPeriodica::instancia();
    const auto atoms = table.analizar_formula(formula);
    double mass = 0;
    for (const auto& atom : atoms) {
        const auto* element = table.por_simbolo(atom.elem);
        if (!element) return {};
        mass += atom.cuenta * element->masa_atomica;
    }
    std::map<std::string,double> fractions;
    if (mass <= 0) return fractions;
    for (const auto& atom : atoms) fractions[atom.elem] = atom.cuenta * table.por_simbolo(atom.elem)->masa_atomica / mass;
    return fractions;
}

std::map<std::string,double> SpeciesConverter::analisisAEspecies(
    const std::map<std::string,double>& input, const std::vector<std::string>& targets) {
    std::map<std::string,double> result;
    if (input.empty() || targets.empty() || targets.size() > 500) return result;
    for (const auto& [name,value] : input) if (!std::isfinite(value) || value < 0) return result;
    std::vector<std::string> rows;
    for (const auto& [name,value] : input) rows.push_back(name);
    Eigen::MatrixXd A = Eigen::MatrixXd::Zero(static_cast<Eigen::Index>(rows.size()),static_cast<Eigen::Index>(targets.size()));
    Eigen::VectorXd b(static_cast<Eigen::Index>(rows.size()));
    for (size_t i=0;i<rows.size();++i) b(static_cast<Eigen::Index>(i)) = input.at(rows[i]);
    for (size_t j=0;j<targets.size();++j) {
        auto fractions = composicion(targets[j]);
        // Compound assays supplied as a row (SiO2, Al2O3...) pass through.
        if (input.count(targets[j]) && !cma::nucleo::TablaPeriodica::instancia().por_simbolo(targets[j])) fractions = {{targets[j],1}};
        if (fractions.empty()) continue;
        if (std::any_of(fractions.begin(),fractions.end(),[&](const auto& f){return !input.count(f.first);})) continue;
        for (size_t i=0;i<rows.size();++i) if (fractions.count(rows[i])) A(static_cast<Eigen::Index>(i),static_cast<Eigen::Index>(j)) = fractions.at(rows[i]);
    }
    // Nonnegative least-squares by cyclic coordinate descent. Report the
    // reconstructed assay in the UI instead of claiming an exact decomposition.
    Eigen::VectorXd amounts = Eigen::VectorXd::Zero(A.cols());
    Eigen::VectorXd residual = b;
    for (int iteration=0;iteration<10000;++iteration) {
        double change=0;
        for (Eigen::Index j=0;j<A.cols();++j) {
            const double norm = A.col(j).squaredNorm();
            if (norm < 1e-20) continue;
            const double next = std::max(0.0, amounts(j)+A.col(j).dot(residual)/norm);
            change = std::max(change,std::abs(next-amounts(j)));
            residual -= A.col(j)*(next-amounts(j));
            amounts(j)=next;
        }
        if (change < 1e-10) break;
    }
    for (size_t j=0;j<targets.size();++j) result[targets[j]] += amounts(static_cast<Eigen::Index>(j));
    return result;
}
}
