#pragma once

#include <cstddef>
#include <vector>

namespace hase::numerics {

enum class SolveError {
    none,
    dimension_mismatch,
    non_finite_input,
    rank_deficient,
};

struct LeastSquaresResult {
    std::vector<double> solution;
    double residual_norm{};
    SolveError error{SolveError::none};

    explicit operator bool() const noexcept { return error == SolveError::none; }
};

// Coefficients are row-major: coefficients[row * columns + column].
[[nodiscard]] LeastSquaresResult solve_least_squares(
    std::size_t rows,
    std::size_t columns,
    const std::vector<double>& coefficients,
    const std::vector<double>& right_hand_side);

} // namespace hase::numerics
