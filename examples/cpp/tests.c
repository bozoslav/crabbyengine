#include "crabby.h"
#include <stdio.h>
#include <stdlib.h>
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
int main(void) {
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
    CrabbyPointLight3D point = {{1, .5f, .1f, 1}, 20, 12};
    CrabbySpotLight3D spot = {{.1f, .5f, 1, 1}, 30, 18, .2f, .5f};
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
    return 0;
}
