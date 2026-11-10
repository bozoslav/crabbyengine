#include "ponko.h"
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#define REQUIRE(x) do { if (!(x)) { fprintf(stderr, "failed: %s:%d: %s\n", __FILE__, __LINE__, #x); abort(); } } while (0)
int main(void) {
    PonkoWorld *w = ponko_world_new();
    uint32_t id = 0;
    REQUIRE(ponko_component_register(w, "Health", sizeof(float), _Alignof(float), &id) == PONKO_OK);
    PonkoField f = {"hp", PONKO_FIELD_F32, 0};
    REQUIRE(ponko_component_reflect(w, id, &f, 1) == PONKO_OK);
    REQUIRE(ponko_resource_reflect(w, "Score", sizeof(float), _Alignof(float), &f, 1) == PONKO_OK);
    const char *schema = "{\"Object\":[[\"hp\",\"Number\"]]}";
    REQUIRE(ponko_world_describe_component(w, "Health", schema) == PONKO_OK);
    REQUIRE(ponko_world_describe_resource(w, "Score", schema) == PONKO_OK);
    REQUIRE(ponko_world_describe_component(w, "Missing", schema) != PONKO_OK);
    REQUIRE(ponko_world_describe_resource(w, "Missing", schema) != PONKO_OK);
    REQUIRE(ponko_world_describe_component(w, "Health", "{") != PONKO_OK);
    REQUIRE(ponko_world_describe_resource(w, "Score", "\"Unknown\"") != PONKO_OK);
    REQUIRE(ponko_world_describe_component(NULL, "Health", schema) != PONKO_OK);
    REQUIRE(ponko_world_describe_resource(NULL, "Score", schema) != PONKO_OK);
    REQUIRE(ponko_world_describe_component(w, NULL, schema) != PONKO_OK);
    REQUIRE(ponko_world_describe_resource(w, "Score", NULL) != PONKO_OK);
    REQUIRE(ponko_world_describe_component(w, "Health", "\xff") != PONKO_OK);
    REQUIRE(ponko_world_describe_resource(w, "\xff", schema) != PONKO_OK);
    ponko_world_free(w);
}
