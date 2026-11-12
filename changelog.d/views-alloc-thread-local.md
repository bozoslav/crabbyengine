### Fixed
- The view allocation test counts only its own thread's allocations, so allocations by the test harness's other threads no longer fail it.
