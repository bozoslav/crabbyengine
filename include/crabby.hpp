// Deprecated compatibility shim. Use ponko.hpp and ponko::.
// Removal: a future major release, no earlier than 2027-11-10.
#ifndef CRABBY_HPP
#define CRABBY_HPP
#include "ponko.hpp"
#include "crabby.h"
namespace crabby = ponko;
#define CRABBY_PROFILE_SCOPE(name) PONKO_PROFILE_SCOPE(name)
#define CRABBY_COUNT_CPP_ALLOCATIONS PONKO_COUNT_CPP_ALLOCATIONS
#endif
