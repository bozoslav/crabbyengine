### Added
- Typed ECS `Or` filters accept tuple arms that require all their filters,
  including nested combinations and empty arms. The existing inline limits
  apply to the expanded clauses; cached and parallel queries use the same matches.
- Equal-work release benchmarks compare plain, typed, and nested OR/AND queries.
