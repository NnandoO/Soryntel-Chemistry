#include "process.hpp"
#include <vector>
#include <cctype>
#include <stdexcept>
#include <filesystem>
#ifdef _WIN32
#include <windows.h>
#include <commdlg.h>
#else
#include <spawn.h>
#include <sys/wait.h>
#include <unistd.h>
#include <fcntl.h>
#include <cerrno>
extern char** environ;
#endif

namespace cma::adapters {
std::string resource_path(const std::string& relative) {
    namespace fs = std::filesystem;
    fs::path executable;
#ifdef _WIN32
    char path[32768] = {};
    GetModuleFileNameA(nullptr, path, sizeof(path));
    executable = path;
#else
    std::error_code error;
    executable = fs::read_symlink("/proc/self/exe", error);
#endif
    const fs::path candidates[] = {
        executable.parent_path() / ".." / "share" / "cma-chemistry" / relative,
        executable.parent_path() / relative,
        fs::path(CMA_SOURCE_RESOURCE_DIR) / relative
    };
    for (const auto& candidate : candidates) {
        std::error_code error;
        if (fs::is_regular_file(candidate, error)) return candidate.lexically_normal().string();
    }
    return {};
}
namespace {
std::vector<std::string> arguments(const std::string& command) {
    std::vector<std::string> result;
    std::string word;
    char quote = 0;
    for (char c : command) {
        if (quote) { if (c == quote) quote = 0; else word += c; }
        else if (c == '\'' || c == '"') quote = c;
        else if (std::isspace(static_cast<unsigned char>(c))) {
            if (!word.empty()) { result.push_back(word); word.clear(); }
        } else word += c;
    }
    if (quote) return {};
    if (!word.empty()) result.push_back(word);
    if (result.empty() || result.front() != "zenity") return {};
    for (auto it = result.begin() + 1; it != result.end();) {
        if (*it == "2>/dev/null") it = result.erase(it);
        else if (*it == "--file-selection" || *it == "--save" || *it == "--confirm-overwrite" ||
            it->rfind("--title=", 0) == 0 || it->rfind("--file-filter=", 0) == 0) ++it;
        else return {};
    }
    return result;
}
#ifndef _WIN32
int run(const std::vector<std::string>& values, FILE* output) {
    std::vector<char*> argv;
    for (const auto& value : values) argv.push_back(const_cast<char*>(value.c_str()));
    argv.push_back(nullptr);
    posix_spawn_file_actions_t actions;
    if (posix_spawn_file_actions_init(&actions) != 0) return -1;
    if (output) posix_spawn_file_actions_adddup2(&actions, fileno(output), STDOUT_FILENO);
    posix_spawn_file_actions_addopen(&actions, STDERR_FILENO, "/dev/null", O_WRONLY, 0);
    pid_t child = 0;
    const int status = posix_spawnp(&child, argv.front(), &actions, nullptr, argv.data(), environ);
    posix_spawn_file_actions_destroy(&actions);
    if (status != 0) return -1;
    int result = 0;
    while (waitpid(child, &result, 0) < 0) if (errno != EINTR) return -1;
    return WIFEXITED(result) ? WEXITSTATUS(result) : -1;
}
#endif
}

std::string DesktopFileDialog::select(const std::string& title, const std::string& filter, bool save) {
#ifdef _WIN32
    char path[32768] = {};
    std::string patterns = filter;
    const auto separator = patterns.find('|');
    if (separator != std::string::npos) patterns = patterns.substr(separator + 1);
    patterns.erase(0, patterns.find_first_not_of(' '));
    std::string native_filter = filter + '\0' + patterns + '\0' + '\0';
    OPENFILENAMEA dialog = {};
    dialog.lStructSize = sizeof(dialog);
    dialog.lpstrFile = path;
    dialog.nMaxFile = sizeof(path);
    dialog.lpstrTitle = title.c_str();
    dialog.lpstrFilter = native_filter.c_str();
    dialog.Flags = OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST | (save ? OFN_OVERWRITEPROMPT : OFN_FILEMUSTEXIST);
    return (save ? GetSaveFileNameA(&dialog) : GetOpenFileNameA(&dialog)) ? path : "";
#else
    FILE* output = std::tmpfile();
    if (!output) return {};
    std::vector<std::string> argv = {"zenity", "--file-selection", "--title=" + title, "--file-filter=" + filter};
    if (save) { argv.push_back("--save"); argv.push_back("--confirm-overwrite"); }
    const int result = run(argv, output);
    std::rewind(output);
    std::string path;
    char buffer[4096];
    while (std::fgets(buffer, sizeof(buffer), output)) path += buffer;
    std::fclose(output);
    if (result != 0) return {};
    while (!path.empty() && (path.back() == '\n' || path.back() == '\r')) path.pop_back();
    return path;
#endif
}

FILE* dialog_pipe(const std::string& command) {
    const auto argv = arguments(command);
    if (argv.empty()) return nullptr;
    bool save = false;
    std::string title = "Seleccionar archivo", filter = "*";
    for (const auto& arg : argv) {
        if (arg == "--save") save = true;
        if (arg.rfind("--title=", 0) == 0) title = arg.substr(8);
        if (arg.rfind("--file-filter=", 0) == 0) filter = arg.substr(14);
    }
    DesktopFileDialog dialog;
    const auto path = dialog.select(title, filter, save);
    if (path.empty()) return nullptr;
    FILE* output = std::tmpfile();
    if (!output) return nullptr;
    std::fwrite(path.data(), 1, path.size(), output);
    std::fputc('\n', output);
    std::rewind(output);
    return output;
}

bool render_molecule(const std::string& script, const std::string& input, const std::string& output) {
#ifdef _WIN32
    (void)script; (void)input; (void)output;
    return false;
#else
    return run({"python3", script, input, output}, nullptr) == 0;
#endif
}
}
