#include "packet.hpp"

#include <array>
#include <iostream>
#include <limits>

namespace {

using hase::DecodeError;

std::vector<std::byte> make_bytes(std::initializer_list<unsigned int> values) {
    std::vector<std::byte> result;
    result.reserve(values.size());
    for (const auto value : values) result.push_back(static_cast<std::byte>(value));
    return result;
}

bool expect_error(std::span<const std::byte> bytes, DecodeError error) {
    const auto result = hase::deserialize(bytes);
    return !result && !result.packet && result.error == error;
}

}  // namespace

int main() {
    hase::Packet empty{0, 0, {}};
    const auto empty_bytes = hase::serialize(empty);
    if (!empty_bytes || empty_bytes->size() != 24 || !hase::deserialize(*empty_bytes)) {
        std::cerr << "empty payload round trip failed\n";
        return 1;
    }

    hase::Packet boundary{0xff, std::numeric_limits<std::uint64_t>::max(),
                          std::vector<std::byte>(hase::kMaxPayloadSize, std::byte{0xa7})};
    const auto boundary_bytes = hase::serialize(boundary);
    const auto boundary_decoded = boundary_bytes ? hase::deserialize(*boundary_bytes) : hase::DecodeResult{};
    if (!boundary_bytes || !boundary_decoded || *boundary_decoded.packet != boundary) {
        std::cerr << "boundary payload or timestamp failed\n";
        return 2;
    }

    if (!expect_error({}, DecodeError::truncated) ||
        !expect_error(make_bytes({'H', 'A', 'S'}), DecodeError::truncated) ||
        !expect_error(make_bytes({'X'}), DecodeError::invalid_header)) {
        std::cerr << "short fixed header classification failed\n";
        return 3;
    }
    auto bad_magic = *empty_bytes;
    bad_magic[0] = std::byte{'X'};
    if (!expect_error(bad_magic, DecodeError::invalid_header)) {
        std::cerr << "invalid magic classification failed\n";
        return 4;
    }
    auto bad_version = *empty_bytes;
    bad_version[4] = std::byte{2};
    if (!expect_error(bad_version, DecodeError::invalid_header)) return 5;
    auto bad_reserved = *empty_bytes;
    bad_reserved[6] = std::byte{1};
    if (!expect_error(bad_reserved, DecodeError::invalid_header)) return 6;

    auto impossible = *empty_bytes;
    impossible[8] = std::byte{0}; impossible[9] = std::byte{0};
    impossible[10] = std::byte{4}; impossible[11] = std::byte{1};
    impossible.resize(20);
    if (!expect_error(impossible, DecodeError::impossible_length)) {
        std::cerr << "impossible length classification failed\n";
        return 7;
    }
    auto truncated = *empty_bytes;
    truncated[11] = std::byte{1};
    truncated.resize(20);
    if (!expect_error(truncated, DecodeError::truncated)) {
        std::cerr << "payload truncation classification failed\n";
        return 8;
    }
    auto missing_checksum = *empty_bytes;
    missing_checksum.pop_back();
    if (!expect_error(missing_checksum, DecodeError::truncated)) return 9;
    auto trailing = *empty_bytes;
    trailing.push_back(std::byte{0});
    if (!expect_error(trailing, DecodeError::invalid_header)) return 10;
}
