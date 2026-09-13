#pragma once

#include "event.hpp"

#include <cstdint>
#include <mutex>
#include <string>
#include <vector>

struct SourceStats {
    std::string source;
    std::uint64_t count{};
    std::uint64_t sum{};
    std::uint64_t alerts{};
};

struct StatsSnapshot {
    std::uint64_t count{};
    std::uint64_t sum{};
    std::uint64_t alerts{};
    std::vector<SourceStats> sources;
};

class Statistics {
public:
    void record(const Event& event, std::uint32_t threshold);
    [[nodiscard]] StatsSnapshot snapshot() const;

private:
    mutable std::mutex mutex_;
    StatsSnapshot state_;
};
