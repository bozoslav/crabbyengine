#include "crabby.hpp"
#include <algorithm>
#include <chrono>
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
    auto probes = crabby::game("probes", 1, 1).run_headless(0, [](crabby::ctx& ctx) {
        auto e = ctx.world().spawn(crabby::transform3d_at({0, 0, 0}), crabby::reflection_probe3d_default());
        ctx.bake_reflection_probe(e);
        auto p = ctx.world().get<crabby::reflection_probe3d>(e);
        CHECK(p.content == 1 && p.box_projection);
        ctx.bake_reflection_probe(e);
        CHECK(ctx.world().get<crabby::reflection_probe3d>(e).content == 1);
    });
    CHECK(probes.size() == 1);
    auto decals = crabby::game("decals", 1, 1).run_headless(0, [](crabby::ctx& ctx) {
        std::vector<std::uint8_t> pixels(2 * 2 * 4, 255);
        auto colors = ctx.create_texture_array(2, 2, pixels, crabby::texture_kind::color);
        throws([&] { ctx.set_decal_textures(colors); }); // decals want TextureKind::Data arrays
        auto images = ctx.create_texture_array(2, 2, pixels, crabby::texture_kind::data);
        ctx.set_decal_textures(images);
        CHECK(!ctx.supports_decals()); // no GPU in a headless run
        auto mark = crabby::decal3d_default();
        mark.size = {2, 2, 1};
        mark.order = 3;
        mark.normal_image = crabby::decal_no_image;
        auto e = ctx.world().spawn(crabby::transform3d_at({0, 1, 0}), mark, crabby::receives_decals{0});
        CHECK(ctx.world().get<crabby::decal3d>(e).order == 3 && ctx.world().get<crabby::decal3d>(e).size.x == 2);
        CHECK(ctx.world().get<crabby::receives_decals>(e).layers == 0);
        ctx.clear_decal_textures();
    });
    CHECK(decals.size() == 1);
    crabby::world lights;
    auto lamp = lights.spawn(crabby::transform3d_at({1, 2, 3}), crabby::point_light3d{crabby::colors::red, 20, 12, false});
    auto beam = lights.spawn(crabby::transform3d_at({0, 3, 0}), crabby::spot_light3d{crabby::colors::white, 40, 25, .2f, .5f, true});
    CHECK(lights.get<crabby::point_light3d>(lamp).range == 12);
    CHECK(lights.get<crabby::spot_light3d>(beam).inner_angle == .2f);
    CHECK(lights.get<crabby::spot_light3d>(beam).shadows);
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
        CHECK(post.anti_aliasing == 0 && !post.ao_enabled && post.ao_radius == 1);
        post.anti_aliasing = uint32_t(crabby::anti_aliasing::smaa);
        post.ao_enabled = true; post.ao_radius = 2; post.ao_intensity = 1.5f;
        post.ao_slices = 4; post.ao_steps = 8;
        ctx.set_post(post);
        CHECK(ctx.post().tonemap == uint32_t(crabby::tonemap::neutral));
        CHECK(ctx.post().ao_enabled && ctx.post().ao_radius == 2 && ctx.post().ao_steps == 8);
        CHECK(ctx.post().anti_aliasing == uint32_t(crabby::anti_aliasing::smaa));
        auto invalid_post = post; invalid_post.anti_aliasing = 3;
        throws([&] { ctx.set_post(invalid_post); });
        invalid_post = post; invalid_post.ao_radius = 0;
        throws([&] { ctx.set_post(invalid_post); });
        post.saturation = -1;
        throws([&] { ctx.set_post(post); });

        // Graphics quality presets, switched at run time.
        CHECK(ctx.post().render_scale == 1 && ctx.post().sharpen == 0);
        auto quality = crabby::graphics_quality_preset(crabby::quality_level::medium);
        CHECK(crabby::graphics_quality_level(quality) == crabby::quality_level::medium);
        CHECK(quality.shadow_cascades == 3 && quality.anti_aliasing == uint32_t(crabby::anti_aliasing::fxaa));
        ctx.set_graphics_quality(quality);
        CHECK(ctx.lighting().shadow_cascades == 3 && ctx.lighting().shadow_resolution == 2048);
        CHECK(ctx.graphics_quality().light_shadow_views == 4);
        quality = crabby::graphics_quality_preset(crabby::quality_level::low);
        ctx.set_graphics_quality(quality);
        CHECK(ctx.post().render_scale < 1 && ctx.lighting().light_shadow_views == 0);
        quality.shadow_cascades = 3;
        CHECK(crabby::graphics_quality_level(quality) == crabby::quality_level::custom);
        auto bad_quality = quality; bad_quality.render_scale = 4;
        throws([&] { ctx.set_graphics_quality(bad_quality); });
        throws([&] { crabby::graphics_quality_preset(crabby::quality_level::custom); });
        CHECK(ctx.graphics_quality().shadow_cascades == 2);
        ctx.set_graphics_quality(crabby::graphics_quality_preset(crabby::quality_level::high));
        auto light_q = ctx.lighting();
        light_q.shadow_resolution = 100;
        throws([&] { ctx.set_lighting(light_q); });

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

static void vehicles() {
    // A box car on raycast wheels settles on its suspension, then drives
    // off; the vehicle is a query column (as tests/vehicle3d.rs).
    crabby::entity car = 0;
    std::optional<crabby::vec3> center;
    auto w = crabby::game("t", 1, 1)
        .sim_system([](crabby::ctx& ctx) {
            if (ctx.sim_tick() < 100) return;
            ctx.world().each([](crabby::vehicle3d& v) {
                v.wheels[2].motor_torque = v.wheels[3].motor_torque = 500;
                v.wheels[0].steer_angle = v.wheels[1].steer_angle = 0.2f;
            });
        }, "drive")
        .physics3d({0, -9.81f, 0})
        .sim_system([&](crabby::ctx& ctx) {
            if (ctx.sim_tick() == 99) center = ctx.center_of_mass3d(car);
        }, "probe")
        .run_headless(200, [&](crabby::ctx& ctx) {
            auto world = ctx.world();
            world.spawn(crabby::transform3d_at({0, -0.5f, 0}), crabby::fixed_body3d(), crabby::box_collider({200, 1, 200}));
            const float x = 0.8f, y = -0.3f, z = 1.3f, r = 0.35f;
            car = world.spawn(crabby::transform3d_at({0, 1, 0}),
                              crabby::with_center_of_mass(crabby::with_mass(crabby::dynamic_body3d(), 1200), {0, -0.3f, 0}),
                              crabby::box_collider({1.8f, 0.6f, 4.2f}),
                              crabby::vehicle({crabby::wheel({-x, y, -z}, r), crabby::wheel({x, y, -z}, r),
                                               crabby::wheel({-x, y, z}, r), crabby::wheel({x, y, z}, r)}));
        });
    CHECK(center && std::fabs(center->y - 0.5f) < 0.02f);
    const auto v = w.get<crabby::vehicle3d>(car);
    const auto t = w.get<crabby::transform3d>(car);
    CHECK(v.wheel_count == 4 && v.wheels[0].grounded && crabby::rpm(v.wheels[3]) > 50);
    CHECK(t.position.z < -3 && t.position.x > 0.2f && t.position.y > 0.7f && t.position.y < 0.9f);
    CHECK(std::fabs(crabby::wheel_pose(v.wheels[0], t).position.y - 0.35f) < 0.05f);
    CHECK(crabby::evaluate(v.wheels[0].forward_friction, 0.4f) == 1);
    throws([] { crabby::vehicle({crabby::wheel({}, 1), crabby::wheel({}, 1), crabby::wheel({}, 1), crabby::wheel({}, 1),
                                 crabby::wheel({}, 1), crabby::wheel({}, 1), crabby::wheel({}, 1), crabby::wheel({}, 1),
                                 crabby::wheel({}, 1)}); });
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

static void lod_groups() {
    float fov = 60 * 3.14159265f / 180;
    CHECK(std::abs(crabby::lod_screen_height_at(2, 10, fov) - 0.1732f) < 1e-3f);
    CHECK(std::abs(crabby::lod_distance_at(2, 0.1732f, fov) - 10) < 0.01f);
    crabby::game("lod", 64, 64).run_headless(0, [](crabby::ctx& ctx) {
        auto white = ctx.default_material();
        auto high = ctx.create_mesh(crabby::mesh_data::cube(1), white);
        auto low = ctx.create_mesh(crabby::mesh_data::cube(2), white);
        auto e = ctx.world().spawn(crabby::transform3d_at({0, 0, 0}));
        crabby::lod_level a{0.5f, 0, {crabby::lod_renderer_of(high)}};
        crabby::lod_level b{0.1f, 0, {crabby::lod_renderer_of(low)}, true, {}};
        ctx.set_lod_group(e, 2, {a, b});
        auto at = [&](float d) {
            auto cam = ctx.camera3d();
            cam.position = {0, 0, d};
            ctx.set_camera3d(cam);
            ctx.camera3d_look_at({0, 0, 0});
            return ctx.lod_pick_of(e);
        };
        CHECK(at(1).kind == 1 && at(1).level == 0);
        CHECK(at(5).kind == 1 && at(5).level == 1);
        CHECK(at(100).kind == 0);
        ctx.set_lod({20, true});
        CHECK(at(5).level == 0 && ctx.lod().bias == 20);
        throws([&] { ctx.set_lod({-1, true}); });
        throws([&] { ctx.lod_pick_of(ctx.world().spawn(crabby::transform3d_at({0, 0, 0}))); });
    });
}

static void skeletons() {
    crabby::game("skeletons", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        auto arm = ctx.world().spawn(crabby::transform3d_at({0, 0, 0}));
        crabby::joint base{"base"}, tip{"tip", 0, crabby::transform3d_at({0, 1, 0})};
        ctx.set_skeleton(arm, {base, tip});
        CHECK(ctx.joint_count(arm) == 2);
        CHECK(ctx.find_joint(arm, "tip") == std::optional<std::size_t>(1));
        CHECK(!ctx.find_joint(arm, "tail"));
        // Bend the base a quarter turn: the tip swings from +Y to -X.
        auto pose = ctx.joint_pose(arm, 0);
        pose.rotation = crabby::quat_from_rotation_z(1.5707963f);
        ctx.set_joint_pose(arm, 0, pose);
        auto m = ctx.joint_matrix(arm, 1);
        CHECK(std::fabs(m[12] + 1) < 1e-5f && std::fabs(m[13]) < 1e-5f);
        ctx.reset_pose(arm);
        CHECK(std::fabs(ctx.joint_matrix(arm, 1)[13] - 1) < 1e-6f);
        throws([&] { ctx.joint_pose(arm, 2); });
        throws([&] { ctx.set_skeleton(arm, {crabby::joint{"loop", 0}}); });

        // A post: corners above the middle follow the tip.
        auto post = crabby::mesh_data::cube(0.2f);
        std::vector<std::array<std::uint16_t, 4>> joints;
        std::vector<std::array<float, 4>> weights;
        auto view = post.view();
        for (std::size_t i = 0; i < view.vertex_count; ++i) {
            joints.push_back({std::uint16_t(view.positions[i].y > 0 ? 1 : 0), 0, 0, 0});
            weights.push_back({1, 0, 0, 0});
        }
        post.set_skin(joints, weights);
        auto material = ctx.default_material();
        auto mesh = ctx.create_skinned_mesh(post, material, arm);
        ctx.world().spawn(crabby::model_of(mesh));
        auto hat = ctx.world().spawn(crabby::model_of(mesh));
        ctx.set_skin_of(hat, arm);
        auto cup = ctx.world().spawn(crabby::transform3d_at({0, 0, 0}));
        ctx.attach_to_joint(cup, arm, 1, crabby::transform3d_at({0, 0.1f, 0}));
        throws([&] { ctx.create_skinned_mesh(crabby::mesh_data::cube(1), material, arm); });
        joints.assign(joints.size(), {7, 0, 0, 0});
        post.set_skin(joints, weights);
        throws([&] { ctx.create_skinned_mesh(post, material, arm); });
        throws([&] { post.set_skin({{0, 0, 0, 0}}, {{1, 0, 0, 0}}); });
        throws([&] { ctx.create_skinned_mesh(post, material, cup); });
        throws([&] { ctx.load_clip("no/such.glb", "walk"); });
    });
}

static void morph_targets() {
    crabby::game("morph targets", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        // Two targets on a plane: one dents the whole sheet, one tips a corner.
        auto sheet = crabby::mesh_data::plane(1.0);
        crabby::morph_target dent{"dent", std::vector<crabby::vec3>(4, {0, 0, -0.7f}), {}};
        sheet.add_morph_target(dent);
        crabby::morph_target tip{"tip", std::vector<crabby::vec3>(4), {}}; // normals stay flat
        tip.positions[0] = {0, 0.4f, 0};
        sheet.add_morph_target(tip);
        sheet.add_morph_frame(0, 1, 0.5f);
        sheet.add_morph_frame(0, 0, 1.0f);
        throws([&] { sheet.add_morph_frame(0, 0, 1.0f); });
        throws([&] { sheet.add_morph_target(crabby::morph_target{"short", std::vector<crabby::vec3>(3), {}}); });

        auto mesh = ctx.create_mesh(sheet, ctx.default_material());
        CHECK(ctx.morph_target(mesh, "dent") == std::optional<std::size_t>(0));
        CHECK(ctx.morph_target(mesh, "tip") == std::optional<std::size_t>(1));
        CHECK(!ctx.morph_target(mesh, "sag"));

        auto e = ctx.world().spawn(crabby::transform3d_at({0, 0, 0}), crabby::model_of(mesh));
        CHECK(!ctx.morph_weight(e, 0)); // no weights: its mesh's defaults show
        ctx.set_morph_weight(e, 0, 0.5f);
        ctx.set_morph_weight(e, 1, 1.0f);
        CHECK(ctx.morph_weight(e, 0) == std::optional(0.5f));
        CHECK(ctx.morph_weight(e, 1) == std::optional(1.0f));
        CHECK(ctx.morph_weight(e, 2) == std::optional(0.0f));
        ctx.set_skeleton(e, {});
        crabby::clip face("face");
        face.set_morph_track("dent", {0, 2}, {0, 1});
        CHECK(face.duration() == 2);
        ctx.sample_clip(face, e, 1);
        CHECK(ctx.morph_weight(e, 0) == std::optional(0.5f));
        CHECK(ctx.morph_weight(e, 1) == std::optional(1.0f));
        throws([&] { face.set_morph_track("dent", {0, 1}, {0}); });
        ctx.write_animated_morphs();
        ctx.clear_morph_weights(e);
        CHECK(!ctx.morph_weight(e, 0));
        throws([&] { ctx.set_morph_weight(12345, 0, 0.5f); }); // no such entity
    });
}

static void sound3d() {
    crabby::game("sound3d", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        float samples[64] = {};
        auto hum = ctx.create_sound(44100, samples, 64);
        auto music = ctx.audio_bus("music");
        CHECK(music.index == ctx.audio_bus("music").index);
        CHECK(music.index != crabby::audio_bus{}.index);
        ctx.set_audio_bus_volume(music, 0.5f);
        ctx.mute_audio_bus(music, true);
        auto settings = crabby::audio_settings3d_defaults();
        CHECK(settings.max_distance == 100.0f && settings.doppler == 1.0f);
        settings.rolloff = CRABBY_ROLLOFF_LINEAR;
        settings.bus = music;
        auto e = ctx.world().spawn(crabby::transform3d_at({4, 0, 0}));
        ctx.set_audio_source3d(e, hum, settings);
        ctx.remove_audio_source3d(e);
        auto voice = ctx.play_sound_at(hum, {-4, 0, 0}, settings);
        ctx.set_sound_effects(voice, {800.0f, 0.3f});
        ctx.set_audio_listener(crabby::audio_listener3d{{7, 2, 3}, {0, 0, 0, 1}});
        CHECK(ctx.audio_listener().position.x == 7.0f);
        ctx.set_audio_listener(std::nullopt);
        CHECK(ctx.audio_listener().position.x != 7.0f);
        throws([&] { ctx.load_stream("missing.ogg"); });
    });
}

// A two-joint skin with one clip, "rise": the arm goes up 2 over 1 second.
static void animators() {
    const auto file = std::filesystem::temp_directory_path() / "crabby-cpp-rise.gltf";
    std::ofstream(file) << R"({"asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": [0]}],
        "nodes": [{"name": "root", "children": [1]}, {"name": "arm", "mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 2}}]}],
        "skins": [{"joints": [0, 1]}],
        "animations": [{"name": "rise", "samplers": [{"input": 0, "output": 1}],
            "channels": [{"sampler": 0, "target": {"node": 1, "path": "translation"}}]}],
        "buffers": [{"byteLength": 68, "uri": "data:application/octet-stream;base64,AAAAAAAAgD8AAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAAAAAACAPwAAAAA="}],
        "bufferViews": [{"buffer": 0, "byteLength": 8}, {"buffer": 0, "byteOffset": 8, "byteLength": 24},
            {"buffer": 0, "byteOffset": 32, "byteLength": 36}],
        "accessors": [{"bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR", "min": [0], "max": [1]},
            {"bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 1, 0]}]})";
    const std::string path = file.string();
    crabby::game("animators", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        auto rise = ctx.load_clip(path.c_str(), "rise");
        CHECK(rise.duration() == 1.0f);
        crabby::animation_graph_builder b;
        auto speed = b.add_float();
        auto still = b.add_state(crabby::motion::of(rise), 0);
        auto up = b.add_state(crabby::motion::of(rise), 1, std::nullopt, false);
        b.add_event(up, 0.5f, "half");
        auto go = b.add_transition(still, up, 0);
        b.add_condition(go, CRABBY_CONDITION_GREATER, speed, 0.5f);
        auto tree = crabby::motion::blend1d(speed);
        tree.add(0, crabby::motion::of(rise)).add(1, crabby::motion::of(rise), 2);
        b.add_state(std::move(tree));
        auto graph = b.build(still);
        throws([&] { crabby::animation_graph_builder bad; bad.build(3); });

        auto e = ctx.world().spawn(crabby::transform3d_at({0, 0, 0}));
        throws([&] { ctx.set_animator(e, graph); }); // no skeleton yet
        ctx.load_skeleton(e, path.c_str());
        ctx.set_animator(e, graph);
        ctx.animator_set_manual(e, true);
        auto arm = *ctx.find_joint(e, "arm");
        ctx.animator_update(e, 0.1f);
        CHECK(ctx.animator_state(e) == still);
        ctx.animator_set_float(e, speed, 1);
        ctx.animator_update(e, 0.0f); // takes the transition
        CHECK(ctx.animator_state(e) == up);
        ctx.animator_update(e, 0.75f);
        CHECK(std::fabs(ctx.joint_pose(e, arm).position.y - 1.5f) < 1e-4f);
        CHECK(ctx.animator_normalized_time(e) == 0.75f);
        CHECK(ctx.animator_next_event(e) == std::optional<std::string>("half"));
        CHECK(!ctx.animator_next_event(e));
        throws([&] { ctx.animator_set_bool(e, speed, true); }); // speed is a float
        ctx.animator_play(e, still);
        ctx.animator_update(e, 0.0f);
        CHECK(ctx.animator_state(e) == still);

        ctx.animator_set_float(e, speed, 0); // the base stays still
        crabby::animation_graph_builder lb;
        lb.add_state(crabby::motion::of(rise), 1, std::nullopt, false);
        auto overlay = lb.build(0);
        auto layer = ctx.animator_add_layer(e, overlay, crabby::layer_blend::override_pose, "arm");
        CHECK(layer == 0);
        ctx.animator_set_layer_weight(e, layer, 0.5f);
        ctx.animator_update(e, 0.5f); // layer: y = 1, base: y = 0
        CHECK(std::fabs(ctx.joint_pose(e, arm).position.y - 0.5f) < 1e-4f);
        throws([&] { ctx.animator_set_layer_weight(e, 1, 1); });
        throws([&] { ctx.animator_add_layer(e, overlay, crabby::layer_blend::additive, "tail"); });
        ctx.animator_set_root_motion(e, crabby::root_motion::extract, arm);
        ctx.animator_update(e, 0.1f);
        auto [moved, turned] = ctx.animator_root_motion(e);
        CHECK(moved.x == 0 && moved.z == 0 && turned.w == 1); // rising isn't horizontal
    });
    std::filesystem::remove(file);
}

// A ball on a 1 m ball joint below a fixed anchor swings but stays 1 m away.
static void joints3d() {
    crabby::entity ball = 0, joint = 0;
    std::size_t joints_seen = 0;
    float reach = 0, lowest = 5;
    crabby::game("joints", 1, 1)
        .physics3d({0, -9.81f, 0})
        .sim_system([&](crabby::ctx& ctx) {
            if (ctx.sim_tick() == 10) joints_seen = ctx.joint_count();
            if (ctx.sim_tick() < 60) lowest = std::min(lowest, ctx.world().get<crabby::transform3d>(ball).position.y);
            if (ctx.sim_tick() == 60) {
                auto p = ctx.world().get<crabby::transform3d>(ball).position;
                reach = std::sqrt(p.x * p.x + (p.y - 5) * (p.y - 5) + p.z * p.z);
                ctx.world().despawn(joint);
            }
        }, "probe")
        .run_headless(70, [&](crabby::ctx& ctx) {
            auto world = ctx.world();
            auto anchor = world.spawn(crabby::transform3d_at({0, 5, 0}), crabby::fixed_body3d(), crabby::sphere_collider(0.1f));
            ball = world.spawn(crabby::transform3d_at({1, 5, 0}), crabby::with_dominance(crabby::dynamic_body3d(), 3),
                               crabby::sphere_collider(0.2f));
            auto j = crabby::spherical_joint3d(anchor, ball);
            j.frame_b = crabby::transform3d_at({-1, 0, 0}); // the anchor, seen from the ball
            joint = world.spawn(j);
            CHECK(world.get<crabby::joint3d>(joint).axes[crabby::joint_axis::linear_x].kind == CRABBY_JOINT_LOCKED);
            CHECK(world.get<crabby::rigid_body3d>(ball).dominance == 3);
            auto bad = crabby::with_limits(crabby::revolute_joint3d(anchor, ball), crabby::joint_axis::angular_x, 1, -1);
            throws([&] { world.spawn(bad); });
            auto motor = crabby::with_drive(crabby::revolute_joint3d(anchor, ball), crabby::joint_axis::angular_x, crabby::velocity_drive(2, 10));
            CHECK(motor.drives[crabby::joint_axis::angular_x].enabled && motor.drives[3].target_velocity == 2);
        });
    CHECK(joints_seen == 1);
    CHECK(std::fabs(reach - 1) < 0.05f);
    CHECK(lowest < 4.1f); // it swung down to about 1 m below the anchor
}

// A leg of two 1 m bones bends so its foot lands on a reachable target.
static void inverse_kinematics() {
    auto g = crabby::game("ik", 1, 1);
    g.late_system([](crabby::ctx&) {}, "after animation");
    g.run_headless(0, [&](crabby::ctx& ctx) {
        auto leg = ctx.world().spawn(crabby::transform3d_at({5, 0, 0}));
        ctx.set_skeleton(leg, {crabby::joint{"hip"}, crabby::joint{"knee", 0, crabby::transform3d_at({0, -1, 0})},
                               crabby::joint{"foot", 1, crabby::transform3d_at({0, -1, 0})}});
        ctx.two_bone_ik(leg, 0, 1, 2, {5.5f, -1.5f, 0}, {0, 0, 1});
        auto foot = ctx.joint_matrix(leg, 2); // model space: the leg stands at x = 5
        CHECK(std::fabs(foot[12] - 0.5f) < 1e-3f && std::fabs(foot[13] + 1.5f) < 1e-3f);
        // The hip's -Y axis (down the leg) aims at a point in front.
        ctx.reset_pose(leg);
        ctx.look_at_joint(leg, 0, {0, -1, 0}, {5, 0, 3});
        auto knee = ctx.joint_matrix(leg, 1);
        CHECK(std::fabs(knee[14] - 1) < 1e-3f && std::fabs(knee[13]) < 1e-3f);
        ctx.draw_skeleton(leg, crabby::colors::white);
        throws([&] { ctx.two_bone_ik(leg, 0, 1, 9, {0, 0, 0}, {0, 0, 1}); });
    });
}

// A stick figure ragdolled 1 m up falls and is posed by its bodies; stopping removes them.
static void ragdolls() {
    crabby::entity figure = 0;
    float hips_y = 0, rest_y = 0;
    crabby::game("ragdolls", 1, 1)
        .physics3d({0, -9.81f, 0})
        .sim_system([&](crabby::ctx& ctx) {
            if (ctx.sim_tick() == 60) {
                hips_y = ctx.joint_pose(figure, 0).position.y;
                ctx.stop_ragdoll(figure, 0.5f);
                CHECK(ctx.ragdolled(figure) && ctx.ragdoll_blending(figure));
                CHECK(!ctx.ragdoll_body(figure, 0));
            }
        }, "probe")
        .run_headless(62, [&](crabby::ctx& ctx) {
            auto world = ctx.world();
            world.spawn(crabby::transform3d_at({0, -0.5f, 0}), crabby::fixed_body3d(), crabby::box_collider({20, 1, 20}));
            figure = world.spawn(crabby::transform3d_at({0, 1, 0}));
            ctx.set_skeleton(figure, {crabby::joint{"hips"}, crabby::joint{"spine", 0, crabby::transform3d_at({0, 0.6f, 0})},
                                      crabby::joint{"head", 1, crabby::transform3d_at({0, 0.4f, 0})}});
            rest_y = ctx.joint_pose(figure, 0).position.y;
            std::size_t before = world.size();
            ctx.start_ragdoll(figure, {crabby::ragdoll_bone("hips", 0.12f), crabby::ragdoll_bone("spine", 0.1f).swing(0.5f).twist(0.2f)});
            CHECK(world.size() == before + 3); // two bodies and a joint
            auto spine = ctx.find_joint(figure, "spine");
            auto body = ctx.ragdoll_body(figure, *spine);
            CHECK(body && ctx.apply_impulse3d(*body, {1, 0, 0}));
            CHECK(!ctx.ragdoll_body(figure, 2)); // the head has no body of its own
            CHECK(ctx.ragdolled(figure) && !ctx.ragdoll_blending(figure));
            throws([&] { ctx.start_ragdoll(figure, {crabby::ragdoll_bone("tail", 0.1f)}); });
        });
    CHECK(hips_y < rest_y - 0.3f); // it fell
}

// One-way platforms pass things up through, belts carry them, and a hook can ignore a pair.
static void contacts3d() {
    auto floor = [](crabby::world_view world, crabby::vec3 at, crabby::contact_modifier3d m) {
        return world.spawn(crabby::transform3d_at(at), crabby::fixed_body3d(), crabby::box_collider({20, 0.4f, 20}), m);
    };
    auto ball = [](crabby::world_view world, crabby::vec3 at, crabby::vec3 linear) {
        return world.spawn(crabby::transform3d_at(at), crabby::dynamic_body3d(), crabby::sphere_collider(0.25f),
                           crabby::velocity3d{linear, {0, 0, 0}});
    };
    crabby::entity from_below = 0, blocked = 0, carried = 0, ghost = 0, standing = 0;
    std::size_t hook_calls = 0;
    bool pair_ok = true;
    auto world = crabby::game("contacts", 1, 1).physics3d({0, -9.81f, 0}).run_headless(150, [&](crabby::ctx& ctx) {
        auto w = ctx.world();
        // A one-way platform: a ball thrown up from below comes up through it.
        auto up = crabby::one_way_modifier3d({0, 1, 0});
        CHECK(up.has_one_way && up.one_way_angle == 1.2f);
        floor(w, {0, 0, 0}, up);
        from_below = ball(w, {0, -1, 0}, {0, 8, 0});
        // An ordinary one blocks the same throw.
        floor(w, {30, 0, 0}, crabby::default_contact_modifier3d());
        blocked = ball(w, {30, -1, 0}, {0, 8, 0});
        // A belt carrying things along +X at 2 m/s.
        auto belt = crabby::conveyor_modifier3d({2, 0, 0});
        CHECK(belt.surface_velocity.x == 2);
        floor(w, {60, 0, 0}, belt);
        carried = ball(w, {60, 0.5f, 0}, {0, 0, 0});
        // A hook: only `ghost` is let through the hooked floor.
        auto hooked = floor(w, {90, 0, 0}, crabby::hook_modifier3d());
        ghost = ball(w, {90, 1, 0}, {0, 0, 0});
        standing = ball(w, {93, 1, 0}, {0, 0, 0});
        ctx.set_contact_hook3d([&, hooked](crabby::contact_pair3d& pair) {
            ++hook_calls;
            pair_ok = pair_ok && (pair.a() == hooked || pair.b() == hooked);
            auto count = pair.point_count();
            pair_ok = pair_ok && !pair.point(count + 5);
            pair.set_friction(0.5f);
            pair.set_restitution(0.25f);
            pair_ok = pair_ok && pair.friction() == 0.5f && pair.restitution() == 0.25f;
            if (count > 0) {
                pair_ok = pair_ok && std::abs(pair.normal().y) > 0.99f;
                auto p = pair.point(0);
                pair_ok = pair_ok && p.has_value();
                if (p && p->position.x < 91.5f) {
                    pair.ignore();
                    pair_ok = pair_ok && pair.point_count() == 0;
                }
            }
            pair.remove_point(count + 5);
            pair.set_surface_velocity({0, 0, 0});
            throws([&] { pair.set_friction(std::nanf("")); });
        });
    });
    float rest = 0.2f + 0.25f;
    CHECK(std::abs(world.get<crabby::transform3d>(from_below).position.y - rest) < 0.05f);
    CHECK(world.get<crabby::transform3d>(blocked).position.y < -0.3f);
    CHECK(world.get<crabby::transform3d>(carried).position.x > 61);
    CHECK(hook_calls > 0 && pair_ok);
    CHECK(world.get<crabby::transform3d>(ghost).position.y < -1);
    CHECK(world.get<crabby::transform3d>(standing).position.y > 0.3f);

    // A cleared hook leaves the hooked floor ordinary.
    std::size_t calls_after_clear = 0;
    crabby::entity ball_on_floor = 0;
    auto cleared = crabby::game("contacts cleared", 1, 1).physics3d({0, -9.81f, 0}).run_headless(100, [&](crabby::ctx& ctx) {
        auto w = ctx.world();
        floor(w, {0, 0, 0}, crabby::hook_modifier3d());
        ball_on_floor = ball(w, {0, 1, 0}, {0, 0, 0});
        ctx.set_contact_hook3d([&](crabby::contact_pair3d& pair) { ++calls_after_clear; pair.ignore(); });
        ctx.clear_contact_hook3d();
    });
    CHECK(calls_after_clear == 0 && cleared.get<crabby::transform3d>(ball_on_floor).position.y > 0.3f);

    // An exception in the hook never unwinds into the engine: the run ends and rethrows it.
    bool caught = false;
    try {
        crabby::game("contacts throwing", 1, 1).physics3d({0, -9.81f, 0}).run_headless(100, [&](crabby::ctx& ctx) {
            auto w = ctx.world();
            floor(w, {0, 0, 0}, crabby::hook_modifier3d());
            ball(w, {0, 1, 0}, {0, 0, 0});
            ctx.set_contact_hook3d([](crabby::contact_pair3d&) { throw std::runtime_error("boom"); });
        });
    } catch (const std::runtime_error& e) { caught = std::string(e.what()) == "boom"; }
    CHECK(caught);

    // The modifier is an ordinary component: read it back, and it is no query column.
    crabby::world plain;
    auto m = crabby::one_way_modifier3d({0, 1, 0});
    m.hook = true;
    auto e = plain.spawn(m);
    CHECK(plain.get<crabby::contact_modifier3d>(e).hook);
    m.one_way_angle = -1;
    throws([&] { plain.set(e, m); });
}

// Two endpoints in memory: connect, then a reliable message and a broadcast.
static void messages() {
    throws([] { crabby::net_messages::steam_host(480, 0); });
    throws([] { crabby::net_messages::steam_join(480, 0); });
    throws([] { crabby::net_messages::steam_host(480, 2); }); // no Steam support/client in native test
    throws([] { crabby::steam_lobby::create(480, crabby::steam_lobby_visibility::private_lobby, 0); });
    throws([] { crabby::steam_lobby::create(480, crabby::steam_lobby_visibility::friends_only, 2); });
    throws([] { crabby::steam_lobby::join(480, 0); });
    auto peers = crabby::net_messages::loopback(2);
    auto& host = peers[0];
    auto& client = peers[1];
    std::vector<crabby::message_event> got;
    for (int i = 0; i < 3000 && got.size() < 4; i++) { // the handshake takes real time
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
        host.poll();
        client.poll();
        while (auto e = host.next_event()) got.push_back(*e);
        while (auto e = client.next_event()) {
            if (e->kind == CRABBY_MESSAGE_CONNECTED) {
                client.send(0, CRABBY_LANE_RELIABLE, std::string("buy apple"));
                host.send_to_all(CRABBY_LANE_UNRELIABLE, std::string("tick"));
            }
            got.push_back(*e);
        }
    }
    auto has = [&](CrabbyMessageEventKind kind, const std::string& text) {
        for (auto& e : got)
            if (e.kind == kind && std::string(e.bytes.begin(), e.bytes.end()) == text) return true;
        return false;
    };
    CHECK(has(CRABBY_MESSAGE_CONNECTED, ""));
    CHECK(has(CRABBY_MESSAGE, "buy apple"));
    CHECK(has(CRABBY_MESSAGE, "tick"));
    CHECK(host.stats().packets_sent > 0);
    throws([&] { client.send(0, CRABBY_LANE_RELIABLE, nullptr, 2 * 1024 * 1024); });
    throws([] { crabby::net_messages::loopback(1); });
}

static void navigation_carving() {
    crabby::entity box = 0;
    int tick = 0;
    bool cut = false, restored = false;
    crabby::game("carve", 1, 1).navmesh3d()
        .sim_system([&](crabby::ctx& ctx) {
            ++tick;
            auto state = ctx.navmesh_update_carving(1);
            CHECK(state.first <= 1);
            if (tick == 20) {
                CHECK(!state.second);
                auto path = ctx.navmesh_find_path({-7, 0, 0}, {7, 0, 0});
                CHECK(path.size() >= 3);
                CHECK(!ctx.navmesh_sample_position({0, 0, 0}, 0.2f));
                cut = true;
                ctx.remove_nav(box);
            }
            if (tick == 40) {
                CHECK(!state.second);
                auto path = ctx.navmesh_find_path({-7, 0, 0}, {7, 0, 0});
                CHECK(path.size() == 2);
                restored = true;
            }
        }).run_headless(45, [&](crabby::ctx& ctx) {
            crabby::nav_sources sources;
            sources.add_mesh(crabby::mesh_data::plane(20), crabby::transform3d_at({0, 0, 0}));
            auto settings = crabby::nav_bake_defaults();
            settings.carving = true;
            ctx.bake_navmesh(sources, settings);
            box = ctx.world().spawn();
            ctx.set_nav_carve_obstacle(box, {0, 1, 0}, {0.5f, 1, 2});
        });
    CHECK(cut && restored);
}

// A 20 m floor with a wall across the middle, open at one end: the agent walks around.
static void navigation() {
    crabby::entity walker = 0;
    std::size_t corners = 0;
    auto w = crabby::game("nav", 1, 1).navmesh3d().run_headless(600, [&](crabby::ctx& ctx) {
        crabby::nav_sources sources;
        sources.add_mesh(crabby::mesh_data::plane(20), crabby::transform3d_at({0, 0, 0}));
        auto wall = crabby::mesh_data::cube(1);
        auto t = crabby::transform3d_at({0, 1, -2});
        t.scale = {1, 2, 16}; // x = -0.5..0.5, z = -10..6: open beyond z = 6
        sources.add_mesh(wall, t);
        ctx.bake_navmesh(sources);
        auto snapped = ctx.navmesh_sample_position({-5, 3, 0}, 5);
        CHECK(snapped && std::fabs(snapped->y) < 0.3f);
        auto path = ctx.navmesh_find_path({-5, 0, 0}, {5, 0, 0});
        auto status = ctx.navmesh_update_carving(0);
        CHECK(status.first == 0 && !status.second);
        ctx.set_nav_carve_obstacle(ctx.world().spawn(), {8, 1, 8}, {0.5f, 1, 0.5f});
        corners = path.size();
        CHECK(path.size() >= 3); // around the wall's end
        walker = ctx.world().spawn(crabby::transform3d_at({-5, 0, 0}));
        ctx.set_nav_agent(walker, 4);
        ctx.set_nav_avoidance(walker);
        throws([&] { ctx.set_nav_avoidance(walker, true, -1); });
        throws([&] { ctx.set_nav_avoidance(walker, true, 2, 0); });
        throws([&] { ctx.set_nav_avoidance(ctx.world().spawn()); });
        ctx.set_nav_destination(walker, crabby::vec3{5, 0, 0});
        throws([&] { ctx.set_nav_agent(walker, -1); });
        crabby::nav_sources empty;
        throws([&] { ctx.bake_navmesh(empty); });
    });
    auto end = w.get<crabby::transform3d>(walker).position;
    CHECK(corners >= 3);
    CHECK(std::fabs(end.x - 5) < 0.3f && std::fabs(end.z) < 0.3f);
}

static void particles3d() {
    crabby::game("particles", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        auto e = ctx.world().spawn(crabby::transform3d_at({0, 1, 0}));
        auto fire = crabby::particle_settings()
                        .rate(40)
                        .cone(0.3f)
                        .lifetime(0.5f, 1)
                        .size_over_life({{0, 1}, {1, 0}})
                        .color_over_life({{0, {1, 0.6f, 0.1f, 1}}, {1, {1, 0, 0, 0}}})
                        .blend(crabby::particle_blend::additive)
                        .max_particles(256);
        ctx.set_particle_emitter3d(e, fire);
        ctx.particle_burst(e, 20);
        // Particles simulate in the visual frame phase, which headless runs skip.
        CHECK(ctx.particle_live_count(e) == 0);
        throws([&] { ctx.set_particle_emitter3d(e, crabby::particle_settings().lifetime(0, 1)); });
        throws([&] { ctx.set_particle_emitter3d(e, crabby::particle_settings().lifetime(2, 1)); });
        throws([&] { ctx.set_particle_emitter3d(e, crabby::particle_settings().max_particles(0)); });
        ctx.remove_particle_emitter3d(e);
        throws([&] { ctx.particle_burst(e, 1); });
    });
}

// A 4 x 4 m floor facing up, as a glTF file.
static void ribbons_and_decals() {
    const auto file = std::filesystem::temp_directory_path() / "crabby-cpp-floor.gltf";
    std::ofstream(file) << R"({"asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
        "buffers": [{"byteLength": 60, "uri": "data:application/octet-stream;base64,AAAAwAAAAAAAAABAAAAAQAAAAAAAAABAAAAAQAAAAAAAAADAAAAAwAAAAAAAAADAAAABAAIAAAACAAMA"}],
        "bufferViews": [{"buffer": 0, "byteLength": 48}, {"buffer": 0, "byteOffset": 48, "byteLength": 12}],
        "accessors": [{"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3", "min": [-2, 0, -2], "max": [2, 0, 2]},
            {"bufferView": 1, "componentType": 5123, "count": 6, "type": "SCALAR"}]})";
    const std::string path = file.string();
    // The box's +Z turned up, 0.5 m wide and 0.2 m deep, at (1, 0, 0).
    const std::array<float, 16> down = {0.5f, 0, 0, 0,   0, 0, -0.2f, 0,   0, 0.5f, 0, 0,   1, 0, 0, 1};
    crabby::game("ribbons", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        auto e = ctx.world().spawn(crabby::transform3d_at({0, 1, 0}));
        auto tracer = crabby::trail3d(0.25f, 0.05f)
                          .width_over_age({{0, 0.1f}, {1, 0}})
                          .color_over_age({{0, {1, 0.8f, 0.3f, 1}}, {1, {1, 0.3f, 0, 0}}})
                          .blend(crabby::particle_blend::additive)
                          .min_distance(0.1f)
                          .max_points(32);
        ctx.set_trail3d(e, tracer);
        // Trails record in the visual frame phase, which headless runs skip.
        CHECK(ctx.trail3d_point_count(e) == 0);
        ctx.set_trail3d_emitting(e, false);
        ctx.clear_trail3d(e);
        throws([&] { ctx.set_trail3d(e, crabby::trail3d(0, 0.1f)); });
        throws([&] { ctx.set_trail3d(e, crabby::trail3d(1, -0.1f)); });
        throws([&] { ctx.set_trail3d(e, crabby::trail3d(1, 0.1f).max_points(1)); });
        throws([&] { ctx.set_trail3d(e, crabby::trail3d(1, 0.1f).width_over_age({{0, -1}})); });
        ctx.remove_trail3d(e);
        throws([&] { ctx.trail3d_point_count(e); });
        throws([&] { ctx.clear_trail3d(e); });

        auto beam = crabby::line3d({{0, 0, 0}, {0, 0, -20}}, 0.02f, crabby::colors::red)
                        .blend(crabby::particle_blend::additive)
                        .texture_length(2);
        ctx.set_line3d(e, beam);
        ctx.set_line3d(e, crabby::line3d({{0, 0, 0}, {1, 0, 0}, {1, 1, 0}}, 0.1f, crabby::colors::white)); // replaces it
        throws([&] { ctx.set_line3d(e, crabby::line3d({{0, 0, 0}, {1, 0, 0}}, -1, crabby::colors::white)); });
        throws([&] { ctx.set_line3d(e, crabby::line3d({{0, 0, 0}, {1, NAN, 0}}, 0.1f, crabby::colors::white)); });
        ctx.remove_line3d(e);

        auto floor = ctx.load_mesh_data(path.c_str());
        CHECK(floor.view().vertex_count == 4 && floor.view().index_count == 6);
        auto mark = floor.decal(down, 0.002f);
        auto view = mark.view();
        CHECK(view.index_count >= 3 && view.index_count % 3 == 0);
        for (std::size_t i = 0; i < view.vertex_count; i++) {
            CHECK(std::fabs(view.positions[i].x - 1) <= 0.26f && std::fabs(view.positions[i].z) <= 0.26f);
            CHECK(std::fabs(view.positions[i].y - 0.002f) < 1e-4f);
        }
        // A box that misses gives an empty decal; a broken projector throws.
        auto away = down;
        away[12] = 50;
        CHECK(floor.decal(away, 0.002f).view().index_count == 0);
        auto broken = down;
        broken[0] = NAN;
        throws([&] { floor.decal(broken, 0.002f); });
        throws([&] { ctx.load_mesh_data("no/such.glb"); });
    });
    std::filesystem::remove(file);
}

static void ui_panels() {
    crabby::game("ui panels", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        namespace u = crabby::ui;
        auto e = ctx.world().spawn(crabby::transform3d_at({0, 1.5f, -2}));
        auto texture = ctx.create_ui_panel(e, 600, 400, {1.2f, 0.8f});
        CHECK(texture.width == 600 && texture.height == 400);
        CHECK(ctx.world().has<crabby::model>(e));

        // The panel's UI takes the same calls as the screen's, in the panel's pixels.
        auto panel = ctx.panel_ui(e);
        auto root = panel.add_panel(u::none, u::layout().column().size(u::percent(1), u::percent(1)).padding(20));
        auto buy = panel.add_button(root, "Buy", u::layout().size(200, 60).shrink(0));
        panel.push_screen(root);
        panel.layout_for({600, 400});
        CHECK(panel.top_screen() == root && panel.rect(buy).max.x == 220 && panel.rect(root).max.y == 400);
        CHECK(!ctx.ui_retained().top_screen());  // the screen's own UI is a different tree

        auto input = u::default_input();
        input.pointer = {50, 50};
        input.pressed = input.held = true;
        panel.frame({600, 400}, input);
        input.pressed = input.held = false;
        input.released = true;
        panel.frame({600, 400}, input);
        auto events = panel.events();
        CHECK(events.size() == 1 && events[0].kind == u::event_kind::clicked && events[0].node == buy);
        CHECK(ctx.ui_retained().events().empty());

        // Headless runs don't give panels the mouse; the calls still work.
        ctx.set_ui_pointer_ray({0, 1.5f, 0}, {0, 0, -1});
        CHECK(!ctx.ui_panel_pointer(e));
        ctx.set_ui_panel_interactive(e, false);
        ctx.clear_ui_pointer_ray();
        throws([&] { ctx.set_ui_pointer_ray({0, 0, 0}, {0, 0, 0}); });
        throws([&] { ctx.create_ui_panel(e, 0, 10, {1, 1}); });
        throws([&] { ctx.create_ui_panel(e, 10, 10, {-1, 1}); });
        ctx.remove_ui_panel(e);
        throws([&] { panel.add_panel(u::none); });  // no panel on it now
        throws([&] { ctx.ui_panel_pointer(e); });
    });
}

static void retained_ui() {
    crabby::game("retained ui", 1, 1).run_headless(0, [&](crabby::ctx& ctx) {
        auto ui = ctx.ui_retained();
        const crabby::vec2 window{800, 600};
        namespace u = crabby::ui;
        auto root = ui.add_panel(u::none, u::layout().column().size(400, 300).padding(10).gap(5));
        auto label = ui.add_label(root, "Title", u::layout().size(100, 20).shrink(0));
        auto go = ui.add_button(root, "Go", u::layout().size(200, 40).shrink(0));
        auto gift = ui.add_toggle(root, "Gift", false, u::layout().size(100, 20).shrink(0));
        auto volume = ui.add_slider(root, 0.25f, u::layout().size(200, 20).shrink(0));
        auto note = ui.add_text_input(root, "hi", u::layout().size(200, 30).shrink(0));
        CHECK(!ui.top_screen() && !ui.wants_keyboard());
        ui.push_screen(root);
        CHECK(ui.top_screen() == root && ui.wants_keyboard());
        ui.layout_for(window);
        CHECK(ui.rect(go).min.y == 35 && ui.rect(go).max.x == 210);
        CHECK(ui.text(label) == "Title" && ui.value(volume) == 0.25f);
        CHECK(ui.hit_test({50, 50}) == go && !ui.hit_test({700, 500}));

        // A click is a press, then a release over the same button.
        auto input = u::default_input();
        input.pointer = {50, 50};
        input.pressed = input.held = true;
        ui.frame(window, input);
        input.pressed = input.held = false;
        input.released = true;
        ui.frame(window, input);
        auto events = ui.events();
        CHECK(events.size() == 1 && events[0].kind == u::event_kind::clicked && events[0].node == go);
        CHECK(!ui.next_event() && ui.focus() == go);

        // Text, values and enabled state.
        ui.set_text(go, "Buy");
        CHECK(ui.text(go) == "Buy");
        ui.set_value(gift, 1);
        ui.set_value(volume, 9);
        CHECK(ui.value(gift) == 1 && ui.value(volume) == 1);
        ui.set_enabled(go, false);
        input.pointer = {50, 50};
        input.pressed = input.held = true; input.released = false;
        ui.frame(window, input);
        input.pressed = input.held = false; input.released = true;
        ui.frame(window, input);
        CHECK(ui.events().empty());

        // Typing into the input after focusing it.
        input = u::default_input();
        input.pointer = {205, 145};
        input.pressed = input.held = true;
        ui.frame(window, input);
        input = u::default_input();
        input.pointer = {205, 145};
        input.released = true;
        ui.frame(window, input);
        input = u::default_input();
        input.text = "!?";
        ui.frame(window, input);
        CHECK(ui.text(note) == "hi!?");
        events = ui.events();
        CHECK(events.size() == 1 && events[0].kind == u::event_kind::changed && events[0].node == note);

        // Modal screens stack, and Cancel pops the top one.
        auto dialog = ui.add_panel(u::none, u::layout().size(100, 100));
        ui.push_screen(dialog, true);
        CHECK(ui.top_screen() == dialog && ui.wants_pointer());
        input = u::default_input();
        input.cancel = true;
        ui.frame(window, input);
        events = ui.events();
        CHECK(events.size() == 1 && events[0].kind == u::event_kind::screen_popped && events[0].node == dialog);
        CHECK(ui.top_screen() == root);
        ui.push_screen(dialog);
        CHECK(ui.pop_screen() == dialog);
        throws([&] { ui.push_screen(go); });    // it has a parent
        throws([&] { ui.push_screen(root); });  // already a screen

        // Grid tracks, spans and absolute positions.
        auto grid = ui.add_panel(u::none, u::layout().grid({u::fr(1), u::fr(1), u::fr(1)}).auto_rows({u::track_points(40)}).size(300, 200));
        auto cell = ui.add_panel(grid);
        auto wide = ui.add_panel(grid, u::layout().grid_column(u::grid_auto(), u::span(2)));
        auto corner = ui.add_panel(u::none, u::layout().position(u::position::relative).size(200, 100));
        auto pinned = ui.add_panel(corner, u::layout().absolute().size(u::percent(0.5f), u::points(30))
                                               .inset(u::points(20), u::automatic(), u::automatic(), u::points(10)));
        for (auto screen : {grid, corner}) ui.push_screen(screen);
        ui.layout_for(window);
        CHECK(ui.rect(cell).max.x == 100);
        CHECK(ui.rect(wide).min.x == 100 && ui.rect(wide).max.x == 300);  // the next two columns
        CHECK(ui.rect(pinned).min.x == 10 && ui.rect(pinned).min.y == 20 && ui.rect(pinned).max.x == 110);

        // Named areas, owned builders, repeats and named line setters.
        auto named_layout = u::layout().grid({u::repeat(2, u::fr(1))})
            .grid_rows({u::track_points(40), u::track_points(160)})
            .areas({"header header", "side main"}).size(300, 200);
        auto copied = named_layout;
        auto named = ui.add_panel(u::none, copied);
        auto main = ui.add_panel(named, u::layout().area(std::string("main")));
        ui.push_screen(named);
        ui.layout_for(window);
        CHECK(ui.rect(main).min.x == 150 && ui.rect(main).min.y == 40);
        CHECK(ui.rect(main).max.x == 300 && ui.rect(main).max.y == 200);
        throws([&] { ui.set_grid_template_areas(named, {"x x", "x y"}); });
        ui.set_grid_line_names(named, 0, {"left", "right"});
        ui.set_grid_named_line(main, 0, false, "left", 1);
        ui.set_grid_named_line(main, 0, true, "right", 1);
        ui.layout_for(window);
        CHECK(ui.rect(main).min.x == 0 && ui.rect(main).max.x == 300);
        auto repeated_names = ui.add_panel(u::none, u::layout()
            .grid({u::repeat(2, u::fr(1)).names({"begin", "finish"})}).size(300, 100));
        auto repeated_item = ui.add_panel(repeated_names);
        ui.set_grid_named_line(repeated_item, 0, false, "begin", 2);
        ui.set_grid_named_line(repeated_item, 0, true, "finish", -1);
        ui.push_screen(repeated_names);
        ui.layout_for(window);
        CHECK(ui.rect(repeated_item).min.x == 150 && ui.rect(repeated_item).max.x == 300);
        throws([&] { ui.add_panel(u::none, u::layout().grid({u::repeat(2, u::fr(1)).names({"bad"})})); });
        ui.set_grid_area(main, "main");
        ui.layout_for(window);
        CHECK(ui.rect(main).min.x == 150);
        for (auto component : {u::auto_fill(u::track_points(100)), u::auto_fit({u::track_points(100)})}) {
            auto repeated = ui.add_panel(u::none, u::layout().grid({component}).size(300, 100));
            auto item = ui.add_panel(repeated);
            ui.push_screen(repeated);
            ui.layout_for(window);
            CHECK(ui.rect(item).max.x == 100);
        }
        throws([&] { ui.add_panel(u::none, u::layout().grid({u::repeat(0, u::fr(1))})); });
        throws([&] { ui.add_panel(u::none, u::layout().grid({u::auto_fill(u::fr(1))})); });

        // Paint, scrolling and scale.
        ui.set_paint(go, u::paint().background({1, 0, 0, 1}).border({1, 1, 1, 1}, 2).font_size(24).wrap().align(u::text_align::center));
        auto scroll = ui.add_scroll(u::none, u::layout().size(100, 100));
        ui.add_panel(scroll, u::layout().size(100, 400).shrink(0));
        ui.push_screen(scroll);
        ui.layout_for(window);
        ui.set_scroll(scroll, {0, 9999});
        ui.layout_for(window);
        CHECK(ui.scroll(scroll).y == 300);
        ui.set_scale(2);
        ui.layout_for(window);
        CHECK(ui.rect(scroll).max.x == 200);
        throws([&] { ui.set_paint(go, u::paint().font({99}, 16)); });

        // Removing a node takes its children, and its ids stay invalid.
        ui.remove(root);
        for (auto gone : {root, go, note}) {
            throws([&] { (void)ui.rect(gone); });
            throws([&] { ui.set_text(gone, "x"); });
        }
        throws([&] { ui.add_panel(root); });
        throws([&] { ui.add_panel(u::none, u::layout().display(static_cast<u::display>(99))); });
    });
}

static void world_text() {
    crabby::game("text", 1, 1).run_headless(0, [](crabby::ctx& c) {
        auto e = c.world().spawn();
        CHECK(!c.text3d(e));
        crabby::text3d t("Hello\nworld");
        t.size = .5f; t.align = crabby::text_align3d::left;
        t.billboard = true; t.outline_width = .06f;
        c.set_text3d(e, t);
        auto got = c.text3d(e);
        CHECK(got && got->text == t.text && got->size == .5f && got->billboard);
        CHECK(got->align == t.align && got->outline_width == .06f);
        t.text = "Changed"; c.set_text3d(e, t);
        CHECK(c.text3d(e)->text == "Changed");
        t.font.id = UINT32_MAX;
        throws([&] { c.set_text3d(e, t); });
        CHECK(c.text3d(e)->text == "Changed");
        t.font.id = 0; t.align = static_cast<crabby::text_align3d>(99);
        throws([&] { c.set_text3d(e, t); });
        c.remove_text3d(e); CHECK(!c.text3d(e));
        c.world().despawn(e);
        throws([&] { (void)c.text3d(e); });
    });
}
// Replication helpers: sequence numbers, authority, snapshots and interest.
static void replication() {
    CHECK(crabby::seq_newer(0, 65535) && !crabby::seq_newer(5, 5) && crabby::seq_diff(1, 65535) == 2);
    CHECK(crabby::stamp_newer({1, 0}, {0, 60000}) && !crabby::stamp_newer({0, 7}, {0, 7}));

    crabby::authority car(0);
    CHECK(car.check(0, 0) == crabby::authority_check::accept);
    CHECK(car.handoff(1) == 1 && car.owner() == 1 && car.epoch() == 1);
    CHECK(car.check(0, 0) == crabby::authority_check::stale);
    CHECK(car.check(0, 1) == crabby::authority_check::wrong_owner);
    CHECK(car.check(1, 2) == crabby::authority_check::ahead);
    CHECK(!car.adopt(2, 1) && car.owner() == 1); // simultaneous handoff: the lower id wins
    CHECK(car.adopt(2, 2) && car.owner() == 2);

    crabby::latest_wins filter;
    CHECK(filter.accept(5, std::uint16_t(3)) && !filter.accept(5, std::uint16_t(2)) && !filter.accept(5, std::uint16_t(3)));
    CHECK(filter.accept(5, crabby::update_stamp{1, 0})); // a new epoch beats any sequence number
    CHECK(filter.size() == 1);
    filter.forget(5);
    CHECK(filter.accept(5, std::uint16_t(1)) && filter.size() == 1);
    auto moved = std::move(filter);
    moved.clear();
    CHECK(moved.size() == 0);

    crabby::snapshot_stream stream;
    CHECK(!stream.sample(0) && !stream.clock_offset());
    for (int n = 0; n < 20; n++) {
        auto p = crabby::moving(crabby::pose_at({float(n), 0, 0}), {20, 0, 0});
        CHECK(stream.push(5000 + n * 0.05, n * 0.05 + 0.03, p) == crabby::snapshot_push::added);
    }
    CHECK(stream.push(5000, 1.0, crabby::pose_at({0, 0, 0})) == crabby::snapshot_push::duplicate);
    auto shown = stream.sample(1.0);
    CHECK(shown && shown->state == crabby::snapshot_state::interpolated && std::fabs(shown->value.position.x - 17.4f) < 0.05f);
    CHECK(stream.sample(1.2)->state == crabby::snapshot_state::extrapolated);
    CHECK(stream.sample(5.0)->state == crabby::snapshot_state::held);
    CHECK(stream.size() == 20 && stream.clock_offset() && std::fabs(*stream.clock_offset() + 4999.97) < 0.01);
    stream.set_delay(0);
    throws([&] { stream.set_delay(-1); });
    stream.clear();
    CHECK(stream.size() == 0 && !stream.sample(1.0));
    auto config = crabby::snapshot_defaults();
    config.capacity = 1;
    throws([&] { crabby::snapshot_stream bad(config); });

    crabby::interest interest;
    std::vector<crabby::interest_viewer> viewers{{1, {0, 0, 0}}};
    std::vector<crabby::interest_entity> world{{10, {10, 0, 0}}, {20, {100, 0, 0}}, {30, {900, 0, 0}}};
    interest.update(0, viewers, world);
    auto entries = interest.entries(1);
    CHECK(entries.size() == 2 && entries[0].entity == 10 && entries[0].tier == 0 && entries[1].tier == 1 && entries[1].send);
    CHECK(interest.changes().size() == 2 && interest.changes()[0].kind == CRABBY_INTEREST_ENTERED);
    int near_sent = 0, mid_sent = 0;
    for (std::uint64_t tick = 1; tick <= 8; tick++) {
        interest.update(tick, viewers, world);
        CHECK(interest.changes().empty());
        for (auto& e : interest.entries(1)) (e.tier == 0 ? near_sent : mid_sent) += e.send;
    }
    CHECK(near_sent == 8 && mid_sent == 2); // every tick, and every 4th
    interest.update(9, {}, world); // nobody is looking any more
    CHECK(interest.entries(1).empty() && interest.changes().size() == 2);
    interest.remove_peer(1);
    auto bad = crabby::interest_defaults();
    bad.hysteresis = 99;
    throws([&] { crabby::interest invalid(bad); });
}

int main() {
    worlds();
    names_and_reflection();
    resources();
    data_files();
    scenes_and_prefabs();
    world_text();
    three_d();
    skeletons();
    lod_groups();
    morph_targets();
    sound3d();
    animators();
    inverse_kinematics();
    streaming();
    parallel_queries();
    games();
    attachments();
    physics();
    joints3d();
    vehicles();
    lockstep();
    messages();
    replication();
    navigation();
    navigation_carving();
    particles3d();
    ragdolls();
    contacts3d();
    ribbons_and_decals();
    retained_ui();
    ui_panels();
    events();
    rng_matches_rust();
    grid_views_match_engine_queries();
    std::cout << "C++ wrapper tests passed\n";
}
