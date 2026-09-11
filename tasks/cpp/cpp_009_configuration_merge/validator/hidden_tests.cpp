#include "config/value.hpp"

#include <cassert>
#include <cstdint>
#include <string>
#include <variant>

using hase::config::Value;
using hase::config::merge;

namespace {
std::int64_t integer(const Value& value) {
    return std::get<std::int64_t>(value.storage());
}

void recursive_merge_and_type_replacement() {
    Value::Object base{
        {"server", Value::Object{
            {"address", "127.0.0.1"},
            {"tls", Value::Object{{"enabled", false}, {"port", 443}}},
        }},
        {"retries", 3},
        {"mode", "development"},
    };
    Value::Object overrides{
        {"server", Value::Object{
            {"tls", Value::Object{{"enabled", true}, {"certificate", "server.pem"}}},
        }},
        {"retries", Value::Array{Value(1), Value(2)}},
        {"mode", nullptr},
    };

    const Value merged = merge(base, overrides);
    const auto& root = merged.as_object();
    const auto& server = root.at("server").as_object();
    const auto& tls = server.at("tls").as_object();
    assert(std::get<std::string>(server.at("address").storage()) == "127.0.0.1");
    assert(std::get<bool>(tls.at("enabled").storage()));
    assert(integer(tls.at("port")) == 443);
    assert(std::get<std::string>(tls.at("certificate").storage()) == "server.pem");
    assert(root.at("retries").is_array());
    assert(root.at("retries").as_array().size() == 2);
    assert(root.at("mode").is_null());
}

void scalar_replacement_preserves_the_selected_variant_alternative() {
    const Value floating_value = merge(Value(std::int64_t{7}), Value(3.5));
    assert(std::holds_alternative<double>(floating_value.storage()));
    assert(std::get<double>(floating_value.storage()) == 3.5);

    const Value integer_value = merge(Value(1.5), Value(std::int64_t{9}));
    assert(std::holds_alternative<std::int64_t>(integer_value.storage()));
    assert(integer(integer_value) == 9);

    const Value text = merge(Value(false), Value("false"));
    assert(std::holds_alternative<std::string>(text.storage()));
    assert(std::get<std::string>(text.storage()) == "false");
}

void absent_and_null_are_distinct() {
    const Value base(Value::Object{{"keep", 7}, {"erase_me", "not actually erased"}});
    const Value overrides(Value::Object{{"erase_me", nullptr}, {"new", false}});
    const Value merged = merge(base, overrides);
    const auto& result = merged.as_object();
    assert(result.contains("keep"));
    assert(result.contains("erase_me"));
    assert(result.at("erase_me").is_null());
    assert(result.contains("new"));
    assert(!std::get<bool>(result.at("new").storage()));
}

void result_and_inputs_have_independent_recursive_storage() {
    Value base(Value::Object{{"nested", Value::Object{{"from_base", Value::Array{Value(10)}}}}});
    Value overrides(Value::Object{{"nested", Value::Object{{"from_override", Value::Array{Value(20)}}}}});
    Value result = merge(base, overrides);

    result.as_object().at("nested").as_object().at("from_base").as_array().front() = Value(100);
    result.as_object().at("nested").as_object().at("from_override").as_array().front() = Value(200);
    assert(integer(base.as_object().at("nested").as_object().at("from_base").as_array().front()) == 10);
    assert(integer(overrides.as_object().at("nested").as_object().at("from_override").as_array().front()) == 20);

    base.as_object().at("nested").as_object().at("from_base").as_array().front() = Value(300);
    overrides.as_object().at("nested").as_object().at("from_override").as_array().front() = Value(400);
    assert(integer(result.as_object().at("nested").as_object().at("from_base").as_array().front()) == 100);
    assert(integer(result.as_object().at("nested").as_object().at("from_override").as_array().front()) == 200);
}

void deeply_nested_objects_merge_at_every_level() {
    Value base(Value::Object{{"a", Value::Object{{"b", Value::Object{{"c", Value::Object{{"left", 1}, {"same", 2}}}}}}}});
    Value overrides(Value::Object{{"a", Value::Object{{"b", Value::Object{{"c", Value::Object{{"same", 9}, {"right", 3}}}}}}}});
    const Value merged = merge(base, overrides);
    const auto& c = merged.as_object().at("a").as_object().at("b").as_object().at("c").as_object();
    assert(integer(c.at("left")) == 1);
    assert(integer(c.at("same")) == 9);
    assert(integer(c.at("right")) == 3);
}

void falsey_values_and_arrays_are_exact_replacements() {
    const Value base(Value::Object{
        {"enabled", true},
        {"retries", 7},
        {"label", "configured"},
        {"items", Value::Array{Value(1), Value(2)}},
    });
    const Value overrides(Value::Object{
        {"enabled", false},
        {"retries", 0},
        {"label", ""},
        {"items", Value::Array{}},
    });

    const Value merged = merge(base, overrides);
    const auto& object = merged.as_object();
    assert(!std::get<bool>(object.at("enabled").storage()));
    assert(integer(object.at("retries")) == 0);
    assert(std::get<std::string>(object.at("label").storage()).empty());
    assert(object.at("items").is_array());
    assert(object.at("items").as_array().empty());

    const Value replaced_array = merge(
        Value::Array{Value::Object{{"retained", 1}}, Value(2)},
        Value::Array{Value::Object{{"replacement", 3}}});
    assert(replaced_array.is_array());
    assert(replaced_array.as_array().size() == 1);
    const auto& element = replaced_array.as_array().front().as_object();
    assert(!element.contains("retained"));
    assert(integer(element.at("replacement")) == 3);
}
} // namespace

int main() {
    recursive_merge_and_type_replacement();
    scalar_replacement_preserves_the_selected_variant_alternative();
    absent_and_null_are_distinct();
    result_and_inputs_have_independent_recursive_storage();
    deeply_nested_objects_merge_at_every_level();
    falsey_values_and_arrays_are_exact_replacements();
}
