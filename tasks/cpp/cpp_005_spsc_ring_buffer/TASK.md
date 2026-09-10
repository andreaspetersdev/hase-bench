# CPP-005: SPSC ring buffer

Implement the C++20 `hase::SpscRingBuffer<T, Capacity>` template. It is used by exactly one producer thread and exactly one consumer thread concurrently; it is not an MPMC queue.

`try_push(T)` and `try_pop(T&)` must be non-blocking. A buffer has room for exactly `Capacity` values, reports that capacity, preserves FIFO ordering, and handles wrap-around. It must support move-only values and must not require `T` to be default-constructible.

Use atomics with acquire/release synchronization appropriate for SPSC ownership. Do not solve the task by putting a mutex around every operation or by relying solely on `memory_order_seq_cst`. Do not change the public API or add dependencies. Build and run the visible CTest suite.
