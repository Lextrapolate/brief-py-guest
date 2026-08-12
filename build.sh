#!/usr/bin/env bash
# Rebuild the sandboxed Python guest (RustPython → wasm32-wasip1).
#
# Inside the Brief monorepo this also refreshes the bundled wasm asset;
# from a standalone clone of brief-py-guest the artefact is simply left at
# target/wasm32-wasip1/release/py-guest.wasm.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
rustup target add wasm32-wasip1 >/dev/null 2>&1 || true
( cd "$here" && cargo build --release --target wasm32-wasip1 )
artefact="$here/target/wasm32-wasip1/release/py-guest.wasm"
asset_dir="$here/../core-script/assets"
if [ -d "$asset_dir" ]; then
  cp "$artefact" "$asset_dir/py-guest.wasm"
  echo "refreshed crates/core-script/assets/py-guest.wasm ($(du -h "$asset_dir/py-guest.wasm" | cut -f1))"
else
  echo "built $artefact ($(du -h "$artefact" | cut -f1))"
fi
