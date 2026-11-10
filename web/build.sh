#!/usr/bin/env bash
# Builds a game for the web, ready to serve as a static site.
#
#   web/build.sh battle                  # an example -> target/web/battle/
#   web/build.sh --bin my_game           # your game's binary, run from its folder
#   web/build.sh battle site/play        # choose the output folder
#   web/build.sh --features 3d web_smoke_3d   # with Cargo features (comma-separated)
#   python3 -m http.server -d target/web/battle 8000
#
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <the wasm-bindgen in Cargo.lock>
set -euo pipefail

kind=example
features=
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) kind=bin; shift ;;
    --features) features=${2:?--features needs a list}; shift 2 ;;
    *) break ;;
  esac
done
name=${1:?usage: build.sh [--bin] [--features <list>] <name> [out-dir]}
here=$(cd "$(dirname "$0")" && pwd)
target=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
out=${2:-$target/web/$name}

feature_args=()
if [ -n "$features" ]; then
  feature_args=(--features "$features")
fi
cargo build --release --target wasm32-unknown-unknown "--$kind" "$name" ${feature_args[@]+"${feature_args[@]}"}
if [ "$kind" = example ]; then
  wasm="$target/wasm32-unknown-unknown/release/examples/$name.wasm"
else
  wasm="$target/wasm32-unknown-unknown/release/$name.wasm"
fi

rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section \
  --out-dir "$out" --out-name game "$wasm"

# Assets, and the list of them that the engine fetches before setup.
if [ -d assets ]; then
  cp -R assets "$out/assets"
  (cd "$out/assets" && find . -type f ! -name index.json ! -name '.*' | sed 's|^\./||' | sort |
    python3 -c 'import json, sys; print(json.dumps([l.strip() for l in sys.stdin]))' > index.json)
fi

sed "s/{{TITLE}}/$name/" "$here/index.html" > "$out/index.html"
echo "built $out ($(du -sh "$out" | cut -f1)); try it: python3 -m http.server -d $out 8000"
