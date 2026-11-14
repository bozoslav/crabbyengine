### Changed
- A world reclaims empty archetypes: it keeps at most 64 of them, or as
  many as it has archetypes with entities if that is more, and reclaims
  the one empty longest past that. The reclaimed slot and storage are
  reused for the next new component set, so a world whose entities keep
  visiting new component combinations levels off in archetypes and memory
  and stops allocating once it settles (`crates/ponko_bench/SPARSE_SET.md`).
  An index into `RawWorldRef::archetypes` names an archetype only until
  the next structural change, a reclaimed slot shows as an empty archetype
  without components, and query order, which follows the archetype slots,
  can differ from before (the same for the same operations).
