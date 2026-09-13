#include "config.hpp"

#include <charconv>

namespace {
bool number(std::string_view text, std::uint32_t limit, std::uint32_t& value) {
    if (text.empty()) return false;
    for (const char c : text) if (c < '0' || c > '9') return false;
    unsigned long long parsed{};
    auto result = std::from_chars(text.data(), text.data() + text.size(), parsed);
    if (result.ec != std::errc{} || result.ptr != text.data() + text.size() || parsed > limit) return false;
    value = static_cast<std::uint32_t>(parsed);
    return true;
}

bool source_name(std::string_view name) {
    if (name.empty() || name.front() < 'a' || name.front() > 'z') return false;
    for (char c : name)
        if (!(c >= 'a' && c <= 'z') && !(c >= '0' && c <= '9') && c != '_') return false;
    return true;
}
}

std::optional<Config> Config::parse(std::string_view text) {
    if (text.empty()) return std::nullopt;
    Config result;
    bool capacity_seen = false, line_seen = false, default_seen = false;
    while (!text.empty()) {
        const auto end = text.find('\n');
        const auto line = text.substr(0, end);
        if (line.empty()) return std::nullopt;
        const auto equal = line.find('=');
        if (equal == std::string_view::npos) return std::nullopt;
        const auto key = line.substr(0, equal);
        const auto raw = line.substr(equal + 1);
        std::uint32_t value{};
        if (key == "capacity") {
            if (capacity_seen || !number(raw, 32, value) || value == 0) return std::nullopt;
            result.capacity_ = value;
            capacity_seen = true;
        } else if (key == "max_line_bytes") {
            if (line_seen || !number(raw, 256, value) || value < 16) return std::nullopt;
            result.max_line_bytes_ = value;
            line_seen = true;
        } else if (key == "default_threshold") {
            if (default_seen || !number(raw, 1000000, value)) return std::nullopt;
            result.default_threshold_ = value;
            default_seen = true;
        } else if (key.starts_with("threshold.")) {
            const auto source = key.substr(10);
            if (!source_name(source) || !number(raw, 1000000, value)) return std::nullopt;
            for (const auto& [existing, ignored] : result.overrides_)
                if (existing == source) return std::nullopt;
            result.overrides_.emplace_back(source, value);
        } else {
            return std::nullopt;
        }
        if (end == std::string_view::npos) break;
        text.remove_prefix(end + 1);
    }
    if (!capacity_seen || !line_seen || !default_seen) return std::nullopt;
    return result;
}

std::uint32_t Config::threshold_for(std::string_view source) const noexcept {
    for (const auto& [name, threshold] : overrides_)
        if (name == source) return threshold;
    return default_threshold_;
}
