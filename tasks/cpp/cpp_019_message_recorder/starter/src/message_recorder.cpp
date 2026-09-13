#include "message_recorder.hpp"

#include <condition_variable>
#include <deque>
#include <mutex>
#include <stdexcept>
#include <thread>

struct MessageRecorder::State {
    explicit State(std::size_t limit, Sink writer) : capacity(limit), sink(std::move(writer)) {}
    std::mutex mutex;
    std::condition_variable ready;
    std::deque<Persisted> queue;
    std::vector<Persisted> persisted;
    const std::size_t capacity;
    Sink sink;
    std::uint64_t next_sequence{};
    bool closing{};
    bool sink_failed{};
    std::thread worker;
};

MessageRecorder::MessageRecorder(std::size_t capacity, Sink sink)
    : state_(std::make_unique<State>(capacity, std::move(sink))) {
    if (capacity == 0) throw std::invalid_argument("capacity must be positive");
    state_->worker = std::thread([this] {
        for (;;) {
            std::unique_lock lock(state_->mutex);
            state_->ready.wait(lock, [this] { return state_->closing || !state_->queue.empty(); });
            if (state_->closing) return; // BUG: pending accepted work is abandoned.
            Persisted item = std::move(state_->queue.front());
            state_->queue.pop_front();
            lock.unlock();
            if (state_->sink) state_->sink(item); // BUG: an exception terminates the process.
            lock.lock();
            state_->persisted.push_back(std::move(item));
        }
    });
}

MessageRecorder::~MessageRecorder() { close(); }

bool MessageRecorder::submit(std::uint64_t timestamp, std::string bytes) {
    std::lock_guard lock(state_->mutex);
    if (state_->closing || state_->sink_failed || state_->queue.size() >= state_->capacity) return false;
    state_->queue.push_back({state_->next_sequence++, timestamp, std::move(bytes)});
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

std::vector<Persisted> MessageRecorder::undelivered() const { return {}; }
