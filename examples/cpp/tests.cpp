#include "ponko.hpp"
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
PONKO_COUNT_CPP_ALLOCATIONS
// The wrapper throws the engine's message; an argument the engine refused says so
// (a panic would say "panicked").
template<class F> void throws_invalid(F f, const char* what) {
    bool caught = false;
    try { f(); } catch (const std::exception& e) {
        std::string message = e.what();
        caught = message.find(what) != std::string::npos && message.find("panic") == std::string::npos;
    }
    CHECK(caught);
}
template<class F> void throws(F f) { bool caught = false; try { f(); } catch (const std::exception&) { caught = true; } CHECK(caught); }

static void worlds() {
    static_assert(!std::is_copy_constructible<ponko::world>::value, "world owns its handle");
    ponko::world a;
    auto e = a.spawn(Pos{1, 2}, Vel{3, 4});
    CHECK(a.size() == 1 && a.alive(e));
    a.each([&](ponko::entity id, Pos& p, const Vel& v) { p.x += v.x; CHECK(id == e); });
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
    ponko::world c(std::move(a)); CHECK(c.get<Pos>(e).x == 12);
    throws([&] { (void)a.size(); });
    ponko::world b;
    b = std::move(c); CHECK(b.get<Pos>(e).x == 12);
    b.remove<Vel>(e); throws([&] { (void)b.get<Vel>(e); });
    b.despawn(e); CHECK(!b.alive(e)); throws([&] { b.despawn(e); });
    CHECK(!b.try_get<Pos>(e));
}

static void names_and_reflection() {
    // Components are named after their C++ type, namespaces included.
    CHECK(ponko::detail::type_name<game::Health>() == "game::Health");
    CHECK(ponko::detail::type_name<Pos>() == "Pos");
    ponko::world w;
    w.reflect<game::Health>({ponko::field("value", &game::Health::value)});
    auto e = w.spawn(game::Health{5}, other::Health{7});
    CHECK(w.get<game::Health>(e).value == 5 && w.get<other::Health>(e).value == 7);
    const std::string saved = w.save();
    CHECK(saved.find("\"game::Health\":{\"value\":5.0}") != std::string::npos);
    w.set(e, game::Health{1});
    w.load(saved);
    CHECK(w.get<game::Health>(e).value == 5);
    // A chosen name must come before first use, and can't be taken twice.
    ponko::world n;
    n.reflect<Vel>("Velocity", {ponko::field("x", &Vel::x), ponko::field("y", &Vel::y)});
    n.spawn(Vel{1, 2});
    CHECK(n.save().find("\"Velocity\":{\"x\":1.0,\"y\":2.0}") != std::string::npos);
    throws([&] { n.reflect<Pos>("Velocity", {ponko::field("x", &Pos::x)}); });
    n.spawn(Pos{});
    throws([&] { n.reflect<Pos>("Position", {ponko::field("x", &Pos::x)}); });
}

namespace game { struct Score { std::uint32_t red, blue; }; }
static void resources() {
    ponko::world w;
    CHECK(!w.try_resource<game::Score>());
    throws([&] { w.resource<game::Score>(); });
    w.set_resource(game::Score{1, 2});
    CHECK(w.resource<game::Score>().red == 1 && w.resource<game::Score>().blue == 2);
    w.set_resource(game::Score{3, 2});
    CHECK(w.resource<game::Score>().red == 3);
    // Described, it's saved with the entities and loaded back.
    w.reflect_resource<game::Score>({ponko::field("red", &game::Score::red), ponko::field("blue", &game::Score::blue)});
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
    const auto file = std::filesystem::temp_directory_path() / "ponko-cpp-tuning.json";
    std::ofstream(file) << R"({"speed": 2.5, "waves": 3})";
    const std::string path = file.string();
    auto w = ponko::game("data", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        auto world = ctx.world();
        world.reflect_resource<game::Tuning>({ponko::field("speed", &game::Tuning::speed), ponko::field("waves", &game::Tuning::waves)});
        ctx.load_json<game::Tuning>(path.c_str());
        world.reflect<game::Tuning>({ponko::field("speed", &game::Tuning::speed), ponko::field("waves", &game::Tuning::waves)});
        const ponko::entity e = world.spawn(game::Tuning{0, 9});
        ctx.load_json<game::Tuning>(path.c_str(), e);
        CHECK(world.get<game::Tuning>(e).speed == 2.5f);
    });
    CHECK(w.resource<game::Tuning>().speed == 2.5f && w.resource<game::Tuning>().waves == 3);
    std::filesystem::remove(file);
}

namespace game { struct Armor { float rating; }; }
// A schema described from C++ is what the capabilities answer reports.
static void described_schemas_reach_capabilities() {
    ponko::game("described", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        auto world = ctx.world();
        world.reflect<game::Armor>("DescribedInCpp", {ponko::field("rating", &game::Armor::rating)});
        const auto before = ctx.capabilities_json();
        CHECK(before.find("\"DescribedInCpp\"") != std::string::npos);
        CHECK(before.find("\"MadeInCpp\"") == std::string::npos);
        world.describe_component("DescribedInCpp", R"({"Object":[["rating","Number"],["kind",{"Enum":["MadeInCpp"]}]]})");
        CHECK(ctx.capabilities_json().find("\"MadeInCpp\"") != std::string::npos);
        throws([&] { world.describe_component("NotReflected", R"("Number")"); });
    });
}

// Saved under the executable's assets/ copy (build output), so nothing to clean.
static void scenes_and_prefabs() {
    ponko::entity first = 0;
    std::vector<ponko::entity> palms;
    auto w = ponko::game("scenes", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        auto world = ctx.world();
        world.reflect<game::Health>({ponko::field("value", &game::Health::value)});
        // Sprites are saved by their image's name, and spawning loads it.
        auto model = world.spawn(ponko::transform_at({70, 80}), game::Health{9}, ponko::sprite_of(ctx.load_texture("crab.png"), ponko::colors::white, 4));
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
    CHECK(w.get<ponko::transform2d>(palms[1]).position.x == 10 && w.get<game::Health>(palms[1]).value == 9);
    CHECK(w.get<ponko::sprite>(palms[1]).z == 4);
    auto loaded = ponko::game("scenes", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        ctx.world().reflect<game::Health>({ponko::field("value", &game::Health::value)});
        ctx.world().spawn(Tag{});   // replaced by the scene
        ctx.load_scene("test-cpp-scene.json");
    });
    CHECK(loaded.size() == 2 && loaded.get<game::Health>(first).value == 9);
    // Offline, a lockstep load is a plain load; a failed one changes nothing.
    auto offline = ponko::game("scenes", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        ctx.world().reflect<game::Health>({ponko::field("value", &game::Health::value)});
        ctx.world().spawn(Tag{});
        ctx.load_scene_lockstep("test-cpp-scene.json");
        throws([&] { ctx.load_scene_lockstep("test-cpp-missing.json"); });
    });
    CHECK(offline.size() == 2 && offline.get<game::Health>(first).value == 9);
}

static void three_d() {
    auto probes = ponko::game("probes", 1, 1).run_headless(0, [](ponko::ctx& ctx) {
        auto e = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}), ponko::reflection_probe3d_default());
        ctx.bake_reflection_probe(e);
        auto p = ctx.world().get<ponko::reflection_probe3d>(e);
        CHECK(p.content == 1 && p.box_projection);
        ctx.bake_reflection_probe(e);
        CHECK(ctx.world().get<ponko::reflection_probe3d>(e).content == 1);
    });
    CHECK(probes.size() == 1);
    auto decals = ponko::game("decals", 1, 1).run_headless(0, [](ponko::ctx& ctx) {
        std::vector<std::uint8_t> pixels(2 * 2 * 4, 255);
        auto colors = ctx.create_texture_array(2, 2, pixels, ponko::texture_kind::color);
        throws([&] { ctx.set_decal_textures(colors); }); // decals want TextureKind::Data arrays
        auto images = ctx.create_texture_array(2, 2, pixels, ponko::texture_kind::data);
        ctx.set_decal_textures(images);
        CHECK(!ctx.supports_decals()); // no GPU in a headless run
        auto mark = ponko::decal3d_default();
        mark.size = {2, 2, 1};
        mark.order = 3;
        mark.normal_image = ponko::decal_no_image;
        auto e = ctx.world().spawn(ponko::transform3d_at({0, 1, 0}), mark, ponko::receives_decals{0});
        CHECK(ctx.world().get<ponko::decal3d>(e).order == 3 && ctx.world().get<ponko::decal3d>(e).size.x == 2);
        CHECK(ctx.world().get<ponko::receives_decals>(e).layers == 0);
        ctx.clear_decal_textures();
    });
    CHECK(decals.size() == 1);
    ponko::world lights;
    auto lamp = lights.spawn(ponko::transform3d_at({1, 2, 3}), ponko::point_light3d{ponko::colors::red, 20, 12, false});
    auto beam = lights.spawn(ponko::transform3d_at({0, 3, 0}), ponko::spot_light3d{ponko::colors::white, 40, 25, .2f, .5f, true, 256});
    CHECK(lights.get<ponko::point_light3d>(lamp).range == 12);
    CHECK(lights.get<ponko::spot_light3d>(beam).inner_angle == .2f);
    CHECK(lights.get<ponko::spot_light3d>(beam).shadows);
    CHECK(lights.get<ponko::spot_light3d>(beam).shadow_resolution == 256);
    CHECK(lights.get<ponko::point_light3d>(lamp).shadow_resolution == 0);
    lights.each([](ponko::point_light3d& p) { p.intensity += 1; });
    CHECK(lights.get<ponko::point_light3d>(lamp).intensity == 21);
    std::vector<ponko::entity> cubes;
    ponko::mesh cube{};
    auto spin = [](ponko::ctx& ctx) {
        ctx.world().each([](ponko::transform3d& t, const ponko::model&) { t.rotation = ponko::quat_from_rotation_y(0.1f) * t.rotation; });
    };
    auto w = ponko::game("3d", 640, 480).sim_system(spin, "spin").run_headless(5, [&](ponko::ctx& ctx) {
        auto stone = ctx.create_material(ponko::material_of(ponko::rgb(.5f, .5f, .5f)));
        cube = ctx.create_mesh(ponko::mesh_data::cube(1), stone);
        for (int i = 0; i < 3; ++i)
            cubes.push_back(ctx.world().spawn(ponko::transform3d_at({float(i) * 2, 0, 0}), ponko::model_of(cube, ponko::colors::red)));
        throws([&] { ctx.create_mesh(ponko::mesh_data::cube(1), ponko::material{99}); });
        auto shiny = ponko::material_of(ponko::colors::white);
        shiny.metallic = 2;
        throws([&] { ctx.create_material(shiny); });
        throws([] { ponko::mesh_data({{0, 0, 0}, {1, 0, 0}, {0, 1, 0}}, {0, 1}); });
        auto tile = ctx.create_terrain_tile(std::vector<float>(81, 0.0f), 16, stone);
        CHECK(tile.lod_distance == 16 && tile.lods[0].id != tile.lods[3].id);
        ctx.world().spawn(ponko::transform3d_at({0, 0, 0}), tile);
        throws([&] { ctx.create_terrain_tile(std::vector<float>(80, 0.0f), 16, stone); });
        throws([] { ponko::mesh_data::heightfield({0, 0, 0}, 1); });
        auto layers = ctx.create_texture_array(1, 1, std::vector<std::uint8_t>(8, 200), ponko::texture_kind::color);
        CHECK(layers.layers == 2);
        auto weights = ctx.create_texture_array(1, 1, std::vector<std::uint8_t>(4, 255), ponko::texture_kind::data);
        auto ground = ctx.create_terrain_material(ponko::terrain_material_of(layers, weights, {0, 0}, {16, 16}));
        CHECK(ground.id != stone.id);
        throws([&] { ctx.create_texture_array(1, 1, std::vector<std::uint8_t>(6, 0), ponko::texture_kind::color); });
        throws([&] { ctx.create_terrain_material(ponko::terrain_material_of(layers, weights, {0, 0}, {0, 0})); });
        auto grey = ctx.create_effect("grey", "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(p.color.g); }");
        ctx.set_effects({ponko::screen_effect_of(grey), ponko::screen_effect_of(grey, stone)});
        CHECK(ctx.effects().size() == 2 && ctx.effects()[1].material.id == stone.id);
        throws([&] { ctx.create_effect("bad", "fn effect(p: Pixel) -> f32 { return 1.0; }"); });
        {
            auto shaders = ctx.shader_status();
            CHECK(shaders.size() >= 2);
            auto& grey_status = shaders[shaders.size() - 2];
            CHECK(grey_status.shader == "grey" && grey_status.ok() && grey_status.errors.empty());
            CHECK(grey_status.info.kind == PONKO_SHADER_EFFECT && grey_status.info.outcome == PONKO_SHADER_COMPILED);
            auto& bad = shaders.back();
            CHECK(bad.shader == "bad" && !bad.ok() && bad.info.outcome == PONKO_SHADER_FAILED);
            CHECK(bad.info.diagnostics == 1 && bad.info.line > 0 && bad.errors.rfind("bad", 0) == 0);
            CHECK(ctx.shader_status_json().find("\"outcome\":\"failed\"") != std::string::npos);
        }
        throws([&] { ctx.set_effects({ponko::screen_effect_of(ponko::effect{99})}); });
        ctx.set_effects({});
        CHECK(ctx.effects().empty());
        auto mirror = ctx.create_render_target(32, 16);
        CHECK(mirror.width == 32 && mirror.height == 16);
        auto eye = ctx.camera3d();
        eye.position = {0, 5, 0};
        auto below = ctx.reflection_camera(eye, mirror, 1);
        CHECK(below.camera.position.y == -3 && below.has_clip_plane && below.clip_plane[3] == -1);
        ctx.set_target_cameras({below, ponko::target_camera_of(eye, mirror)});
        CHECK(ctx.target_cameras().size() == 2 && !ctx.target_cameras()[1].has_clip_plane);
        const std::uint8_t px[4] = {255, 255, 255, 255};
        auto plain = ctx.create_texture(1, 1, px, sizeof px);
        throws([&] { ctx.set_target_cameras({ponko::target_camera_of(eye, plain)}); });
        throws([&] { ctx.create_render_target(0, 4); });
        ctx.set_target_cameras({});
        auto batch_material = ctx.create_material(ponko::material_of(ponko::colors::white));
        auto batch_mesh = ctx.create_mesh(ponko::mesh_data::cube(1.0f), batch_material);
        std::vector<std::array<float,16>> batch_matrices = {{{1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1}}};
        auto batch = ctx.create_instance_batch(batch_mesh,batch_matrices);
        ctx.update_instance_batch(batch,batch_matrices);
        ctx.free_instance_batch(batch);
        throws([&] {ctx.update_instance_batch(batch,batch_matrices);});
        auto buffer = ctx.create_gpu_buffer(65);
        ctx.write_gpu_buffer(buffer, 64, {{1, 2, 3, 4}});
        throws([&] { ctx.write_gpu_buffer(buffer, 65, {{1, 2, 3, 4}}); });
        auto spare = ctx.create_gpu_buffer(2);
        ctx.free_gpu_buffer(spare);
        throws([&] { ctx.free_gpu_buffer(spare); });
        throws([&] { ctx.write_gpu_buffer(spare, 0, {{1, 2, 3, 4}}); });
        CHECK(ctx.create_gpu_buffer(3).id == spare.id);  // a freed buffer's id is taken again
        auto update = ctx.create_compute("update", "fn update(i: u32) { items[i].x += frame.time.y; }");
        ctx.set_compute_passes({ponko::buffer_compute_pass(update, buffer)});
        CHECK(ctx.compute_passes()[0].has_buffer && ctx.compute_passes()[0].buffer.id == buffer.id);
        auto fade = ctx.create_compute("fade", "fn compute(c: Cell) -> vec4<f32> { return c.value * 0.9; }");
        ctx.set_compute_passes({ponko::compute_pass_of(fade, mirror), ponko::compute_pass_of(fade, mirror, stone)});
        CHECK(ctx.compute_passes().size() == 2 && ctx.compute_passes()[1].material.id == stone.id);
        throws([&] { ctx.create_compute("bad", "fn compute(c: Cell) -> f32 { return 1.0; }"); });
        throws([&] { ctx.set_compute_passes({ponko::compute_pass_of(fade, plain)}); });
        throws([&] { ctx.set_compute_passes({ponko::compute_pass_of(ponko::compute{9}, mirror)}); });
        ctx.set_compute_passes({});

        auto cam = ctx.camera3d();
        cam.position = {0, 5, 10};
        ctx.set_camera3d(cam);
        ctx.camera3d_look_at({0, 0, 0});
        CHECK(!ctx.occlusion_culling());
        ctx.set_occlusion_culling(true);
        CHECK(ctx.occlusion_culling());
        ctx.set_occlusion_culling(false);
        CHECK(ponko::forward(ponko::looking_at(ponko::transform3d_at(cam.position), {0, 0, 0})).z < 0);
        cam.near = 0;
        throws([&] { ctx.set_camera3d(cam); });
        auto light = ctx.lighting();
        CHECK(light.shadows && light.shadow_cascades == 4);
        light.sun_direction = {-1, -1, 0};
        light.shadow_cascades = 2;
        CHECK(!light.has_equator);
        light.has_equator = true;
        light.equator_color = ponko::rgb(0, 1, 0);
        ctx.set_lighting(light);
        CHECK(ctx.lighting().shadow_cascades == 2);
        CHECK(ctx.lighting().has_equator && ctx.lighting().equator_color.g == 1);
        light.has_equator = false;
        ctx.set_lighting(light);
        CHECK(!ctx.lighting().has_equator);
        {
            auto bounced = light;
            CHECK(!bounced.gi_enabled && bounced.gi_extent.x == 32);
            bounced.gi_enabled = true;
            bounced.gi_spacing = 0;
            throws([&] { ctx.set_lighting(bounced); });
            bounced.gi_spacing = 1.5f;
            ctx.set_lighting(bounced);
            CHECK(ctx.lighting().gi_enabled && ctx.lighting().gi_spacing == 1.5f);
            ctx.set_lighting(light);
        }
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
        sky.light_shafts = 2;
        throws([&] { ctx.set_sky(sky); });
        sky.light_shafts = 1;
        ctx.set_sky(sky);
        CHECK(ctx.sky().light_shafts == 1);
        sky.light_shafts = 0;
        sky.sun_size = 0;
        throws([&] { ctx.set_sky(sky); });
        CHECK(ponko::sun_direction_at(12).y < -0.8f);
        auto post = ctx.post();
        CHECK(post.tonemap == uint32_t(ponko::tonemap::aces) && post.contrast == 1 && !post.has_lut);
        post.tonemap = uint32_t(ponko::tonemap::neutral);
        post.bloom = 0.2f;
        CHECK(post.anti_aliasing == 0 && !post.ao_enabled && post.ao_radius == 1);
        post.anti_aliasing = uint32_t(ponko::anti_aliasing::smaa);
        post.ao_enabled = true; post.ao_half_resolution = true; post.ao_temporal = true; post.ao_radius = 2; post.ao_intensity = 1.5f;
        post.ao_slices = 4; post.ao_steps = 8;
        ctx.set_post(post);
        CHECK(ctx.post().tonemap == uint32_t(ponko::tonemap::neutral));
        CHECK(ctx.post().ao_half_resolution && ctx.post().ao_temporal && ctx.post().ao_enabled && ctx.post().ao_radius == 2 && ctx.post().ao_steps == 8);
        CHECK(ctx.post().anti_aliasing == uint32_t(ponko::anti_aliasing::smaa));
        auto invalid_post = post; invalid_post.anti_aliasing = 4;
        throws([&] { ctx.set_post(invalid_post); });
        invalid_post = post; invalid_post.taa_sharpness = 2;
        throws([&] { ctx.set_post(invalid_post); });
        invalid_post = post; invalid_post.contact_shadows = -1;
        throws([&] { ctx.set_post(invalid_post); });
        invalid_post = post; invalid_post.ssr = 1.5f;
        throws([&] { ctx.set_post(invalid_post); });
        invalid_post = post; invalid_post.ssr_max_distance = -1;
        throws([&] { ctx.set_post(invalid_post); });
        {
            auto reflective = post;
            reflective.ssr = 0.7f;
            reflective.ssr_max_distance = 15;
            ctx.set_post(reflective);
            CHECK(ctx.post().ssr == 0.7f && ctx.post().ssr_max_distance == 15);
            ctx.set_post(post);
        }
        {
            auto temporal = post;
            temporal.anti_aliasing = uint32_t(ponko::anti_aliasing::taa);
            temporal.taa_sharpness = 0.5f;
            temporal.ao_enabled = true;
            temporal.ao_temporal = true;
            ctx.set_post(temporal);
            ctx.reset_temporal_history();
            CHECK(ctx.post().anti_aliasing == 3 && ctx.post().taa_sharpness == 0.5f);
            ctx.set_post(post);
        }
        invalid_post = post; invalid_post.ao_radius = 0;
        throws([&] { ctx.set_post(invalid_post); });
        post.saturation = -1;
        throws([&] { ctx.set_post(post); });

        // Graphics quality presets, switched at run time.
        CHECK(ctx.post().render_scale == 1 && ctx.post().sharpen == 0);
        auto quality = ponko::graphics_quality_preset(ponko::quality_level::medium);
        CHECK(ponko::graphics_quality_level(quality) == ponko::quality_level::medium);
        CHECK(quality.shadow_cascades == 3 && quality.anti_aliasing == uint32_t(ponko::anti_aliasing::fxaa));
        ctx.set_graphics_quality(quality);
        CHECK(ctx.lighting().shadow_cascades == 3 && ctx.lighting().shadow_resolution == 2048);
        CHECK(ctx.graphics_quality().light_shadow_views == 4);
        CHECK(ctx.lighting().light_shadow_atlas == 2048);
        quality = ponko::graphics_quality_preset(ponko::quality_level::low);
        ctx.set_graphics_quality(quality);
        CHECK(ctx.post().render_scale < 1 && ctx.lighting().light_shadow_views == 0);
        quality.shadow_cascades = 3;
        CHECK(ponko::graphics_quality_level(quality) == ponko::quality_level::custom);
        auto bad_quality = quality; bad_quality.render_scale = 4;
        throws([&] { ctx.set_graphics_quality(bad_quality); });
        throws([&] { ponko::graphics_quality_preset(ponko::quality_level::custom); });
        CHECK(ctx.graphics_quality().shadow_cascades == 2);
        // GPU tiers: no GPU headless; tier reports and presets need none.
        CHECK(!ctx.gpu_capabilities() && !ctx.gpu_tier() && !ctx.recommended_graphics_quality());
        CHECK(!ctx.gpu_has(ponko::gpu_feature::compute));
        CHECK(ponko::gpu_tier_report(ponko::gpu_tier::desktop).find("\"tier\":\"desktop\"") != std::string::npos);
        CHECK(!ponko::gpu_tier_has(ponko::gpu_tier::webgl2, ponko::gpu_feature::screen_space_reflections));
        CHECK(ponko::gpu_tier_has(ponko::gpu_tier::baseline, ponko::gpu_feature::compute_culling));
        CHECK(!ponko::gpu_tier_has(ponko::gpu_tier::baseline, ponko::gpu_feature::gpu_skinning));
        CHECK(ponko::graphics_quality_level(ponko::graphics_quality_for_tier(ponko::gpu_tier::baseline)) == ponko::quality_level::medium);
        CHECK(ponko::graphics_quality_for_tier(ponko::gpu_tier::webgl2).anti_aliasing == uint32_t(ponko::anti_aliasing::fxaa));
        throws([&] { ponko::gpu_tier_has(ponko::gpu_tier(7), ponko::gpu_feature::compute); });
        ctx.set_graphics_quality(ponko::graphics_quality_preset(ponko::quality_level::high));
        CHECK(ctx.graphics_quality().ssr == 1 && ctx.post().ssr == 1);
        CHECK(!ctx.graphics_quality().gi && ponko::graphics_quality_preset(ponko::quality_level::ultra).gi);
        auto light_q = ctx.lighting();
        light_q.shadow_resolution = 100;
        throws([&] { ctx.set_lighting(light_q); });

        auto tiles = ctx.create_shader("tiles", R"(
            fn surface(s: Surface) -> Surface {
                var out = s;
                out.albedo *= step(0.5, fract(s.uv.x * 8.0));
                return out;
            })");
        auto look = ponko::material_of(ponko::rgb(1, 1, 1));
        look.has_shader = true;
        look.shader = tiles;
        look.blend = uint32_t(ponko::blend::transparent);
        auto tiled = ctx.create_material(look);
        look.double_sided = true;
        ctx.update_material(tiled, look);
        throws([&] { ctx.create_shader("broken", "fn surface(s: Surface) -> Surface { return 1; }"); });
    });
    const auto t = w.get<ponko::transform3d>(cubes[1]);
    CHECK(t.position.x == 2 && std::abs(t.rotation.y - std::sin(0.25f)) < 1e-5f);
    CHECK(w.get<ponko::model>(cubes[2]).mesh.id == cube.id);
    // Quaternion math matches the engine's.
    const auto q = ponko::quat_from_rotation_y(1.5707964f);
    const auto v = q * ponko::vec3{1, 0, 0};
    CHECK(std::abs(v.z + 1) < 1e-5f);
    auto tri = ponko::mesh_data({{0, 0, 0}, {1, 0, 0}, {0, 1, 0}}, {0, 1, 2});
    CHECK(tri.view().vertex_count == 3 && tri.view().normals == nullptr);
    tri.set_colors({{1, 0, 0, 1}, {0, 1, 0, 1}, {0, 0, 1, 1}});
    throws([&] { tri.set_colors({{1, 1, 1, 1}}); });
}

static void parallel_queries() {
    ponko::world w;
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

struct Fuel { int value; };
struct Heat { int value; };

static void change_detection() {
    using list = std::vector<ponko::entity>;
    ponko::world w;
    w.track_changes<Fuel>();
    const auto a = w.spawn(Fuel{1}, Heat{1}), b = w.spawn(Fuel{2}, Heat{2}), c = w.spawn(Fuel{3});
    auto changed = [&] { list out; w.each<ponko::changed<Fuel>>([&](ponko::entity e, const Fuel&) { out.push_back(e); }); return out; };
    auto added = [&] { list out; w.each<ponko::added<Fuel>>([&](ponko::entity e, const Fuel&) { out.push_back(e); }); return out; };
    // Outside a game the reader keeps its own place: look, then move on.
    auto next = [&] { w.set_changes_since(w.advance_change_tick()); };
    CHECK(w.change_tick() == 1 && w.changes_since() == 0);
    CHECK(changed() == list{a, b, c} && added() == list{a, b, c});   // nothing seen yet: everything
    next();
    CHECK(w.change_tick() == 2 && w.changes_since() == 1);
    CHECK(changed().empty() && added().empty() && w.count<ponko::changed<Fuel>>() == 0);

    // Reads change nothing: const&, copies, const columns, get, count.
    w.each([](const Fuel&) {}); w.each([](Fuel) {}); w.each_par([](const Fuel&, const Heat&) {});
    w.query<const Fuel>([](auto chunk) {
        auto [fuel] = chunk.columns();
        static_assert(std::is_same<decltype(fuel), const Fuel*>::value, "const columns are read-only");
    });
    (void)w.get<Fuel>(a); CHECK(w.count<Fuel>() == 3);
    CHECK(changed().empty());

    // Writes do: set, a `&` parameter (every row visited), a column, mark_changed.
    w.set(b, Fuel{20}); CHECK(changed() == list{b} && added().empty());
    next();
    w.each([](Fuel&, const Heat&) {}); CHECK(changed() == list{a, b});
    next();
    w.query<Fuel>([](auto) {}); CHECK(changed() == list{a, b, c});
    next();
    w.query_par<const Heat, Fuel>([](auto) {}); CHECK(changed() == list{a, b});
    next();
    CHECK(w.mark_changed<Fuel>(c) && !w.mark_changed<Heat>(c)); CHECK(changed() == list{c});
    CHECK(w.count<ponko::changed<Fuel>>() == 1 && w.count<Heat, ponko::changed<Fuel>>() == 0);

    // A filtered loop that writes marks only the rows it visits.
    next();
    w.set(a, Fuel{5});
    w.each<ponko::changed<Fuel>>([](Fuel& f) { f.value += 1; });
    CHECK(w.get<Fuel>(a).value == 6 && w.get<Fuel>(b).value == 20 && changed() == list{a});
    // Filtered columns: runs of rows, numbered by first().
    w.mark_changed<Fuel>(c);
    std::size_t rows = 0;
    w.query<Fuel, ponko::changed<Fuel>>([&](auto chunk) { CHECK(chunk.first() == rows); rows += chunk.size(); });
    CHECK(rows == 2);

    // Added: a new component on an old entity; a filter needn't be a column.
    next();
    w.remove<Fuel>(c); w.set(c, Fuel{7}); w.set(a, Fuel{8});
    CHECK(added() == list{c} && w.count<ponko::changed<Fuel>>() == 2);
    list heated; w.each<ponko::added<Fuel>>([&](ponko::entity e, const Heat&) { heated.push_back(e); });
    CHECK(heated.empty());
    w.set(c, Heat{3});
    w.each<ponko::added<Fuel>>([&](ponko::entity e, const Heat&) { heated.push_back(e); });
    CHECK(heated == list{c});
    // Several filters must all pass; any_of needs one.
    CHECK(w.count<ponko::changed<Fuel>, ponko::added<Fuel>>() == 1);
    CHECK((w.count<ponko::any_of<ponko::changed<Fuel>, ponko::added<Fuel>>>() == 2));
    // A filter starts tracking: Heat is new to it, so all three count.
    CHECK(w.count<ponko::changed<Heat>>() == 3);

    // Removals: a removed component or a despawned entity, oldest first.
    CHECK(w.removed<Fuel>() == list{c});   // from the remove above
    next();
    CHECK(w.removed<Fuel>().empty());
    w.remove<Fuel>(a); w.despawn(b);
    CHECK(w.removed<Fuel>() == list{a, b} && w.removed<Heat>() == list{b});
    const auto seen = w.advance_change_tick();
    CHECK(w.removed_since<Fuel>(seen).empty() && w.removed_since<Fuel>(0) == list{c, a, b});
    throws([&] { (void)w.removed<Pos>(); });   // not tracked
    throws([&] { ponko::world moved(std::move(w)); (void)w.change_tick(); });

    // On all cores: each passing row once.
    ponko::world big;
    big.track_changes<int>();
    std::vector<ponko::entity> es;
    for (int i = 0; i < 50000; ++i) es.push_back(i % 2 ? big.spawn(i) : big.spawn(i, 'x'));
    big.set_changes_since(big.advance_change_tick());
    for (std::size_t i = 0; i < es.size(); i += 3) big.mark_changed<int>(es[i]);
    std::atomic<long long> sum{0}, visits{0};
    big.each_par<ponko::changed<int>>([&](const int& n) { sum += n; ++visits; });
    long long want = 0, n = 0;
    for (int i = 0; i < 50000; i += 3) { want += i; ++n; }
    CHECK(sum == want && visits == n && big.count<ponko::changed<int>>() == std::size_t(n));

    // In a game every system has its own place: each change is seen once,
    // by a reader on either side of the writer, also through commands.
    for (bool reader_first : {true, false}) {
        std::vector<std::size_t> seen_changes, seen_removed, own;
        ponko::entity target = 0;
        auto reader = [&](ponko::ctx& ctx) {
            seen_changes.push_back(ctx.world().count<ponko::changed<Fuel>>());
            seen_removed.push_back(ctx.world().removed<Fuel>().size());
        };
        auto writer = [&](ponko::ctx& ctx) {
            own.push_back(ctx.world().count<ponko::changed<Fuel>>());   // its own writes aren't news to it
            if (ctx.sim_tick() == 1) ctx.world().set(target, Fuel{9});
            if (ctx.sim_tick() == 3) ctx.set_later(target, Fuel{10});
            if (ctx.sim_tick() == 5) ctx.despawn_later(target);
        };
        ponko::game g("t", 1, 1);
        if (reader_first) g.sim_system(reader, "reader").sim_system(writer, "writer");
        else g.sim_system(writer, "writer").sim_system(reader, "reader");
        g.run_headless(8, [&](ponko::ctx& ctx) {
            ctx.world().track_changes<Fuel>();
            target = ctx.world().spawn(Fuel{0}); ctx.world().spawn(Fuel{0});
        });
        const std::size_t late = reader_first ? 1 : 0;
        std::vector<std::size_t> changes(8, 0), removals(8, 0);
        changes[0] = 2; changes[1 + late] = 1; changes[3 + late] = 1; removals[5 + late] = 1;
        CHECK(seen_changes == changes && seen_removed == removals);
        CHECK(own == std::vector<std::size_t>{2, 0, 0, 0, 0, 0, 0, 0});
    }
}

struct Hp { int value; };
struct Bullet { float speed; };

static void games() {
    // Systems, the clock, quit and exceptions.
    std::uint64_t ticks = 0; float dt = 0;
    auto sim = ponko::game("t", 1, 1).fixed_hz(50)
        .sim_system([&](ponko::ctx& ctx) {
            ++ticks; dt = ctx.dt();
            CHECK(ctx.gamepads().empty());   // no pads in headless runs
            if (ctx.sim_tick() == 2) ctx.quit();
        }, "count")
        .run_headless(100, [&](ponko::ctx& ctx) { ctx.world().spawn(Pos{1, 1}); });
    CHECK(ticks == 3 && dt > .019f && dt < .021f && sim.size() == 1);
    // The returned world knows the game's components.
    sim.each([](const Pos& p) { CHECK(p.x == 1); });
    throws([] { ponko::game("t", 1, 1).run_headless(1, [](ponko::ctx&) { throw std::runtime_error("setup"); }); });
    bool second = false;
    throws([&] { ponko::game("t", 1, 1).sim_system([](ponko::ctx&) { throw std::runtime_error("tick"); })
                     .sim_system([&](ponko::ctx&) { second = true; }).run_headless(5); });
    CHECK(!second);
    throws([] { ponko::game("t", 1, 1).fixed_hz(0); });
    throws([] { ponko::game("t", 1, 1).remove_system("nope"); });
    ponko::game g("t", 1, 1);
    g.run_headless(1);
    throws([&] { g.run_headless(1); });

    // Commands from inside a loop (Bullet is first used there); spatial
    // queries; random streams; built-ins.
    std::vector<ponko::grid_hit> hits;
    std::size_t near = 0;
    std::uint64_t first_random = 0;
    auto w = ponko::game("t", 1, 1).spatial_grid(16).seed(3)
        .sim_system([&](ponko::ctx& ctx) {
            ctx.world().each([&](ponko::entity e, const Hp& hp) {
                if (hp.value <= 0) { ctx.despawn_later(e); ctx.spawn_later(Hp{100}, Bullet{2}); }
            });
            ctx.spatial_radius({45, 0}, 16, hits);
            near = hits.size();
            CHECK(ctx.spatial_nearest({1, 0}, 100).has_value());
            CHECK(!ctx.spatial_nearest({1e6f, 0}, 1).has_value());
            first_random = ctx.random().of(7).u64();
            ctx.quit();
        })
        .run_headless(10, [&](ponko::ctx& ctx) {
            auto world = ctx.world();
            std::uint8_t px[16] = {};
            auto tex = ctx.create_texture(2, 2, px, sizeof px);
            for (int x = 0; x < 10; ++x)
                world.spawn(Hp{x % 2 ? 5 : 0}, ponko::transform_at({x * 10.0f, 0}),
                            ponko::sprite_of(tex, ponko::colors::red, float(x)));
        });
    CHECK(w.size() == 10);
    CHECK(near == 4);
    long long total = 0;
    w.each([&](const Hp& hp) { total += hp.value; });
    CHECK(total == 5 * 5 + 5 * 100);
    CHECK(w.count<Hp, Bullet>() == 5);
    CHECK(first_random == ponko::random(0).of(7).u64() || first_random != 0);

    // set_later and remove_later, also for types first used inside a loop.
    auto changed = ponko::game("t", 1, 1)
        .sim_system([](ponko::ctx& ctx) {
            ctx.world().each([&](ponko::entity e, Hp& hp) {
                if (hp.value == 1) ctx.set_later(e, Vel{1, 1});
                if (hp.value == 2) ctx.remove_later<Tag>(e);
            });
        })
        .run_headless(1, [](ponko::ctx& ctx) { ctx.world().spawn(Hp{1}); ctx.world().spawn(Hp{2}, Tag{}); });
    CHECK(changed.count<Vel>() == 1 && changed.count<Tag>() == 0);
}

static void grid_views_match_engine_queries() {
    ponko::grid g(8);
    ponko::rng r(5);
    std::vector<ponko::grid_item> items;
    for (std::uint32_t i = 0; i < 20000; ++i)
        items.push_back({{i, 0}, {r.range(-500, 500), r.range(-500, 500)}, i});
    g.rebuild(items);
    auto view = g.view();
    CHECK(view.size() == items.size());
    std::vector<ponko::grid_item> engine;
    for (int q = 0; q < 300; ++q) {
        const ponko::vec2 c{r.range(-600, 600), r.range(-600, 600)};
        const float radius = q % 50 == 0 ? 2000.0f : r.range(0, 40);
        g.radius(c, radius, engine);
        std::vector<std::uint32_t> a, b;
        for (auto& e : engine) a.push_back(e.data);
        view.for_each_in_radius(c, radius, [&](const ponko::grid_item& e) { b.push_back(e.data); });
        std::sort(a.begin(), a.end());
        std::sort(b.begin(), b.end());
        CHECK(a == b);
    }
    throws([&] { ponko::grid_view<ponko::grid_hit> wrong(PonkoGridView{nullptr, 20, nullptr, nullptr, 1, 0, 0, 0, 0, 0}); });
}

struct Died { ponko::entity who; std::uint64_t tick; };
struct Never { int x; };

static void events() {
    // Sent from inside a loop (the first send is the type's first use), read
    // by a later system in the same tick, once each.
    std::vector<std::uint64_t> seen;
    auto w = ponko::game("t", 1, 1)
        .sim_system([](ponko::ctx& ctx) {
            ctx.world().each([&](ponko::entity e, const Hp& hp) {
                if (hp.value <= 0) ctx.send(Died{e, ctx.sim_tick()});
            });
        }, "deaths")
        .sim_system([&](ponko::ctx& ctx) {
            for (const Died& d : ctx.read<Died>()) { seen.push_back(d.tick); ctx.despawn_later(d.who); }
            CHECK(ctx.read<Died>().empty());
            CHECK(ctx.read<Never>().empty());
        }, "funerals")
        .run_headless(3, [](ponko::ctx& ctx) {
            for (int h : {0, 5, -1}) ctx.world().spawn(Hp{h});
        });
    CHECK(w.size() == 1);
    CHECK((seen == std::vector<std::uint64_t>{0, 0}));
}

static void rng_matches_rust() {
    // The same numbers as Rust's Rng (pinned in src/random.rs).
    ponko::rng r(42);
    CHECK(r.u64() == 0xc757960b442b0ac3ull);
    CHECK(r.u64() == 0x4bb22a7f77ff8c6cull);
    CHECK(r.u64() == 0x04950439d3c5eafeull);
    CHECK(ponko::random(7).stream(3).u64() == 0x6045c0e00af2e148ull);
    ponko::rng f(1);
    for (int i = 0; i < 1000; ++i) { float x = f.f32(); CHECK(x >= 0 && x < 1); CHECK(f.below(10) < 10); }
}

static void attachments() {
    // Chains follow their root; orphans are despawned with it.
    ponko::entity root = 0, arm = 0, hand = 0;
    ponko::transform2d seen{};
    auto w = ponko::game("t", 1, 1)
        .sim_system([&](ponko::ctx& ctx) {
            auto world = ctx.world();
            auto t = world.get<ponko::transform2d>(root);
            t.position.x += 1;
            world.set(root, t);
            if (ctx.sim_tick() == 2) ctx.despawn_later(root);
        })
        .follow_attachments()
        .sim_system([&](ponko::ctx& ctx) {
            if (ctx.sim_tick() == 1) seen = ctx.world().get<ponko::transform2d>(hand);
        })
        .run_headless(3, [&](ponko::ctx& ctx) {
            auto world = ctx.world();
            root = world.spawn(ponko::transform_at({5, 5}));
            arm = world.spawn(ponko::transform_at({0, 0}), ponko::attach_to(root, {1, 0}));
            hand = world.spawn(ponko::transform_at({0, 0}), ponko::attach_to(arm, {0, 1}, 0.5f));
        });
    CHECK(seen.position == (ponko::vec2{8, 6}) && seen.rotation == 0.5f);
    CHECK(ponko::bits(ponko::entity_of(arm)) == arm);
    CHECK(!w.alive(root) && !w.alive(arm) && !w.alive(hand));
}

static void physics() {
    // A ball drops through a sensor onto the floor; queries and events work from C++.
    ponko::entity ball = 0, floor = 0, sensor = 0;
    std::vector<ponko::collision> started;
    std::optional<ponko::ray_hit> hit;
    std::optional<float> mass;
    bool kicked_nothing = true;
    auto w = ponko::game("t", 1, 1)
        .physics({0, 980})
        .sim_system([&](ponko::ctx& ctx) {
            for (const auto& c : ctx.collisions_started()) started.push_back(c);
            if (ctx.sim_tick() == 1) {
                hit = ctx.cast_ray({150, 0}, {0, 1}, 1000);
                mass = ctx.mass(ball);
                kicked_nothing = ctx.apply_impulse(12345, {1, 0});
                std::vector<PonkoEntity> at;
                ctx.entities_at({150, 300}, at);
                CHECK(at.size() == 1 && ponko::bits(at[0]) == floor);
            }
        })
        .run_headless(120, [&](ponko::ctx& ctx) {
            auto world = ctx.world();
            auto bouncy = ponko::circle_collider(10);
            bouncy.restitution = 0;
            ball = world.spawn(ponko::transform_at({0, 0}), ponko::dynamic_body(), bouncy, ponko::velocity{});
            auto trigger = ponko::rect_collider({80, 20});
            trigger.sensor = true;
            sensor = world.spawn(ponko::transform_at({0, 100}), ponko::fixed_body(), trigger);
            floor = world.spawn(ponko::transform_at({0, 300}), ponko::fixed_body(), ponko::rect_collider({400, 20}));
        });
    CHECK(std::fabs(w.get<ponko::transform2d>(ball).position.y - 280) < 2);
    CHECK(std::fabs(w.get<ponko::velocity>(ball).linear.y) < 5);
    CHECK(w.get<ponko::rigid_body>(floor).kind == PONKO_BODY_FIXED);
    CHECK(w.get<ponko::collider>(sensor).sensor);
    auto touched = [&](ponko::entity a, ponko::entity b) {
        return std::any_of(started.begin(), started.end(), [&](const ponko::collision& c) {
            const auto x = ponko::bits(c.a), y = ponko::bits(c.b);
            return (x == a && y == b) || (x == b && y == a);
        });
    };
    CHECK(touched(ball, sensor) && touched(ball, floor));

    // A wedge, flat side down, comes to rest on it (as tests/physics.rs).
    ponko::entity wedge = 0;
    auto rested = ponko::game("t", 1, 1).physics({0, 980}).run_headless(240, [&](ponko::ctx& ctx) {
        auto world = ctx.world();
        auto shape = ponko::polygon_collider({{-30, 15}, {30, 15}, {-30, -15}});
        shape.friction = 0.9f;
        wedge = world.spawn(ponko::transform_at({0, 0}), ponko::dynamic_body(), shape);
        world.spawn(ponko::transform_at({0, 200}), ponko::fixed_body(), ponko::rect_collider({400, 20}));
    });
    const auto t = rested.get<ponko::transform2d>(wedge);
    CHECK(std::fabs(t.position.y - 175) < 1.5f && std::fabs(t.rotation) < 0.01f);
    CHECK(rested.collider_points(wedge).size() == 3);
    CHECK(rested.get<ponko::collider>(wedge).shape == PONKO_SHAPE_POLYGON);
    CHECK(rested.get<ponko::collider>(wedge).friction == 0.9f);
    throws([] { ponko::polygon_collider({{0, 0}, {1, 0}}); });
    CHECK(hit && ponko::bits(hit->entity) == floor && std::fabs(hit->distance - 290) < 1e-3f);
    CHECK(mass && std::fabs(*mass - 314.159f) < 1);
    CHECK(!kicked_nothing);
    // Bad values are rejected, not simulated.
    auto flat = ponko::rect_collider({10, 0});
    throws([&] { w.set(floor, flat); });
}

static void phases() {
    // add(phase::before_physics) feeds this tick's step; a sim_system, even
    // one added before physics3d, runs after the step (as tests/phases.rs).
    ponko::entity ball = 0;
    float before = -1, after = -1;
    ponko::game("t", 1, 1)
        .fixed_hz(50)
        .sim_system([&](ponko::ctx& ctx) { after = ctx.world().get<ponko::transform3d>(ball).position.x; }, "watch")
        .add(ponko::phase::before_physics, [&](ponko::ctx& ctx) {
            before = ctx.world().get<ponko::transform3d>(ball).position.x;
            ctx.world().set(ball, ponko::velocity3d{{5, 0, 0}, {0, 0, 0}});
        }, "kick")
        .physics3d({0, 0, 0})
        .run_headless(1, [&](ponko::ctx& ctx) {
            ball = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}), ponko::dynamic_body3d(),
                                     ponko::sphere_collider(0.5f), ponko::velocity3d{{0, 0, 0}, {0, 0, 0}});
        });
    CHECK(before == 0);
    CHECK(std::fabs(after - 5.0f / 50.0f) < 1e-3f);
    throws([] { ponko::game("t", 1, 1).add(ponko::phase::frame, [](ponko::ctx&) {}, "bad", 2); });
}

// A plugin with startup work, and one that only adds systems.
struct ScoreArea {
    std::string* log;
    void build(ponko::game& game) const {
        game.sim_system([log = log](ponko::ctx&) { *log += 'a'; }, "score");
    }
    void setup(ponko::ctx&) const { *log += 's'; }
};
struct KickArea {
    std::string* log;
    void build(ponko::game& game) const {
        game.add(ponko::phase::before_physics, [log = log](ponko::ctx&) { *log += 'b'; }, "kick");
    }
};

static void plugins_setups_and_system_ids() {
    // Plugins build at once and set up in the order they were added, with
    // setup() callbacks in between and the run setup last. The phase, not
    // the plugin order, decides when their systems run.
    std::string log;
    ponko::game("t", 1, 1)
        .plugin(ScoreArea{&log})
        .setup([&](ponko::ctx&) { log += 'c'; })
        .plugin(KickArea{&log})
        .run_headless(2, [&](ponko::ctx&) { log += 'r'; });
    CHECK(log == "scrbaba");
    // setup() alone runs with zero ticks, and applies its commands.
    auto w = ponko::game("t", 1, 1).setup([](ponko::ctx& ctx) { ctx.spawn_later(Pos{1, 2}); }).run_headless(0);
    CHECK(w.size() == 1);

    // Ids: two systems from the same function are told apart.
    log.clear();
    ponko::game g("t", 1, 1);
    auto mark = [&](char c) { return [&log, c](ponko::ctx&) { log += c; }; };
    ponko::system_id first = g.add_system(ponko::phase::after_physics, mark('1'), "same");
    ponko::system_id second = g.add_system(ponko::phase::after_physics, mark('2'), "same");
    CHECK(first != second && first.bits != 0);
    g.remove(first).remove(first);                  // twice is fine
    g.remove(ponko::systems::draw_sprites)
        .remove(ponko::systems::update_animators)
        .remove(ponko::systems::step_navmesh3d);    // not installed: nothing happens
    throws([&] { g.remove_system("ponko::ecs::draw_sprites"); });   // already gone
    g.run_headless(2);
    CHECK(log == "22");
    throws([&] { g.setup([](ponko::ctx&) {}); });   // already run
    throws([&] { g.plugin(KickArea{&log}); });
    throws([] { ponko::game("t", 1, 1).add_system(ponko::phase::frame, [](ponko::ctx&) {}, "bad", 2); });
    CHECK(ponko::systems::draw_sprites.bits == PONKO_SYSTEM_DRAW_SPRITES);
}

static void vehicles() {
    // A box car on raycast wheels settles on its suspension, then drives
    // off; the vehicle is a query column (as tests/vehicle3d.rs).
    ponko::entity car = 0;
    std::optional<ponko::vec3> center;
    auto w = ponko::game("t", 1, 1)
        .add(ponko::phase::before_physics, [](ponko::ctx& ctx) {
            if (ctx.sim_tick() < 100) return;
            ctx.world().each([](ponko::vehicle3d& v) {
                v.wheels[2].motor_torque = v.wheels[3].motor_torque = 500;
                v.wheels[0].steer_angle = v.wheels[1].steer_angle = 0.2f;
            });
        }, "drive")
        .physics3d({0, -9.81f, 0})
        .sim_system([&](ponko::ctx& ctx) {
            if (ctx.sim_tick() == 99) center = ctx.center_of_mass3d(car);
        }, "probe")
        .run_headless(200, [&](ponko::ctx& ctx) {
            auto world = ctx.world();
            world.spawn(ponko::transform3d_at({0, -0.5f, 0}), ponko::fixed_body3d(), ponko::box_collider({200, 1, 200}));
            const float x = 0.8f, y = -0.3f, z = 1.3f, r = 0.35f;
            car = world.spawn(ponko::transform3d_at({0, 1, 0}),
                              ponko::with_center_of_mass(ponko::with_mass(ponko::dynamic_body3d(), 1200), {0, -0.3f, 0}),
                              ponko::box_collider({1.8f, 0.6f, 4.2f}),
                              ponko::vehicle({ponko::wheel({-x, y, -z}, r), ponko::wheel({x, y, -z}, r),
                                               ponko::wheel({-x, y, z}, r), ponko::wheel({x, y, z}, r)}));
        });
    CHECK(center && std::fabs(center->y - 0.5f) < 0.02f);
    const auto v = w.get<ponko::vehicle3d>(car);
    const auto t = w.get<ponko::transform3d>(car);
    CHECK(v.wheel_count == 4 && v.wheels[0].grounded && ponko::rpm(v.wheels[3]) > 50);
    CHECK(t.position.z < -3 && t.position.x > 0.2f && t.position.y > 0.7f && t.position.y < 0.9f);
    CHECK(std::fabs(ponko::wheel_pose(v.wheels[0], t).position.y - 0.35f) < 0.05f);
    CHECK(ponko::evaluate(v.wheels[0].forward_friction, 0.4f) == 1);
    throws([] { ponko::vehicle({ponko::wheel({}, 1), ponko::wheel({}, 1), ponko::wheel({}, 1), ponko::wheel({}, 1),
                                 ponko::wheel({}, 1), ponko::wheel({}, 1), ponko::wheel({}, 1), ponko::wheel({}, 1),
                                 ponko::wheel({}, 1)}); });
}

static void lockstep() {
    // Two games over UDP on localhost see the same inputs, tick for tick.
    struct Order { std::uint32_t tick; float x; };
    struct Seen { std::uint64_t sum = 0; std::uint32_t others = 0, player = 9; bool desync = false; std::size_t loaded = 0; };
    // Both peers load it in setup and again at tick 50, in a sim system:
    // places every peer runs, so the loads keep them in step.
    ponko::game("scene", 1, 1).run_headless(0, [](ponko::ctx& ctx) {
        ctx.world().spawn(ponko::transform_at({1, 2}));
        ctx.world().spawn(ponko::transform_at({3, 4}));
        ctx.save_scene("test-cpp-lockstep.json");
    });
    auto play = [](ponko::game& g, Seen& seen) {
        g.sim_system([&seen](ponko::ctx& ctx) {
            if (ctx.sim_tick() == 50) {
                ctx.world().spawn(ponko::transform_at({5, 6}));
                ctx.load_scene_lockstep("test-cpp-lockstep.json");
                seen.loaded = ctx.world().size();
            }
            seen.player = ctx.player();
            ctx.set_input(Order{std::uint32_t(ctx.sim_tick()), float(ctx.player())});
            for (std::uint32_t p = 0; p < ctx.players(); ++p)
                if (auto o = ctx.input<Order>(p)) {
                    seen.sum += o->tick * (p + 1);
                    if (p != ctx.player()) ++seen.others;
                }
            seen.desync = ctx.desynced();
        });
        g.run_headless(200, [](ponko::ctx& ctx) { ctx.load_scene("test-cpp-lockstep.json"); });
    };
    std::uint16_t port = 0;
    Seen host_seen, guest_seen;
    ponko::game host("host", 1, 1);
    host.lockstep_host("127.0.0.1:0", 2, 2, 30, &port);
    CHECK(port != 0);
    std::thread host_thread([&] { play(host, host_seen); });
    ponko::game guest("guest", 1, 1);
    guest.lockstep_join(("127.0.0.1:" + std::to_string(port)).c_str());
    play(guest, guest_seen);
    host_thread.join();
    CHECK(host_seen.player == 0 && guest_seen.player == 1);
    CHECK(host_seen.sum == guest_seen.sum && host_seen.others > 150 && guest_seen.others > 150);
    CHECK(!host_seen.desync && !guest_seen.desync);
    CHECK(host_seen.loaded == 2 && guest_seen.loaded == 2);
    throws([] { ponko::game("t", 1, 1).lockstep_host("127.0.0.1:0", 1); });
    throws([] { ponko::game("t", 1, 1).lockstep_steam_host(480, 2); });   // built without `steam`
}

static void streaming() {
    ponko::cell_streamer cells(10, 25, 40);
    cells.update({5, 0, 5}, 2);
    CHECK(cells.to_load().size() == 2 && cells.to_load()[0].x == 0 && cells.to_load()[0].z == 0);
    cells.update({5, 0, 5}, 1000);
    CHECK(cells.loaded_count() > 2 && cells.is_loaded({1, 0}) && !cells.is_loaded({9, 9}));
    cells.update({5000, 0, 5}, 1000);
    CHECK(!cells.to_drop().empty() && !cells.is_loaded({0, 0}));
    CHECK(cells.corner({2, -1}).x == 20 && cells.corner({2, -1}).z == -10);
    cells.clear();
    CHECK(cells.loaded_count() == 0);
    throws([] { ponko::cell_streamer(0, 1, 1); });
    throws([] { ponko::terrain_tile_data(std::vector<float>(8, 0.0f), 4); });
    ponko::heightfield slope({0, 2, 0, 2}, 1);
    CHECK(std::abs(*slope.height_at(0.5f, 0.5f) - 1) < 1e-5f && slope.normal_at(0.5f, 0.5f)->x < 0);
    CHECK(!slope.height_at(2, 0.5f) && !slope.normal_at(-1, 0));
    throws([] { ponko::heightfield({0, 0, 0}, 1); });
    ponko::game("stream", 64, 64).run_headless(0, [](ponko::ctx& ctx) {
        auto ground = ctx.create_material(ponko::material_of(ponko::colors::white));
        // Built on another thread, uploaded on the game's.
        auto data = std::async(std::launch::async, [] { return ponko::terrain_tile_data(std::vector<float>(33 * 33, 1.0f), 64); });
        auto tile = ctx.upload_terrain_tile(data.get(), ground);
        CHECK(tile.lod_distance == 64 && ctx.is_mesh_live(tile.lods[0]));
        ctx.free_terrain_tile(tile);
        CHECK(!ctx.is_mesh_live(tile.lods[0]) && !ctx.is_mesh_live(tile.lods[3]));
        auto cube = ctx.create_mesh(ponko::mesh_data::cube(1), ground);
        CHECK(ctx.mesh_shadows(cube));
        ctx.set_mesh_shadows(cube, false);
        CHECK(!ctx.mesh_shadows(cube));
        ctx.free_mesh(cube);
        ctx.free_mesh(cube);
        CHECK(!ctx.is_mesh_live(cube));
    });
}

static void model_material_overrides() {
    ponko::game("overrides", 64, 64).run_headless(2, [](ponko::ctx& ctx) {
        auto gold = ctx.create_material(ponko::material_of(ponko::rgb(1, .8f, .2f)));
        auto wood = ctx.create_material(ponko::material_of(ponko::rgb(.5f, .3f, .1f)));
        auto cube = ctx.create_plain_mesh(ponko::mesh_data::cube(1));
        CHECK(ctx.is_mesh_live(cube));
        auto e = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}), ponko::model_of(cube));
        CHECK(!ctx.model_material(e));
        ctx.set_model_material(e, gold);
        CHECK(ctx.model_material(e) && ctx.model_material(e)->id == gold.id);
        throws([&] { ctx.set_model_material(e, ponko::material{99}); });
        ctx.clear_model_material(e);
        CHECK(!ctx.model_material(e));

        CHECK(ctx.model_part_materials(e).empty());
        ctx.set_model_part_materials(e, {{3, gold}, {0, wood}, {3, wood}});
        auto parts = ctx.model_part_materials(e);
        CHECK(parts.size() == 2 && parts[0].first == 0 && parts[0].second.id == wood.id);
        CHECK(parts[1].first == 3 && parts[1].second.id == wood.id);
        throws([&] { ctx.set_model_part_materials(e, {{0, ponko::material{99}}}); });
        CHECK(ctx.model_part_materials(e).size() == 2);
        ctx.set_model_part_materials(e, {});
        CHECK(ctx.model_part_materials(e).empty());
        // Left on for the frames after setup.
        ctx.set_model_material(e, gold);
        ctx.set_model_part_materials(e, {{0, wood}});
    });
}

static void asset_streaming() {
    ponko::game("streaming", 1, 1).run_headless(0, [](ponko::ctx& ctx) {
        ctx.set_deterministic_uploads(true);
        ctx.set_upload_budget(std::nullopt);
        CHECK(!ctx.stream_texture("crab.png"));
        CHECK(ctx.asset_state(ponko::residency_kind::texture, "crab.png") == ponko::asset_state::loading);
        ctx.finish_uploads();
        auto crab = ctx.stream_texture("crab.png");
        CHECK(crab && crab->width > 0 && ctx.is_live(*crab));
        CHECK(ctx.asset_state(ponko::residency_kind::texture, "crab.png") == ponko::asset_state::resident);
        CHECK(!ctx.upload_stats().has_budget && ctx.upload_stats().pending == 0);
        CHECK(ctx.resident_texture("crab.png").id == crab->id);
        CHECK(ctx.is_resident(ponko::residency_kind::texture, "crab.png"));
        ctx.set_residency_budget(ponko::residency_kind::texture, 1);
        auto r = ctx.residency(ponko::residency_kind::texture);
        CHECK(r.has_budget && r.budget == 1 && r.resident == 1 && r.over_budget_bytes > 0);  // asked for this tick
        auto house = ctx.resident_mesh("models/house.glb", true);
        CHECK(ctx.is_live(house) && ctx.residency(ponko::residency_kind::mesh).pinned == 1);
        ctx.unpin_resident(ponko::residency_kind::mesh, "models/house.glb");
        CHECK(!ctx.stream_mesh("models/nope.glb"));
        ctx.finish_uploads();
        CHECK(ctx.asset_state(ponko::residency_kind::mesh, "models/nope.glb") == ponko::asset_state::failed);
        CHECK(ctx.asset_error(ponko::residency_kind::mesh, "models/nope.glb").find("nope.glb") != std::string::npos);
        CHECK(ctx.asset_error(ponko::residency_kind::mesh, "models/house.glb").empty());
        auto e = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}));
        ctx.set_lod_group(e, 4, {ponko::lod_level{0.0f, 0, {ponko::lod_renderer_of(ponko::mesh_none)}, false, {}}});
        ctx.set_streamed_lod(e, {"models/house.glb"});
        throws([] { ponko::detail::check(ponko_ctx_set_streamed_lod(nullptr, 0, nullptr, 0)); });
    });
}

static void handle_lifecycle() {
    ponko::game("handles", 64, 64).run_headless(0, [](ponko::ctx& ctx) {
        auto before = ctx.live_counts();
        const std::uint8_t px[4] = {255, 255, 255, 255};
        auto t = ctx.create_texture(1, 1, px, sizeof px);
        auto m = ctx.create_material(ponko::material_of(ponko::colors::white));
        auto mesh = ctx.create_mesh(ponko::mesh_data::cube(1), m);
        auto target = ctx.create_render_target(8, 8);
        CHECK(ctx.is_live(t) && ctx.is_live(m) && ctx.is_live(mesh) && ctx.is_live(target));
        CHECK(ctx.live_counts().textures == before.textures + 2 && ctx.live_counts().render_targets == 1);
        ctx.free(t);
        ctx.free(m);
        ctx.free(mesh);
        ctx.free(target);
        ctx.free(t);  // again: nothing
        CHECK(!ctx.is_live(t) && !ctx.is_live(m) && !ctx.is_live(mesh) && !ctx.is_live(target));
        CHECK(!ctx.is_mesh_live(mesh));
        auto again = ctx.create_texture(1, 1, px, sizeof px);
        CHECK(again.id != t.id && ctx.is_live(again));
        ctx.free(again);
        CHECK(!ctx.is_live(ponko::none<ponko::texture>()) && !ctx.is_live(ponko::mesh_none));
        CHECK(ponko::none<ponko::mesh>().id == ponko::mesh_none.id);
        CHECK(ctx.is_live(ponko::font{}));  // the built-in font
        ctx.free(ponko::font{});
        CHECK(ctx.is_live(ponko::font{}));
        auto after = ctx.live_counts();
        CHECK(after.textures == before.textures && after.meshes == before.meshes && after.materials == before.materials);
        throws([] { ponko::detail::check(ponko_ctx_handle_free(nullptr, PONKO_HANDLE_MESH, 0)); });
    });
}

static void streamed_worlds() {
    // Far ground, cut where a tile is shown.
    ponko::game("far", 64, 64).run_headless(0, [](ponko::ctx& ctx) {
        auto ground = ctx.default_material();
        ponko::far_terrain far(std::vector<float>(129 * 129, 1.0f), 129, {-1024, -1024}, 16, 128, ground);
        far.update(ctx);
        CHECK(far.mesh_count() == 4 && !far.is_cut({0, 0}));
        far.cut({0, 0});
        far.update(ctx);
        CHECK(far.is_cut({0, 0}) && far.mesh_count() == 4);
        far.uncut({0, 0});
        CHECK(!far.is_cut({0, 0}));
        far.clear(ctx);
        CHECK(far.mesh_count() == 0);
    });
    throws([] { ponko::far_terrain(std::vector<float>(16, 0.0f), 4, {5, 0}, 10, 10, ponko::material{}); });
    throws([] { ponko::collision_mesh_data::heightfield(std::vector<float>(8, 0.0f), 4); });
    throws([] { ponko::collision_mesh_data::convex_hull({{0, 0, 0}, {1, 0, 0}, {0, 1, 0}}); });
    // A ball on terrain built on another thread; a far wall that comes in
    // once the focus walks to it.
    ponko::entity ball = 0, wall = 0, focus = 0, ground = 0;
    bool wall_at_start = true, wall_later = false;
    auto w = ponko::game("t", 1, 1)
        .physics3d({0, -9.81f, 0})
        .sim_system([&](ponko::ctx& ctx) {
            if (ctx.sim_tick() == 0) {
                wall_at_start = ctx.is_collider_active(wall);
                ctx.world().set(focus, ponko::transform3d_at({290, 0, 0}));
            } else if (ctx.sim_tick() == 1) {
                wall_later = ctx.is_collider_active(wall);
            }
        })
        .run_headless(120, [&](ponko::ctx& ctx) {
            ctx.stream_colliders(8);
            auto data = std::async(std::launch::async, [] {
                return ponko::collision_mesh_data::heightfield(std::vector<float>(9, 2.0f), 20);
            });
            auto mesh = ctx.add_collision_mesh(data.get());
            auto world = ctx.world();
            ground = world.spawn(ponko::transform3d_at({-10, 0, -10}), ponko::solid_terrain(mesh));
            ball = world.spawn(ponko::transform3d_at({0, 4, 0}), ponko::dynamic_body3d(), ponko::sphere_collider(0.5f));
            wall = world.spawn(ponko::transform3d_at({300, 0, 0}), ponko::fixed_body3d(), ponko::box_collider({1, 1, 1}));
            focus = world.spawn(ponko::transform3d_at({0, 0, 0}), ponko::focus_colliders(50, 80));
        });
    CHECK(std::fabs(w.get<ponko::transform3d>(ball).position.y - 2.5f) < 0.05f);
    CHECK(!wall_at_start && wall_later);
    CHECK(w.get<ponko::terrain_collider>(ground).friction == 0.5f);
    CHECK(w.get<ponko::collider_focus>(focus).keep_radius == 80);
}

static void lod_groups() {
    float fov = 60 * 3.14159265f / 180;
    CHECK(std::abs(ponko::lod_screen_height_at(2, 10, fov) - 0.1732f) < 1e-3f);
    CHECK(std::abs(ponko::lod_distance_at(2, 0.1732f, fov) - 10) < 0.01f);
    ponko::game("lod", 64, 64).run_headless(0, [](ponko::ctx& ctx) {
        auto white = ctx.default_material();
        auto high = ctx.create_mesh(ponko::mesh_data::cube(1), white);
        auto low = ctx.create_mesh(ponko::mesh_data::cube(2), white);
        auto e = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}));
        ponko::lod_level a{0.5f, 0, {ponko::lod_renderer_of(high)}};
        ponko::lod_level b{0.1f, 0, {ponko::lod_renderer_of(low)}, true, {}};
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
        throws([&] { ctx.lod_pick_of(ctx.world().spawn(ponko::transform3d_at({0, 0, 0}))); });
    });
}

static void skeletons() {
    ponko::game("skeletons", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        auto arm = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}));
        ponko::joint base{"base"}, tip{"tip", 0, ponko::transform3d_at({0, 1, 0})};
        ctx.set_skeleton(arm, {base, tip});
        CHECK(ctx.joint_count(arm) == 2);
        CHECK(ctx.find_joint(arm, "tip") == std::optional<std::size_t>(1));
        CHECK(!ctx.find_joint(arm, "tail"));
        // Bend the base a quarter turn: the tip swings from +Y to -X.
        auto pose = ctx.joint_pose(arm, 0);
        pose.rotation = ponko::quat_from_rotation_z(1.5707963f);
        ctx.set_joint_pose(arm, 0, pose);
        auto m = ctx.joint_matrix(arm, 1);
        CHECK(std::fabs(m[12] + 1) < 1e-5f && std::fabs(m[13]) < 1e-5f);
        ctx.reset_pose(arm);
        CHECK(std::fabs(ctx.joint_matrix(arm, 1)[13] - 1) < 1e-6f);
        throws([&] { ctx.joint_pose(arm, 2); });
        throws([&] { ctx.set_skeleton(arm, {ponko::joint{"loop", 0}}); });

        // A post: corners above the middle follow the tip.
        auto post = ponko::mesh_data::cube(0.2f);
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
        ctx.world().spawn(ponko::model_of(mesh));
        auto hat = ctx.world().spawn(ponko::model_of(mesh));
        ctx.set_skin_of(hat, arm);
        auto cup = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}));
        ctx.attach_to_joint(cup, arm, 1, ponko::transform3d_at({0, 0.1f, 0}));
        throws([&] { ctx.create_skinned_mesh(ponko::mesh_data::cube(1), material, arm); });
        joints.assign(joints.size(), {7, 0, 0, 0});
        post.set_skin(joints, weights);
        throws([&] { ctx.create_skinned_mesh(post, material, arm); });
        throws([&] { post.set_skin({{0, 0, 0, 0}}, {{1, 0, 0, 0}}); });
        throws([&] { ctx.create_skinned_mesh(post, material, cup); });
        throws([&] { ctx.load_clip("no/such.glb", "walk"); });
    });
}

static void morph_targets() {
    ponko::game("morph targets", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        // Two targets on a plane: one dents the whole sheet, one tips a corner.
        auto sheet = ponko::mesh_data::plane(1.0);
        ponko::morph_target dent{"dent", std::vector<ponko::vec3>(4, {0, 0, -0.7f}), {}};
        sheet.add_morph_target(dent);
        ponko::morph_target tip{"tip", std::vector<ponko::vec3>(4), {}}; // normals stay flat
        tip.positions[0] = {0, 0.4f, 0};
        sheet.add_morph_target(tip);
        sheet.add_morph_frame(0, 1, 0.5f);
        sheet.add_morph_frame(0, 0, 1.0f);
        throws([&] { sheet.add_morph_frame(0, 0, 1.0f); });
        throws([&] { sheet.add_morph_target(ponko::morph_target{"short", std::vector<ponko::vec3>(3), {}}); });

        auto mesh = ctx.create_mesh(sheet, ctx.default_material());
        CHECK(ctx.morph_target(mesh, "dent") == std::optional<std::size_t>(0));
        CHECK(ctx.morph_target(mesh, "tip") == std::optional<std::size_t>(1));
        CHECK(!ctx.morph_target(mesh, "sag"));

        auto e = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}), ponko::model_of(mesh));
        CHECK(!ctx.morph_weight(e, 0)); // no weights: its mesh's defaults show
        ctx.set_morph_weight(e, 0, 0.5f);
        ctx.set_morph_weight(e, 1, 1.0f);
        CHECK(ctx.morph_weight(e, 0) == std::optional(0.5f));
        CHECK(ctx.morph_weight(e, 1) == std::optional(1.0f));
        CHECK(ctx.morph_weight(e, 2) == std::optional(0.0f));
        ctx.set_skeleton(e, {});
        ponko::clip face("face");
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
    ponko::game("sound3d", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        float samples[64] = {};
        auto hum = ctx.create_sound(44100, samples, 64);
        auto music = ctx.audio_bus("music");
        CHECK(music.index == ctx.audio_bus("music").index);
        CHECK(music.index != ponko::audio_bus{}.index);
        ctx.set_audio_bus_volume(music, 0.5f);
        ctx.mute_audio_bus(music, true);
        auto settings = ponko::audio_settings3d_defaults();
        CHECK(settings.max_distance == 100.0f && settings.doppler == 1.0f);
        settings.rolloff = PONKO_ROLLOFF_LINEAR;
        settings.bus = music;
        auto e = ctx.world().spawn(ponko::transform3d_at({4, 0, 0}));
        ctx.set_audio_source3d(e, hum, settings);
        ctx.remove_audio_source3d(e);
        auto voice = ctx.play_sound_at(hum, {-4, 0, 0}, settings);
        ctx.set_sound_effects(voice, {800.0f, 0.3f});
        ctx.set_audio_listener(ponko::audio_listener3d{{7, 2, 3}, {0, 0, 0, 1}});
        CHECK(ctx.audio_listener().position.x == 7.0f);
        ctx.set_audio_listener(std::nullopt);
        CHECK(ctx.audio_listener().position.x != 7.0f);
        throws([&] { ctx.load_stream("missing.ogg"); });
    });
}

// Writes the two-joint skin with the clip "rise" to the temp folder.
static std::filesystem::path write_rise_gltf(const char* name) {
    const auto file = std::filesystem::temp_directory_path() / name;
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
    return file;
}

// A two-joint skin with one clip, "rise": the arm goes up 2 over 1 second.
static void animators() {
    const auto file = write_rise_gltf("ponko-cpp-rise.gltf");
    const std::string path = file.string();
    ponko::game("animators", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        auto rise = ctx.load_clip(path.c_str(), "rise");
        CHECK(rise.duration() == 1.0f);
        ponko::animation_graph_builder b;
        auto speed = b.add_float();
        auto still = b.add_state(ponko::motion::of(rise), 0);
        auto up = b.add_state(ponko::motion::of(rise), 1, std::nullopt, false);
        b.add_event(up, 0.5f, "half");
        auto go = b.add_transition(still, up, 0);
        b.add_condition(go, PONKO_CONDITION_GREATER, speed, 0.5f);
        auto tree = ponko::motion::blend1d(speed);
        tree.add(0, ponko::motion::of(rise)).add(1, ponko::motion::of(rise), 2);
        b.add_state(std::move(tree));
        auto graph = b.build(still);
        throws([&] { ponko::animation_graph_builder bad; bad.build(3); });

        auto e = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}));
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
        throws_invalid([&] { ctx.animator_set_bool(e, speed, true); }, "not a bool"); // speed is a float
        throws_invalid([&] { ctx.animator_crossfade(e, 7, 0.1f); }, "no such animation state");
        throws_invalid([&] { ctx.animator_crossfade(e, still, -1.0f); }, "duration");
        ctx.animator_play(e, still);
        ctx.animator_update(e, 0.0f);
        CHECK(ctx.animator_state(e) == still);

        ctx.animator_set_float(e, speed, 0); // the base stays still
        ponko::animation_graph_builder lb;
        lb.add_state(ponko::motion::of(rise), 1, std::nullopt, false);
        auto overlay = lb.build(0);
        auto layer = ctx.animator_add_layer(e, overlay, ponko::layer_blend::override_pose, "arm");
        CHECK(layer == 0);
        ctx.animator_set_layer_weight(e, layer, 0.5f);
        ctx.animator_update(e, 0.5f); // layer: y = 1, base: y = 0
        CHECK(std::fabs(ctx.joint_pose(e, arm).position.y - 0.5f) < 1e-4f);
        throws([&] { ctx.animator_set_layer_weight(e, 1, 1); });
        throws([&] { ctx.animator_add_layer(e, overlay, ponko::layer_blend::additive, "tail"); });
        ctx.animator_set_root_motion(e, ponko::root_motion::extract, arm);
        ctx.animator_update(e, 0.1f);
        auto [moved, turned] = ctx.animator_root_motion(e);
        CHECK(moved.x == 0 && moved.z == 0 && turned.w == 1); // rising isn't horizontal
    });
    std::filesystem::remove(file);
}

// A ball on a 1 m ball joint below a fixed anchor swings but stays 1 m away.
static void joints3d() {
    ponko::entity ball = 0, joint = 0;
    std::size_t joints_seen = 0;
    float reach = 0, lowest = 5;
    ponko::game("joints", 1, 1)
        .physics3d({0, -9.81f, 0})
        .sim_system([&](ponko::ctx& ctx) {
            if (ctx.sim_tick() == 10) joints_seen = ctx.joint_count();
            if (ctx.sim_tick() < 60) lowest = std::min(lowest, ctx.world().get<ponko::transform3d>(ball).position.y);
            if (ctx.sim_tick() == 60) {
                auto p = ctx.world().get<ponko::transform3d>(ball).position;
                reach = std::sqrt(p.x * p.x + (p.y - 5) * (p.y - 5) + p.z * p.z);
                ctx.world().despawn(joint);
            }
        }, "probe")
        .run_headless(70, [&](ponko::ctx& ctx) {
            auto world = ctx.world();
            auto anchor = world.spawn(ponko::transform3d_at({0, 5, 0}), ponko::fixed_body3d(), ponko::sphere_collider(0.1f));
            ball = world.spawn(ponko::transform3d_at({1, 5, 0}), ponko::with_dominance(ponko::dynamic_body3d(), 3),
                               ponko::sphere_collider(0.2f));
            auto j = ponko::spherical_joint3d(anchor, ball);
            j.frame_b = ponko::transform3d_at({-1, 0, 0}); // the anchor, seen from the ball
            joint = world.spawn(j);
            CHECK(world.get<ponko::joint3d>(joint).axes[ponko::joint_axis::linear_x].kind == PONKO_JOINT_LOCKED);
            CHECK(world.get<ponko::rigid_body3d>(ball).dominance == 3);
            auto bad = ponko::with_limits(ponko::revolute_joint3d(anchor, ball), ponko::joint_axis::angular_x, 1, -1);
            throws([&] { world.spawn(bad); });
            auto motor = ponko::with_drive(ponko::revolute_joint3d(anchor, ball), ponko::joint_axis::angular_x, ponko::velocity_drive(2, 10));
            CHECK(motor.drives[ponko::joint_axis::angular_x].enabled && motor.drives[3].target_velocity == 2);
        });
    CHECK(joints_seen == 1);
    CHECK(std::fabs(reach - 1) < 0.05f);
    CHECK(lowest < 4.1f); // it swung down to about 1 m below the anchor
}

// A leg of two 1 m bones bends so its foot lands on a reachable target.
static void inverse_kinematics() {
    auto g = ponko::game("ik", 1, 1);
    g.late_system([](ponko::ctx&) {}, "after animation");
    g.run_headless(0, [&](ponko::ctx& ctx) {
        auto leg = ctx.world().spawn(ponko::transform3d_at({5, 0, 0}));
        ctx.set_skeleton(leg, {ponko::joint{"hip"}, ponko::joint{"knee", 0, ponko::transform3d_at({0, -1, 0})},
                               ponko::joint{"foot", 1, ponko::transform3d_at({0, -1, 0})}});
        ctx.two_bone_ik(leg, 0, 1, 2, {5.5f, -1.5f, 0}, {0, 0, 1});
        auto foot = ctx.joint_matrix(leg, 2); // model space: the leg stands at x = 5
        CHECK(std::fabs(foot[12] - 0.5f) < 1e-3f && std::fabs(foot[13] + 1.5f) < 1e-3f);
        // The hip's -Y axis (down the leg) aims at a point in front.
        ctx.reset_pose(leg);
        ctx.look_at_joint(leg, 0, {0, -1, 0}, {5, 0, 3});
        auto knee = ctx.joint_matrix(leg, 1);
        CHECK(std::fabs(knee[14] - 1) < 1e-3f && std::fabs(knee[13]) < 1e-3f);
        ctx.draw_skeleton(leg, ponko::colors::white);
        throws([&] { ctx.two_bone_ik(leg, 0, 1, 9, {0, 0, 0}, {0, 0, 1}); });
    });
}

// A stick figure ragdolled 1 m up falls and is posed by its bodies; stopping removes them.
static void ragdolls() {
    ponko::entity figure = 0;
    float hips_y = 0, rest_y = 0;
    ponko::game("ragdolls", 1, 1)
        .physics3d({0, -9.81f, 0})
        .sim_system([&](ponko::ctx& ctx) {
            if (ctx.sim_tick() == 60) {
                hips_y = ctx.joint_pose(figure, 0).position.y;
                ctx.stop_ragdoll(figure, 0.5f);
                CHECK(ctx.ragdolled(figure) && ctx.ragdoll_blending(figure));
                CHECK(!ctx.ragdoll_body(figure, 0));
            }
        }, "probe")
        .run_headless(62, [&](ponko::ctx& ctx) {
            auto world = ctx.world();
            world.spawn(ponko::transform3d_at({0, -0.5f, 0}), ponko::fixed_body3d(), ponko::box_collider({20, 1, 20}));
            figure = world.spawn(ponko::transform3d_at({0, 1, 0}));
            ctx.set_skeleton(figure, {ponko::joint{"hips"}, ponko::joint{"spine", 0, ponko::transform3d_at({0, 0.6f, 0})},
                                      ponko::joint{"head", 1, ponko::transform3d_at({0, 0.4f, 0})}});
            rest_y = ctx.joint_pose(figure, 0).position.y;
            std::size_t before = world.size();
            ctx.start_ragdoll(figure, {ponko::ragdoll_bone("hips", 0.12f), ponko::ragdoll_bone("spine", 0.1f).swing(0.5f).twist(0.2f)});
            CHECK(world.size() == before + 3); // two bodies and a joint
            auto spine = ctx.find_joint(figure, "spine");
            auto body = ctx.ragdoll_body(figure, *spine);
            CHECK(body && ctx.apply_impulse3d(*body, {1, 0, 0}));
            CHECK(!ctx.ragdoll_body(figure, 2)); // the head has no body of its own
            CHECK(ctx.ragdolled(figure) && !ctx.ragdoll_blending(figure));
            throws([&] { ctx.start_ragdoll(figure, {ponko::ragdoll_bone("tail", 0.1f)}); });
        });
    CHECK(hips_y < rest_y - 0.3f); // it fell
}

// One-way platforms pass things up through, belts carry them, and a hook can ignore a pair.
static void contacts3d() {
    auto floor = [](ponko::world_view world, ponko::vec3 at, ponko::contact_modifier3d m) {
        return world.spawn(ponko::transform3d_at(at), ponko::fixed_body3d(), ponko::box_collider({20, 0.4f, 20}), m);
    };
    auto ball = [](ponko::world_view world, ponko::vec3 at, ponko::vec3 linear) {
        return world.spawn(ponko::transform3d_at(at), ponko::dynamic_body3d(), ponko::sphere_collider(0.25f),
                           ponko::velocity3d{linear, {0, 0, 0}});
    };
    ponko::entity from_below = 0, blocked = 0, carried = 0, ghost = 0, standing = 0;
    std::size_t hook_calls = 0;
    bool pair_ok = true;
    auto world = ponko::game("contacts", 1, 1).physics3d({0, -9.81f, 0}).run_headless(150, [&](ponko::ctx& ctx) {
        auto w = ctx.world();
        // A one-way platform: a ball thrown up from below comes up through it.
        auto up = ponko::one_way_modifier3d({0, 1, 0});
        CHECK(up.has_one_way && up.one_way_angle == 1.2f);
        floor(w, {0, 0, 0}, up);
        from_below = ball(w, {0, -1, 0}, {0, 8, 0});
        // An ordinary one blocks the same throw.
        floor(w, {30, 0, 0}, ponko::default_contact_modifier3d());
        blocked = ball(w, {30, -1, 0}, {0, 8, 0});
        // A belt carrying things along +X at 2 m/s.
        auto belt = ponko::conveyor_modifier3d({2, 0, 0});
        CHECK(belt.surface_velocity.x == 2);
        floor(w, {60, 0, 0}, belt);
        carried = ball(w, {60, 0.5f, 0}, {0, 0, 0});
        // A hook: only `ghost` is let through the hooked floor.
        auto hooked = floor(w, {90, 0, 0}, ponko::hook_modifier3d());
        ghost = ball(w, {90, 1, 0}, {0, 0, 0});
        standing = ball(w, {93, 1, 0}, {0, 0, 0});
        ctx.set_contact_hook3d([&, hooked](ponko::contact_pair3d& pair) {
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
    CHECK(std::abs(world.get<ponko::transform3d>(from_below).position.y - rest) < 0.05f);
    CHECK(world.get<ponko::transform3d>(blocked).position.y < -0.3f);
    CHECK(world.get<ponko::transform3d>(carried).position.x > 61);
    CHECK(hook_calls > 0 && pair_ok);
    CHECK(world.get<ponko::transform3d>(ghost).position.y < -1);
    CHECK(world.get<ponko::transform3d>(standing).position.y > 0.3f);

    // A cleared hook leaves the hooked floor ordinary.
    std::size_t calls_after_clear = 0;
    ponko::entity ball_on_floor = 0;
    auto cleared = ponko::game("contacts cleared", 1, 1).physics3d({0, -9.81f, 0}).run_headless(100, [&](ponko::ctx& ctx) {
        auto w = ctx.world();
        floor(w, {0, 0, 0}, ponko::hook_modifier3d());
        ball_on_floor = ball(w, {0, 1, 0}, {0, 0, 0});
        ctx.set_contact_hook3d([&](ponko::contact_pair3d& pair) { ++calls_after_clear; pair.ignore(); });
        ctx.clear_contact_hook3d();
    });
    CHECK(calls_after_clear == 0 && cleared.get<ponko::transform3d>(ball_on_floor).position.y > 0.3f);

    // An exception in the hook never unwinds into the engine: the run ends and rethrows it.
    bool caught = false;
    try {
        ponko::game("contacts throwing", 1, 1).physics3d({0, -9.81f, 0}).run_headless(100, [&](ponko::ctx& ctx) {
            auto w = ctx.world();
            floor(w, {0, 0, 0}, ponko::hook_modifier3d());
            ball(w, {0, 1, 0}, {0, 0, 0});
            ctx.set_contact_hook3d([](ponko::contact_pair3d&) { throw std::runtime_error("boom"); });
        });
    } catch (const std::runtime_error& e) { caught = std::string(e.what()) == "boom"; }
    CHECK(caught);

    // The modifier is an ordinary component: read it back, and it is no query column.
    ponko::world plain;
    auto m = ponko::one_way_modifier3d({0, 1, 0});
    m.hook = true;
    auto e = plain.spawn(m);
    CHECK(plain.get<ponko::contact_modifier3d>(e).hook);
    m.one_way_angle = -1;
    throws([&] { plain.set(e, m); });
}

// Two endpoints in memory: connect, then a reliable message and a broadcast.
static void messages() {
    throws([] { ponko::net_messages::steam_host(480, 0); });
    throws([] { ponko::net_messages::steam_join(480, 0); });
    throws([] { ponko::net_messages::steam_host(480, 2); }); // no Steam support/client in native test
    throws([] { ponko::steam_lobby::create(480, ponko::steam_lobby_visibility::private_lobby, 0); });
    throws([] { ponko::steam_lobby::create(480, ponko::steam_lobby_visibility::friends_only, 2); });
    throws([] { ponko::steam_lobby::join(480, 0); });
    auto peers = ponko::net_messages::loopback(2);
    auto& host = peers[0];
    auto& client = peers[1];
    std::vector<ponko::message_event> got;
    for (int i = 0; i < 3000 && got.size() < 4; i++) { // the handshake takes real time
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
        host.poll();
        client.poll();
        while (auto e = host.next_event()) got.push_back(*e);
        while (auto e = client.next_event()) {
            if (e->kind == PONKO_MESSAGE_CONNECTED) {
                client.send(0, PONKO_LANE_RELIABLE, std::string("buy apple"));
                host.send_to_all(PONKO_LANE_UNRELIABLE, std::string("tick"));
            }
            got.push_back(*e);
        }
    }
    auto has = [&](PonkoMessageEventKind kind, const std::string& text) {
        for (auto& e : got)
            if (e.kind == kind && std::string(e.bytes.begin(), e.bytes.end()) == text) return true;
        return false;
    };
    CHECK(has(PONKO_MESSAGE_CONNECTED, ""));
    CHECK(has(PONKO_MESSAGE, "buy apple"));
    CHECK(has(PONKO_MESSAGE, "tick"));
    CHECK(host.stats().packets_sent > 0);
    throws([&] { client.send(0, PONKO_LANE_RELIABLE, nullptr, 2 * 1024 * 1024); });
    throws([] { ponko::net_messages::loopback(1); });
}

static void navigation_carving() {
    ponko::entity box = 0;
    int tick = 0;
    bool cut = false, restored = false;
    ponko::game("carve", 1, 1).navmesh3d()
        .sim_system([&](ponko::ctx& ctx) {
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
        }).run_headless(45, [&](ponko::ctx& ctx) {
            ponko::nav_sources sources;
            sources.add_mesh(ponko::mesh_data::plane(20), ponko::transform3d_at({0, 0, 0}));
            auto settings = ponko::nav_bake_defaults();
            settings.carving = true;
            ctx.bake_navmesh(sources, settings);
            box = ctx.world().spawn();
            ctx.set_nav_carve_obstacle(box, {0, 1, 0}, {0.5f, 1, 2});
        });
    CHECK(cut && restored);
}

// A 20 m floor with a wall across the middle, open at one end: the agent walks around.
static void navigation() {
    ponko::entity walker = 0;
    std::size_t corners = 0;
    auto w = ponko::game("nav", 1, 1).navmesh3d().run_headless(600, [&](ponko::ctx& ctx) {
        ponko::nav_sources sources;
        sources.add_mesh(ponko::mesh_data::plane(20), ponko::transform3d_at({0, 0, 0}));
        auto wall = ponko::mesh_data::cube(1);
        auto t = ponko::transform3d_at({0, 1, -2});
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
        walker = ctx.world().spawn(ponko::transform3d_at({-5, 0, 0}));
        ctx.set_nav_agent(walker, 4);
        ctx.set_nav_avoidance(walker);
        throws([&] { ctx.set_nav_avoidance(walker, true, -1); });
        throws([&] { ctx.set_nav_avoidance(walker, true, 2, 0); });
        throws([&] { ctx.set_nav_avoidance(ctx.world().spawn()); });
        ctx.set_nav_destination(walker, ponko::vec3{5, 0, 0});
        throws([&] { ctx.set_nav_agent(walker, -1); });
        ponko::nav_sources empty;
        throws([&] { ctx.bake_navmesh(empty); });
    });
    auto end = w.get<ponko::transform3d>(walker).position;
    CHECK(corners >= 3);
    CHECK(std::fabs(end.x - 5) < 0.3f && std::fabs(end.z) < 0.3f);
}

static void particles3d() {
    ponko::game("particles", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        auto e = ctx.world().spawn(ponko::transform3d_at({0, 1, 0}));
        auto fire = ponko::particle_settings()
                        .rate(40)
                        .cone(0.3f)
                        .lifetime(0.5f, 1)
                        .size_over_life({{0, 1}, {1, 0}})
                        .color_over_life({{0, {1, 0.6f, 0.1f, 1}}, {1, {1, 0, 0, 0}}})
                        .blend(ponko::particle_blend::additive)
                        .max_particles(256);
        ctx.set_particle_emitter3d(e, fire);
        ctx.particle_burst(e, 20);
        // Particles simulate in the visual frame phase, which headless runs skip.
        CHECK(ctx.particle_live_count(e) == 0);
        throws([&] { ctx.set_particle_emitter3d(e, ponko::particle_settings().lifetime(0, 1)); });
        throws([&] { ctx.set_particle_emitter3d(e, ponko::particle_settings().lifetime(2, 1)); });
        throws([&] { ctx.set_particle_emitter3d(e, ponko::particle_settings().max_particles(0)); });
        ctx.remove_particle_emitter3d(e);
        throws([&] { ctx.particle_burst(e, 1); });
    });
}

// A 4 x 4 m floor facing up, as a glTF file.
static void ribbons_and_decals() {
    const auto file = std::filesystem::temp_directory_path() / "ponko-cpp-floor.gltf";
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
    ponko::game("ribbons", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        auto e = ctx.world().spawn(ponko::transform3d_at({0, 1, 0}));
        auto tracer = ponko::trail3d(0.25f, 0.05f)
                          .width_over_age({{0, 0.1f}, {1, 0}})
                          .color_over_age({{0, {1, 0.8f, 0.3f, 1}}, {1, {1, 0.3f, 0, 0}}})
                          .blend(ponko::particle_blend::additive)
                          .min_distance(0.1f)
                          .max_points(32);
        ctx.set_trail3d(e, tracer);
        // Trails record in the visual frame phase, which headless runs skip.
        CHECK(ctx.trail3d_point_count(e) == 0);
        ctx.set_trail3d_emitting(e, false);
        ctx.clear_trail3d(e);
        throws([&] { ctx.set_trail3d(e, ponko::trail3d(0, 0.1f)); });
        throws([&] { ctx.set_trail3d(e, ponko::trail3d(1, -0.1f)); });
        throws([&] { ctx.set_trail3d(e, ponko::trail3d(1, 0.1f).max_points(1)); });
        throws([&] { ctx.set_trail3d(e, ponko::trail3d(1, 0.1f).width_over_age({{0, -1}})); });
        ctx.remove_trail3d(e);
        throws([&] { ctx.trail3d_point_count(e); });
        throws([&] { ctx.clear_trail3d(e); });

        auto beam = ponko::line3d({{0, 0, 0}, {0, 0, -20}}, 0.02f, ponko::colors::red)
                        .blend(ponko::particle_blend::additive)
                        .texture_length(2);
        ctx.set_line3d(e, beam);
        ctx.set_line3d(e, ponko::line3d({{0, 0, 0}, {1, 0, 0}, {1, 1, 0}}, 0.1f, ponko::colors::white)); // replaces it
        throws([&] { ctx.set_line3d(e, ponko::line3d({{0, 0, 0}, {1, 0, 0}}, -1, ponko::colors::white)); });
        throws([&] { ctx.set_line3d(e, ponko::line3d({{0, 0, 0}, {1, NAN, 0}}, 0.1f, ponko::colors::white)); });
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
    ponko::game("ui panels", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        namespace u = ponko::ui;
        auto e = ctx.world().spawn(ponko::transform3d_at({0, 1.5f, -2}));
        auto texture = ctx.create_ui_panel(e, 600, 400, {1.2f, 0.8f});
        CHECK(texture.width == 600 && texture.height == 400);
        CHECK(ctx.world().has<ponko::model>(e));

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

// Layout presets, the theme, titles, names and take_click.
static void retained_ui_presets() {
    ponko::game("retained ui presets", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        namespace u = ponko::ui;
        auto ui = ctx.ui_retained();
        const ponko::vec2 window{800, 600};
        auto root = ui.add_panel(u::none, u::fill().padding(10).gap(5));
        auto title = ui.add_title(root, "Shop", u::row(40));
        auto bar = ui.add_panel(root, u::row(50).gap(10));
        auto side = ui.add_panel(bar, u::column(120));
        auto buy = ui.add_button(bar, "Buy", u::fixed(100, 30));
        auto map = ui.add_panel(root, u::anchored(0.75f, 0, 0.25f, 0.25f));
        ui.push_screen(root);
        auto dialog = ui.add_panel(u::none, u::centered(200, 100).top(40));
        ui.push_screen(dialog, true);
        ui.layout_for(window);
        CHECK(ui.rect(title).min.y == 10 && ui.rect(title).max.x == 790);
        CHECK(ui.rect(bar).min.y == 55 && ui.rect(side).max.x == 130 && ui.rect(side).max.y == 105);
        CHECK(ui.rect(buy).min.x == 140 && ui.rect(buy).max.x == 240);
        CHECK(ui.rect(map).min.x == 600 && ui.rect(map).max.y == 150);
        CHECK(ui.rect(dialog).min.x == 300 && ui.rect(dialog).min.y == 40);
        CHECK(ui.pop_screen() == dialog);

        // One theme for the tree and the immediate widgets.
        auto theme = ui.theme();
        CHECK(theme.title_size > theme.text_size && ui.resolved_paint(title).font_size == theme.title_size);
        theme.accent = {0, 1, 0, 1};
        theme.title = {1, 1, 0, 1};
        theme.title_size = 30;
        ui.set_theme(theme);
        CHECK(ui.theme().title_size == 30 && ui.resolved_paint(title).font_size == 30);
        CHECK(ui.resolved_paint(buy).border.g == 1 && ctx.ui_style().accent.g == 1);
        theme.border_width = -1;
        throws([&] { ui.set_theme(theme); });

        // Names instead of a list of ids.
        ui.set_name(buy, "shop.buy");
        CHECK(ui.find("shop.buy") == buy && ui.name(buy) == "shop.buy" && !ui.find("shop.sell"));
        ui.set_name(side, "shop.buy");  // moves the name
        CHECK(ui.find("shop.buy") == side && ui.name(buy).empty());

        // take_click: the last click's node, once.
        auto input = u::default_input();
        input.pointer = {190, 70};
        input.pressed = input.held = true;
        ui.frame(window, input);
        input.pressed = input.held = false;
        input.released = true;
        ui.frame(window, input);
        CHECK(ui.take_click() == buy && !ui.take_click());
        ui.remove(root);
        CHECK(!ui.find("shop.buy"));
    });
}

static void retained_ui() {
    ponko::game("retained ui", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        auto ui = ctx.ui_retained();
        const ponko::vec2 window{800, 600};
        namespace u = ponko::ui;
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
    ponko::game("text", 1, 1).run_headless(0, [](ponko::ctx& c) {
        auto e = c.world().spawn();
        CHECK(!c.text3d(e));
        ponko::text3d t("Hello\nworld");
        t.size = .5f; t.align = ponko::text_align3d::left;
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
        t.font.id = 0; t.align = static_cast<ponko::text_align3d>(99);
        throws([&] { c.set_text3d(e, t); });
        c.remove_text3d(e); CHECK(!c.text3d(e));
        c.world().despawn(e);
        throws([&] { (void)c.text3d(e); });
    });
}
// Replication helpers: sequence numbers, authority, snapshots and interest.
static void replication() {
    CHECK(ponko::seq_newer(0, 65535) && !ponko::seq_newer(5, 5) && ponko::seq_diff(1, 65535) == 2);
    CHECK(ponko::stamp_newer({1, 0}, {0, 60000}) && !ponko::stamp_newer({0, 7}, {0, 7}));

    ponko::authority car(0);
    CHECK(car.check(0, 0) == ponko::authority_check::accept);
    CHECK(car.handoff(1) == 1 && car.owner() == 1 && car.epoch() == 1);
    CHECK(car.check(0, 0) == ponko::authority_check::stale);
    CHECK(car.check(0, 1) == ponko::authority_check::wrong_owner);
    CHECK(car.check(1, 2) == ponko::authority_check::ahead);
    CHECK(!car.adopt(2, 1) && car.owner() == 1); // simultaneous handoff: the lower id wins
    CHECK(car.adopt(2, 2) && car.owner() == 2);

    ponko::latest_wins filter;
    CHECK(filter.accept(5, std::uint16_t(3)) && !filter.accept(5, std::uint16_t(2)) && !filter.accept(5, std::uint16_t(3)));
    CHECK(filter.accept(5, ponko::update_stamp{1, 0})); // a new epoch beats any sequence number
    CHECK(filter.size() == 1);
    filter.forget(5);
    CHECK(filter.accept(5, std::uint16_t(1)) && filter.size() == 1);
    auto moved = std::move(filter);
    moved.clear();
    CHECK(moved.size() == 0);

    ponko::snapshot_stream stream;
    CHECK(!stream.sample(0) && !stream.clock_offset());
    for (int n = 0; n < 20; n++) {
        auto p = ponko::moving(ponko::pose_at({float(n), 0, 0}), {20, 0, 0});
        CHECK(stream.push(5000 + n * 0.05, n * 0.05 + 0.03, p) == ponko::snapshot_push::added);
    }
    CHECK(stream.push(5000, 1.0, ponko::pose_at({0, 0, 0})) == ponko::snapshot_push::duplicate);
    auto shown = stream.sample(1.0);
    CHECK(shown && shown->state == ponko::snapshot_state::interpolated && std::fabs(shown->value.position.x - 17.4f) < 0.05f);
    CHECK(stream.sample(1.2)->state == ponko::snapshot_state::extrapolated);
    CHECK(stream.sample(5.0)->state == ponko::snapshot_state::held);
    CHECK(stream.size() == 20 && stream.clock_offset() && std::fabs(*stream.clock_offset() + 4999.97) < 0.01);
    stream.set_delay(0);
    throws([&] { stream.set_delay(-1); });
    stream.clear();
    CHECK(stream.size() == 0 && !stream.sample(1.0));
    auto config = ponko::snapshot_defaults();
    config.capacity = 1;
    throws([&] { ponko::snapshot_stream bad(config); });

    ponko::interest interest;
    std::vector<ponko::interest_viewer> viewers{{1, {0, 0, 0}}};
    std::vector<ponko::interest_entity> world{{10, {10, 0, 0}}, {20, {100, 0, 0}}, {30, {900, 0, 0}}};
    interest.update(0, viewers, world);
    auto entries = interest.entries(1);
    CHECK(entries.size() == 2 && entries[0].entity == 10 && entries[0].tier == 0 && entries[1].tier == 1 && entries[1].send);
    CHECK(interest.changes().size() == 2 && interest.changes()[0].kind == PONKO_INTEREST_ENTERED);
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
    auto bad = ponko::interest_defaults();
    bad.hysteresis = 99;
    throws([&] { ponko::interest invalid(bad); });
}

static void profiling() {
    auto work = [](ponko::ctx&) {
        PONKO_PROFILE_SCOPE("cpp_work");
        volatile float x = 0;
        for (int i = 0; i < 3000; ++i) x = x + std::sqrt(float(i));
    };
    auto leaky = [](ponko::ctx&) { int* p = new int[4]; *static_cast<volatile int*>(p) = 7; delete[] p; };
    auto run = ponko::game("p", 1, 1)
        .sim_system(work, "work")
        .sim_system(leaky, "leaky")
        .profile_headless(5, 40, [](ponko::ctx& ctx) { ctx.world().spawn(Pos{1, 2}); });
    CHECK(run.world.size() == 1);
    const std::string text = run.report.text();
    CHECK(text.find("40 frames") != std::string::npos && text.find("cpp_work") != std::string::npos);
    // C++ `new` is counted: the leaky system is named, the other isn't.
    CHECK(text.find("`leaky` allocates") != std::string::npos);
    CHECK(text.find("`work` allocates") == std::string::npos);
    CHECK(run.report.within(R"({"frame_p99_ms": 1000, "systems": {"work": 1000}})"));
    const std::string broken = run.report.check(R"({"allocs_per_frame": 0, "scopes": {"cpp_work": 0}})");
    CHECK(broken.find("allocations per frame") != std::string::npos && broken.find("cpp_work") != std::string::npos);
    CHECK(run.report.regressions(run.report).empty());
    throws([&] { (void)run.report.check("not json"); });
    const auto file = std::filesystem::temp_directory_path() / "ponko-cpp-report.json";
    run.report.save(file.string().c_str());
    CHECK(ponko::perf_report::load(file.string().c_str()).json() == run.report.json());
    std::filesystem::remove(file);
    // Scopes and profiling toggles work from a live ctx too.
    ponko::game("q", 1, 1).run_headless(0, [](ponko::ctx& ctx) {
        ctx.set_profiling(true);
        CHECK(ctx.perf_report().json().size() > 2);
        ctx.set_profiling(false);
        // Headless: no renderer, so nothing uploaded to the GPU.
        const ponko::gpu_uploads uploads = ctx.gpu_uploads();
        CHECK(uploads.bytes == 0 && uploads.buffers_created == 0 && uploads.reallocations == 0);
    });
}

// Animator parameters a layer reads (bool, trigger, float), a 2D blend tree,
// and the C++ face of the 3D joint kinds, queries and clock.
static void animator_layers_and_blend_trees() {
    const auto file = write_rise_gltf("ponko-cpp-layers.gltf");
    const std::string path = file.string();
    ponko::game("layers", 1, 1).run_headless(0, [&](ponko::ctx& ctx) {
        auto rise = ctx.load_clip(path.c_str(), "rise");
        auto e = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}));
        ctx.load_skeleton(e, path.c_str());
        auto arm = *ctx.find_joint(e, "arm");

        // The base animation holds still; a layer rises when told to.
        ponko::animation_graph_builder base;
        base.add_state(ponko::motion::of(rise), 0);
        ctx.set_animator(e, base.build(0));
        ctx.animator_set_manual(e, true);

        ponko::animation_graph_builder lb;
        auto flag = lb.add_bool();
        auto fire = lb.add_trigger();
        auto amount = lb.add_float();
        auto idle = lb.add_state(ponko::motion::of(rise), 0);
        auto run = lb.add_state(ponko::motion::of(rise), 1, std::nullopt, false);
        auto by_flag = lb.add_transition(idle, run, 0);
        lb.add_condition(by_flag, PONKO_CONDITION_BOOL, flag, 1);
        auto by_trigger = lb.add_transition(idle, run, 0);
        lb.add_condition(by_trigger, PONKO_CONDITION_TRIGGER, fire);
        auto by_float = lb.add_transition(idle, run, 0);
        lb.add_condition(by_float, PONKO_CONDITION_GREATER, amount, 0.5f);
        auto overlay = lb.build(idle);
        auto layer = ctx.animator_add_layer(e, overlay, ponko::layer_blend::override_pose);

        auto arm_y = [&] { return ctx.joint_pose(e, arm).position.y; };
        auto back_to_idle = [&] {
            ctx.animator_layer_crossfade(e, layer, idle, 0);
            ctx.animator_update(e, 0);
            CHECK(std::fabs(arm_y()) < 1e-4f);
        };
        ctx.animator_update(e, 0.5f);
        CHECK(std::fabs(arm_y()) < 1e-4f); // nothing asked the layer to move

        ctx.animator_layer_set_bool(e, layer, flag, true);
        ctx.animator_update(e, 0);
        ctx.animator_update(e, 0.5f);
        CHECK(std::fabs(arm_y() - 1.0f) < 1e-4f); // by the bool
        ctx.animator_layer_set_bool(e, layer, flag, false);
        back_to_idle();

        ctx.animator_layer_set_trigger(e, layer, fire);
        ctx.animator_update(e, 0);
        ctx.animator_update(e, 0.5f);
        CHECK(std::fabs(arm_y() - 1.0f) < 1e-4f); // by the trigger
        back_to_idle();

        ctx.animator_layer_set_float(e, layer, amount, 1.0f);
        ctx.animator_update(e, 0);
        ctx.animator_update(e, 0.5f);
        CHECK(std::fabs(arm_y() - 1.0f) < 1e-4f); // by the float
        throws([&] { ctx.animator_layer_set_float(e, layer + 1, amount, 1.0f); }); // no such layer
        throws_invalid([&] { ctx.animator_layer_set_bool(e, layer, amount, true); }, "not a bool"); // amount is a float

        // A base graph with a trigger: it fires once and is used up.
        ponko::animation_graph_builder tb;
        auto go = tb.add_trigger();
        auto first = tb.add_state(ponko::motion::of(rise), 0);
        auto second = tb.add_state(ponko::motion::of(rise), 0);
        auto t = tb.add_transition(first, second, 0);
        tb.add_condition(t, PONKO_CONDITION_TRIGGER, go);
        auto second_entity = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}));
        ctx.load_skeleton(second_entity, path.c_str());
        ctx.set_animator(second_entity, tb.build(first));
        ctx.animator_set_manual(second_entity, true);
        ctx.animator_update(second_entity, 0.1f);
        CHECK(ctx.animator_state(second_entity) == first);
        ctx.animator_set_trigger(second_entity, go);
        ctx.animator_update(second_entity, 0);
        CHECK(ctx.animator_state(second_entity) == second);

        // A 2D blend tree of the same clip at three points plays the clip
        // wherever the two parameters put the blend.
        ponko::animation_graph_builder bb;
        auto x = bb.add_float();
        auto y = bb.add_float();
        auto tree = ponko::motion::blend2d(x, y);
        tree.add(0, 0, ponko::motion::of(rise)).add(1, 0, ponko::motion::of(rise)).add(0, 1, ponko::motion::of(rise), 1);
        bb.add_state(std::move(tree), 1);
        auto third = ctx.world().spawn(ponko::transform3d_at({0, 0, 0}));
        ctx.load_skeleton(third, path.c_str());
        ctx.set_animator(third, bb.build(0));
        ctx.animator_set_manual(third, true);
        ctx.animator_set_float(third, x, 0.3f);
        ctx.animator_set_float(third, y, 0.6f);
        ctx.animator_update(third, 0.5f);
        CHECK(std::fabs(ctx.joint_pose(third, arm).position.y - 1.0f) < 1e-4f);
    });
    std::filesystem::remove(file);
}

// Joint kinds, 3D queries, 3D body kinds, gravity, the clock, idle input and
// the headless camera helpers, through the C++ wrapper.
static void headless_wrapper_calls() {
    // Joints: which axes each kind leaves free.
    {
        auto count = [](const ponko::joint3d& j, PonkoJointMotionKind kind) {
            int n = 0;
            for (auto& axis : j.axes) n += axis.kind == kind;
            return n;
        };
        CHECK(count(ponko::fixed_joint3d(1, 2), PONKO_JOINT_LOCKED) == 6);
        CHECK(count(ponko::generic_joint3d(1, 2), PONKO_JOINT_FREE) == 6);
        auto slider = ponko::prismatic_joint3d(1, 2);
        CHECK(slider.axes[ponko::joint_axis::linear_x].kind == PONKO_JOINT_FREE && count(slider, PONKO_JOINT_FREE) == 1);
        CHECK(ponko::kinematic_body3d().kind == PONKO_BODY_KINEMATIC);
        CHECK(ponko::kinematic_body().kind == PONKO_BODY_KINEMATIC);
    }

    ponko::entity wall = 0, ball = 0, plain = 0;
    bool asked = false;
    ponko::game("queries", 1, 1)
        .physics3d({0, 0, 0})
        .physics({0, 0})
        .spatial_grid(16)
        .sim_system([&](ponko::ctx& ctx) {
            if (ctx.sim_tick() != 1) return;
            asked = true;
            // A ray along +Z meets the wall (4 to 6 m away) and then the ball.
            std::vector<ponko::hit3d> hits;
            ctx.cast_ray3d_all({0, 0, 0}, {0, 0, 1}, 100, hits);
            CHECK(hits.size() == 2);
            CHECK(ponko_entity_bits(hits[0].entity) == wall);
            CHECK(std::fabs(hits[0].distance - 4.0f) < 1e-3f);
            CHECK(ponko_entity_bits(hits[1].entity) == ball);
            CHECK(hits[1].distance > hits[0].distance);
            // A ball swept along +Z touches the wall half its radius early.
            auto swept = ctx.cast_sphere3d({0, 0, 0}, 0.5f, {0, 0, 1}, 100);
            CHECK(swept && std::fabs(swept->distance - 3.5f) < 1e-3f);
            CHECK(!ctx.cast_sphere3d({0, 0, 0}, 0.5f, {0, 1, 0}, 100));
            auto capsule = ctx.cast_capsule3d({0, 0, 0}, 2, 0.5f, {0, 0, 1}, 100);
            CHECK(capsule && std::fabs(capsule->distance - 3.5f) < 1e-3f);
            // Overlaps: a box around the wall finds it and not the ball.
            std::vector<PonkoEntity> found;
            ctx.overlap_box3d({0, 0, 5}, {4, 4, 4}, {0, 0, 0, 1}, found);
            CHECK(found.size() == 1 && ponko_entity_bits(found[0]) == wall);
            // Masses: the ball has one, a plain entity is no body.
            CHECK(ctx.mass3d(ball) && *ctx.mass3d(ball) > 0);
            CHECK(!ctx.mass3d(plain));
            // The built-in grid sees the 2D transforms, and nothing outside the rectangle.
            std::vector<ponko::grid_hit> cells;
            ctx.spatial_rect({-1, -1}, {1, 1}, cells);
            CHECK(cells.empty());

            // The clock, idle input and cameras of a headless run.
            ctx.set_sim_speed(0.5f);
            CHECK(ctx.sim_speed() == 0.5f);
            throws([&] { ctx.set_sim_speed(-1); });
            CHECK(!ctx.key_released(PONKO_KEY_A) && !ctx.mouse_released(PONKO_MOUSE_LEFT));
            CHECK(ctx.mouse_pos().x == 0 && ctx.mouse_pos().y == 0);
            CHECK(ctx.mouse_wheel().y == 0 && ctx.mouse_motion().x == 0);
            CHECK(!ctx.cursor_locked());
            auto [origin, direction] = ctx.screen_ray(ctx.window_size() * 0.5f);
            auto camera = ctx.camera3d();
            CHECK(origin.x == camera.position.x && origin.y == camera.position.y && origin.z == camera.position.z);
            CHECK(std::fabs(std::sqrt(direction.x * direction.x + direction.y * direction.y + direction.z * direction.z) - 1.0f) < 1e-4f);
            CHECK(ctx.models_drawn() == 0);
            ctx.set_gravity({0, 10});
            ctx.set_gravity3d({0, -1, 0});
            throws([&] { ctx.set_gravity({NAN, 0}); });
            ctx.warm_render_pipelines(); // nothing to build without a renderer
        })
        .run_headless(3, [&](ponko::ctx& ctx) {
            auto world = ctx.world();
            wall = world.spawn(ponko::transform3d_at({0, 0, 5}), ponko::fixed_body3d(), ponko::box_collider({2, 2, 2}));
            ball = world.spawn(ponko::transform3d_at({0, 0, 10}), ponko::dynamic_body3d(), ponko::sphere_collider(0.5f));
            plain = world.spawn(ponko::transform3d_at({30, 0, 0}));
            auto pole = world.spawn(ponko::transform3d_at({50, 0, 0}), ponko::kinematic_body3d(), ponko::capsule_collider3d(1, 0.25f));
            CHECK(world.get<ponko::rigid_body3d>(pole).kind == PONKO_BODY_KINEMATIC);
        });
    CHECK(asked);
}

// Shader, effect and compute files load once, and the same file gives the same handle.
// Hot reload is off for the files that must fail: with it on (the default in
// debug builds of the engine) a file that doesn't compile loads as a stand-in.
static void shader_files() {
    const auto dir = std::filesystem::temp_directory_path();
    const auto surface = dir / "ponko-cpp-wind.wgsl", effect = dir / "ponko-cpp-grey.wgsl", compute = dir / "ponko-cpp-fade.wgsl";
    std::ofstream(surface) << "fn vertex(v: Vertex) -> Vertex { var o = v; o.position.x += material.custom[0].x; return o; }";
    std::ofstream(effect) << "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(dot(p.color, vec3<f32>(0.3, 0.6, 0.1))); }";
    std::ofstream(compute) << "fn compute(c: Cell) -> vec4<f32> { return c.value * 0.9; }";
    ponko::game("shaders", 1, 1).hot_reload(false).run_headless(0, [&](ponko::ctx& ctx) {
        auto a = ctx.load_shader(surface.string().c_str());
        CHECK(a.id == ctx.load_shader(surface.string().c_str()).id);
        auto e = ctx.load_effect(effect.string().c_str());
        CHECK(e.id == ctx.load_effect(effect.string().c_str()).id);
        auto c = ctx.load_compute(compute.string().c_str());
        CHECK(c.id == ctx.load_compute(compute.string().c_str()).id);
        throws([&] { ctx.load_shader("no-such-shader.wgsl"); });
        throws([&] { ctx.load_effect(surface.string().c_str()); }); // a surface shader is no effect
        throws([&] { ctx.load_compute("no-such-compute.wgsl"); });
    });
    // With hot reload on, the file that is no effect still gets a handle: a
    // stand-in, reported as failed and watched. A missing file is still an error.
    ponko::game("stand-ins", 1, 1).hot_reload(true).run_headless(0, [&](ponko::ctx& ctx) {
        (void)ctx.load_effect(surface.string().c_str());
        auto shaders = ctx.shader_status();
        CHECK(shaders.size() == 1 && !shaders[0].ok());
        CHECK(shaders[0].info.kind == PONKO_SHADER_EFFECT && shaders[0].info.outcome == PONKO_SHADER_FAILED);
        CHECK(!shaders[0].errors.empty());
        throws([&] { ctx.load_effect("no-such-effect.wgsl"); });
    });
    for (auto& f : {surface, effect, compute}) std::filesystem::remove(f);
}

static void declared_access() {
    // A system declares what it reads and writes; debug builds of the
    // library stop the game if it uses anything else (tests/system_access.rs).
    struct Score { int value; };
    auto mover = ponko::access().write<Pos>().read<Vel>().read_service(ponko::service::time);
    auto scorer = ponko::access().write_resource<Score>().read_events<Died>();
    auto killer = ponko::access().read<Hp>().send_events<Died>().commands();
    CHECK(mover.conflicts_before(scorer) == 0);
    // The scorer would see the killer's events and applied commands.
    CHECK(killer.conflicts_before(scorer) == 2);
    CHECK(scorer.conflicts_before(killer) == 0);
    CHECK(scorer.conflicts_before(killer, false) == 2);
    ponko::access copy = killer;
    CHECK(copy.conflicts_before(scorer) == 2);
    copy = mover;
    CHECK(copy.conflicts_before(mover) == 1);
    int scored = 0;
    auto g = ponko::game("t", 1, 1);
    g.add(ponko::phase::after_physics, mover, [](ponko::ctx& ctx) {
         ctx.world().each([](Pos& p, const Vel& v) { p.x += v.x; });
     }, "mover")
     .add(ponko::phase::after_physics, killer, [](ponko::ctx& ctx) {
         ctx.world().each([&](ponko::entity e, const Hp& hp) {
             if (hp.value <= 0) { ctx.send(Died{e, ctx.sim_tick()}); ctx.despawn_later(e); }
         });
     }, "killer")
     .add(ponko::phase::after_physics, scorer, [&](ponko::ctx& ctx) {
         scored += static_cast<int>(ctx.read<Died>().size());
     }, "scorer");
    const std::string report = g.access_report();
    CHECK(report.find("mover + killer: could run together") == std::string::npos);
    CHECK(report.find("mover + killer: no conflict, but a `Ctx` system stays sequential") != std::string::npos);
    CHECK(report.find("killer + scorer: conflict on events") != std::string::npos);
    auto w = g.run_headless(2, [](ponko::ctx& ctx) {
        ctx.world().set_resource(Score{0});
        ctx.world().spawn(Pos{0, 0}, Vel{1, 0});
        ctx.world().spawn(Hp{0});
    });
    CHECK(scored == 1);
    CHECK(w.size() == 1);
}

int main() {
    declared_access();
    worlds();
    names_and_reflection();
    resources();
    data_files();
    described_schemas_reach_capabilities();
    scenes_and_prefabs();
    world_text();
    three_d();
    skeletons();
    lod_groups();
    morph_targets();
    sound3d();
    animators();
    animator_layers_and_blend_trees();
    headless_wrapper_calls();
    shader_files();
    inverse_kinematics();
    streaming();
    streamed_worlds();
    model_material_overrides();
    handle_lifecycle();
    asset_streaming();
    parallel_queries();
    change_detection();
    games();
    attachments();
    physics();
    joints3d();
    vehicles();
    phases();
    plugins_setups_and_system_ids();
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
    retained_ui_presets();
    ui_panels();
    events();
    profiling();
    rng_matches_rust();
    grid_views_match_engine_queries();
    std::cout << "C++ wrapper tests passed\n";
}
