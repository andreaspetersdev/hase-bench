#include "pca.hpp"

#include <cassert>
#include <cmath>
#include <vector>

int main() {
    const std::vector<double> observations{1, 2, 3, 4, 5, 6}; // three rows, two features
    const auto model = fit_pca(observations, 3, 2, 2, 1, BackendRequest::scalar);
    assert(model.error == PcaError::none && model.backend == Backend::scalar);
    assert(model.mean.size() == 2 && std::abs(model.mean[0] - 3) < 1e-12);
    assert(std::abs(model.mean[1] - 4) < 1e-12);
    assert(model.covariance.size() == 4);
    for (double value : model.covariance) assert(std::abs(value - 8.0 / 3.0) < 1e-12);
    assert(model.eigenvalues.size() == 2);
    assert(std::abs(model.eigenvalues[0] - 16.0 / 3.0) < 1e-11);
    assert(std::abs(model.eigenvalues[1]) < 1e-11);
    assert(model.axes.size() == 2 && model.axes[0] > 0);
    const auto projection = project_reconstruct(model, std::vector<double>{5, 6});
    assert(projection.error == PcaError::none && projection.scores.size() == 1);
    assert(std::abs(projection.reconstruction[0] - 5) < 1e-11);
    assert(std::abs(projection.reconstruction[1] - 6) < 1e-11);
}
