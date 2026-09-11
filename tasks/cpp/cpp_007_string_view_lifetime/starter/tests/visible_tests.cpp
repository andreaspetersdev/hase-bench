#include "template_engine/template.hpp"
#include "template_engine/template_cache.hpp"

#include <cassert>
#include <stdexcept>
#include <string>

using namespace template_engine;

int main() {
    auto greeting = CompiledTemplate::compile("Hello, {name}!");
    assert(greeting.render("Ada") == "Hello, Ada!");
    assert(greeting.source() == "Hello, {name}!");

    TemplateCache cache;
    cache.insert("one", "[{name}]");
    cache.insert("two", "welcome {name}");
    assert(cache.render("one", "Lin") == "[Lin]");
    assert(cache.render("two", "Lin") == "welcome Lin");
    cache.insert("one", "again: {name}");
    assert(cache.render("one", "Lin") == "again: Lin");
    assert(cache.find("missing") == nullptr);
    try {
        (void)cache.render("missing", "Lin");
        assert(false);
    } catch (const std::out_of_range&) {
    }

    std::string external = "external {name}";
    auto view = TemplateView::from_external(external);
    assert(view.render("source") == "external source");
}
