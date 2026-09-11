#include "lru_cache.hpp"
#include <iostream>
#include <memory>
#include <string>
#include <type_traits>
int main() {
 static_assert(!std::is_copy_constructible_v<hase::LruCache<int, int>>);
 static_assert(!std::is_copy_assignable_v<hase::LruCache<int, int>>);
 static_assert(!std::is_move_constructible_v<hase::LruCache<int, int>>);
 static_assert(!std::is_move_assignable_v<hase::LruCache<int, int>>);
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
 move_only.put(1, std::make_unique<int>(6));
 if (!move_only.get(1) || **move_only.get(1) != 6) return 6;
 hase::LruCache<std::string, int> strings(2);
 std::string first = "first";
 strings.put(first, 1);
 strings.put(std::string{"second"}, 2);
 strings.put(std::string{"first"}, 3);
 if (!strings.get("first") || *strings.get("first") != 3 || !strings.get("second")) return 7;
}
