// The C++ code shown in the guide (book/src), next to the Rust versions.
// Everything here compiles in CI; main() runs the headless parts as a test.
#include "crabby.hpp"
#include <algorithm>
#include <array>
#include <cassert>
#include <cmath>
#include <cstdio>
#include <future>
#include <map>
#include <optional>
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
    crabby::entity e = world.spawn(transform_at({10, 20}), Health{100}, Enemy{});

    // Many at once:
    for (int i = 0; i < 100000; ++i)
        world.spawn(transform_at({float(i), 0}), Health{10});
    // ANCHOR_END: ecs_spawn

    // ANCHOR: ecs_one
    Health h = world.get<Health>(e);
    world.set(e, Health{h.value - 5});
    world.set(e, Enemy{});            // add or replace a component
    if (world.has<Enemy>(e)) world.remove<Enemy>(e);
    world.despawn(e);
    assert(!world.alive(e));          // old handles stay dead, even if the slot is reused
    assert(!world.try_get<Health>(e));
    // ANCHOR_END: ecs_one

    // ANCHOR: ecs_query
    world.each([](transform2d& t, const Health& hp) {
        t.position.y += hp.value;
    });
    // ANCHOR_END: ecs_query

    // ANCHOR: ecs_columns
    world.query<transform2d, Health>([](auto chunk) {
        auto [t, hp] = chunk.columns();   // one array per component
        for (std::size_t i = 0; i < chunk.size(); ++i)
            t[i].position.y += hp[i].value;
    });
    // ANCHOR_END: ecs_columns
}

// ANCHOR: ecs_commands
static void die(crabby::ctx& ctx) {
    ctx.world().each([&](crabby::entity e, const Health& hp) {
        if (hp.value <= 0) ctx.despawn_later(e);
    });
}
// ANCHOR_END: ecs_commands

// ANCHOR: resources
struct Score { std::uint32_t points; };

[[maybe_unused]] static void score_point(crabby::ctx& ctx) {
    auto world = ctx.world();
    Score score = world.try_resource<Score>().value_or(Score{0});
    score.points += 1;
    world.set_resource(score);
}
// ANCHOR_END: resources

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
            ctx.spawn_later(crabby::transform_at({0, 0}));
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

// ---- Input -------------------------------------------------------------------
// ANCHOR: gamepad
struct Player { std::uint32_t slot; float spin; };   // one per connected pad

static void move_players(crabby::ctx& ctx) {
    const float dt = ctx.dt();
    // Read the pads first: the loop below borrows the world.
    crabby::vec2 sticks[CRABBY_MAX_GAMEPADS] = {};
    bool spins[CRABBY_MAX_GAMEPADS] = {};
    float grow[CRABBY_MAX_GAMEPADS] = {};
    for (crabby::gamepad pad : ctx.gamepads()) {
        sticks[pad.slot] = ctx.gamepad_stick(pad, crabby::stick::left);
        spins[pad.slot] = ctx.gamepad_pressed(pad, CRABBY_GAMEPAD_SOUTH);
        grow[pad.slot] = ctx.gamepad_axis(pad, CRABBY_GAMEPAD_RIGHT_TRIGGER_AXIS);
    }
    ctx.world().each([&](crabby::transform2d& t, Player& player) {
        t.position += sticks[player.slot] * 400 * dt;
        if (spins[player.slot]) player.spin = 6.2831853f;
        t.scale = crabby::vec2{1, 1} * (1 + 2 * grow[player.slot]);
    });
}
// ANCHOR_END: gamepad

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
static void count_neighbors(crabby::ctx& ctx) {
    const auto grid = ctx.spatial_view();
    ctx.world().each_par([&](crabby::entity me, const crabby::transform2d& t, Neighbors& n) {
        n.count = 0;
        grid.for_each_in_radius(t.position, 10, [&](const crabby::grid_hit& o) {
            if (crabby::bits(o.entity) != me) ++n.count;
        });
    });
}
// ANCHOR_END: spatial

// ---- Drawing ---------------------------------------------------------------------
[[maybe_unused]] static void drawing(crabby::ctx& ctx) {
    // ANCHOR: sprites
    auto crab = ctx.load_texture("crab.png");
    auto world = ctx.world();
    world.spawn(crabby::transform_at({400, 300}, 0.3f, {2, 2}),
                crabby::sprite_of(crab, crabby::colors::red, 1));
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
    world.spawn(crabby::transform_at(pos), crabby::sprite_of(sheet, 5));   // frame 5
    // ANCHOR_END: sheet
    // ANCHOR: nearest
    ctx.set_texture_filter(tex, crabby::texture_filter::nearest);
    // ANCHOR_END: nearest
}

static void animate(crabby::world_view world, crabby::sprite_sheet sheet, crabby::vec2 pos) {
    // ANCHOR: animation
    world.spawn(crabby::transform_at(pos), crabby::sprite_of(sheet, 0),
                crabby::animation_of(sheet, 0, 8, 12));   // frames 0-7 at 12 fps
    auto boom = crabby::animation_of(sheet, 8, 8, 24);
    boom.looping = false;   // stops on the last frame; crabby::finished(boom) says when
    // ANCHOR_END: animation
    (void)boom;
}

constexpr std::uint16_t GRASS = 0, STONE = 3;

// ANCHOR: tilemap
static crabby::entity make_map(crabby::world_view world, crabby::sprite_sheet tiles, std::uint32_t side) {
    const crabby::entity map = world.spawn(crabby::transform_at({0, 0}));
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
    const crabby::vec2 origin = world.get<crabby::transform2d>(map).position;
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

// ---- Physics -----------------------------------------------------------------------
// ANCHOR: physics_bodies
struct Bell { float rang; };

static void build_level(crabby::world_view world) {
    // Fixed bodies never move: the ground.
    world.spawn(crabby::transform_at({500, 690}), crabby::fixed_body(), crabby::rect_collider({1000, 20}));
    // A bouncy ball and a crate, both dynamic: gravity and collisions move them.
    auto ball = crabby::circle_collider(16);
    ball.restitution = 0.8f;
    world.spawn(crabby::transform_at({300, 100}), crabby::dynamic_body(), ball);
    auto box = crabby::rect_collider({40, 40});
    box.friction = 0.9f;
    world.spawn(crabby::transform_at({320, 0}), crabby::dynamic_body(), box);
    // A ramp: any convex shape of up to 8 corners, around its position.
    world.spawn(crabby::transform_at({150, 620}), crabby::fixed_body(),
                crabby::polygon_collider({{-140, 60}, {140, 60}, {-140, -60}}));
    // A sensor reports overlaps but doesn't push.
    auto bucket = crabby::rect_collider({120, 80});
    bucket.sensor = true;
    world.spawn(crabby::transform_at({800, 640}), crabby::fixed_body(), bucket, Bell{0});
}
// ANCHOR_END: physics_bodies

// ANCHOR: physics_collisions
static void ring_the_bell(crabby::ctx& ctx) {
    auto world = ctx.world();
    bool rang = false;
    for (const crabby::collision& c : ctx.collisions_started())
        rang |= c.sensor && (world.has<Bell>(crabby::bits(c.a)) || world.has<Bell>(crabby::bits(c.b)));
    world.each([&](Bell& bell) { bell.rang = rang ? 1.0f : std::max(0.0f, bell.rang - ctx.dt() * 2); });
}
// ANCHOR_END: physics_collisions

// ANCHOR: physics_queries
static void blast(crabby::ctx& ctx, crabby::vec2 center) {
    std::vector<CrabbyEntity> near;
    ctx.entities_in_circle(center, 200, near);
    for (CrabbyEntity found : near) {
        const crabby::entity e = crabby::bits(found);
        const crabby::vec2 away = crabby::normalize_or(ctx.world().get<crabby::transform2d>(e).position - center, {0, -1});
        if (auto mass = ctx.mass(e)) ctx.apply_impulse(e, away * *mass * 600.0f);
    }
}

static std::optional<crabby::ray_hit> laser(crabby::ctx& ctx, crabby::vec2 from) {
    return ctx.cast_ray(from, {0, 1}, 700);   // what's straight below
}
// ANCHOR_END: physics_queries

static bool physics_snippets() {
    bool landed = false;
    // ANCHOR: physics_setup
    auto world = crabby::game("Physics", 1000, 700)
        .physics({0, 980})   // pixels/s², y is down
        .sim_system(ring_the_bell, "bell")
        // ANCHOR_END: physics_setup
        .sim_system([&](crabby::ctx& ctx) {
            if (ctx.sim_tick() == 100) blast(ctx, {300, 690});
            if (ctx.sim_tick() == 1) landed = laser(ctx, {500, 0}).has_value();
        }, "test")
        .run_headless(120, [](crabby::ctx& ctx) { build_level(ctx.world()); });
    return landed && world.size() == 5;
}

// ---- Lockstep networking ----------------------------------------------------------
// ANCHOR: lockstep_input
struct Order { float x, y; bool go; };   // what one player did this tick
struct Owner { std::uint32_t player; };

static void send_orders(crabby::ctx& ctx) {
    // Usually from the mouse, in a frame system; offline it just works too.
    ctx.set_input(Order{100, 50, ctx.sim_tick() % 30 == 0});
}

static void obey_orders(crabby::ctx& ctx) {
    // Every peer applies every player's input, in the same order.
    std::vector<std::optional<Order>> orders;
    for (std::uint32_t p = 0; p < ctx.players(); ++p) orders.push_back(ctx.input<Order>(p));
    ctx.world().each([&](crabby::transform2d& t, const Owner& owner) {
        if (owner.player < orders.size() && orders[owner.player] && orders[owner.player]->go)
            t.position = {orders[owner.player]->x, orders[owner.player]->y};
    });
}
// ANCHOR_END: lockstep_input

[[maybe_unused]] static void multiplayer(bool hosting) {
    // ANCHOR: lockstep_setup
    crabby::game game("RTS", 1280, 720);
    if (hosting)
        game.lockstep_host("0.0.0.0:7777", 2);        // 2 players, 3 ticks input delay
    else
        game.lockstep_join("192.168.1.20:7777");
    game.sim_system(send_orders).sim_system(obey_orders).run([](crabby::ctx&) {});
    // ANCHOR_END: lockstep_setup
}

[[maybe_unused]] static void steam_multiplayer(bool hosting) {
    // ANCHOR: steam_setup
    // crabby_ffi built with --features steam; 480 is Valve's test app.
    crabby::game game("RTS", 1280, 720);
    if (std::uint64_t lobby = crabby::steam_lobby_from_args())   // launched from an invite
        game.lockstep_steam_join(480, lobby);
    else if (hosting)
        game.lockstep_steam_host(480, 2);                          // then ctx.steam_invite()
    else
        game.lockstep_steam_join(480, crabby::steam_wait_for_invite(480, 120000));
    // ANCHOR_END: steam_setup
    game.sim_system(send_orders).sim_system(obey_orders).run([](crabby::ctx&) {});
}

// ---- Data files ----------------------------------------------------------------------
// ANCHOR: data_files
struct Tuning { float speed; float damage; };

[[maybe_unused]] static void load_tuning(crabby::ctx& ctx) {
    ctx.world().reflect_resource<Tuning>({crabby::field("speed", &Tuning::speed), crabby::field("damage", &Tuning::damage)});
    ctx.load_json<Tuning>("tuning.json");   // {"speed": 30, "damage": 2}
    // Systems read ctx.world().resource<Tuning>(): the file's current values.
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

static void think(crabby::ctx& ctx) {
    const crabby::turns turn = ctx.turns(8);   // each ant thinks every 8th tick
    ctx.world().each_par([&](crabby::entity e, Brain& brain) {
        if (turn.includes(e))
            brain.plans += 1;   // turn.dt(): time since this ant's last turn
    });
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

static void reflect_units(crabby::world_view world) {
    // Tools and agents (crabby-mcp) now see {"current", "max", "home", "target", "angry"}.
    world.reflect<Unit>({
        crabby::field("current", &Unit::current),
        crabby::field("max", &Unit::max),
        crabby::field("home", &Unit::home),
        crabby::field("target", &Unit::target),
        crabby::field("angry", &Unit::angry),
    });
}
// ANCHOR_END: reflect

// ANCHOR: reflect_resource
struct Teams { std::uint32_t red, blue; };

static void reflect_teams(crabby::world_view world) {
    world.reflect_resource<Teams>({crabby::field("red", &Teams::red), crabby::field("blue", &Teams::blue)});
    world.set_resource(Teams{0, 0});   // agents: get_resource "Teams" -> {"blue": 0, "red": 0}
}
// ANCHOR_END: reflect_resource

// ---- Inspector ----------------------------------------------------------------------
[[maybe_unused]] static void playtest_build() {
    // ANCHOR: inspector
    crabby::game("Playtest", 1280, 720)
        .inspector(true)   // also in release builds: the ` key opens it
        .run([](crabby::ctx& ctx) { reflect_units(ctx.world()); });
    // ANCHOR_END: inspector
}

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
    crabby::entity thing = 0;
    auto world = crabby::game("test", 800, 600)
        .sim_system([](crabby::ctx& ctx) {
            const float dt = ctx.dt();
            ctx.world().each([dt](Pos& p) { p.y -= 10 * dt; });
        }, "fall")
        .run_headless(60, [&](crabby::ctx& ctx) { thing = ctx.world().spawn(Pos{100}); });
    const float y = world.get<Pos>(thing).y;
    if (std::fabs(y - 90) > 1e-3f) { std::fprintf(stderr, "fell to %f\n", double(y)); std::abort(); }
}
// ANCHOR_END: headless

// ---- Randomness ------------------------------------------------------------------
// ANCHOR: random
struct Heading { float radians; };

static void wander(crabby::ctx& ctx) {
    const crabby::random random = ctx.random();
    ctx.world().each_par([&](crabby::entity e, Heading& h) {
        h.radians += random.of(e).range(-0.2f, 0.2f);
    });
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
static void steer(crabby::ctx& ctx, const crabby::flow_field& flow) {
    const float step = 40 * ctx.dt();
    const crabby::flow_view view = flow.view();   // inline lookups, safe from every thread
    ctx.world().each_par([&](crabby::transform2d& t) {
        t.position += view.direction_at(t.position) * step;
    });
}
// ANCHOR_END: flow_steer

// ---- Events --------------------------------------------------------------------------
// ANCHOR: events
struct Hp { int value; };
struct Died { crabby::entity who; };

// Sends one Died per entity out of health, from inside a loop.
static void deaths(crabby::ctx& ctx) {
    ctx.world().each([&](crabby::entity e, const Hp& hp) {
        if (hp.value <= 0) ctx.send(Died{e});
    });
}

// Any later system (or this one next tick) reads them, once each.
static void funerals(crabby::ctx& ctx) {
    for (const Died& d : ctx.read<Died>()) ctx.despawn_later(d.who);
}
// ANCHOR_END: events

static void the_dead_are_buried() {
    auto world = crabby::game("events", 1, 1)
        .sim_system(deaths, "deaths")
        .sim_system(funerals, "funerals")
        .run_headless(1, [](crabby::ctx& ctx) {
            // ANCHOR: events_reflect
            // Described with its fields: tools can collect Died events as JSON.
            ctx.world().reflect<Died>({crabby::field("who", &Died::who)});
            // ANCHOR_END: events_reflect
            for (int hp : {0, 3, -1, 5}) ctx.world().spawn(Hp{hp});
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
    auto world = crabby::game("flow", 1, 1)
        .sim_system([&](crabby::ctx& ctx) { steer(ctx, flow); }, "steer")
        .run_headless(1200, [](crabby::ctx& ctx) {
            for (int i = 0; i < 1000; ++i)
                ctx.world().spawn(crabby::transform_at({2.0f + float(i % 30) * 5, 2.0f + float(i / 30) * 9}));
        });
    std::size_t arrived = 0;
    world.each([&](const crabby::transform2d& t) {
        arrived += flow.distance_at(t.position).value_or(1e9f) < 12;
    });
    const auto start = flow.distance_at({2, 2});
    if (arrived < 990 || !start || *start < 390 || nav.walkable({162, 10}) || !nav.walkable({2, 2})) {
        std::fprintf(stderr, "flow: %zu of 1000 arrived\n", arrived);
        std::abort();
    }
}

// ---- Attachments -----------------------------------------------------------------
// ANCHOR: attach
struct Tank { float speed; };

static void turrets_ride_on_tanks() {
    crabby::entity turret = 0;
    auto world = crabby::game("tanks", 800, 600)
        .sim_system([](crabby::ctx& ctx) {
            ctx.world().each([](crabby::transform2d& t, const Tank& tank) { t.position.x += tank.speed; });
        }, "drive")
        .follow_attachments() // after the systems that move things
        .run_headless(10, [&](crabby::ctx& ctx) {
            auto w = ctx.world();
            const auto tank = w.spawn(crabby::transform_at({0, 0}), Tank{1});
            turret = w.spawn(crabby::transform_at({0, 0}), crabby::attach_to(tank, {0, -4}));
        });
    const auto at = world.get<crabby::transform2d>(turret).position;
    if (at != crabby::vec2{10, -4}) { std::fprintf(stderr, "turret at %f,%f\n", double(at.x), double(at.y)); std::abort(); }
}
// ANCHOR_END: attach

// ---- Scenes and prefabs -------------------------------------------------------------
// ANCHOR: scenes
struct Palm { float height; };

static void beach(crabby::ctx& ctx) {
    ctx.world().reflect<Palm>({crabby::field("height", &Palm::height)});   // components in files must be reflected
    ctx.load_scene("levels/beach.json");   // the level: its entities, ids kept
    for (int i = 0; i < 3; ++i)            // three more palms, all alike, in a row
        ctx.spawn_prefab("prefabs/palm.json", {400.0f + float(i) * 64, 400});
}
// ANCHOR_END: scenes

static void the_beach_has_five_palms() {
    auto world = crabby::game("guide", 1, 1).run_headless(0, beach);
    std::size_t palms = 0;
    world.each([&](const Palm&) { ++palms; });
    if (palms != 5) { std::fprintf(stderr, "%zu palms\n", palms); std::abort(); }
}

// ---- 3D -------------------------------------------------------------------------------
// ANCHOR: three_d
/// Turns around its own Y axis, this many radians a second.
struct Spin { float speed; };

static void village(crabby::ctx& ctx) {
    auto grass_look = crabby::material_of(crabby::rgb(.37f, .55f, .29f));
    grass_look.roughness = 0.9f;
    auto grass = ctx.create_material(grass_look);
    auto ground = ctx.create_mesh(crabby::mesh_data::plane(100), grass);
    ctx.world().spawn(crabby::transform3d_at({0, 0, 0}), crabby::model_of(ground));

    auto house = ctx.load_mesh("models/house.glb");   // its walls and roof keep their materials
    for (int i = 0; i < 5; ++i) {
        crabby::vec3 at{float(i) * 4 - 8, 0, -6};
        ctx.world().spawn(crabby::transform3d_at(at), crabby::model_of(house), Spin{0.5f});
    }

    auto camera = ctx.camera3d();
    camera.position = {0, 6, 12};
    ctx.set_camera3d(camera);
    ctx.camera3d_look_at({0, 0, -4});
    auto light = ctx.lighting();
    light.sun_direction = {-0.8f, -0.6f, -0.4f};   // late afternoon: long shadows
    ctx.set_lighting(light);
}

static void spin(crabby::ctx& ctx) {
    const float dt = ctx.dt();
    ctx.world().each([&](crabby::transform3d& t, const Spin& s) {
        t.rotation *= crabby::quat_from_rotation_y(s.speed * dt);
    });
}
// ANCHOR_END: three_d

// ANCHOR: sky
/// A misty morning: the sun low in the east, fog lying in the valleys.
static void misty_morning(crabby::ctx& ctx) {
    auto light = ctx.lighting();
    light.sun_direction = crabby::sun_direction_at(7.5f);
    ctx.set_lighting(light);
    auto sky = ctx.sky();
    sky.haze = 2;              // a paler sky, a bigger glow around the sun
    sky.fog_density = 0.03f;   // thick near the ground
    sky.fog_height = 0;
    sky.fog_falloff = 10;      // and thin ten meters up
    ctx.set_sky(sky);
}
// ANCHOR_END: sky

// ANCHOR: post
/// A warm, glowing evening look.
static void golden_hour(crabby::ctx& ctx) {
    auto post = ctx.post();
    post.exposure = 0.3f;      // a third of a stop brighter
    post.bloom = 0.2f;         // highlights glow
    post.temperature = 0.4f;   // warmer
    post.saturation = 1.1f;
    post.vignette = 0.3f;      // darker corners
    post.tonemap = uint32_t(crabby::tonemap::agx);
    ctx.set_post(post);
}
// ANCHOR_END: post

// ANCHOR: shader
/// Grass that sways in the wind: the higher a vertex, the more it moves.
static const char* wind_shader = R"(
fn vertex(v: Vertex) -> Vertex {
    var out = v;
    let sway = sin(frame.time.x * 2.0 + v.position.x) * material.custom[0].x;
    out.position.x += sway * v.uv.y * v.uv.y;
    return out;
})";

static void meadow(crabby::ctx& ctx) {
    auto look = crabby::material_of(crabby::rgb(.5f, .66f, .24f));
    look.has_shader = true;
    look.shader = ctx.create_shader("wind", wind_shader);
    look.double_sided = true;
    look.custom[0] = 0.2f;
    auto grass = ctx.create_material(look);
    auto blade = ctx.create_mesh(crabby::mesh_data::plane(0.5f), grass);
    ctx.world().spawn(crabby::transform3d_at({0, 0, 0}), crabby::model_of(blade));
}
// ANCHOR_END: shader

// ANCHOR: effect
/// Under water, light fades to deep blue with distance (and the sky above
/// is gone). `material.custom[0].x` is the water's height.
static const char* underwater_effect = R"(
fn effect(p: Pixel) -> vec3<f32> {
    if frame.camera.y > material.custom[0].x {
        return p.color; // above the water: nothing changes
    }
    let deep = vec3<f32>(0.02, 0.12, 0.18);
    let fade = 1.0 - exp(-p.distance * 0.08);
    return mix(p.color * vec3<f32>(0.6, 0.9, 1.0), deep, fade);
})";

static void dive(crabby::ctx& ctx) {
    auto underwater = ctx.create_effect("underwater.wgsl", underwater_effect);
    auto desc = crabby::material_of(crabby::colors::white);
    desc.custom[0] = 2; // water at y = 2
    auto lake = ctx.create_material(desc);
    auto list = ctx.effects();
    list.push_back(crabby::screen_effect_of(underwater, lake));
    ctx.set_effects(list);
}
// ANCHOR_END: effect

// ANCHOR: compute
/// Ripples: each texel moves toward the average of its neighbours,
/// overshooting like water. Red is the height now, green the height a
/// frame ago; `material.custom[0]` drops a stone (uv x, uv y, strength).
static const char* ripples_compute = R"(
fn compute(c: Cell) -> vec4<f32> {
    let around = cell_at(c.pixel + vec2<i32>(1, 0)).x + cell_at(c.pixel - vec2<i32>(1, 0)).x
        + cell_at(c.pixel + vec2<i32>(0, 1)).x + cell_at(c.pixel - vec2<i32>(0, 1)).x;
    var height = (around * 0.5 - c.value.y) * 0.985;
    let drop = material.custom[0];
    if drop.z > 0.0 && distance(c.uv, drop.xy) * f32(c.size.x) < 3.0 {
        height -= drop.z;
    }
    return vec4<f32>(height, c.value.x, 0.0, 0.0);
})";

struct ripples_state { crabby::texture heights; crabby::material stones; };

/// The simulated heights, for a water material to read, and the material
/// that drops stones into them.
static ripples_state ripples(crabby::ctx& ctx) {
    auto heights = ctx.create_render_target(256, 256);
    auto ripples = ctx.create_compute("ripples", ripples_compute);
    auto stones = ctx.create_material(crabby::material_of(crabby::colors::white));
    auto list = ctx.compute_passes();
    list.push_back(crabby::compute_pass_of(ripples, heights, stones));
    ctx.set_compute_passes(list);
    return {heights, stones};
}

/// Drops a stone at `uv` (0 to 1 across the water) this frame; `strength`
/// 0 drops none.
static void drop_stone(crabby::ctx& ctx, crabby::material stones, crabby::vec2 uv, float strength) {
    auto desc = crabby::material_of(crabby::colors::white);
    desc.custom[0] = uv.x;
    desc.custom[1] = uv.y;
    desc.custom[2] = strength;
    ctx.update_material(stones, desc);
}
// ANCHOR_END: compute

// ANCHOR: mirror
/// A still pond: its color is the world mirrored in it, which a target
/// camera below the surface draws into `custom_texture0`, upside down.
static const char* mirror_shader = R"(
fn surface(s: Surface) -> Surface {
    var out = s;
    let uv = s.screen * frame.screen.zw;
    out.emissive = textureSample(custom_texture0, material_sampler, vec2<f32>(uv.x, 1.0 - uv.y)).rgb * 0.9;
    out.albedo = vec3<f32>(0.0);
    return out;
})";

static void pond(crabby::ctx& ctx) {
    auto size = ctx.window_size();
    auto reflection = ctx.create_render_target(std::uint32_t(size.x / 2), std::uint32_t(size.y / 2));
    auto look = crabby::material_of(crabby::colors::white);
    look.has_shader = true;
    look.shader = ctx.create_shader("mirror", mirror_shader);
    look.has_custom_textures[0] = true;
    look.custom_textures[0] = reflection;
    auto surface = ctx.create_mesh(crabby::mesh_data::plane(20), ctx.create_material(look));
    ctx.world().spawn(crabby::transform3d_at({0, 0, 0}), crabby::model_of(surface));
    ctx.set_target_cameras({ctx.reflection_camera(ctx.camera3d(), reflection, 0)});
}

/// Every frame: the mirrored camera follows the real one.
static void follow_reflections(crabby::ctx& ctx) {
    auto list = ctx.target_cameras();
    for (auto& t : list) t = ctx.reflection_camera(ctx.camera3d(), t.target, 0);
    ctx.set_target_cameras(list);
}
// ANCHOR_END: mirror

// ANCHOR: terrain_material
static float hill_height(float x, float z) { return std::sin(x * 0.02f) * 6 + std::cos(z * 0.03f) * 4; }

/// Three ground layers, speckled so they don't look flat (in a game,
/// ctx.load_texture_array from image files), and one weight map over
/// all nine tiles from the heights: red grass, green dirt, blue rock.
static crabby::material hill_ground(crabby::ctx& ctx) {
    std::vector<std::uint8_t> pixels;
    for (auto base : {std::array<int, 3>{70, 120, 50}, {110, 85, 60}, {120, 120, 115}})
        for (std::uint32_t i = 0; i < 64 * 64; ++i) {
            int shade = int((i * 2654435761u) >> 27); // 0..31
            for (int c : base) pixels.push_back(std::uint8_t(std::min(c + shade, 255)));
            pixels.push_back(255);
        }
    auto layers = ctx.create_texture_array(64, 64, pixels, crabby::texture_kind::color);
    const std::uint32_t cells = 96;
    const float area = 384; // 4 m a pixel, from (-128, -128)
    std::vector<std::uint8_t> weights;
    for (std::uint32_t i = 0; i < cells * cells; ++i) {
        float x = float(i % cells) * area / float(cells) - 128, z = float(i / cells) * area / float(cells) - 128;
        float up = std::clamp((hill_height(x, z) + 10) / 20, 0.0f, 1.0f); // 0 low, 1 high
        float rock = std::clamp((up - 0.7f) * 4, 0.0f, 1.0f);
        float dirt = std::clamp(1 - std::abs(up - 0.5f) * 4, 0.0f, 1.0f);
        float grass = 1 - std::max(rock, dirt);
        for (float w : {grass, dirt, rock, 0.0f}) weights.push_back(std::uint8_t(w * 255));
    }
    auto weight_map = ctx.create_texture_array(cells, cells, weights, crabby::texture_kind::data);
    auto desc = crabby::terrain_material_of(layers, weight_map, {-128, -128}, {area, area});
    const float sizes[] = {6, 4, 8};
    std::copy(std::begin(sizes), std::end(sizes), desc.layer_size);
    return ctx.create_terrain_material(desc);
}
// ANCHOR_END: terrain_material

// ANCHOR: terrain
/// Rolling hills: 3 x 3 tiles of 128 m, each from 65 x 65 heights 2 m
/// apart, sharing their edges, with a pond-sized hole in the middle tile.
/// Grass in the valleys, dirt higher up and rock on the tops.
static void hills(crabby::ctx& ctx) {
    auto ground = hill_ground(ctx);
    const int n = 65;
    const float size = 128;
    for (int tz = -1; tz <= 1; ++tz)
        for (int tx = -1; tx <= 1; ++tx) {
            crabby::vec3 corner{float(tx) * size, 0, float(tz) * size};
            std::vector<float> heights(n * n);
            for (int i = 0; i < n * n; ++i) {
                float x = float(i % n) * 2 + corner.x, z = float(i / n) * 2 + corner.z;
                bool hole = tx == 0 && tz == 0 && std::hypot(x - 64, z - 64) < 6;
                heights[i] = hole ? NAN : hill_height(x, z);
            }
            auto tile = ctx.create_terrain_tile(heights, size, ground);
            ctx.world().spawn(crabby::transform3d_at(corner), tile);
        }
    auto cam = ctx.camera3d();
    cam.position = {64, 30, 220};
    ctx.set_camera3d(cam);
    ctx.camera3d_look_at({64, 0, 64});
}
// ANCHOR_END: terrain

// ANCHOR: streaming
/// Ground that loads around the camera: tiles 128 m wide within 600 m,
/// built on worker threads, freed past 800 m.
static float hill(float x, float z) { return std::sin(x * 0.01f) * std::cos(z * 0.013f) * 20.0f; }

struct Ground {
    struct Tile {
        std::future<crabby::terrain_tile_data> building;
        crabby::entity entity = 0;   // once built (`building` is then spent)
        crabby::terrain_tile tile{};
    };
    crabby::cell_streamer cells{128, 600, 800};
    std::map<std::pair<int, int>, Tile> tiles;
    crabby::material material;

    /// Every frame: start up to 4 tiles, drop the far ones, and put up to
    /// 2 finished ones in the world.
    void update(crabby::ctx& ctx) {
        cells.update(ctx.camera3d().position, 4);
        for (auto cell : cells.to_load()) {
            auto corner = cells.corner(cell);
            tiles[{cell.x, cell.z}].building = std::async(std::launch::async, [corner] {
                // 65 x 65 heights, 2 m apart.
                std::vector<float> heights(65 * 65);
                for (std::size_t i = 0; i < heights.size(); ++i)
                    heights[i] = hill(corner.x + float(i % 65) * 2, corner.z + float(i / 65) * 2);
                return crabby::terrain_tile_data(heights, 128);
            });
        }
        for (auto cell : cells.to_drop()) {
            auto found = tiles.find({cell.x, cell.z});
            if (found == tiles.end()) continue;
            if (!found->second.building.valid()) {
                ctx.world().despawn(found->second.entity);
                ctx.free_terrain_tile(found->second.tile);
            }
            tiles.erase(found);   // a tile still building is waited for here
        }
        int uploads = 0;
        for (auto& [cell, t] : tiles) {
            if (uploads == 2) break;
            if (!t.building.valid() || t.building.wait_for(std::chrono::seconds(0)) != std::future_status::ready) continue;
            t.tile = ctx.upload_terrain_tile(t.building.get(), material);
            t.entity = ctx.world().spawn(crabby::transform3d_at(cells.corner({cell.first, cell.second})), t.tile);
            ++uploads;
        }
    }
};
// ANCHOR_END: streaming

static void the_village_has_five_spinning_houses() {
    auto world = crabby::game("guide", 640, 480).sim_system(spin, "spin").run_headless(60, [](crabby::ctx& ctx) {
        village(ctx);
        golden_hour(ctx);
        misty_morning(ctx);
        meadow(ctx);
        if (ctx.post().tonemap != uint32_t(crabby::tonemap::agx)) { std::fprintf(stderr, "post\n"); std::abort(); }
    });
    int houses = 0;
    world.each([&](const crabby::transform3d& t, const Spin&) {
        ++houses;
        if (std::abs(t.rotation.y - std::sin(0.25f)) > 1e-3f) { std::fprintf(stderr, "rotation %f\n", t.rotation.y); std::abort(); }
    });
    if (houses != 5) { std::fprintf(stderr, "%d houses\n", houses); std::abort(); }
}

static void the_dive_adds_one_effect() {
    crabby::game("dive", 64, 64).run_headless(1, [](crabby::ctx& ctx) {
        dive(ctx);
        auto list = ctx.effects();
        if (list.size() != 1 || !list[0].has_material) { std::fprintf(stderr, "effects\n"); std::abort(); }
    });
}

static void the_ripples_run_on_a_render_target() {
    crabby::game("ripples", 64, 64).run_headless(1, [](crabby::ctx& ctx) {
        auto state = ripples(ctx);
        drop_stone(ctx, state.stones, {0.5f, 0.5f}, 1);
        auto list = ctx.compute_passes();
        if (list.size() != 1 || list[0].target.id != state.heights.id || !list[0].has_material) {
            std::fprintf(stderr, "ripples\n");
            std::abort();
        }
    });
}

static void the_pond_mirrors_the_camera() {
    crabby::game("pond", 64, 64).run_headless(1, [](crabby::ctx& ctx) {
        pond(ctx);
        auto cam = ctx.camera3d();
        cam.position = {3, 5, 1};
        ctx.set_camera3d(cam);
        follow_reflections(ctx);
        auto list = ctx.target_cameras();
        if (list.size() != 1 || list[0].camera.position.y != -5 || list[0].target.width != 32) {
            std::fprintf(stderr, "pond\n");
            std::abort();
        }
    });
}

static void the_hills_are_nine_tiles() {
    auto world = crabby::game("hills", 640, 480).run_headless(1, [](crabby::ctx& ctx) { hills(ctx); });
    int tiles = 0;
    world.each([&](const crabby::terrain_tile& t) {
        ++tiles;
        if (t.lod_distance != 128) { std::fprintf(stderr, "lod distance %f\n", t.lod_distance); std::abort(); }
    });
    if (tiles != 9) { std::fprintf(stderr, "%d tiles\n", tiles); std::abort(); }
}

static void the_ground_streams_in_and_out() {
    crabby::game("ground", 1, 1).run_headless(0, [](crabby::ctx& ctx) {
        Ground ground{};
        ground.material = ctx.create_material(crabby::material_of(crabby::colors::white));
        auto live = [&] { std::size_t n = 0; for (auto& [c, t] : ground.tiles) n += !t.building.valid(); return n; };
        for (int i = 0; i < 100000 && (live() < ground.tiles.size() || ground.tiles.empty()); ++i) ground.update(ctx);
        auto first = live();
        if (first < 20) { std::fprintf(stderr, "%zu tiles\n", first); std::abort(); }
        auto camera = ctx.camera3d();
        camera.position = {10000, 0, 0};
        ctx.set_camera3d(camera);
        ground.update(ctx);
        if (ground.cells.to_drop().size() != first) { std::fprintf(stderr, "dropped %zu\n", ground.cells.to_drop().size()); std::abort(); }
    });
}

int main() {
    the_ripples_run_on_a_render_target();
    the_pond_mirrors_the_camera();
    the_hills_are_nine_tiles();
    the_ground_streams_in_and_out();
    the_dive_adds_one_effect();
    the_beach_has_five_palms();
    the_village_has_five_spinning_houses();
    things_fall_ten_units_per_second();
    turrets_ride_on_tanks();
    crowds_reach_the_goal();
    the_dead_are_buried();

    // Run the other snippets headless once, so they're exercised, not only compiled.
    crabby::sound click{};
    crabby::entity map = 0;
    crabby::entity ordered = 0;
    std::uint32_t most = 0;
    auto world = crabby::game("guide", 1, 1)
        .spatial_grid(10)
        .sim_system(die, "die")
        .sim_system(count_neighbors, "neighbors")
        .sim_system(wander, "wander")
        .sim_system(Spawner{2}, "spawner")
        .sim_system(think, "think")
        .sim_system(send_orders, "orders")
        .sim_system(obey_orders, "obey")
        .system(Hud{}, "hud")
        .sim_system(move_players, "players")   // no pads headless: nothing moves
        .camera_controls()
        .sim_system([&](crabby::ctx& ctx) {
            ctx.world().each([&](const Neighbors& n) { most = std::max(most, n.count); });
        }, "check")
        .run_headless(8, [&](crabby::ctx& ctx) {
            ecs(ctx);
            auto w = ctx.world();
            for (int i = 0; i < 16; ++i) w.spawn(Brain{0});
            ordered = w.spawn(crabby::transform_at({0, 0}), Owner{0});
            click = beep(ctx);
            std::vector<std::uint8_t> pixels(64 * 40 * 4, 255);
            const auto sheet = crabby::sheet_of(ctx.create_texture(64, 40, pixels.data(), pixels.size()), 16, 16);
            animate(w, sheet, {0, 0});
            map = make_map(w, sheet, 50);
            paint(ctx, map);   // nobody holds the mouse headless: nothing changes
            ctx.play_sound(click);   // silent: headless
            for (int i = 0; i < 100; ++i)
                w.spawn(crabby::transform_at({float(i), 0}), Neighbors{0}, Heading{0}, Health{float(i % 2)});
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
    world.each([&](const Brain& b) { plans += b.plans; });
    if (plans != 16 || std::fabs(click.seconds - 0.1f) > 1e-3f) {
        std::fprintf(stderr, "unexpected: %u plans, %f s click\n", plans, double(click.seconds));
        return 1;
    }
    // 100k from ecs() + 16 brains + 50 survivors + 4 spawned by Spawner{2} in 8 ticks + 1 animated + 1 map + 1 ordered.
    if (world.size() != 100000 + 16 + 50 + 4 + 3 || most < 10) {
        std::fprintf(stderr, "unexpected: %zu entities, %u neighbors\n", world.size(), most);
        return 1;
    }
    // Reflected components survive a save and load with their ids.
    {
        auto w = crabby::game("save", 1, 1).run_headless(0, [](crabby::ctx&) {});
        reflect_units(w);
        const crabby::entity a = w.spawn(Unit{3, 10, {1, 2}, 0, false});
        const crabby::entity b = w.spawn(Unit{5, 10, {0, 0}, a, true});
        const std::string before = w.save();
        w.set(b, Unit{0, 0, {0, 0}, 0, false});
        save_and_load(w);   // saves the changed world and loads it straight back
        w.load(before);
        const Unit u = w.get<Unit>(b);
        if (u.current != 5 || u.target != a || !u.angry || before.find("\"angry\":true") == std::string::npos) {
            std::fprintf(stderr, "save/load lost data: %s\n", before.c_str());
            return 1;
        }
    }
    // Resources: plain structs by type, saved when reflected.
    {
        auto w = crabby::game("resources", 1, 1).run_headless(0, [](crabby::ctx&) {});
        reflect_teams(w);
        w.set_resource(Teams{2, 3});
        if (w.save().find("\"Teams\":{\"blue\":3,\"red\":2}") == std::string::npos) {
            std::fprintf(stderr, "resource not saved: %s\n", w.save().c_str());
            return 1;
        }
    }
    if (!(world.get<crabby::transform2d>(ordered).position == crabby::vec2{100, 50})) {
        std::fprintf(stderr, "offline orders were not applied\n");
        return 1;
    }
    if (!physics_snippets()) {
        std::fprintf(stderr, "physics snippets failed\n");
        return 1;
    }
    (void)&multiplayer;
    (void)&score_point;
    (void)&my_game;
    (void)&controls;
    std::puts("guide snippets ok");
}
