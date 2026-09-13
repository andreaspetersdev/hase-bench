#include "parser.hpp"

#include <charconv>
#include <cstdint>

namespace {
template <typename Integer>
bool decimal(std::string_view text, Integer& value) {
    if (text.empty()) return false;
    for (char c : text) if (c < '0' || c > '9') return false;
    const auto result = std::from_chars(text.data(), text.data() + text.size(), value);
    return result.ec == std::errc{} && result.ptr == text.data() + text.size();
}

bool valid_source(std::string_view name) {
    if (name.empty() || name.front() < 'a' || name.front() > 'z') return false;
    for (char c : name)
        if (!(c >= 'a' && c <= 'z') && !(c >= '0' && c <= '9') && c != '_') return false;
    return true;
}

bool parse_record(std::string_view line, Event& event) {
    const auto first = line.find(',');
    if (first == std::string_view::npos) return false;
    const auto second = line.find(',', first + 1);
    if (second != std::string_view::npos && line.find(',', second + 1) != std::string_view::npos) return false;
    const auto raw_value = second == std::string_view::npos ? line.substr(first + 1) : line.substr(second + 1);
    std::uint32_t value{};
    if (!decimal(line.substr(0, first), event.timestamp) ||
        !decimal(raw_value, value) || value > 1000000) return false;
    if (second == std::string_view::npos) {
        event.source = "default";
    } else {
        const auto source = line.substr(first + 1, second - first - 1);
        if (!valid_source(source)) return false;
        event.source = source;
    }
    event.value = value;
    return true;
}
}

bool RecordParser::feed(std::string_view chunk, std::vector<Event>& completed) {
    partial_.append(chunk);
    for (;;) {
        const auto newline = partial_.find('\n');
        if (newline == std::string::npos) return partial_.size() <= max_line_bytes_;
        if (newline > max_line_bytes_) return false;
        std::string_view line(partial_.data(), newline);
        if (!line.empty() && line.back() == '\r') line.remove_suffix(1);
        Event event;
        if (!parse_record(line, event)) return false;
        completed.push_back(std::move(event));
        partial_.erase(0, newline + 1);
    }
}
