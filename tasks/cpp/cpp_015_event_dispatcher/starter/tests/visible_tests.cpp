#include "event_dispatcher.hpp"
#include <cassert>
struct Ping { int value; }; struct Pong { int value; };
int main() { Dispatcher d; int total = 0; auto a = d.subscribe<Ping>([&](const Ping& p) { total += p.value; }); auto b = d.subscribe<Ping>([&](const Ping&) { total += 10; }); d.emit(Ping{2}); assert(total == 12); a.reset(); d.emit(Ping{3}); assert(total == 22); d.emit(Pong{9}); assert(total == 22); }
