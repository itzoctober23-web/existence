#!/usr/bin/env bash
# P2 — THE SEARCH TRACK. MASTER_PLAN:276 calls it "open-ended, runs from day one".
#
# It was not running. `evolve_search.log` shows a run today that reached generation 13 and died
# mid-generation, and nothing restarted it. That is half the project's thesis sitting idle: the
# NET track (train an evaluator, gate it) has run all day and produced the 0.864 plateau, while
# the track that is supposed to LEARN THE SEARCH ITSELF had stopped after 12 completed
# generations.
#
# WHAT THE STOPPED RUN ACTUALLY SHOWED, before anyone reads improvement into a restart: 288
# candidates, all 24 per generation type-checking cleanly (0 ill-typed, so the mutation operators
# and GRAMMAR 3 type checker are doing their job), 8-14 passing the mate oracle each generation,
# and ZERO passing the surrogate. "none beat it" twelve times out of twelve. Single-edit mutations
# of a 71-node alpha-beta did not improve mates-per-cost even once. That is a real, if negative,
# reading and the restart does not erase it -- it extends it.
#
# CORES. 12-14, not 15: gate_ab and its scoring own core 15, and the 4PC benchmark is a MOVETIME
# match on P-cores 0-11 whose timing is the number it exists to produce. nice 19 + ionice idle so
# this yields to everything, including his desktop.
#
# The binary is NOT rebuilt here. ./target/release is shared with jobs that are mid-run, and a
# rebuild under a running job has already cost this project a gate at 323 pairs.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-400}
POP=${POP:-24}
LOG=${LOG:-search_track.log}

[ -x ./target/release/examples/evolve ] || { echo "no evolve binary; build it when nothing is running"; exit 1; }

echo "=== P2 search track: $GENS generations, population $POP, seeded from bare alpha-beta ===" >> "$LOG"
echo "=== started $(date +%F_%H:%M) — prior run reached gen 13 with 0 improvements ===" >> "$LOG"
exec taskset -c 12-14 nice -n 19 ionice -c 3 ./target/release/examples/evolve "$GENS" "$POP" >> "$LOG" 2>&1
