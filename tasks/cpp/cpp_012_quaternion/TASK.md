# CPP-012 — Quaternion rotation utilities

Complete the C++20 implementation declared in
`include/geometry/quaternion.hpp`. Do not change the public API, CMake target
names, or add external dependencies.

This project provides finite, right-handed three-dimensional rotations. A
`Quaternion` is ordered `(w, x, y, z)`. A `Matrix3` is row-major. Implement
all declared functions and preserve the following contract.

## Successful rotations

- `normalize_rotation` returns a unit quaternion representing the same
  rotation. `from_axis_angle` normalizes its axis. `compose(first, second)`
  means apply `first` and then `second`, so its rotation is `second * first`.
- `inverse_rotation` returns the inverse of the represented rotation.
  `rotate` first normalizes its input quaternion, then applies
  `q * (0, vector) * conjugate(q)`.
- `to_rotation_matrix` first normalizes its quaternion and produces the
  corresponding proper row-major rotation matrix. `from_rotation_matrix`
  accepts only a proper rotation matrix and returns its rotation.
- All successful quaternion-returning functions must return a *canonical unit
  quaternion*: its norm is one; `w` is non-negative; if `w` is exactly zero,
  the first non-zero component among `x`, `y`, `z` is positive. This resolves
  the ordinary `q`/`-q` sign ambiguity. Inputs themselves need not already be
  canonical or unit length.

## Rejection and numerical policy

- A scalar/vector/quaternion/matrix input containing a NaN or infinity is
  `RotationError::non_finite_input`.
- A quaternion or axis whose Euclidean norm is at most `1e-12` is
  `RotationError::zero_norm`. On error, quaternion results contain identity,
  vector results contain `{0,0,0}`, and matrix results contain identity.
- A matrix is `invalid_matrix` unless every pair of rows has dot product zero
  (different rows) or one (the same row) within `1e-10`, and its determinant
  is within `1e-10` of positive one. Do not silently orthogonalize, reflect, or
  otherwise repair matrices. Matrix validation is performed after the
  non-finite check.
- The angle is measured in radians. Large finite angles are valid. For the
  purposes of canonicalization, a component whose absolute value is at most
  `1e-15` may be treated as zero.

The public structs own no external memory. Use ordinary floating-point
tolerances in tests; do not compare rotations only by their four raw input
components.

Build and run the visible tests before finishing:

```powershell
cmake -S . -B build
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```
