#include "geometry/quaternion.hpp"

namespace geometry {

QuaternionResult normalize_rotation(Quaternion) { return {RotationError::zero_norm, {}}; }
QuaternionResult from_axis_angle(Vec3, double) { return {RotationError::zero_norm, {}}; }
QuaternionResult compose(Quaternion, Quaternion) { return {RotationError::zero_norm, {}}; }
QuaternionResult inverse_rotation(Quaternion) { return {RotationError::zero_norm, {}}; }
VectorResult rotate(Quaternion, Vec3) { return {RotationError::zero_norm, {}}; }
MatrixResult to_rotation_matrix(Quaternion) { return {RotationError::zero_norm, {}}; }
QuaternionResult from_rotation_matrix(const Matrix3&) { return {RotationError::invalid_matrix, {}}; }

}  // namespace geometry
