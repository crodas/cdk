#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <shared-library.so>" >&2
  exit 2
fi

file="$1"
# The NDK ships one prebuilt toolchain per host; CI is linux-x86_64 but a
# developer checking locally may be on macOS.
prebuilt="${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt"
objdump=""
for host in linux-x86_64 darwin-x86_64 darwin-arm64 windows-x86_64; do
  if [[ -x "${prebuilt}/${host}/bin/llvm-objdump" ]]; then
    objdump="${prebuilt}/${host}/bin/llvm-objdump"
    break
  fi
done
if [[ -z "$objdump" ]]; then
  echo "no llvm-objdump under ${prebuilt}" >&2
  exit 1
fi

load_segments="$("$objdump" -p "$file" | grep '^[[:space:]]*LOAD' || true)"
if [[ -z "$load_segments" ]]; then
  echo "no LOAD segments found in $file" >&2
  exit 1
fi

failed=0
while IFS= read -r line; do
  align="${line##*align }"
  if [[ "$align" =~ ^2\*\*([0-9]+)$ ]]; then
    exponent="${BASH_REMATCH[1]}"
  else
    echo "could not parse LOAD segment alignment: $line" >&2
    failed=1
    continue
  fi

  if (( exponent < 14 )); then
    echo "LOAD segment is not 16 KB aligned: $line" >&2
    failed=1
  fi
done <<< "$load_segments"

if (( failed )); then
  exit 1
fi

echo "$file has 16 KB ELF LOAD alignment"
