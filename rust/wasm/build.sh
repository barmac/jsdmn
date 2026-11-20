#!/usr/bin/env sh
# Build the WebAssembly package.
#
#   wasm/www/pkg   ES module (used by the browser example in wasm/www and,
#                  through `initSync`, by the Node.js tests in wasm/test)
#
# Requires the wasm32-unknown-unknown target and the wasm-bindgen CLI in the
# version of the wasm-bindgen crate:
#
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version 0.2.130 --locked
set -e
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown -p rsdmn-wasm
WASM=target/wasm32-unknown-unknown/release/rsdmn_wasm.wasm
wasm-bindgen --target web --remove-name-section --remove-producers-section --out-dir wasm/www/pkg "$WASM"
ls -l wasm/www/pkg/*.wasm
