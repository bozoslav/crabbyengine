#include "crabby.hpp"
#include <cmath>
#include <iostream>
// ANCHOR: cpp_components
struct Position { float x, y; };
struct Velocity { float x, y; };
// ANCHOR_END: cpp_components
int main() {
    // ANCHOR: cpp_world
    crabby::world world;
    auto player = world.spawn(Position{0, 0}, Velocity{1, 2});
    world.each([](Position& p, const Velocity& v) {
        p.x += v.x;
        p.y += v.y;
    });
    auto result = world.get<Position>(player);
    // ANCHOR_END: cpp_world
    std::cout << "Position: " << result.x << ", " << result.y << '\n';
    // ANCHOR: cpp_headless
    crabby::entity agent = 0;
    auto simulation = crabby::game("headless", 1, 1)
        .sim_system([](crabby::ctx& ctx) {
            const float dt = ctx.dt();
            ctx.world().each([dt](Position& p, const Velocity& v) {
                p.x += v.x * dt;
                p.y += v.y * dt;
            });
        }, "movement")
        .run_headless(60, [&](crabby::ctx& ctx) {
            agent = ctx.world().spawn(Position{0, 0}, Velocity{1, 2});
        });
    auto final_position = simulation.get<Position>(agent);
    // ANCHOR_END: cpp_headless
    std::cout << "After 60 ticks: " << final_position.x << ", " << final_position.y << '\n';
    return simulation.size() == 1 && result.x == 1 && result.y == 2 &&
        std::abs(final_position.x - 1.0f) < 0.0001f &&
        std::abs(final_position.y - 2.0f) < 0.0001f ? 0 : 1;
}
