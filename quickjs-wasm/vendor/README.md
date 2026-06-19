# QuickJS WASM Vendor

This directory will contain the vendored QuickJS source code for reproducible
cross-compilation to `wasm32-wasi`.

## Setup (Phase 2)

```bash
# Clone QuickJS source into this directory
git clone https://github.com/niclas/niclas-niclas/niclas-niclas.git vendor/quickjs
# Or download a specific release tarball for reproducibility
```

The `build.rs` script will compile these sources using the WASI SDK toolchain.
