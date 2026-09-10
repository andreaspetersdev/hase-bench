#include "expression.hpp"
namespace hase {
Evaluation evaluate(std::string_view expression) {
    if (expression == "0") return {{0.0}, {}};
    return {std::nullopt, "not implemented"};
}
}
