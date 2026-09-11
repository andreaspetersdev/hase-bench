#include "thread_pool.hpp"

#include <atomic>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

int main() {
  hase::ThreadPool pool(3);

  auto number = pool.submit([](int a, int b) { return a + b; }, 19, 23);
  auto text = pool.submit([](std::string value) { return value + " pool"; },
                          std::string{"thread"});
  auto no_result = pool.submit([] {});
  if (number.get() != 42 || text.get() != "thread pool") {
    std::cerr << "basic results are wrong\n";
    return 1;
  }
  no_result.get();

  auto throws = pool.submit([] { throw std::logic_error("expected"); });
  try {
    throws.get();
    std::cerr << "exception was not propagated through future\n";
    return 2;
  } catch (const std::logic_error&) {
  }
  if (pool.submit([] { return 7; }).get() != 7) {
    std::cerr << "worker did not continue after exception\n";
    return 3;
  }

  std::atomic<int> completed = 0;
  std::vector<std::future<void>> jobs;
  for (int i = 0; i < 40; ++i) {
    jobs.push_back(pool.submit([&completed] { completed.fetch_add(1); }));
  }
  for (auto& job : jobs) job.get();
  if (completed != 40) {
    std::cerr << "not every task ran\n";
    return 4;
  }

  pool.shutdown();
  pool.shutdown();
  try {
    (void)pool.submit([] { return 1; });
    std::cerr << "submit after shutdown was accepted\n";
    return 5;
  } catch (const std::runtime_error&) {
  }
}
