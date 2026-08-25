#!/usr/bin/env bash
#
# Records what a binding release was built from, so the dependency graph and
# toolchain can be checked after the fact without re-running the release.
#
# Usage: write-build-manifest.sh <language> <downstream-dir> <output-path>
#   run from the monorepo checkout root. Reads TAG, CDK_REF and
#   GITHUB_REPOSITORY from the environment.

set -euo pipefail

LANGUAGE="${1:?usage: write-build-manifest.sh <language> <downstream-dir> <output-path>}"
DOWNSTREAM_DIR="${2:?missing downstream dir}"
OUTPUT="${3:?missing output path}"

sha256() {
  if [[ ! -f "$1" ]]; then
    echo "null"
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

RUST_CHANNEL="$(grep '^channel' rust-toolchain.toml | cut -d'"' -f2)"
WORKSPACE_LOCK="$(sha256 Cargo.lock)"
BINDING_LOCK="$(sha256 "${DOWNSTREAM_DIR}/rust/Cargo.lock")"
FLAKE_LOCK="$(sha256 flake.lock)"

# The binding lock is absent for Go, which builds cdk-ffi straight from the
# monorepo and therefore uses the workspace lock directly.
jq -n \
  --arg language "${LANGUAGE}" \
  --arg tag "${TAG:-}" \
  --arg repo "${GITHUB_REPOSITORY:-cashubtc/cdk}" \
  --arg commit "${CDK_REF:-}" \
  --arg rust_channel "${RUST_CHANNEL}" \
  --arg workspace_lock "${WORKSPACE_LOCK}" \
  --arg binding_lock "${BINDING_LOCK}" \
  --arg flake_lock "${FLAKE_LOCK}" \
  '{
     schema_version: 1,
     language: $language,
     release_tag: $tag,
     source: { repo: $repo, commit: $commit },
     toolchain: { rust_channel: $rust_channel },
     lockfiles: {
       workspace_cargo_lock_sha256: $workspace_lock,
       binding_cargo_lock_sha256: (if $binding_lock == "null" then null else $binding_lock end),
       flake_lock_sha256: $flake_lock
     }
   }' > "${OUTPUT}"

cat "${OUTPUT}"
