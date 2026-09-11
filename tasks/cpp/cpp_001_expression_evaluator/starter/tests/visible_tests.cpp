#include "expression.hpp"
#include <cmath>
#include <iostream>
int main() {
  auto value = hase::evaluate("2 + 3 * 4");
  if (!value || std::abs(*value.value - 14.0) > 1e-12) { std::cerr << "precedence failed\n"; return 1; }
  value = hase::evaluate("-(2\t+\n3) * +.5");
  if (!value || std::abs(*value.value + 2.5) > 1e-12) { std::cerr << "whitespace or unary parentheses failed\n"; return 2; }
  if (hase::evaluate("1 / 0")) { std::cerr << "division by zero accepted\n"; return 1; }
  if (hase::evaluate("1 / -0.0")) { std::cerr << "signed division by zero accepted\n"; return 3; }
}
