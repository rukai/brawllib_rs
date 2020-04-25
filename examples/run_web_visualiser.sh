#!/bin/sh

#　気を付けて
set -e;set -u

cd ..
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build --target wasm32-unknown-unknown --example visualiser
wasm-bindgen --out-dir target/generated --web target/wasm32-unknown-unknown/debug/examples/visualiser.wasm
cp examples/index.html target/generated
cd target/generated
python3 -m http.server
