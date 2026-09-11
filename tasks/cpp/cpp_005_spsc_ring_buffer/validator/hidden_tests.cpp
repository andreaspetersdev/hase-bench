#include "spsc_ring_buffer.hpp"
#include <atomic>
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
}
int main() {
  { hase::SpscRingBuffer<int, 2> q; int result{}; if (!q.try_push(1)||!q.try_push(2)||q.try_push(3)) return 1; if(!q.try_pop(result)||result!=1||!q.try_push(3)) return 2; if(!q.try_pop(result)||result!=2||!q.try_pop(result)||result!=3||!q.empty()) return 3; }
  { hase::SpscRingBuffer<LifetimeProbe, 2> q; if (!q.try_push(LifetimeProbe{9})) return 4; }
  if (LifetimeProbe::live != 0) { std::cerr << "queued object lifetime leak or double destruction\n"; return 5; }
  constexpr int count=200000; hase::SpscRingBuffer<int, 127> q; std::atomic<bool> failed=false;
  std::thread producer([&] { for(int i=0;i<count;) { if(q.try_push(i)) ++i; } });
  std::thread consumer([&] { for(int expected=0;expected<count;) { int got; if(q.try_pop(got)) { if(got!=expected) failed=true; ++expected; } } });
  producer.join(); consumer.join(); if(failed) { std::cerr << "FIFO violation\n"; return 6; }
}
