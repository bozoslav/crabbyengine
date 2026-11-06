#include "crabby.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr, "failed: %s\n", #x); abort(); } } while (0)
typedef struct { int x; } Position;
typedef struct { CrabbyWorld* world; size_t seen; uint64_t entity; } Context;
static void visit(const CrabbyChunk* chunk, void* user) {
    Context* c = user;
    CHECK(chunk->column_count == 1);
    for (size_t i = 0; i < chunk->count; ++i) {
        CHECK(crabby_entity_bits(chunk->entities[i]) == c->entity);
        ((Position*)chunk->columns[0])[i].x += 2;
        ++c->seen;
    }
    size_t len;
    CHECK(crabby_world_len(c->world, &len) == CRABBY_BUSY);
    CHECK(crabby_world_free(c->world) == CRABBY_BUSY);
}
typedef struct { size_t ticks; float dt; CrabbyStatus busy_quit; } Run;
static void setup(CrabbyCtx* ctx, void* user) {
    (void)user;
    CrabbyGpuBuffer buffer;
    const float items[2][4] = {{1, 2, 3, 4}, {0, 0, 0, 0}};
    CHECK(crabby_ctx_create_gpu_buffer(ctx, 2, items, 2, &buffer) == CRABBY_OK);
    CHECK(crabby_ctx_write_gpu_buffer(ctx, buffer, 1, items, 1) == CRABBY_OK);
    CHECK(crabby_ctx_write_gpu_buffer(ctx, buffer, 2, items, 1) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_ctx_create_gpu_buffer(ctx, 0, NULL, 0, &buffer) == CRABBY_INVALID_ARGUMENT);
    bool culling = true;
    CHECK(crabby_ctx_occlusion_culling(ctx, &culling) == CRABBY_OK && !culling);
    CHECK(crabby_ctx_set_occlusion_culling(ctx, true) == CRABBY_OK);
    CHECK(crabby_ctx_occlusion_culling(ctx, &culling) == CRABBY_OK && culling);
    CHECK(crabby_ctx_set_occlusion_culling(ctx, false) == CRABBY_OK);
    CHECK(crabby_ctx_occlusion_culling(NULL, &culling) == CRABBY_INVALID_ARGUMENT);
    CrabbyLighting light;
    CHECK(crabby_ctx_lighting(ctx, &light) == CRABBY_OK && !light.has_equator);
    light.has_equator = true;
    light.equator_color = (CrabbyColor){0, 1, 0, 1};
    CHECK(crabby_ctx_set_lighting(ctx, &light) == CRABBY_OK);
    CrabbyLighting lit;
    CHECK(crabby_ctx_lighting(ctx, &lit) == CRABBY_OK && lit.has_equator && lit.equator_color.g == 1);
    light.has_equator = false;
    CHECK(crabby_ctx_set_lighting(ctx, &light) == CRABBY_OK);
    CrabbyPost post;
    CHECK(crabby_ctx_post(ctx, &post) == CRABBY_OK);
    CHECK(!post.ao_enabled && post.anti_aliasing == 0 && post.ao_radius == 1);
    post.ao_enabled = true; post.ao_half_resolution = true; post.ao_temporal = true; post.ao_radius = 2; post.ao_intensity = 1.5f;
    post.ao_slices = 4; post.ao_steps = 8; post.anti_aliasing = 1;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_OK);
    CrabbyPost back;
    CHECK(crabby_ctx_post(ctx, &back) == CRABBY_OK);
    CHECK(back.ao_half_resolution && back.ao_temporal && back.ao_enabled && back.ao_radius == 2 && back.anti_aliasing == 1);
    post.anti_aliasing = 4;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_INVALID_ARGUMENT);
    CHECK(back.render_scale == 1 && back.sharpen == 0);
    post.anti_aliasing = 1;
    post.render_scale = 0.1f;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_INVALID_ARGUMENT);
    post.render_scale = 0.5f; post.sharpen = 0.3f;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_OK);
    CHECK(crabby_ctx_post(ctx, &back) == CRABBY_OK && back.render_scale == 0.5f && back.sharpen == 0.3f);
    CrabbyGraphicsQuality quality, now;
    uint32_t level = 99;
    CHECK(crabby_graphics_quality_preset(4, &quality) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_graphics_quality_preset(0, &quality) == CRABBY_OK);
    CHECK(quality.shadow_cascades == 2 && quality.light_shadow_views == 0 && quality.render_scale < 1);
    CHECK(quality.light_shadow_atlas == 1024);
    CHECK(crabby_graphics_quality_level(&quality, &level) == CRABBY_OK && level == 0);
    CHECK(crabby_ctx_set_graphics_quality(ctx, &quality) == CRABBY_OK);
    CHECK(crabby_ctx_graphics_quality(ctx, &now) == CRABBY_OK && now.shadow_resolution == 1024);
    CHECK(crabby_ctx_post(ctx, &back) == CRABBY_OK && back.render_scale == quality.render_scale);
    quality.shadow_resolution = 100;
    CHECK(crabby_ctx_set_graphics_quality(ctx, &quality) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_graphics_quality_preset(3, &quality) == CRABBY_OK);
    CHECK(crabby_ctx_set_graphics_quality(ctx, &quality) == CRABBY_OK);
    CHECK(crabby_ctx_graphics_quality(ctx, &now) == CRABBY_OK && now.shadow_resolution == 4096 && now.ao_steps == 8 && now.shadow_refresh == 1 && now.ssr == 1);
    CHECK(now.light_shadow_atlas == 8192 && now.light_shadow_views == 24);
    quality.light_shadow_atlas = 3000; /* not a power of two */
    CHECK(crabby_ctx_set_graphics_quality(ctx, &quality) == CRABBY_INVALID_ARGUMENT);
    post.anti_aliasing = 3; /* temporal */
    post.taa_sharpness = 2;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_INVALID_ARGUMENT);
    post.taa_sharpness = 0.5f;
    post.contact_shadows = -1;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_INVALID_ARGUMENT);
    post.contact_shadows = 0.3f;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_OK);
    CHECK(post.ssr == 0 && post.ssr_max_distance == 30); /* screen-space reflections: off */
    post.ssr = 2;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_INVALID_ARGUMENT);
    post.ssr = 0.5f;
    post.ssr_max_distance = 0;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_INVALID_ARGUMENT);
    post.ssr_max_distance = 20;
    CHECK(crabby_ctx_set_post(ctx, &post) == CRABBY_OK);
    CHECK(crabby_ctx_reset_temporal_history(ctx) == CRABBY_OK);
    CHECK(crabby_ctx_post(ctx, &back) == CRABBY_OK && back.anti_aliasing == 3 && back.taa_sharpness == 0.5f && back.contact_shadows == 0.3f && back.ssr == 0.5f && back.ssr_max_distance == 20);
    uint64_t e;
    CHECK(crabby_spawn(crabby_ctx_world(ctx), NULL, 0, &e) == CRABBY_OK);
    CHECK(crabby_world_free(crabby_ctx_world(ctx)) == CRABBY_BUSY);
    CrabbyText3D t = crabby_text3d_default(), got;
    t.billboard = true; t.align = 2;
    CHECK(crabby_ctx_set_text3d(ctx, e, "Hello\nworld", &t) == CRABBY_OK);
    CHECK(crabby_ctx_text3d(ctx, e, &got) == CRABBY_OK && got.billboard && got.align == 2);
    char text[32]; size_t len;
    CHECK(crabby_ctx_text3d_text(ctx, e, text, sizeof(text), &len) == CRABBY_OK);
    CHECK(len == 11 && strcmp(text, "Hello\nworld") == 0);
    t.font.id = UINT32_MAX;
    CHECK(crabby_ctx_set_text3d(ctx, e, "bad", &t) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_ctx_remove_text3d(ctx, e) == CRABBY_OK);
    CHECK(crabby_ctx_text3d(ctx, e, &got) == CRABBY_NOT_FOUND);

    CHECK(crabby_ctx_set_nav_avoidance(ctx, e, true, 2, 3.6f) == CRABBY_NOT_FOUND);
    CHECK(crabby_ctx_set_nav_agent(ctx, e, 3, 0.3f, 10) == CRABBY_OK);
    CrabbyVec3 center = {0, 1, 0}, half = {1, 1, 1};
    CHECK(!crabby_nav_bake_settings_default().carving && crabby_nav_bake_settings_default().tile_size == 32);
    CHECK(crabby_ctx_set_nav_carve_obstacle(ctx, e, center, half) == CRABBY_OK);
    size_t steps = 0; bool pending = false;
    CHECK(crabby_ctx_navmesh_update_carving(ctx, 0, &steps, &pending) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_ctx_set_nav_avoidance(ctx, e, true, 2, 3.6f) == CRABBY_OK);
    CHECK(crabby_ctx_set_nav_avoidance(ctx, e, false, 0, 1) == CRABBY_OK);
    CHECK(crabby_ctx_set_nav_avoidance(ctx, e, true, -1, 3) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_ctx_set_nav_avoidance(ctx, e, true, 2, 0) == CRABBY_INVALID_ARGUMENT);
}
static void tick(CrabbyCtx* ctx, void* user) {
    Run* r = user;
    size_t len;
    CHECK(crabby_world_len(crabby_ctx_world(ctx), &len) == CRABBY_OK && len == 1);
    r->dt = crabby_ctx_dt(ctx);
    if (++r->ticks == 3) CHECK(crabby_ctx_quit(ctx) == CRABBY_OK);
}
/* A box car on four raycast wheels: spawned and driven through the C API,
   the vehicle as a query column. */
typedef struct { uint32_t transform, body, collider, vehicle; uint64_t car; } Cars;
static void ids(CrabbyWorld* w, Cars* c) {
    CHECK(crabby_transform3d_component(w, &c->transform) == CRABBY_OK);
    CHECK(crabby_rigid_body3d_component(w, &c->body) == CRABBY_OK);
    CHECK(crabby_collider3d_component(w, &c->collider) == CRABBY_OK);
    CHECK(crabby_vehicle3d_component(w, &c->vehicle) == CRABBY_OK);
}
static void car_setup(CrabbyCtx* ctx, void* user) {
    Cars* c = user;
    CrabbyWorld* w = crabby_ctx_world(ctx);
    ids(w, c);
    CrabbyTransform3D floor_at = crabby_transform3d_at((CrabbyVec3){0, -0.5f, 0});
    CrabbyRigidBody3D fixed = crabby_rigid_body3d(CRABBY_BODY_FIXED);
    CrabbyCollider3D floor = crabby_collider3d_box((CrabbyVec3){200, 1, 200});
    CrabbyValue ground[] = {{c->transform, &floor_at, sizeof floor_at}, {c->body, &fixed, sizeof fixed},
                            {c->collider, &floor, sizeof floor}};
    uint64_t e; CHECK(crabby_spawn(w, ground, 3, &e) == CRABBY_OK);
    CrabbyTransform3D at = crabby_transform3d_at((CrabbyVec3){0, 0.8f, 0});
    CrabbyRigidBody3D chassis = crabby_rigid_body3d(CRABBY_BODY_DYNAMIC);
    chassis.mass = 1200;
    chassis.has_center_of_mass = true;
    chassis.center_of_mass = (CrabbyVec3){0, -0.3f, 0};
    CrabbyCollider3D box = crabby_collider3d_box((CrabbyVec3){1.8f, 0.6f, 4.2f});
    CrabbyVehicle3D car = crabby_vehicle3d();
    const float x[] = {-0.8f, 0.8f, -0.8f, 0.8f}, z[] = {-1.3f, -1.3f, 1.3f, 1.3f};
    for (int i = 0; i < 4; ++i)
        CHECK(crabby_vehicle3d_add_wheel(&car, crabby_wheel3d((CrabbyVec3){x[i], -0.3f, z[i]}, 0.35f)) == CRABBY_OK);
    CrabbyValue values[] = {{c->transform, &at, sizeof at}, {c->body, &chassis, sizeof chassis},
                            {c->collider, &box, sizeof box}, {c->vehicle, &car, sizeof car}};
    CHECK(crabby_spawn(w, values, 4, &c->car) == CRABBY_OK);
}
static void throttle(const CrabbyChunk* chunk, void* user) {
    (void)user;
    CrabbyVehicle3D* cars = chunk->columns[0];
    for (size_t i = 0; i < chunk->count; ++i) {
        cars[i].wheels[2].motor_torque = 500;
        cars[i].wheels[3].motor_torque = 500;
    }
}
static void drive(CrabbyCtx* ctx, void* user) {
    Cars* c = user;
    if (crabby_ctx_sim_tick(ctx) > 25) CHECK(crabby_query(crabby_ctx_world(ctx), &c->vehicle, 1, throttle, NULL) == CRABBY_OK);
}
static void vehicles(void) {
    Cars c = {0};
    CrabbyGame* game = crabby_game_new("c vehicles", 1, 1); CHECK(game);
    CHECK(crabby_game_sim_system(game, "drive", 1, drive, &c) == CRABBY_OK);
    CHECK(crabby_game_physics3d(game, (CrabbyVec3){0, -9.81f, 0}) == CRABBY_OK);
    CrabbyWorld* w = NULL;
    CHECK(crabby_game_run_headless(game, 150, car_setup, &c, &w) == CRABBY_OK);
    CrabbyVehicle3D car; CrabbyTransform3D t;
    CHECK(crabby_get(w, c.car, c.vehicle, &car, sizeof car) == CRABBY_OK);
    CHECK(crabby_get(w, c.car, c.transform, &t, sizeof t) == CRABBY_OK);
    CHECK(car.wheel_count == 4 && car.wheels[0].grounded && car.wheels[3].angular_velocity > 0);
    CHECK(t.position.z < -3 && t.position.y > 0.7f && t.position.y < 0.9f);
    CrabbyTransform3D wheel = crabby_wheel3d_pose(&car.wheels[0], t);
    CHECK(wheel.position.y > 0.3f && wheel.position.y < 0.4f);
    CHECK(crabby_friction_curve_evaluate(crabby_friction_curve_sideways(), 0.2f) == 1);
    CHECK(crabby_world_free(w) == CRABBY_OK);
}
/* The retained UI: a column with a button, laid out and clicked from C. */
static void ui_setup(CrabbyCtx* ctx, void* user) {
    (void)user;
    CrabbyUiLayout column = crabby_ui_layout_default();
    column.flex_direction = CRABBY_UI_FLEX_COLUMN;
    column.width.kind = CRABBY_UI_LENGTH_POINTS; column.width.value = 300;
    column.height.kind = CRABBY_UI_LENGTH_POINTS; column.height.value = 200;
    uint64_t root, button, label;
    CHECK(crabby_ctx_ui_add_panel(ctx, CRABBY_UI_SCREEN, CRABBY_UI_NONE, &column, &root) == CRABBY_OK);
    CrabbyUiLayout fixed = crabby_ui_layout_default();
    fixed.width.kind = CRABBY_UI_LENGTH_POINTS; fixed.width.value = 120;
    fixed.height.kind = CRABBY_UI_LENGTH_POINTS; fixed.height.value = 40;
    CHECK(crabby_ctx_ui_add_label(ctx, CRABBY_UI_SCREEN, root, "Shop", &fixed, &label) == CRABBY_OK);
    CHECK(crabby_ctx_ui_add_button(ctx, CRABBY_UI_SCREEN, root, "Buy", &fixed, &button) == CRABBY_OK);
    CHECK(crabby_ctx_ui_push_screen(ctx, CRABBY_UI_SCREEN, root, false) == CRABBY_OK);
    CrabbyVec2 window = {800, 600};
    CHECK(crabby_ctx_ui_frame(ctx, CRABBY_UI_SCREEN, window, NULL) == CRABBY_OK);
    CrabbyRect r;
    CHECK(crabby_ctx_ui_rect(ctx, CRABBY_UI_SCREEN, button, &r) == CRABBY_OK && r.min.y == 40 && r.max.x == 120);
    char text[8]; size_t len;
    CHECK(crabby_ctx_ui_text(ctx, CRABBY_UI_SCREEN, button, text, sizeof text, &len) == CRABBY_OK && len == 3 && text[0] == 'B');
    CrabbyUiInput in = crabby_ui_input_default();
    in.pointer.x = 10; in.pointer.y = 50; in.pressed = true; in.held = true;
    CHECK(crabby_ctx_ui_frame(ctx, CRABBY_UI_SCREEN, window, &in) == CRABBY_OK);
    in.pressed = false; in.held = false; in.released = true;
    CHECK(crabby_ctx_ui_frame(ctx, CRABBY_UI_SCREEN, window, &in) == CRABBY_OK);
    CrabbyUiEvent event;
    CHECK(crabby_ctx_ui_next_event(ctx, CRABBY_UI_SCREEN, &event) == CRABBY_OK);
    CHECK(event.kind == CRABBY_UI_EVENT_CLICKED && event.id == button);
    CHECK(crabby_ctx_ui_next_event(ctx, CRABBY_UI_SCREEN, &event) == CRABBY_NOT_FOUND);
    /* The original flat track arrays remain valid; templates add repeat fragments. */
    CrabbyUiTrack repeated_track = {CRABBY_UI_TRACK_FR, 1};
    CrabbyUiTemplateTrack repeated = {0};
    repeated.repetition = 1; repeated.count = 2;
    repeated.tracks = &repeated_track; repeated.track_count = 1;
    CrabbyUiTrack rows[] = {{CRABBY_UI_TRACK_POINTS, 40}, {CRABBY_UI_TRACK_POINTS, 160}};
    CrabbyUiLayout named = crabby_ui_layout_default();
    named.display = CRABBY_UI_DISPLAY_GRID;
    named.width = (CrabbyUiLength){CRABBY_UI_LENGTH_POINTS, 300};
    named.height = (CrabbyUiLength){CRABBY_UI_LENGTH_POINTS, 200};
    named.grid_template_columns = &repeated; named.grid_template_column_count = 1;
    named.grid_rows = rows; named.grid_row_count = 2;
    uint64_t grid, main;
    CHECK(crabby_ctx_ui_add_panel(ctx, CRABBY_UI_SCREEN, CRABBY_UI_NONE, &named, &grid) == CRABBY_OK);
    const char* areas[] = {"header header", "side main"};
    CHECK(crabby_ctx_ui_set_grid_template_areas(ctx, CRABBY_UI_SCREEN, grid, areas, 2) == CRABBY_OK);
    CHECK(crabby_ctx_ui_add_panel(ctx, CRABBY_UI_SCREEN, grid, NULL, &main) == CRABBY_OK);
    CHECK(crabby_ctx_ui_set_grid_area(ctx, CRABBY_UI_SCREEN, main, "main") == CRABBY_OK);
    CHECK(crabby_ctx_ui_push_screen(ctx, CRABBY_UI_SCREEN, grid, false) == CRABBY_OK);
    CHECK(crabby_ctx_ui_layout(ctx, CRABBY_UI_SCREEN, (CrabbyVec2){800, 600}) == CRABBY_OK);
    CrabbyRect main_rect;
    CHECK(crabby_ctx_ui_rect(ctx, CRABBY_UI_SCREEN, main, &main_rect) == CRABBY_OK);
    CHECK(main_rect.min.x == 150 && main_rect.min.y == 40 && main_rect.max.x == 300 && main_rect.max.y == 200);
    const char* invalid_areas[] = {"x x", "x y"};
    CHECK(crabby_ctx_ui_set_grid_template_areas(ctx, CRABBY_UI_SCREEN, grid, invalid_areas, 2) == CRABBY_INVALID_ARGUMENT);
    const char* names[] = {"start", "finish"};
    CHECK(crabby_ctx_ui_set_grid_line_names(ctx, CRABBY_UI_SCREEN, grid, 0, names, 2) == CRABBY_OK);
    CHECK(crabby_ctx_ui_set_grid_named_line(ctx, CRABBY_UI_SCREEN, main, 0, false, "start", 1) == CRABBY_OK);
    CHECK(crabby_ctx_ui_set_grid_named_line(ctx, CRABBY_UI_SCREEN, main, 0, true, "finish", 1) == CRABBY_OK);
    CHECK(crabby_ctx_ui_layout(ctx, CRABBY_UI_SCREEN, (CrabbyVec2){800, 600}) == CRABBY_OK);
    CHECK(crabby_ctx_ui_rect(ctx, CRABBY_UI_SCREEN, main, &main_rect) == CRABBY_OK);
    CHECK(main_rect.min.x == 0 && main_rect.max.x == 300);
    column.display = 99; /* not a CrabbyUiDisplay */
    uint64_t bad;
    CHECK(crabby_ctx_ui_add_panel(ctx, CRABBY_UI_SCREEN, root, &column, &bad) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_ctx_ui_remove(ctx, CRABBY_UI_SCREEN, root) == CRABBY_OK);
    CHECK(crabby_ctx_ui_remove(ctx, CRABBY_UI_SCREEN, button) == CRABBY_NOT_FOUND);
}
/* A world-space UI panel: its UI takes the same calls, with the entity as the target. */
static void panel_setup(CrabbyCtx* ctx, void* user) {
    (void)user;
    CrabbyWorld* w = crabby_ctx_world(ctx);
    uint32_t transform; uint64_t entity;
    CHECK(crabby_transform3d_component(w, &transform) == CRABBY_OK);
    CrabbyTransform3D at = crabby_transform3d_at((CrabbyVec3){0, 1.5f, -2});
    CrabbyValue value = {transform, &at, sizeof at};
    CHECK(crabby_spawn(w, &value, 1, &entity) == CRABBY_OK);
    CrabbyTexture texture;
    CHECK(crabby_ctx_create_ui_panel(ctx, entity, 600, 400, (CrabbyVec2){1.2f, 0.8f}, &texture) == CRABBY_OK);
    CHECK(texture.width == 600 && texture.height == 400);
    CrabbyUiLayout fixed = crabby_ui_layout_default();
    fixed.width.kind = CRABBY_UI_LENGTH_POINTS; fixed.width.value = 200;
    fixed.height.kind = CRABBY_UI_LENGTH_POINTS; fixed.height.value = 60;
    uint64_t button, top;
    CHECK(crabby_ctx_ui_add_button(ctx, entity, CRABBY_UI_NONE, "Buy", &fixed, &button) == CRABBY_OK);
    CHECK(crabby_ctx_ui_push_screen(ctx, entity, button, false) == CRABBY_OK);
    CHECK(crabby_ctx_ui_top_screen(ctx, CRABBY_UI_SCREEN, &top) == CRABBY_OK && top == CRABBY_UI_NONE);
    CHECK(crabby_ctx_ui_top_screen(ctx, entity, &top) == CRABBY_OK && top == button);
    CrabbyVec2 pixels = {600, 400};
    CHECK(crabby_ctx_ui_layout(ctx, entity, pixels) == CRABBY_OK);
    CrabbyRect r;
    CHECK(crabby_ctx_ui_rect(ctx, entity, button, &r) == CRABBY_OK && r.max.x == 200 && r.max.y == 60);
    bool over; CrabbyVec2 pixel;
    CHECK(crabby_ctx_ui_panel_pointer(ctx, entity, &over, &pixel) == CRABBY_OK && !over);
    CHECK(crabby_ctx_set_ui_panel_interactive(ctx, entity, false) == CRABBY_OK);
    CHECK(crabby_ctx_set_ui_pointer_ray(ctx, (CrabbyVec3){0, 1.5f, 0}, (CrabbyVec3){0, 0, -1}) == CRABBY_OK);
    CHECK(crabby_ctx_set_ui_pointer_ray(ctx, (CrabbyVec3){0, 0, 0}, (CrabbyVec3){0, 0, 0}) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_ctx_clear_ui_pointer_ray(ctx) == CRABBY_OK);
    CHECK(crabby_ctx_remove_ui_panel(ctx, entity) == CRABBY_OK);
    CHECK(crabby_ctx_ui_top_screen(ctx, entity, &top) == CRABBY_NOT_FOUND);
}
static void ui_panels(void) {
    CrabbyGame* game = crabby_game_new("c panels", 1, 1); CHECK(game);
    CrabbyWorld* w = NULL;
    CHECK(crabby_game_run_headless(game, 0, panel_setup, NULL, &w) == CRABBY_OK);
    CHECK(crabby_world_free(w) == CRABBY_OK);
}
static void retained_ui(void) {
    CrabbyGame* game = crabby_game_new("c ui", 1, 1); CHECK(game);
    CrabbyWorld* w = NULL;
    CHECK(crabby_game_run_headless(game, 0, ui_setup, NULL, &w) == CRABBY_OK);
    CHECK(crabby_world_free(w) == CRABBY_OK);
}
/* Ribbon and line defaults, and a decal cut from a floor. */
static void ribbons_and_decals(void) {
    CrabbyTrail3D trail = crabby_trail3d_new(0.25f, 0.05f);
    CHECK(trail.lifetime == 0.25f && trail.width == 0.05f && trail.max_points == 64 && trail.emitting && !trail.has_texture);
    CrabbyLine3D line = crabby_line3d_new(0.1f, (CrabbyColor){1, 0, 0, 1});
    CHECK(line.width == 0.1f && line.point_count == 0 && line.points == NULL && line.texture_length == 0);
    CrabbyMeshData* floor = crabby_mesh_data_plane(4);
    float down[16] = {0.5f, 0, 0, 0, 0, 0, -0.2f, 0, 0, 0.5f, 0, 0, 1, 0, 0, 1};
    CrabbyMeshData* mark = crabby_mesh_data_decal(floor, down, 0.002f);
    CHECK(mark);
    CrabbyMeshView view;
    CHECK(crabby_mesh_data_view(mark, &view) == CRABBY_OK && view.index_count >= 3);
    crabby_mesh_data_free(mark);
    CHECK(crabby_mesh_data_decal(floor, NULL, 0.002f) == NULL);
    crabby_mesh_data_free(floor);
}
static void lod_helpers(void) {
    float fov = 60 * 3.14159265f / 180;
    float h = crabby_lod_screen_height_at(2, 10, fov);
    CHECK(h > 0.172f && h < 0.174f);
    float d = crabby_lod_distance_at(2, h, fov);
    CHECK(d > 9.99f && d < 10.01f);
    CrabbyLodRenderer r = crabby_lod_renderer((CrabbyMesh){3});
    CHECK(r.mesh.id == 3 && r.color.r == 1 && r.offset.scale.y == 1);
    CrabbyLodLevel level = {0.1f, 0, &r, 1, false, NULL, 0};
    CHECK(crabby_ctx_set_lod_group(NULL, 0, 1, (CrabbyVec3){0, 0, 0}, &level, 1) == CRABBY_INVALID_ARGUMENT);
}
static void steam_messages_validation(void) {
    CrabbyNetMessages* endpoint = NULL;
    CrabbySteamLobby* lobby = NULL;
    CHECK(crabby_net_messages_steam_host(480, 0, &endpoint) == CRABBY_INVALID_ARGUMENT && !endpoint);
    CHECK(crabby_net_messages_steam_join(480, 0, &endpoint) == CRABBY_INVALID_ARGUMENT && !endpoint);
    CHECK(crabby_steam_lobby_create(480, 3, 2, &lobby) == CRABBY_INVALID_ARGUMENT && !lobby);
    CHECK(crabby_steam_lobby_join(480, 0, &lobby) == CRABBY_INVALID_ARGUMENT && !lobby);
    CHECK(crabby_net_messages_steam_host(480, 2, &endpoint) != CRABBY_OK && !endpoint);
    CHECK(crabby_steam_lobby_create(480, 0, 2, &lobby) != CRABBY_OK && !lobby);
    CHECK(crabby_steam_lobby_poll(NULL) != CRABBY_OK);
    CHECK(crabby_steam_lobby_leave(NULL) != CRABBY_OK);
    CHECK(crabby_steam_lobby_set_metadata(NULL, "version", "1") != CRABBY_OK);
    crabby_steam_lobby_free(NULL);
}

static void morph_clip_api(void) {
    CrabbyClip* clip = NULL;
    CHECK(crabby_clip_new("face", &clip) == CRABBY_OK);
    float times[] = {0, 2}, values[] = {0, 1};
    CHECK(crabby_clip_set_morph_track(clip, "smile", times, values, 2, 1) == CRABBY_OK);
    CHECK(crabby_clip_duration(clip) == 2);
    float cubic[] = {0, 0, 1, 1, 1, 0};
    CHECK(crabby_clip_set_morph_track(clip, "smile", times, cubic, 2, 2) == CRABBY_OK);
    CHECK(crabby_clip_set_morph_track(clip, "smile", times, values, 2, 9) == CRABBY_INVALID_ARGUMENT);
    crabby_clip_free(clip);
    CrabbyMeshData* mesh = crabby_mesh_data_plane(1);
    CrabbyVec3 deltas[4] = {{0, 1, 0}, {0, 1, 0}, {0, 1, 0}, {0, 1, 0}};
    CHECK(crabby_mesh_data_add_morph_target(mesh, "full", deltas, NULL, 4) == CRABBY_OK);
    CHECK(crabby_mesh_data_add_morph_target(mesh, "half", deltas, NULL, 4) == CRABBY_OK);
    CHECK(crabby_mesh_data_add_morph_frame(mesh, 0, 1, 0.5f) == CRABBY_OK);
    CHECK(crabby_mesh_data_add_morph_frame(mesh, 0, 0, 1) == CRABBY_OK);
    CHECK(crabby_mesh_data_add_morph_frame(mesh, 0, 0, 1) == CRABBY_INVALID_ARGUMENT);
    crabby_mesh_data_free(mesh);
}
/* Replication helpers: sequence numbers, authority, snapshots, interest. */
static void replication(void) {
    CHECK(crabby_seq_newer(1, 0) && crabby_seq_newer(0, 65535) && !crabby_seq_newer(7, 7));
    CHECK(!crabby_seq_newer(65535, 0) && crabby_seq_diff(1, 65535) == 2 && crabby_seq_diff(65535, 1) == -2);
    CrabbyUpdateStamp old = {0, 60000}, next = {1, 0};
    CHECK(crabby_stamp_newer(next, old) && !crabby_stamp_newer(old, next) && !crabby_stamp_newer(old, old));

    CrabbyAuthority a = crabby_authority_new(0);
    uint16_t epoch = 99;
    bool changed = false;
    CrabbyAuthorityCheck check;
    CHECK(crabby_authority_check(&a, 0, 0, &check) == CRABBY_OK && check == CRABBY_AUTHORITY_ACCEPT);
    CHECK(crabby_authority_handoff(&a, 1, &epoch) == CRABBY_OK && epoch == 1 && a.owner == 1);
    CHECK(crabby_authority_check(&a, 0, 0, &check) == CRABBY_OK && check == CRABBY_AUTHORITY_STALE);
    CHECK(crabby_authority_check(&a, 0, 1, &check) == CRABBY_OK && check == CRABBY_AUTHORITY_WRONG_OWNER);
    CHECK(crabby_authority_check(&a, 1, 2, &check) == CRABBY_OK && check == CRABBY_AUTHORITY_AHEAD);
    CHECK(crabby_authority_adopt(&a, 2, 1, &changed) == CRABBY_OK && !changed && a.owner == 1); /* race: lower wins */
    CHECK(crabby_authority_adopt(&a, 3, 2, &changed) == CRABBY_OK && changed && a.owner == 3 && a.epoch == 2);
    CHECK(crabby_authority_adopt(NULL, 1, 1, &changed) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_authority_check(&a, 1, 1, NULL) == CRABBY_INVALID_ARGUMENT);

    CrabbyLatestWins* filter = crabby_latest_wins_new();
    bool ok = false;
    CrabbyUpdateStamp s1 = {0, 1}, s3 = {0, 3};
    CHECK(crabby_latest_wins_accept(filter, 9, s3, &ok) == CRABBY_OK && ok);
    CHECK(crabby_latest_wins_accept(filter, 9, s1, &ok) == CRABBY_OK && !ok);
    CHECK(crabby_latest_wins_accept(filter, 9, s3, &ok) == CRABBY_OK && !ok);
    CHECK(crabby_latest_wins_accept(filter, 10, s1, &ok) == CRABBY_OK && ok && crabby_latest_wins_len(filter) == 2);
    CHECK(crabby_latest_wins_forget(filter, 9) == CRABBY_OK);
    CHECK(crabby_latest_wins_accept(filter, 9, s1, &ok) == CRABBY_OK && ok);
    CHECK(crabby_latest_wins_clear(filter) == CRABBY_OK && crabby_latest_wins_len(filter) == 0);
    CHECK(crabby_latest_wins_accept(NULL, 1, s1, &ok) == CRABBY_INVALID_ARGUMENT);
    crabby_latest_wins_free(filter);
    crabby_latest_wins_free(NULL);

    /* A sender 5000 s ahead walks along x at 20 units/s, 20 snapshots a second. */
    CrabbySnapshotStream* stream = NULL;
    CHECK(crabby_snapshot_stream_new(NULL, &stream) == CRABBY_OK && stream);
    CrabbyPose pose = {0}, shown = {0};
    pose.rotation.w = 1;
    pose.has_velocity = true;
    pose.velocity.x = 20;
    CrabbySnapshotState state;
    CrabbySnapshotPush result;
    bool found = true;
    CHECK(crabby_snapshot_stream_sample(stream, 0, &shown, &state, &found) == CRABBY_OK && !found);
    for (int n = 0; n < 20; n++) {
        pose.position.x = (float)n;
        CHECK(crabby_snapshot_stream_push(stream, 5000 + n * 0.05, n * 0.05 + 0.03, &pose, &result) == CRABBY_OK);
        CHECK(result == CRABBY_SNAPSHOT_ADDED);
    }
    CHECK(crabby_snapshot_stream_push(stream, 5000, 1.0, &pose, &result) == CRABBY_OK && result == CRABBY_SNAPSHOT_DUPLICATE);
    CHECK(crabby_snapshot_stream_push(stream, NAN, 1.0, &pose, &result) == CRABBY_OK && result == CRABBY_SNAPSHOT_INVALID);
    CHECK(crabby_snapshot_stream_len(stream) == 20);
    CHECK(crabby_snapshot_stream_sample(stream, 1.0, &shown, &state, &found) == CRABBY_OK && found);
    CHECK(state == CRABBY_SNAPSHOT_INTERPOLATED && shown.position.x > 17.3f && shown.position.x < 17.5f && shown.has_velocity);
    /* Packets stop: predicted for 250 ms, then held where the prediction ended. */
    CHECK(crabby_snapshot_stream_sample(stream, 1.2, &shown, &state, &found) == CRABBY_OK && state == CRABBY_SNAPSHOT_EXTRAPOLATED);
    CHECK(crabby_snapshot_stream_sample(stream, 5.0, &shown, &state, &found) == CRABBY_OK && state == CRABBY_SNAPSHOT_HELD);
    CHECK(shown.position.x > 23.9f && shown.position.x < 24.1f); /* 19 + 0.25 s * 20 */
    double offset = 0;
    CHECK(crabby_snapshot_stream_clock_offset(stream, &offset, &found) == CRABBY_OK && found && offset < -4999.9 && offset > -5000.0);
    CHECK(crabby_snapshot_stream_set_delay(stream, 0.0) == CRABBY_OK);
    CHECK(crabby_snapshot_stream_set_delay(stream, -1.0) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_snapshot_stream_clear(stream) == CRABBY_OK && crabby_snapshot_stream_len(stream) == 0);
    CHECK(crabby_snapshot_stream_sample(stream, 1.0, &shown, NULL, &found) == CRABBY_OK && !found);
    CHECK(crabby_snapshot_stream_reset(stream) == CRABBY_OK);
    CHECK(crabby_snapshot_stream_clock_offset(stream, &offset, &found) == CRABBY_OK && !found);
    CHECK(crabby_snapshot_stream_push(stream, 1, 1, NULL, &result) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_snapshot_stream_sample(NULL, 1.0, &shown, &state, &found) == CRABBY_INVALID_ARGUMENT);
    crabby_snapshot_stream_free(stream);
    crabby_snapshot_stream_free(NULL);
    CrabbySnapshotConfig bad = crabby_snapshot_config_default();
    bad.capacity = 1;
    CHECK(crabby_snapshot_stream_new(&bad, &stream) == CRABBY_INVALID_ARGUMENT);
    bad = crabby_snapshot_config_default();
    bad.delay = -1;
    CHECK(crabby_snapshot_stream_new(&bad, &stream) == CRABBY_INVALID_ARGUMENT);

    CrabbyInterest* interest = NULL;
    CrabbyInterestConfig config = crabby_interest_config_default();
    CHECK(config.tier_count == 2 && config.tiers[0].radius == 40 && config.tiers[1].every == 4);
    CHECK(crabby_interest_new(&config, &interest) == CRABBY_OK);
    CrabbyInterestViewer viewer = {1, {0, 0, 0}};
    CrabbyInterestEntity world[3] = {{10, {10, 0, 0}}, {20, {100, 0, 0}}, {30, {900, 0, 0}}};
    CHECK(crabby_interest_update(interest, 0, &viewer, 1, world, 3) == CRABBY_OK);
    CrabbyInterestEntry entries[4];
    size_t count = 0;
    CHECK(crabby_interest_entries(interest, 1, entries, 4, &count) == CRABBY_OK && count == 2);
    CHECK(entries[0].entity == 10 && entries[0].tier == 0 && entries[0].send);
    CHECK(entries[1].entity == 20 && entries[1].tier == 1 && entries[1].send && entries[1].distance == 100);
    CrabbyInterestChange changes[4];
    CHECK(crabby_interest_changes(interest, changes, 4, &count) == CRABBY_OK && count == 2);
    CHECK(changes[0].kind == CRABBY_INTEREST_ENTERED && changes[0].peer == 1 && changes[0].entity == 10);
    /* The mid entity wobbles across the edge of the near ring (40): hysteresis keeps it in its ring. */
    for (uint64_t tick = 1; tick < 20; tick++) {
        world[1].position.x = 40 + (tick % 2 ? 3.0f : -3.0f);
        CHECK(crabby_interest_update(interest, tick, &viewer, 1, world, 2) == CRABBY_OK);
        CHECK(crabby_interest_entries(interest, 1, entries, 4, &count) == CRABBY_OK && count == 2 && entries[1].tier == 1);
        CHECK(crabby_interest_changes(interest, changes, 4, &count) == CRABBY_OK && count == 0);
    }
    world[1].position.x = 500;
    CHECK(crabby_interest_update(interest, 20, &viewer, 1, world, 2) == CRABBY_OK);
    CHECK(crabby_interest_changes(interest, changes, 4, &count) == CRABBY_OK && count == 1);
    CHECK(changes[0].kind == CRABBY_INTEREST_LEFT && changes[0].entity == 20);
    CHECK(crabby_interest_entries(interest, 99, NULL, 0, &count) == CRABBY_OK && count == 0);
    CHECK(crabby_interest_remove_peer(interest, 1) == CRABBY_OK);
    CHECK(crabby_interest_update(interest, 21, NULL, 0, NULL, 0) == CRABBY_OK);
    CHECK(crabby_interest_update(interest, 21, NULL, 1, NULL, 0) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_interest_entries(NULL, 1, NULL, 0, &count) == CRABBY_INVALID_ARGUMENT);
    crabby_interest_free(interest);
    interest = NULL;
    crabby_interest_free(NULL);
    config.tiers[1].radius = 10; /* not increasing */
    CHECK(crabby_interest_new(&config, &interest) == CRABBY_INVALID_ARGUMENT && !interest);
    config = crabby_interest_config_default();
    config.tier_count = 0;
    CHECK(crabby_interest_new(&config, &interest) == CRABBY_INVALID_ARGUMENT);
    config.tier_count = 9;
    CHECK(crabby_interest_new(&config, &interest) == CRABBY_INVALID_ARGUMENT);
    config = crabby_interest_config_default();
    config.hysteresis = 40;
    CHECK(crabby_interest_new(&config, &interest) == CRABBY_INVALID_ARGUMENT);
}

int main(void) {
    steam_messages_validation();
    morph_clip_api();
    replication();
    CHECK(crabby_abi_version() == 2);
    CrabbyWorld* w = crabby_world_new(); CHECK(w);
    uint32_t p; CHECK(crabby_component_register(w, "Pos", sizeof(Position), _Alignof(Position), &p) == CRABBY_OK);
    Position initial = {7}; CrabbyValue value = {p, &initial, sizeof(initial)};
    uint64_t e; CHECK(crabby_spawn(w, &value, 1, &e) == CRABBY_OK);
    Context context = {w, 0, e}; CHECK(crabby_query(w, &p, 1, visit, &context) == CRABBY_OK); CHECK(context.seen == 1);
    Position result; CHECK(crabby_get(w, e, p, &result, sizeof(result)) == CRABBY_OK && result.x == 9);
    CHECK(crabby_get(w, e, p, &result, 0) == CRABBY_INVALID_ARGUMENT);
    uint32_t duplicate[] = {p, p}; CHECK(crabby_query(w, duplicate, 2, visit, &context) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_despawn(w, e) == CRABBY_OK); bool alive = true;
    CHECK(crabby_is_alive(w, e, &alive) == CRABBY_OK && !alive);
    CHECK(crabby_despawn(w, e) == CRABBY_NOT_FOUND);
    CHECK(crabby_world_free(w) == CRABBY_OK);

    w = crabby_world_new();
    uint32_t probe_id;
    CHECK(crabby_reflection_probe3d_component(w, &probe_id) == CRABBY_OK);
    CrabbyReflectionProbe3D probe = crabby_reflection_probe3d_default();
    CHECK(probe.content == 0 && probe.box_projection && probe.size.x == 10);
    CrabbyValue probe_value = {probe_id, &probe, sizeof(probe)};
    CHECK(crabby_spawn(w, &probe_value, 1, &e) == CRABBY_OK);
    CrabbyReflectionProbe3D probe_out;
    CHECK(crabby_get(w, e, probe_id, &probe_out, sizeof(probe_out)) == CRABBY_OK && probe_out.blend_distance == 1);
    uint32_t decal_id, receives_id;
    CHECK(crabby_decal3d_component(w, &decal_id) == CRABBY_OK);
    CHECK(crabby_receives_decals_component(w, &receives_id) == CRABBY_OK);
    CrabbyDecal3D decal = crabby_decal3d_default();
    CHECK(decal.opacity == 1 && decal.image == 0 && decal.normal_image == CRABBY_DECAL_NO_IMAGE && decal.layers == 0xFFFF);
    decal.size.x = 2; decal.order = 4;
    CrabbyReceivesDecals none = {0};
    CrabbyValue decal_values[] = {{decal_id, &decal, sizeof(decal)}, {receives_id, &none, sizeof(none)}};
    CHECK(crabby_spawn(w, decal_values, 2, &e) == CRABBY_OK);
    CrabbyDecal3D decal_out; CrabbyReceivesDecals none_out = {1};
    CHECK(crabby_get(w, e, decal_id, &decal_out, sizeof(decal_out)) == CRABBY_OK && decal_out.size.x == 2 && decal_out.order == 4);
    CHECK(crabby_get(w, e, receives_id, &none_out, sizeof(none_out)) == CRABBY_OK && none_out.layers == 0);
    uint32_t point_id, spot_id;
    CHECK(crabby_point_light3d_component(w, &point_id) == CRABBY_OK);
    CHECK(crabby_spot_light3d_component(w, &spot_id) == CRABBY_OK);
    CrabbyPointLight3D point = {{1, .5f, .1f, 1}, 20, 12, true, 512};
    CrabbySpotLight3D spot = {{.1f, .5f, 1, 1}, 30, 18, .2f, .5f, true};
    CrabbyValue lights[] = {{point_id, &point, sizeof(point)}, {spot_id, &spot, sizeof(spot)}};
    CHECK(crabby_spawn(w, lights, 2, &e) == CRABBY_OK);
    CrabbyPointLight3D point_out;
    CrabbySpotLight3D spot_out;
    CHECK(crabby_get(w, e, point_id, &point_out, sizeof(point_out)) == CRABBY_OK && point_out.intensity == 20);
    CHECK(point_out.shadow_resolution == 512 && point_out.shadows);
    CHECK(crabby_get(w, e, spot_id, &spot_out, sizeof(spot_out)) == CRABBY_OK && spot_out.outer_angle == .5f);
    CHECK(crabby_world_free(w) == CRABBY_OK);

    Run r = {0, 0, CRABBY_OK};
    CrabbyGame* game = crabby_game_new("c tests", 1, 1); CHECK(game);
    CHECK(crabby_game_fixed_hz(game, 50) == CRABBY_OK);
    CHECK(crabby_game_fixed_hz(game, 0) == CRABBY_INVALID_ARGUMENT);
    CHECK(crabby_game_sim_system(game, "tick", 1, tick, &r) == CRABBY_OK);
    CrabbyWorld* sim = NULL;
    CHECK(crabby_game_run_headless(game, 100, setup, NULL, &sim) == CRABBY_OK);
    CHECK(r.ticks == 3 && r.dt > .019f && r.dt < .021f && sim);
    CHECK(crabby_world_free(sim) == CRABBY_OK);
    CrabbyDrawParams params = crabby_draw_params_default();
    CHECK(params.scale.x == 1 && params.pivot.x == .5f && !params.screen);
    vehicles();
    retained_ui();
    ui_panels();
    ribbons_and_decals();
    lod_helpers();
    return 0;
}
