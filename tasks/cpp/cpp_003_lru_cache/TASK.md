# CPP-003: Generic LRU cache

Complete the C++20 `hase::LruCache<Key, Value>` template without changing its public API. `get` returns a pointer to the stored value (or `nullptr`) and counts as access. `put` inserts or replaces a value. The most recently accessed or inserted item is most-recent; when full, `put` evicts the least-recent item. A zero-capacity cache stores nothing.

Expected operations should be approximately O(1). Values must support move-only types. Do not add external dependencies. Build and run the visible CTest suite.
