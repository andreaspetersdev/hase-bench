#include "message_recorder.hpp"

#include <cassert>
#include <chrono>
#include <future>
#include <string>

int main() {
    std::promise<void> writer_entered;
    std::promise<void> release_writer;
    auto released = release_writer.get_future().share();
    MessageRecorder recorder(1, [&](const Persisted& item) {
        if (item.sequence == 0) {
            writer_entered.set_value();
            released.wait();
        }
    });
    assert(recorder.submit(900, std::string("a\0b", 3)));
    assert(writer_entered.get_future().wait_for(std::chrono::seconds(5)) == std::future_status::ready);
    assert(recorder.submit(100, "second"));
    assert(!recorder.submit(0, "full"));
    release_writer.set_value();
    recorder.close();
    recorder.close();
    const auto written = recorder.snapshot();
    assert(written.size() == 2);
    assert(written[0].sequence == 0 && written[0].timestamp == 900 && written[0].bytes == std::string("a\0b", 3));
    assert(written[1].sequence == 1 && written[1].timestamp == 100 && written[1].bytes == "second");
    assert(recorder.undelivered().empty());
    assert(!recorder.failed() && !recorder.submit(0, "closed"));
}
