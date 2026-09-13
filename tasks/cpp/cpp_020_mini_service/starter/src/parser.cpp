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

bool parse_legacy(std::string_view line, Event& event) {
    const auto comma = line.find(',');
    if (comma == std::string_view::npos || line.find(',', comma + 1) != std::string_view::npos) return false;
    std::uint32_t value{};
    if (!decimal(line.substr(0, comma), event.timestamp) ||
        !decimal(line.substr(comma + 1), value) || value > 1000000) return false;
    event.source = "default";
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
        if (!parse_legacy(line, event)) return false;
        completed.push_back(std::move(event));
        partial_.erase(0, newline + 1);
    }
}
