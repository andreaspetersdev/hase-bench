#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <span>
#include <vector>

namespace hase {

inline constexpr std::size_t kMaxPayloadSize = 1024;

struct Packet {
    std::uint8_t flags{};
    std::uint64_t timestamp{};
    std::vector<std::byte> payload;

    bool operator==(const Packet&) const = default;
};

enum class DecodeError {
    none,
    invalid_header,
    impossible_length,
    truncated,
    checksum_mismatch,
};

struct DecodeResult {
    std::optional<Packet> packet;
    DecodeError error{DecodeError::none};

    explicit operator bool() const { return packet.has_value(); }
};

std::optional<std::vector<std::byte>> serialize(const Packet& packet);
DecodeResult deserialize(std::span<const std::byte> bytes);

}  // namespace hase
