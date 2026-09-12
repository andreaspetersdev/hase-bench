#include "numerics/least_squares.hpp"

#include <cassert>
#include <cmath>
#include <vector>

using hase::numerics::SolveError;
using hase::numerics::solve_least_squares;

namespace {

bool close(double actual, double expected, double tolerance = 1e-10) {
    return std::abs(actual - expected) <= tolerance;
}

void solves_an_exact_rectangular_system() {
    // x = {2, -1}; rows are {1, 0}, {0, 1}, {1, 1}, and {2, -1}.
    const std::vector<double> matrix{
        1, 0,
        0, 1,
        1, 1,
        2, -1,
    };
    const std::vector<double> rhs{2, -1, 1, 5};
    const auto result = solve_least_squares(4, 2, matrix, rhs);
    assert(result);
    assert(result.solution.size() == 2);
    assert(close(result.solution[0], 2.0));
    assert(close(result.solution[1], -1.0));
    assert(close(result.residual_norm, 0.0));
}

void reports_basic_invalid_inputs() {
    const auto shape = solve_least_squares(2, 3, {1, 2, 3, 4, 5, 6}, {1, 2});
    assert(!shape && shape.error == SolveError::dimension_mismatch);
    assert(shape.solution.empty() && shape.residual_norm == 0.0);

    const auto rank = solve_least_squares(3, 2, {1, 2, 2, 4, 3, 6}, {1, 2, 3});
    assert(!rank && rank.error == SolveError::rank_deficient);
    assert(rank.solution.empty() && rank.residual_norm == 0.0);
}

} // namespace

int main() {
    solves_an_exact_rectangular_system();
    reports_basic_invalid_inputs();
}
