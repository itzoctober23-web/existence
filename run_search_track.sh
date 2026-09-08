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
# SIZED FROM A MEASUREMENT, not a guess. At the previous settings (80 mate-in-1 + 40 mate-in-2,
# pop 24) ONE generation took 330 SECONDS -- timed directly rather than inferred -- so the
# 400-generation run would have taken 37 hours, and the loop printed only every 10th barren
# generation so 55 minutes of silence looked identical to a hang.
#
# Cost is per candidate-evaluation: pop x (N1 + N2) positions at ~0.115 s each. pop 12 over 60
# positions is ~83 s per generation, a ~4x speedup, and it also updates the champion 2x more often
# -- for a (1+lambda) hill climb, more frequent steps beat a wider sample per step once lambda is
# already comfortably above 1.
#
# N2 (the depth-requiring half) is NOT cut proportionally. It is the guard that makes "must not
# lose mates" bite at all; without it the surrogate is maximised by searching less, which is
# exactly the degenerate 333x accept this loop produced before the mixed set was added.
GENS=${GENS:-2000}
POP=${POP:-12}
# SET SHRUNK because the fitness depth moved 2 -> 3, which costs ~11x per position. The GRAMMAR 9
# ladder has exactly one verified ascent step from the seed (hash reuse, 0.98x the seed's cost for
# identical mates) and it exists ONLY at D=3; at D=2 that same rung measures 1.01x, a loss. This
# track was evaluating at D=2 -- the one depth where the single known improvement is invisible --
# and has accepted zero of ~690 candidates.
#
# Fitness here is DETERMINISTIC (fixed positions, fixed net, no sampling), so a 2% cost difference
# is exact at any set size. A smaller set samples fewer positions; it does not add noise.
N1=${N1:-15}
N2=${N2:-5}
DEPTH=${DEPTH:-3}
LOG=${LOG:-search_track.log}

[ -x ./target/release/examples/evolve ] || { echo "no evolve binary; build it when nothing is running"; exit 1; }

echo "=== P2 search track: $GENS generations, pop $POP, set ${N1}+${N2} at depth ${DEPTH}, seeded from bare alpha-beta ===" >> "$LOG"
echo "=== started $(date +%F_%H:%M) — prior run reached gen 13 with 0 improvements ===" >> "$LOG"
exec taskset -c 12-14 nice -n 19 ionice -c 3 ./target/release/examples/evolve "$GENS" "$POP" "$N1" "$N2" "$DEPTH" >> "$LOG" 2>&1
