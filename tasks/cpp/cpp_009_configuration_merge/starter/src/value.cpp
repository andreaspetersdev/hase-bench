#include "config/value.hpp"

#include <utility>

namespace hase::config {

Value::Value() noexcept : storage_(nullptr) {}
Value::Value(std::nullptr_t) noexcept : storage_(nullptr) {}
Value::Value(bool value) noexcept : storage_(value) {}
Value::Value(std::int64_t value) noexcept : storage_(value) {}
Value::Value(int value) noexcept : storage_(static_cast<std::int64_t>(value)) {}
Value::Value(double value) noexcept : storage_(value) {}
Value::Value(const char* value) : storage_(std::string(value)) {}
Value::Value(std::string value) : storage_(std::move(value)) {}
Value::Value(Array value) : storage_(std::move(value)) {}
Value::Value(Object value) : storage_(std::move(value)) {}

const Value::Storage& Value::storage() const noexcept { return storage_; }
bool Value::is_null() const noexcept { return std::holds_alternative<std::nullptr_t>(storage_); }
bool Value::is_object() const noexcept { return std::holds_alternative<Object>(storage_); }
bool Value::is_array() const noexcept { return std::holds_alternative<Array>(storage_); }
const Value::Object& Value::as_object() const { return std::get<Object>(storage_); }
Value::Object& Value::as_object() { return std::get<Object>(storage_); }
const Value::Array& Value::as_array() const { return std::get<Array>(storage_); }
Value::Array& Value::as_array() { return std::get<Array>(storage_); }

Value merge(const Value& base, const Value& override_value) {
    // Replace this starter implementation with the recursive object merge
    // described in TASK.md.
    (void)base;
    return override_value;
}

} // namespace hase::config
