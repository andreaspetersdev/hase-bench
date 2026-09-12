# CPP-015 — Generic event dispatcher

Implement the C++20 API in `include/event_dispatcher.hpp`. Do not change its
public declarations, CMake target names, or add dependencies.

`Dispatcher` supports type-safe subscriptions and synchronous event delivery.
`subscribe<Event>(callable)` returns a move-only `Subscription`; `emit(event)`
delivers only to subscribers of exactly that event type.

- Subscribers run in subscription order. A callback added while an event is
  being emitted does not receive that in-progress emission.
- `reset()` is idempotent. A reset subscription must not run later in the same
  emission, including when reset by an earlier callback. Self-reset is valid.
- Nested `emit` is valid and obeys the same rules independently.
- Callback exceptions propagate to the caller of `emit` and stop that emission;
  the dispatcher remains usable afterwards.
- `Dispatcher` is movable but not copyable. Existing subscriptions remain
  connected after move construction or move assignment; subscriptions belonging
  to the dispatcher previously held by a move-assignment target become inert.
- Destroying a dispatcher makes outstanding subscriptions inert and safe to
  reset. The API is single-threaded; concurrent calls are outside scope.

Build and run the visible tests:

```powershell
cmake -S . -B build
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```
