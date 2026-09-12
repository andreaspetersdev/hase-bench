#include "audit.hpp"
#include <cassert>
int main(){Record r{{{"user","ana"},{"token","secret"}}}; assert(make_renderer("text")->render(r)=="user=ana token=secret"); assert(make_renderer("json",{"token"})->render(r)=="{\"user\":\"ana\",\"token\":\"***\"}");}
