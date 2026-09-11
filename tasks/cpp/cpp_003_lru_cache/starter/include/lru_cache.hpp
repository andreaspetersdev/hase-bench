#pragma once
#include <cstddef>
#include <optional>
namespace hase {
template <class Key, class Value> class LruCache {
public:
  explicit LruCache(std::size_t capacity) : capacity_(capacity) {}
  LruCache(const LruCache&) = delete;
  LruCache& operator=(const LruCache&) = delete;
  LruCache(LruCache&&) = delete;
  LruCache& operator=(LruCache&&) = delete;
  [[nodiscard]] std::size_t size() const noexcept { return size_; }
  [[nodiscard]] std::size_t capacity() const noexcept { return capacity_; }
  Value* get(const Key&) noexcept { return nullptr; }
  template <class K, class V> void put(K&&, V&&) { }
private: std::size_t capacity_; std::size_t size_ = 0;
};
}
