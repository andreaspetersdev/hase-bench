#include "message_recorder.hpp"

#include <algorithm>
#include <array>
#include <cassert>
#include <chrono>
#include <future>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

namespace {
void check_drain_on_close() {
    std::promise<void> writer_entered;
    std::promise<void> release_writer;
    auto released = release_writer.get_future().share();
    std::vector<Persisted> sink_items;
    MessageRecorder recorder(2, [&](const Persisted& item) {
        sink_items.push_back(item);
        if (item.sequence == 0) {
            writer_entered.set_value();
            released.wait();
        }
    });
    assert(recorder.submit(300, "zero"));
    assert(writer_entered.get_future().wait_for(std::chrono::seconds(5)) == std::future_status::ready);
    assert(recorder.submit(200, "one"));
    assert(recorder.submit(100, "two"));
    assert(!recorder.submit(0, "rejected"));
    std::thread closer([&] { recorder.close(); });
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (!recorder.closed() && std::chrono::steady_clock::now() < deadline) std::this_thread::yield();
    assert(recorder.closed());
    assert(!recorder.submit(0, "closing"));
    assert(recorder.snapshot().empty()); // The gated sink cannot hold the recorder mutex.
    release_writer.set_value();
    closer.join();
    recorder.close();
    assert(!recorder.submit(0, "closed"));
    assert(!recorder.failed() && recorder.undelivered().empty());
    const auto persisted = recorder.snapshot();
    assert(persisted.size() == 3 && sink_items.size() == 3);
    for (std::size_t i = 0; i < persisted.size(); ++i) assert(persisted[i].sequence == i);
    assert(persisted[0].timestamp == 300 && persisted[1].timestamp == 200 && persisted[2].timestamp == 100);
    assert(persisted[0].bytes == "zero" && persisted[1].bytes == "one" && persisted[2].bytes == "two");
    for (std::size_t i = 0; i < persisted.size(); ++i) {
        assert(sink_items[i].sequence == persisted[i].sequence);
        assert(sink_items[i].timestamp == persisted[i].timestamp);
        assert(sink_items[i].bytes == persisted[i].bytes);
    }
}

void check_failure_preserves_accepted() {
    std::promise<void> writer_entered;
    std::promise<void> release_writer;
    auto released = release_writer.get_future().share();
    MessageRecorder recorder(2, [&](const Persisted& item) {
        if (item.sequence == 0) {
            writer_entered.set_value();
            released.wait();
        }
        if (item.sequence == 1) throw std::runtime_error("sink failure");
    });
    assert(recorder.submit(900, "ok"));
    assert(writer_entered.get_future().wait_for(std::chrono::seconds(5)) == std::future_status::ready);
    assert(recorder.submit(901, "fails"));
    assert(recorder.submit(902, std::string("pending\0bytes", 13)));
    assert(!recorder.submit(0, "full"));
    release_writer.set_value();
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (!recorder.failed() && std::chrono::steady_clock::now() < deadline) std::this_thread::yield();
    assert(recorder.failed());
    assert(!recorder.submit(0, "after failure"));
    const auto persisted = recorder.snapshot();
    const auto undelivered = recorder.undelivered();
    assert(persisted.size() == 1 && persisted[0].sequence == 0 && persisted[0].timestamp == 900 && persisted[0].bytes == "ok");
    assert(undelivered.size() == 2);
    assert(undelivered[0].sequence == 1 && undelivered[0].timestamp == 901 && undelivered[0].bytes == "fails");
    assert(undelivered[1].sequence == 2 && undelivered[1].timestamp == 902 && undelivered[1].bytes == std::string("pending\0bytes", 13));
    recorder.close();
    assert(recorder.closed() && recorder.failed());
    assert(recorder.snapshot().size() == persisted.size());
    assert(recorder.undelivered().size() == undelivered.size());
}

void check_concurrent_submitters() {
    constexpr int producers = 8;
    constexpr int each = 8;
    std::promise<void> writer_entered;
    std::promise<void> release_writer;
    auto released = release_writer.get_future().share();
    MessageRecorder recorder(producers * each, [&](const Persisted& item) {
        if (item.sequence == 0) {
            writer_entered.set_value();
            released.wait();
        }
    });
    assert(recorder.submit(999, "seed"));
    assert(writer_entered.get_future().wait_for(std::chrono::seconds(5)) == std::future_status::ready);
    std::array<std::thread, producers> threads;
    for (int producer = 0; producer < producers; ++producer) {
        threads[producer] = std::thread([&, producer] {
            for (int index = 0; index < each; ++index)
                assert(recorder.submit(static_cast<std::uint64_t>(producer * each + index), std::to_string(producer) + ":" + std::to_string(index)));
        });
    }
    for (auto& thread : threads) thread.join();
    assert(!recorder.submit(0, "full"));
    release_writer.set_value();
    recorder.close();
    const auto persisted = recorder.snapshot();
    assert(persisted.size() == 1 + producers * each);
    for (std::size_t index = 0; index < persisted.size(); ++index) assert(persisted[index].sequence == index);
    assert(persisted[0].timestamp == 999);
    std::vector<std::string> payloads;
    for (std::size_t i = 1; i < persisted.size(); ++i) {
        const auto separator = persisted[i].bytes.find(':');
        assert(separator != std::string::npos);
        const int producer = std::stoi(persisted[i].bytes.substr(0, separator));
        const int index = std::stoi(persisted[i].bytes.substr(separator + 1));
        assert(persisted[i].timestamp == static_cast<std::uint64_t>(producer * each + index));
        payloads.push_back(persisted[i].bytes);
    }
    std::sort(payloads.begin(), payloads.end());
    std::vector<std::string> expected;
    for (int producer = 0; producer < producers; ++producer)
        for (int index = 0; index < each; ++index)
            expected.push_back(std::to_string(producer) + ":" + std::to_string(index));
    std::sort(expected.begin(), expected.end());
    assert(payloads == expected && recorder.undelivered().empty());
}
}

int main() {
    bool rejected_zero = false;
    try { MessageRecorder invalid(0); } catch (const std::invalid_argument&) { rejected_zero = true; }
    assert(rejected_zero);
    MessageRecorder default_sink(1);
    assert(default_sink.submit(42, "default"));
    default_sink.close();
    assert(default_sink.snapshot().size() == 1 && default_sink.undelivered().empty());
    check_drain_on_close();
    check_failure_preserves_accepted();
    check_concurrent_submitters();
}
