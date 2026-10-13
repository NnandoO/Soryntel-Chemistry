#include "especiacion_acuosa.hpp"
#include <Eigen/Dense>
#include <algorithm>
#include <cmath>
#include <limits>
namespace cma::motores {
namespace {
double gamma_log(double charge,double strength,ActivityModel model) {
    if (model == ActivityModel::Ideal) return 0;
    const double root = std::sqrt(strength);
    return -0.509 * charge * charge * (root / (1 + root) - 0.3 * strength);
}
}
SpeciationResult AqueousSpeciation::solve(const SpeciationInput& input) const {
    SpeciationResult result;
    auto invalid = [&](const std::string& message) { result.message = message; return result; };
    if (!std::isfinite(input.kelvin) || input.kelvin <= 0 || input.totals_molal.empty() ||
        input.totals_molal.size() > 100) return invalid("Temperatura/totales inválidos");
    if (input.model == ActivityModel::Davies && std::abs(input.kelvin-298.15)>1e-8)
        return invalid("Davies implementado a 298.15 K; no se extrapola sin parámetros");
    std::vector<int> ids;
    for (const auto& [id,total] : input.totals_molal) {
        if (!database_.components.count(id) || id == 2 || id == 330 || id == 1 ||
            input.fixed_log_activities.count(id) || !std::isfinite(total) || total <= 0)
            return invalid("Totales deben ser positivos y no representar H2O/H+/electrones ni actividades fijadas");
        ids.push_back(id);
    }
    for (const auto& [id,value] : input.fixed_log_activities)
        if (!database_.components.count(id) || !std::isfinite(value) || value < -100 || value > 10)
            return invalid("Actividad fijada inválida");
    std::vector<const domain::FormationReaction*> included;
    for (const auto& [id,reaction] : database_.reactions) {
        bool supported = true;
        for (const auto& [component,coefficient] : reaction.components) {
            if (!input.totals_molal.count(component) && !input.fixed_log_activities.count(component)) supported=false;
            if (input.totals_molal.count(component) && coefficient < 0) supported=false;
        }
        if (supported) included.push_back(&reaction);
    }
    const auto count = static_cast<Eigen::Index>(ids.size());
    Eigen::VectorXd logs(count), totals(count);
    for (Eigen::Index i=0;i<count;++i) { totals(i)=input.totals_molal.at(ids[static_cast<size_t>(i)]); logs(i)=std::log10(totals(i)); }
    double strength = 0;
    std::map<int,double> concentrations, activities;
    for (int outer=0;outer<(input.model==ActivityModel::Ideal ? 1 : 80);++outer) {
        auto evaluate = [&](const Eigen::VectorXd& trial,Eigen::VectorXd& computed,Eigen::MatrixXd* jacobian) {
            computed = Eigen::VectorXd::Zero(count);
            if (jacobian) *jacobian = Eigen::MatrixXd::Zero(count,count);
            activities=input.fixed_log_activities; concentrations.clear();
            for (Eigen::Index i=0;i<count;++i) {
                const auto id=ids[static_cast<size_t>(i)];
                const double concentration=std::pow(10.0,trial(i));
                activities[id]=trial(i)+gamma_log(database_.components.at(id).charge,strength,input.model);
                computed(i)=concentration;
                if (jacobian) (*jacobian)(i,i)=std::log(10.0)*concentration;
            }
            for (const auto* reaction : included) {
                if (reaction->phase != domain::AqueousPhase::Dissolved) continue;
                double logarithm = reaction->log_k(input.kelvin);
                for (const auto& [id,coefficient] : reaction->components) logarithm+=coefficient*activities.at(id);
                logarithm-=gamma_log(reaction->charge,strength,input.model);
                if (logarithm > 100 || !std::isfinite(logarithm)) throw std::invalid_argument("Actividad fuera de rango");
                const double concentration=std::pow(10.0,std::max(-300.0,logarithm));
                concentrations[reaction->id]=concentration;
                for (Eigen::Index i=0;i<count;++i) {
                    const auto it=reaction->components.find(ids[static_cast<size_t>(i)]);
                    if (it==reaction->components.end()) continue;
                    computed(i)+=it->second*concentration;
                    if (jacobian) for (Eigen::Index j=0;j<count;++j) {
                        const auto jt=reaction->components.find(ids[static_cast<size_t>(j)]);
                        if (jt!=reaction->components.end()) (*jacobian)(i,j)+=std::log(10.0)*it->second*jt->second*concentration;
                    }
                }
            }
            computed=(computed-totals).cwiseQuotient(totals);
            if (jacobian) for (Eigen::Index i=0;i<count;++i) jacobian->row(i)/=totals(i);
        };
        try {
            bool solved=false;
            for (int iteration=0;iteration<150;++iteration) {
                ++result.iterations;
                Eigen::VectorXd residual; Eigen::MatrixXd jacobian;
                evaluate(logs,residual,&jacobian);
                if (residual.cwiseAbs().maxCoeff()<1e-10) { solved=true; break; }
                Eigen::VectorXd step=jacobian.colPivHouseholderQr().solve(-residual);
                if (!step.allFinite()) break;
                const double maximum=step.cwiseAbs().maxCoeff();
                if (maximum>2) step*=2/maximum;
                bool accepted=false;
                for (double alpha=1;alpha>1e-10;alpha*=0.5) {
                    const Eigen::VectorXd trial=logs+alpha*step;
                    Eigen::VectorXd next; evaluate(trial,next,nullptr);
                    if (next.allFinite() && next.norm()<residual.norm()) { logs=trial; accepted=true; break; }
                }
                if (!accepted) break;
            }
            if (!solved) return invalid("Especiación no convergió");
            Eigen::VectorXd final; evaluate(logs,final,nullptr);
            result.mass_residual=final.cwiseAbs().maxCoeff();
            result.dissolved_species=concentrations;
            result.free_components.clear(); result.calculated_totals.clear();
            double next_strength=0, charge=0;
            for (const auto& [id,log_activity] : activities) {
                if (id==2) continue; // solvent has activity, not solute molality
                const double concentration=std::pow(10.0,log_activity-gamma_log(database_.components.at(id).charge,strength,input.model));
                const double z=database_.components.at(id).charge;
                result.free_components[id]=concentration;
                next_strength+=0.5*concentration*z*z; charge+=concentration*z;
            }
            for (const auto& [id,concentration] : concentrations) {
                const double z=database_.reactions.at(id).charge;
                next_strength+=0.5*concentration*z*z; charge+=concentration*z;
            }
            for (Eigen::Index i=0;i<count;++i) result.calculated_totals[ids[static_cast<size_t>(i)]]=totals(i)*(1+final(i));
            result.ionic_strength=next_strength; result.charge_residual=charge;
            if (input.model==ActivityModel::Davies && next_strength>0.5)
                return invalid("Davies fuera de rango: fuerza iónica > 0.5 mol/kg");
            if (input.model==ActivityModel::Ideal || std::abs(next_strength-strength)<1e-9*std::max(1.0,next_strength)) {
                for (const auto* reaction : included) if (reaction->phase==domain::AqueousPhase::Solid) {
                    double index=reaction->log_k(input.kelvin);
                    for (const auto& [id,coefficient] : reaction->components) index+=coefficient*activities.at(id);
                    result.saturation_indices[reaction->id]=index;
                }
                result.ok=true;
                result.message="pH/actividades fijadas; sólidos reportan saturación, sin precipitación. Residuo de carga requiere revisar buffer/contraiones.";
                return result;
            }
            strength=0.5*(strength+next_strength);
        } catch (const std::exception& e) { return invalid(e.what()); }
    }
    return invalid("Coeficientes de actividad no convergieron");
}
}
