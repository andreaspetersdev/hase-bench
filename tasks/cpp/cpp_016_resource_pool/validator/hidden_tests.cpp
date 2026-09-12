#include "resource_pool.hpp"
#include <cassert>
#include <stdexcept>
int main(){bool threw=false;try{ResourcePool x(0);}catch(const std::invalid_argument&){threw=true;}assert(threw); ResourcePool p(2); auto a=p.acquire(); auto b=p.acquire(); a=std::move(b); assert(!*b&&p.available()==1); a->reset(); assert(p.available()==2); auto h=p.acquire(); ResourcePool q(std::move(p)); h->reset(); assert(q.available()==2); auto stale=q.acquire(); ResourcePool replacement(1); replacement=std::move(q); stale->reset(); assert(replacement.available()==2); ResourcePool::Handle orphan; {ResourcePool short_lived(1);orphan=std::move(*short_lived.acquire());} assert(!orphan);orphan.reset();}
