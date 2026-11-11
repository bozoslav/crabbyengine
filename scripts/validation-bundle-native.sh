#!/usr/bin/env bash
# Puts a C++ game through the validation bundle tool: starts the built
# battle example (examples/cpp/battle.cpp) headless, runs the checks in
# examples/cpp/battle-validation.json against it through the MCP tools, and
# requires the bundle to pass and to be intact. Then requires a check the
# game can't meet to fail with the step, tick, expected and actual values.
#
#   scripts/validation-bundle-native.sh GAME [OUT_DIR]
#
# GAME is the built executable (target/native/ponko_cpp_battle after the
# CMake build). Headless: no window is opened. See
# book/src/validation-bundles.md.
set -euo pipefail

game="${1:?usage: validation-bundle-native.sh GAME [OUT_DIR]}"
out="${2:-target/validation-bundles-native}"
rm -rf "$out"
mkdir -p "$out"

cargo build --example validation_bundle
tool() { cargo run --quiet --example validation_bundle -- "$@"; }
run() {
  tool --out "$1" --launch-program "$game" --scenario "$2" \
    --launch-env UNITS=400 --launch-env SEED=7 --launch-env TICKS=1000000 --redact-paths
}

run "$out/battle" examples/cpp/battle-validation.json
tool verify "$out/battle"
grep -q '"status": "passed"' "$out/battle/scenario-report.json"

# The same game against a check it can't meet: 400 units, 401 expected.
sed 's/"equals": 400/"equals": 401/' examples/cpp/battle-validation.json > "$out/wrong.json"
got=0
run "$out/battle-wrong" "$out/wrong.json" || got=$?
if [ "$got" != 1 ]; then
  echo "validation-bundle-native: a failing check must exit 1, got $got" >&2
  exit 1
fi
grep -q '"expected": 401' "$out/battle-wrong/scenario-report.json"
grep -q '"actual": 400' "$out/battle-wrong/scenario-report.json"
tool verify "$out/battle-wrong"
# Nothing of this machine is in what gets published.
bash scripts/validation-bundle-guard.sh "$out"
echo "validation-bundle-native: ok, in $out"
