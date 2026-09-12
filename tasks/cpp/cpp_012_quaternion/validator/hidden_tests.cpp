#include "geometry/quaternion.hpp"

#include <cassert>
#include <cmath>
#include <limits>
#include <numbers>

using namespace geometry;

namespace {
bool close(double left, double right, double tolerance = 2e-10) { return std::abs(left - right) <= tolerance; }
void expect_vector(Vec3 actual, Vec3 expected, double tolerance = 2e-10) {
    assert(close(actual.x, expected.x, tolerance)); assert(close(actual.y, expected.y, tolerance));
    assert(close(actual.z, expected.z, tolerance));
}
double norm(Quaternion value) { return std::sqrt(value.w * value.w + value.x * value.x + value.y * value.y + value.z * value.z); }
void expect_canonical(Quaternion value) {
    assert(close(norm(value), 1.0));
    assert(value.w >= -1e-15);
    if (std::abs(value.w) <= 1e-15) {
        if (std::abs(value.x) > 1e-15) assert(value.x > 0);
        else if (std::abs(value.y) > 1e-15) assert(value.y > 0);
        else if (std::abs(value.z) > 1e-15) assert(value.z > 0);
    }
}
void expect_identity(const Matrix3& matrix) {
    const Matrix3 identity{1, 0, 0, 0, 1, 0, 0, 0, 1};
    for (size_t index = 0; index != matrix.size(); ++index) assert(matrix[index] == identity[index]);
}
}

int main() {
    const auto around_x = from_axis_angle({4, 0, 0}, std::numbers::pi / 3);
    const auto around_y = from_axis_angle({0, -3, 0}, -std::numbers::pi / 4);
    assert(around_x.error == RotationError::none && around_y.error == RotationError::none);
    expect_canonical(around_x.value); expect_canonical(around_y.value);

    const auto combined = compose(around_x.value, around_y.value);
    assert(combined.error == RotationError::none);
    const Vec3 original{0.25, -2.0, 5.5};
    const auto sequential = rotate(around_y.value, rotate(around_x.value, original).value);
    const auto once = rotate(combined.value, original);
    assert(sequential.error == RotationError::none && once.error == RotationError::none);
    expect_vector(once.value, sequential.value);

    const auto inverse = inverse_rotation({combined.value.w * 17, combined.value.x * 17, combined.value.y * 17, combined.value.z * 17});
    assert(inverse.error == RotationError::none);
    expect_vector(rotate(inverse.value, once.value).value, original);

    const auto sign = normalize_rotation({-2, 0, 0, 0});
    assert(sign.error == RotationError::none); expect_canonical(sign.value); assert(sign.value.w == 1);
    const auto half = from_axis_angle({-1, 0, 0}, std::numbers::pi);
    assert(half.error == RotationError::none); expect_canonical(half.value); assert(half.value.x > 0);

    const auto matrix = to_rotation_matrix({combined.value.w * 8, combined.value.x * 8, combined.value.y * 8, combined.value.z * 8});
    assert(matrix.error == RotationError::none);
    for (int row = 0; row != 3; ++row) for (int column = 0; column != 3; ++column) {
        double dot = 0;
        for (int element = 0; element != 3; ++element) dot += matrix.value[3 * row + element] * matrix.value[3 * column + element];
        assert(close(dot, row == column ? 1.0 : 0.0));
    }
    const auto from_matrix = from_rotation_matrix(matrix.value);
    assert(from_matrix.error == RotationError::none); expect_canonical(from_matrix.value);
    expect_vector(rotate(from_matrix.value, original).value, once.value);

    const Matrix3 reflection{-1, 0, 0, 0, 1, 0, 0, 0, 1};
    assert(from_rotation_matrix(reflection).error == RotationError::invalid_matrix);
    Matrix3 skew{1, 1e-5, 0, 0, 1, 0, 0, 0, 1};
    assert(from_rotation_matrix(skew).error == RotationError::invalid_matrix);
    Matrix3 non_finite{1, 0, 0, 0, 1, 0, 0, 0, std::numeric_limits<double>::infinity()};
    assert(from_rotation_matrix(non_finite).error == RotationError::non_finite_input);

    assert(from_axis_angle({1, 0, 0}, std::numeric_limits<double>::quiet_NaN()).error == RotationError::non_finite_input);
    assert(rotate({1, 0, 0, 0}, {0, std::numeric_limits<double>::infinity(), 0}).error == RotationError::non_finite_input);
    assert(normalize_rotation({1e-13, 0, 0, 0}).error == RotationError::zero_norm);
    expect_identity(to_rotation_matrix({0, 0, 0, 0}).value);
}
