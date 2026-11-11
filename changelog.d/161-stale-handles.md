### Changed
- A freed `Mesh` or `GpuBuffer` slot is reused under a new id: the top 8 bits
  of the id count the slot's generation. A handle kept past `free` stays dead
  (`is_live` is false, models showing it draw nothing and are listed as
  dangling, freeing it again leaves the new resource alone) instead of
  reaching the next mesh or buffer made in its slot. Code that expected the
  next mesh or buffer to come back with the freed id must compare handles by
  liveness instead. The C ABI layout is unchanged.

### Fixed
- A stale mesh or GPU buffer handle could free, write to or draw the resource
  made after it in the same slot.
