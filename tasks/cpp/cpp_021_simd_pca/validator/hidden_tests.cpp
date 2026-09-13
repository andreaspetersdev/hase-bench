#include "pca.hpp"

#include <algorithm>
#include <cassert>
#include <cmath>
#include <cstdint>
#include <limits>
#include <span>
#include <vector>

namespace {
bool close(double actual, double expected, double abs_tol = 1e-9, double rel_tol = 1e-9) {
    return std::abs(actual - expected) <= abs_tol + rel_tol * std::abs(expected);
}

std::vector<double> trusted_covariance(std::span<const double> data,
                                       std::size_t rows, std::size_t n,
                                       std::size_t stride) {
    std::vector<long double> mean(n, 0);
    for (std::size_t i = 0; i < n; ++i)
        for (std::size_t r = 0; r < rows; ++r)
            mean[i] += static_cast<long double>(data[r * stride + i]) / rows;
    std::vector<double> result(n * n);
    for (std::size_t i = 0; i < n; ++i)
        for (std::size_t j = 0; j < n; ++j) {
            long double sum = 0;
            for (std::size_t r = 0; r < rows; ++r)
                sum += (static_cast<long double>(data[r * stride + i]) - mean[i]) *
                       (static_cast<long double>(data[r * stride + j]) - mean[j]);
            result[i * n + j] = static_cast<double>(sum / rows);
        }
    return result;
}

void check_model(const PcaModel& model, std::span<const double> data,
                 std::size_t rows, std::size_t n, std::size_t stride,
                 std::size_t k) {
    assert(model.error == PcaError::none);
    assert(model.features == n && model.components == k);
    assert(model.mean.size() == n && model.covariance.size() == n * n);
    assert(model.eigenvalues.size() == n && model.axes.size() == k * n);
    const auto expected = trusted_covariance(data, rows, n, stride);
    for (std::size_t i = 0; i < n; ++i) {
        long double mean = 0;
        for (std::size_t r = 0; r < rows; ++r)
            mean += static_cast<long double>(data[r * stride + i]) / rows;
        assert(close(model.mean[i], static_cast<double>(mean), 1e-6, 1e-12));
    }
    double scale = 0;
    for (double value : expected) scale = std::max(scale, std::abs(value));
    for (std::size_t i = 0; i < n * n; ++i)
        assert(close(model.covariance[i], expected[i], 1e-8 * scale + 1e-12, 1e-8));
    for (std::size_t i = 0; i < n; ++i) {
        assert(model.eigenvalues[i] >= -1e-10 * scale);
        if (i) assert(model.eigenvalues[i - 1] + 1e-9 * scale >= model.eigenvalues[i]);
    }
    for (std::size_t a = 0; a < k; ++a) {
        std::size_t pivot = 0;
        for (std::size_t i = 1; i < n; ++i)
            if (std::abs(model.axes[a * n + i]) > std::abs(model.axes[a * n + pivot])) pivot = i;
        assert(model.axes[a * n + pivot] >= 0);
        for (std::size_t b = 0; b <= a; ++b) {
            double dot = 0;
            for (std::size_t i = 0; i < n; ++i)
                dot += model.axes[a * n + i] * model.axes[b * n + i];
            assert(close(dot, a == b ? 1.0 : 0.0, 1e-8));
        }
        for (std::size_t i = 0; i < n; ++i) {
            double product = 0;
            for (std::size_t j = 0; j < n; ++j)
                product += expected[i * n + j] * model.axes[a * n + j];
            assert(close(product, model.eigenvalues[a] * model.axes[a * n + i],
                         1e-7 * scale + 1e-11, 1e-7));
        }
    }
}

void dimensions_and_values() {
    const std::vector<double> data{1, 2, 3, 4};
    const auto error = [&](std::size_t rows, std::size_t n, std::size_t stride,
                           std::size_t k, PcaError expected) {
        const auto model = fit_pca(data, rows, n, stride, k);
        assert(model.error == expected && model.mean.empty() && model.covariance.empty() &&
               model.eigenvalues.empty() && model.axes.empty());
    };
    error(0, 2, 2, 1, PcaError::empty_input);
    error(1, 0, 2, 1, PcaError::invalid_dimensions);
    error(1, 9, 9, 1, PcaError::invalid_dimensions);
    error(1, 2, 1, 1, PcaError::invalid_dimensions);
    error(1, 2, 2, 0, PcaError::invalid_dimensions);
    error(1, 2, 2, 3, PcaError::invalid_dimensions);
    error(3, 2, 2, 1, PcaError::invalid_dimensions);
    error(std::numeric_limits<std::size_t>::max(), 2, 2, 1, PcaError::invalid_dimensions);
    const std::vector<double> nonfinite{1, std::numeric_limits<double>::infinity(), 3, 4};
    const auto bad = fit_pca(nonfinite, 2, 2, 2, 1);
    assert(bad.error == PcaError::non_finite_input && bad.mean.empty());
    assert(fit_pca(nonfinite, 3, 2, 2, 1).error == PcaError::invalid_dimensions);
    const auto model = fit_pca(data, 2, 2, 2, 2, BackendRequest::scalar);
    assert(project_reconstruct(model, std::vector<double>{1}).error == PcaError::invalid_observation);
    assert(project_reconstruct(model, std::vector<double>{1, std::numeric_limits<double>::quiet_NaN()})
               .error == PcaError::invalid_observation);
}

void stride_and_dispatch() {
    const double nan = std::numeric_limits<double>::quiet_NaN();
    const std::vector<double> data{
        1, 2, 5, nan, nan, 2, 0, 8, nan, nan, -1, 3, 0, nan, nan,
        4, 1, 2, nan, nan, 0, -2, 7, nan, nan, 3, 5, 1
    };
    assert(backend_supported(Backend::scalar));
    const auto scalar = fit_pca(data, 6, 3, 5, 3, BackendRequest::scalar);
    assert(scalar.backend == Backend::scalar);
    check_model(scalar, data, 6, 3, 5, 3);
    const auto automatic = fit_pca(data, 6, 3, 5, 3);
    assert(automatic.backend == (backend_supported(Backend::avx2) ? Backend::avx2 : Backend::scalar));
    check_model(automatic, data, 6, 3, 5, 3);
    const auto forced = fit_pca(data, 6, 3, 5, 3, BackendRequest::avx2);
    if (backend_supported(Backend::avx2)) {
        assert(forced.backend == Backend::avx2);
        check_model(forced, data, 6, 3, 5, 3);
        for (std::size_t i = 0; i < scalar.eigenvalues.size(); ++i)
            assert(close(forced.eigenvalues[i], scalar.eigenvalues[i], 1e-8, 1e-8));
    } else {
        assert(forced.error == PcaError::unsupported_backend && forced.mean.empty());
    }
    const std::vector<double> observation{4, 1, 2};
    const auto projection = project_reconstruct(scalar, observation);
    assert(projection.error == PcaError::none && projection.scores.size() == 3);
    for (std::size_t i = 0; i < 3; ++i)
        assert(close(projection.reconstruction[i], observation[i], 1e-8));
    const auto reduced = fit_pca(data, 6, 3, 5, 1, BackendRequest::scalar);
    check_model(reduced, data, 6, 3, 5, 1);
    double squared_residual = 0;
    for (std::size_t r = 0; r < 6; ++r) {
        const auto projected = project_reconstruct(reduced,
            std::span<const double>(data.data() + r * 5, 3));
        assert(projected.error == PcaError::none);
        for (std::size_t i = 0; i < 3; ++i)
            squared_residual += std::pow(projected.reconstruction[i] - data[r * 5 + i], 2);
    }
    assert(close(squared_residual, 6 * (reduced.eigenvalues[1] + reduced.eigenvalues[2]), 1e-7, 1e-7));

    // Exercise the upper feature bound and both four-lane covariance halves.
    std::vector<double> wide(8 * 10, nan);
    for (std::size_t r = 0; r < 8; ++r)
        for (std::size_t f = 0; f < 8; ++f)
            wide[r * 10 + f] = static_cast<double>(((r + 1) * (f + 3) + f * f + 3 * r * r) % 31) - 15;
    for (const auto request : {BackendRequest::scalar, BackendRequest::automatic}) {
        const auto wide_model = fit_pca(wide, 8, 8, 10, 8, request);
        check_model(wide_model, wide, 8, 8, 10, 8);
        const auto full = project_reconstruct(wide_model, std::span<const double>(wide.data() + 20, 8));
        assert(full.error == PcaError::none);
        for (std::size_t f = 0; f < 8; ++f)
            assert(close(full.reconstruction[f], wide[20 + f], 1e-8));
    }
}

void degeneracy_and_scale() {
    const std::vector<double> circle{2, 0, -2, 0, 0, 2, 0, -2};
    const auto repeated = fit_pca(circle, 4, 2, 2, 2, BackendRequest::scalar);
    check_model(repeated, circle, 4, 2, 2, 2);
    assert(close(repeated.eigenvalues[0], 2) && close(repeated.eigenvalues[1], 2));
    const auto projected = project_reconstruct(repeated, std::vector<double>{1, -3});
    assert(close(projected.reconstruction[0], 1) && close(projected.reconstruction[1], -3));
    const std::vector<double> constant{7, -3, 0, 7, -3, 0, 7, -3, 0};
    const auto zero = fit_pca(constant, 3, 3, 3, 3, BackendRequest::scalar);
    check_model(zero, constant, 3, 3, 3, 3);
    for (double eigenvalue : zero.eigenvalues) assert(eigenvalue == 0);
    const std::vector<double> scaled{
        1e10 + 0.25, 0.001, 1e10 - 0.25, 0.003,
        1e10 + 0.50, -0.002, 1e10 - 0.50, 0.004,
        1e10 + 0.75, 0.002
    };
    const auto model = fit_pca(scaled, 5, 2, 2, 2, BackendRequest::scalar);
    check_model(model, scaled, 5, 2, 2, 2);
    assert(model.covariance[0] > 0.18 && model.covariance[0] < 0.23);
    if (backend_supported(Backend::avx2)) {
        const auto vectorized = fit_pca(scaled, 5, 2, 2, 2, BackendRequest::avx2);
        check_model(vectorized, scaled, 5, 2, 2, 2);
        for (std::size_t i = 0; i < 4; ++i)
            assert(close(vectorized.covariance[i], model.covariance[i], 1e-9, 1e-8));
    }
    const std::vector<double> single{42};
    const auto one = fit_pca(single, 1, 1, 1, 1, BackendRequest::scalar);
    check_model(one, single, 1, 1, 1, 1);
    assert(one.eigenvalues[0] == 0 && one.axes[0] == 1);

    // Uniform scaling must not turn a rank-one eigenspace into two equal axes.
    // The covariance entries are 2.5e-16, well within double's normal range.
    const std::vector<double> tiny{
        1e-8, 1e-8, -1e-8, -1e-8, 2e-8, 2e-8, -2e-8, -2e-8
    };
    for (const auto request : {BackendRequest::scalar, BackendRequest::automatic}) {
        const auto scaled = fit_pca(tiny, 4, 2, 2, 1, request);
        assert(scaled.error == PcaError::none);
        assert(close(scaled.covariance[1], 2.5e-16, 1e-22, 1e-8));
        assert(close(scaled.eigenvalues[0], 5e-16, 1e-22, 1e-8));
        assert(std::abs(scaled.eigenvalues[1]) < 1e-22);
        const auto projected = project_reconstruct(scaled, std::vector<double>{3e-8, 3e-8});
        assert(projected.error == PcaError::none);
        assert(close(projected.reconstruction[0], 3e-8, 1e-14, 1e-8));
        assert(close(projected.reconstruction[1], 3e-8, 1e-14, 1e-8));
    }
}
}

int main() {
    dimensions_and_values();
    stride_and_dispatch();
    degeneracy_and_scale();
}
