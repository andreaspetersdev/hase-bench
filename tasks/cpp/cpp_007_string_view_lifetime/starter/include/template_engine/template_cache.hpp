#pragma once

#include "template_engine/template.hpp"

#include <string>
#include <string_view>
#include <vector>

namespace template_engine {

class TemplateCache {
public:
    void insert(std::string key, std::string source);
    [[nodiscard]] const CompiledTemplate* find(std::string_view key) const noexcept;
    [[nodiscard]] std::string render(std::string_view key, std::string_view name) const;

private:
    struct Entry {
        std::string key;
        CompiledTemplate value;
    };
    std::vector<Entry> entries_;
};

} // namespace template_engine
