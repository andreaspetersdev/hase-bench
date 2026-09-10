#include "lru_cache.hpp"
#include <iostream>
#include <memory>
#include <string>
int main() {
 hase::LruCache<int, std::string> one(1);
 one.put(1, "first"); one.put(1, "replacement");
 if (one.size() != 1 || !one.get(1) || *one.get(1) != "replacement") return 1;
 one.put(2, "second");
 if (one.get(1) || !one.get(2) || *one.get(2) != "second") return 2;
 hase::LruCache<int, int> c(3);
 c.put(1, 10); c.put(2, 20); c.put(3, 30); (void)c.get(1); c.put(4, 40);
 if (c.get(2)) return 3;
 c.put(5, 50);
 if (c.get(3) || !c.get(1) || !c.get(4) || !c.get(5)) return 4;
 hase::LruCache<int, std::unique_ptr<int>> move_only(1);
 move_only.put(1, std::make_unique<int>(5));
 if (!move_only.get(1) || **move_only.get(1) != 5) return 5;
}
