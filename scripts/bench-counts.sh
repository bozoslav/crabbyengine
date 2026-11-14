#!/usr/bin/env bash
# Counts the tracked workloads' instructions for one engine tree:
#
#   scripts/bench-counts.sh <engine-tree> <workloads-tree> <out.json> [machine]
#
# <engine-tree> is the checkout being measured. <workloads-tree> is the
# checkout whose `crates/ponko_bench/src/bin/bench_track` is used: when the
# two differ, that folder is copied over the engine tree's own first, so both
# sides of a comparison (a pull request and its base, or two releases) run
# exactly the same workloads. Needs valgrind (Linux). BENCH_REPEAT sets the
# runs per workload (default 3). The build goes to CARGO_TARGET_DIR if set.
set -euo pipefail

if [ "$#" -lt 3 ]; then
  echo "usage: $0 <engine-tree> <workloads-tree> <out.json> [machine]" >&2
  exit 2
fi
engine=$(cd "$1" && pwd)
workloads=$(cd "$2" && pwd)
out=$3
machine=${4:-${PONKO_BENCH_MACHINE:-}}
tool=crates/ponko_bench/src/bin/bench_track

if [ "$engine" != "$workloads" ]; then
  rm -rf "${engine:?}/$tool"
  cp -R "$workloads/$tool" "$engine/$tool"
fi

cd "$engine"
cargo build --release --locked -p ponko_bench --bin bench_track
bin=${CARGO_TARGET_DIR:-$engine/target}/release/bench_track
args=(counts --repeat "${BENCH_REPEAT:-3}" --out "$out")
if [ -n "$machine" ]; then
  args+=(--machine "$machine")
fi
# From the engine tree, so the document records its commit.
"$bin" "${args[@]}"
