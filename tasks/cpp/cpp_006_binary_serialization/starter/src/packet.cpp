#include "packet.hpp"

namespace hase {

std::optional<std::vector<std::byte>> serialize(const Packet&) {
    return std::nullopt;
}

DecodeResult deserialize(std::span<const std::byte>) {
    return {};
}

}  // namespace hase
