#include "expression.hpp"
#include <cmath>
#include <iostream>
int main() {
  auto value = hase::evaluate("2 + 3 * 4");
  if (!value || std::abs(*value.value - 14.0) > 1e-12) { std::cerr << "precedence failed\n"; return 1; }
  if (hase::evaluate("1 / 0")) { std::cerr << "division by zero accepted\n"; return 1; }
}
