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
        uint64_t e;
        CHECK(crabby_chunk_entity(chunk, i, &e) == CRABBY_OK);
        CHECK(e == c->entity);
        ((Position*)chunk->columns[0])[i].x += 2;
        ++c->seen;
    }
    size_t len;
    CHECK(crabby_world_len(c->world, &len) == CRABBY_BUSY);
    CHECK(crabby_world_free(c->world) == CRABBY_BUSY);
}
static bool setup(CrabbyWorld* world, void* user) { (void)user; uint64_t e; CHECK(crabby_spawn(world, NULL, 0, &e) == CRABBY_OK); return true; }
static bool step(CrabbyWorld* world, uint64_t tick, float dt, void* user) {
    (void)tick; size_t len; CHECK(crabby_world_len(world, &len) == CRABBY_OK && len == 1);
    CHECK(dt > .019f && dt < .021f); ++*(size_t*)user; return true;
}
int main(void) {
    CHECK(crabby_abi_version() == 1);
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
    size_t ticks = 0; CrabbyWorld* sim = NULL;
    CHECK(crabby_run_headless(3, 50, setup, step, &ticks, &sim) == CRABBY_OK);
    CHECK(ticks == 3 && sim); CHECK(crabby_world_free(sim) == CRABBY_OK);
}
