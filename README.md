# brief-py-guest

The Python calculation sandbox used by [Brief](https://lextrapolate.com):
[RustPython](https://github.com/RustPython/RustPython) compiled to a
WASI (`wasm32-wasip1`) module, executed by the Brief application inside
[wasmtime](https://wasmtime.dev) with no filesystem preopens, no network,
fuel metering and a memory cap. The guest receives a script on stdin and
returns printed output; a small `brief` module (provided by the host)
is its only I/O.

## Why this repository is public

Brief itself is proprietary, but this guest is a **separate program**
distributed alongside it, and it links LGPL-3.0-only components
(the `malachite` big-integer crates, via RustPython's bignum backend).
Publishing the guest's complete source and build recipe satisfies the
LGPL's requirement that users be able to modify the LGPL components and
rebuild the combined work:

```sh
./build.sh   # requires the wasm32-wasip1 target: rustup target add wasm32-wasip1
```

The result is functionally identical to the artefact shipped in Brief,
whose SHA-256 is pinned in Brief's CI. Byte-for-byte reproduction
additionally requires the pinned toolchain (rust-toolchain.toml) and the
same build path — Rust embeds source paths in the artefact, so builds
from a different directory differ in path metadata only. Licences of all
components are listed in Brief's THIRD-PARTY-NOTICES.md; this wrapper's
own code is MIT.

## Licence

The wrapper code in `src/` is MIT (see LICENSE). Dependencies carry their
own licences, including LGPL-3.0-only for the malachite crates.
