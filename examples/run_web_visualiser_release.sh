#!/bin/sh

set -e;set -u

# TODO: if subaction_data.bin does not exist
# TODO:     request user run `cargo run --release --example export_subaction_for_wasm -- ...`

cd ..
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build --target wasm32-unknown-unknown --example visualiser --release
wasm-bindgen --out-dir target/generated --web target/wasm32-unknown-unknown/release/examples/visualiser.wasm
cp examples/index.html target/generated
cd target/generated
python3 -m http.server
