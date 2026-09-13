#include "message_recorder.hpp"

#include <condition_variable>
#include <deque>
#include <mutex>
#include <optional>
#include <stdexcept>
#include <thread>
#include <utility>

struct MessageRecorder::State {
    explicit State(std::size_t limit, Sink writer) : capacity(limit), sink(std::move(writer)) {}
    std::mutex mutex;
    std::condition_variable ready;
    std::deque<Persisted> queue;
    std::vector<Persisted> persisted;
    std::optional<Persisted> failed_item;
    const std::size_t capacity;
    Sink sink;
    std::uint64_t next_sequence{};
    bool closing{};
    bool sink_failed{};
    std::thread worker;
};

MessageRecorder::MessageRecorder(std::size_t capacity, Sink sink) {
    if (capacity == 0) throw std::invalid_argument("capacity must be positive");
    state_ = std::make_unique<State>(capacity, std::move(sink));
    State* const state = state_.get();
    state->worker = std::thread([state] {
        for (;;) {
            Persisted item;
            {
                std::unique_lock lock(state->mutex);
                state->ready.wait(lock, [state] { return state->closing || !state->queue.empty(); });
                if (state->queue.empty()) return;
                item = std::move(state->queue.front());
                state->queue.pop_front();
            }
            try {
                if (state->sink) state->sink(item);
            } catch (...) {
                std::lock_guard lock(state->mutex);
                state->failed_item = std::move(item);
                state->sink_failed = true;
                return;
            }
            {
                std::lock_guard lock(state->mutex);
                state->persisted.push_back(std::move(item));
            }
        }
    });
}

MessageRecorder::~MessageRecorder() { close(); }

bool MessageRecorder::submit(std::uint64_t timestamp, std::string bytes) {
    std::lock_guard lock(state_->mutex);
    if (state_->closing || state_->sink_failed || state_->queue.size() >= state_->capacity) return false;
    state_->queue.push_back({state_->next_sequence, timestamp, std::move(bytes)});
    ++state_->next_sequence;
    state_->ready.notify_one();
    return true;
}

void MessageRecorder::close() {
    {
        std::lock_guard lock(state_->mutex);
        state_->closing = true;
    }
    state_->ready.notify_one();
    if (state_->worker.joinable()) state_->worker.join();
}

bool MessageRecorder::failed() const {
    std::lock_guard lock(state_->mutex);
    return state_->sink_failed;
}

bool MessageRecorder::closed() const {
    std::lock_guard lock(state_->mutex);
    return state_->closing;
}

std::vector<Persisted> MessageRecorder::snapshot() const {
    std::lock_guard lock(state_->mutex);
    return state_->persisted;
}

std::vector<Persisted> MessageRecorder::undelivered() const {
    std::lock_guard lock(state_->mutex);
    std::vector<Persisted> result;
    result.reserve(state_->queue.size() + (state_->failed_item ? 1 : 0));
    if (state_->failed_item) result.push_back(*state_->failed_item);
    result.insert(result.end(), state_->queue.begin(), state_->queue.end());
    return result;
}
