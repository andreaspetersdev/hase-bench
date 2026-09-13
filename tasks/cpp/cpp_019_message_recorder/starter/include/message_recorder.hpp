#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <string>
#include <vector>

struct Persisted {
    std::uint64_t sequence;
    std::uint64_t timestamp;
    std::string bytes;
};

class MessageRecorder {
public:
    using Sink = std::function<void(const Persisted&)>;

    explicit MessageRecorder(std::size_t capacity, Sink sink = {});
    MessageRecorder(const MessageRecorder&) = delete;
    MessageRecorder& operator=(const MessageRecorder&) = delete;
    MessageRecorder(MessageRecorder&&) = delete;
    MessageRecorder& operator=(MessageRecorder&&) = delete;
    ~MessageRecorder();

    bool submit(std::uint64_t timestamp, std::string bytes);
    void close();
    [[nodiscard]] bool closed() const;
    [[nodiscard]] bool failed() const;
    [[nodiscard]] std::vector<Persisted> snapshot() const;
    [[nodiscard]] std::vector<Persisted> undelivered() const;

private:
    struct State;
    std::unique_ptr<State> state_;
};
