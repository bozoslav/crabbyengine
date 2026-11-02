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
    auto position = world.component<Position>("Position");
    auto velocity = world.component<Velocity>("Velocity");
    auto player = world.spawn(position.with(Position{0, 0}), velocity.with(Velocity{1, 2}));
    world.query([](auto chunk) {
        auto positions = chunk.template column<0>();
        auto velocities = chunk.template column<1>();
        for (std::size_t i = 0; i < chunk.size(); ++i) {
            positions[i].x += velocities[i].x;
            positions[i].y += velocities[i].y;
        }
    }, position, velocity);
    auto result = world.get(player, position);
    // ANCHOR_END: cpp_world
    std::cout << "Position: " << result.x << ", " << result.y << '\n';
    // ANCHOR: cpp_headless
    crabby::component<Position> pos;
    crabby::component<Velocity> vel;
    crabby::entity agent = 0;
    auto simulation = crabby::game("headless", 1, 1)
        .sim_system([&](crabby::ctx& ctx) {
            const float dt = ctx.dt();
            ctx.world().query([dt](auto chunk) {
                auto positions = chunk.template column<0>();
                auto velocities = chunk.template column<1>();
                for (std::size_t i = 0; i < chunk.size(); ++i) {
                    positions[i].x += velocities[i].x * dt;
                    positions[i].y += velocities[i].y * dt;
                }
            }, pos, vel);
        }, "movement")
        .run_headless(60, [&](crabby::ctx& ctx) {
            auto w = ctx.world();
            pos = w.component<Position>("Position");
            vel = w.component<Velocity>("Velocity");
            agent = w.spawn(pos.with(Position{0, 0}), vel.with(Velocity{1, 2}));
        });
    auto final_position = simulation.get(agent, pos);
    // ANCHOR_END: cpp_headless
    std::cout << "After 60 ticks: " << final_position.x << ", " << final_position.y << '\n';
    return simulation.size() == 1 && result.x == 1 && result.y == 2 &&
        std::abs(final_position.x - 1.0f) < 0.0001f &&
        std::abs(final_position.y - 2.0f) < 0.0001f ? 0 : 1;
}
