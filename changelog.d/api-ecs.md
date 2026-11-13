### Breaking changes
- **ECS traits are sealed.** `QueryData`, `ReadOnlyQueryData`, `Bundle`,
  `ResourceSet` and `ResourceTuple` can no longer be implemented outside
  `ponko_ecs`: a custom impl → the provided `&T`, `&mut T`, `Option<..>`,
  `Entity` and tuples of them (nest tuples past 12). The parallel loops and
  views that share a world call `fetch` on worker threads, which is only
  sound for impls this crate controls.
- **`Query::with::<T>()` and `Query::without::<T>()` are removed**, on world
  queries and view queries: `.with::<T>()` → `.filter::<With<T>>()`,
  `.without::<T>()` → `.filter::<Without<T>>()`; several at once as a tuple,
  `.filter::<(With<A>, Without<B>)>()`.
- **Change tracking is always explicit.** A `Changed<T>` or `Added<T>` filter
  in a world query no longer starts tracking `T`; it panics for an untracked
  component, as it already did in a view query: add
  `world.track_changes::<T>()` in setup, before the first query that filters
  on `T`. The same holds for the `changed`/`added` bits of a `ChunkQuery`:
  call `world.raw().track_changes_dynamic(id)` first. The C and C++ filters
  are unchanged and still start tracking on first use.
- **The by-id calls moved off `World`**, behind `World::raw()` and
  `World::raw_ref()`:
  - `world.register_component(..)` → `world.raw().register_component(..)`
  - `world.track_changes_dynamic(id)` → `world.raw().track_changes_dynamic(id)`
  - `world.mark_changed_dynamic(e, id)` → `world.raw().mark_changed_dynamic(e, id)`
  - `world.removed_dynamic(id, since)` → `world.raw_ref().removed_dynamic(id, since)`
  - `world.query_chunks(query, f)` → `world.raw().query_chunks(query, f)`
  - `world.query_chunks_filtered(query, &filter, f)` →
    `world.raw().query_chunks(query.filter(&filter), f)`
  - `world.query_chunks_cached(query, filter, cache, f)` →
    `world.raw().query_chunks(query.cache(cache), f)`, with `.filter(..)` if
    there is one
- **`ChunkQuery` is built, not written out.** Its fields are private:
  `ChunkQuery { writes: 0, changed: 1, since: Some(t), ..ChunkQuery::new(&ids) }`
  → `ChunkQuery::new(&ids).writes(0).changed(1).since(t)`; `added`,
  `match_any` (was the `any` field), `filter` and `cache` are set the same
  way, and `ids()` reads the components back.
- **Raw pointers follow the borrow they came from.**
  `RawWorldRef::get_dynamic`, `RawWorld::get_dynamic`,
  `RawWorldRef::get_resource_dynamic`, `RawWorld::get_resource_dynamic` and
  `Archetype::column_ptr` return `*const u8` (was `*mut u8`). To write:
  `world.raw().get_dynamic_mut(e, id)`,
  `world.raw().get_resource_dynamic_mut(name)`, or a chunk loop
  (`raw().for_each_chunk`, `raw().query_chunks`). `get_dynamic` now records a
  read in the access log; `get_dynamic_mut` records the write.
- **`ViewState::prepare`, `finish`, `view` and `is_prepared` are private.**
  One view: `state.prepare(w); unsafe { state.view(w) }; state.finish(w)` →
  `state.run(&mut world, |view| ..)`. Several views sharing a world →
  `ViewGroup::new(states)` and `group.run(..)`.
- **`ViewState::set_changes_since` → `ViewState::override_changes_since`**
  and **`ViewState::set_change_tick` → `ViewState::override_change_tick`**:
  they no longer share a name with `World::set_changes_since`, which is
  unchanged.
- **`Events::send` needs `T: Send + Sync + 'static`** (was `T: 'static`),
  like the event writer of a view system: an event type that holds an `Rc`
  or a `RefCell` → `Arc`, a `Mutex`, or plain data.
- **`#[non_exhaustive]`** on `DynFilter`, `FilterError`, `FilterShape`,
  `AccessLog`, `Chunk`, `SchedulePlan`, `PlannedSystem`, `Stage`,
  `AccessReport` and `ReportedSystem`: a `match` on the enums needs a `_`
  arm, and the structs can't be built with a struct expression outside
  their crate (`AccessLog { .. }` → `AccessLog::default()` and assign the
  fields).

### Added
- `Query::single()` and `ViewQuery::single()`: the item of the only matching
  entity, or a `SingleError` saying whether none or several matched. It ends
  the query, so the item can be kept:
  `let pos = world.query::<&mut Pos>().filter::<With<Player>>().single()?;`.

### Changed
- `World::reset` documents that handles kept from before the reset can match
  entities spawned after it (generations start over, which is what lets a
  loaded save keep its handles), and what to do about it.
- The docs of `QueryData`, `Changed` and `World::track_changes` say that
  fetching `&mut T` marks `T` changed for every entity the query visits,
  written or not.
