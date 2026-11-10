#include "ponko.hpp"
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <vector>
#define CHECK(x) do { if (!(x)) std::abort(); } while (false)
int main() {
    ponko::entity solid2 = 0, solid3 = 0;
    bool queried = false;
    auto world = ponko::game("dimensions", 1, 1).physics({0, 0}).physics3d({0, 0, 0})
        .sim_system([&](ponko::ctx& ctx) {
            queried = true;
            auto hit2 = ctx.cast_ray_filtered(ponko::vec2{0, 0}, {1, 0}, 100, ponko::filter2d{});
            auto hit3 = ctx.cast_ray_filtered(ponko::vec3{0, 0, 0}, {1, 0, 0}, 100, ponko::filter3d{});
            CHECK(hit2 && ponko_entity_bits(hit2->entity) == solid2 && std::fabs(hit2->distance - 9) < .001f);
            CHECK(hit3 && ponko_entity_bits(hit3->entity) == solid3 && std::fabs(hit3->distance - 9) < .001f);
            CHECK(!ctx.cast_ray_filtered(ponko::vec2{0, 0}, {1, 0}, 100, ponko::filter2d{}.exclude(solid2)));
            CHECK(!ctx.cast_ray_filtered(ponko::vec3{0, 0, 0}, {1, 0, 0}, 100, ponko::filter3d{}.exclude(solid3)));
            auto sensor2 = ctx.cast_ray_filtered(ponko::vec2{0, 0}, {1, 0}, 100, ponko::filter2d{}.with_sensors());
            CHECK(sensor2 && std::fabs(sensor2->distance - 4) < .001f);
            std::vector<PonkoEntity> hits;
            ctx.overlap_ball(ponko::vec2{5, 0}, 1, ponko::filter2d{}, hits); CHECK(hits.empty());
            ctx.overlap_ball(ponko::vec2{5, 0}, 1, ponko::filter2d{}.with_sensors(), hits); CHECK(hits.size() == 1);
            ctx.overlap_ball(ponko::vec3{5, 0, 0}, 1, ponko::filter3d{}, hits); CHECK(hits.empty());
            ctx.overlap_ball(ponko::vec3{5, 0, 0}, 1, ponko::filter3d{}.with_sensors(), hits); CHECK(hits.size() == 1);
            CHECK(ctx.cast_ray({0, 0}, {1, 0}, 100)); // old 2D spelling still includes sensors
        }).run_headless(1, [&](ponko::ctx& ctx) {
            auto w = ctx.world();
            solid2 = w.spawn(ponko::transform_at({10, 0}), ponko::fixed_body(), ponko::circle_collider(1));
            solid3 = w.spawn(ponko::transform3d_at({10, 0, 0}), ponko::fixed_body3d(), ponko::sphere_collider(1));
            auto c2 = ponko::circle_collider(1); c2.sensor = true;
            auto c3 = ponko::sphere_collider(1); c3.sensor = true;
            w.spawn(ponko::transform_at({5, 0}), ponko::fixed_body(), c2);
            w.spawn(ponko::transform3d_at({5, 0, 0}), ponko::fixed_body3d(), c3);
            auto t = ponko::transform3d_at({0, 0, 0}); t.scale = {2, 3, 4};
            const auto original = w.spawn(t);
            ctx.save_prefab(original, "test-dimensions-cpp3d.json");
            auto e = ctx.spawn_prefab3d("test-dimensions-cpp3d.json", {7, 8, 9});
            CHECK(w.get<ponko::transform3d>(e).position.y == 8 && w.get<ponko::transform3d>(e).scale.z == 4);
            t.position.x = 11; t.scale.x = 6;
            e = ctx.spawn_prefab_transform3d("test-dimensions-cpp3d.json", t);
            CHECK(w.get<ponko::transform3d>(e).position.x == 11 && w.get<ponko::transform3d>(e).scale.x == 6);
            auto t2 = ponko::transform_at({1, 2}); t2.rotation = .25f;
            const auto original2 = w.spawn(t2);
            ctx.save_prefab(original2, "test-dimensions-cpp2d.json");
            // The old brace-init spelling remains unambiguous.
            e = ctx.spawn_prefab("test-dimensions-cpp2d.json", {3, 4});
            CHECK(w.get<ponko::transform2d>(e).rotation == .25f);
            t2.rotation = .75f; t2.scale = {3, 4};
            e = ctx.spawn_prefab_transform2d("test-dimensions-cpp2d.json", t2);
            CHECK(w.get<ponko::transform2d>(e).rotation == .75f && w.get<ponko::transform2d>(e).scale.y == 4);
        });
    CHECK(queried && world.size() > 4);
    std::remove("assets/test-dimensions-cpp2d.json"); std::remove("assets/test-dimensions-cpp3d.json");
}
