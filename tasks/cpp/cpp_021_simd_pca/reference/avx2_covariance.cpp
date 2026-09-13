#include "pca.hpp"

#include <cstddef>
#if defined(_MSC_VER) && (defined(_M_X64) || defined(_M_IX86)) || defined(__AVX2__)
#include <immintrin.h>
#define PCA_AVX2_COMPILED 1
#endif

void pca_avx2_covariance(std::span<const double> data, std::size_t rows,
                         std::size_t n, std::size_t stride,
                         std::span<const double> mean, std::span<double> cov) {
    for (std::size_t i = 0; i < n; ++i) {
        for (std::size_t j = i; j < n; ++j) {
            double sum = 0;
            std::size_t r = 0;
#if defined(PCA_AVX2_COMPILED)
            __m256d acc = _mm256_setzero_pd();
            const __m256d mi = _mm256_set1_pd(mean[i]);
            const __m256d mj = _mm256_set1_pd(mean[j]);
            for (; r + 4 <= rows; r += 4) {
                const __m256d x = _mm256_set_pd(data[(r + 3) * stride + i],
                    data[(r + 2) * stride + i], data[(r + 1) * stride + i], data[r * stride + i]);
                const __m256d y = _mm256_set_pd(data[(r + 3) * stride + j],
                    data[(r + 2) * stride + j], data[(r + 1) * stride + j], data[r * stride + j]);
                acc = _mm256_add_pd(acc, _mm256_mul_pd(_mm256_sub_pd(x, mi), _mm256_sub_pd(y, mj)));
            }
            alignas(32) double lanes[4];
            _mm256_store_pd(lanes, acc);
            for (double lane : lanes) sum += lane;
#endif
            for (; r < rows; ++r)
                sum += (data[r * stride + i] - mean[i]) * (data[r * stride + j] - mean[j]);
            cov[i * n + j] = cov[j * n + i] = sum / static_cast<double>(rows);
        }
    }
}
