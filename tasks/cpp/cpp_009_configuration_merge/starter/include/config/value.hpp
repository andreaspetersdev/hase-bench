#pragma once

#include <cstdint>
#include <map>
#include <string>
#include <variant>
#include <vector>

namespace hase::config {

class Value {
public:
    using Array = std::vector<Value>;
    using Object = std::map<std::string, Value, std::less<>>;
    using Storage = std::variant<std::nullptr_t, bool, std::int64_t, double, std::string, Array, Object>;

    Value() noexcept;
    Value(std::nullptr_t) noexcept;
    Value(bool value) noexcept;
    Value(std::int64_t value) noexcept;
    Value(int value) noexcept;
    Value(double value) noexcept;
    Value(const char* value);
    Value(std::string value);
    Value(Array value);
    Value(Object value);

    [[nodiscard]] const Storage& storage() const noexcept;
    [[nodiscard]] bool is_null() const noexcept;
    [[nodiscard]] bool is_object() const noexcept;
    [[nodiscard]] bool is_array() const noexcept;

    [[nodiscard]] const Object& as_object() const;
    [[nodiscard]] Object& as_object();
    [[nodiscard]] const Array& as_array() const;
    [[nodiscard]] Array& as_array();

private:
    Storage storage_;
};

[[nodiscard]] Value merge(const Value& base, const Value& override_value);

} // namespace hase::config
