#!/bin/sh
set -eu

cd "$(dirname "$0")/../groundstation"

bindgen=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p;}' Cargo.lock)
[ "$(wasm-bindgen --version 2>/dev/null)" = "wasm-bindgen $bindgen" ] || cargo install wasm-bindgen-cli --version "$bindgen" --locked

archiver=${AR_wasm32_unknown_unknown:-$(command -v llvm-ar || ls /opt/homebrew/opt/llvm*/bin/llvm-ar 2>/dev/null | head -n 1 || true)}
[ -n "$archiver" ] || { echo "llvm-ar is needed to archive SQLite for wasm: brew install llvm, or apt install llvm" >&2; exit 1; }

rustup target add wasm32-unknown-unknown
AR_wasm32_unknown_unknown=$archiver cargo rustc --lib --release --target wasm32-unknown-unknown --crate-type cdylib
wasm-bindgen --target web --out-dir ../web/src/core/pkg target/wasm32-unknown-unknown/release/groundstation.wasm
