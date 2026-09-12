#include "resource_pool.hpp"
#include <cassert>
int main(){ResourcePool p(2); auto a=p.acquire(),b=p.acquire(); assert(a&&b&&!p.acquire()&&p.available()==0); const int released=a->value(); a->reset(); assert(p.available()==1); auto c=p.acquire(); assert(c&&c->value()==released);}
