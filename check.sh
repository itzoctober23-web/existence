#!/usr/bin/env bash
# Pre-commit gate. Build ALL targets (examples included) before trusting any test result:
# a broken example makes `cargo test` compile-fail, print nothing, and a grep for "FAILED"
# then reports 0 failures. That exact sequence let a broken tree get committed once.
set -uo pipefail
cd "$(dirname "$0")"
echo "== build (all targets) =="
# Check CARGO's exit status, not a grep's. The previous form was
#   if ! cargo build ... | grep -E "^error"; then echo ok
# and under `set -o pipefail` the pipeline's status is cargo's (101 on failure), not grep's,
# so a BROKEN BUILD took the `!`-true branch and printed "ok". It reported a green build stage
# while the compiler was printing errors on the same screen.
build_out=$(cargo build --release --workspace --all-targets 2>&1)
if [ $? -ne 0 ]; then
  echo "  BUILD FAILED"; grep -E "^error" <<<"$build_out" | head; exit 1
fi
echo "  ok"
echo "== tests =="
out=$(cargo test --release --workspace 2>&1)
if grep -qE "FAILED|panicked|^error" <<<"$out"; then
  echo "$out" | grep -E "FAILED|panicked|^error" | head; exit 1
fi
n=$(grep -oE "test result: ok\. [0-9]+ passed" <<<"$out" | grep -oE "[0-9]+" | paste -sd+ | bc)
echo "  $n tests passed"
# Ratchet: the floor is the count at the last commit, so a test that silently stops being
# compiled (or gets deleted) fails the gate instead of passing a smaller suite quietly.
[ "${n:-0}" -ge 33 ] || { echo "  EXPECTED >=33 tests, got ${n:-0} — did they compile?"; exit 1; }
echo "ALL GREEN"
