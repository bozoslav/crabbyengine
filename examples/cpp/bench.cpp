#include "crabby.hpp"
#include <algorithm>
#include <array>
#include <chrono>
#include <iostream>
struct Pos { float x, y; };
struct Vel { float x, y; };
int main() {
    crabby::world world;
    auto p = world.component<Pos>("Pos");
    auto v = world.component<Vel>("Vel");
    for (std::size_t i = 0; i < 1000000; ++i) world.spawn(p.with(Pos{0, 0}), v.with(Vel{1, 2}));
    auto update = [&] {
        world.query([](auto chunk) {
            auto ps = chunk.template column<0>();
            auto vs = chunk.template column<1>();
            for (std::size_t i = 0; i < chunk.size(); ++i) { ps[i].x += vs[i].x; ps[i].y += vs[i].y; }
        }, p, v);
    };
    for (int i = 0; i < 10; ++i) update();
    std::array<double, 200> timings{};
    for (auto& time : timings) {
        auto start = std::chrono::steady_clock::now(); update();
        time = std::chrono::duration<double, std::micro>(std::chrono::steady_clock::now() - start).count();
    }
    std::sort(timings.begin(), timings.end());
    double checksum = 0;
    world.query([&](auto chunk) { auto ps = chunk.template column<0>(); for (std::size_t i = 0; i < chunk.size(); ++i) checksum += double(ps[i].x) + double(ps[i].y); }, p);
    std::cout.precision(12);
    std::cout << "cpp/dynamic entities=1000000 warmup=10 iterations=200 median_us=" << (timings[99] + timings[100]) / 2 << " checksum=" << checksum << '\n';
    return checksum == 630000000 ? 0 : 1;
}
