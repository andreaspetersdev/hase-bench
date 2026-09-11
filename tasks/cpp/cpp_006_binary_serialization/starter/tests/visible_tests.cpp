#include "packet.hpp"

#include <iostream>

namespace {

std::vector<std::byte> bytes(std::initializer_list<unsigned int> values) {
    std::vector<std::byte> result;
    result.reserve(values.size());
    for (const auto value : values) {
        result.push_back(static_cast<std::byte>(value));
    }
    return result;
}

}  // namespace

int main() {
    const hase::Packet expected{0xa5, 0x0102030405060708ULL, bytes({0, 1, 0x7f, 0xff})};
    const auto encoded = hase::serialize(expected);
    if (!encoded || encoded->size() != 28) {
        std::cerr << "serialize failed\n";
        return 1;
    }
    const auto decoded = hase::deserialize(*encoded);
    if (!decoded || decoded.error != hase::DecodeError::none || *decoded.packet != expected) {
        std::cerr << "round trip failed\n";
        return 2;
    }
    if ((*encoded)[0] != std::byte{'H'} || (*encoded)[3] != std::byte{'E'} ||
        (*encoded)[8] != std::byte{0} || (*encoded)[11] != std::byte{4} ||
        (*encoded)[12] != std::byte{1} || (*encoded)[19] != std::byte{8}) {
        std::cerr << "wire byte order failed\n";
        return 3;
    }
    auto corrupt = *encoded;
    corrupt[20] = std::byte{42};
    const auto bad_checksum = hase::deserialize(corrupt);
    if (bad_checksum || bad_checksum.error != hase::DecodeError::checksum_mismatch) {
        std::cerr << "checksum corruption accepted\n";
        return 4;
    }
    hase::Packet too_large{};
    too_large.payload.resize(hase::kMaxPayloadSize + 1);
    if (hase::serialize(too_large)) {
        std::cerr << "oversized payload serialized\n";
        return 5;
    }
}
