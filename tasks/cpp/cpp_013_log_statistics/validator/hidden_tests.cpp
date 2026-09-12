#include "log_statistics.hpp"

#include <cassert>
#include <cmath>
#include <stdexcept>

namespace {
bool close(double actual, double expected, double tolerance = 1e-9) {
    return std::abs(actual - expected) <= tolerance;
}
}

int main() {
    bool threw = false;
    try { LogStatistics invalid(5, 4, 1); } catch (const std::invalid_argument&) { threw = true; }
    assert(threw);
    threw = false;
    try { LogStatistics invalid(4, 5, 0); } catch (const std::invalid_argument&) { threw = true; }
    assert(threw);

    LogStatistics syntax(10, 20, 2);
    syntax.ingest("10 INFO 1. message with spaces\r");
    syntax.ingest("20 WARN 12. valid trailing decimal");
    syntax.ingest("9 ERROR 3. outside before");
    syntax.ingest("21 ERROR 3. outside after");
    syntax.ingest("10 DEBUG 3. unknown level");
    syntax.ingest("10 INFO .5 missing integer digits");
    syntax.ingest("10 INFO +5 sign is forbidden");
    syntax.ingest("10 INFO 1e2 exponent is forbidden");
    syntax.ingest("10 INFO 3 no-newline\ninside");
    syntax.ingest("9223372036854775808 INFO 3. timestamp overflow");
    syntax.ingest(" 10 INFO 3. leading whitespace");
    const auto parsed = syntax.summary();
    assert(parsed.received == 11 && parsed.accepted == 2);
    assert(parsed.outside_window == 2 && parsed.malformed == 7);
    assert(parsed.info == 1 && parsed.warn == 1 && parsed.error == 0);
    assert(close(parsed.mean_latency_ms, 6.5));
    assert(close(parsed.p50_latency_ms, 1) && close(parsed.p95_latency_ms, 12));

    LogStatistics rolling(0, 100, 3);
    rolling.ingest("1 INFO 100 first");
    rolling.ingest("200 INFO 200 outside does not advance sample");
    rolling.ingest("2 WARN 1 second");
    rolling.ingest("3 ERROR 5 third");
    rolling.ingest("4 INFO 9 fourth");
    const auto rolled = rolling.summary();
    assert(rolled.accepted == 4 && rolled.outside_window == 1);
    assert(close(rolled.mean_latency_ms, 28.75));
    // The retained FIFO is [1, 5, 9], not the first three lines or all four.
    assert(close(rolled.p50_latency_ms, 5) && close(rolled.p95_latency_ms, 9));

    LogStatistics stable(0, 1, 1);
    stable.ingest("0 INFO 10000000000000000. large");
    for (int index = 0; index != 10000; ++index) stable.ingest("0 INFO 1. small");
    const long double expected = (10000000000000000.0L + 10000.0L) / 10001.0L;
    assert(close(stable.summary().mean_latency_ms, static_cast<double>(expected), 0.1));

    // A valid long fractional spelling must be rounded as a complete decimal,
    // rather than truncated after an implementation-chosen number of digits.
    LogStatistics precision(0, 0, 1);
    precision.ingest("0 INFO 1.00000000000000015 precision");
    assert(precision.summary().accepted == 1);
    assert(precision.summary().mean_latency_ms > 1.0);

    LogStatistics empty(0, 1, 1);
    const auto none = empty.summary();
    assert(none.received == 0 && none.accepted == 0 && none.mean_latency_ms == 0.0);
    assert(none.p50_latency_ms == 0.0 && none.p95_latency_ms == 0.0);
}
