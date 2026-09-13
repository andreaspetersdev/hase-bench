#include "pca.hpp"

#include <cmath>

bool backend_supported(Backend backend) noexcept { return backend == Backend::scalar; }

PcaModel fit_pca(std::span<const double> data, std::size_t rows,
                 std::size_t features, std::size_t stride,
                 std::size_t components, BackendRequest request) {
    PcaModel result;
    if (features == 0 || features > 8 || components == 0 || components > features ||
        stride < features || (rows && (data.size() < features ||
        (rows - 1) > (data.size() - features) / stride))) {
        result.error = PcaError::invalid_dimensions;
        return result;
    }
    if (rows == 0) { result.error = PcaError::empty_input; return result; }
    for (std::size_t r = 0; r < rows; ++r)
        for (std::size_t f = 0; f < features; ++f)
            if (!std::isfinite(data[r * stride + f])) {
                result.error = PcaError::non_finite_input; return result;
            }
    if (request == BackendRequest::avx2) {
        result.error = PcaError::unsupported_backend; return result;
    }
    result.features = features;
    result.components = components;
    result.mean.assign(features, 0.0);
    result.covariance.assign(features * features, 0.0);
    result.eigenvalues.assign(features, 0.0);
    result.axes.assign(components * features, 0.0);
    // Deliberately incomplete starter: no centering, covariance, eigensolve, or projection.
    return result;
}

Projection project_reconstruct(const PcaModel& model,
                               std::span<const double> observation) {
    Projection result;
    if (model.error != PcaError::none || observation.size() != model.features) {
        result.error = PcaError::invalid_observation; return result;
    }
    result.scores.assign(model.components, 0.0);
    result.reconstruction = model.mean;
    return result;
}
