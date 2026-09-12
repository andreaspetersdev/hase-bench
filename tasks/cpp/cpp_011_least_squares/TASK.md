# CPP-011 — QR least-squares solver

Complete the C++20 implementation declared in
`include/numerics/least_squares.hpp`. Do not change the public API, CMake
target names, or add external dependencies.

`solve_least_squares` receives an `rows` by `columns` real matrix in row-major
order and a length-`rows` right-hand-side vector. It must solve the
overdetermined least-squares problem:

```text
minimize ||A x - b||₂
```

For a valid, full-column-rank input, return `SolveError::none`, a
length-`columns` solution, and the Euclidean residual norm of that returned
solution. The solution and residual must both be finite. The solver must work
for rectangular matrices with `rows >= columns`, including exact and noisy
systems.

Use a numerically stable QR factorization (for example Householder QR, or
appropriately re-orthogonalized modified Gram-Schmidt). Do not solve by
explicitly forming or inverting the normal equations `(AᵀA)⁻¹Aᵀb`; that loses
too much accuracy for inputs in this benchmark.

Validation and failure behavior:

1. `rows == 0`, `columns == 0`, `rows < columns`, an incorrectly sized
   coefficient vector, or an incorrectly sized right-hand side is
   `dimension_mismatch`.
2. Any non-finite coefficient or right-hand-side value is `non_finite_input`.
   This check takes precedence over numerical rank detection whenever the
   dimensions are otherwise valid.
3. A finite, correctly shaped matrix that is not numerically full column rank
   is `rank_deficient`. Treat a column direction whose QR diagonal magnitude
   is at most `1e-12` times the largest original column 2-norm, including a
   later input column, as numerically rank deficient. This is a relative criterion: uniformly scaling a valid
   matrix must not change success into failure.
4. On every failure, `solution` is empty and `residual_norm` is zero.

The API owns no input memory and must not modify either input vector. This task
does not require weighted least squares, regularization, sparse matrices,
column pivoting, or an underdetermined minimum-norm solution.

Build and run the visible tests before finishing:

```powershell
cmake -S . -B build
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```
