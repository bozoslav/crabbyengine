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
    return 0;
}
