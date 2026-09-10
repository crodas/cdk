#!/usr/bin/env bash
#
# Compares a downstream binding's Cargo.lock against the workspace lockfile.
#
# Third-party crates are matched on source and checksum as well as version, so
# a swapped registry, a git URL substituted for crates.io, or a rewritten
# checksum shows up as drift. Workspace members are matched on version only,
# since they legitimately move from a path dependency to a registry or git one
# downstream, but their downstream source must be crates.io or the monorepo at
# the expected commit. A git source pins a content-addressed sha, so the commit
# suffix is the guarantee and the repository URL does not need pinning.
#
# Usage: compare-lockfiles.sh <workspace-lock> <binding-dir> <expected-commit>

set -euo pipefail
export LC_ALL=C

WORKSPACE_LOCK="${1:?usage: compare-lockfiles.sh <workspace-lock> <binding-dir> <expected-commit>}"
BINDING_DIR="${2:?missing binding dir}"
EXPECTED_COMMIT="${3:?missing expected commit}"

BINDING_LOCK="${BINDING_DIR}/Cargo.lock"
BINDING_MANIFEST="${BINDING_DIR}/Cargo.toml"
CRATES_IO="registry+https://github.com/rust-lang/crates.io-index"

for f in "${WORKSPACE_LOCK}" "${BINDING_LOCK}" "${BINDING_MANIFEST}"; do
  if [[ ! -f "${f}" ]]; then
    echo "::error::${f} not found" >&2
    exit 1
  fi
done

if [[ ! "${EXPECTED_COMMIT}" =~ ^[0-9a-f]{40}$ ]]; then
  echo "::error::expected commit '${EXPECTED_COMMIT}' is not a full sha" >&2
  exit 1
fi

packages() {
  awk -F'"' '
    function emit() {
      if (pkg && name != "" && ver != "") printf "%s\t%s\t%s\t%s\n", name, ver, src, sum
      name = ""; ver = ""; src = "-"; sum = "-"
    }
    /^\[\[package\]\]/ { emit(); pkg = 1; next }
    /^\[/              { emit(); pkg = 0; next }
    pkg && /^name = "/     { name = $2; next }
    pkg && /^version = "/  { ver  = $2; next }
    pkg && /^source = "/   { src  = $2; next }
    pkg && /^checksum = "/ { sum  = $2; next }
    END { emit() }
  ' "$1"
}

WRAPPER="$(awk -F'"' '
  /^\[package\]/ { p = 1; next }
  /^\[/          { p = 0 }
  p && /^name = "/ { print $2; exit }
' "${BINDING_MANIFEST}")"

if [[ -z "${WRAPPER}" ]]; then
  echo "::error::cannot read the package name from ${BINDING_MANIFEST}" >&2
  exit 1
fi

workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

packages "${WORKSPACE_LOCK}" > "${workdir}/ws.tsv"
packages "${BINDING_LOCK}"   > "${workdir}/bd.tsv"

{
  printf 'E\tcdk-ffi\nE\t%s\n' "${WRAPPER}"
  awk -F'\t' '$3 == "-" { print "M\t" $1 }' "${workdir}/ws.tsv"
} | sort -u > "${workdir}/classes.tsv"

awk -F'\t' -v third="${workdir}/bd-third-party.tsv" \
           -v member="${workdir}/bd-members.tsv" \
           -v path_dep="${workdir}/bd-path-deps.tsv" '
  NR == FNR   { if ($1 == "E") excluded[$2] = 1; else member_of_ws[$2] = 1; next }
  $1 in excluded     { next }
  $3 == "-"          { print > path_dep; next }
  $1 in member_of_ws { print > member; next }
                     { print > third }
' "${workdir}/classes.tsv" "${workdir}/bd.tsv"

for f in bd-third-party bd-members bd-path-deps; do
  touch "${workdir}/${f}.tsv"
  sort -u -o "${workdir}/${f}.tsv" "${workdir}/${f}.tsv"
done

failed=0

if [[ -s "${workdir}/bd-path-deps.tsv" ]]; then
  echo "::error::the binding lockfile contains path dependencies other than ${WRAPPER}"
  awk -F'\t' '{ print "  " $1 " " $2 }' "${workdir}/bd-path-deps.tsv"
  failed=1
fi

awk -F'\t' '$3 != "-"' "${workdir}/ws.tsv" | sort -u > "${workdir}/ws-third-party.tsv"
drift="$(comm -13 "${workdir}/ws-third-party.tsv" "${workdir}/bd-third-party.tsv")"
if [[ -n "${drift}" ]]; then
  echo "::error::third-party dependencies the workspace lockfile did not pin"
  echo "${drift}" | awk -F'\t' '{ printf "  %s %s\n    source:   %s\n    checksum: %s\n", $1, $2, $3, $4 }'
  failed=1
fi

awk -F'\t' '
  NR == FNR { if ($1 == "E") excluded[$2] = 1; else member_of_ws[$2] = 1; next }
  ($1 in member_of_ws) && !($1 in excluded) { print $1 "\t" $2 }
' "${workdir}/classes.tsv" "${workdir}/ws.tsv" | sort -u > "${workdir}/ws-member-versions.tsv"
cut -f1,2 "${workdir}/bd-members.tsv" | sort -u > "${workdir}/bd-member-versions.tsv"

drift="$(comm -13 "${workdir}/ws-member-versions.tsv" "${workdir}/bd-member-versions.tsv")"
if [[ -n "${drift}" ]]; then
  echo "::error::workspace crates resolved to versions the workspace lockfile did not pin"
  echo "${drift}" | sed 's/^/  /'
  failed=1
fi

bad_source="$(awk -F'\t' -v io="${CRATES_IO}" -v commit="${EXPECTED_COMMIT}" '
  $3 == io                            { next }
  $3 ~ /^git\+/ && $3 ~ ("#" commit "$") { next }
  { printf "  %s %s\n    source: %s\n", $1, $2, $3 }
' "${workdir}/bd-members.tsv")"
if [[ -n "${bad_source}" ]]; then
  echo "::error::workspace crates came from somewhere other than crates.io or ${EXPECTED_COMMIT}"
  echo "${bad_source}"
  failed=1
fi

if (( failed )); then
  exit 1
fi

printf '%s third-party and %s workspace crates match the workspace lockfile.\n' \
  "$(wc -l < "${workdir}/bd-third-party.tsv" | tr -d ' ')" \
  "$(wc -l < "${workdir}/bd-members.tsv" | tr -d ' ')"
