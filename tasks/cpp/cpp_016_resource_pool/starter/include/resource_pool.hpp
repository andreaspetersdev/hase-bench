#pragma once
#include <cstddef>
#include <memory>
#include <optional>
class ResourcePool { struct State; public: class Handle { public: Handle()=default; Handle(const Handle&)=delete; Handle& operator=(const Handle&)=delete; Handle(Handle&&) noexcept=default; Handle& operator=(Handle&&) noexcept=default; ~Handle(){reset();} [[nodiscard]] int value() const; [[nodiscard]] explicit operator bool() const noexcept; void reset() noexcept; private: friend class ResourcePool; Handle(std::weak_ptr<State>,int) noexcept; std::weak_ptr<State> state_; int value_{-1}; }; explicit ResourcePool(std::size_t); ResourcePool(const ResourcePool&)=delete; ResourcePool& operator=(const ResourcePool&)=delete; ResourcePool(ResourcePool&&) noexcept; ResourcePool& operator=(ResourcePool&&) noexcept; ~ResourcePool(); [[nodiscard]] std::optional<Handle> acquire(); [[nodiscard]] std::size_t available() const noexcept; private: std::shared_ptr<State> state_; };
