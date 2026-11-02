// Boids in C++: the same flocking as examples/boids.rs, on all CPU cores.
//
//   ./crabby_cpp_boids                       (window, 20k boids)
//   BOIDS=100000 ./crabby_cpp_boids
//   HEADLESS=1 BOIDS=1000000 TICKS=300 ./crabby_cpp_boids
//
// Keys: Space pause, Period single step, 1-4 speed (1x/4x/16x/64x), Escape quit.
#include "crabby.hpp"
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

using crabby::vec2;

struct Vel { vec2 v; };

constexpr float RADIUS = 24.0f;
constexpr float MAX_SPEED = 140.0f;
constexpr float MIN_SPEED = 60.0f;
constexpr std::uint64_t SPATIAL_SORT_EVERY = 32;

static double env_number(const char* name, double fallback) {
    const char* value = std::getenv(name);
    return value ? std::atof(value) : fallback;
}

// ANCHOR: cpp_flocking
// The flocking system. Its grid stores each boid's index into `velocities`,
// so neighbors' velocities can be read while the query borrows the world.
struct Flocking {
    explicit Flocking(vec2 world_size) : bounds(world_size) {}
    vec2 bounds;
    crabby::component<crabby::transform2d> transforms;
    crabby::component<Vel> vel;
    std::shared_ptr<crabby::grid> grid = std::make_shared<crabby::grid>(RADIUS);
    std::shared_ptr<std::vector<crabby::grid_item>> items = std::make_shared<std::vector<crabby::grid_item>>();
    std::shared_ptr<std::vector<vec2>> velocities = std::make_shared<std::vector<vec2>>();

    void operator()(crabby::ctx& ctx) {
        auto world = ctx.world();
        const float dt = ctx.dt();
        // Keep boids that are close in space close in memory (cache hits).
        if (ctx.sim_tick() % SPATIAL_SORT_EVERY == 0) world.sort_spatial(RADIUS);
        items->clear();
        velocities->clear();
        world.query([&](auto chunk) {
            auto t = chunk.template column<0>();
            auto v = chunk.template column<1>();
            for (std::size_t i = 0; i < chunk.size(); ++i) {
                items->push_back({chunk.entities()[i], t[i].position, std::uint32_t(velocities->size())});
                velocities->push_back(v[i].v);
            }
        }, transforms, vel);
        grid->rebuild(*items);

        const auto view = grid->view();
        const auto& vels = *velocities;
        const vec2 b = bounds;
        world.query_par([&](auto chunk) {
            auto t = chunk.template column<0>();
            auto v = chunk.template column<1>();
            for (std::size_t i = 0; i < chunk.size(); ++i) {
                const CrabbyEntity me = chunk.entities()[i];
                vec2 away{0, 0}, heading{0, 0}, center{0, 0};
                float n = 0;
                view.for_each_in_radius(t[i].position, RADIUS, [&](const crabby::grid_item& o) {
                    if (o.entity.index == me.index && o.entity.generation == me.generation) return;
                    const vec2 d = t[i].position - o.position;
                    away += d / std::max(crabby::length_squared(d), 1.0f);
                    heading += vels[o.data];
                    center += o.position;
                    n += 1;
                });
                vec2 steer = away * 900.0f;
                if (n > 0) steer += (heading / n - v[i].v) * 1.5f + (center / n - t[i].position) * 1.2f;
                const vec2 next = v[i].v + steer * dt;
                const float speed = std::min(std::max(crabby::length(next), MIN_SPEED), MAX_SPEED);
                v[i].v = crabby::normalize_or(next, {1, 0}) * speed;
                t[i].position = crabby::rem_euclid(t[i].position + v[i].v * dt, b);
                t[i].rotation = crabby::to_angle(v[i].v);
            }
        }, transforms, vel);
    }
};
// ANCHOR_END: cpp_flocking

static void controls(crabby::ctx& ctx) {
    if (ctx.key_pressed(CRABBY_KEY_SPACE)) { if (ctx.is_paused()) ctx.resume(); else ctx.pause(); }
    if (ctx.key_pressed(CRABBY_KEY_PERIOD)) ctx.step();
    const std::pair<crabby::key, float> speeds[] = {
        {CRABBY_KEY_NUM_1, 1}, {CRABBY_KEY_NUM_2, 4}, {CRABBY_KEY_NUM_3, 16}, {CRABBY_KEY_NUM_4, 64}};
    for (auto [k, speed] : speeds) if (ctx.key_pressed(k)) ctx.set_sim_speed(speed);
    if (ctx.key_pressed(CRABBY_KEY_ESCAPE)) ctx.quit();
}

static void spawn_boids(crabby::ctx& ctx, Flocking& f, std::size_t n) {
    auto world = ctx.world();
    f.transforms = world.transforms();
    f.vel = world.component<Vel>("Vel");
    auto sprites = world.sprites();
    const crabby::texture tex = ctx.load_texture("crab.png");
    std::uint64_t s = 0x9E3779B97F4A7C15ull;
    auto rand = [&] {
        s ^= s << 13; s ^= s >> 7; s ^= s << 17;
        return float(s >> 40) / float(1u << 24);
    };
    for (std::size_t i = 0; i < n; ++i) {
        const vec2 pos{rand() * f.bounds.x, rand() * f.bounds.y};
        const vec2 vel = crabby::from_angle(rand() * 6.2831853f) * MIN_SPEED;
        const crabby::color tint = crabby::rgb(0.5f + rand() * 0.5f, 0.7f + rand() * 0.3f, 0.9f);
        world.spawn(f.transforms.with(crabby::transform_at(pos, 0, {0.5f, 0.5f})),
                    sprites.with(crabby::sprite_of(tex, tint)),
                    f.vel.with(Vel{vel}));
    }
}

int main() {
    const auto n = std::size_t(env_number("BOIDS", 20000));
    if (env_number("HEADLESS", 0) != 0) {
        // Keep density like the windowed version: scale the world with the count.
        const float side = std::sqrt(float(n) / 20000.0f) * 1280.0f;
        Flocking flocking({side, side});
        const auto ticks = std::uint64_t(env_number("TICKS", 300));
        auto start = std::chrono::steady_clock::now();
        auto world = crabby::game("boids", 1, 1)
            .sim_system([&](crabby::ctx& ctx) { flocking(ctx); }, "flocking")
            .run_headless(ticks, [&](crabby::ctx& ctx) {
                spawn_boids(ctx, flocking, n);
                start = std::chrono::steady_clock::now();
            });
        const double secs = std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
        std::printf("%zu boids, %llu ticks: %.2f ms/tick (%.0f ticks/s)\n", world.size(),
                    (unsigned long long)ticks, secs * 1e3 / double(ticks), double(ticks) / secs);
        return 0;
    }
    Flocking flocking({1280, 720});
    double next = 1.0;
    crabby::game("Crabby Engine: C++ boids", 1280, 720)
        .sim_system([&](crabby::ctx& ctx) { flocking(ctx); }, "flocking")
        .system(controls, "controls")
        .system([&](crabby::ctx& ctx) {
            if (ctx.time() >= next) {
                next += 1.0;
                std::printf("boids: %zu  fps: %.0f  tick: %llu\n", ctx.world().size(), ctx.fps(),
                            (unsigned long long)ctx.sim_tick());
            }
        }, "stats")
        .run([&](crabby::ctx& ctx) { spawn_boids(ctx, flocking, n); });
}
