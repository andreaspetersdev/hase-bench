#include "template_engine/template.hpp"

#include <utility>

namespace template_engine {
namespace {

std::vector<std::string_view> split(std::string_view source) {
    std::vector<std::string_view> result;
    while (!source.empty()) {
        const auto marker = source.find("{name}");
        if (marker == std::string_view::npos) {
            result.push_back(source);
            break;
        }
        if (marker != 0) {
            result.push_back(source.substr(0, marker));
        }
        result.push_back(std::string_view{"{name}"});
        source.remove_prefix(marker + 6);
    }
    return result;
}

std::string render_pieces(const std::vector<std::string_view>& pieces, std::string_view name) {
    std::string result;
    for (const auto piece : pieces) {
        result += (piece == "{name}") ? name : piece;
    }
    return result;
}

} // namespace

TemplateView::TemplateView(std::string_view source) : source_(source), pieces_(split(source)) {}

TemplateView TemplateView::from_external(std::string_view source) {
    return TemplateView(source);
}

std::string TemplateView::render(std::string_view name) const { return render_pieces(pieces_, name); }
std::string_view TemplateView::source() const noexcept { return source_; }

CompiledTemplate::CompiledTemplate(std::string source, std::vector<std::string_view> pieces)
    : source_(std::move(source)), pieces_(std::move(pieces)) {}

CompiledTemplate CompiledTemplate::compile(std::string source) {
    return CompiledTemplate(std::move(source), split(source));
}

std::string CompiledTemplate::render(std::string_view name) const { return render_pieces(pieces_, name); }
std::string_view CompiledTemplate::source() const noexcept { return source_; }

} // namespace template_engine
