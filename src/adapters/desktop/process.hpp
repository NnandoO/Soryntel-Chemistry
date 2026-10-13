#pragma once
#include "application/ports/FileDialog.hpp"
#include <cstdio>
#include <string>
namespace cma::adapters {
class DesktopFileDialog final : public application::FileDialog {
public:
    std::string select(const std::string& title, const std::string& filter, bool save) override;
};
// Compatibility boundary for the existing panels: accepts Zenity arguments only.
// It never invokes a shell; output is returned as a temporary file owned by caller.
FILE* dialog_pipe(const std::string& command);
bool render_molecule(const std::string& script, const std::string& input, const std::string& output);
std::string resource_path(const std::string& relative);
}
