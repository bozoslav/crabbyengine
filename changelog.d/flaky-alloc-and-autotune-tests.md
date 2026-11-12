### Fixed
- The view allocation test counts only its own thread's allocations, so allocations by the test harness's other threads no longer fail it.
- The live automatic-quality GPU test draws until each decision lands (capped) instead of for a fixed frame count, so one late GPU sample on a busy machine no longer fails it.
