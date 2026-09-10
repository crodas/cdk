#!/usr/bin/env bash
#
# Checks, after the fact, that a published binding release resolved only
# dependency versions that the monorepo's Cargo.lock pinned at the commit the
# release claims to come from.
#
# Usage: verify-binding-lockfile.sh <language> <release-tag>
#   run from the monorepo checkout root. Needs `gh` and a checkout at the
#   source commit recorded in the release's build-manifest.json.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

LANGUAGE="${1:?usage: verify-binding-lockfile.sh <language> <release-tag>}"
TAG="${2:?missing release tag}"

case "${LANGUAGE}" in
  dart|swift|kotlin|go) ;;
  *) echo "unknown language: ${LANGUAGE}" >&2; exit 2 ;;
esac

REPO="cashubtc/cdk-${LANGUAGE}"
workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

git clone --quiet --depth 1 --branch="${TAG}" \
  -- "https://github.com/${REPO}.git" "${workdir}/downstream"

MANIFEST="${workdir}/downstream/build-manifest.json"
if [[ ! -f "${MANIFEST}" ]]; then
  echo "::error::${REPO}@${TAG} has no build-manifest.json; it predates provenance recording"
  exit 1
fi

CLAIMED_COMMIT="$(jq -r '.source.commit' "${MANIFEST}")"
HEAD_COMMIT="$(git rev-parse HEAD)"
if [[ "${CLAIMED_COMMIT}" != "${HEAD_COMMIT}" ]]; then
  echo "::error::checkout mismatch: release was built from ${CLAIMED_COMMIT}, this tree is ${HEAD_COMMIT}"
  echo "Run: git checkout ${CLAIMED_COMMIT}"
  exit 1
fi

CLAIMED_WS_LOCK="$(jq -r '.lockfiles.workspace_cargo_lock_sha256' "${MANIFEST}")"
ACTUAL_WS_LOCK="$(shasum -a 256 Cargo.lock 2>/dev/null | cut -d' ' -f1)"
if [[ "${CLAIMED_WS_LOCK}" != "${ACTUAL_WS_LOCK}" ]]; then
  echo "::error::workspace Cargo.lock hash mismatch"
  echo "  manifest: ${CLAIMED_WS_LOCK}"
  echo "  checkout: ${ACTUAL_WS_LOCK}"
  exit 1
fi

BINDING_LOCK="${workdir}/downstream/rust/Cargo.lock"
if [[ ! -f "${BINDING_LOCK}" ]]; then
  # Go builds cdk-ffi from the monorepo, so the workspace lock hash checked
  # above is the whole guarantee. Nothing further to compare.
  echo "${LANGUAGE}: no downstream lockfile; verified against the workspace lock."
  exit 0
fi

"${SCRIPT_DIR}/compare-lockfiles.sh" Cargo.lock "${workdir}/downstream/rust" "${CLAIMED_COMMIT}"
echo "${LANGUAGE} ${TAG}: verified against the workspace lock at ${CLAIMED_COMMIT}."
