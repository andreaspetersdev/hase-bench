#include "service.hpp"

#include <cassert>
#include <chrono>
#include <future>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
Config config(std::size_t capacity) {
    auto parsed = Config::parse(
        "capacity=" + std::to_string(capacity) +
        "\nmax_line_bytes=64\ndefault_threshold=10\nthreshold.alpha=7\n");
    assert(parsed);
    return *parsed;
}

void check_rollback_and_legacy() {
    Service service(config(3));
    assert(!service.ingest("1,alpha,7\nbad\n"));
    assert(service.ingest("1,alpha,7\n"));
    assert(service.ingest("2,5\r\n3,beta,10\n"));
    service.close();
    const auto stats = service.snapshot();
    assert(stats.count == 3 && stats.sum == 22 && stats.alerts == 2);
    assert(stats.sources.size() == 3);
    assert(stats.sources[0].source == "alpha" && stats.sources[0].count == 1);
    assert(stats.sources[1].source == "beta" && stats.sources[1].count == 1);
    assert(stats.sources[2].source == "default" && stats.sources[2].count == 1);
    assert(!service.failed() && service.undelivered().empty());
}

void check_batch_capacity_and_drain() {
    std::promise<void> entered;
    std::promise<void> release;
    const auto released = release.get_future().share();
    std::vector<std::uint64_t> sink_order;
    Service service(config(2), [&](const Event& event) {
        sink_order.push_back(event.timestamp);
        if (event.timestamp == 1) {
            entered.set_value();
            released.wait();
        }
    });
    assert(service.ingest("1,1\n"));
    assert(entered.get_future().wait_for(std::chrono::seconds(5)) == std::future_status::ready);
    assert(service.ingest("2,2\n"));
    assert(!service.ingest("3,3\n4,4\n"));
    assert(service.snapshot().count == 0);
    release.set_value();
    service.close();
    service.close();
    assert(sink_order == std::vector<std::uint64_t>({1, 2}));
    assert(service.snapshot().count == 2 && service.snapshot().sum == 3);
    assert(!service.ingest("5,5\n") && service.undelivered().empty());
}

void check_partial_retry_after_full_queue() {
    std::promise<void> first_entered, release_first, second_entered, release_second;
    const auto first_released = release_first.get_future().share();
    const auto second_released = release_second.get_future().share();
    std::vector<std::uint64_t> order;
    Service service(config(1), [&](const Event& event) {
        order.push_back(event.timestamp);
        if (event.timestamp == 1) {
            first_entered.set_value();
            first_released.wait();
        }
        if (event.timestamp == 2) {
            second_entered.set_value();
            second_released.wait();
        }
    });
    assert(service.ingest("1,1\n"));
    assert(first_entered.get_future().wait_for(std::chrono::seconds(5)) == std::future_status::ready);
    assert(service.ingest("2,2\n"));
    assert(service.ingest("3,"));
    assert(!service.ingest("3\n"));
    release_first.set_value();
    assert(second_entered.get_future().wait_for(std::chrono::seconds(5)) == std::future_status::ready);
    assert(service.ingest("3\n"));
    release_second.set_value();
    service.close();
    assert(order == std::vector<std::uint64_t>({1, 2, 3}));
    assert(service.snapshot().count == 3 && service.snapshot().sum == 6);
}

void check_sink_failure() {
    Service service(config(3), [](const Event& event) {
        if (event.timestamp == 2) throw std::runtime_error("sink failed");
    });
    assert(service.ingest("1,alpha,7\n2,alpha,8\n3,beta,9\n"));
    service.close();
    assert(service.failed() && !service.ingest("4,1\n"));
    const auto stats = service.snapshot();
    assert(stats.count == 1 && stats.sum == 7 && stats.alerts == 1);
    const auto pending = service.undelivered();
    assert(pending.size() == 2);
    assert(pending[0].timestamp == 2 && pending[0].source == "alpha" && pending[0].value == 8);
    assert(pending[1].timestamp == 3 && pending[1].source == "beta" && pending[1].value == 9);
    service.close();
    assert(service.undelivered().size() == 2);
}
}

int main() {
    check_rollback_and_legacy();
    check_batch_capacity_and_drain();
    check_partial_retry_after_full_queue();
    check_sink_failure();
}
