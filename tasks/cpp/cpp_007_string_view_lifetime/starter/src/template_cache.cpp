#include "template_engine/template_cache.hpp"

#include <stdexcept>
#include <utility>

namespace template_engine {

void TemplateCache::insert(std::string key, std::string source) {
    for (auto& entry : entries_) {
        if (entry.key == key) {
            entry.value = CompiledTemplate::compile(std::move(source));
            return;
        }
    }
    entries_.push_back(Entry{std::move(key), CompiledTemplate::compile(std::move(source))});
}

const CompiledTemplate* TemplateCache::find(std::string_view key) const noexcept {
    for (const auto& entry : entries_) {
        if (entry.key == key) return &entry.value;
    }
    return nullptr;
}

std::string TemplateCache::render(std::string_view key, std::string_view name) const {
    const auto* value = find(key);
    if (!value) throw std::out_of_range("template key not found");
    return value->render(name);
}

} // namespace template_engine
