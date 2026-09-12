#include "resource_pool.hpp"
#include <stdexcept>
struct ResourcePool::State {};
ResourcePool::ResourcePool(std::size_t n){if(!n)throw std::invalid_argument("n");}
ResourcePool::ResourcePool(ResourcePool&&) noexcept=default; ResourcePool& ResourcePool::operator=(ResourcePool&&) noexcept=default; ResourcePool::~ResourcePool()=default;
ResourcePool::Handle::Handle(std::weak_ptr<State>,int) noexcept{} int ResourcePool::Handle::value() const{return value_;} ResourcePool::Handle::operator bool() const noexcept{return false;} void ResourcePool::Handle::reset() noexcept{} std::optional<ResourcePool::Handle> ResourcePool::acquire(){return std::nullopt;} std::size_t ResourcePool::available() const noexcept{return 0;}
