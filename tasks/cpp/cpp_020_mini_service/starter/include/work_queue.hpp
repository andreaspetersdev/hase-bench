#pragma once

#include "event.hpp"
#include "stats.hpp"

#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <deque>
#include <functional>
#include <mutex>
#include <optional>
#include <thread>
#include <vector>

struct WorkItem {
    Event event;
    std::uint32_t threshold{};
};

class WorkQueue {
public:
    using Sink = std::function<void(const Event&)>;
    WorkQueue(std::size_t capacity, Sink sink, Statistics& statistics);
    WorkQueue(const WorkQueue&) = delete;
    WorkQueue& operator=(const WorkQueue&) = delete;
    ~WorkQueue();

    [[nodiscard]] bool submit_batch(std::vector<WorkItem> items);
    void close();
    [[nodiscard]] bool failed() const;
    [[nodiscard]] std::vector<Event> undelivered() const;

private:
    void run();
    std::size_t capacity_;
    Sink sink_;
    Statistics& statistics_;
    mutable std::mutex mutex_;
    std::condition_variable ready_;
    std::deque<WorkItem> pending_;
    std::optional<WorkItem> failed_item_;
    bool closing_{};
    bool failed_{};
    std::thread worker_;
};
