#include "spsc_ring_buffer.hpp"
#include <array>
#include <atomic>
#include <cstdint>
#include <iostream>
#include <thread>
#include <utility>
namespace {
struct LifetimeProbe {
  static inline int live = 0;
  explicit LifetimeProbe(int value) : value(value) { ++live; }
  LifetimeProbe(const LifetimeProbe&) = delete;
  LifetimeProbe& operator=(const LifetimeProbe&) = delete;
  LifetimeProbe(LifetimeProbe&& other) noexcept : value(other.value) { ++live; }
  LifetimeProbe& operator=(LifetimeProbe&& other) noexcept { value = other.value; return *this; }
  ~LifetimeProbe() { --live; }
  int value;
};

struct Message {
  explicit Message(std::uint64_t sequence) : sequence(sequence), seal(sequence ^ kSeal) {
    for (std::size_t i = 0; i < words.size(); ++i) {
      words[i] = sequence * 0x9e3779b97f4a7c15ULL + i * 0xbf58476d1ce4e5b9ULL;
    }
  }
  Message(const Message&) = delete;
  Message& operator=(const Message&) = delete;
  Message(Message&&) noexcept = default;
  Message& operator=(Message&&) noexcept = default;

  [[nodiscard]] bool is_valid(std::uint64_t expected) const {
    if (sequence != expected || seal != (expected ^ kSeal)) return false;
    for (std::size_t i = 0; i < words.size(); ++i) {
      if (words[i] != expected * 0x9e3779b97f4a7c15ULL + i * 0xbf58476d1ce4e5b9ULL) return false;
    }
    return true;
  }

  static constexpr std::uint64_t kSeal = 0xd6e8feb86659fd93ULL;
  std::uint64_t sequence;
  std::array<std::uint64_t, 32> words{};
  std::uint64_t seal;
};
}
int main() {
  {
    hase::SpscRingBuffer<int, 1> one;
    int result = 41;
    if (!one.empty() || one.try_pop(result) || result != 41 || !one.try_push(7) ||
        one.try_push(8) || !one.try_pop(result) || result != 7 || !one.empty()) return 1;
  }
  {
    hase::SpscRingBuffer<int, 2> q;
    int result{};
    if (!q.try_push(1) || !q.try_push(2) || q.try_push(3)) return 2;
    if (!q.try_pop(result) || result != 1 || !q.try_push(3)) return 3;
    if (!q.try_pop(result) || result != 2 || !q.try_pop(result) || result != 3 || !q.empty()) return 4;
  }
  {
    hase::SpscRingBuffer<LifetimeProbe, 2> q;
    LifetimeProbe result{-1};
    if (!q.try_push(LifetimeProbe{1}) || !q.try_push(LifetimeProbe{2}) ||
        !q.try_pop(result) || result.value != 1 || !q.try_push(LifetimeProbe{3})) return 5;
  }
  if (LifetimeProbe::live != 0) {
    std::cerr << "queued object lifetime leak or double destruction\n";
    return 6;
  }

  constexpr std::uint64_t count = 120000;
  hase::SpscRingBuffer<Message, 7> q;
  std::atomic<bool> failed = false;
  std::thread producer([&] {
    for (std::uint64_t i = 0; i < count;) {
      if (q.try_push(Message{i})) ++i;
    }
  });
  std::thread consumer([&] {
    Message received{UINT64_MAX};
    for (std::uint64_t expected = 0; expected < count;) {
      if (q.try_pop(received)) {
        if (!received.is_valid(expected)) failed.store(true, std::memory_order_relaxed);
        ++expected;
      }
    }
  });
  producer.join();
  consumer.join();
  if (failed.load(std::memory_order_relaxed)) {
    std::cerr << "FIFO or published payload integrity violation\n";
    return 7;
  }
}
