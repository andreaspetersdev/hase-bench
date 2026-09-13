#include "work_queue.hpp"

#include <utility>

WorkQueue::WorkQueue(std::size_t capacity, Sink sink, Statistics& statistics)
    : capacity_(capacity), sink_(std::move(sink)), statistics_(statistics),
      worker_([this] { run(); }) {}

WorkQueue::~WorkQueue() { close(); }

bool WorkQueue::submit_batch(std::vector<WorkItem> items) {
    std::lock_guard lock(mutex_);
    if (closing_ || failed_) return false;
    if (items.size() > capacity_ - pending_.size()) return false;
    for (auto& item : items) {
        pending_.push_back(std::move(item));
    }
    if (!items.empty()) ready_.notify_one();
    return true;
}

void WorkQueue::close() {
    {
        std::lock_guard lock(mutex_);
        closing_ = true;
    }
    ready_.notify_one();
    if (worker_.joinable()) worker_.join();
}

bool WorkQueue::failed() const {
    std::lock_guard lock(mutex_);
    return failed_;
}

std::vector<Event> WorkQueue::undelivered() const {
    std::lock_guard lock(mutex_);
    std::vector<Event> result;
    if (failed_item_) result.push_back(failed_item_->event);
    for (const auto& item : pending_) result.push_back(item.event);
    return result;
}

void WorkQueue::run() {
    for (;;) {
        WorkItem item;
        {
            std::unique_lock lock(mutex_);
            ready_.wait(lock, [this] { return closing_ || !pending_.empty(); });
            if (pending_.empty()) return;
            item = std::move(pending_.front());
            pending_.pop_front();
        }
        try {
            if (sink_) sink_(item.event);
        } catch (...) {
            std::lock_guard lock(mutex_);
            failed_item_ = std::move(item);
            failed_ = true;
            return;
        }
        statistics_.record(item.event, item.threshold);
    }
}
