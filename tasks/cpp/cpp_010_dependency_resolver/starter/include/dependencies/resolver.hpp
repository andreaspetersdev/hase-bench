#pragma once

#include <string>
#include <vector>

namespace hase::dependencies {

struct Module {
    std::string name;
    std::vector<std::string> dependencies;
};

enum class ResolutionError {
    none,
    duplicate_module,
    missing_dependency,
    cycle,
};

struct ResolutionResult {
    std::vector<std::string> order;
    ResolutionError error{ResolutionError::none};
    std::string module;
    std::string dependency;
    std::vector<std::string> cycle_path;

    explicit operator bool() const noexcept { return error == ResolutionError::none; }
};

[[nodiscard]] ResolutionResult resolve(const std::vector<Module>& modules);

} // namespace hase::dependencies
