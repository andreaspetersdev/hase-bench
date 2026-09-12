#include "geometry/quaternion.hpp"

#include <cassert>
#include <cmath>
#include <numbers>

using namespace geometry;

namespace {
bool close(double left, double right, double tolerance = 1e-10) {
    return std::abs(left - right) <= tolerance;
}
void expect_vector(Vec3 value, Vec3 expected) {
    assert(close(value.x, expected.x)); assert(close(value.y, expected.y)); assert(close(value.z, expected.z));
}
}

int main() {
    const auto quarter_turn = from_axis_angle({0, 0, 2}, std::numbers::pi / 2);
    assert(quarter_turn.error == RotationError::none);
    expect_vector(rotate(quarter_turn.value, {1, 0, 0}).value, {0, 1, 0});

    const auto half_turn = compose(quarter_turn.value, quarter_turn.value);
    assert(half_turn.error == RotationError::none);
    expect_vector(rotate(half_turn.value, {1, 0, 0}).value, {-1, 0, 0});

    const auto matrix = to_rotation_matrix(quarter_turn.value);
    assert(matrix.error == RotationError::none);
    const auto restored = from_rotation_matrix(matrix.value);
    assert(restored.error == RotationError::none);
    expect_vector(rotate(restored.value, {1, 0, 0}).value, {0, 1, 0});

    assert(normalize_rotation({0, 0, 0, 0}).error == RotationError::zero_norm);
    assert(from_axis_angle({0, 0, 0}, 0.5).error == RotationError::zero_norm);
}
