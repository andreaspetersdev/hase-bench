#include "template_engine/template.hpp"
#include "template_engine/template_cache.hpp"

#include <cassert>
#include <string>
#include <utility>
#include <vector>

using namespace template_engine;

namespace {
std::string long_template(std::string_view prefix) {
    return std::string(prefix) + " 012345678901234567890123456789 {name} -- tail";
}
}

int main() {
    // A temporary source must not leak through the owning parser boundary.
    auto compiled = CompiledTemplate::compile(long_template("temporary"));
    std::vector<std::string> churn(200, std::string(256, 'x'));
    assert(compiled.render("Ada") == "temporary 012345678901234567890123456789 Ada -- tail");

    // Owning values must re-establish their internal state across all ordinary
    // value operations, including an assignment after a prior unrelated value.
    auto copy = compiled;
    auto moved = std::move(compiled);
    CompiledTemplate assigned = CompiledTemplate::compile("old {name}");
    assigned = copy;
    CompiledTemplate move_assigned = CompiledTemplate::compile("other {name}");
    move_assigned = std::move(moved);
    assert(copy.render("B") == "temporary 012345678901234567890123456789 B -- tail");
    assert(assigned.render("C") == "temporary 012345678901234567890123456789 C -- tail");
    assert(move_assigned.render("D") == "temporary 012345678901234567890123456789 D -- tail");

    // Reallocation and replacement exercise the cache's second ownership boundary.
    TemplateCache cache;
    for (int i = 0; i != 80; ++i) {
        cache.insert("key-" + std::to_string(i), long_template("item" + std::to_string(i)));
    }
    assert(cache.render("key-0", "X") == "item0 012345678901234567890123456789 X -- tail");
    cache.insert("key-0", long_template("replacement"));
    assert(cache.render("key-0", "Y") == "replacement 012345678901234567890123456789 Y -- tail");

    // A borrowing view remains zero-copy: changed external storage is observed.
    std::string external = "before {name}";
    auto view = TemplateView::from_external(external);
    external.replace(0, 7, "after! ");
    assert(view.render("Z") == "after! Z");
    assert(view.source().data() == external.data());

    auto literal = CompiledTemplate::compile("x {other} {name} {");
    assert(literal.render("N") == "x {other} N {");
}
