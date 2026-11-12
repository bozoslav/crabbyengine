### Documented
- The command and event barrier contract is complete (#146): the guide's
  "When changes become visible" adds the rows for `Game::setup` callbacks
  and scene loads, a paired Rust and C++ example, the rule that a C or C++
  query callback can record commands but can't read the world it walks,
  and "Moving a system to a view system", a checklist for systems that
  relied on the flush after every system. No behavior changed.

### Added
- Tests for the contract with the parallel scheduler on:
  `tests/schedule_barriers.rs` runs `&mut Ctx` systems and a batch between
  them with the scheduler off, on, and on threads; `examples/cpp/tests.c`
  and `tests.cpp` pin the same rules for C and C++ systems.
