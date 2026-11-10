### Changed
- `Character3D`s move on the thread pool from 16 characters (was 8),
  measured: one thread is faster at 8 and ties at 12. The new
  `ponko_bench --bin characters` re-measures it.
