/* Compile and link both naming generations in the same translation unit. */
#include "ponko.h"
#include "crabby.h"
#include <assert.h>
int main(void) {
    assert(ponko_abi_version() == crabby_abi_version());
    struct PonkoWorld *world = ponko_world_new();
    assert(world != 0);
    struct CrabbyWorld *legacy = world;
    assert(crabby_world_free(legacy) == PONKO_OK);
    world = crabby_world_new();
    assert(ponko_world_free(world) == CRABBY_OK);
    return 0;
}
