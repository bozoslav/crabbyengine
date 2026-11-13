### Breaking changes

- C ABI: the `crabby_*` compatibility layer is removed. It was a rename layer
  over the `ponko_*` functions and no native artifact was ever released with it.
  `include/crabby.h` and `include/crabby.hpp` are gone, as are the exported
  `crabby_*` functions, the `Crabby*` types and `CRABBY_*` constants (also the
  Rust names `ponko_ffi::crabby_*` and `Crabby*`), and the `legacy-names`
  feature of `ponko_ffi`. Replace `crabby_` with `ponko_`, `Crabby` with
  `Ponko`, `CRABBY_` with `PONKO_`, `crabby::` with `ponko::`, and include
  `ponko.h` / `ponko.hpp`. Struct layouts, numeric values and every `ponko_*`
  symbol are unchanged, and the ABI version stays 2.
- CMake: the `CRABBY_LIBRARY_DIR` and `CRABBY_INCLUDE_DIR` cache variables of
  `examples/cpp` are removed; use `PONKO_LIBRARY_DIR` and `PONKO_INCLUDE_DIR`.

### Added

- `ponko.h` defines `PONKO_ABI_VERSION` (2). `ponko.hpp` compares it with
  `ponko_abi_version()` when the first `ponko::game` or `ponko::world` is made
  and throws a `ponko::error` naming both numbers on a mismatch.
- The book's C/C++ chapter documents the constructor convention for new C
  functions (a status plus an out-pointer) and why `ponko_world_free` returns a
  status.
