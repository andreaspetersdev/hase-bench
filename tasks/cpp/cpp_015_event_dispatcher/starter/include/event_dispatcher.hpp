#pragma once
#include <cstddef>
#include <functional>
#include <memory>
#include <typeindex>
#include <utility>

class Dispatcher {
    struct State;
public:
    class Subscription {
    public:
        Subscription() = default;
        Subscription(const Subscription&) = delete;
        Subscription& operator=(const Subscription&) = delete;
        Subscription(Subscription&&) noexcept = default;
        Subscription& operator=(Subscription&&) noexcept = default;
        ~Subscription() { reset(); }
        void reset() noexcept;
        [[nodiscard]] explicit operator bool() const noexcept;
    private:
        friend class Dispatcher;
        Subscription(std::weak_ptr<State> state, std::type_index type, std::size_t id) noexcept;
        std::weak_ptr<State> state_; std::type_index type_{typeid(void)}; std::size_t id_{};
    };
    Dispatcher();
    Dispatcher(const Dispatcher&) = delete;
    Dispatcher& operator=(const Dispatcher&) = delete;
    Dispatcher(Dispatcher&&) noexcept;
    Dispatcher& operator=(Dispatcher&&) noexcept;
    ~Dispatcher();
    template<class Event, class F> Subscription subscribe(F&& callback) {
        return subscribe_erased(typeid(Event), [fn = std::forward<F>(callback)](const void* event) { fn(*static_cast<const Event*>(event)); });
    }
    template<class Event> void emit(const Event& event) { emit_erased(typeid(Event), &event); }
private:
    Subscription subscribe_erased(std::type_index, std::function<void(const void*)>);
    void emit_erased(std::type_index, const void*);
    std::shared_ptr<State> state_;
};
