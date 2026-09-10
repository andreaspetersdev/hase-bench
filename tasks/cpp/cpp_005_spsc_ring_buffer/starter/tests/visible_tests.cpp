#include "spsc_ring_buffer.hpp"
#include <iostream>
#include <memory>
int main() {
 hase::SpscRingBuffer<std::unique_ptr<int>, 2> queue;
 if (!queue.try_push(std::make_unique<int>(7))) { std::cerr << "push failed\n"; return 1; }
 std::unique_ptr<int> value; if (!queue.try_pop(value) || !value || *value != 7) { std::cerr << "pop failed\n"; return 1; }
}
