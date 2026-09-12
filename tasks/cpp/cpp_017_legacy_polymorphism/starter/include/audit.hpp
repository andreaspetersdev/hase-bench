#pragma once
#include <memory>
#include <string>
#include <string_view>
#include <utility>
#include <vector>
struct Record { std::vector<std::pair<std::string,std::string>> fields; };
class Renderer { public: virtual ~Renderer()=default; virtual std::string render(const Record&) const=0; };
std::unique_ptr<Renderer> make_renderer(std::string_view format, std::vector<std::string> sensitive_keys = {});
