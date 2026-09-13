#include "service.hpp"

#include <utility>

Service::Service(Config config, WorkQueue::Sink sink)
    : config_(std::move(config)), parser_(config_.max_line_bytes()),
      queue_(config_.capacity(), std::move(sink), statistics_) {}

Service::~Service() { close(); }

bool Service::ingest(std::string_view chunk) {
    RecordParser candidate = parser_;
    std::vector<Event> parsed;
    if (!candidate.feed(chunk, parsed)) return false;
    std::vector<WorkItem> batch;
    for (auto& event : parsed) {
        const auto threshold = config_.threshold_for(event.source);
        batch.push_back({std::move(event), threshold});
    }
    if (!queue_.submit_batch(std::move(batch))) return false;
    parser_ = std::move(candidate);
    return true;
}

void Service::close() { queue_.close(); }
bool Service::failed() const { return queue_.failed(); }
StatsSnapshot Service::snapshot() const { return statistics_.snapshot(); }
std::vector<Event> Service::undelivered() const { return queue_.undelivered(); }
