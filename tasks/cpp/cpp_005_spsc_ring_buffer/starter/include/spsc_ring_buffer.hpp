#pragma once
#include <atomic>
#include <cstddef>
namespace hase {
template <class T, std::size_t Capacity> class SpscRingBuffer {
  static_assert(Capacity > 0);
public:
  [[nodiscard]] static constexpr std::size_t capacity() noexcept { return Capacity; }
  [[nodiscard]] bool try_push(T) { return false; }
  [[nodiscard]] bool try_pop(T&) { return false; }
  [[nodiscard]] bool empty() const noexcept { return true; }
private:
  std::atomic<std::size_t> producer_{0};
  std::atomic<std::size_t> consumer_{0};
};
}
