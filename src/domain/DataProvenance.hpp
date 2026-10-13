#pragma once
#include <string>
namespace cma::domain {
enum class DataQuality { Unverified, Estimated, Referenced, Validated };
struct DataProvenance {
    std::string source = "legacy catalog; origin not established";
    std::string reference;
    std::string revision;
    DataQuality quality = DataQuality::Unverified;
    double min_kelvin = 0, max_kelvin = 0;
    bool contains(double kelvin) const {
        return min_kelvin > 0 && max_kelvin >= min_kelvin && kelvin >= min_kelvin && kelvin <= max_kelvin;
    }
};
}
