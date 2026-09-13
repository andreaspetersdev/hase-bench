#include "service.hpp"

#include <cassert>
#include <string>
#include <vector>

int main() {
    const auto config = Config::parse(
        "capacity=3\nmax_line_bytes=64\ndefault_threshold=10\nthreshold.alpha=7\n");
    assert(config && config->capacity() == 3 && config->max_line_bytes() == 64);
    assert(config->threshold_for("alpha") == 7);
    assert(config->threshold_for("beta") == 10);

    std::vector<Event> delivered;
    Service service(*config, [&](const Event& event) { delivered.push_back(event); });
    assert(service.ingest("1,5\n2,alpha,"));
    assert(service.ingest("7\r\n3,beta,10\n"));
    service.close();
    service.close();
    assert(!service.failed() && service.undelivered().empty());
    assert(!service.ingest("4,2\n"));
    assert(delivered.size() == 3);
    assert(delivered[0].source == "default" && delivered[0].value == 5);
    assert(delivered[1].source == "alpha" && delivered[1].timestamp == 2);
    assert(delivered[2].source == "beta" && delivered[2].value == 10);
    const auto stats = service.snapshot();
    assert(stats.count == 3 && stats.sum == 22 && stats.alerts == 2);
    assert(stats.sources.size() == 3);
    assert(stats.sources[0].source == "alpha" && stats.sources[0].alerts == 1);
    assert(stats.sources[1].source == "beta" && stats.sources[1].alerts == 1);
    assert(stats.sources[2].source == "default" && stats.sources[2].alerts == 0);
}
