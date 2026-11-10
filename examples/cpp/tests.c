#include "ponko.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr, "failed: %s\n", #x); abort(); } } while (0)
typedef struct { int x; } Position;
typedef struct { PonkoWorld* world; size_t seen; uint64_t entity; } Context;
static void visit(const PonkoChunk* chunk, void* user) {
    Context* c = user;
    CHECK(chunk->column_count == 1);
    for (size_t i = 0; i < chunk->count; ++i) {
        CHECK(ponko_entity_bits(chunk->entities[i]) == c->entity);
        ((Position*)chunk->columns[0])[i].x += 2;
        ++c->seen;
    }
    size_t len;
    CHECK(ponko_world_len(c->world, &len) == PONKO_BUSY);
    CHECK(ponko_world_free(c->world) == PONKO_BUSY);
}
typedef struct { size_t ticks; float dt; PonkoStatus busy_quit; } Run;
static void setup(PonkoCtx* ctx, void* user) {
    (void)user;
    PonkoGpuBuffer buffer;
    const float items[2][4] = {{1, 2, 3, 4}, {0, 0, 0, 0}};
    CHECK(ponko_ctx_create_gpu_buffer(ctx, 2, items, 2, &buffer) == PONKO_OK);
    CHECK(ponko_ctx_write_gpu_buffer(ctx, buffer, 1, items, 1) == PONKO_OK);
    CHECK(ponko_ctx_write_gpu_buffer(ctx, buffer, 2, items, 1) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_create_gpu_buffer(ctx, 0, NULL, 0, &buffer) == PONKO_INVALID_ARGUMENT);
    PonkoMaterial batch_material;
    PonkoMaterialDesc batch_desc = ponko_material_desc((PonkoColor){1,1,1,1});
    CHECK(ponko_ctx_create_material(ctx, &batch_desc, &batch_material) == PONKO_OK);
    PonkoMeshData* batch_data = ponko_mesh_data_cube(1.0f);
    PonkoMesh batch_mesh;
    CHECK(ponko_ctx_create_mesh(ctx, batch_data, batch_material, &batch_mesh) == PONKO_OK);
    ponko_mesh_data_free(batch_data);
    const float matrices[1][16] = {{1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1}};
    PonkoInstanceBatch batch;
    CHECK(ponko_ctx_create_instance_batch(ctx,batch_mesh,matrices,1,(PonkoInstanceBatchOptions){32,100,true,{1,1,1,1}},&batch) == PONKO_OK);
    CHECK(ponko_ctx_update_instance_batch(ctx,batch,matrices,1) == PONKO_OK);
    CHECK(ponko_ctx_free_instance_batch(ctx,batch) == PONKO_OK);
    CHECK(ponko_ctx_update_instance_batch(ctx,batch,matrices,1) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_free_gpu_buffer(ctx, buffer) == PONKO_OK);
    CHECK(ponko_ctx_free_gpu_buffer(ctx, buffer) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_write_gpu_buffer(ctx, buffer, 0, items, 1) == PONKO_INVALID_ARGUMENT);
    /* One lifecycle for every handle kind: a kind and an id. */
    bool live = true;
    CHECK(ponko_ctx_handle_is_live(ctx, PONKO_HANDLE_GPU_BUFFER, buffer.id, &live) == PONKO_OK && !live);
    CHECK(ponko_ctx_handle_is_live(ctx, PONKO_HANDLE_MESH, batch_mesh.id, &live) == PONKO_OK && live);
    PonkoLiveCounts counts;
    CHECK(ponko_ctx_live_counts(ctx, &counts) == PONKO_OK && counts.gpu_buffers == 0);
    size_t meshes = counts.meshes;
    CHECK(ponko_ctx_handle_free(ctx, PONKO_HANDLE_MESH, batch_mesh.id) == PONKO_OK);
    CHECK(ponko_ctx_handle_free(ctx, PONKO_HANDLE_MESH, batch_mesh.id) == PONKO_OK);
    CHECK(ponko_ctx_handle_is_live(ctx, PONKO_HANDLE_MESH, batch_mesh.id, &live) == PONKO_OK && !live);
    CHECK(ponko_ctx_live_counts(ctx, &counts) == PONKO_OK && counts.meshes == meshes - 1);
    CHECK(ponko_ctx_handle_is_live(ctx, PONKO_HANDLE_TEXTURE, PONKO_HANDLE_NONE, &live) == PONKO_OK && !live);
    CHECK(ponko_ctx_handle_free(ctx, PONKO_HANDLE_MATERIAL, PONKO_HANDLE_NONE) == PONKO_OK);
    CHECK(ponko_ctx_handle_free(ctx, 99, 0) == PONKO_INVALID_ARGUMENT);
    bool culling = true;
    CHECK(ponko_ctx_occlusion_culling(ctx, &culling) == PONKO_OK && !culling);
    CHECK(ponko_ctx_set_occlusion_culling(ctx, true) == PONKO_OK);
    CHECK(ponko_ctx_occlusion_culling(ctx, &culling) == PONKO_OK && culling);
    CHECK(ponko_ctx_set_occlusion_culling(ctx, false) == PONKO_OK);
    CHECK(ponko_ctx_occlusion_culling(NULL, &culling) == PONKO_INVALID_ARGUMENT);
    PonkoLighting light;
    CHECK(ponko_ctx_lighting(ctx, &light) == PONKO_OK && !light.has_equator);
    light.has_equator = true;
    light.equator_color = (PonkoColor){0, 1, 0, 1};
    CHECK(ponko_ctx_set_lighting(ctx, &light) == PONKO_OK);
    PonkoLighting lit;
    CHECK(ponko_ctx_lighting(ctx, &lit) == PONKO_OK && lit.has_equator && lit.equator_color.g == 1);
    light.has_equator = false;
    CHECK(ponko_ctx_set_lighting(ctx, &light) == PONKO_OK);
    /* Shader status: what compiled, and where an error is in the game's WGSL. */
    size_t shaders_before = 0;
    CHECK(ponko_ctx_shader_status_count(ctx, &shaders_before) == PONKO_OK);
    PonkoShader broken_shader;
    CHECK(ponko_ctx_create_shader(ctx, "broken.wgsl", "// tint\nfn surface(s: Surface) -> Surface {\n    return 1.0;\n}", &broken_shader) == PONKO_INVALID_ARGUMENT);
    size_t shaders = 0;
    CHECK(ponko_ctx_shader_status_count(ctx, &shaders) == PONKO_OK && shaders == shaders_before + 1);
    PonkoShaderStatus shader_status;
    CHECK(ponko_ctx_shader_status(ctx, shaders - 1, &shader_status) == PONKO_OK);
    CHECK(shader_status.kind == PONKO_SHADER_MATERIAL && shader_status.outcome == PONKO_SHADER_FAILED && !shader_status.ok);
    CHECK(shader_status.line == 3 && shader_status.column == 12 && !shader_status.in_engine_code);
    char shader_text[1024];
    size_t shader_len = 0;
    CHECK(ponko_ctx_shader_status_name(ctx, shaders - 1, shader_text, sizeof shader_text, &shader_len) == PONKO_OK);
    CHECK(strcmp(shader_text, "broken.wgsl") == 0 && shader_len == 11);
    CHECK(ponko_ctx_shader_status_text(ctx, shaders - 1, shader_text, sizeof shader_text, &shader_len) == PONKO_OK);
    CHECK(strncmp(shader_text, "broken.wgsl:3:12: error: ", 25) == 0);
    CHECK(ponko_ctx_shader_status_json(ctx, NULL, 0, &shader_len) == PONKO_OK && shader_len > 0);
    CHECK(ponko_ctx_shader_status(ctx, shaders, &shader_status) == PONKO_INVALID_ARGUMENT);
    /* Global illumination: off by default, its settings checked when on. */
    CHECK(!light.gi_enabled && light.gi_spacing == 2 && light.gi_probes_per_frame == 4);
    light.gi_enabled = true;
    light.gi_probes_per_frame = 0;
    CHECK(ponko_ctx_set_lighting(ctx, &light) == PONKO_INVALID_ARGUMENT);
    light.gi_probes_per_frame = 8;
    CHECK(ponko_ctx_set_lighting(ctx, &light) == PONKO_OK);
    CHECK(ponko_ctx_lighting(ctx, &lit) == PONKO_OK && lit.gi_enabled && lit.gi_probes_per_frame == 8);
    light.gi_enabled = false;
    CHECK(ponko_ctx_set_lighting(ctx, &light) == PONKO_OK);
    PonkoPost post;
    CHECK(ponko_ctx_post(ctx, &post) == PONKO_OK);
    CHECK(!post.ao_enabled && post.anti_aliasing == 0 && post.ao_radius == 1);
    post.ao_enabled = true; post.ao_half_resolution = true; post.ao_temporal = true; post.ao_radius = 2; post.ao_intensity = 1.5f;
    post.ao_slices = 4; post.ao_steps = 8; post.anti_aliasing = 1;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_OK);
    PonkoPost back;
    CHECK(ponko_ctx_post(ctx, &back) == PONKO_OK);
    CHECK(back.ao_half_resolution && back.ao_temporal && back.ao_enabled && back.ao_radius == 2 && back.anti_aliasing == 1);
    post.anti_aliasing = 4;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_INVALID_ARGUMENT);
    CHECK(back.render_scale == 1 && back.sharpen == 0);
    post.anti_aliasing = 1;
    post.render_scale = 0.1f;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_INVALID_ARGUMENT);
    post.render_scale = 0.5f; post.sharpen = 0.3f;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_OK);
    CHECK(ponko_ctx_post(ctx, &back) == PONKO_OK && back.render_scale == 0.5f && back.sharpen == 0.3f);
    PonkoGraphicsQuality quality, now;
    uint32_t level = 99;
    CHECK(ponko_graphics_quality_preset(4, &quality) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_graphics_quality_preset(0, &quality) == PONKO_OK);
    CHECK(quality.shadow_cascades == 2 && quality.light_shadow_views == 0 && quality.render_scale < 1);
    CHECK(quality.light_shadow_atlas == 1024);
    CHECK(ponko_graphics_quality_level(&quality, &level) == PONKO_OK && level == 0);
    CHECK(ponko_ctx_set_graphics_quality(ctx, &quality) == PONKO_OK);
    CHECK(ponko_ctx_graphics_quality(ctx, &now) == PONKO_OK && now.shadow_resolution == 1024);
    CHECK(ponko_ctx_post(ctx, &back) == PONKO_OK && back.render_scale == quality.render_scale);
    quality.shadow_resolution = 100;
    CHECK(ponko_ctx_set_graphics_quality(ctx, &quality) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_graphics_quality_preset(3, &quality) == PONKO_OK);
    CHECK(ponko_ctx_set_graphics_quality(ctx, &quality) == PONKO_OK);
    CHECK(ponko_ctx_graphics_quality(ctx, &now) == PONKO_OK && now.shadow_resolution == 4096 && now.ao_steps == 8 && now.shadow_refresh == 1 && now.shadow_filter == 3 && now.ssr == 1 && now.gi);
    CHECK(now.light_shadow_atlas == 8192 && now.light_shadow_views == 24);
    quality.light_shadow_atlas = 3000; /* not a power of two */
    CHECK(ponko_ctx_set_graphics_quality(ctx, &quality) == PONKO_INVALID_ARGUMENT);
    /* GPU tiers: reports and presets need no GPU; a headless ctx has none. */
    {
        size_t len = 0;
        bool on = true;
        uint32_t tier = 99;
        char small[8];
        PonkoGraphicsQuality web;
        CHECK(ponko_ctx_gpu_capabilities(ctx, NULL, 0, &len) == PONKO_NOT_FOUND);
        CHECK(ponko_ctx_gpu_tier(ctx, &tier) == PONKO_NOT_FOUND && tier == 99);
        CHECK(ponko_ctx_gpu_feature(ctx, PONKO_GPU_FEATURE_DECALS, &on) == PONKO_NOT_FOUND);
        CHECK(ponko_ctx_recommended_graphics_quality(ctx, &web) == PONKO_NOT_FOUND);
        CHECK(ponko_gpu_tier_report(PONKO_GPU_TIER_BASELINE, NULL, 0, &len) == PONKO_OK && len > 100);
        CHECK(ponko_gpu_tier_report(PONKO_GPU_TIER_BASELINE, small, sizeof small, &len) == PONKO_OK && small[0] == 0);
        CHECK(ponko_gpu_tier_report(3, NULL, 0, &len) == PONKO_INVALID_ARGUMENT);
        CHECK(ponko_gpu_tier_feature(PONKO_GPU_TIER_WEBGL2, PONKO_GPU_FEATURE_TEMPORAL_ANTIALIASING, &on) == PONKO_OK && !on);
        CHECK(ponko_gpu_tier_feature(PONKO_GPU_TIER_BASELINE, PONKO_GPU_FEATURE_TEMPORAL_ANTIALIASING, &on) == PONKO_OK && on);
        CHECK(ponko_gpu_tier_feature(PONKO_GPU_TIER_BASELINE, PONKO_GPU_FEATURE_DECALS, &on) == PONKO_OK && !on);
        CHECK(ponko_gpu_tier_feature(PONKO_GPU_TIER_DESKTOP, PONKO_GPU_FEATURE_GLOBAL_ILLUMINATION, &on) == PONKO_OK && on);
        CHECK(ponko_gpu_tier_feature(PONKO_GPU_TIER_DESKTOP, 13, &on) == PONKO_INVALID_ARGUMENT);
        CHECK(ponko_graphics_quality_for_tier(PONKO_GPU_TIER_WEBGL2, &web) == PONKO_OK);
        CHECK(web.anti_aliasing == 1 && !web.ambient_occlusion && web.ssr == 0 && !web.gi && web.shadow_cascades == 2);
        CHECK(ponko_graphics_quality_for_tier(PONKO_GPU_TIER_DESKTOP, &web) == PONKO_OK);
        CHECK(ponko_graphics_quality_level(&web, &level) == PONKO_OK && level == 2);
        CHECK(ponko_graphics_quality_for_tier(3, &web) == PONKO_INVALID_ARGUMENT);
    }
    post.anti_aliasing = 3; /* temporal */
    post.taa_sharpness = 2;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_INVALID_ARGUMENT);
    post.taa_sharpness = 0.5f;
    post.contact_shadows = -1;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_INVALID_ARGUMENT);
    post.contact_shadows = 0.3f;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_OK);
    CHECK(post.ssr == 0 && post.ssr_max_distance == 30); /* screen-space reflections: off */
    post.ssr = 2;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_INVALID_ARGUMENT);
    post.ssr = 0.5f;
    post.ssr_max_distance = 0;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_INVALID_ARGUMENT);
    post.ssr_max_distance = 20;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_OK);
    post.ao_enabled = true; post.ao_temporal = true;
    CHECK(ponko_ctx_set_post(ctx, &post) == PONKO_OK);
    CHECK(ponko_ctx_reset_temporal_history(ctx) == PONKO_OK);
    CHECK(ponko_ctx_post(ctx, &back) == PONKO_OK && back.anti_aliasing == 3 && back.taa_sharpness == 0.5f && back.contact_shadows == 0.3f && back.ssr == 0.5f && back.ssr_max_distance == 20);
    uint64_t e;
    CHECK(ponko_spawn(ponko_ctx_world(ctx), NULL, 0, &e) == PONKO_OK);
    CHECK(ponko_world_free(ponko_ctx_world(ctx)) == PONKO_BUSY);
    PonkoText3D t = ponko_text3d_default(), got;
    t.billboard = true; t.align = 2;
    CHECK(ponko_ctx_set_text3d(ctx, e, "Hello\nworld", &t) == PONKO_OK);
    CHECK(ponko_ctx_text3d(ctx, e, &got) == PONKO_OK && got.billboard && got.align == 2);
    char text[32]; size_t len;
    CHECK(ponko_ctx_text3d_text(ctx, e, text, sizeof(text), &len) == PONKO_OK);
    CHECK(len == 11 && strcmp(text, "Hello\nworld") == 0);
    t.font.id = UINT32_MAX;
    CHECK(ponko_ctx_set_text3d(ctx, e, "bad", &t) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_remove_text3d(ctx, e) == PONKO_OK);
    CHECK(ponko_ctx_text3d(ctx, e, &got) == PONKO_NOT_FOUND);

    CHECK(ponko_ctx_set_nav_avoidance(ctx, e, true, 2, 3.6f) == PONKO_NOT_FOUND);
    CHECK(ponko_ctx_set_nav_agent(ctx, e, 3, 0.3f, 10) == PONKO_OK);
    PonkoVec3 center = {0, 1, 0}, half = {1, 1, 1};
    CHECK(!ponko_nav_bake_settings_default().carving && ponko_nav_bake_settings_default().tile_size == 32);
    CHECK(ponko_ctx_set_nav_carve_obstacle(ctx, e, center, half) == PONKO_OK);
    size_t steps = 0; bool pending = false;
    CHECK(ponko_ctx_navmesh_update_carving(ctx, 0, &steps, &pending) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_set_nav_avoidance(ctx, e, true, 2, 3.6f) == PONKO_OK);
    CHECK(ponko_ctx_set_nav_avoidance(ctx, e, false, 0, 1) == PONKO_OK);
    CHECK(ponko_ctx_set_nav_avoidance(ctx, e, true, -1, 3) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_set_nav_avoidance(ctx, e, true, 2, 0) == PONKO_INVALID_ARGUMENT);
}
static void tick(PonkoCtx* ctx, void* user) {
    Run* r = user;
    size_t len;
    CHECK(ponko_world_len(ponko_ctx_world(ctx), &len) == PONKO_OK && len == 1);
    r->dt = ponko_ctx_dt(ctx);
    if (++r->ticks == 3) CHECK(ponko_ctx_quit(ctx) == PONKO_OK);
}
/* A box car on four raycast wheels: spawned and driven through the C API,
   the vehicle as a query column. */
typedef struct { uint32_t transform, body, collider, vehicle; uint64_t car; } Cars;
static void ids(PonkoWorld* w, Cars* c) {
    CHECK(ponko_transform3d_component(w, &c->transform) == PONKO_OK);
    CHECK(ponko_rigid_body3d_component(w, &c->body) == PONKO_OK);
    CHECK(ponko_collider3d_component(w, &c->collider) == PONKO_OK);
    CHECK(ponko_vehicle3d_component(w, &c->vehicle) == PONKO_OK);
}
static void car_setup(PonkoCtx* ctx, void* user) {
    Cars* c = user;
    PonkoWorld* w = ponko_ctx_world(ctx);
    ids(w, c);
    PonkoTransform3D floor_at = ponko_transform3d_at((PonkoVec3){0, -0.5f, 0});
    PonkoRigidBody3D fixed = ponko_rigid_body3d(PONKO_BODY_FIXED);
    PonkoCollider3D floor = ponko_collider3d_box((PonkoVec3){200, 1, 200});
    PonkoValue ground[] = {{c->transform, &floor_at, sizeof floor_at}, {c->body, &fixed, sizeof fixed},
                            {c->collider, &floor, sizeof floor}};
    uint64_t e; CHECK(ponko_spawn(w, ground, 3, &e) == PONKO_OK);
    PonkoTransform3D at = ponko_transform3d_at((PonkoVec3){0, 0.8f, 0});
    PonkoRigidBody3D chassis = ponko_rigid_body3d(PONKO_BODY_DYNAMIC);
    chassis.mass = 1200;
    chassis.has_center_of_mass = true;
    chassis.center_of_mass = (PonkoVec3){0, -0.3f, 0};
    PonkoCollider3D box = ponko_collider3d_box((PonkoVec3){1.8f, 0.6f, 4.2f});
    PonkoVehicle3D car = ponko_vehicle3d();
    const float x[] = {-0.8f, 0.8f, -0.8f, 0.8f}, z[] = {-1.3f, -1.3f, 1.3f, 1.3f};
    for (int i = 0; i < 4; ++i)
        CHECK(ponko_vehicle3d_add_wheel(&car, ponko_wheel3d((PonkoVec3){x[i], -0.3f, z[i]}, 0.35f)) == PONKO_OK);
    PonkoValue values[] = {{c->transform, &at, sizeof at}, {c->body, &chassis, sizeof chassis},
                            {c->collider, &box, sizeof box}, {c->vehicle, &car, sizeof car}};
    CHECK(ponko_spawn(w, values, 4, &c->car) == PONKO_OK);
}
static void throttle(const PonkoChunk* chunk, void* user) {
    (void)user;
    PonkoVehicle3D* cars = chunk->columns[0];
    for (size_t i = 0; i < chunk->count; ++i) {
        cars[i].wheels[2].motor_torque = 500;
        cars[i].wheels[3].motor_torque = 500;
    }
}
static void drive(PonkoCtx* ctx, void* user) {
    Cars* c = user;
    if (ponko_ctx_sim_tick(ctx) > 25) CHECK(ponko_query(ponko_ctx_world(ctx), &c->vehicle, 1, throttle, NULL) == PONKO_OK);
}
static void vehicles(void) {
    Cars c = {0};
    PonkoGame* game = ponko_game_new("c vehicles", 1, 1); CHECK(game);
    CHECK(ponko_game_sim_system(game, "drive", 1, drive, &c) == PONKO_OK);
    CHECK(ponko_game_physics3d(game, (PonkoVec3){0, -9.81f, 0}) == PONKO_OK);
    PonkoWorld* w = NULL;
    CHECK(ponko_game_run_headless(game, 150, car_setup, &c, &w) == PONKO_OK);
    PonkoVehicle3D car; PonkoTransform3D t;
    CHECK(ponko_get(w, c.car, c.vehicle, &car, sizeof car) == PONKO_OK);
    CHECK(ponko_get(w, c.car, c.transform, &t, sizeof t) == PONKO_OK);
    CHECK(car.wheel_count == 4 && car.wheels[0].grounded && car.wheels[3].angular_velocity > 0);
    CHECK(t.position.z < -3 && t.position.y > 0.7f && t.position.y < 0.9f);
    PonkoTransform3D wheel = ponko_wheel3d_pose(&car.wheels[0], t);
    CHECK(wheel.position.y > 0.3f && wheel.position.y < 0.4f);
    CHECK(ponko_friction_curve_evaluate(ponko_friction_curve_sideways(), 0.2f) == 1);
    CHECK(ponko_world_free(w) == PONKO_OK);
}
/* Phases: a BEFORE_PHYSICS system feeds this tick's step; a sim system,
   even one added before physics3d, runs after the step and sees its result. */
typedef struct { uint32_t transform, velocity; uint64_t ball; float seen_before, seen_after; } Kick;
static void kick_setup(PonkoCtx* ctx, void* user) {
    Kick* k = user;
    PonkoWorld* w = ponko_ctx_world(ctx);
    uint32_t body, collider;
    CHECK(ponko_transform3d_component(w, &k->transform) == PONKO_OK);
    CHECK(ponko_velocity3d_component(w, &k->velocity) == PONKO_OK);
    CHECK(ponko_rigid_body3d_component(w, &body) == PONKO_OK);
    CHECK(ponko_collider3d_component(w, &collider) == PONKO_OK);
    PonkoTransform3D at = ponko_transform3d_at((PonkoVec3){0, 0, 0});
    PonkoRigidBody3D rb = ponko_rigid_body3d(PONKO_BODY_DYNAMIC);
    PonkoCollider3D sphere = ponko_collider3d_sphere(0.5f);
    PonkoVelocity3D still = {{0, 0, 0}, {0, 0, 0}};
    PonkoValue values[] = {{k->transform, &at, sizeof at}, {body, &rb, sizeof rb},
                            {collider, &sphere, sizeof sphere}, {k->velocity, &still, sizeof still}};
    CHECK(ponko_spawn(w, values, 4, &k->ball) == PONKO_OK);
}
static void kick(PonkoCtx* ctx, void* user) {
    Kick* k = user;
    PonkoWorld* w = ponko_ctx_world(ctx);
    PonkoTransform3D t; CHECK(ponko_get(w, k->ball, k->transform, &t, sizeof t) == PONKO_OK);
    k->seen_before = t.position.x;
    PonkoVelocity3D v = {{5, 0, 0}, {0, 0, 0}};
    CHECK(ponko_set(w, k->ball, k->velocity, &v, sizeof v) == PONKO_OK);
}
static void watch(PonkoCtx* ctx, void* user) {
    Kick* k = user;
    PonkoTransform3D t; CHECK(ponko_get(ponko_ctx_world(ctx), k->ball, k->transform, &t, sizeof t) == PONKO_OK);
    k->seen_after = t.position.x;
}
static void phases(void) {
    Kick k = {0};
    PonkoGame* game = ponko_game_new("c phases", 1, 1); CHECK(game);
    CHECK(ponko_game_fixed_hz(game, 50) == PONKO_OK);
    CHECK(ponko_game_sim_system(game, "watch", 1, watch, &k) == PONKO_OK);
    CHECK(ponko_game_add_system(game, PONKO_PHASE_BEFORE_PHYSICS, "kick", 1, kick, &k) == PONKO_OK);
    CHECK(ponko_game_add_system(game, 99, "bad", 1, kick, &k) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_game_add_system(game, PONKO_PHASE_FRAME, "bad", 2, kick, &k) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_game_physics3d(game, (PonkoVec3){0, 0, 0}) == PONKO_OK);
    PonkoWorld* w = NULL;
    CHECK(ponko_game_run_headless(game, 1, kick_setup, &k, &w) == PONKO_OK);
    CHECK(k.seen_before == 0);
    CHECK(fabsf(k.seen_after - 5.0f / 50.0f) < 1e-3f);
    CHECK(ponko_world_free(w) == PONKO_OK);
}
/* The retained UI: a column with a button, laid out and clicked from C. */
static void ui_setup(PonkoCtx* ctx, void* user) {
    (void)user;
    PonkoUiLayout column = ponko_ui_layout_default();
    column.flex_direction = PONKO_UI_FLEX_COLUMN;
    column.width.kind = PONKO_UI_LENGTH_POINTS; column.width.value = 300;
    column.height.kind = PONKO_UI_LENGTH_POINTS; column.height.value = 200;
    uint64_t root, button, label;
    CHECK(ponko_ctx_ui_add_panel(ctx, PONKO_UI_SCREEN, PONKO_UI_NONE, &column, &root) == PONKO_OK);
    PonkoUiLayout fixed = ponko_ui_layout_default();
    fixed.width.kind = PONKO_UI_LENGTH_POINTS; fixed.width.value = 120;
    fixed.height.kind = PONKO_UI_LENGTH_POINTS; fixed.height.value = 40;
    CHECK(ponko_ctx_ui_add_label(ctx, PONKO_UI_SCREEN, root, "Shop", &fixed, &label) == PONKO_OK);
    CHECK(ponko_ctx_ui_add_button(ctx, PONKO_UI_SCREEN, root, "Buy", &fixed, &button) == PONKO_OK);
    CHECK(ponko_ctx_ui_push_screen(ctx, PONKO_UI_SCREEN, root, false) == PONKO_OK);
    PonkoVec2 window = {800, 600};
    CHECK(ponko_ctx_ui_frame(ctx, PONKO_UI_SCREEN, window, NULL) == PONKO_OK);
    PonkoRect r;
    CHECK(ponko_ctx_ui_rect(ctx, PONKO_UI_SCREEN, button, &r) == PONKO_OK && r.min.y == 40 && r.max.x == 120);
    char text[8]; size_t len;
    CHECK(ponko_ctx_ui_text(ctx, PONKO_UI_SCREEN, button, text, sizeof text, &len) == PONKO_OK && len == 3 && text[0] == 'B');
    PonkoUiInput in = ponko_ui_input_default();
    in.pointer.x = 10; in.pointer.y = 50; in.pressed = true; in.held = true;
    CHECK(ponko_ctx_ui_frame(ctx, PONKO_UI_SCREEN, window, &in) == PONKO_OK);
    in.pressed = false; in.held = false; in.released = true;
    CHECK(ponko_ctx_ui_frame(ctx, PONKO_UI_SCREEN, window, &in) == PONKO_OK);
    PonkoUiEvent event;
    CHECK(ponko_ctx_ui_next_event(ctx, PONKO_UI_SCREEN, &event) == PONKO_OK);
    CHECK(event.kind == PONKO_UI_EVENT_CLICKED && event.id == button);
    CHECK(ponko_ctx_ui_next_event(ctx, PONKO_UI_SCREEN, &event) == PONKO_NOT_FOUND);
    /* The original flat track arrays remain valid; templates add repeat fragments. */
    PonkoUiTrack repeated_track = {PONKO_UI_TRACK_FR, 1};
    PonkoUiTemplateTrack repeated = {0};
    repeated.repetition = 1; repeated.count = 2;
    repeated.tracks = &repeated_track; repeated.track_count = 1;
    PonkoUiTrack rows[] = {{PONKO_UI_TRACK_POINTS, 40}, {PONKO_UI_TRACK_POINTS, 160}};
    PonkoUiLayout named = ponko_ui_layout_default();
    named.display = PONKO_UI_DISPLAY_GRID;
    named.width = (PonkoUiLength){PONKO_UI_LENGTH_POINTS, 300};
    named.height = (PonkoUiLength){PONKO_UI_LENGTH_POINTS, 200};
    named.grid_template_columns = &repeated; named.grid_template_column_count = 1;
    named.grid_rows = rows; named.grid_row_count = 2;
    uint64_t grid, main;
    CHECK(ponko_ctx_ui_add_panel(ctx, PONKO_UI_SCREEN, PONKO_UI_NONE, &named, &grid) == PONKO_OK);
    const char* areas[] = {"header header", "side main"};
    CHECK(ponko_ctx_ui_set_grid_template_areas(ctx, PONKO_UI_SCREEN, grid, areas, 2) == PONKO_OK);
    CHECK(ponko_ctx_ui_add_panel(ctx, PONKO_UI_SCREEN, grid, NULL, &main) == PONKO_OK);
    CHECK(ponko_ctx_ui_set_grid_area(ctx, PONKO_UI_SCREEN, main, "main") == PONKO_OK);
    CHECK(ponko_ctx_ui_push_screen(ctx, PONKO_UI_SCREEN, grid, false) == PONKO_OK);
    CHECK(ponko_ctx_ui_layout(ctx, PONKO_UI_SCREEN, (PonkoVec2){800, 600}) == PONKO_OK);
    PonkoRect main_rect;
    CHECK(ponko_ctx_ui_rect(ctx, PONKO_UI_SCREEN, main, &main_rect) == PONKO_OK);
    CHECK(main_rect.min.x == 150 && main_rect.min.y == 40 && main_rect.max.x == 300 && main_rect.max.y == 200);
    const char* invalid_areas[] = {"x x", "x y"};
    CHECK(ponko_ctx_ui_set_grid_template_areas(ctx, PONKO_UI_SCREEN, grid, invalid_areas, 2) == PONKO_INVALID_ARGUMENT);
    const char* names[] = {"start", "finish"};
    CHECK(ponko_ctx_ui_set_grid_line_names(ctx, PONKO_UI_SCREEN, grid, 0, names, 2) == PONKO_OK);
    CHECK(ponko_ctx_ui_set_grid_named_line(ctx, PONKO_UI_SCREEN, main, 0, false, "start", 1) == PONKO_OK);
    CHECK(ponko_ctx_ui_set_grid_named_line(ctx, PONKO_UI_SCREEN, main, 0, true, "finish", 1) == PONKO_OK);
    CHECK(ponko_ctx_ui_layout(ctx, PONKO_UI_SCREEN, (PonkoVec2){800, 600}) == PONKO_OK);
    CHECK(ponko_ctx_ui_rect(ctx, PONKO_UI_SCREEN, main, &main_rect) == PONKO_OK);
    CHECK(main_rect.min.x == 0 && main_rect.max.x == 300);
    column.display = 99; /* not a PonkoUiDisplay */
    uint64_t bad;
    CHECK(ponko_ctx_ui_add_panel(ctx, PONKO_UI_SCREEN, root, &column, &bad) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_ui_remove(ctx, PONKO_UI_SCREEN, root) == PONKO_OK);
    CHECK(ponko_ctx_ui_remove(ctx, PONKO_UI_SCREEN, button) == PONKO_NOT_FOUND);
}
/* A world-space UI panel: its UI takes the same calls, with the entity as the target. */
static void panel_setup(PonkoCtx* ctx, void* user) {
    (void)user;
    PonkoWorld* w = ponko_ctx_world(ctx);
    uint32_t transform; uint64_t entity;
    CHECK(ponko_transform3d_component(w, &transform) == PONKO_OK);
    PonkoTransform3D at = ponko_transform3d_at((PonkoVec3){0, 1.5f, -2});
    PonkoValue value = {transform, &at, sizeof at};
    CHECK(ponko_spawn(w, &value, 1, &entity) == PONKO_OK);
    PonkoTexture texture;
    CHECK(ponko_ctx_create_ui_panel(ctx, entity, 600, 400, (PonkoVec2){1.2f, 0.8f}, &texture) == PONKO_OK);
    CHECK(texture.width == 600 && texture.height == 400);
    PonkoUiLayout fixed = ponko_ui_layout_default();
    fixed.width.kind = PONKO_UI_LENGTH_POINTS; fixed.width.value = 200;
    fixed.height.kind = PONKO_UI_LENGTH_POINTS; fixed.height.value = 60;
    uint64_t button, top;
    CHECK(ponko_ctx_ui_add_button(ctx, entity, PONKO_UI_NONE, "Buy", &fixed, &button) == PONKO_OK);
    CHECK(ponko_ctx_ui_push_screen(ctx, entity, button, false) == PONKO_OK);
    CHECK(ponko_ctx_ui_top_screen(ctx, PONKO_UI_SCREEN, &top) == PONKO_OK && top == PONKO_UI_NONE);
    CHECK(ponko_ctx_ui_top_screen(ctx, entity, &top) == PONKO_OK && top == button);
    PonkoVec2 pixels = {600, 400};
    CHECK(ponko_ctx_ui_layout(ctx, entity, pixels) == PONKO_OK);
    PonkoRect r;
    CHECK(ponko_ctx_ui_rect(ctx, entity, button, &r) == PONKO_OK && r.max.x == 200 && r.max.y == 60);
    bool over; PonkoVec2 pixel;
    CHECK(ponko_ctx_ui_panel_pointer(ctx, entity, &over, &pixel) == PONKO_OK && !over);
    CHECK(ponko_ctx_set_ui_panel_interactive(ctx, entity, false) == PONKO_OK);
    CHECK(ponko_ctx_set_ui_pointer_ray(ctx, (PonkoVec3){0, 1.5f, 0}, (PonkoVec3){0, 0, -1}) == PONKO_OK);
    CHECK(ponko_ctx_set_ui_pointer_ray(ctx, (PonkoVec3){0, 0, 0}, (PonkoVec3){0, 0, 0}) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_clear_ui_pointer_ray(ctx) == PONKO_OK);
    CHECK(ponko_ctx_remove_ui_panel(ctx, entity) == PONKO_OK);
    CHECK(ponko_ctx_ui_top_screen(ctx, entity, &top) == PONKO_NOT_FOUND);
}
static void ui_panels(void) {
    PonkoGame* game = ponko_game_new("c panels", 1, 1); CHECK(game);
    PonkoWorld* w = NULL;
    CHECK(ponko_game_run_headless(game, 0, panel_setup, NULL, &w) == PONKO_OK);
    CHECK(ponko_world_free(w) == PONKO_OK);
}
static void retained_ui(void) {
    PonkoGame* game = ponko_game_new("c ui", 1, 1); CHECK(game);
    PonkoWorld* w = NULL;
    CHECK(ponko_game_run_headless(game, 0, ui_setup, NULL, &w) == PONKO_OK);
    CHECK(ponko_world_free(w) == PONKO_OK);
}
/* Ribbon and line defaults, and a decal cut from a floor. */
static void ribbons_and_decals(void) {
    PonkoTrail3D trail = ponko_trail3d_new(0.25f, 0.05f);
    CHECK(trail.lifetime == 0.25f && trail.width == 0.05f && trail.max_points == 64 && trail.emitting && !trail.has_texture);
    PonkoLine3D line = ponko_line3d_new(0.1f, (PonkoColor){1, 0, 0, 1});
    CHECK(line.width == 0.1f && line.point_count == 0 && line.points == NULL && line.texture_length == 0);
    PonkoMeshData* floor = ponko_mesh_data_plane(4);
    float down[16] = {0.5f, 0, 0, 0, 0, 0, -0.2f, 0, 0, 0.5f, 0, 0, 1, 0, 0, 1};
    PonkoMeshData* mark = ponko_mesh_data_decal(floor, down, 0.002f);
    CHECK(mark);
    PonkoMeshView view;
    CHECK(ponko_mesh_data_view(mark, &view) == PONKO_OK && view.index_count >= 3);
    ponko_mesh_data_free(mark);
    CHECK(ponko_mesh_data_decal(floor, NULL, 0.002f) == NULL);
    ponko_mesh_data_free(floor);
}
static void far_ground(PonkoCtx* ctx, void* user) {
    (void)user;
    PonkoMaterial ground; CHECK(ponko_ctx_default_material(ctx, &ground) == PONKO_OK);
    static float heights[33 * 33];
    for (int i = 0; i < 33 * 33; ++i) heights[i] = 1.0f;
    PonkoVec2 corner = {-256, -256};
    PonkoFarTerrain* far = ponko_far_terrain_new(heights, 33 * 33, 33, corner, 16, 64, ground); CHECK(far);
    CHECK(ponko_far_terrain_update(far, ctx) == PONKO_OK);
    PonkoCell cell = {0, 0}; bool cut = true; size_t meshes = 0;
    CHECK(ponko_far_terrain_is_cut(far, cell, &cut, &meshes) == PONKO_OK && !cut && meshes == 1);
    CHECK(ponko_far_terrain_cut(far, cell, true) == PONKO_OK);
    CHECK(ponko_far_terrain_update(far, ctx) == PONKO_OK);
    CHECK(ponko_far_terrain_is_cut(far, cell, &cut, NULL) == PONKO_OK && cut);
    CHECK(ponko_far_terrain_clear(far, ctx) == PONKO_OK);
    ponko_far_terrain_free(far);
    PonkoCollisionMeshData* data = ponko_collision_mesh_data_heightfield(heights, 33 * 33, 64); CHECK(data);
    PonkoCollisionMesh mesh;
    CHECK(ponko_ctx_physics3d_add_mesh(ctx, data, &mesh) == PONKO_OK);
    ponko_collision_mesh_data_free(data);
    PonkoTerrainCollider solid = ponko_terrain_collider(mesh);
    CHECK(solid.friction == .5f && solid.mesh.id == mesh.id);
    PonkoColliderFocus focus = ponko_collider_focus(100, 150);
    CHECK(focus.radius == 100 && focus.keep_radius == 150);
    CHECK(ponko_ctx_physics3d_stream_colliders(ctx, true, 4) == PONKO_OK);
    bool active = true;
    CHECK(ponko_ctx_physics3d_is_collider_active(ctx, 0, &active) == PONKO_OK && !active);
}

static void streamed_worlds(void) {
    PonkoGame* game = ponko_game_new("far", 1, 1); CHECK(game);
    CHECK(ponko_game_physics3d(game, (PonkoVec3){0, -9.81f, 0}) == PONKO_OK);
    PonkoWorld* w = NULL;
    CHECK(ponko_game_run_headless(game, 0, far_ground, NULL, &w) == PONKO_OK);
    CHECK(ponko_world_free(w) == PONKO_OK);
    float flat[4] = {0, 0, 0, 0};
    PonkoMaterial none = {0};
    CHECK(!ponko_far_terrain_new(flat, 4, 2, (PonkoVec2){0, 0}, 1, 1.5f, none));
    CHECK(!ponko_collision_mesh_data_heightfield(flat, 3, 1));
}

static void lod_helpers(void) {
    float fov = 60 * 3.14159265f / 180;
    float h = ponko_lod_screen_height_at(2, 10, fov);
    CHECK(h > 0.172f && h < 0.174f);
    float d = ponko_lod_distance_at(2, h, fov);
    CHECK(d > 9.99f && d < 10.01f);
    PonkoLodRenderer r = ponko_lod_renderer((PonkoMesh){3});
    CHECK(r.mesh.id == 3 && r.color.r == 1 && r.offset.scale.y == 1);
    PonkoLodLevel level = {0.1f, 0, &r, 1, false, NULL, 0};
    CHECK(ponko_ctx_set_lod_group(NULL, 0, 1, (PonkoVec3){0, 0, 0}, &level, 1) == PONKO_INVALID_ARGUMENT);
}
static void streaming_setup(PonkoCtx* ctx, void* user) {
    (void)user;
    CHECK(ponko_ctx_set_deterministic_uploads(ctx, true) == PONKO_OK);
    CHECK(ponko_ctx_set_upload_budget(ctx, true, 1 << 20) == PONKO_OK);
    PonkoTexture t = {0};
    bool ready = true;
    CHECK(ponko_ctx_stream_texture(ctx, "crab.png", &t, &ready) == PONKO_OK && !ready);
    uint32_t state = 0;
    CHECK(ponko_ctx_asset_state(ctx, PONKO_RESIDENCY_TEXTURE, "crab.png", &state) == PONKO_OK);
    CHECK(state == PONKO_ASSET_LOADING);
    PonkoMesh m = {0};
    CHECK(ponko_ctx_stream_mesh(ctx, "models/house.glb", &m, &ready) == PONKO_OK && !ready);
    CHECK(ponko_ctx_finish_uploads(ctx) == PONKO_OK);
    CHECK(ponko_ctx_stream_texture(ctx, "crab.png", &t, &ready) == PONKO_OK && ready && t.width > 0);
    CHECK(ponko_ctx_asset_state(ctx, PONKO_RESIDENCY_TEXTURE, "crab.png", &state) == PONKO_OK);
    CHECK(state == PONKO_ASSET_RESIDENT);
    CHECK(ponko_ctx_stream_mesh(ctx, "models/house.glb", &m, &ready) == PONKO_OK && ready);
    PonkoUploadStats uploads;
    CHECK(ponko_ctx_upload_stats(ctx, &uploads) == PONKO_OK);
    CHECK(uploads.has_budget && uploads.budget == 1 << 20 && uploads.pending == 0 && uploads.total_uploads >= 2);
    /* Budgets: the mesh is pinned, so a zero budget can't evict it. */
    PonkoMesh pinned = {0};
    CHECK(ponko_ctx_resident_mesh(ctx, "models/house.glb", true, &pinned) == PONKO_OK && pinned.id == m.id);
    CHECK(ponko_ctx_set_residency_budget(ctx, PONKO_RESIDENCY_MESH, true, 0) == PONKO_OK);
    PonkoResidencyStats stats;
    CHECK(ponko_ctx_residency(ctx, PONKO_RESIDENCY_MESH, &stats) == PONKO_OK);
    CHECK(stats.has_budget && stats.resident == 1 && stats.pinned == 1 && stats.over_budget_bytes > 0);
    bool resident = false;
    CHECK(ponko_ctx_is_resident(ctx, PONKO_RESIDENCY_MESH, "models/house.glb", &resident) == PONKO_OK && resident);
    CHECK(ponko_ctx_unpin_resident(ctx, PONKO_RESIDENCY_MESH, "models/house.glb") == PONKO_OK);
    PonkoTexture crab = {0};
    CHECK(ponko_ctx_resident_texture(ctx, "crab.png", false, &crab) == PONKO_OK && crab.id == t.id);
    /* A missing file fails with a message. */
    CHECK(ponko_ctx_stream_texture(ctx, "nope.png", &t, &ready) == PONKO_OK && !ready);
    CHECK(ponko_ctx_finish_uploads(ctx) == PONKO_OK);
    CHECK(ponko_ctx_asset_state(ctx, PONKO_RESIDENCY_TEXTURE, "nope.png", &state) == PONKO_OK);
    CHECK(state == PONKO_ASSET_FAILED);
    char message[256];
    size_t len = 0;
    CHECK(ponko_ctx_asset_error(ctx, PONKO_RESIDENCY_TEXTURE, "nope.png", message, sizeof message, &len) == PONKO_OK);
    CHECK(len > 0 && strstr(message, "nope.png"));
    CHECK(ponko_ctx_asset_state(ctx, 7, "nope.png", &state) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_resident_texture(ctx, "nope.png", false, &crab) == PONKO_INVALID_ARGUMENT);
}

static void asset_streaming(void) {
    PonkoGame* game = ponko_game_new("streaming", 1, 1); CHECK(game);
    PonkoWorld* w = NULL;
    CHECK(ponko_game_run_headless(game, 0, streaming_setup, NULL, &w) == PONKO_OK);
    CHECK(ponko_world_free(w) == PONKO_OK);
}

static void steam_messages_validation(void) {
    PonkoNetMessages* endpoint = NULL;
    PonkoSteamLobby* lobby = NULL;
    CHECK(ponko_net_messages_steam_host(480, 0, &endpoint) == PONKO_INVALID_ARGUMENT && !endpoint);
    CHECK(ponko_net_messages_steam_join(480, 0, &endpoint) == PONKO_INVALID_ARGUMENT && !endpoint);
    CHECK(ponko_steam_lobby_create(480, 3, 2, &lobby) == PONKO_INVALID_ARGUMENT && !lobby);
    CHECK(ponko_steam_lobby_join(480, 0, &lobby) == PONKO_INVALID_ARGUMENT && !lobby);
    CHECK(ponko_net_messages_steam_host(480, 2, &endpoint) != PONKO_OK && !endpoint);
    CHECK(ponko_steam_lobby_create(480, 0, 2, &lobby) != PONKO_OK && !lobby);
    CHECK(ponko_steam_lobby_poll(NULL) != PONKO_OK);
    CHECK(ponko_steam_lobby_leave(NULL) != PONKO_OK);
    CHECK(ponko_steam_lobby_set_metadata(NULL, "version", "1") != PONKO_OK);
    ponko_steam_lobby_free(NULL);
}

static void morph_clip_api(void) {
    PonkoClip* clip = NULL;
    CHECK(ponko_clip_new("face", &clip) == PONKO_OK);
    float times[] = {0, 2}, values[] = {0, 1};
    CHECK(ponko_clip_set_morph_track(clip, "smile", times, values, 2, 1) == PONKO_OK);
    CHECK(ponko_clip_duration(clip) == 2);
    float cubic[] = {0, 0, 1, 1, 1, 0};
    CHECK(ponko_clip_set_morph_track(clip, "smile", times, cubic, 2, 2) == PONKO_OK);
    CHECK(ponko_clip_set_morph_track(clip, "smile", times, values, 2, 9) == PONKO_INVALID_ARGUMENT);
    ponko_clip_free(clip);
    PonkoMeshData* mesh = ponko_mesh_data_plane(1);
    PonkoVec3 deltas[4] = {{0, 1, 0}, {0, 1, 0}, {0, 1, 0}, {0, 1, 0}};
    CHECK(ponko_mesh_data_add_morph_target(mesh, "full", deltas, NULL, 4) == PONKO_OK);
    CHECK(ponko_mesh_data_add_morph_target(mesh, "half", deltas, NULL, 4) == PONKO_OK);
    CHECK(ponko_mesh_data_add_morph_frame(mesh, 0, 1, 0.5f) == PONKO_OK);
    CHECK(ponko_mesh_data_add_morph_frame(mesh, 0, 0, 1) == PONKO_OK);
    CHECK(ponko_mesh_data_add_morph_frame(mesh, 0, 0, 1) == PONKO_INVALID_ARGUMENT);
    ponko_mesh_data_free(mesh);
}
/* Replication helpers: sequence numbers, authority, snapshots, interest. */
static void replication(void) {
    CHECK(ponko_seq_newer(1, 0) && ponko_seq_newer(0, 65535) && !ponko_seq_newer(7, 7));
    CHECK(!ponko_seq_newer(65535, 0) && ponko_seq_diff(1, 65535) == 2 && ponko_seq_diff(65535, 1) == -2);
    PonkoUpdateStamp old = {0, 60000}, next = {1, 0};
    CHECK(ponko_stamp_newer(next, old) && !ponko_stamp_newer(old, next) && !ponko_stamp_newer(old, old));

    PonkoAuthority a = ponko_authority_new(0);
    uint16_t epoch = 99;
    bool changed = false;
    PonkoAuthorityCheck check;
    CHECK(ponko_authority_check(&a, 0, 0, &check) == PONKO_OK && check == PONKO_AUTHORITY_ACCEPT);
    CHECK(ponko_authority_handoff(&a, 1, &epoch) == PONKO_OK && epoch == 1 && a.owner == 1);
    CHECK(ponko_authority_check(&a, 0, 0, &check) == PONKO_OK && check == PONKO_AUTHORITY_STALE);
    CHECK(ponko_authority_check(&a, 0, 1, &check) == PONKO_OK && check == PONKO_AUTHORITY_WRONG_OWNER);
    CHECK(ponko_authority_check(&a, 1, 2, &check) == PONKO_OK && check == PONKO_AUTHORITY_AHEAD);
    CHECK(ponko_authority_adopt(&a, 2, 1, &changed) == PONKO_OK && !changed && a.owner == 1); /* race: lower wins */
    CHECK(ponko_authority_adopt(&a, 3, 2, &changed) == PONKO_OK && changed && a.owner == 3 && a.epoch == 2);
    CHECK(ponko_authority_adopt(NULL, 1, 1, &changed) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_authority_check(&a, 1, 1, NULL) == PONKO_INVALID_ARGUMENT);

    PonkoLatestWins* filter = ponko_latest_wins_new();
    bool ok = false;
    PonkoUpdateStamp s1 = {0, 1}, s3 = {0, 3};
    CHECK(ponko_latest_wins_accept(filter, 9, s3, &ok) == PONKO_OK && ok);
    CHECK(ponko_latest_wins_accept(filter, 9, s1, &ok) == PONKO_OK && !ok);
    CHECK(ponko_latest_wins_accept(filter, 9, s3, &ok) == PONKO_OK && !ok);
    CHECK(ponko_latest_wins_accept(filter, 10, s1, &ok) == PONKO_OK && ok && ponko_latest_wins_len(filter) == 2);
    CHECK(ponko_latest_wins_forget(filter, 9) == PONKO_OK);
    CHECK(ponko_latest_wins_accept(filter, 9, s1, &ok) == PONKO_OK && ok);
    CHECK(ponko_latest_wins_clear(filter) == PONKO_OK && ponko_latest_wins_len(filter) == 0);
    CHECK(ponko_latest_wins_accept(NULL, 1, s1, &ok) == PONKO_INVALID_ARGUMENT);
    ponko_latest_wins_free(filter);
    ponko_latest_wins_free(NULL);

    /* A sender 5000 s ahead walks along x at 20 units/s, 20 snapshots a second. */
    PonkoSnapshotStream* stream = NULL;
    CHECK(ponko_snapshot_stream_new(NULL, &stream) == PONKO_OK && stream);
    PonkoPose pose = {0}, shown = {0};
    pose.rotation.w = 1;
    pose.has_velocity = true;
    pose.velocity.x = 20;
    PonkoSnapshotState state;
    PonkoSnapshotPush result;
    bool found = true;
    CHECK(ponko_snapshot_stream_sample(stream, 0, &shown, &state, &found) == PONKO_OK && !found);
    for (int n = 0; n < 20; n++) {
        pose.position.x = (float)n;
        CHECK(ponko_snapshot_stream_push(stream, 5000 + n * 0.05, n * 0.05 + 0.03, &pose, &result) == PONKO_OK);
        CHECK(result == PONKO_SNAPSHOT_ADDED);
    }
    CHECK(ponko_snapshot_stream_push(stream, 5000, 1.0, &pose, &result) == PONKO_OK && result == PONKO_SNAPSHOT_DUPLICATE);
    CHECK(ponko_snapshot_stream_push(stream, NAN, 1.0, &pose, &result) == PONKO_OK && result == PONKO_SNAPSHOT_INVALID);
    CHECK(ponko_snapshot_stream_len(stream) == 20);
    CHECK(ponko_snapshot_stream_sample(stream, 1.0, &shown, &state, &found) == PONKO_OK && found);
    CHECK(state == PONKO_SNAPSHOT_INTERPOLATED && shown.position.x > 17.3f && shown.position.x < 17.5f && shown.has_velocity);
    /* Packets stop: predicted for 250 ms, then held where the prediction ended. */
    CHECK(ponko_snapshot_stream_sample(stream, 1.2, &shown, &state, &found) == PONKO_OK && state == PONKO_SNAPSHOT_EXTRAPOLATED);
    CHECK(ponko_snapshot_stream_sample(stream, 5.0, &shown, &state, &found) == PONKO_OK && state == PONKO_SNAPSHOT_HELD);
    CHECK(shown.position.x > 23.9f && shown.position.x < 24.1f); /* 19 + 0.25 s * 20 */
    double offset = 0;
    CHECK(ponko_snapshot_stream_clock_offset(stream, &offset, &found) == PONKO_OK && found && offset < -4999.9 && offset > -5000.0);
    CHECK(ponko_snapshot_stream_set_delay(stream, 0.0) == PONKO_OK);
    CHECK(ponko_snapshot_stream_set_delay(stream, -1.0) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_snapshot_stream_clear(stream) == PONKO_OK && ponko_snapshot_stream_len(stream) == 0);
    CHECK(ponko_snapshot_stream_sample(stream, 1.0, &shown, NULL, &found) == PONKO_OK && !found);
    CHECK(ponko_snapshot_stream_reset(stream) == PONKO_OK);
    CHECK(ponko_snapshot_stream_clock_offset(stream, &offset, &found) == PONKO_OK && !found);
    CHECK(ponko_snapshot_stream_push(stream, 1, 1, NULL, &result) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_snapshot_stream_sample(NULL, 1.0, &shown, &state, &found) == PONKO_INVALID_ARGUMENT);
    ponko_snapshot_stream_free(stream);
    ponko_snapshot_stream_free(NULL);
    PonkoSnapshotConfig bad = ponko_snapshot_config_default();
    bad.capacity = 1;
    CHECK(ponko_snapshot_stream_new(&bad, &stream) == PONKO_INVALID_ARGUMENT);
    bad = ponko_snapshot_config_default();
    bad.delay = -1;
    CHECK(ponko_snapshot_stream_new(&bad, &stream) == PONKO_INVALID_ARGUMENT);

    PonkoInterest* interest = NULL;
    PonkoInterestConfig config = ponko_interest_config_default();
    CHECK(config.tier_count == 2 && config.tiers[0].radius == 40 && config.tiers[1].every == 4);
    CHECK(ponko_interest_new(&config, &interest) == PONKO_OK);
    PonkoInterestViewer viewer = {1, {0, 0, 0}};
    PonkoInterestEntity world[3] = {{10, {10, 0, 0}}, {20, {100, 0, 0}}, {30, {900, 0, 0}}};
    CHECK(ponko_interest_update(interest, 0, &viewer, 1, world, 3) == PONKO_OK);
    PonkoInterestEntry entries[4];
    size_t count = 0;
    CHECK(ponko_interest_entries(interest, 1, entries, 4, &count) == PONKO_OK && count == 2);
    CHECK(entries[0].entity == 10 && entries[0].tier == 0 && entries[0].send);
    CHECK(entries[1].entity == 20 && entries[1].tier == 1 && entries[1].send && entries[1].distance == 100);
    PonkoInterestChange changes[4];
    CHECK(ponko_interest_changes(interest, changes, 4, &count) == PONKO_OK && count == 2);
    CHECK(changes[0].kind == PONKO_INTEREST_ENTERED && changes[0].peer == 1 && changes[0].entity == 10);
    /* The mid entity wobbles across the edge of the near ring (40): hysteresis keeps it in its ring. */
    for (uint64_t tick = 1; tick < 20; tick++) {
        world[1].position.x = 40 + (tick % 2 ? 3.0f : -3.0f);
        CHECK(ponko_interest_update(interest, tick, &viewer, 1, world, 2) == PONKO_OK);
        CHECK(ponko_interest_entries(interest, 1, entries, 4, &count) == PONKO_OK && count == 2 && entries[1].tier == 1);
        CHECK(ponko_interest_changes(interest, changes, 4, &count) == PONKO_OK && count == 0);
    }
    world[1].position.x = 500;
    CHECK(ponko_interest_update(interest, 20, &viewer, 1, world, 2) == PONKO_OK);
    CHECK(ponko_interest_changes(interest, changes, 4, &count) == PONKO_OK && count == 1);
    CHECK(changes[0].kind == PONKO_INTEREST_LEFT && changes[0].entity == 20);
    CHECK(ponko_interest_entries(interest, 99, NULL, 0, &count) == PONKO_OK && count == 0);
    CHECK(ponko_interest_remove_peer(interest, 1) == PONKO_OK);
    CHECK(ponko_interest_update(interest, 21, NULL, 0, NULL, 0) == PONKO_OK);
    CHECK(ponko_interest_update(interest, 21, NULL, 1, NULL, 0) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_interest_entries(NULL, 1, NULL, 0, &count) == PONKO_INVALID_ARGUMENT);
    ponko_interest_free(interest);
    interest = NULL;
    ponko_interest_free(NULL);
    config.tiers[1].radius = 10; /* not increasing */
    CHECK(ponko_interest_new(&config, &interest) == PONKO_INVALID_ARGUMENT && !interest);
    config = ponko_interest_config_default();
    config.tier_count = 0;
    CHECK(ponko_interest_new(&config, &interest) == PONKO_INVALID_ARGUMENT);
    config.tier_count = 9;
    CHECK(ponko_interest_new(&config, &interest) == PONKO_INVALID_ARGUMENT);
    config = ponko_interest_config_default();
    config.hysteresis = 40;
    CHECK(ponko_interest_new(&config, &interest) == PONKO_INVALID_ARGUMENT);
}

typedef struct { size_t rows; uint64_t last; } Rows;
static void count_rows(const PonkoChunk* chunk, void* user) {
    Rows* r = user;
    r->rows += chunk->count;
    if (chunk->count) r->last = ponko_entity_bits(chunk->entities[chunk->count - 1]);
}
static void bump(const PonkoChunk* chunk, void* user) {
    (void)user;
    for (size_t i = 0; i < chunk->count; ++i) ((Position*)chunk->columns[0])[i].x += 1;
}
static size_t rows_of(PonkoWorld* w, uint32_t id, PonkoQueryOptions options, uint64_t* last) {
    Rows rows = {0, 0};
    CHECK(ponko_query_with(w, &id, 1, &options, count_rows, &rows) == PONKO_OK);
    if (last) *last = rows.last;
    return rows.rows;
}
static void change_detection(void) {
    PonkoWorld* w = ponko_world_new(); CHECK(w);
    uint32_t p, other;
    CHECK(ponko_component_register(w, "Pos", sizeof(Position), _Alignof(Position), &p) == PONKO_OK);
    CHECK(ponko_component_register(w, "Other", sizeof(Position), _Alignof(Position), &other) == PONKO_OK);
    CHECK(ponko_component_track_changes(w, p) == PONKO_OK);
    CHECK(ponko_component_track_changes(w, 99) == PONKO_INVALID_ARGUMENT);
    Position value = {1}; PonkoValue pos = {p, &value, sizeof(value)};
    uint64_t a, b, c, seen, tick, last;
    CHECK(ponko_spawn(w, &pos, 1, &a) == PONKO_OK && ponko_spawn(w, &pos, 1, &b) == PONKO_OK);
    CHECK(ponko_world_advance_change_tick(w, &seen) == PONKO_OK && seen == 1);
    CHECK(ponko_world_change_tick(w, &tick) == PONKO_OK && tick == 2);

    // The defaults are ponko_query: every column may be written.
    PonkoQueryOptions read = ponko_query_options_default();
    CHECK(read.writes == UINT32_MAX && !read.changed && !read.added && !read.any && !read.parallel);
    CHECK(read.since == PONKO_SINCE_LAST_RUN);
    read.writes = 0;
    PonkoQueryOptions changed = read; changed.changed = 1; changed.since = seen;
    PonkoQueryOptions added = read; added.added = 1; added.since = seen;
    // A read-only query changes nothing.
    CHECK(rows_of(w, p, read, NULL) == 2 && rows_of(w, p, changed, NULL) == 0);
    // set and mark_changed do.
    value.x = 5; CHECK(ponko_set(w, b, p, &value, sizeof(value)) == PONKO_OK);
    CHECK(rows_of(w, p, changed, &last) == 1 && last == b && rows_of(w, p, added, NULL) == 0);
    CHECK(ponko_mark_changed(w, a, p) == PONKO_OK && rows_of(w, p, changed, NULL) == 2);
    CHECK(ponko_mark_changed(w, a, other) == PONKO_NOT_FOUND);
    CHECK(ponko_mark_changed(w, a, 99) == PONKO_INVALID_ARGUMENT);
    // The plain query marks every row it visits; a new entity is added.
    CHECK(ponko_world_advance_change_tick(w, &seen) == PONKO_OK);
    changed.since = added.since = seen;
    CHECK(rows_of(w, p, changed, NULL) == 0);
    CHECK(ponko_query(w, &p, 1, bump, NULL) == PONKO_OK && rows_of(w, p, changed, NULL) == 2);
    CHECK(ponko_spawn(w, &pos, 1, &c) == PONKO_OK);
    CHECK(rows_of(w, p, added, &last) == 1 && last == c && rows_of(w, p, changed, NULL) == 3);
    // Writing through a filtered query, on all cores.
    PonkoQueryOptions write_new = added; write_new.writes = 1; write_new.parallel = true;
    CHECK(ponko_query_with(w, &p, 1, &write_new, bump, NULL) == PONKO_OK);
    Position out;
    CHECK(ponko_get(w, c, p, &out, sizeof(out)) == PONKO_OK && out.x == 6);
    CHECK(ponko_get(w, a, p, &out, sizeof(out)) == PONKO_OK && out.x == 2);
    // PONKO_SINCE_LAST_RUN reads the world's own tick (a game sets it per system).
    changed.since = PONKO_SINCE_LAST_RUN;
    CHECK(rows_of(w, p, changed, NULL) == 3);
    CHECK(ponko_world_advance_change_tick(w, &seen) == PONKO_OK);
    CHECK(ponko_world_set_changes_since(w, seen) == PONKO_OK);
    CHECK(ponko_world_changes_since(w, &tick) == PONKO_OK && tick == seen);
    CHECK(rows_of(w, p, changed, NULL) == 0);
    // Removals: a removed component and a despawned entity.
    uint64_t gone[2] = {0, 0}; size_t n = 9;
    CHECK(ponko_removed(w, p, seen, NULL, 0, &n) == PONKO_OK && n == 0);
    CHECK(ponko_remove(w, a, p) == PONKO_OK && ponko_despawn(w, b) == PONKO_OK);
    CHECK(ponko_removed(w, p, PONKO_SINCE_LAST_RUN, gone, 1, &n) == PONKO_OK && n == 2 && gone[0] == a && gone[1] == 0);
    CHECK(ponko_removed(w, p, seen, gone, 2, &n) == PONKO_OK && n == 2 && gone[1] == b);
    CHECK(ponko_removed(w, other, 0, gone, 2, &n) == PONKO_INVALID_ARGUMENT);   // not tracked
    CHECK(ponko_removed(w, p, 0, NULL, 2, &n) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_query_with(w, &p, 1, &read, NULL, NULL) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_query_with(w, &p, 1, NULL, count_rows, &(Rows){0, 0}) == PONKO_OK);
    CHECK(ponko_world_change_tick(w, NULL) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_world_free(w) == PONKO_OK);
}

static void profiling_c(void) {
    const PonkoProfileScope* scope = ponko_profile_scope("c scope");
    CHECK(scope && scope == ponko_profile_scope("c scope"));
    CHECK(ponko_profile_scope(NULL) == NULL);
    ponko_profile_end(scope, ponko_profile_begin(scope));
    ponko_note_allocation(16);
    PonkoGame* game = ponko_game_new("c perf", 1, 1); CHECK(game);
    PonkoWorld* world = NULL;
    CHECK(ponko_game_profile_headless(game, 2, 10, NULL, NULL, &world) == PONKO_OK && world);
    CHECK(ponko_world_free(world) == PONKO_OK);
    size_t len = 0;
    CHECK(ponko_last_perf_report(NULL, 0, &len) == PONKO_OK && len > 10);
    char* json = malloc(len + 1);
    CHECK(ponko_last_perf_report(json, len + 1, &len) == PONKO_OK && json[0] == '{');
    size_t problems = 1;
    CHECK(ponko_perf_check(json, "{\"frame_p99_ms\": 100000}", NULL, 0, &problems) == PONKO_OK && problems == 0);
    CHECK(ponko_perf_check(json, "{\"systems\": {\"missing\": 1}}", NULL, 0, &problems) == PONKO_OK && problems > 0);
    CHECK(ponko_perf_report_text("nope", NULL, 0, &problems) == PONKO_INVALID_ARGUMENT);
    free(json);
}

/* A two-joint skin with one clip, "rise": the arm goes up 2 over 1 second. */
static const char RISE_GLTF[] =
    "{\"asset\": {\"version\": \"2.0\"}, \"scene\": 0, \"scenes\": [{\"nodes\": [0]}],"
    "\"nodes\": [{\"name\": \"root\", \"children\": [1]}, {\"name\": \"arm\", \"mesh\": 0}],"
    "\"meshes\": [{\"primitives\": [{\"attributes\": {\"POSITION\": 2}}]}],"
    "\"skins\": [{\"joints\": [0, 1]}],"
    "\"animations\": [{\"name\": \"rise\", \"samplers\": [{\"input\": 0, \"output\": 1}],"
    "\"channels\": [{\"sampler\": 0, \"target\": {\"node\": 1, \"path\": \"translation\"}}]}],"
    "\"buffers\": [{\"byteLength\": 68, \"uri\": \"data:application/octet-stream;base64,"
    "AAAAAAAAgD8AAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAAAAAACAPwAAAAA=\"}],"
    "\"bufferViews\": [{\"buffer\": 0, \"byteLength\": 8}, {\"buffer\": 0, \"byteOffset\": 8, \"byteLength\": 24},"
    "{\"buffer\": 0, \"byteOffset\": 32, \"byteLength\": 36}],"
    "\"accessors\": [{\"bufferView\": 0, \"componentType\": 5126, \"count\": 2, \"type\": \"SCALAR\", \"min\": [0], \"max\": [1]},"
    "{\"bufferView\": 1, \"componentType\": 5126, \"count\": 2, \"type\": \"VEC3\"},"
    "{\"bufferView\": 2, \"componentType\": 5126, \"count\": 3, \"type\": \"VEC3\", \"min\": [0, 0, 0], \"max\": [1, 1, 0]}]}";

static const char* temp_dir(void) {
    const char* dir = getenv("TMPDIR");
    if (!dir) dir = getenv("TEMP");
    if (!dir) dir = getenv("TMP");
    return dir ? dir : "/tmp";
}

typedef struct { char path[1024]; } AnimatorRun;

/* A graph with a float parameter, a resting state and a "rise" state with an
   event half way, then an animator on a skeleton, run with manual updates. */
static void animator_setup(PonkoCtx* ctx, void* user) {
    const AnimatorRun* run = user;
    PonkoClip* rise = NULL;
    CHECK(ponko_ctx_load_clip(ctx, run->path, "rise", &rise) == PONKO_OK && rise);
    CHECK(fabsf(ponko_clip_duration(rise) - 1.0f) < 1e-5f);
    PonkoClip* missing = NULL;
    CHECK(ponko_ctx_load_clip(ctx, run->path, "no such clip", &missing) != PONKO_OK && !missing);

    PonkoAnimationGraphBuilder* b = ponko_animation_graph_builder_new();
    size_t speed = 99, still = 99, up = 99, go = 99;
    CHECK(ponko_animation_graph_add_float(b, 0.0f, &speed) == PONKO_OK && speed == 0);
    PonkoMotion* motion = NULL;
    CHECK(ponko_motion_clip(rise, &motion) == PONKO_OK);
    CHECK(ponko_animation_graph_add_state(b, motion, 0.0f, -1, true, &still) == PONKO_OK && still == 0);
    CHECK(ponko_motion_clip(rise, &motion) == PONKO_OK);
    CHECK(ponko_animation_graph_add_state(b, motion, 1.0f, -1, false, &up) == PONKO_OK && up == 1);
    CHECK(ponko_animation_graph_add_event(b, up, 0.5f, "half") == PONKO_OK);
    CHECK(ponko_animation_graph_add_event(b, 9, 0.5f, "nowhere") == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_animation_graph_add_transition(b, (int64_t)still, up, -1.0f, 0.0f, 0.0f, &go) == PONKO_OK);
    CHECK(ponko_animation_graph_add_condition(b, go, PONKO_CONDITION_GREATER, speed, 0.5f) == PONKO_OK);
    PonkoAnimationGraph* graph = NULL;
    CHECK(ponko_animation_graph_build(b, still, &graph) == PONKO_OK && graph);
    /* A graph without states does not build; the builder is used up either way. */
    PonkoAnimationGraphBuilder* empty = ponko_animation_graph_builder_new();
    PonkoAnimationGraph* none = NULL;
    CHECK(ponko_animation_graph_build(empty, 3, &none) != PONKO_OK && !none);

    PonkoWorld* world = ponko_ctx_world(ctx);
    uint32_t transform_id;
    CHECK(ponko_transform3d_component(world, &transform_id) == PONKO_OK);
    PonkoTransform3D at = ponko_transform3d_at((PonkoVec3){0, 0, 0});
    PonkoValue part = {transform_id, &at, sizeof at};
    uint64_t e;
    CHECK(ponko_spawn(world, &part, 1, &e) == PONKO_OK);
    CHECK(ponko_ctx_set_animator(ctx, e, graph) != PONKO_OK); /* no skeleton yet */
    CHECK(ponko_ctx_load_skeleton(ctx, e, run->path) == PONKO_OK);
    CHECK(ponko_ctx_set_animator(ctx, e, graph) == PONKO_OK);
    CHECK(ponko_ctx_animator_set_manual(ctx, e, true) == PONKO_OK);

    size_t state = 99; float time = -1;
    CHECK(ponko_ctx_animator_update(ctx, e, 0.1f) == PONKO_OK);
    CHECK(ponko_ctx_animator_state(ctx, e, &state, &time) == PONKO_OK && state == still);
    CHECK(ponko_ctx_animator_set_float(ctx, e, speed, 1.0f) == PONKO_OK);
    /* A wrong kind or index, or bad values, are invalid arguments. */
    CHECK(ponko_ctx_animator_set_bool(ctx, e, speed, true) == PONKO_INVALID_ARGUMENT); /* it is a float */
    CHECK(ponko_ctx_animator_set_float(ctx, e, 9, 1.0f) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_animator_set_float(ctx, e, speed, NAN) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_animator_set_trigger(ctx, e, speed, true) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_animator_update(ctx, e, 0.0f) == PONKO_OK); /* takes the transition */
    CHECK(ponko_ctx_animator_state(ctx, e, &state, &time) == PONKO_OK && state == up);
    CHECK(ponko_ctx_animator_update(ctx, e, 0.75f) == PONKO_OK);
    CHECK(ponko_ctx_animator_state(ctx, e, &state, &time) == PONKO_OK && fabsf(time - 0.75f) < 1e-4f);
    PonkoTransform3D arm;
    CHECK(ponko_ctx_joint_pose(ctx, e, 1, &arm) == PONKO_OK && fabsf(arm.position.y - 1.5f) < 1e-4f);

    char name[64]; bool found = false;
    CHECK(ponko_ctx_animator_next_event(ctx, e, name, sizeof name, &found) == PONKO_OK && found);
    CHECK(strcmp(name, "half") == 0);
    CHECK(ponko_ctx_animator_next_event(ctx, e, name, sizeof name, &found) == PONKO_OK && !found);
    CHECK(ponko_ctx_animator_crossfade(ctx, e, still, 0.0f, 0.0f) == PONKO_OK);
    CHECK(ponko_ctx_animator_update(ctx, e, 0.0f) == PONKO_OK);
    CHECK(ponko_ctx_animator_state(ctx, e, &state, &time) == PONKO_OK && state == still);
    CHECK(ponko_ctx_animator_crossfade(ctx, e, 7, 0.0f, 0.0f) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_animator_crossfade(ctx, e, still, -1.0f, 0.0f) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_animator_crossfade(ctx, e, still, 0.0f, NAN) == PONKO_INVALID_ARGUMENT);

    ponko_animation_graph_free(graph);
    ponko_clip_free(rise);
}

static void animation_graphs(void) {
    AnimatorRun run;
    snprintf(run.path, sizeof run.path, "%s/ponko-c-animator.gltf", temp_dir());
    FILE* file = fopen(run.path, "wb");
    CHECK(file);
    CHECK(fputs(RISE_GLTF, file) >= 0);
    CHECK(fclose(file) == 0);
    PonkoGame* game = ponko_game_new("animator", 1, 1);
    PonkoWorld* world = NULL;
    CHECK(ponko_game_run_headless(game, 0, animator_setup, &run, &world) == PONKO_OK);
    CHECK(ponko_world_free(world) == PONKO_OK);
    remove(run.path);
}

/* Declared access: a C system says what it reads and writes. */
typedef struct { uint32_t pos; uint64_t entity; size_t hits; } Declared;
static void declared_mover(PonkoCtx* ctx, void* user) {
    Declared* d = user;
    Position at;
    CHECK(ponko_get(ponko_ctx_world(ctx), d->entity, d->pos, &at, sizeof(at)) == PONKO_OK);
    at.x += 1;
    CHECK(ponko_set(ponko_ctx_world(ctx), d->entity, d->pos, &at, sizeof(at)) == PONKO_OK);
    CHECK(ponko_ctx_send_event(ctx, d->pos, &at, sizeof(at)) == PONKO_OK);
}
static void declared_reader(PonkoCtx* ctx, void* user) {
    Declared* d = user;
    const void* data; size_t count;
    CHECK(ponko_ctx_read_events(ctx, d->pos, sizeof(Position), &data, &count) == PONKO_OK);
    d->hits += count;
}
static void declared_setup(PonkoCtx* ctx, void* user) {
    Declared* d = user;
    PonkoWorld* w = ponko_ctx_world(ctx);
    CHECK(ponko_component_register(w, "DeclPos", sizeof(Position), _Alignof(Position), &d->pos) == PONKO_OK);
    Position start = {0}; PonkoValue v = {d->pos, &start, sizeof(start)};
    CHECK(ponko_spawn(w, &v, 1, &d->entity) == PONKO_OK);
}
static void declared_access(void) {
    PonkoAccess* mover = ponko_access_new(); CHECK(mover);
    CHECK(ponko_access_write_component(mover, "DeclPos") == PONKO_OK);
    CHECK(ponko_access_send_events(mover, "DeclPos") == PONKO_OK);
    CHECK(ponko_access_service(mover, PONKO_SERVICE_TIME, false) == PONKO_OK);
    CHECK(ponko_access_service(mover, 99, false) == PONKO_INVALID_ARGUMENT);
    PonkoAccess* reader = ponko_access_new(); CHECK(reader);
    CHECK(ponko_access_read_events(reader, "DeclPos") == PONKO_OK);
    size_t n = 9;
    CHECK(ponko_access_conflicts(reader, mover, true, &n) == PONKO_OK && n == 0);
    CHECK(ponko_access_conflicts(mover, reader, true, &n) == PONKO_OK && n == 1);
    CHECK(ponko_access_conflicts(reader, mover, false, &n) == PONKO_OK && n == 1);
    PonkoAccess* spawner = ponko_access_new(); CHECK(spawner);
    CHECK(ponko_access_commands(spawner) == PONKO_OK);
    CHECK(ponko_access_conflicts(spawner, mover, true, &n) == PONKO_OK && n == 1);
    CHECK(ponko_access_conflicts(mover, spawner, true, &n) == PONKO_OK && n == 0);
    ponko_access_free(spawner);
    Declared d = {0, 0, 0};
    PonkoGame* g = ponko_game_new("declared", 1, 1); CHECK(g);
    CHECK(ponko_game_add_system_with_access(g, PONKO_PHASE_AFTER_PHYSICS, "mover", 1, mover, declared_mover, &d) == PONKO_OK);
    CHECK(ponko_game_add_system_with_access(g, PONKO_PHASE_AFTER_PHYSICS, "reader", 1, reader, declared_reader, &d) == PONKO_OK);
    CHECK(ponko_game_add_system_with_access(g, PONKO_PHASE_AFTER_PHYSICS, "bad", 0, reader, declared_reader, &d) == PONKO_INVALID_ARGUMENT);
    ponko_access_free(mover);
    ponko_access_free(reader);
    size_t len = 0;
    CHECK(ponko_game_access_report(g, NULL, 0, &len) == PONKO_OK && len > 0);
    char* text = malloc(len + 1);
    CHECK(ponko_game_access_report(g, text, len + 1, &len) == PONKO_OK);
    CHECK(strcmp(text, "mover + reader: conflict on events `DeclPos`\n") == 0);
    free(text);
    PonkoWorld* w = NULL;
    CHECK(ponko_game_run_headless(g, 3, declared_setup, &d, &w) == PONKO_OK && w);
    Position at;
    CHECK(ponko_get(w, d.entity, d.pos, &at, sizeof(at)) == PONKO_OK && at.x == 3);
    CHECK(d.hits == 3);
    CHECK(ponko_world_free(w) == PONKO_OK);
}

int main(void) {
    change_detection();
    declared_access();
    steam_messages_validation();
    morph_clip_api();
    replication();
    CHECK(ponko_abi_version() == 2);
    PonkoWorld* w = ponko_world_new(); CHECK(w);
    uint32_t p; CHECK(ponko_component_register(w, "Pos", sizeof(Position), _Alignof(Position), &p) == PONKO_OK);
    Position initial = {7}; PonkoValue value = {p, &initial, sizeof(initial)};
    uint64_t e; CHECK(ponko_spawn(w, &value, 1, &e) == PONKO_OK);
    Context context = {w, 0, e}; CHECK(ponko_query(w, &p, 1, visit, &context) == PONKO_OK); CHECK(context.seen == 1);
    Position result; CHECK(ponko_get(w, e, p, &result, sizeof(result)) == PONKO_OK && result.x == 9);
    CHECK(ponko_get(w, e, p, &result, 0) == PONKO_INVALID_ARGUMENT);
    uint32_t duplicate[] = {p, p}; CHECK(ponko_query(w, duplicate, 2, visit, &context) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_despawn(w, e) == PONKO_OK); bool alive = true;
    CHECK(ponko_is_alive(w, e, &alive) == PONKO_OK && !alive);
    CHECK(ponko_despawn(w, e) == PONKO_NOT_FOUND);
    CHECK(ponko_world_free(w) == PONKO_OK);

    w = ponko_world_new();
    uint32_t probe_id;
    CHECK(ponko_reflection_probe3d_component(w, &probe_id) == PONKO_OK);
    PonkoReflectionProbe3D probe = ponko_reflection_probe3d_default();
    CHECK(probe.content == 0 && probe.box_projection && probe.size.x == 10);
    PonkoValue probe_value = {probe_id, &probe, sizeof(probe)};
    CHECK(ponko_spawn(w, &probe_value, 1, &e) == PONKO_OK);
    PonkoReflectionProbe3D probe_out;
    CHECK(ponko_get(w, e, probe_id, &probe_out, sizeof(probe_out)) == PONKO_OK && probe_out.blend_distance == 1);
    uint32_t decal_id, receives_id;
    CHECK(ponko_decal3d_component(w, &decal_id) == PONKO_OK);
    CHECK(ponko_receives_decals_component(w, &receives_id) == PONKO_OK);
    PonkoDecal3D decal = ponko_decal3d_default();
    CHECK(decal.opacity == 1 && decal.image == 0 && decal.normal_image == PONKO_DECAL_NO_IMAGE && decal.layers == 0xFFFF);
    decal.size.x = 2; decal.order = 4;
    PonkoReceivesDecals none = {0};
    PonkoValue decal_values[] = {{decal_id, &decal, sizeof(decal)}, {receives_id, &none, sizeof(none)}};
    CHECK(ponko_spawn(w, decal_values, 2, &e) == PONKO_OK);
    PonkoDecal3D decal_out; PonkoReceivesDecals none_out = {1};
    CHECK(ponko_get(w, e, decal_id, &decal_out, sizeof(decal_out)) == PONKO_OK && decal_out.size.x == 2 && decal_out.order == 4);
    CHECK(ponko_get(w, e, receives_id, &none_out, sizeof(none_out)) == PONKO_OK && none_out.layers == 0);
    uint32_t point_id, spot_id;
    CHECK(ponko_point_light3d_component(w, &point_id) == PONKO_OK);
    CHECK(ponko_spot_light3d_component(w, &spot_id) == PONKO_OK);
    PonkoPointLight3D point = {{1, .5f, .1f, 1}, 20, 12, true, 512};
    PonkoSpotLight3D spot = {{.1f, .5f, 1, 1}, 30, 18, .2f, .5f, true};
    PonkoValue lights[] = {{point_id, &point, sizeof(point)}, {spot_id, &spot, sizeof(spot)}};
    CHECK(ponko_spawn(w, lights, 2, &e) == PONKO_OK);
    PonkoPointLight3D point_out;
    PonkoSpotLight3D spot_out;
    CHECK(ponko_get(w, e, point_id, &point_out, sizeof(point_out)) == PONKO_OK && point_out.intensity == 20);
    CHECK(point_out.shadow_resolution == 512 && point_out.shadows);
    CHECK(ponko_get(w, e, spot_id, &spot_out, sizeof(spot_out)) == PONKO_OK && spot_out.outer_angle == .5f);
    CHECK(ponko_world_free(w) == PONKO_OK);

    Run r = {0, 0, PONKO_OK};
    PonkoGame* game = ponko_game_new("c tests", 1, 1); CHECK(game);
    CHECK(ponko_game_fixed_hz(game, 50) == PONKO_OK);
    CHECK(ponko_game_fixed_hz(game, 0) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_game_sim_system(game, "tick", 1, tick, &r) == PONKO_OK);
    PonkoWorld* sim = NULL;
    CHECK(ponko_game_run_headless(game, 100, setup, NULL, &sim) == PONKO_OK);
    CHECK(r.ticks == 3 && r.dt > .019f && r.dt < .021f && sim);
    CHECK(ponko_world_free(sim) == PONKO_OK);
    PonkoDrawParams params = ponko_draw_params_default();
    CHECK(params.scale.x == 1 && params.pivot.x == .5f && !params.screen);
    vehicles();
    phases();
    retained_ui();
    ui_panels();
    ribbons_and_decals();
    lod_helpers();
    streamed_worlds();
    asset_streaming();
    profiling_c();
    animation_graphs();
    return 0;
}
