// Ponko for C++17. The legacy namespace remains source and binary compatible.
#ifndef PONKO_HPP
#define PONKO_HPP
#include "ponko.h"
#include "crabby.hpp"
namespace ponko = crabby;
#define PONKO_PROFILE_SCOPE(name) CRABBY_PROFILE_SCOPE(name)
#define PONKO_COUNT_CPP_ALLOCATIONS CRABBY_COUNT_CPP_ALLOCATIONS
#endif
