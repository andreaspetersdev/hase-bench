#include "config.hpp"
#include "parser.hpp"
#include "stats.hpp"

#include <algorithm>
#include <cassert>
#include <string>
#include <string_view>
#include <vector>

namespace {
void config_contract() {
    std::string input = "threshold.sensor_2=7\ncapacity=2\nmax_line_bytes=32\ndefault_threshold=9";
    auto config = Config::parse(input);
    assert(config);
    std::fill(input.begin(), input.end(), 'x');
    assert(config->capacity() == 2 && config->max_line_bytes() == 32);
    assert(config->threshold_for("sensor_2") == 7);
    assert(config->threshold_for("other") == 9);
    assert(!Config::parse("capacity=2\nmax_line_bytes=32\n"));
    assert(!Config::parse("capacity=0\nmax_line_bytes=32\ndefault_threshold=0"));
    assert(!Config::parse("capacity=2\ncapacity=3\nmax_line_bytes=32\ndefault_threshold=0"));
    assert(!Config::parse("capacity=2\nmax_line_bytes=32\ndefault_threshold=0\nthreshold.a=1\nthreshold.a=2"));
    assert(!Config::parse("capacity=2\nmax_line_bytes=32\ndefault_threshold=0\nthreshold.Bad=1"));
    assert(!Config::parse("capacity=2\nmax_line_bytes=32\ndefault_threshold=0\nunknown=1"));
    assert(!Config::parse("capacity=2\nmax_line_bytes=32\ndefault_threshold=1000001"));
    assert(!Config::parse("capacity=2\nmax_line_bytes=32\ndefault_threshold=-1"));
}

void parser_contract() {
    const std::string corpus = "1,4\n2,sensor_2,8\r\n3,0\n";
    for (std::size_t split = 0; split <= corpus.size(); ++split) {
        RecordParser parser(32);
        std::vector<Event> events;
        assert(parser.feed(std::string_view(corpus).substr(0, split), events));
        assert(parser.feed(std::string_view(corpus).substr(split), events));
        assert(events.size() == 3);
        assert(events[0].timestamp == 1 && events[0].source == "default" && events[0].value == 4);
        assert(events[1].timestamp == 2 && events[1].source == "sensor_2" && events[1].value == 8);
        assert(events[2].timestamp == 3 && events[2].source == "default" && events[2].value == 0);
    }
    const std::vector<std::string> invalid = {
        "\n", "1,2\rX\n", "1,2\r\r\n", "-1,2\n", "1,+2\n",
        "1,Upper,2\n", "1,2,3,4\n", "1,,2\n", "1,1000001\n",
        "18446744073709551616,1\n", "1,2x\n"
    };
    for (const auto& input : invalid) {
        RecordParser parser(64);
        std::vector<Event> events;
        assert(!parser.feed(input, events));
    }
    RecordParser limit(16);
    std::vector<Event> events;
    assert(limit.feed("1234567890123456", events));
    assert(!limit.feed("7", events));
}

void statistics_contract() {
    Statistics statistics;
    statistics.record(Event{1, "zeta", 5}, 5);
    statistics.record(Event{2, "alpha", 3}, 4);
    statistics.record(Event{3, "alpha", 4}, 4);
    const auto snapshot = statistics.snapshot();
    assert(snapshot.count == 3 && snapshot.sum == 12 && snapshot.alerts == 2);
    assert(snapshot.sources.size() == 2);
    assert(snapshot.sources[0].source == "alpha" && snapshot.sources[0].count == 2);
    assert(snapshot.sources[0].sum == 7 && snapshot.sources[0].alerts == 1);
    assert(snapshot.sources[1].source == "zeta" && snapshot.sources[1].count == 1);
    assert(snapshot.sources[1].sum == 5 && snapshot.sources[1].alerts == 1);
}
}

int main() {
    config_contract();
    parser_contract();
    statistics_contract();
}
