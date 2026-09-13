#include "stats.hpp"

#include <algorithm>

void Statistics::record(const Event& event, std::uint32_t threshold) {
    std::lock_guard lock(mutex_);
    ++state_.count;
    state_.sum += event.value;
    const bool alert = event.value >= threshold;
    if (alert) ++state_.alerts;
    auto it = std::lower_bound(state_.sources.begin(), state_.sources.end(), event.source,
        [](const SourceStats& row, const std::string& name) { return row.source < name; });
    if (it == state_.sources.end() || it->source != event.source)
        it = state_.sources.insert(it, SourceStats{event.source});
    ++it->count;
    it->sum += event.value;
    if (alert) ++it->alerts;
}

StatsSnapshot Statistics::snapshot() const {
    std::lock_guard lock(mutex_);
    return state_;
}
