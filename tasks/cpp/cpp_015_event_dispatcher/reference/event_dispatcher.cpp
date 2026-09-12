#include "event_dispatcher.hpp"
#include <algorithm>
#include <unordered_map>
#include <vector>

struct Dispatcher::State {
    struct Entry { std::size_t id; std::function<void(const void*)> callback; };
    std::unordered_map<std::type_index, std::vector<Entry>> entries;
    std::size_t next_id{1};
    void erase(std::type_index type, std::size_t id) noexcept {
        auto it = entries.find(type); if (it == entries.end()) return;
        auto& list = it->second;
        list.erase(std::remove_if(list.begin(), list.end(), [=](const Entry& entry) { return entry.id == id; }), list.end());
    }
    bool has(std::type_index type, std::size_t id) const noexcept {
        auto it = entries.find(type); return it != entries.end() && std::any_of(it->second.begin(), it->second.end(), [=](const Entry& entry) { return entry.id == id; });
    }
};
Dispatcher::Dispatcher() : state_(std::make_shared<State>()) {}
Dispatcher::Dispatcher(Dispatcher&&) noexcept = default;
Dispatcher& Dispatcher::operator=(Dispatcher&&) noexcept = default;
Dispatcher::~Dispatcher() = default;
Dispatcher::Subscription::Subscription(std::weak_ptr<State> state, std::type_index type, std::size_t id) noexcept : state_(std::move(state)), type_(type), id_(id) {}
void Dispatcher::Subscription::reset() noexcept { if (auto state = state_.lock()) state->erase(type_, id_); state_.reset(); id_ = 0; }
Dispatcher::Subscription::operator bool() const noexcept { auto state = state_.lock(); return state && state->has(type_, id_); }
Dispatcher::Subscription Dispatcher::subscribe_erased(std::type_index type, std::function<void(const void*)> callback) { const auto id = state_->next_id++; state_->entries[type].push_back({id, std::move(callback)}); return Subscription(state_, type, id); }
void Dispatcher::emit_erased(std::type_index type, const void* event) { auto it = state_->entries.find(type); if (it == state_->entries.end()) return; std::vector<std::size_t> snapshot; for (const auto& entry : it->second) snapshot.push_back(entry.id); for (auto id : snapshot) { it = state_->entries.find(type); if (it == state_->entries.end()) return; auto found = std::find_if(it->second.begin(), it->second.end(), [=](const State::Entry& entry) { return entry.id == id; }); if (found != it->second.end()) found->callback(event); } }
