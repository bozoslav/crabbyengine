# Typed-query matching cache prototype (#139)

Run `cargo run --release -p ponko_bench -- query-cache`. This separate group
uses one entity per archetype, 1/16 matching layouts (two `with`, two `without`
filters), and reconstructs the typed query on every use. It measures 1, 100 and
1,000 queries per batch at 1, 8, 32, 64, 128, 256 and 1,024 layouts. `dense`
counts an unfiltered single-component query matching every layout. `cold`
uses a fresh world for every sample. Setup is untimed. Steady samples have five
warmup batches and 100 measured batches; cold samples have one warmup.

## Measurements (2026-11-09)

Apple M4 Max, macOS, aarch64-apple-darwin, rustc 1.99.1
(58cf8f9c3, LLVM 23.1.1), Cargo release profile, default benchmark features
(including ECS parallel support). Source baseline: `5895d3b`; pre-cache benchmark
commit: `cfb4578`. Final paired results below include the opt-in prototype.
Other builds were active: the substantial p95 scheduling outliers mean these
are microbenchmark observations, not isolated CPU latency or engine frame times.
GPU metrics are inapplicable. No overall engine performance claim is made.

All values are milliseconds per batch of 1,000 reconstructed queries:

| Layouts | Sparse count scan p50 / p95 | Sparse cached p50 / p95 | Dense scan p50 | Dense cached p50 |
| --- | --- | --- | --- | --- |
| 1 | 0.029 / 0.030 | 0.030 / 0.030 | 0.018 | 0.018 |
| 8 | 0.043 / 0.045 | 0.043 / 0.124 | 0.023 | 0.024 |
| 32 | 0.109 / 0.110 | 0.107 / 0.242 | 0.097 | 0.059 |
| 64 | 0.178 / 14.848 | 0.199 / 0.200 | 0.100 | 0.099 |
| 128 | 0.345 / 0.796 | 0.190 / 0.195 | 0.181 | 0.303 |
| 256 | 0.756 / 31.824 | 0.233 / 0.235 | 0.369 | 0.468 |
| 1024 | 3.519 / 107.541 | 0.537 / 14.444 | 1.663 | 1.722 |

Below 128 layouts, both strategies scan. An earlier forced-cache experiment
showed sparse count at 32 layouts slowing from 0.083 to 0.148 ms, and at 64
layouts roughly breaking even (0.150 vs 0.156 ms). The conservative sparse-query
crossover is therefore between 64 and 128 layouts for this workload; the threshold
is 128. It is not a universal hardware/selectivity crossover. At 1024 layouts,
paired sequential traversal p50 is 0.529 ms cached and iterator p50 is 0.568 ms
cached. The initial unmodified baseline was about 2.9–3.0 ms for these sparse
operations. Cold count at 1024 layouts is 0.006 ms scan vs 0.007 ms cached
(p95 0.029 / 0.015 ms); those cold measurements include registration and matching.

Dense queries can lose (at 128 layouts, 0.181 vs 0.303 ms). For that reason,
`Query::cached()` is opt-in, and default queries keep scanning. Broader workload,
selectivity and churn measurements are needed before automatic global enablement.

## Ownership, allocations and memory

The world owns at most 128 signature entries. Further signatures scan without
eviction. The key contains an inline 32-ID required-component array and inline
four-ID positive/negative filter arrays; building/looking up a warmed signature
allocates nothing. Tests count allocations across reconstructed queries, `count`,
`for_each` and `iter`. First use and newly appended matching archetypes can grow
the retained vectors. Optional fetches do not restrict matching; mutability does
not change the candidate set. Filter order is normalized.

Each list stores only `usize` archetype indices: on this 64-bit target a 1/16
sparse signature at 1024 layouts holds 64 indices, a 512-byte payload; a dense
signature holds 1024 indices, an 8192-byte payload. At 128 dense signatures this
is 1 MiB of index payload, plus vector spare capacity and hash-table/key storage.
These are payload calculations, not allocator RSS measurements. No component or
archetype pointers survive a query borrow. The append-only layout count is a
non-wrapping generation; extensions match only newly appended layouts, including
empty layouts that may be repopulated later. Existing layouts never change their
component sets. Component registration alone cannot invalidate their membership.

This stage adds a Rust query hint; existing Rust and native APIs keep their
behavior. Native hint bindings, automatic policy tuning, general filter expansion
(#143), broad differential fuzzing (#133), and integrated frame-time/RSS studies
remain maintainer follow-ups. The isolated benchmark group does not replace #132.
