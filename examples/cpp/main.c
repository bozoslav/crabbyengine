#include "ponko.h"
#include <stdio.h>
#include <stdlib.h>
typedef struct { float x, y; } Position;
typedef struct { float x, y; } Velocity;
static void check(PonkoStatus status) {
    if (status != PONKO_OK) { fprintf(stderr, "%s\n", ponko_last_error()); exit(1); }
}
// ANCHOR: c_columns
static void update(const PonkoChunk* chunk, void* user) {
    (void)user;
    Position* positions = chunk->columns[0];
    const Velocity* velocities = chunk->columns[1];
    for (size_t i = 0; i < chunk->count; ++i) {
        positions[i].x += velocities[i].x;
        positions[i].y += velocities[i].y;
    }
}
// ANCHOR_END: c_columns

// ANCHOR: c_game
typedef struct { uint32_t position, velocity; uint64_t agent; } Game;

static void setup(PonkoCtx* ctx, void* user) {
    Game* g = user;
    PonkoWorld* world = ponko_ctx_world(ctx);
    check(ponko_component_register(world, "Position", sizeof(Position), _Alignof(Position), &g->position));
    check(ponko_component_register(world, "Velocity", sizeof(Velocity), _Alignof(Velocity), &g->velocity));
    Position p = {0, 0};
    Velocity v = {1, 2};
    PonkoValue values[] = {{g->position, &p, sizeof(p)}, {g->velocity, &v, sizeof(v)}};
    check(ponko_spawn(world, values, 2, &g->agent));
}

static void movement_chunk(const PonkoChunk* chunk, void* user) {
    float dt = *(const float*)user;
    Position* positions = chunk->columns[0];
    const Velocity* velocities = chunk->columns[1];
    for (size_t i = 0; i < chunk->count; ++i) {
        positions[i].x += velocities[i].x * dt;
        positions[i].y += velocities[i].y * dt;
    }
}

static void movement(PonkoCtx* ctx, void* user) {
    Game* g = user;
    float dt = ponko_ctx_dt(ctx);
    uint32_t columns[] = {g->position, g->velocity};
    check(ponko_query(ponko_ctx_world(ctx), columns, 2, movement_chunk, &dt));
}
// ANCHOR_END: c_game

int main(void) {
    // ANCHOR: c_world
    PonkoWorld* world = ponko_world_new();
    if (!world) { fprintf(stderr, "%s\n", ponko_last_error()); return 1; }
    uint32_t position, velocity;
    check(ponko_component_register(world, "Position", sizeof(Position), _Alignof(Position), &position));
    check(ponko_component_register(world, "Velocity", sizeof(Velocity), _Alignof(Velocity), &velocity));
    Position p = {0, 0};
    Velocity v = {1, 2};
    PonkoValue values[] = {{position, &p, sizeof(p)}, {velocity, &v, sizeof(v)}};
    uint64_t entity;
    check(ponko_spawn(world, values, 2, &entity));
    uint32_t columns[] = {position, velocity};
    check(ponko_query(world, columns, 2, update, NULL));
    check(ponko_get(world, entity, position, &p, sizeof(p)));
    check(ponko_world_free(world));
    // ANCHOR_END: c_world
    printf("Position: %g, %g\n", (double)p.x, (double)p.y);

    // ANCHOR: c_run
    Game g = {0};
    PonkoGame* game = ponko_game_new("C headless", 1, 1);
    check(ponko_game_sim_system(game, "movement", 1, movement, &g));
    PonkoWorld* sim = NULL;
    check(ponko_game_run_headless(game, 60, setup, &g, &sim)); // frees `game`
    Position final;
    check(ponko_get(sim, g.agent, g.position, &final, sizeof(final)));
    check(ponko_world_free(sim));
    // ANCHOR_END: c_run
    printf("After 60 ticks: %g, %g\n", (double)final.x, (double)final.y);
    int ok = p.x == 1 && p.y == 2 && final.x > 0.999f && final.x < 1.001f && final.y > 1.999f && final.y < 2.001f;
    return ok ? 0 : 1;
}
