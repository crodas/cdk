# CDK – Cashu Development Kit for Dart

Dart bindings for [CDK](https://github.com/cashubtc/cdk), a Cashu protocol implementation.

## Installation

Add to your `pubspec.yaml`:

```yaml
dependencies:
  cdk:
    git:
      url: https://github.com/cashubtc/cdk-dart
      ref: vX.Y.Z  # replace with desired version
```

## Requirements

- Dart SDK `^3.10.0`
- Network access to `github.com` on the first build of a project, to fetch the native library

## Usage

```dart
import 'package:cdk/cdk.dart';
```

## Native library

There is no compilation step and no Rust toolchain involved. The build hook
fetches the native library for your target on the first build of a project,
verifies it against the sha256 recorded in `prebuilt_manifest.json`, and caches
it under `.dart_tool/`. Later builds reuse the cache and do no network work.

The library is published as a release asset built from the CDK monorepo at the
commit this version was tagged from, so the binary and the Dart bindings in this
package always come from the same tree.

Supported targets:

| Platform | Architecture |
|----------|-------------|
| Linux | x86_64, aarch64 |
| macOS | aarch64, x86_64 |
| Windows | x86_64 |
| Android | aarch64, armv7, x86_64 |
| iOS | aarch64 |

Each target ships the flavour Dart asks for on that platform: dynamic
everywhere except iOS, which is statically linked. A request for the other
flavour fails with the triple and filename it wanted, rather than falling back
to a long build.

## Offline and restricted networks

Build hooks receive a filtered environment, so `HTTP_PROXY` and friends do not
reach this one and the download cannot be routed through a proxy. Where egress
to `github.com` is unavailable, pre-seed the library and point the hook at it
from the **consuming app's** `pubspec.yaml`:

```yaml
hooks:
  user_defines:
    cdk:
      prebuilt_dir: third_party/cdk-prebuilt
```

The directory is laid out `<target-triple>/<library-file>`, matching the release
asset names, so seeding it is one download per target you build for. Relative
paths resolve against the pubspec that declares them.

There is no build-from-source option here, because this package ships no Rust
sources. `force_build: true` exists for the same reason `prebuilt_dir` does, but
it only has an effect when the package is consumed as a path dependency on a CDK
monorepo checkout, where `rust/` is present. Anywhere else the hook reports the
triple and filename it could not find.

## CI/CD — Publishing Workflow

The `dart-publish.yml` workflow (in the CDK monorepo) builds native binaries,
syncs sources to `cdk-dart`, and creates a tagged release. The following secrets
and variables must be configured in the **CDK monorepo** repository settings
(Settings → Secrets and variables → Actions).

### Secrets

| Name | Purpose |
|---|---|
| `FFI_DEPLOY_KEY` | Personal access token (PAT) with `repo` scope on the FFI target repos (`cdk-dart`, `cdk-kotlin`, `cdk-swift`). Used to clone, push, and create releases. Shared across all FFI publish workflows. |

#### How to create the PAT

1. Go to **GitHub → Settings → Developer settings → Personal access tokens → Fine-grained tokens**.
2. Create a token scoped to the `cdk-dart`, `cdk-kotlin`, and `cdk-swift` repositories with **Contents** (read/write) and **Metadata** (read) permissions.
3. Add it as a repository secret named `FFI_DEPLOY_KEY` in the monorepo.

### Variables

| Name | Purpose | Example |
|---|---|---|
| `CDK_DART_REPO` | Owner/repo of the target Dart package repository. | `cashubtc/cdk-dart` |

Set this under **Settings → Secrets and variables → Actions → Variables**.

## Testing

By default, running tests will skip live mint integration tests to allow offline/local testing:

```bash
dart test
```

To run the live mint integration tests, provide the `CDK_DART_TEST_MINT_URL` environment variable:

```bash
CDK_DART_TEST_MINT_URL=https://testnut.cashudevkit.org dart test
```

If the mint has a slower auto-payment settlement, you can optionally configure the settlement delay (in seconds):

```bash
CDK_DART_TEST_MINT_URL=https://testnut.cashudevkit.org CDK_DART_MINT_SETTLEMENT_DELAY_SECONDS=5 dart test
```

## License

[MIT](https://github.com/cashubtc/cdk/blob/main/LICENSE)

