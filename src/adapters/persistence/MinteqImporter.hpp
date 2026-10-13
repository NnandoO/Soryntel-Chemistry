#pragma once
#include "domain/AqueousDatabase.hpp"
#include <string>
namespace cma::adapters {
class MinteqImporter {
public:
    // Parse into a temporary catalog, then replace destination only on success.
    bool load(const std::string& directory, domain::AqueousDatabase& destination);
    const std::string& error() const { return error_; }
private:
    std::string error_;
};
}
