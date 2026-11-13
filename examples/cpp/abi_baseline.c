/* Old-header client: compiled against the retained ABI-2 baseline header
 * (crates/ponko_ffi/tests/fixtures/abi/ponko_abi2.h), linked against the
 * current library. If a struct layout, a function signature or a startup
 * check changed incompatibly, this program fails.
 *
 * The baseline header is a copy from the engine's main branch at a recorded
 * revision, not a header from a published release: it shows that the current
 * library honors what that revision's header promised, nothing about earlier
 * binary releases. See the README next to the header.
 */
#include "ponko_abi2.h"
#include <stdio.h>
#include <string.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr, "abi_baseline failed: %s (line %d)\n", #x, __LINE__); return 1; } } while (0)
#define CHECK_VOID(x) do { if (!(x)) { fprintf(stderr, "abi_baseline failed: %s (line %d)\n", #x, __LINE__); failed = 1; } } while (0)

typedef struct { int x; } Position;
typedef struct { int tag; } Tag;
typedef struct { size_t rows; size_t next_first; uint64_t entities[3]; } Seen;
static int failed = 0;

/* Two columns and three rows, so that count, column_count and first all
 * differ and a reordered PonkoChunk is noticed. */
static void visit(const struct PonkoChunk *chunk, void *user) {
    Seen *seen = user;
    CHECK_VOID(chunk->column_count == 2);
    CHECK_VOID(chunk->columns != NULL && chunk->entities != NULL);
    CHECK_VOID(chunk->first == seen->next_first);
    for (size_t i = 0; i < chunk->count && seen->rows < 3; ++i) {
        CHECK_VOID(ponko_entity_bits(chunk->entities[i]) == seen->entities[seen->rows]);
        ((Position *)chunk->columns[0])[i].x += 2;
        CHECK_VOID(((Tag *)chunk->columns[1])[i].tag == (int)seen->rows + 100);
        seen->rows++;
    }
    seen->next_first += chunk->count;
}

typedef struct { uint64_t ticks; uint64_t last_tick; float dt; } Run;

static void system_fn(struct PonkoCtx *ctx, void *user) {
    Run *run = user;
    run->ticks++;
    run->last_tick = ponko_ctx_sim_tick(ctx);
    run->dt = ponko_ctx_dt(ctx);
}

static void setup(struct PonkoCtx *ctx, void *user) { (void)ctx; (void)user; }

int main(void) {
    /* Startup check STABILITY.md tells dynamic loaders to make. */
    CHECK(ponko_abi_version() == 2);

    /* Plain values, ids and status codes. */
    struct PonkoWorld *w = ponko_world_new();
    CHECK(w != NULL);
    uint32_t pos = 0;
    CHECK(ponko_component_register(w, "Position", sizeof(Position), _Alignof(Position), &pos) == PONKO_OK);
    CHECK(ponko_component_register(w, "bad", 3, 2, &pos) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_component_register(w, "Position", sizeof(Position), _Alignof(Position), &pos) == PONKO_OK);

    uint32_t tag = 0;
    CHECK(ponko_component_register(w, "Tag", sizeof(Tag), _Alignof(Tag), &tag) == PONKO_OK);
    Seen seen = {0, 0, {0, 0, 0}};
    for (int i = 0; i < 3; ++i) {
        Position start = {40};
        Tag t = {100 + i};
        struct PonkoValue values[2] = {{pos, &start, sizeof start}, {tag, &t, sizeof t}};
        CHECK(ponko_spawn(w, values, 2, &seen.entities[i]) == PONKO_OK);
    }
    uint64_t entity = seen.entities[0];

    /* A query: PonkoChunk's layout is read by the callback. */
    uint32_t both[2] = {pos, tag};
    CHECK(ponko_query(w, both, 2, visit, &seen) == PONKO_OK);
    CHECK(seen.rows == 3 && seen.next_first == 3 && !failed);
    Position out = {0};
    CHECK(ponko_get(w, entity, pos, &out, sizeof out) == PONKO_OK && out.x == 42);

    /* A built-in component passes by value in a struct. */
    uint32_t transform = 0;
    CHECK(ponko_transform_component(w, &transform) == PONKO_OK);
    struct PonkoTransform2D at = {{3.0f, -4.0f}, 0.5f, {2.0f, 1.0f}};
    struct PonkoValue tv = {transform, &at, sizeof at};
    uint64_t moved = 0;
    CHECK(ponko_spawn(w, &tv, 1, &moved) == PONKO_OK);
    struct PonkoTransform2D back;
    memset(&back, 0, sizeof back);
    CHECK(ponko_get(w, moved, transform, &back, sizeof back) == PONKO_OK);
    CHECK(back.position.x == 3.0f && back.position.y == -4.0f && back.rotation == 0.5f);
    CHECK(back.scale.x == 2.0f && back.scale.y == 1.0f);

    /* Stale handles stay dead. */
    CHECK(ponko_despawn(w, entity) == PONKO_OK);
    bool alive = true;
    CHECK(ponko_is_alive(w, entity, &alive) == PONKO_OK && !alive);
    size_t len = 0;
    CHECK(ponko_world_len(w, &len) == PONKO_OK && len == 3);
    CHECK(ponko_world_len(NULL, &len) == PONKO_INVALID_ARGUMENT);
    CHECK(ponko_last_error() != NULL);
    CHECK(ponko_world_free(w) == PONKO_OK);

    /* Structs returned and passed by value. */
    struct PonkoAudioEffects fx = ponko_audio_effects_default();
    CHECK(fx.low_pass_hz == 20000.0f && fx.reverb == 0.0f);
    struct PonkoVec3 v = {1.0f, 2.0f, 3.0f};
    struct PonkoContactModifier3D belt = ponko_contact_modifier3d_conveyor(v);
    CHECK(belt.surface_velocity.x == 1.0f && belt.surface_velocity.y == 2.0f && belt.surface_velocity.z == 3.0f);
    CHECK(!belt.has_one_way && !belt.hook);

    /* A headless game: callbacks get a context and the clock runs. */
    struct PonkoGame *game = ponko_game_new("abi baseline", 64, 64);
    CHECK(game != NULL);
    Run run = {0, 0, 0.0f};
    CHECK(ponko_game_sim_system(game, "count", 1, system_fn, &run) == PONKO_OK);
    struct PonkoWorld *final_world = NULL;
    CHECK(ponko_game_run_headless(game, 5, setup, NULL, &final_world) == PONKO_OK);
    CHECK(run.ticks == 5 && run.last_tick == 4 && run.dt > 0.0f);
    if (final_world != NULL) CHECK(ponko_world_free(final_world) == PONKO_OK);
    puts("abi_baseline ok");
    return 0;
}
