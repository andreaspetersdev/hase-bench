# CPP-016 — Resource pool with RAII handle

Complete the C++20 implementation in `include/resource_pool.hpp`; preserve its
public API and do not add dependencies.

`ResourcePool(n)` creates resources numbered `0` through `n - 1`. `acquire()`
returns an empty optional when exhausted; otherwise it returns an owning,
move-only `Handle`. A live handle returns its resource exactly once when reset
or destroyed. `available()` is the number currently obtainable.

- Moved-from handles are inert. Move assignment releases the destination’s
  previously owned resource before taking the source resource.
- A pool is movable but not copyable. Handles acquired before moving a pool
  remain connected to the moved-to pool. Handles associated with a pool replaced
  by move assignment become inert, even if reset later.
- Destruction of a pool makes outstanding handles inert and safe to destroy.
- `ResourcePool(0)` throws `std::invalid_argument`. The API is single-threaded.

Run the visible tests with CMake before finishing.
