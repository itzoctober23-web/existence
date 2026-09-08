#!/usr/bin/env bash
# Pre-commit gate. Build ALL targets (examples included) before trusting any test result:
# a broken example makes `cargo test` compile-fail, print nothing, and a grep for "FAILED"
# then reports 0 failures. That exact sequence let a broken tree get committed once.
set -uo pipefail
cd "$(dirname "$0")"
# Report live runs, do NOT refuse. MEASURED 2026-09-07: a full check.sh rebuild ran while a
# 40-generation training run was in flight and the run kept producing generations -- cargo
# replaces the binary via rename, and the running process keeps its original inode (which then
# shows as "(deleted)" in /proc). Rebuilding under a Rust run is safe here. Refusing would only
# stop me gating during the long runs where gating matters most.
#
# NOTE THE PATTERN. `readlink /proc/N/exe` returns "<path> (deleted)" once the file is replaced,
# so a `case "$e" in */learn)` match silently fails and reports the process as GONE. That is how
# I concluded a rebuild had killed a run that was in fact still running -- a broken predicate
# read as an absence, which is its own entry on the do-not-repeat list.
for p in /proc/[0-9]*; do
  e=$(readlink "$p/exe" 2>/dev/null) || continue
  e=${e% (deleted)}
  case "$e" in
    */existence/target/release/learn|*/existence/target/release/examples/*)
      echo "note: run live (pid ${p#/proc/}, $(basename "$e")) — rebuilding is safe, it keeps its inode";;
  esac
done

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
[ "${n:-0}" -ge 34 ] || { echo "  EXPECTED >=34 tests, got ${n:-0} — did they compile?"; exit 1; }

# The movegen cross-check must actually RUN, not silently skip. The test passes when the
# reference engine is absent (so the repo stays testable without Stockfish), which would be a
# hole in the gate — so assert the marker here. A check that can quietly not-run is not a check.
echo "== movegen cross-check vs external engine =="
xc=$(cargo test --release -p board --test xcheck -- --nocapture 2>&1)
if line=$(grep -m1 "XCHECK-RAN" <<<"$xc"); then
  echo "  ${line#XCHECK-RAN: }"
  grep -q "0 divergences" <<<"$line" || { echo "  DIVERGENCES FOUND"; exit 1; }
else
  echo "  NOT RUN — $(grep -m1 'XCHECK-SKIPPED' <<<"$xc" || echo 'no marker at all')"
  echo "  install stockfish or set XCHECK_ENGINE; half (b) of CRATE.md 2 is not being verified"
  exit 1
fi

echo "ALL GREEN"
