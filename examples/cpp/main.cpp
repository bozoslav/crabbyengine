#include "crabby.hpp"
#include <iostream>
#include <optional>
#include <cmath>
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
    std::optional<crabby::component<Position>> sim_position;
    std::optional<crabby::component<Velocity>> sim_velocity;
    crabby::entity agent = 0;
    auto simulation = crabby::world::run_headless(60, 60,
        [&](crabby::world_view& w) {
            sim_position = w.component<Position>("Position");
            sim_velocity = w.component<Velocity>("Velocity");
            agent = w.spawn(sim_position->with(Position{0, 0}),
                            sim_velocity->with(Velocity{1, 2}));
            return true;
        },
        [&](crabby::world_view& w, std::uint64_t, float dt) {
            w.query([dt](auto chunk) {
                auto positions = chunk.template column<0>();
                auto velocities = chunk.template column<1>();
                for (std::size_t i = 0; i < chunk.size(); ++i) {
                    positions[i].x += velocities[i].x * dt;
                    positions[i].y += velocities[i].y * dt;
                }
            }, *sim_position, *sim_velocity);
            return true;
        });
    auto final_position = simulation.get(agent, *sim_position);
    // ANCHOR_END: cpp_headless
    std::cout << "After 60 ticks: " << final_position.x << ", " << final_position.y << '\n';
    return simulation.size() == 1 && result.x == 1 && result.y == 2 &&
        std::abs(final_position.x - 1.0f) < 0.0001f &&
        std::abs(final_position.y - 2.0f) < 0.0001f ? 0 : 1;
}
