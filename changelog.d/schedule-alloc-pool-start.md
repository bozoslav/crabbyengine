### Fixed

- Tests: the scheduled-system allocation test starts the thread pool's
  workers before it counts, so their start-up no longer lands in a measured
  tick on slow machines.
