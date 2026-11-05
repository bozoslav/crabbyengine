#include "crabby.hpp"
#include <algorithm>
#include <cmath>
#include <atomic>
#include <cstdlib>
#include <filesystem>
#include <future>
#include <fstream>
#include <iostream>
#include <optional>
#include <string>
#include <thread>
#include <type_traits>
#include <vector>
#define CHECK(...) do { if (!(__VA_ARGS__)) { std::cerr << "failed: " #__VA_ARGS__ << " (line " << __LINE__ << ")\n"; std::abort(); } } while (false)
struct Pos { float x, y; };
struct Vel { float x, y; };
struct Tag {};
namespace game { struct Health { float value; }; }
namespace other { struct Health { int value; }; }
template<class F> void throws(F f) { bool caught = false; try { f(); } catch (const std::exception&) { caught = true; } CHECK(caught); }

static void worlds() {
    static_assert(!std::is_copy_constructible<crabby::world>::value, "world owns its handle");
    crabby::world a;
    auto e = a.spawn(Pos{1, 2}, Vel{3, 4});
    CHECK(a.size() == 1 && a.alive(e));
    a.each([&](crabby::entity id, Pos& p, const Vel& v) { p.x += v.x; CHECK(id == e); });
    CHECK(a.get<Pos>(e).x == 4);
    a.query<Pos, Vel>([&](auto chunk) {
        CHECK(chunk.size() == 1);
        auto [ps, vs] = chunk.columns();
        ps[0].y += vs[0].y;
        CHECK(chunk.entity(0) == e);
        throws([&] { (void)chunk.entity(1); });
    });
    CHECK(a.get<Pos>(e).y == 6);
    a.set(e, Pos{9, 8}); CHECK(a.get<Pos>(e).y == 8);
    // try_get, has, add and remove.
    CHECK(!a.has<Tag>(e) && !a.try_get<Tag>(e));
    a.set(e, Tag{}); CHECK(a.has<Tag>(e));
    CHECK(a.count<Pos>() == 1 && a.count<Pos, Tag>() == 1 && a.count<Vel, Tag>() == 1);
    a.remove<Tag>(e); CHECK(!a.has<Tag>(e) && a.count<Pos, Tag>() == 0);
    throws([&] { (void)a.get<Tag>(e); });
    // A function object works as well as a lambda.
    struct Move { void operator()(Pos& p, const Vel& v) const { p.x += v.x; } };
    a.each(Move{}); CHECK(a.get<Pos>(e).x == 12);
    // Exceptions from callbacks come out unchanged; world calls inside a loop throw.
    bool original_exception = false;
    try { a.each([](Pos&) { throw std::runtime_error("callback"); }); }
    catch (const std::runtime_error& error) { original_exception = std::string(error.what()) == "callback"; }
    CHECK(original_exception);
    throws([&] { a.each([&](Pos&) { (void)a.size(); }); });
    throws([&] { a.query<Pos, Pos>([](auto) {}); });
    // Moving worlds keeps entities and component ids.
    crabby::world c(std::move(a)); CHECK(c.get<Pos>(e).x == 12);
    throws([&] { (void)a.size(); });
    crabby::world b;
    b = std::move(c); CHECK(b.get<Pos>(e).x == 12);
    b.remove<Vel>(e); throws([&] { (void)b.get<Vel>(e); });
    b.despawn(e); CHECK(!b.alive(e)); throws([&] { b.despawn(e); });
    CHECK(!b.try_get<Pos>(e));
}

static void names_and_reflection() {
    // Components are named after their C++ type, namespaces included.
    CHECK(crabby::detail::type_name<game::Health>() == "game::Health");
    CHECK(crabby::detail::type_name<Pos>() == "Pos");
    crabby::world w;
    w.reflect<game::Health>({crabby::field("value", &game::Health::value)});
    auto e = w.spawn(game::Health{5}, other::Health{7});
    CHECK(w.get<game::Health>(e).value == 5 && w.get<other::Health>(e).value == 7);
    const std::string saved = w.save();
    CHECK(saved.find("\"game::Health\":{\"value\":5.0}") != std::string::npos);
    w.set(e, game::Health{1});
    w.load(saved);
    CHECK(w.get<game::Health>(e).value == 5);
    // A chosen name must come before first use, and can't be taken twice.
    crabby::world n;
    n.reflect<Vel>("Velocity", {crabby::field("x", &Vel::x), crabby::field("y", &Vel::y)});
    n.spawn(Vel{1, 2});
    CHECK(n.save().find("\"Velocity\":{\"x\":1.0,\"y\":2.0}") != std::string::npos);
    throws([&] { n.reflect<Pos>("Velocity", {crabby::field("x", &Pos::x)}); });
    n.spawn(Pos{});
    throws([&] { n.reflect<Pos>("Position", {crabby::field("x", &Pos::x)}); });
}

namespace game { struct Score { std::uint32_t red, blue; }; }
static void resources() {
    crabby::world w;
    CHECK(!w.try_resource<game::Score>());
    throws([&] { w.resource<game::Score>(); });
    w.set_resource(game::Score{1, 2});
    CHECK(w.resource<game::Score>().red == 1 && w.resource<game::Score>().blue == 2);
    w.set_resource(game::Score{3, 2});
    CHECK(w.resource<game::Score>().red == 3);
    // Described, it's saved with the entities and loaded back.
    w.reflect_resource<game::Score>({crabby::field("red", &game::Score::red), crabby::field("blue", &game::Score::blue)});
    const std::string saved = w.save();
    CHECK(saved.find("\"game::Score\":{\"blue\":2,\"red\":3}") != std::string::npos);
    w.set_resource(game::Score{0, 0});
    w.load(saved);
    CHECK(w.resource<game::Score>().red == 3);
    CHECK(w.remove_resource<game::Score>());
    CHECK(!w.remove_resource<game::Score>());
    CHECK(!w.try_resource<game::Score>());
}

namespace game { struct Tuning { float speed; std::uint32_t waves; }; }
static void data_files() {
    const auto file = std::filesystem::temp_directory_path() / "crabby-cpp-tuning.json";
    std::ofstream(file) << R"({"speed": 2.5, "waves": 3})";
    const std::string path = file.string();
    auto w = crabby::game("data", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        auto world = ctx.world();
        world.reflect_resource<game::Tuning>({crabby::field("speed", &game::Tuning::speed), crabby::field("waves", &game::Tuning::waves)});
        ctx.load_json<game::Tuning>(path.c_str());
        world.reflect<game::Tuning>({crabby::field("speed", &game::Tuning::speed), crabby::field("waves", &game::Tuning::waves)});
        const crabby::entity e = world.spawn(game::Tuning{0, 9});
        ctx.load_json<game::Tuning>(path.c_str(), e);
        CHECK(world.get<game::Tuning>(e).speed == 2.5f);
    });
    CHECK(w.resource<game::Tuning>().speed == 2.5f && w.resource<game::Tuning>().waves == 3);
    std::filesystem::remove(file);
}

// Saved under the executable's assets/ copy (build output), so nothing to clean.
static void scenes_and_prefabs() {
    crabby::entity first = 0;
    std::vector<crabby::entity> palms;
    auto w = crabby::game("scenes", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        auto world = ctx.world();
        world.reflect<game::Health>({crabby::field("value", &game::Health::value)});
        // Sprites are saved by their image's name, and spawning loads it.
        auto model = world.spawn(crabby::transform_at({70, 80}), game::Health{9}, crabby::sprite_of(ctx.load_texture("crab.png"), crabby::colors::white, 4));
        ctx.save_prefab(model, "test-cpp-prefab.json");
        world.despawn(model);
        for (float x : {0.0f, 10.0f}) palms.push_back(ctx.spawn_prefab("test-cpp-prefab.json", {x, 100}));
        throws([&] { ctx.spawn_prefab("test-cpp-missing.json", {0, 0}); });
        throws([&] { ctx.load_scene("test-cpp-missing.json"); });
        throws([&] { ctx.save_prefab(model, "test-cpp-dead.json"); });
        first = palms[0];
        ctx.save_scene("test-cpp-scene.json");
    });
    CHECK(w.size() == 2);
    CHECK(w.get<crabby::transform2d>(palms[1]).position.x == 10 && w.get<game::Health>(palms[1]).value == 9);
    CHECK(w.get<crabby::sprite>(palms[1]).z == 4);
    auto loaded = crabby::game("scenes", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        ctx.world().reflect<game::Health>({crabby::field("value", &game::Health::value)});
        ctx.world().spawn(Tag{});   // replaced by the scene
        ctx.load_scene("test-cpp-scene.json");
    });
    CHECK(loaded.size() == 2 && loaded.get<game::Health>(first).value == 9);
}

static void three_d() {
    crabby::world lights;
    auto lamp = lights.spawn(crabby::transform3d_at({1, 2, 3}), crabby::point_light3d{crabby::colors::red, 20, 12});
    auto beam = lights.spawn(crabby::transform3d_at({0, 3, 0}), crabby::spot_light3d{crabby::colors::white, 40, 25, .2f, .5f});
    CHECK(lights.get<crabby::point_light3d>(lamp).range == 12);
    CHECK(lights.get<crabby::spot_light3d>(beam).inner_angle == .2f);
    lights.each([](crabby::point_light3d& p) { p.intensity += 1; });
    CHECK(lights.get<crabby::point_light3d>(lamp).intensity == 21);
    std::vector<crabby::entity> cubes;
    crabby::mesh cube{};
    auto spin = [](crabby::ctx& ctx) {
        ctx.world().each([](crabby::transform3d& t, const crabby::model&) { t.rotation = crabby::quat_from_rotation_y(0.1f) * t.rotation; });
    };
    auto w = crabby::game("3d", 640, 480).sim_system(spin, "spin").run_headless(5, [&](crabby::ctx& ctx) {
        auto stone = ctx.create_material(crabby::material_of(crabby::rgb(.5f, .5f, .5f)));
        cube = ctx.create_mesh(crabby::mesh_data::cube(1), stone);
        for (int i = 0; i < 3; ++i)
            cubes.push_back(ctx.world().spawn(crabby::transform3d_at({float(i) * 2, 0, 0}), crabby::model_of(cube, crabby::colors::red)));
        throws([&] { ctx.create_mesh(crabby::mesh_data::cube(1), crabby::material{99}); });
        auto shiny = crabby::material_of(crabby::colors::white);
        shiny.metallic = 2;
        throws([&] { ctx.create_material(shiny); });
        throws([] { crabby::mesh_data({{0, 0, 0}, {1, 0, 0}, {0, 1, 0}}, {0, 1}); });
        auto tile = ctx.create_terrain_tile(std::vector<float>(81, 0.0f), 16, stone);
        CHECK(tile.lod_distance == 16 && tile.lods[0].id != tile.lods[3].id);
        ctx.world().spawn(crabby::transform3d_at({0, 0, 0}), tile);
        throws([&] { ctx.create_terrain_tile(std::vector<float>(80, 0.0f), 16, stone); });
        throws([] { crabby::mesh_data::heightfield({0, 0, 0}, 1); });
        auto layers = ctx.create_texture_array(1, 1, std::vector<std::uint8_t>(8, 200), crabby::texture_kind::color);
        CHECK(layers.layers == 2);
        auto weights = ctx.create_texture_array(1, 1, std::vector<std::uint8_t>(4, 255), crabby::texture_kind::data);
        auto ground = ctx.create_terrain_material(crabby::terrain_material_of(layers, weights, {0, 0}, {16, 16}));
        CHECK(ground.id != stone.id);
        throws([&] { ctx.create_texture_array(1, 1, std::vector<std::uint8_t>(6, 0), crabby::texture_kind::color); });
        throws([&] { ctx.create_terrain_material(crabby::terrain_material_of(layers, weights, {0, 0}, {0, 0})); });
        auto grey = ctx.create_effect("grey", "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(p.color.g); }");
        ctx.set_effects({crabby::screen_effect_of(grey), crabby::screen_effect_of(grey, stone)});
        CHECK(ctx.effects().size() == 2 && ctx.effects()[1].material.id == stone.id);
        throws([&] { ctx.create_effect("bad", "fn effect(p: Pixel) -> f32 { return 1.0; }"); });
        throws([&] { ctx.set_effects({crabby::screen_effect_of(crabby::effect{99})}); });
        ctx.set_effects({});
        CHECK(ctx.effects().empty());
        auto mirror = ctx.create_render_target(32, 16);
        CHECK(mirror.width == 32 && mirror.height == 16);
        auto eye = ctx.camera3d();
        eye.position = {0, 5, 0};
        auto below = ctx.reflection_camera(eye, mirror, 1);
        CHECK(below.camera.position.y == -3 && below.has_clip_plane && below.clip_plane[3] == -1);
        ctx.set_target_cameras({below, crabby::target_camera_of(eye, mirror)});
        CHECK(ctx.target_cameras().size() == 2 && !ctx.target_cameras()[1].has_clip_plane);
        const std::uint8_t px[4] = {255, 255, 255, 255};
        auto plain = ctx.create_texture(1, 1, px, sizeof px);
        throws([&] { ctx.set_target_cameras({crabby::target_camera_of(eye, plain)}); });
        throws([&] { ctx.create_render_target(0, 4); });
        ctx.set_target_cameras({});
        auto fade = ctx.create_compute("fade", "fn compute(c: Cell) -> vec4<f32> { return c.value * 0.9; }");
        ctx.set_compute_passes({crabby::compute_pass_of(fade, mirror), crabby::compute_pass_of(fade, mirror, stone)});
        CHECK(ctx.compute_passes().size() == 2 && ctx.compute_passes()[1].material.id == stone.id);
        throws([&] { ctx.create_compute("bad", "fn compute(c: Cell) -> f32 { return 1.0; }"); });
        throws([&] { ctx.set_compute_passes({crabby::compute_pass_of(fade, plain)}); });
        throws([&] { ctx.set_compute_passes({crabby::compute_pass_of(crabby::compute{9}, mirror)}); });
        ctx.set_compute_passes({});

        auto cam = ctx.camera3d();
        cam.position = {0, 5, 10};
        ctx.set_camera3d(cam);
        ctx.camera3d_look_at({0, 0, 0});
        CHECK(crabby::forward(crabby::looking_at(crabby::transform3d_at(cam.position), {0, 0, 0})).z < 0);
        cam.near = 0;
        throws([&] { ctx.set_camera3d(cam); });
        auto light = ctx.lighting();
        CHECK(light.shadows && light.shadow_cascades == 4);
        light.sun_direction = {-1, -1, 0};
        light.shadow_cascades = 2;
        ctx.set_lighting(light);
        CHECK(ctx.lighting().shadow_cascades == 2);
        light.shadow_cascades = 0;
        throws([&] { ctx.set_lighting(light); });
        auto sky = ctx.sky();
        CHECK(sky.atmosphere && sky.fog_density == 0);
        sky.fog_density = 0.01f;
        ctx.set_sky(sky);
        CHECK(ctx.sky().fog_density == 0.01f);
        CHECK(sky.fog_end == 0 && sky.fog_sun_focus == 0.7f);
        sky.fog_start = 500;
        sky.fog_end = 1200;
        ctx.set_sky(sky);
        CHECK(ctx.sky().fog_end == 1200);
        sky.fog_sun_focus = 1;
        throws([&] { ctx.set_sky(sky); });
        sky.fog_sun_focus = 0.7f;
        sky.sun_size = 0;
        throws([&] { ctx.set_sky(sky); });
        CHECK(crabby::sun_direction_at(12).y < -0.8f);
        auto post = ctx.post();
        CHECK(post.tonemap == uint32_t(crabby::tonemap::aces) && post.contrast == 1 && !post.has_lut);
        post.tonemap = uint32_t(crabby::tonemap::neutral);
        post.bloom = 0.2f;
        ctx.set_post(post);
        CHECK(ctx.post().tonemap == uint32_t(crabby::tonemap::neutral));
        post.saturation = -1;
        throws([&] { ctx.set_post(post); });

        auto tiles = ctx.create_shader("tiles", R"(
            fn surface(s: Surface) -> Surface {
                var out = s;
                out.albedo *= step(0.5, fract(s.uv.x * 8.0));
                return out;
            })");
        auto look = crabby::material_of(crabby::rgb(1, 1, 1));
        look.has_shader = true;
        look.shader = tiles;
        look.blend = uint32_t(crabby::blend::transparent);
        auto tiled = ctx.create_material(look);
        look.double_sided = true;
        ctx.update_material(tiled, look);
        throws([&] { ctx.create_shader("broken", "fn surface(s: Surface) -> Surface { return 1; }"); });
    });
    const auto t = w.get<crabby::transform3d>(cubes[1]);
    CHECK(t.position.x == 2 && std::abs(t.rotation.y - std::sin(0.25f)) < 1e-5f);
    CHECK(w.get<crabby::model>(cubes[2]).mesh.id == cube.id);
    // Quaternion math matches the engine's.
    const auto q = crabby::quat_from_rotation_y(1.5707964f);
    const auto v = q * crabby::vec3{1, 0, 0};
    CHECK(std::abs(v.z + 1) < 1e-5f);
    auto tri = crabby::mesh_data({{0, 0, 0}, {1, 0, 0}, {0, 1, 0}}, {0, 1, 2});
    CHECK(tri.view().vertex_count == 3 && tri.view().normals == nullptr);
    tri.set_colors({{1, 0, 0, 1}, {0, 1, 0, 1}, {0, 0, 1, 1}});
    throws([&] { tri.set_colors({{1, 1, 1, 1}}); });
}

static void parallel_queries() {
    crabby::world w;
    for (int i = 0; i < 100000; ++i) w.spawn(i);
    std::atomic<long long> rows{0};
    w.query_par<int>([&](auto chunk) {
        auto [col] = chunk.columns();
        for (std::size_t i = 0; i < chunk.size(); ++i) col[i] *= 2;
        rows += (long long)chunk.size();
    });
    CHECK(rows == 100000);
    w.each_par([](int& n) { n += 1; });
    long long sum = 0;
    w.each([&](int n) { sum += n; });
    CHECK(sum == 100000LL * 99999LL + 100000);
    throws([&] { w.each_par([](int&) { throw std::runtime_error("par"); }); });

    // Filling a buffer from all cores: rows are numbered the same way in both queries.
    for (int i = 0; i < 5000; ++i) w.spawn(-1, 'x');
    CHECK(w.count<int>() == 105000 && w.count<int, char>() == 5000);
    std::vector<int> seq(w.count<int>()), par(w.count<int>());
    w.query<int>([&](auto chunk) { auto [n] = chunk.columns(); for (std::size_t i = 0; i < chunk.size(); ++i) seq[chunk.first() + i] = n[i]; });
    w.query_par<int>([&](auto chunk) { auto [n] = chunk.columns(); for (std::size_t i = 0; i < chunk.size(); ++i) par[chunk.first() + i] = n[i]; });
    CHECK(seq == par);
    CHECK(std::count(seq.begin(), seq.end(), -1) == 5000);
}

struct Hp { int value; };
struct Bullet { float speed; };

static void games() {
    // Systems, the clock, quit and exceptions.
    std::uint64_t ticks = 0; float dt = 0;
    auto sim = crabby::game("t", 1, 1).fixed_hz(50)
        .sim_system([&](crabby::ctx& ctx) {
            ++ticks; dt = ctx.dt();
            CHECK(ctx.gamepads().empty());   // no pads in headless runs
            if (ctx.sim_tick() == 2) ctx.quit();
        }, "count")
        .run_headless(100, [&](crabby::ctx& ctx) { ctx.world().spawn(Pos{1, 1}); });
    CHECK(ticks == 3 && dt > .019f && dt < .021f && sim.size() == 1);
    // The returned world knows the game's components.
    sim.each([](const Pos& p) { CHECK(p.x == 1); });
    throws([] { crabby::game("t", 1, 1).run_headless(1, [](crabby::ctx&) { throw std::runtime_error("setup"); }); });
    bool second = false;
    throws([&] { crabby::game("t", 1, 1).sim_system([](crabby::ctx&) { throw std::runtime_error("tick"); })
                     .sim_system([&](crabby::ctx&) { second = true; }).run_headless(5); });
    CHECK(!second);
    throws([] { crabby::game("t", 1, 1).fixed_hz(0); });
    throws([] { crabby::game("t", 1, 1).remove_system("nope"); });
    crabby::game g("t", 1, 1);
    g.run_headless(1);
    throws([&] { g.run_headless(1); });

    // Commands from inside a loop (Bullet is first used there); spatial
    // queries; random streams; built-ins.
    std::vector<crabby::grid_hit> hits;
    std::size_t near = 0;
    std::uint64_t first_random = 0;
    auto w = crabby::game("t", 1, 1).spatial_grid(16).seed(3)
        .sim_system([&](crabby::ctx& ctx) {
            ctx.world().each([&](crabby::entity e, const Hp& hp) {
                if (hp.value <= 0) { ctx.despawn_later(e); ctx.spawn_later(Hp{100}, Bullet{2}); }
            });
            ctx.spatial_radius({45, 0}, 16, hits);
            near = hits.size();
            CHECK(ctx.spatial_nearest({1, 0}, 100).has_value());
            CHECK(!ctx.spatial_nearest({1e6f, 0}, 1).has_value());
            first_random = ctx.random().of(7).u64();
            ctx.quit();
        })
        .run_headless(10, [&](crabby::ctx& ctx) {
            auto world = ctx.world();
            std::uint8_t px[16] = {};
            auto tex = ctx.create_texture(2, 2, px, sizeof px);
            for (int x = 0; x < 10; ++x)
                world.spawn(Hp{x % 2 ? 5 : 0}, crabby::transform_at({x * 10.0f, 0}),
                            crabby::sprite_of(tex, crabby::colors::red, float(x)));
        });
    CHECK(w.size() == 10);
    CHECK(near == 4);
    long long total = 0;
    w.each([&](const Hp& hp) { total += hp.value; });
    CHECK(total == 5 * 5 + 5 * 100);
    CHECK(w.count<Hp, Bullet>() == 5);
    CHECK(first_random == crabby::random(0).of(7).u64() || first_random != 0);

    // set_later and remove_later, also for types first used inside a loop.
    auto changed = crabby::game("t", 1, 1)
        .sim_system([](crabby::ctx& ctx) {
            ctx.world().each([&](crabby::entity e, Hp& hp) {
                if (hp.value == 1) ctx.set_later(e, Vel{1, 1});
                if (hp.value == 2) ctx.remove_later<Tag>(e);
            });
        })
        .run_headless(1, [](crabby::ctx& ctx) { ctx.world().spawn(Hp{1}); ctx.world().spawn(Hp{2}, Tag{}); });
    CHECK(changed.count<Vel>() == 1 && changed.count<Tag>() == 0);
}

static void grid_views_match_engine_queries() {
    crabby::grid g(8);
    crabby::rng r(5);
    std::vector<crabby::grid_item> items;
    for (std::uint32_t i = 0; i < 20000; ++i)
        items.push_back({{i, 0}, {r.range(-500, 500), r.range(-500, 500)}, i});
    g.rebuild(items);
    auto view = g.view();
    CHECK(view.size() == items.size());
    std::vector<crabby::grid_item> engine;
    for (int q = 0; q < 300; ++q) {
        const crabby::vec2 c{r.range(-600, 600), r.range(-600, 600)};
        const float radius = q % 50 == 0 ? 2000.0f : r.range(0, 40);
        g.radius(c, radius, engine);
        std::vector<std::uint32_t> a, b;
        for (auto& e : engine) a.push_back(e.data);
        view.for_each_in_radius(c, radius, [&](const crabby::grid_item& e) { b.push_back(e.data); });
        std::sort(a.begin(), a.end());
        std::sort(b.begin(), b.end());
        CHECK(a == b);
    }
    throws([&] { crabby::grid_view<crabby::grid_hit> wrong(CrabbyGridView{nullptr, 20, nullptr, nullptr, 1, 0, 0, 0, 0, 0}); });
}

struct Died { crabby::entity who; std::uint64_t tick; };
struct Never { int x; };

static void events() {
    // Sent from inside a loop (the first send is the type's first use), read
    // by a later system in the same tick, once each.
    std::vector<std::uint64_t> seen;
    auto w = crabby::game("t", 1, 1)
        .sim_system([](crabby::ctx& ctx) {
            ctx.world().each([&](crabby::entity e, const Hp& hp) {
                if (hp.value <= 0) ctx.send(Died{e, ctx.sim_tick()});
            });
        }, "deaths")
        .sim_system([&](crabby::ctx& ctx) {
            for (const Died& d : ctx.read<Died>()) { seen.push_back(d.tick); ctx.despawn_later(d.who); }
            CHECK(ctx.read<Died>().empty());
            CHECK(ctx.read<Never>().empty());
        }, "funerals")
        .run_headless(3, [](crabby::ctx& ctx) {
            for (int h : {0, 5, -1}) ctx.world().spawn(Hp{h});
        });
    CHECK(w.size() == 1);
    CHECK((seen == std::vector<std::uint64_t>{0, 0}));
}

static void rng_matches_rust() {
    // The same numbers as Rust's Rng (pinned in src/random.rs).
    crabby::rng r(42);
    CHECK(r.u64() == 0xc757960b442b0ac3ull);
    CHECK(r.u64() == 0x4bb22a7f77ff8c6cull);
    CHECK(r.u64() == 0x04950439d3c5eafeull);
    CHECK(crabby::random(7).stream(3).u64() == 0x6045c0e00af2e148ull);
    crabby::rng f(1);
    for (int i = 0; i < 1000; ++i) { float x = f.f32(); CHECK(x >= 0 && x < 1); CHECK(f.below(10) < 10); }
}

static void attachments() {
    // Chains follow their root; orphans are despawned with it.
    crabby::entity root = 0, arm = 0, hand = 0;
    crabby::transform2d seen{};
    auto w = crabby::game("t", 1, 1)
        .sim_system([&](crabby::ctx& ctx) {
            auto world = ctx.world();
            auto t = world.get<crabby::transform2d>(root);
            t.position.x += 1;
            world.set(root, t);
            if (ctx.sim_tick() == 2) ctx.despawn_later(root);
        })
        .follow_attachments()
        .sim_system([&](crabby::ctx& ctx) {
            if (ctx.sim_tick() == 1) seen = ctx.world().get<crabby::transform2d>(hand);
        })
        .run_headless(3, [&](crabby::ctx& ctx) {
            auto world = ctx.world();
            root = world.spawn(crabby::transform_at({5, 5}));
            arm = world.spawn(crabby::transform_at({0, 0}), crabby::attach_to(root, {1, 0}));
            hand = world.spawn(crabby::transform_at({0, 0}), crabby::attach_to(arm, {0, 1}, 0.5f));
        });
    CHECK(seen.position == (crabby::vec2{8, 6}) && seen.rotation == 0.5f);
    CHECK(crabby::bits(crabby::entity_of(arm)) == arm);
    CHECK(!w.alive(root) && !w.alive(arm) && !w.alive(hand));
}

static void physics() {
    // A ball drops through a sensor onto the floor; queries and events work from C++.
    crabby::entity ball = 0, floor = 0, sensor = 0;
    std::vector<crabby::collision> started;
    std::optional<crabby::ray_hit> hit;
    std::optional<float> mass;
    bool kicked_nothing = true;
    auto w = crabby::game("t", 1, 1)
        .physics({0, 980})
        .sim_system([&](crabby::ctx& ctx) {
            for (const auto& c : ctx.collisions_started()) started.push_back(c);
            if (ctx.sim_tick() == 1) {
                hit = ctx.cast_ray({150, 0}, {0, 1}, 1000);
                mass = ctx.mass(ball);
                kicked_nothing = ctx.apply_impulse(12345, {1, 0});
                std::vector<CrabbyEntity> at;
                ctx.entities_at({150, 300}, at);
                CHECK(at.size() == 1 && crabby::bits(at[0]) == floor);
            }
        })
        .run_headless(120, [&](crabby::ctx& ctx) {
            auto world = ctx.world();
            auto bouncy = crabby::circle_collider(10);
            bouncy.restitution = 0;
            ball = world.spawn(crabby::transform_at({0, 0}), crabby::dynamic_body(), bouncy, crabby::velocity{});
            auto trigger = crabby::rect_collider({80, 20});
            trigger.sensor = true;
            sensor = world.spawn(crabby::transform_at({0, 100}), crabby::fixed_body(), trigger);
            floor = world.spawn(crabby::transform_at({0, 300}), crabby::fixed_body(), crabby::rect_collider({400, 20}));
        });
    CHECK(std::fabs(w.get<crabby::transform2d>(ball).position.y - 280) < 2);
    CHECK(std::fabs(w.get<crabby::velocity>(ball).linear.y) < 5);
    CHECK(w.get<crabby::rigid_body>(floor).kind == CRABBY_BODY_FIXED);
    CHECK(w.get<crabby::collider>(sensor).sensor);
    auto touched = [&](crabby::entity a, crabby::entity b) {
        return std::any_of(started.begin(), started.end(), [&](const crabby::collision& c) {
            const auto x = crabby::bits(c.a), y = crabby::bits(c.b);
            return (x == a && y == b) || (x == b && y == a);
        });
    };
    CHECK(touched(ball, sensor) && touched(ball, floor));

    // A wedge, flat side down, comes to rest on it (as tests/physics.rs).
    crabby::entity wedge = 0;
    auto rested = crabby::game("t", 1, 1).physics({0, 980}).run_headless(240, [&](crabby::ctx& ctx) {
        auto world = ctx.world();
        auto shape = crabby::polygon_collider({{-30, 15}, {30, 15}, {-30, -15}});
        shape.friction = 0.9f;
        wedge = world.spawn(crabby::transform_at({0, 0}), crabby::dynamic_body(), shape);
        world.spawn(crabby::transform_at({0, 200}), crabby::fixed_body(), crabby::rect_collider({400, 20}));
    });
    const auto t = rested.get<crabby::transform2d>(wedge);
    CHECK(std::fabs(t.position.y - 175) < 1.5f && std::fabs(t.rotation) < 0.01f);
    CHECK(rested.collider_points(wedge).size() == 3);
    CHECK(rested.get<crabby::collider>(wedge).shape == CRABBY_SHAPE_POLYGON);
    CHECK(rested.get<crabby::collider>(wedge).friction == 0.9f);
    throws([] { crabby::polygon_collider({{0, 0}, {1, 0}}); });
    CHECK(hit && crabby::bits(hit->entity) == floor && std::fabs(hit->distance - 290) < 1e-3f);
    CHECK(mass && std::fabs(*mass - 314.159f) < 1);
    CHECK(!kicked_nothing);
    // Bad values are rejected, not simulated.
    auto flat = crabby::rect_collider({10, 0});
    throws([&] { w.set(floor, flat); });
}

static void lockstep() {
    // Two games over UDP on localhost see the same inputs, tick for tick.
    struct Order { std::uint32_t tick; float x; };
    struct Seen { std::uint64_t sum = 0; std::uint32_t others = 0, player = 9; bool desync = false; };
    auto play = [](crabby::game& g, Seen& seen) {
        g.sim_system([&seen](crabby::ctx& ctx) {
            seen.player = ctx.player();
            ctx.set_input(Order{std::uint32_t(ctx.sim_tick()), float(ctx.player())});
            for (std::uint32_t p = 0; p < ctx.players(); ++p)
                if (auto o = ctx.input<Order>(p)) {
                    seen.sum += o->tick * (p + 1);
                    if (p != ctx.player()) ++seen.others;
                }
            seen.desync = ctx.desynced();
        });
        g.run_headless(200, [](crabby::ctx&) {});
    };
    std::uint16_t port = 0;
    Seen host_seen, guest_seen;
    crabby::game host("host", 1, 1);
    host.lockstep_host("127.0.0.1:0", 2, 2, 30, &port);
    CHECK(port != 0);
    std::thread host_thread([&] { play(host, host_seen); });
    crabby::game guest("guest", 1, 1);
    guest.lockstep_join(("127.0.0.1:" + std::to_string(port)).c_str());
    play(guest, guest_seen);
    host_thread.join();
    CHECK(host_seen.player == 0 && guest_seen.player == 1);
    CHECK(host_seen.sum == guest_seen.sum && host_seen.others > 150 && guest_seen.others > 150);
    CHECK(!host_seen.desync && !guest_seen.desync);
    throws([] { crabby::game("t", 1, 1).lockstep_host("127.0.0.1:0", 1); });
    throws([] { crabby::game("t", 1, 1).lockstep_steam_host(480, 2); });   // built without `steam`
}

static void streaming() {
    crabby::cell_streamer cells(10, 25, 40);
    cells.update({5, 0, 5}, 2);
    CHECK(cells.to_load().size() == 2 && cells.to_load()[0].x == 0 && cells.to_load()[0].z == 0);
    cells.update({5, 0, 5}, 1000);
    CHECK(cells.loaded_count() > 2 && cells.is_loaded({1, 0}) && !cells.is_loaded({9, 9}));
    cells.update({5000, 0, 5}, 1000);
    CHECK(!cells.to_drop().empty() && !cells.is_loaded({0, 0}));
    CHECK(cells.corner({2, -1}).x == 20 && cells.corner({2, -1}).z == -10);
    cells.clear();
    CHECK(cells.loaded_count() == 0);
    throws([] { crabby::cell_streamer(0, 1, 1); });
    throws([] { crabby::terrain_tile_data(std::vector<float>(8, 0.0f), 4); });
    crabby::heightfield slope({0, 2, 0, 2}, 1);
    CHECK(std::abs(*slope.height_at(0.5f, 0.5f) - 1) < 1e-5f && slope.normal_at(0.5f, 0.5f)->x < 0);
    CHECK(!slope.height_at(2, 0.5f) && !slope.normal_at(-1, 0));
    throws([] { crabby::heightfield({0, 0, 0}, 1); });
    crabby::game("stream", 64, 64).run_headless(0, [](crabby::ctx& ctx) {
        auto ground = ctx.create_material(crabby::material_of(crabby::colors::white));
        // Built on another thread, uploaded on the game's.
        auto data = std::async(std::launch::async, [] { return crabby::terrain_tile_data(std::vector<float>(33 * 33, 1.0f), 64); });
        auto tile = ctx.upload_terrain_tile(data.get(), ground);
        CHECK(tile.lod_distance == 64 && ctx.is_mesh_live(tile.lods[0]));
        ctx.free_terrain_tile(tile);
        CHECK(!ctx.is_mesh_live(tile.lods[0]) && !ctx.is_mesh_live(tile.lods[3]));
        auto cube = ctx.create_mesh(crabby::mesh_data::cube(1), ground);
        CHECK(ctx.mesh_shadows(cube));
        ctx.set_mesh_shadows(cube, false);
        CHECK(!ctx.mesh_shadows(cube));
        ctx.free_mesh(cube);
        ctx.free_mesh(cube);
        CHECK(!ctx.is_mesh_live(cube));
    });
}

int main() {
    worlds();
    names_and_reflection();
    resources();
    data_files();
    scenes_and_prefabs();
    three_d();
    streaming();
    parallel_queries();
    games();
    attachments();
    physics();
    lockstep();
    events();
    rng_matches_rust();
    grid_views_match_engine_queries();
    std::cout << "C++ wrapper tests passed\n";
}
