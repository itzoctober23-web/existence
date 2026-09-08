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
# Prove the binary is newer than every source it ACTUALLY depends on, so a silent build skip
# cannot pass.
#
# The first version compared against every *.rs under crates/, which is a different question and
# answers it wrongly. `hyper_ab` is an example and does not depend on pipeline/src/main.rs, a
# separate bin target -- so editing main.rs made a correctly-built example look stale and this
# script refused to launch it. A whole datagen-depth sweep silently never started that way.
#
# Cargo already computes the exact answer and writes it to <target>.d: the target's real
# transitive source deps (24 files for hyper_ab; main.rs is not among them). Using it is both
# stricter and quieter -- it still catches the two failures above, since a wrong-tree or
# never-built-example run has no depfile at all. A missing or empty depfile is an ERROR, not a
# pass: a check that can quietly not-run is not a check.
dep="${bin}.d"
[ -f "$dep" ] || { echo "no depfile at $dep — cannot prove the binary matches its source"; exit 1; }
srcs=$(sed 's/^[^:]*://' "$dep" | tr ' ' '\n' | grep '\.rs$' || true)
[ -n "$srcs" ] || { echo "depfile $dep lists no sources — refusing to trust it"; exit 1; }
# NOTE the shell subtlety. Written as `[ -e "$s" ] && [ "$s" -nt "$bin" ] && { ...exit... }`,
# a source that simply does not exist makes the whole && list return 1, and under `set -e` that
# terminates the script -- a guard that aborts the run it was meant to protect, reported as if
# the launch had failed. Same family as the pipefail bug that had check.sh printing "ok" over a
# failing build. Explicit `if` has no such reading.
#
# WHOLE-SECOND tolerance, and it is not sloppiness. `-nt` compares to nanosecond precision, and
# MEASURED after a clean build: crates/nnue/src/lib.rs 07:58:30.760 vs the example binary
# 07:58:30.730 -- the source is 30ms "newer" than a binary cargo considers fresh, purely from
# ordering inside one build. A guard that fires on that blocks a correct launch, which is the
# failure mode this check has ALREADY caused once. Cargo's dependency graph is authoritative for
# freshness; this is a secondary net for the case cargo never ran at all, so it only needs to
# catch whole-second staleness. stat %Y is seconds, so same-build skew collapses to 0.
while read -r s; do
  if [ -e "$s" ] && [ "$s" -nt "$bin" ] &&
     [ $(( $(stat -c %Y "$s") - $(stat -c %Y "$bin") )) -ge 1 ]; then
    echo "STALE: $s is newer than $bin"; exit 1
  fi
done <<<"$srcs"
exec taskset -c 12-15 nice -n 19 "$bin" "$@"
