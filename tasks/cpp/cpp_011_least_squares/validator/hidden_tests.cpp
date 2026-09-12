#include "numerics/least_squares.hpp"

#include <cassert>
#include <cmath>
#include <limits>
#include <numeric>
#include <vector>

using hase::numerics::LeastSquaresResult;
using hase::numerics::SolveError;
using hase::numerics::solve_least_squares;

namespace {

double residual_norm(std::size_t rows, std::size_t columns, const std::vector<double>& matrix,
                     const std::vector<double>& rhs, const std::vector<double>& solution) {
    double sum = 0.0;
    for (std::size_t row = 0; row < rows; ++row) {
        double value = -rhs[row];
        for (std::size_t column = 0; column < columns; ++column) {
            value += matrix[row * columns + column] * solution[column];
        }
        sum = std::hypot(sum, value);
    }
    return sum;
}

void require_success(const LeastSquaresResult& result, std::size_t columns) {
    assert(result && result.error == SolveError::none);
    assert(result.solution.size() == columns);
    assert(std::isfinite(result.residual_norm));
    for (const double value : result.solution) {
        assert(std::isfinite(value));
    }
}

void noisy_overdetermined_system_has_a_small_residual() {
    // Measurements of y = 1.5 - 0.75x, with deterministic noise.
    const std::vector<double> matrix{
        1, -3, 1, -2, 1, -1, 1, 0, 1, 1, 1, 2, 1, 3,
    };
    const std::vector<double> rhs{3.76, 3.01, 2.23, 1.48, 0.73, -0.02, -0.77};
    const auto result = solve_least_squares(7, 2, matrix, rhs);
    require_success(result, 2);
    const double residual = residual_norm(7, 2, matrix, rhs, result.solution);
    assert(std::abs(result.residual_norm - residual) < 1e-12);
    assert(residual < 0.05);
    assert(std::abs(result.solution[0] - 1.49) < 0.02);
    assert(std::abs(result.solution[1] + 0.755) < 0.01);
}

void higher_dimensional_and_badly_scaled_systems() {
    const std::vector<double> matrix{
        1, 0, 2, 0, 1, -1, 2, 1, 0, -1, 2, 1, 3, -1, 0,
        1, -2, 2, 2, 0, 1, -1, -1, 2,
    };
    const std::vector<double> expected{0.5, -2.0, 1.25};
    std::vector<double> rhs;
    for (std::size_t row = 0; row < 8; ++row) {
        double value = 0.0;
        for (std::size_t column = 0; column < 3; ++column) {
            value += matrix[row * 3 + column] * expected[column];
        }
        rhs.push_back(value);
    }
    const auto result = solve_least_squares(8, 3, matrix, rhs);
    require_success(result, 3);
    assert(residual_norm(8, 3, matrix, rhs, result.solution) < 1e-11);
    for (std::size_t index = 0; index < expected.size(); ++index) {
        assert(std::abs(result.solution[index] - expected[index]) < 1e-10);
    }

    // The two columns have very different magnitudes but are independent.
    const std::vector<double> scaled{
        1e10, 1, 2e10, -1, -1e10, 2, 3e10, 0, -2e10, -3,
    };
    const std::vector<double> scaled_rhs{1e10 + 1, 2e10 - 1, -1e10 + 2, 3e10, -2e10 - 3};
    const auto scaled_result = solve_least_squares(5, 2, scaled, scaled_rhs);
    require_success(scaled_result, 2);
    assert(std::abs(scaled_result.solution[0] - 1.0) < 1e-10);
    assert(std::abs(scaled_result.solution[1] - 1.0) < 1e-6);
}

void conditioning_rank_and_input_contract() {
    // Full rank, but near enough to collinear that normal equations are fragile.
    const std::vector<double> near_collinear{
        1, 1, 1, 1.000001, 1, 0.999999, 1, 1.000002, 1, 0.999998,
    };
    const std::vector<double> rhs{3, 3.000002, 2.999998, 3.000004, 2.999996};
    const auto near_result = solve_least_squares(5, 2, near_collinear, rhs);
    require_success(near_result, 2);
    assert(std::abs(near_result.solution[0] - 1.0) < 1e-7);
    assert(std::abs(near_result.solution[1] - 2.0) < 1e-7);

    const auto deficient = solve_least_squares(4, 2, {1, 1, 2, 2, 3, 3, 4, 4}, {2, 4, 6, 8});
    assert(!deficient && deficient.error == SolveError::rank_deficient);

    // The largest *original* column is the second one, but its direction has
    // only a 0.5-sized component outside the first column. A threshold based
    // only on R's diagonal would incorrectly accept it.
    const auto ordered_near_rank_deficient = solve_least_squares(
        3, 2,
        {1, 1e12,
         0, 0.5,
         1, 1e12},
        {1, 0, 1});
    assert(!ordered_near_rank_deficient && ordered_near_rank_deficient.error == SolveError::rank_deficient);

    const auto non_finite = solve_least_squares(2, 1, {1, std::numeric_limits<double>::infinity()}, {1, 2});
    assert(!non_finite && non_finite.error == SolveError::non_finite_input);
    const auto precedence = solve_least_squares(3, 2, {1, 1, 2, 2, 3, 3},
                                                 {1, std::numeric_limits<double>::quiet_NaN(), 3});
    assert(!precedence && precedence.error == SolveError::non_finite_input);

    const auto malformed = solve_least_squares(2, 2, {1, 0, 0}, {1, 2});
    assert(!malformed && malformed.error == SolveError::dimension_mismatch);
}

} // namespace

int main() {
    noisy_overdetermined_system_has_a_small_residual();
    higher_dimensional_and_badly_scaled_systems();
    conditioning_rank_and_input_contract();
}
