#include "config/value.hpp"

#include <cassert>
#include <cstdint>
#include <string>

using hase::config::Value;
using hase::config::merge;

int main() {
    assert(std::get<std::int64_t>(merge(Value(1), Value(2)).storage()) == 2);
    assert(merge(Value("old"), Value(nullptr)).is_null());

    Value::Object base{
        {"host", "localhost"},
        {"port", 8080},
        {"flags", Value::Array{Value("a"), Value("b")}},
    };
    Value::Object override_value{
        {"port", 9090},
        {"flags", Value::Array{Value("production")}},
        {"enabled", true},
    };
    const Value result = merge(base, override_value);
    const auto& object = result.as_object();
    assert(std::get<std::string>(object.at("host").storage()) == "localhost");
    assert(std::get<std::int64_t>(object.at("port").storage()) == 9090);
    assert(std::get<bool>(object.at("enabled").storage()));
    assert(object.at("flags").as_array().size() == 1);
    assert(std::get<std::string>(object.at("flags").as_array().front().storage()) == "production");
}
