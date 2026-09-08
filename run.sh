#!/usr/bin/env bash
# Build-then-run, in ONE tree. Use this for every experiment launch.
#
# WHY THIS EXISTS. Experiments run from an isolated CARGO_TARGET_DIR so that rebuilding does not
# disturb a live training run, while check.sh builds the DEFAULT tree. Two trees, and nothing
# tied "the source I just edited" to "the binary that runs". It has cost twice:
#   - an hour of the search-track ledger recording a deliberately-broken negative control,
#     because I restored mutate.rs, ran check.sh, and relaunched from the isolated tree whose
#     last build was the broken one;
#   - a ladder re-run that printed numbers identical to 7 significant figures after a semantic
#     change, because `cargo build --release` does not rebuild examples.
# Restoring or editing source is not deploying it.
#
#   ./run.sh <package> <example> [args...]
set -euo pipefail
cd "$(dirname "$0")"
pkg=$1; ex=$2; shift 2
: "${XTREE:=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/t2}"
export CARGO_TARGET_DIR="$XTREE"
# --example is explicit: a bare `cargo build --release` skips examples entirely.
taskset -c 12-15 nice -n 19 cargo build --release --example "$ex" -p "$pkg" >/dev/null
bin="$XTREE/release/examples/$ex"
[ -x "$bin" ] || { echo "no binary at $bin after building"; exit 1; }
# Prove the binary is newer than every source file, so a silent build skip cannot pass.
newest=$(find crates -name '*.rs' -newer "$bin" -print -quit)
[ -z "$newest" ] || { echo "STALE: $newest is newer than $bin"; exit 1; }
exec taskset -c 12-15 nice -n 19 "$bin" "$@"
