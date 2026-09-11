# CPP-004: Thread pool

Complete the C++20 `hase::ThreadPool` in `include/thread_pool.hpp` without changing its public API or adding external dependencies. Build and run the visible CTest suite.

`ThreadPool(worker_count)` starts exactly `worker_count` worker threads. `worker_count == 0` must throw `std::invalid_argument`. The pool is non-copyable and non-movable.

`submit(f, args...)` accepts lvalue or rvalue callables and arguments, schedules one invocation equivalent to `std::invoke(f, args...)`, and returns a `std::future` for its result. It must support ordinary return values, `void`, exceptions (reported by the returned future), move-only callables, and move-only arguments. Calls to `submit` from multiple threads are supported. A task exception must not stop a worker or prevent later tasks from running.

`shutdown()` stops acceptance of new work, waits for all work accepted before shutdown to finish, and joins every worker. It is safe to call more than once. Once shutdown has begun, `submit` must throw `std::runtime_error` and must not enqueue work. Destruction has the same draining and joining behavior as `shutdown()`. Concurrent calls to `shutdown()` itself are outside this task's scope.

The pool must not lose or execute a task more than once. Do not run tasks while holding the queue mutex, and do not use polling or arbitrary delays to coordinate workers. Calling `shutdown()` from one of the pool's own worker tasks is outside this task's scope.
