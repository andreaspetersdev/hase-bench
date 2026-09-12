#pragma once

#include <array>

namespace geometry {

struct Vec3 {
    double x{};
    double y{};
    double z{};
};

struct Quaternion {
    double w{1.0};
    double x{};
    double y{};
    double z{};
};

using Matrix3 = std::array<double, 9>;

enum class RotationError { none, non_finite_input, zero_norm, invalid_matrix };

struct QuaternionResult { RotationError error{RotationError::none}; Quaternion value{}; };
struct VectorResult { RotationError error{RotationError::none}; Vec3 value{}; };
struct MatrixResult { RotationError error{RotationError::none}; Matrix3 value{1, 0, 0, 0, 1, 0, 0, 0, 1}; };

QuaternionResult normalize_rotation(Quaternion rotation);
QuaternionResult from_axis_angle(Vec3 axis, double radians);
QuaternionResult compose(Quaternion first, Quaternion second);
QuaternionResult inverse_rotation(Quaternion rotation);
VectorResult rotate(Quaternion rotation, Vec3 vector);
MatrixResult to_rotation_matrix(Quaternion rotation);
QuaternionResult from_rotation_matrix(const Matrix3& matrix);

}  // namespace geometry
