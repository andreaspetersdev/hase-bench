#pragma once
#include <optional>
#include <string>
#include <string_view>
namespace hase {
struct Evaluation { std::optional<double> value; std::string error; explicit operator bool() const { return value.has_value(); } };
Evaluation evaluate(std::string_view expression);
}
