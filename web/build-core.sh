#!/bin/sh
set -eu

cd "$(dirname "$0")/../groundstation"

bindgen=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p;}' Cargo.lock)
[ "$(wasm-bindgen --version 2>/dev/null)" = "wasm-bindgen $bindgen" ] || cargo install wasm-bindgen-cli --version "$bindgen" --locked

archiver=${AR_wasm32_unknown_unknown:-$(command -v llvm-ar || ls /opt/homebrew/opt/llvm*/bin/llvm-ar 2>/dev/null | head -n 1 || true)}
[ -n "$archiver" ] || { echo "llvm-ar is needed to archive SQLite for wasm: brew install llvm, or apt install llvm" >&2; exit 1; }

params=../.cache/CPM/ardupilotparams
if [ -z "${QGC_APM_PARAMS_DIR:-}" ] && ! ls -d "$params"/*/Copter-4.5 >/dev/null 2>&1; then
  git clone --depth 1 https://github.com/ArduPilot/ParameterRepository "$params/main"
fi

rustup target add wasm32-unknown-unknown
built=$(AR_wasm32_unknown_unknown=$archiver cargo rustc --lib --profile wasm --target wasm32-unknown-unknown --crate-type cdylib --features synthetic --message-format=json-render-diagnostics \
  | jq -r 'select(.reason == "build-script-executed" and (.package_id | test("/groundstation#"))) | .out_dir')
[ -n "$built" ] || { echo "the core build failed or reported no build-script output" >&2; exit 1; }
pkg=../web/src/core/pkg
wasm-bindgen --target web --remove-name-section --out-dir "$pkg" target/wasm32-unknown-unknown/wasm/groundstation.wasm
rm -rf "$pkg/apm"
mkdir -p "$pkg/apm"
cp "$built"/*.json.gz "$pkg/apm/"
cp src/synthetic/facades/colour.jpg "$pkg/facade-colour.jpg"
