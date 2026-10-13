#pragma once
#include "domain/ports/AqueousEngine.hpp"
#include <utility>
namespace cma::adapters {
class PhreeqcEngine final : public domain::AqueousEngine {
public:
    explicit PhreeqcEngine(std::string directory) : directory_(std::move(directory)) {}
    bool available() const override;
    domain::AqueousResponse solve(const domain::AqueousRequest& input) const override;
private:
    std::string directory_;
};
}
