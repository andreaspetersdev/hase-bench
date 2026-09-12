#include "log_statistics.hpp"

#include <cassert>
#include <cmath>

namespace { bool close(double a, double b) { return std::abs(a - b) < 1e-10; } }

int main() {
    LogStatistics stats(100, 200, 3);
    stats.ingest("100 INFO 10.0 started service");
    stats.ingest("150 WARN 20 cache is warm");
    stats.ingest("200 ERROR 30. error detail");
    stats.ingest("201 INFO 99 too late");
    stats.ingest("bad input");
    const auto result = stats.summary();
    assert(result.received == 5 && result.accepted == 3);
    assert(result.malformed == 1 && result.outside_window == 1);
    assert(result.info == 1 && result.warn == 1 && result.error == 1);
    assert(close(result.mean_latency_ms, 20));
    assert(close(result.p50_latency_ms, 20));
    assert(close(result.p95_latency_ms, 30));
}
