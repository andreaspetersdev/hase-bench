#pragma once

#include <condition_variable>
#include <cstddef>
#include <functional>
#include <future>
#include <mutex>
#include <queue>
#include <stdexcept>
#include <thread>
#include <type_traits>
#include <utility>
#include <vector>

namespace hase {

class ThreadPool {
public:
  explicit ThreadPool(std::size_t worker_count) {
    (void)worker_count;
  }

  ~ThreadPool() { shutdown(); }

  ThreadPool(const ThreadPool&) = delete;
  ThreadPool& operator=(const ThreadPool&) = delete;
  ThreadPool(ThreadPool&&) = delete;
  ThreadPool& operator=(ThreadPool&&) = delete;

  template <class Function, class... Args>
  auto submit(Function&& function, Args&&... args)
      -> std::future<std::invoke_result_t<Function, Args...>> {
    using Result = std::invoke_result_t<Function, Args...>;
    (void)function;
    (void)sizeof...(args);
    throw std::runtime_error("ThreadPool::submit is not implemented");
  }

  void shutdown() {}

private:
  void worker_loop() {}

  std::mutex mutex_;
  std::condition_variable work_available_;
  std::queue<std::function<void()>> work_;
  std::vector<std::thread> workers_;
  bool stopping_ = false;
};

}  // namespace hase
