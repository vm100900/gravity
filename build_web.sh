#!/bin/sh
# Builds the WebAssembly version into web/. Serve that folder with any static server, e.g.:
#   python3 -m http.server 8000 --directory web
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/gravity.wasm web/
echo "Built web/gravity.wasm — run: python3 -m http.server 8000 --directory web"
