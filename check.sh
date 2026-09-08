#!/usr/bin/env bash
# Pre-commit gate. Build ALL targets (examples included) before trusting any test result:
# a broken example makes `cargo test` compile-fail, print nothing, and a grep for "FAILED"
# then reports 0 failures. That exact sequence let a broken tree get committed once.
set -uo pipefail
cd "$(dirname "$0")"
echo "== build (all targets) =="
if ! cargo build --release --workspace --all-targets 2>&1 | grep -E "^error" ; then
  echo "  ok"
else
  echo "  BUILD FAILED"; exit 1
fi
echo "== tests =="
out=$(cargo test --release --workspace 2>&1)
if grep -qE "FAILED|panicked|^error" <<<"$out"; then
  echo "$out" | grep -E "FAILED|panicked|^error" | head; exit 1
fi
n=$(grep -oE "test result: ok\. [0-9]+ passed" <<<"$out" | grep -oE "[0-9]+" | paste -sd+ | bc)
echo "  $n tests passed"
[ "${n:-0}" -ge 9 ] || { echo "  EXPECTED >=9 tests, got ${n:-0} — did they compile?"; exit 1; }
echo "ALL GREEN"
