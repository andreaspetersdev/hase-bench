#include "thread_pool.hpp"

#include <atomic>
#include <chrono>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <thread>
#include <type_traits>
#include <vector>

using namespace std::chrono_literals;

int main() {
  static_assert(!std::is_copy_constructible_v<hase::ThreadPool>);
  static_assert(!std::is_copy_assignable_v<hase::ThreadPool>);
  static_assert(!std::is_move_constructible_v<hase::ThreadPool>);
  static_assert(!std::is_move_assignable_v<hase::ThreadPool>);

  try {
    hase::ThreadPool invalid(0);
    std::cerr << "zero workers were accepted\n";
    return 1;
  } catch (const std::invalid_argument&) {
  }

  hase::ThreadPool pool(4);
  auto move_only = pool.submit(
      [state = std::make_unique<int>(40)](std::unique_ptr<int> increment) {
        return *state + *increment;
      },
      std::make_unique<int>(2));
  if (move_only.get() != 42) {
    std::cerr << "move-only callable or argument failed\n";
    return 2;
  }

  int offset = 40;
  auto lvalue_callable = [&offset](int increment) { return offset + increment; };
  int lvalue_argument = 2;
  if (pool.submit(lvalue_callable, lvalue_argument).get() != 42) {
    std::cerr << "lvalue callable or argument failed\n";
    return 7;
  }

  std::atomic<int> entered = 0;
  std::atomic<bool> release = false;
  std::vector<std::future<void>> blockers;
  for (int i = 0; i < 4; ++i) {
    blockers.push_back(pool.submit([&] {
      entered.fetch_add(1, std::memory_order_release);
      while (!release.load(std::memory_order_acquire)) std::this_thread::yield();
    }));
  }
  const auto deadline = std::chrono::steady_clock::now() + 3s;
  while (entered.load(std::memory_order_acquire) != 4 &&
         std::chrono::steady_clock::now() < deadline) {
    std::this_thread::yield();
  }
  if (entered.load(std::memory_order_acquire) != 4) {
    std::cerr << "fixed worker count was not reached\n";
    return 3;
  }

  constexpr int submitter_count = 6;
  constexpr int jobs_per_submitter = 50;
  std::atomic<int> executions = 0;
  std::vector<std::thread> submitters;
  std::vector<std::vector<std::future<int>>> futures(submitter_count);
  for (int producer = 0; producer < submitter_count; ++producer) {
    submitters.emplace_back([&, producer] {
      futures[producer].reserve(jobs_per_submitter);
      for (int i = 0; i < jobs_per_submitter; ++i) {
        futures[producer].push_back(pool.submit([&, producer, i] {
          executions.fetch_add(1, std::memory_order_relaxed);
          return producer * jobs_per_submitter + i;
        }));
      }
    });
  }
  for (auto& submitter : submitters) submitter.join();
  release.store(true, std::memory_order_release);
  for (auto& blocker : blockers) blocker.get();
  for (int producer = 0; producer < submitter_count; ++producer) {
    for (int i = 0; i < jobs_per_submitter; ++i) {
      if (futures[producer][i].get() != producer * jobs_per_submitter + i) {
        std::cerr << "a future returned the wrong result\n";
        return 4;
      }
    }
  }
  if (executions != submitter_count * jobs_per_submitter) {
    std::cerr << "a concurrently submitted task was lost or duplicated\n";
    return 5;
  }

  std::atomic<int> drained = 0;
  for (int i = 0; i < 100; ++i) {
    (void)pool.submit([&] { drained.fetch_add(1, std::memory_order_relaxed); });
  }
  pool.shutdown();
  if (drained != 100) {
    std::cerr << "shutdown discarded accepted work\n";
    return 6;
  }
}
