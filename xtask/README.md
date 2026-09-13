# xtask

Repository automation, run as `cargo xtask <command>`.

| Command | What it does |
|---|---|
| `bindings` | Regenerates the React Native Nitro bindings from the UniFFI metadata |
| `check-nitro` | Type-checks the generated adapters against the Nitro and JSI headers |
| `test-nitro` | Builds and runs the C++ harness over the generated bridge |
| `test-node` | Runs the Node harness and the cashu-ts parity tests |
| `bench-nitro` | Compares cashu-ts against the Rust implementation |
| `ios` | Builds the Rust library for the iOS targets and assembles an XCFramework |
| `android` | Builds the Rust library for every Android ABI into `jniLibs` |
