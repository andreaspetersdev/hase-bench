#include <fstream>
#include <iostream>
#include <iterator>
#include <regex>
#include <string>

namespace {

std::string without_comments(std::string source) {
  source = std::regex_replace(source, std::regex(R"(/\*[\s\S]*?\*/)"), "");
  return std::regex_replace(source, std::regex(R"(//[^\r\n]*)"), "");
}

}  // namespace

int main(int argc, char** argv) {
  if (argc != 2) return 1;
  std::ifstream input(argv[1]);
  const std::string source{std::istreambuf_iterator<char>{input}, {}};
  if (!input && !input.eof()) {
    std::cerr << "could not read public ring-buffer header\n";
    return 1;
  }

  const auto code = without_comments(source);
  if (std::regex_search(code, std::regex(R"(std::\s*::\s*(mutex|lock_guard|unique_lock|scoped_lock))"))) {
    std::cerr << "SPSC operations must not be implemented with standard mutex locking\n";
    return 2;
  }

  const std::regex order(R"(memory_order_(relaxed|consume|acquire|release|acq_rel|seq_cst))");
  bool has_order = false;
  bool has_acquire_release = false;
  for (std::sregex_iterator it(code.begin(), code.end(), order), end; it != end; ++it) {
    has_order = true;
    const auto value = (*it)[1].str();
    has_acquire_release = has_acquire_release || value == "acquire" || value == "release" || value == "acq_rel";
  }
  if (!has_order || !has_acquire_release) {
    std::cerr << "SPSC publication requires explicit acquire/release synchronization\n";
    return 3;
  }
}
