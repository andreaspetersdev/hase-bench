# CPP-021 — SIMD PCA / covariance kernel

Complete the C++20 component declared in `include/pca.hpp`. Preserve its public
API and CMake target names. Do not add external dependencies. The input is a
row-major array of `rows` observations, each with `features` active doubles;
consecutive rows start `stride` doubles apart. Padding is ignored and the input
must never be modified.

Valid dimensions are `rows >= 1`, `1 <= features <= 8`,
`1 <= components <= features`, `stride >= features`, and an input span large
enough to contain the last active element. Invalid dimensions return
`invalid_dimensions`, except `rows == 0` returns `empty_input` when the other
dimensions are valid. Dimension errors take precedence over input values.
Any non-finite active input is `non_finite_input`; non-finite padding is ignored.
On any failure, all result vectors are empty. A forced AVX2 request on an
unsupported CPU returns `unsupported_backend`. `backend_supported(avx2)` must
reflect runtime CPU and OS support. Automatic selection uses AVX2 when
supported, scalar otherwise. The scalar path must work without AVX2 hardware;
do not compile the entire project with AVX2 enabled.

For success, compute each feature mean and the **population** centered
covariance (`sum((x-mean)(y-mean))/rows`). Return all `features` eigenvalues in
descending order, with small roundoff-negative values clamped to zero, and the
leading `components` unit eigenvectors as row-major axes. Eigenvectors are
orthonormal. Fix each axis sign by making its largest-magnitude entry positive
(lowest index wins an exact tie). Repeated eigenvalues may have any
orthonormal basis within their shared eigenspace; tests will not demand one
arbitrary basis. Zero-variance features and a wholly constant dataset are
valid. `project_reconstruct` returns dot products against centered input and
the reconstruction from those components; it rejects a wrong-sized or
non-finite observation.

Use a deterministic symmetric eigensolver suitable for at most eight features
and a genuine AVX2 covariance accumulation path selected at runtime. Numerical
tests use scale-aware tolerances for means, covariance, eigenvalues,
orthogonality, and projection/reconstruction residuals; bitwise agreement
between backends is not required. No throughput threshold is imposed.

Build and run visible tests before finishing.
