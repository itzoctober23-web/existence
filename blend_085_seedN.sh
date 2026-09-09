#!/usr/bin/env bash
# BLEND 0.85 vs the shipped 0.75 — one training seed per invocation, seed-scoped filenames.
#
# WHY MORE SEEDS. Two seeds put 0.85-over-0.75 at mean +0.0685, 95% CI [+0.011, +0.126] using the
# pooled between-seed sd, against a power requirement of ~2.9 seeds. That clears zero by 0.011 —
# at the boundary, not past it. This project has withdrawn two candidates today that looked this
# good with one fewer seed (the "+0.025 depth lever" and blend 1.00's "depth-2 only" mechanism), so
# the honest response to a boundary result is more seeds, not a louder claim.
#
# WHY A NEW SCRIPT. `blend_085_seed2.sh` hardcodes `s2_*` output names and skips training when they
# exist, so passing a different SEED to it is a SILENT NO-OP: it re-matches the same nets and reports
# them as a replication. That was tried, caught on its first line of output, and killed.
# `blend_085_seed3.sh` fixed it by hardcoding `s3_*` instead — same defect, one seed later, and it is
# currently RUNNING so it cannot be edited (bash reads a script by byte offset as it executes).
# Here the tag is DERIVED from the seed, so the bug cannot recur however many seeds are added.
#
# BOTH ARMS ARE TRAINED PER SEED. A 0.75 arm from a different seed is not a control: each run faces
# its own `Net::random(width, seed)` origin, and cross-seed origin rates are not comparable. Training
# the pair is what keeps the comparison within-seed.
#
# PRE-REGISTERED, unchanged from the first two seeds:
#   * 0.85 wins again => the cross-seed mean moves further from zero and this becomes the
#     best-evidenced open candidate in the tree, justifying a real campaign — NOT a default change.
#   * 0.85 loses or ties => the mean drops back toward the boundary and the candidate returns to
#     unproven, which is the outcome the boundary exists to allow.
set -uo pipefail
cd "$(dirname "$0")"
SEED=${SEED:?usage: SEED=<n> CORE=<n> ./blend_085_seedN.sh}
CORE=${CORE:-12}
GENS=${GENS:-20}
TAG="bs${SEED}"                    # derived, never hardcoded
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt_nm/release/examples/netmatch}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "no netmatch at $NM"; exit 1; }

for B in 0.75 0.85; do
  out="${TAG}_${B/./}"
  if [ -f "$out.net" ]; then echo "=== $out.net exists, skipping ==="; continue; fi
  echo "=== seed $SEED, blend $B, $GENS generations -> $out.net ==="
  timeout 4200 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 --blend "$B" \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "$out.net" --ledger "$out.jsonl" > "$out.log" 2>&1
  echo "  $(grep -cE '^gen +[0-9]+ ' "$out.log") generations"
done

a=$(grep -cE '^gen +[0-9]+ ' "${TAG}_075.log" 2>/dev/null)
b=$(grep -cE '^gen +[0-9]+ ' "${TAG}_085.log" 2>/dev/null)
echo "  arms: ${TAG}_075 $a generations, ${TAG}_085 $b generations"
[ "$a" = "$b" ] || echo "  *** UNEQUAL ARMS -- the match below is confounded with training amount ***"

echo "=== seed $SEED: blend 0.75 vs 0.85, DIRECT, depth 4 ==="
timeout 5400 taskset -c "$CORE" nice -n 19 ionice -c 3 "$NM" \
  "${TAG}_075.net" "${TAG}_085.net" 448 4 777 \
  2>&1 | grep -E 'scores|=>|arms:|effect|ONE SEED|defensible' | sed 's/^/  /'
echo "  ${TAG}_075's score is shown; below 0.5 means blend 0.85 is stronger."
