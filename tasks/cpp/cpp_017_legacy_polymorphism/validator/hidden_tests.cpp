#include "audit.hpp"
#include <cassert>
#include <stdexcept>
int main(){Record r{{{"password","p"},{"name","A\\\"B"},{"password","q"}}};assert(make_renderer("text",{"password"})->render(r)=="password=*** name=A\\\"B password=***");assert(make_renderer("json")->render(r)=="{\"password\":\"p\",\"name\":\"A\\\\\\\"B\",\"password\":\"q\"}");bool bad=false;try{make_renderer("xml");}catch(const std::invalid_argument&){bad=true;}assert(bad);}
