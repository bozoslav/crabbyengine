#include "crabby.h"
#include <stdio.h>
#include <stdlib.h>
typedef struct { float x, y; } Position;
typedef struct { float x, y; } Velocity;
static void check(CrabbyStatus status) {
    if (status != CRABBY_OK) { fprintf(stderr, "%s\n", crabby_last_error()); exit(1); }
}
// ANCHOR: c_columns
static void update(const CrabbyChunk* chunk, void* user) {
    (void)user;
    Position* positions = chunk->columns[0];
    const Velocity* velocities = chunk->columns[1];
    for (size_t i = 0; i < chunk->count; ++i) {
        positions[i].x += velocities[i].x;
        positions[i].y += velocities[i].y;
    }
}
// ANCHOR_END: c_columns
int main(void) {
    // ANCHOR: c_world
    CrabbyWorld* world = crabby_world_new();
    if (!world) { fprintf(stderr, "%s\n", crabby_last_error()); return 1; }
    uint32_t position, velocity;
    check(crabby_component_register(world, "Position", sizeof(Position), _Alignof(Position), &position));
    check(crabby_component_register(world, "Velocity", sizeof(Velocity), _Alignof(Velocity), &velocity));
    Position p = {0, 0};
    Velocity v = {1, 2};
    CrabbyValue values[] = {{position, &p, sizeof(p)}, {velocity, &v, sizeof(v)}};
    uint64_t entity;
    check(crabby_spawn(world, values, 2, &entity));
    uint32_t columns[] = {position, velocity};
    check(crabby_query(world, columns, 2, update, NULL));
    check(crabby_get(world, entity, position, &p, sizeof(p)));
    check(crabby_world_free(world));
    // ANCHOR_END: c_world
    printf("Position: %g, %g\n", (double)p.x, (double)p.y);
    return p.x == 1 && p.y == 2 ? 0 : 1;
}
