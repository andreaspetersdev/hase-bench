#include "numerics/least_squares.hpp"

namespace hase::numerics {

LeastSquaresResult solve_least_squares(
    std::size_t,
    std::size_t,
    const std::vector<double>&,
    const std::vector<double>&) {
    return { {}, 0.0, SolveError::rank_deficient };
}

} // namespace hase::numerics
