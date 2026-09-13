#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

class Config {
public:
    [[nodiscard]] static std::optional<Config> parse(std::string_view text);
    [[nodiscard]] std::size_t capacity() const noexcept { return capacity_; }
    [[nodiscard]] std::size_t max_line_bytes() const noexcept { return max_line_bytes_; }
    [[nodiscard]] std::uint32_t threshold_for(std::string_view source) const noexcept;

private:
    std::size_t capacity_{};
    std::size_t max_line_bytes_{};
    std::uint32_t default_threshold_{};
    std::vector<std::pair<std::string, std::uint32_t>> overrides_;
    std::string_view original_text_;
};
