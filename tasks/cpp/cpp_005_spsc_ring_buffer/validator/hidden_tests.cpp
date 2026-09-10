#include "spsc_ring_buffer.hpp"
#include <atomic>
#include <iostream>
#include <thread>
#include <utility>
int main() {
  { hase::SpscRingBuffer<int, 2> q; int result{}; if (!q.try_push(1)||!q.try_push(2)||q.try_push(3)) return 1; if(!q.try_pop(result)||result!=1||!q.try_push(3)) return 2; if(!q.try_pop(result)||result!=2||!q.try_pop(result)||result!=3||!q.empty()) return 3; }
  constexpr int count=200000; hase::SpscRingBuffer<int, 127> q; std::atomic<bool> failed=false;
  std::thread producer([&] { for(int i=0;i<count;) { if(q.try_push(i)) ++i; } });
  std::thread consumer([&] { for(int expected=0;expected<count;) { int got; if(q.try_pop(got)) { if(got!=expected) failed=true; ++expected; } } });
  producer.join(); consumer.join(); if(failed) { std::cerr << "FIFO violation\n"; return 4; }
}
