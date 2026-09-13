#pragma once

#include "config.hpp"
#include "parser.hpp"
#include "stats.hpp"
#include "work_queue.hpp"

#include <string_view>
#include <vector>

class Service {
public:
    explicit Service(Config config, WorkQueue::Sink sink = {});
    Service(const Service&) = delete;
    Service& operator=(const Service&) = delete;
    ~Service();

    [[nodiscard]] bool ingest(std::string_view chunk);
    void close();
    [[nodiscard]] bool failed() const;
    [[nodiscard]] StatsSnapshot snapshot() const;
    [[nodiscard]] std::vector<Event> undelivered() const;

private:
    Config config_;
    RecordParser parser_;
    Statistics statistics_;
    WorkQueue queue_;
};
