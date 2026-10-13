#pragma once
#include <string>
namespace cma::application {
class FileDialog {
public:
    virtual ~FileDialog() = default;
    virtual std::string select(const std::string& title, const std::string& filter, bool save) = 0;
};
}
