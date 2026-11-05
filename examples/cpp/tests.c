#include "crabby.h"
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

int main(void) {
    steam_messages_validation();
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
    uint32_t point_id, spot_id;
    CHECK(crabby_point_light3d_component(w, &point_id) == CRABBY_OK);
    CHECK(crabby_spot_light3d_component(w, &spot_id) == CRABBY_OK);
    CrabbyPointLight3D point = {{1, .5f, .1f, 1}, 20, 12, false};
    CrabbySpotLight3D spot = {{.1f, .5f, 1, 1}, 30, 18, .2f, .5f, true};
    CrabbyValue lights[] = {{point_id, &point, sizeof(point)}, {spot_id, &spot, sizeof(spot)}};
    CHECK(crabby_spawn(w, lights, 2, &e) == CRABBY_OK);
    CrabbyPointLight3D point_out;
    CrabbySpotLight3D spot_out;
    CHECK(crabby_get(w, e, point_id, &point_out, sizeof(point_out)) == CRABBY_OK && point_out.intensity == 20);
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
    return 0;
}
