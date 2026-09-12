#pragma once

#include <cstddef>
#include <cstdint>
#include <deque>

struct LogSummary {
    std::size_t received{};
    std::size_t accepted{};
    std::size_t malformed{};
    std::size_t outside_window{};
    std::size_t info{};
    std::size_t warn{};
    std::size_t error{};
    double mean_latency_ms{};
    double p50_latency_ms{};
    double p95_latency_ms{};
};

class LogStatistics {
public:
    LogStatistics(std::int64_t window_start, std::int64_t window_end, std::size_t percentile_capacity);
    void ingest(const char* line);
    [[nodiscard]] LogSummary summary() const;

private:
    std::int64_t window_start_;
    std::int64_t window_end_;
    std::size_t percentile_capacity_;
    LogSummary summary_;
    double sum_{};
    double compensation_{};
    std::deque<double> percentile_sample_;
};
