#include "pca.hpp"

// The accelerated implementation belongs in this separately compiled file.
void pca_avx2_covariance(std::span<const double> data, std::size_t rows,
                         std::size_t features, std::size_t stride,
                         std::span<const double> mean, std::span<double> covariance) {
    (void)data; (void)rows; (void)features; (void)stride; (void)mean; (void)covariance;
}
