#include "audit.hpp"
#include <stdexcept>

namespace {
class TextRenderer final : public Renderer {
public:
    std::string render(const Record& record) const override {
        std::string result;
        for (std::size_t index = 0; index < record.fields.size(); ++index) {
            if (index) result += ' ';
            result += record.fields[index].first + '=' + record.fields[index].second;
        }
        return result;
    }
};
class JsonRenderer final : public Renderer {
    static std::string escape(std::string_view value) {
        std::string result;
        for (char character : value) {
            if (character == '\\' || character == '"') result += '\\';
            result += character;
        }
        return result;
    }
public:
    std::string render(const Record& record) const override {
        std::string result{"{"};
        for (std::size_t index = 0; index < record.fields.size(); ++index) {
            if (index) result += ',';
            result += '"' + escape(record.fields[index].first) + "\":\"" + escape(record.fields[index].second) + '"';
        }
        return result + '}';
    }
};
}

std::unique_ptr<Renderer> make_renderer(std::string_view format, std::vector<std::string>) {
    if (format == "text") return std::make_unique<TextRenderer>();
    if (format == "json") return std::make_unique<JsonRenderer>();
    throw std::invalid_argument("format");
}
