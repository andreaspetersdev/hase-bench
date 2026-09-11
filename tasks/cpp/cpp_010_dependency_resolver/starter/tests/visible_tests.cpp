#include "dependencies/resolver.hpp"

#include <cassert>
#include <string>
#include <vector>

using hase::dependencies::Module;
using hase::dependencies::ResolutionError;
using hase::dependencies::resolve;

int main() {
    const auto resolved = resolve({
        {"app", {"core", "ui"}},
        {"ui", {"core"}},
        {"core", {}},
        {"tools", {}},
    });
    assert(resolved);
    assert((resolved.order == std::vector<std::string>{"core", "tools", "ui", "app"}));

    const auto missing = resolve({{"app", {"not-installed"}}});
    assert(!missing);
    assert(missing.error == ResolutionError::missing_dependency);
    assert(missing.module == "app");
    assert(missing.dependency == "not-installed");

    const auto cyclic = resolve({{"a", {"b"}}, {"b", {"a"}}});
    assert(!cyclic);
    assert(cyclic.error == ResolutionError::cycle);
    assert((cyclic.cycle_path == std::vector<std::string>{"a", "b", "a"}));
}
