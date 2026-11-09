#include "crabby.hpp"
#include "ponko.hpp"
#include <type_traits>
#include <cassert>
static_assert(std::is_same_v<ponko::game, crabby::game>);
static_assert(std::is_same_v<PonkoVec2, CrabbyVec2>);
int main() {
    assert(ponko_abi_version() == 2);
    ponko::game("Compatibility", 64, 64).run_headless(1, [](ponko::ctx&) {});
    crabby::game("Legacy", 64, 64).run_headless(1, [](crabby::ctx&) {});
}
