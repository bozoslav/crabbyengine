### Added
- View queries take typed filters: `ViewQuery::filter::<F>()` with `With`,
  `Without`, `Or`, `Added<T>`, `Changed<T>` and tuples, plus
  `ViewQuery::since`. Change filters need `T` declared (read is enough) and
  already tracked; an undeclared one panics with the view's name and the
  missing access, and a view never registers or starts tracking a component.
  `for_each`, `iter`, `count` and `get` filter rows, and a mutable fetch
  marks only the rows it visits.
- `WorldView::removed::<T>()`, `removed_since::<T>()` and `change_tick()`.
  A view system is shown changes and removals since it last ran, like a
  `&mut Ctx` system.

### Fixed
- `ViewQuery::get` with a row filter returns `None` for a row the filter
  rejects.
