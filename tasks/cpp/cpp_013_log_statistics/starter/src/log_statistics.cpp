#include "log_statistics.hpp"

#include <stdexcept>

LogStatistics::LogStatistics(std::int64_t window_start, std::int64_t window_end, std::size_t percentile_capacity)
    : window_start_(window_start), window_end_(window_end), percentile_capacity_(percentile_capacity) {
    if (window_start > window_end || percentile_capacity == 0) throw std::invalid_argument("invalid log statistics configuration");
}

void LogStatistics::ingest(const char*) {
    ++summary_.received;
}

LogSummary LogStatistics::summary() const {
    return summary_;
}
