#pragma once

#include <string>
#include <string_view>
#include <vector>

namespace template_engine {

class TemplateView {
public:
    static TemplateView from_external(std::string_view source);
    [[nodiscard]] std::string render(std::string_view name) const;
    [[nodiscard]] std::string_view source() const noexcept;

private:
    explicit TemplateView(std::string_view source);
    std::string_view source_;
    std::vector<std::string_view> pieces_;
};

class CompiledTemplate {
public:
    static CompiledTemplate compile(std::string source);
    [[nodiscard]] std::string render(std::string_view name) const;
    [[nodiscard]] std::string_view source() const noexcept;

private:
    CompiledTemplate(std::string source, std::vector<std::string_view> pieces);
    std::string source_;
    std::vector<std::string_view> pieces_;
};

} // namespace template_engine
