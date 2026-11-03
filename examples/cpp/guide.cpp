// The C++ code shown in the guide (book/src), next to the Rust versions.
// Everything here compiles in CI; main() runs the headless parts as a test.
#include "crabby.hpp"
#include <cassert>
#include <cmath>
#include <cstdio>
#include <string>
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

// ---- Sprite sheets, animation and tilemaps ------------------------------------------
[[maybe_unused]] static void sheets(crabby::ctx& ctx, crabby::vec2 pos) {
    auto world = ctx.world();
    // ANCHOR: sheet
    auto tex = ctx.load_texture("units.png");
    auto sheet = crabby::sheet_of(tex, 32, 32);   // 32 x 32 frames
    world.spawn(world.transforms().with(crabby::transform_at(pos)),
                world.sprites().with(crabby::sprite_of(sheet, 5)));   // frame 5
    // ANCHOR_END: sheet
    // ANCHOR: nearest
    ctx.set_texture_filter(tex, crabby::texture_filter::nearest);
    // ANCHOR_END: nearest
}

static void animate(crabby::world_view world, crabby::sprite_sheet sheet, crabby::vec2 pos) {
    // ANCHOR: animation
    world.spawn(world.transforms().with(crabby::transform_at(pos)),
                world.sprites().with(crabby::sprite_of(sheet, 0)),
                world.animations().with(crabby::animation_of(sheet, 0, 8, 12)));   // frames 0-7 at 12 fps
    auto boom = crabby::animation_of(sheet, 8, 8, 24);
    boom.looping = false;   // stops on the last frame; crabby::finished(boom) says when
    // ANCHOR_END: animation
    (void)boom;
}

constexpr std::uint16_t GRASS = 0, STONE = 3;

// ANCHOR: tilemap
static crabby::entity make_map(crabby::world_view world, crabby::sprite_sheet tiles, std::uint32_t side) {
    const crabby::entity map = world.spawn(world.transforms().with(crabby::transform_at({0, 0})));
    auto t = world.add_tilemap(map, tiles, side, side);   // the transform is its top-left corner
    for (std::uint32_t y = 0; y < side; ++y)
        for (std::uint32_t x = 0; x < side; ++x)
            t.set(x, y, (x * 7 + y * 3) % 5 == 0 ? STONE : GRASS);
    return map;
}
// ANCHOR_END: tilemap

// ANCHOR: paint
static void paint(crabby::ctx& ctx, crabby::entity map) {
    if (!ctx.mouse_down(CRABBY_MOUSE_LEFT) || ctx.mouse_over_ui()) return;
    auto world = ctx.world();
    const crabby::vec2 origin = world.get(map, world.transforms()).position;
    auto tiles = world.tilemap(map);
    if (auto tile = tiles.tile_at(origin, ctx.mouse_world())) tiles.set((*tile)[0], (*tile)[1], STONE);
}
// ANCHOR_END: paint

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
    ctx.draw_text("Hello", {20, 20}, 24, crabby::colors::white);
    const crabby::vec2 size = ctx.measure_text("Paused", 32);   // for centering
    const crabby::font title = ctx.load_font("fonts/Title.ttf");  // TrueType or OpenType
    auto params = crabby::text_defaults();
    params.font = title;
    params.size = 40;
    ctx.draw_text("Colony", {20, 60}, params);
    // ANCHOR_END: text
    (void)size;
}

[[maybe_unused]] static void shapes(crabby::ctx& ctx, crabby::vec2 from, crabby::vec2 to, crabby::vec2 tower) {
    // ANCHOR: shapes
    ctx.draw_rect(crabby::rect_at({0, 0}, {200, 20}), crabby::rgb(0.23f, 0.35f, 0.25f));
    ctx.draw_line(from, to, 2, crabby::colors::red);
    ctx.draw_circle(tower, 120, crabby::rgba(0, 0, 1, 0.2f));
    // ANCHOR_END: shapes
}

// ANCHOR: hud
struct Hud {
    int dropped = 0;
    bool follow = false;
    void operator()(crabby::ctx& ctx) {
        const crabby::rect panel = crabby::rect_at({12, 12}, {230, 172});
        ctx.ui_panel(panel);
        const float pad = ctx.ui_style().padding;
        crabby::vec2 at = panel.min + crabby::vec2{pad, pad};
        char lines[128];
        std::snprintf(lines, sizeof lines, "zoom %.2fx\ncrabs dropped: %d\n%.0f FPS",
                      double(ctx.camera().zoom), dropped, double(ctx.fps()));
        at.y += ctx.ui_label(at, lines).y + 8;

        if (ctx.ui_button(crabby::rect_at(at, {210, 30}), follow ? "Stop following" : "Follow orbiter"))
            follow = !follow;
        at.y += 38;
        if (ctx.ui_button(crabby::rect_at(at, {210, 30}), "Reset camera")) {
            ctx.set_camera({{0, 0}, 1});
            follow = false;
        }
    }
};
// crabby::game("UI", 1280, 720).camera_controls().system(Hud{}, "hud");
// ANCHOR_END: hud

[[maybe_unused]] static void screenshot(crabby::ctx& ctx) {
    // ANCHOR: screenshot
    if (ctx.key_pressed(CRABBY_KEY_F12)) ctx.save_screenshot("shot.png");
    // ANCHOR_END: screenshot
}

// ---- Hot reload --------------------------------------------------------------------
[[maybe_unused]] static void hot_reload_game() {
    // ANCHOR: hot_reload
    crabby::texture level{};
    crabby::game("Art test", 1280, 720)
        .hot_reload(true)   // the default in debug builds of the engine
        .system([&](crabby::ctx& ctx) {
            for (crabby::texture t : ctx.reloaded_textures())
                if (t.id == level.id) std::puts("level.png changed: rebuilding the map");
        }, "watch_level")
        .run([&](crabby::ctx& ctx) { level = ctx.load_texture("level.png"); });
    // ANCHOR_END: hot_reload
}

// ---- Data files ----------------------------------------------------------------------
// ANCHOR: data_files
struct Tuning { float speed; float damage; };

[[maybe_unused]] static void load_tuning(crabby::ctx& ctx) {
    auto tuning = ctx.world().component<Tuning>("Tuning",
        {crabby::field("speed", &Tuning::speed), crabby::field("damage", &Tuning::damage)});
    const crabby::entity settings = ctx.world().spawn(tuning.with(Tuning{}));
    ctx.load_json("tuning.json", settings, tuning);   // {"speed": 30, "damage": 2}
    // Systems read ctx.world().get(settings, tuning): the file's current values.
}
// ANCHOR_END: data_files

// ---- Sound -------------------------------------------------------------------------
[[maybe_unused]] static void sound(crabby::ctx& ctx) {
    // ANCHOR: sound
    const crabby::sound hit = ctx.load_sound("hit.ogg");   // OGG Vorbis, WAV, MP3 or FLAC from assets/
    ctx.play_sound(hit);

    const crabby::sound music = ctx.load_sound("theme.ogg");
    auto params = crabby::sound_defaults();
    params.volume = 0.5f;
    params.looping = true;
    const crabby::playing song = ctx.play_sound(music, params);
    ctx.stop_sound(song);
    // ANCHOR_END: sound
}

static crabby::sound beep(crabby::ctx& ctx) {
    // ANCHOR: sound_samples
    std::vector<float> samples(4410);   // 0.1 s at 44.1 kHz
    for (std::size_t i = 0; i < samples.size(); ++i)
        samples[i] = 0.3f * std::sin(float(i) * 880.0f * 6.2831853f / 44100.0f);
    const crabby::sound click = ctx.create_sound(44100, samples.data(), samples.size());
    // ANCHOR_END: sound_samples
    return click;
}

// ANCHOR: turns
struct Brain { std::uint32_t plans; };

static void think(crabby::ctx& ctx, const crabby::component<Brain>& brain) {
    const crabby::turns turn = ctx.turns(8);   // each ant thinks every 8th tick
    ctx.world().query_par([&](auto chunk) {
        auto b = chunk.template column<0>();
        for (std::size_t i = 0; i < chunk.size(); ++i)
            if (turn.includes(chunk.entities()[i]))
                b[i].plans += 1;   // turn.dt(): time since this ant's last turn
    }, brain);
}
// ANCHOR_END: turns

// ---- Reflection and saving ---------------------------------------------------------
// ANCHOR: reflect
struct Unit {
    float current;
    float max;
    crabby::vec2 home;
    crabby::entity target;   // entity ids are uint64_t
    bool angry;
};

static crabby::component<Unit> reflect_units(crabby::world_view world) {
    // Tools and agents (crabby-mcp) now see {"current", "max", "home", "target", "angry"}.
    return world.component<Unit>("Unit", {
        crabby::field("current", &Unit::current),
        crabby::field("max", &Unit::max),
        crabby::field("home", &Unit::home),
        crabby::field("target", &Unit::target),
        crabby::field("angry", &Unit::angry),
    });
}
// ANCHOR_END: reflect

static void save_and_load(crabby::world_view world) {
    // ANCHOR: save
    const std::string save = world.save();   // JSON text; write it to a file
    // ...
    world.load(save);                        // every entity back, with its id
    // ANCHOR_END: save
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

// ---- Pathfinding with flow fields --------------------------------------------------
// ANCHOR: flow_map
static void make_walls(crabby::nav_grid& nav) {
    // 1 is normal ground, higher is slower (mud), nav_grid::wall blocks.
    for (std::uint32_t y = 0; y < 60; ++y) nav.set_cost(40, y, crabby::nav_grid::wall);
    for (std::uint32_t x = 50; x < 70; ++x)
        for (std::uint32_t y = 20; y < 40; ++y) nav.set_cost(x, y, 6);
}
// ANCHOR_END: flow_map

// ANCHOR: flow_steer
static void steer(crabby::ctx& ctx, const crabby::flow_field& flow,
                  const crabby::component<crabby::transform2d>& transforms) {
    const float step = 40 * ctx.dt();
    const crabby::flow_view view = flow.view();   // inline lookups, safe from every thread
    ctx.world().query_par([&](auto chunk) {
        auto t = chunk.template column<0>();
        for (std::size_t i = 0; i < chunk.size(); ++i)
            t[i].position += view.direction_at(t[i].position) * step;
    }, transforms);
}
// ANCHOR_END: flow_steer

// ---- Events --------------------------------------------------------------------------
// ANCHOR: events
struct Hp { int value; };
struct Died { crabby::entity who; };

// Sends one Died per entity out of health, from inside a query.
static void deaths(crabby::ctx& ctx, const crabby::component<Hp>& health,
                   const crabby::component<Died>& died) {
    ctx.world().query([&](auto chunk) {
        auto h = chunk.template column<0>();
        for (std::size_t i = 0; i < chunk.size(); ++i)
            if (h[i].value <= 0) ctx.send(died, Died{chunk.entity(i)});
    }, health);
}

// Any later system (or this one next tick) reads them, once each.
static void funerals(crabby::ctx& ctx, const crabby::component<Died>& died) {
    for (const Died& d : ctx.read(died)) ctx.despawn_later(d.who);
}
// ANCHOR_END: events

static void the_dead_are_buried() {
    crabby::component<Hp> health;
    crabby::component<Died> died;
    auto world = crabby::game("events", 1, 1)
        .sim_system([&](crabby::ctx& ctx) { deaths(ctx, health, died); }, "deaths")
        .sim_system([&](crabby::ctx& ctx) { funerals(ctx, died); }, "funerals")
        .run_headless(1, [&](crabby::ctx& ctx) {
            health = ctx.world().component<Hp>("Hp");
            // ANCHOR: events_reflect
            // Registered with its fields: tools can collect Died events as JSON.
            died = ctx.world().component<Died>("Died", {crabby::field("who", &Died::who)});
            // ANCHOR_END: events_reflect
            for (int hp : {0, 3, -1, 5}) ctx.world().spawn(health.with(Hp{hp}));
        });
    if (world.size() != 2) {
        std::fprintf(stderr, "events: %zu alive, expected 2\n", world.size());
        std::abort();
    }
}

static void crowds_reach_the_goal() {
    // ANCHOR: flow_compute
    crabby::nav_grid nav(100, 80, 4.0f);   // 100 x 80 cells, 4 world units each
    make_walls(nav);
    crabby::flow_field flow;
    flow.compute(nav, crabby::vec2{390, 10});   // or several goals: the nearest wins
    // ANCHOR_END: flow_compute
    crabby::component<crabby::transform2d> transforms;
    auto world = crabby::game("flow", 1, 1)
        .sim_system([&](crabby::ctx& ctx) { steer(ctx, flow, transforms); }, "steer")
        .run_headless(1200, [&](crabby::ctx& ctx) {
            transforms = ctx.world().transforms();
            for (int i = 0; i < 1000; ++i)
                ctx.world().spawn(transforms.with(crabby::transform_at({2.0f + float(i % 30) * 5, 2.0f + float(i / 30) * 9})));
        });
    std::size_t arrived = 0;
    world.query([&](auto chunk) {
        auto t = chunk.template column<0>();
        for (std::size_t i = 0; i < chunk.size(); ++i)
            arrived += flow.distance_at(t[i].position).value_or(1e9f) < 12;
    }, transforms);
    const auto start = flow.distance_at({2, 2});
    if (arrived < 990 || !start || *start < 390 || nav.walkable({162, 10}) || !nav.walkable({2, 2})) {
        std::fprintf(stderr, "flow: %zu of 1000 arrived\n", arrived);
        std::abort();
    }
}

int main() {
    things_fall_ten_units_per_second();
    crowds_reach_the_goal();
    the_dead_are_buried();

    // Run the other snippets headless once, so they're exercised, not only compiled.
    crabby::component<Health> health;
    crabby::component<crabby::transform2d> transforms;
    crabby::component<Neighbors> neighbors;
    crabby::component<Heading> heading;
    crabby::component<Brain> brain;
    crabby::sound click{};
    crabby::entity map = 0;
    std::uint32_t most = 0;
    auto world = crabby::game("guide", 1, 1)
        .spatial_grid(10)
        .sim_system([&](crabby::ctx& ctx) { die(ctx, health); }, "die")
        .sim_system([&](crabby::ctx& ctx) { count_neighbors(ctx, transforms, neighbors); }, "neighbors")
        .sim_system([&](crabby::ctx& ctx) { wander(ctx, heading); }, "wander")
        .sim_system(Spawner{2}, "spawner")
        .sim_system([&](crabby::ctx& ctx) { think(ctx, brain); }, "think")
        .system(Hud{}, "hud")
        .camera_controls()
        .sim_system([&](crabby::ctx& ctx) {
            ctx.world().query([&](auto chunk) {
                auto n = chunk.template column<0>();
                for (std::size_t i = 0; i < chunk.size(); ++i) most = std::max(most, n[i].count);
            }, neighbors);
        }, "check")
        .run_headless(8, [&](crabby::ctx& ctx) {
            ecs(ctx);
            auto w = ctx.world();
            health = w.component<Health>("Health");
            transforms = w.transforms();
            neighbors = w.component<Neighbors>("Neighbors");
            heading = w.component<Heading>("Heading");
            brain = w.component<Brain>("Brain");
            for (int i = 0; i < 16; ++i) w.spawn(brain.with(Brain{0}));
            click = beep(ctx);
            std::vector<std::uint8_t> pixels(64 * 40 * 4, 255);
            const auto sheet = crabby::sheet_of(ctx.create_texture(64, 40, pixels.data(), pixels.size()), 16, 16);
            animate(w, sheet, {0, 0});
            map = make_map(w, sheet, 50);
            paint(ctx, map);   // nobody holds the mouse headless: nothing changes
            ctx.play_sound(click);   // silent: headless
            for (int i = 0; i < 100; ++i)
                w.spawn(transforms.with(crabby::transform_at({float(i), 0})), neighbors.with(Neighbors{0}),
                        heading.with(Heading{0}), health.with(Health{float(i % 2)}));
        });
    {
        auto tiles = world.tilemap(map);
        std::size_t stone = 0;
        for (std::size_t i = 0; i < std::size_t(tiles.width()) * tiles.height(); ++i) stone += tiles.tiles()[i] == STONE;
        if (tiles.width() != 50 || stone != 500 || tiles.get(0, 0) != STONE || tiles.tile_at({0, 0}, {-1, 5})) {
            std::fprintf(stderr, "tilemap: %u wide, %zu stone\n", tiles.width(), stone);
            return 1;
        }
    }
    // Over 8 ticks every brain gets exactly one turn.
    std::uint32_t plans = 0;
    world.query([&](auto chunk) {
        for (std::size_t i = 0; i < chunk.size(); ++i) plans += chunk.template column<0>()[i].plans;
    }, brain);
    if (plans != 16 || std::fabs(click.seconds - 0.1f) > 1e-3f) {
        std::fprintf(stderr, "unexpected: %u plans, %f s click\n", plans, double(click.seconds));
        return 1;
    }
    // 100k from ecs() + 16 brains + 50 survivors + 4 spawned by Spawner{2} in 8 ticks + 1 animated + 1 map.
    if (world.size() != 100000 + 16 + 50 + 4 + 2 || most < 10) {
        std::fprintf(stderr, "unexpected: %zu entities, %u neighbors\n", world.size(), most);
        return 1;
    }
    // Reflected components survive a save and load with their ids.
    {
        auto w = crabby::game("save", 1, 1).run_headless(0, [](crabby::ctx&) {});
        auto unit = reflect_units(w);
        const crabby::entity a = w.spawn(unit.with(Unit{3, 10, {1, 2}, 0, false}));
        const crabby::entity b = w.spawn(unit.with(Unit{5, 10, {0, 0}, a, true}));
        const std::string before = w.save();
        w.set(b, unit, Unit{0, 0, {0, 0}, 0, false});
        save_and_load(w);   // saves the changed world and loads it straight back
        w.load(before);
        const Unit u = w.get(b, unit);
        if (u.current != 5 || u.target != a || !u.angry || before.find("\"angry\":true") == std::string::npos) {
            std::fprintf(stderr, "save/load lost data: %s\n", before.c_str());
            return 1;
        }
    }
    (void)&my_game;
    (void)&controls;
    std::puts("guide snippets ok");
}
