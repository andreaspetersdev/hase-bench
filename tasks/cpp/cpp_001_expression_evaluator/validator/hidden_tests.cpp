#include "expression.hpp"
#include <cmath>
#include <iostream>
namespace { bool equal(std::string_view text, double wanted) { auto got=hase::evaluate(text); return got && std::abs(*got.value-wanted)<1e-10; } }
int main() {
  for (auto [text, wanted] : {std::pair{"(2 + 3) * 4",20.0}, {"-5 + 2",-3.0}, {"3.5 * 2",7.0}, {"10 / 4",2.5}, {"--3",3.0}, {"2*-3",-6.0}, {"8/2/2",2.0}, {"  +.5 + 1.5 ",2.0}})
    if (!equal(text,wanted)) { std::cerr << "incorrect expression: " << text << '\n'; return 1; }
  for (auto text : {"", "1+", "()", "1 2", "(1+2", "1/(2-2)", "hello"})
    if (hase::evaluate(text)) { std::cerr << "accepted invalid expression: " << text << '\n'; return 1; }
}
