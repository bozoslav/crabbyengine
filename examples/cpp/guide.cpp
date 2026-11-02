// The C++ code shown in the guide (book/src), next to the Rust versions.
// Everything here compiles in CI; main() runs the headless parts as a test.
#include "crabby.hpp"
#include <cassert>
#include <cmath>
#include <cstdio>
#include <vector>

using namespace crabby;

// ---- Getting started --------------------------------------------------------
// ANCHOR: first_game_full
static void my_game() {
    crabby::texture crab{};
    crabby::vec2 pos{400, 300};
    crabby::game("My Game", 800, 600)
        .system([&](crabby::ctx& ctx) {
            if (ctx.key_down(CRABBY_KEY_RIGHT)) pos.x += 200 * ctx.dt();
            if (ctx.key_down(CRABBY_KEY_LEFT))  pos.x -= 200 * ctx.dt();
            if (ctx.key_pressed(CRABBY_KEY_ESCAPE)) ctx.quit();
            ctx.draw(crab, pos);
        })
        .run([&](crabby::ctx& ctx) { crab = ctx.load_texture("crab.png"); });
}
// ANCHOR_END: first_game_full

// ---- Entities, components, queries -----------------------------------------
// ANCHOR: ecs_components
struct Health { float value; };
struct Enemy {};
// ANCHOR_END: ecs_components

static void ecs(crabby::ctx& ctx) {
    auto world = ctx.world();
    // ANCHOR: ecs_spawn
    auto transforms = world.transforms();
    auto health = world.component<Health>("Health");
    auto enemy = world.component<Enemy>("Enemy");

    crabby::entity e = world.spawn(transforms.with(transform_at({10, 20})),
                                   health.with(Health{100}), enemy.with(Enemy{}));

    // Many at once:
    for (int i = 0; i < 100000; ++i)
        world.spawn(transforms.with(transform_at({float(i), 0})), health.with(Health{10}));
    // ANCHOR_END: ecs_spawn

    // ANCHOR: ecs_one
    Health h = world.get(e, health);
    world.set(e, health, Health{h.value - 5});
    world.set(e, enemy, Enemy{});     // add or replace a component
    world.remove(e, enemy);
    world.despawn(e);
    assert(!world.alive(e));          // old handles stay dead, even if the slot is reused
    // ANCHOR_END: ecs_one

    // ANCHOR: ecs_query
    world.query([](auto chunk) {
        auto t = chunk.template column<0>();
        auto hp = chunk.template column<1>();
        for (std::size_t i = 0; i < chunk.size(); ++i)
            t[i].position.y += hp[i].value;
    }, transforms, health);
    // ANCHOR_END: ecs_query
}

// ANCHOR: ecs_commands
static void die(crabby::ctx& ctx, const crabby::component<Health>& health) {
    ctx.world().query([&](auto chunk) {
        auto hp = chunk.template column<0>();
        for (std::size_t i = 0; i < chunk.size(); ++i)
            if (hp[i].value <= 0) ctx.despawn_later(chunk.entity(i));
    }, health);
}
// ANCHOR_END: ecs_commands

// ---- Systems -----------------------------------------------------------------
[[maybe_unused]] static void steer(crabby::ctx&) {}
[[maybe_unused]] static void movement(crabby::ctx&) {}
[[maybe_unused]] static void camera_system(crabby::ctx&) {}

[[maybe_unused]] static void systems() {
// ANCHOR: systems
crabby::game("Sim", 1280, 720)
    .sim_system(steer, "steer")          // fixed rate: game logic
    .sim_system(movement, "movement")
    .system(camera_system, "camera")     // once per frame: input, camera, UI
    .run([](crabby::ctx&) { /* spawn things */ });
// ANCHOR_END: systems
}

// ANCHOR: stateful
struct Spawner {
    std::uint64_t every;
    int spawned = 0;
    void operator()(crabby::ctx& ctx) {
        if (ctx.sim_tick() % every == 0) {
            ctx.spawn_later(ctx.world().transforms().with(crabby::transform_at({0, 0})));
            ++spawned;
        }
    }
};
// crabby::game("t", 800, 600).sim_system(Spawner{30}, "spawner");
// ANCHOR_END: stateful

[[maybe_unused]] static void replace_builtins() {
    auto my_draw = [](crabby::ctx&) {};
// ANCHOR: remove_system
crabby::game("t", 800, 600)
    .remove_system("crabbyengine::ecs::draw_sprites")   // draw your own way
    .system(my_draw, "my_draw");
// ANCHOR_END: remove_system
}

// ---- The simulation clock ---------------------------------------------------
// ANCHOR: controls
static void controls(crabby::ctx& ctx) {
    if (ctx.key_pressed(CRABBY_KEY_SPACE)) {
        if (ctx.is_paused()) ctx.resume(); else ctx.pause();
    }
    if (ctx.key_pressed(CRABBY_KEY_PERIOD))
        ctx.step();              // one tick while paused
    if (ctx.key_pressed(CRABBY_KEY_NUM_4))
        ctx.set_sim_speed(64);   // fast-forward
}
// crabby::game("Colony", 1280, 720).sim_system(ants).system(controls);
// ANCHOR_END: controls

[[maybe_unused]] static void plan_routes(crabby::ctx&) {}
[[maybe_unused]] static void grow_food(crabby::ctx&) {}
[[maybe_unused]] static void every() {
// ANCHOR: every
crabby::game("Colony", 1280, 720)
    .sim_system(movement, "movement")             // every tick
    .sim_system(plan_routes, "plan_routes", 10)   // 6 times a second
    .sim_system(grow_food, "grow_food", 10);      // also 6/s, but on other ticks
// ANCHOR_END: every
}

// ---- Spatial queries -----------------------------------------------------------
// ANCHOR: spatial_component
struct Neighbors { std::uint32_t count; };
// ANCHOR_END: spatial_component

// ANCHOR: spatial
// crabby::game("Boids", 1280, 720).spatial_grid(24).sim_system(count_neighbors);
static void count_neighbors(crabby::ctx& ctx, const crabby::component<crabby::transform2d>& transforms,
                            const crabby::component<Neighbors>& neighbors) {
    const auto grid = ctx.spatial_view();
    ctx.world().query_par([&](auto chunk) {
        auto t = chunk.template column<0>();
        auto n = chunk.template column<1>();
        for (std::size_t i = 0; i < chunk.size(); ++i) {
            const CrabbyEntity me = chunk.entities()[i];
            n[i].count = 0;
            grid.for_each_in_radius(t[i].position, 10, [&](const crabby::grid_hit& o) {
                if (o.entity.index != me.index || o.entity.generation != me.generation) ++n[i].count;
            });
        }
    }, transforms, neighbors);
}
// ANCHOR_END: spatial

// ---- Drawing ---------------------------------------------------------------------
[[maybe_unused]] static void drawing(crabby::ctx& ctx) {
    // ANCHOR: sprites
    auto crab = ctx.load_texture("crab.png");
    auto world = ctx.world();
    world.spawn(world.transforms().with(crabby::transform_at({400, 300}, 0.3f, {2, 2})),
                world.sprites().with(crabby::sprite_of(crab, crabby::colors::red, 1)));
    // ANCHOR_END: sprites
    // ANCHOR: immediate
    ctx.clear(crabby::rgb(0.125f, 0.19f, 0.25f));
    ctx.draw(crab, {100, 100});
    auto params = crabby::draw_defaults();
    params.rotation = 0.5f;
    params.scale = {2, 2};
    params.color = crabby::colors::red;
    ctx.draw(crab, {200, 100}, params);
    // ANCHOR_END: immediate
    // ANCHOR: pixels
    std::vector<std::uint8_t> rgba(64 * 64 * 4, 255);
    auto heat = ctx.create_texture(64, 64, rgba.data(), rgba.size());
    rgba[0] = 0;   // ...change pixels...
    ctx.update_texture(heat, rgba.data(), rgba.size());
    // ANCHOR_END: pixels
}

// ---- Camera, text and UI ---------------------------------------------------------
[[maybe_unused]] static void camera(crabby::ctx& ctx, crabby::vec2 bounds) {
    // ANCHOR: camera_fit
    const crabby::vec2 window = ctx.window_size();
    auto cam = ctx.camera();
    cam.zoom = std::min(window.x / bounds.x, window.y / bounds.y);
    cam.position = bounds / 2 - window / (2 * cam.zoom); // look at the middle
    ctx.set_camera(cam);
    // ANCHOR_END: camera_fit
    // ANCHOR: text
    ctx.draw_text("Hello, crabs", {20, 20}, 24, crabby::colors::white);
    const crabby::vec2 size = ctx.measure_text("Hello, crabs", 24);
    // ANCHOR_END: text
    (void)size;
}

// ---- Headless runs and testing ---------------------------------------------------
// ANCHOR: headless
struct Pos { float y; };

static void things_fall_ten_units_per_second() {
    crabby::component<Pos> pos;
    auto world = crabby::game("test", 800, 600)
        .sim_system([&](crabby::ctx& ctx) {
            const float dt = ctx.dt();
            ctx.world().query([dt](auto chunk) {
                auto p = chunk.template column<0>();
                for (std::size_t i = 0; i < chunk.size(); ++i) p[i].y -= 10 * dt;
            }, pos);
        }, "fall")
        .run_headless(60, [&](crabby::ctx& ctx) {
            pos = ctx.world().component<Pos>("Pos");
            ctx.world().spawn(pos.with(Pos{100}));
        });
    float y = 0;
    world.query([&](auto chunk) { y = chunk.template column<0>()[0].y; }, pos);
    if (std::fabs(y - 90) > 1e-3f) { std::fprintf(stderr, "fell to %f\n", double(y)); std::abort(); }
}
// ANCHOR_END: headless

// ---- Randomness ------------------------------------------------------------------
// ANCHOR: random
struct Heading { float radians; };

static void wander(crabby::ctx& ctx, const crabby::component<Heading>& heading) {
    const crabby::random random = ctx.random();
    ctx.world().query_par([&](auto chunk) {
        auto h = chunk.template column<0>();
        for (std::size_t i = 0; i < chunk.size(); ++i)
            h[i].radians += random.of(chunk.entity(i)).range(-0.2f, 0.2f);
    }, heading);
}
// ANCHOR_END: random

int main() {
    things_fall_ten_units_per_second();

    // Run the other snippets headless once, so they're exercised, not only compiled.
    crabby::component<Health> health;
    crabby::component<crabby::transform2d> transforms;
    crabby::component<Neighbors> neighbors;
    crabby::component<Heading> heading;
    std::uint32_t most = 0;
    auto world = crabby::game("guide", 1, 1)
        .spatial_grid(10)
        .sim_system([&](crabby::ctx& ctx) { die(ctx, health); }, "die")
        .sim_system([&](crabby::ctx& ctx) { count_neighbors(ctx, transforms, neighbors); }, "neighbors")
        .sim_system([&](crabby::ctx& ctx) { wander(ctx, heading); }, "wander")
        .sim_system(Spawner{2}, "spawner")
        .sim_system([&](crabby::ctx& ctx) {
            ctx.world().query([&](auto chunk) {
                auto n = chunk.template column<0>();
                for (std::size_t i = 0; i < chunk.size(); ++i) most = std::max(most, n[i].count);
            }, neighbors);
        }, "check")
        .run_headless(4, [&](crabby::ctx& ctx) {
            ecs(ctx);
            auto w = ctx.world();
            health = w.component<Health>("Health");
            transforms = w.transforms();
            neighbors = w.component<Neighbors>("Neighbors");
            heading = w.component<Heading>("Heading");
            for (int i = 0; i < 100; ++i)
                w.spawn(transforms.with(crabby::transform_at({float(i), 0})), neighbors.with(Neighbors{0}),
                        heading.with(Heading{0}), health.with(Health{float(i % 2)}));
        });
    // 100k from ecs() + 50 survivors + 2 spawned by Spawner{2} in 4 ticks.
    if (world.size() != 100000 + 50 + 2 || most < 10) {
        std::fprintf(stderr, "unexpected: %zu entities, %u neighbors\n", world.size(), most);
        return 1;
    }
    (void)&my_game;
    (void)&controls;
    std::puts("guide snippets ok");
}
