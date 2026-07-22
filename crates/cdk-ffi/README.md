# CDK FFI Bindings

UniFFI bindings for the CDK (Cashu Development Kit), providing foreign function interface access to wallet functionality for multiple programming languages.

## Supported Languages

- **🐍 Python** - With REPL integration for development
- **🍎 Swift** - iOS and macOS development
- **🎯 Kotlin** - Android and JVM development

## Development Tasks

### Build & Check
```bash
just ffi-build        # Build FFI library (release)
just ffi-build --debug # Build debug version
just ffi-check         # Check compilation
just ffi-clean         # Clean build artifacts
```

### Generate Bindings
```bash
# Generate for specific languages
just ffi-generate python
just ffi-generate swift
just ffi-generate kotlin

# Generate all languages
just ffi-generate-all

# Use --debug for faster development builds
just ffi-generate python --debug
```

### Development & Testing
```bash
# Python development with REPL
just ffi-dev-python    # Generates bindings and opens Python REPL with cdk_ffi loaded

# Test bindings
just ffi-test-python   # Test Python bindings import
just ffi-test-live-python # Run live Python test against testnut.cashudevkit.org
```

## Quick Start

```bash
# Start development
just ffi-dev-python

# In the Python REPL:
>>> dir(cdk_ffi)  # Explore available functions
>>> help(cdk_ffi.generate_mnemonic)  # Get help
```

## Live Tests

The live Python test in `tests/test_live_async_onchain_melt.py` covers
`PreparedMelt.confirm_prefer_async()`, immediate and pending melt outcomes,
`PendingMelt.wait()`, and `Wallet.finalize_pending_melts()` against
`https://testnut.cashudevkit.org`.

## Language Packages

For production use, install from the published package registries:

| Language | Distribution | Coordinates |
|---|---|---|
| Kotlin / Android / JVM | [Maven Central](https://central.sonatype.com/namespace/org.cashudevkit) | `org.cashudevkit:cdk-android`, `org.cashudevkit:cdk-jvm`, `org.cashudevkit:cdk-jvm-natives` |
| Swift | Swift Package Manager | [cdk-swift](https://github.com/cashubtc/cdk-swift) |
| Go | Go modules | [cdk-go](https://github.com/cashubtc/cdk-go) |
| Python | PyPI | [cdk-python](https://github.com/cashubtc/cdk-python) |

The Kotlin artifacts are published directly to Maven Central under the
`org.cashudevkit` namespace. Android apps depend on `cdk-android`; desktop JVM
apps depend on `cdk-jvm` plus `cdk-jvm-natives`. See
[`bindings/kotlin/README.md`](../../bindings/kotlin/README.md) for coordinates
and install snippets. The `cashubtc/cdk-kotlin` repository is only the internal
release target for the publish workflow; consumers should not depend on it
directly.
