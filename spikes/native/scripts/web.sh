#!/usr/bin/env bash
# The browser build of the app (docs/plans/backend-m12-browser.md): the
# same crates compiled for wasm32-unknown-unknown on GPUI's web platform,
# single-threaded, then `wasm-bindgen` for the JavaScript glue, into
# web/dist/ beside the page shell (web/index.html). Serve web/dist/ with
# any static server; `.wasm` must go out as `application/wasm`.
#
# Needs: the wasm32-unknown-unknown target of the pinned toolchain
# (`rustup target add wasm32-unknown-unknown`), and `wasm-bindgen` of the
# exact version in Cargo.lock (`cargo install wasm-bindgen-cli --version
# <it> --locked`; on PATH or in $WASM_BINDGEN). Optional: `wasm-opt` in
# $WASM_OPT, which shrinks the module. Not taken from PATH: binaryen 108
# (Ubuntu 24.04's package) rewrites wasm-bindgen 0.2.130's externref table
# into a module the browser refuses ("WebAssembly.Table.set(): Argument 1
# is invalid for table"), so the optimiser is chosen, never found.
#
# Usage (from spikes/native): scripts/web.sh [--debug]
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"

profile=release
cargo_flags=(--release)
if [ "${1:-}" = "--debug" ]; then
  profile=debug
  cargo_flags=()
fi

lock_version=$(awk '/^name = "wasm-bindgen"$/{getline; gsub(/version = |"/, ""); print; exit}' Cargo.lock)
bindgen=${WASM_BINDGEN:-$(command -v wasm-bindgen || true)}
if [ -z "$bindgen" ]; then
  echo "wasm-bindgen $lock_version is needed (cargo install wasm-bindgen-cli --version $lock_version --locked)" >&2
  exit 1
fi
have_version=$("$bindgen" --version | awk '{print $2}')
if [ "$have_version" != "$lock_version" ]; then
  echo "wasm-bindgen is $have_version, Cargo.lock has $lock_version: they must match" >&2
  exit 1
fi

cargo build "${cargo_flags[@]}" -p vessel-app --target wasm32-unknown-unknown
target_dir=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
module="$target_dir/wasm32-unknown-unknown/$profile/vessel-app.wasm"

out="$here/web/dist"
rm -rf "$out"
mkdir -p "$out"
"$bindgen" --target web --no-typescript --out-dir "$out" --out-name vessel-app "$module"

optimizer=${WASM_OPT:-}
if [ "$profile" = release ] && [ -n "$optimizer" ]; then
  # -Oz: size first (what a browser downloads); the feature flags are the
  # ones rustc's wasm32 target emits by default.
  "$optimizer" -Oz --strip-debug --enable-bulk-memory --enable-mutable-globals \
    --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types \
    --enable-multivalue \
    "$out/vessel-app_bg.wasm" -o "$out/vessel-app_bg.wasm.opt"
  mv "$out/vessel-app_bg.wasm.opt" "$out/vessel-app_bg.wasm"
fi

cp "$here/web/index.html" "$out/"
# The faces' licences travel with the module that embeds them.
cp "$here"/assets/fonts/*-OFL.txt "$out/"
ls -l "$out"
