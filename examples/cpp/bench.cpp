// 1M entities, position += velocity: the same loop written with each() and
// with query() columns, which must be as fast as each other.
#include "crabby.hpp"
#include <algorithm>
#include <array>
#include <chrono>
#include <iostream>
struct Pos { float x, y; };
struct Vel { float x, y; };

template<class F> double median_us(F update) {
    for (int i = 0; i < 10; ++i) update();
    std::array<double, 200> timings{};
    for (auto& time : timings) {
        auto start = std::chrono::steady_clock::now(); update();
        time = std::chrono::duration<double, std::micro>(std::chrono::steady_clock::now() - start).count();
    }
    std::sort(timings.begin(), timings.end());
    return (timings[99] + timings[100]) / 2;
}

int main() {
    crabby::world world;
    for (std::size_t i = 0; i < 1000000; ++i) world.spawn(Pos{0, 0}, Vel{1, 2});
    const double each_us = median_us([&] {
        world.each([](Pos& p, const Vel& v) { p.x += v.x; p.y += v.y; });
    });
    const double query_us = median_us([&] {
        world.query<Pos, Vel>([](auto chunk) {
            auto [ps, vs] = chunk.columns();
            for (std::size_t i = 0; i < chunk.size(); ++i) { ps[i].x += vs[i].x; ps[i].y += vs[i].y; }
        });
    });
    double checksum = 0;
    world.each([&](const Pos& p) { checksum += double(p.x) + double(p.y); });
    std::cout.precision(12);
    std::cout << "cpp/each entities=1000000 warmup=10 iterations=200 median_us=" << each_us << '\n'
              << "cpp/query entities=1000000 warmup=10 iterations=200 median_us=" << query_us << " checksum=" << checksum << '\n';
    // Two runs of 210 updates each, every one adding (1, 2) to all 1M entities.
    return checksum == 1260000000 ? 0 : 1;
}
