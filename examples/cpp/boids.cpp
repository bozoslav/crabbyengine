// Boids in C++: the same flocking as examples/boids.rs, on all CPU cores.
//
//   ./ponko_cpp_boids                       (window, 20k boids)
//   BOIDS=100000 ./ponko_cpp_boids
//   HEADLESS=1 BOIDS=1000000 TICKS=300 ./ponko_cpp_boids
//
// Keys: Space pause, Period single step, 1-4 speed (1x/4x/16x/64x), Escape quit.
#include "ponko.hpp"
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

using ponko::vec2;

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
    std::shared_ptr<ponko::grid> grid = std::make_shared<ponko::grid>(RADIUS);
    std::shared_ptr<std::vector<ponko::grid_item>> items = std::make_shared<std::vector<ponko::grid_item>>();
    std::shared_ptr<std::vector<vec2>> velocities = std::make_shared<std::vector<vec2>>();

    void operator()(ponko::ctx& ctx) {
        auto world = ctx.world();
        const float dt = ctx.dt();
        // Keep boids that are close in space close in memory (cache hits).
        if (ctx.sim_tick() % SPATIAL_SORT_EVERY == 0) world.sort_spatial(RADIUS);
        items->clear();
        velocities->clear();
        world.each([&](ponko::entity e, const ponko::transform2d& t, const Vel& v) {
            items->push_back({ponko::entity_of(e), t.position, std::uint32_t(velocities->size())});
            velocities->push_back(v.v);
        });
        grid->rebuild(*items);

        const auto view = grid->view();
        const auto& vels = *velocities;
        const vec2 b = bounds;
        world.each_par([&](ponko::entity me, ponko::transform2d& t, Vel& v) {
            vec2 away{0, 0}, heading{0, 0}, center{0, 0};
            float n = 0;
            view.for_each_in_radius(t.position, RADIUS, [&](const ponko::grid_item& o) {
                if (ponko::bits(o.entity) == me) return;
                const vec2 d = t.position - o.position;
                away += d / std::max(ponko::length_squared(d), 1.0f);
                heading += vels[o.data];
                center += o.position;
                n += 1;
            });
            vec2 steer = away * 900.0f;
            if (n > 0) steer += (heading / n - v.v) * 1.5f + (center / n - t.position) * 1.2f;
            const vec2 next = v.v + steer * dt;
            const float speed = std::min(std::max(ponko::length(next), MIN_SPEED), MAX_SPEED);
            v.v = ponko::normalize_or(next, {1, 0}) * speed;
            t.position = ponko::rem_euclid(t.position + v.v * dt, b);
            t.rotation = ponko::to_angle(v.v);
        });
    }
};
// ANCHOR_END: cpp_flocking

static void controls(ponko::ctx& ctx) {
    if (ctx.key_pressed(PONKO_KEY_SPACE)) { if (ctx.is_paused()) ctx.resume(); else ctx.pause(); }
    if (ctx.key_pressed(PONKO_KEY_PERIOD)) ctx.step();
    const std::pair<ponko::key, float> speeds[] = {
        {PONKO_KEY_NUM_1, 1}, {PONKO_KEY_NUM_2, 4}, {PONKO_KEY_NUM_3, 16}, {PONKO_KEY_NUM_4, 64}};
    for (auto [k, speed] : speeds) if (ctx.key_pressed(k)) ctx.set_sim_speed(speed);
    if (ctx.key_pressed(PONKO_KEY_ESCAPE)) ctx.quit();
}

static void spawn_boids(ponko::ctx& ctx, const Flocking& f, std::size_t n) {
    auto world = ctx.world();
    world.reflect<Vel>({ponko::field("v", &Vel::v)});   // agents can read and set it
    const ponko::texture tex = ctx.load_texture("crab.png");
    std::uint64_t s = 0x9E3779B97F4A7C15ull;
    auto rand = [&] {
        s ^= s << 13; s ^= s >> 7; s ^= s << 17;
        return float(s >> 40) / float(1u << 24);
    };
    for (std::size_t i = 0; i < n; ++i) {
        const vec2 pos{rand() * f.bounds.x, rand() * f.bounds.y};
        const vec2 vel = ponko::from_angle(rand() * 6.2831853f) * MIN_SPEED;
        const ponko::color tint = ponko::rgb(0.5f + rand() * 0.5f, 0.7f + rand() * 0.3f, 0.9f);
        world.spawn(ponko::transform_at(pos, 0, {0.5f, 0.5f}), ponko::sprite_of(tex, tint), Vel{vel});
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
        auto world = ponko::game("boids", 1, 1)
            .sim_system([&](ponko::ctx& ctx) { flocking(ctx); }, "flocking")
            .run_headless(ticks, [&](ponko::ctx& ctx) {
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
    ponko::game("Ponko: C++ boids", 1280, 720)
        .sim_system([&](ponko::ctx& ctx) { flocking(ctx); }, "flocking")
        .system(controls, "controls")
        .system([&](ponko::ctx& ctx) {
            if (ctx.time() >= next) {
                next += 1.0;
                std::printf("boids: %zu  fps: %.0f  tick: %llu\n", ctx.world().size(), ctx.fps(),
                            (unsigned long long)ctx.sim_tick());
            }
        }, "stats")
        .run([&](ponko::ctx& ctx) { spawn_boids(ctx, flocking, n); });
}
