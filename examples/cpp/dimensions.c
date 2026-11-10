#include "ponko.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr, "failed line %d: %s\n", __LINE__, #x); abort(); } } while (0)
typedef struct { uint64_t solid; } State;
static void setup(PonkoCtx* ctx, void* user) {
    State* state = (State*)user;
    PonkoWorld* w = ponko_ctx_world(ctx);
    uint32_t t2, body, collider, t3;
    CHECK(ponko_transform_component(w, &t2) == PONKO_OK);
    CHECK(ponko_rigid_body_component(w, &body) == PONKO_OK);
    CHECK(ponko_collider_component(w, &collider) == PONKO_OK);
    CHECK(ponko_transform3d_component(w, &t3) == PONKO_OK);
    PonkoTransform2D at = (PonkoTransform2D){{10, 0}, 0, {1, 1}};
    PonkoRigidBody fixed = ponko_rigid_body(PONKO_BODY_FIXED);
    PonkoCollider shape = ponko_collider_circle(1);
    PonkoValue values[] = {{t2, &at, sizeof at}, {body, &fixed, sizeof fixed}, {collider, &shape, sizeof shape}};
    CHECK(ponko_spawn(w, values, 3, &state->solid) == PONKO_OK);
    at.position.x = 5; shape.sensor = true;
    uint64_t sensor;
    CHECK(ponko_spawn(w, values, 3, &sensor) == PONKO_OK);

    PonkoTransform3D source = ponko_transform3d_at((PonkoVec3){1, 2, 3});
    source.scale = (PonkoVec3){2, 3, 4};
    PonkoValue value = {t3, &source, sizeof source}; uint64_t original, placed;
    CHECK(ponko_spawn(w, &value, 1, &original) == PONKO_OK);
    CHECK(ponko_ctx_save_prefab(ctx, original, "test-dimensions-c3d.json") == PONKO_OK);
    CHECK(ponko_ctx_spawn_prefab3d(ctx, "test-dimensions-c3d.json", (PonkoVec3){7, 8, 9}, &placed) == PONKO_OK);
    PonkoTransform3D got;
    CHECK(ponko_get(w, placed, t3, &got, sizeof got) == PONKO_OK && got.position.x == 7 && got.scale.y == 3);
    source.position.x = 11; source.scale.x = 6;
    CHECK(ponko_ctx_spawn_prefab_transform3d(ctx, "test-dimensions-c3d.json", source, &placed) == PONKO_OK);
    CHECK(ponko_get(w, placed, t3, &got, sizeof got) == PONKO_OK && got.position.x == 11 && got.scale.x == 6);
    CHECK(ponko_ctx_save_prefab(ctx, state->solid, "test-dimensions-c2d.json") == PONKO_OK);
    at.rotation = .75f; at.scale = (PonkoVec2){3, 4};
    CHECK(ponko_ctx_spawn_prefab_transform2d(ctx, "test-dimensions-c2d.json", at, &placed) == PONKO_OK);
    PonkoTransform2D got2;
    CHECK(ponko_get(w, placed, t2, &got2, sizeof got2) == PONKO_OK && got2.rotation == .75f && got2.scale.y == 4);
    /* Keep the copied body out of the ray/overlap tests. */
    CHECK(ponko_despawn(w, placed) == PONKO_OK);
    size_t before, after;
    CHECK(ponko_world_len(w, &before) == PONKO_OK);
    CHECK(ponko_ctx_spawn_prefab3d(ctx, "test-dimensions-c3d.json", (PonkoVec3){0, 0, 0}, NULL) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_spawn_prefab_transform3d(ctx, NULL, source, &placed) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_spawn_prefab_transform2d(ctx, "\xff", at, &placed) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_world_len(w, &after) == PONKO_OK && after == before);
}
static void query(PonkoCtx* ctx, void* user) {
    const State* state = (const State*)user;
    PonkoQueryFilter2D filter = {0};
    bool found = false; PonkoRayHit hit = {0};
    CHECK(ponko_ctx_cast_ray_filtered(ctx, (PonkoVec2){0, 0}, (PonkoVec2){1, 0}, 100, filter, &found, &hit) == PONKO_OK);
    CHECK(found && ponko_entity_bits(hit.entity) == state->solid && fabsf(hit.distance - 9) < .001f && fabsf(hit.point.x - 9) < .001f && fabsf(hit.normal.x + 1) < .001f);
    filter.exclude = true; filter.exclude_entity = state->solid;
    CHECK(ponko_ctx_cast_ray_filtered(ctx, (PonkoVec2){0, 0}, (PonkoVec2){1, 0}, 100, filter, &found, &hit) == PONKO_OK && !found);
    filter.exclude = false; filter.sensors = true;
    CHECK(ponko_ctx_cast_ray_filtered(ctx, (PonkoVec2){0, 0}, (PonkoVec2){1, 0}, 100, filter, &found, &hit) == PONKO_OK && found && fabsf(hit.distance - 4) < .001f);
    size_t count = 99;
    CHECK(ponko_ctx_overlap_ball(ctx, (PonkoVec2){5, 0}, 1, filter, NULL, 0, &count) == PONKO_OK && count == 1);
    PonkoEntity entities[1];
    CHECK(ponko_ctx_overlap_ball(ctx, (PonkoVec2){5, 0}, 1, filter, entities, 1, &count) == PONKO_OK && count == 1);
    filter.sensors = false;
    CHECK(ponko_ctx_overlap_ball(ctx, (PonkoVec2){5, 0}, 1, filter, entities, 1, &count) == PONKO_OK && count == 0);
    CHECK(ponko_ctx_cast_ray_filtered(ctx, (PonkoVec2){NAN, 0}, (PonkoVec2){1, 0}, 100, filter, &found, &hit) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_cast_ray_filtered(ctx, (PonkoVec2){0, 0}, (PonkoVec2){1, 0}, 100, filter, NULL, &hit) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_overlap_ball(ctx, (PonkoVec2){0, 0}, -1, filter, entities, 1, &count) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_ctx_overlap_ball(ctx, (PonkoVec2){0, 0}, 1, filter, NULL, 1, &count) == PONKO_INVALID_ARGUMENT);
}
int main(void) {
    State state = {0}; PonkoWorld* world = NULL;
    PonkoGame* game = ponko_game_new("dimensions", 1, 1);
    CHECK(ponko_game_physics(game, (PonkoVec2){0, 0}) == PONKO_OK);
    CHECK(ponko_game_sim_system(game, "query", 1, query, &state) == PONKO_OK);
    CHECK(ponko_game_run_headless(game, 1, setup, &state, &world) == PONKO_OK);
    CHECK(ponko_world_free(world) == PONKO_OK);
    remove("assets/test-dimensions-c2d.json"); remove("assets/test-dimensions-c3d.json");
}
