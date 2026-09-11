#include "dependencies/resolver.hpp"

#include <algorithm>
#include <cassert>
#include <string>
#include <vector>

using hase::dependencies::Module;
using hase::dependencies::ResolutionError;
using hase::dependencies::resolve;

namespace {

void deterministic_disconnected_diamond() {
    const std::vector<Module> modules{
        {"zeta", {"delta", "alpha"}},
        {"delta", {"bravo"}},
        {"alpha", {"bravo"}},
        {"bravo", {}},
        {"omega", {}},
    };
    const auto result = resolve(modules);
    assert(result);
    assert((result.order == std::vector<std::string>{"bravo", "alpha", "delta", "omega", "zeta"}));

    auto permuted = modules;
    std::reverse(permuted.begin(), permuted.end());
    permuted[0].dependencies = {"alpha", "delta"};
    assert(resolve(permuted).order == result.order);
}

void deep_chain_and_empty_input() {
    std::vector<Module> modules;
    for (int index = 40; index >= 0; --index) {
        modules.push_back({"node_" + std::to_string(index), index == 0 ? std::vector<std::string>{} : std::vector<std::string>{"node_" + std::to_string(index - 1)}});
    }
    const auto result = resolve(modules);
    assert(result);
    assert(result.order.size() == 41);
    for (int index = 0; index <= 40; ++index) {
        assert(result.order[static_cast<std::size_t>(index)] == "node_" + std::to_string(index));
    }
    const auto empty = resolve({});
    assert(empty);
    assert(empty.order.empty());
}

void deterministic_error_precedence_and_diagnostics() {
    const auto duplicate = resolve({
        {"z", {"missing"}}, {"a", {}}, {"z", {}}, {"a", {"also_missing"}},
    });
    assert(!duplicate && duplicate.error == ResolutionError::duplicate_module);
    assert(duplicate.module == "a");
    assert(duplicate.dependency.empty() && duplicate.order.empty() && duplicate.cycle_path.empty());

    const auto missing = resolve({
        {"z", {"m", "a"}}, {"b", {"q", "p"}}, {"a", {"b"}},
    });
    assert(!missing && missing.error == ResolutionError::missing_dependency);
    assert(missing.module == "b" && missing.dependency == "p");
    assert(missing.order.empty() && missing.cycle_path.empty());
}

void directed_cycle_is_closed_canonical_and_lexically_selected() {
    const auto result = resolve({
        {"root", {"z", "a"}},
        {"z", {"y"}},
        {"y", {"z"}},
        {"a", {"c"}},
        {"c", {"b"}},
        {"b", {"a"}},
    });
    assert(!result && result.error == ResolutionError::cycle);
    assert((result.cycle_path == std::vector<std::string>{"a", "c", "b", "a"}));
    assert(result.module.empty() && result.dependency.empty() && result.order.empty());
}

void self_cycle_and_noncycle_dependencies() {
    const auto result = resolve({
        {"consumer", {"library"}},
        {"library", {"library"}},
        {"unrelated", {}},
    });
    assert(!result && result.error == ResolutionError::cycle);
    assert((result.cycle_path == std::vector<std::string>{"library", "library"}));
}

void repeated_dependencies_are_one_edge() {
    const auto result = resolve({
        {"application", {"library", "library", "library"}},
        {"library", {}},
    });
    assert(result);
    assert((result.order == std::vector<std::string>{"library", "application"}));
}

} // namespace

int main() {
    deterministic_disconnected_diamond();
    deep_chain_and_empty_input();
    deterministic_error_precedence_and_diagnostics();
    directed_cycle_is_closed_canonical_and_lexically_selected();
    self_cycle_and_noncycle_dependencies();
    repeated_dependencies_are_one_edge();
}
