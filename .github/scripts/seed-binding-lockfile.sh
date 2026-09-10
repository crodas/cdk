#!/usr/bin/env bash
#
# Seeds a downstream binding crate's Cargo.lock from the workspace lockfile, so
# every release leg builds against the dependency versions pinned at the tagged
# commit instead of re-resolving. Fails if any dependency the binding resolves
# to was not pinned by the workspace lock.
#
# Usage: seed-binding-lockfile.sh <downstream-rust-dir> [root-manifest]
#   run from the monorepo checkout root.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

BINDING_DIR="${1:?usage: seed-binding-lockfile.sh <downstream-rust-dir> [root-manifest]}"
ROOT_MANIFEST="${2:-Cargo.toml}"
ROOT_LOCK="$(dirname "${ROOT_MANIFEST}")/Cargo.lock"

if [[ ! -f "${ROOT_LOCK}" ]]; then
  echo "::error::${ROOT_LOCK} not found; the workspace lockfile must be committed"
  exit 1
fi

cp "${ROOT_LOCK}" "${BINDING_DIR}/Cargo.lock"

# `cargo fetch` resolves minimally against the lock it is handed, changing only
# the entries the rewritten manifest forces (cdk-ffi moves from a path dep to a
# registry or git source). `cargo update` and `cargo generate-lockfile` both
# re-resolve to latest and would silently undo the seeding.
( cd "${BINDING_DIR}" && cargo fetch )

cargo metadata --format-version 1 --locked --manifest-path "${BINDING_DIR}/Cargo.toml" > /dev/null

if ! "${SCRIPT_DIR}/compare-lockfiles.sh" "${ROOT_LOCK}" "${BINDING_DIR}" "$(git rev-parse HEAD)"; then
  echo
  echo "Re-run with the workspace lock updated, or pin the offending crate."
  exit 1
fi
