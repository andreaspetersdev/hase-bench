#include "lru_cache.hpp"
#include <iostream>
int main() { hase::LruCache<int,int> cache(2); cache.put(1, 10); if (!cache.get(1) || *cache.get(1) != 10) { std::cerr << "insert/get failed\n"; return 1; } }
