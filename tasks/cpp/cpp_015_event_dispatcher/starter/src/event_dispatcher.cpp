#include "event_dispatcher.hpp"
struct Dispatcher::State {};
Dispatcher::Dispatcher() = default;
Dispatcher::Dispatcher(Dispatcher&&) noexcept = default;
Dispatcher& Dispatcher::operator=(Dispatcher&&) noexcept = default;
Dispatcher::~Dispatcher() = default;
Dispatcher::Subscription::Subscription(std::weak_ptr<State>, std::type_index, std::size_t) noexcept {}
void Dispatcher::Subscription::reset() noexcept {}
Dispatcher::Subscription::operator bool() const noexcept { return false; }
Dispatcher::Subscription Dispatcher::subscribe_erased(std::type_index, std::function<void(const void*)>) { return {}; }
void Dispatcher::emit_erased(std::type_index, const void*) {}
