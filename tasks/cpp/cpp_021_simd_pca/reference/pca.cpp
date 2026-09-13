#include "pca.hpp"

#include <algorithm>
#include <cmath>
#include <numeric>
#include <utility>
#if defined(_MSC_VER) && (defined(_M_X64) || defined(_M_IX86))
#include <intrin.h>
#endif

void pca_avx2_covariance(std::span<const double>, std::size_t, std::size_t,
                         std::size_t, std::span<const double>, std::span<double>);

namespace {
bool has_avx2() noexcept {
#if defined(_MSC_VER) && (defined(_M_X64) || defined(_M_IX86))
    int regs[4]{};
    __cpuid(regs, 1);
    if ((regs[2] & (1 << 27)) == 0 || (regs[2] & (1 << 28)) == 0) return false;
    if ((_xgetbv(0) & 0x6) != 0x6) return false;
    __cpuidex(regs, 7, 0);
    return (regs[1] & (1 << 5)) != 0;
#elif defined(__x86_64__) || defined(__i386__)
    __builtin_cpu_init();
    return __builtin_cpu_supports("avx2");
#else
    return false;
#endif
}

void scalar_covariance(std::span<const double> data, std::size_t rows,
                       std::size_t n, std::size_t stride,
                       std::span<const double> mean, std::span<double> cov) {
    for (std::size_t i = 0; i < n; ++i) {
        for (std::size_t j = i; j < n; ++j) {
            long double sum = 0;
            for (std::size_t r = 0; r < rows; ++r) {
                const long double x = static_cast<long double>(data[r * stride + i]) - mean[i];
                const long double y = static_cast<long double>(data[r * stride + j]) - mean[j];
                sum += x * y;
            }
            cov[i * n + j] = cov[j * n + i] = static_cast<double>(sum / rows);
        }
    }
}

void eigen_symmetric(std::span<const double> covariance, std::size_t n,
                     std::size_t k, std::vector<double>& eigenvalues,
                     std::vector<double>& axes) {
    std::vector<double> a(covariance.begin(), covariance.end());
    std::vector<double> v(n * n, 0.0);
    for (std::size_t i = 0; i < n; ++i) v[i * n + i] = 1.0;
    for (std::size_t iteration = 0; iteration < 80 * n * n; ++iteration) {
        std::size_t p = 0, q = 0;
        double maximum = 0, scale = 0;
        for (std::size_t i = 0; i < n; ++i) {
            scale = std::max(scale, std::abs(a[i * n + i]));
            for (std::size_t j = i + 1; j < n; ++j) {
                if (std::abs(a[i * n + j]) > maximum) {
                    maximum = std::abs(a[i * n + j]); p = i; q = j;
                }
            }
        }
        if (maximum == 0 || maximum <= 1e-15 * scale) break;
        const double apq = a[p * n + q];
        const double tau = (a[q * n + q] - a[p * n + p]) / (2.0 * apq);
        const double t = std::copysign(1.0, tau) / (std::abs(tau) + std::hypot(1.0, tau));
        const double c = 1.0 / std::sqrt(1.0 + t * t);
        const double s = t * c;
        const double app = a[p * n + p], aqq = a[q * n + q];
        a[p * n + p] = app - t * apq;
        a[q * n + q] = aqq + t * apq;
        a[p * n + q] = a[q * n + p] = 0;
        for (std::size_t i = 0; i < n; ++i) {
            if (i != p && i != q) {
                const double aip = a[i * n + p], aiq = a[i * n + q];
                a[i * n + p] = a[p * n + i] = c * aip - s * aiq;
                a[i * n + q] = a[q * n + i] = s * aip + c * aiq;
            }
            const double vip = v[i * n + p], viq = v[i * n + q];
            v[i * n + p] = c * vip - s * viq;
            v[i * n + q] = s * vip + c * viq;
        }
    }
    std::vector<std::size_t> order(n);
    std::iota(order.begin(), order.end(), 0);
    std::stable_sort(order.begin(), order.end(), [&](std::size_t x, std::size_t y) {
        return a[x * n + x] > a[y * n + y];
    });
    const double scale = std::abs(a[order.front() * n + order.front()]);
    for (std::size_t i = 0; i < n; ++i) {
        double lambda = a[order[i] * n + order[i]];
        if (lambda < 0 && lambda >= -1e-12 * scale) lambda = 0;
        eigenvalues[i] = lambda;
    }
    for (std::size_t axis = 0; axis < k; ++axis) {
        const std::size_t col = order[axis];
        std::size_t pivot = 0;
        for (std::size_t i = 1; i < n; ++i)
            if (std::abs(v[i * n + col]) > std::abs(v[pivot * n + col])) pivot = i;
        const double sign = v[pivot * n + col] < 0 ? -1.0 : 1.0;
        for (std::size_t i = 0; i < n; ++i) axes[axis * n + i] = sign * v[i * n + col];
    }
}
}

bool backend_supported(Backend backend) noexcept {
    return backend == Backend::scalar || has_avx2();
}

PcaModel fit_pca(std::span<const double> data, std::size_t rows,
                 std::size_t features, std::size_t stride,
                 std::size_t components, BackendRequest request) {
    PcaModel result;
    if (features == 0 || features > 8 || components == 0 || components > features ||
        stride < features || (rows && (data.size() < features ||
        (rows - 1) > (data.size() - features) / stride))) {
        result.error = PcaError::invalid_dimensions; return result;
    }
    if (rows == 0) { result.error = PcaError::empty_input; return result; }
    for (std::size_t r = 0; r < rows; ++r)
        for (std::size_t i = 0; i < features; ++i)
            if (!std::isfinite(data[r * stride + i])) {
                result.error = PcaError::non_finite_input; return result;
            }
    if (request == BackendRequest::avx2 && !has_avx2()) {
        result.error = PcaError::unsupported_backend; return result;
    }
    result.backend = request == BackendRequest::scalar ? Backend::scalar :
                     (has_avx2() ? Backend::avx2 : Backend::scalar);
    result.features = features;
    result.components = components;
    result.mean.resize(features);
    result.covariance.resize(features * features);
    result.eigenvalues.resize(features);
    result.axes.resize(components * features);
    for (std::size_t i = 0; i < features; ++i) {
        long double mean = 0;
        for (std::size_t r = 0; r < rows; ++r)
            mean += (static_cast<long double>(data[r * stride + i]) - mean) / (r + 1);
        result.mean[i] = static_cast<double>(mean);
    }
    if (result.backend == Backend::avx2)
        pca_avx2_covariance(data, rows, features, stride, result.mean, result.covariance);
    else
        scalar_covariance(data, rows, features, stride, result.mean, result.covariance);
    eigen_symmetric(result.covariance, features, components, result.eigenvalues, result.axes);
    return result;
}

Projection project_reconstruct(const PcaModel& model,
                               std::span<const double> observation) {
    Projection result;
    if (model.error != PcaError::none || model.features == 0 ||
        observation.size() != model.features || model.mean.size() != model.features ||
        model.axes.size() != model.components * model.features) {
        result.error = PcaError::invalid_observation; return result;
    }
    for (double value : observation)
        if (!std::isfinite(value)) { result.error = PcaError::invalid_observation; return result; }
    result.scores.assign(model.components, 0.0);
    result.reconstruction = model.mean;
    for (std::size_t c = 0; c < model.components; ++c) {
        double score = 0;
        for (std::size_t i = 0; i < model.features; ++i)
            score += (observation[i] - model.mean[i]) * model.axes[c * model.features + i];
        result.scores[c] = score;
        for (std::size_t i = 0; i < model.features; ++i)
            result.reconstruction[i] += score * model.axes[c * model.features + i];
    }
    return result;
}
