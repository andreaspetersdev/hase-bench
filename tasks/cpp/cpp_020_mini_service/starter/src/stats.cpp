#include "stats.hpp"

void Statistics::record(const Event& event, std::uint32_t threshold) {
    std::lock_guard lock(mutex_);
    ++state_.count;
    state_.sum += event.value;
    if (event.value > threshold) ++state_.alerts;
}

StatsSnapshot Statistics::snapshot() const {
    std::lock_guard lock(mutex_);
    return state_;
}
