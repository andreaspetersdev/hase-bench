#pragma once

#include <cstddef>
#include <span>
#include <vector>

enum class PcaError {
    none,
    invalid_dimensions,
    empty_input,
    non_finite_input,
    unsupported_backend,
    invalid_observation
};

enum class Backend { scalar, avx2 };
enum class BackendRequest { automatic, scalar, avx2 };

struct PcaModel {
    PcaError error{PcaError::none};
    Backend backend{Backend::scalar};
    std::size_t features{};
    std::size_t components{};
    std::vector<double> mean;
    std::vector<double> covariance; // row-major features x features
    std::vector<double> eigenvalues; // all features, descending
    std::vector<double> axes;        // row-major components x features
};

struct Projection {
    PcaError error{PcaError::none};
    std::vector<double> scores;
    std::vector<double> reconstruction;
};

[[nodiscard]] bool backend_supported(Backend backend) noexcept;
[[nodiscard]] PcaModel fit_pca(std::span<const double> data,
                               std::size_t rows, std::size_t features,
                               std::size_t stride, std::size_t components,
                               BackendRequest request = BackendRequest::automatic);
[[nodiscard]] Projection project_reconstruct(const PcaModel& model,
                                              std::span<const double> observation);
