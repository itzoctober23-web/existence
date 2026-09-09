#!/usr/bin/env bash
# IS THE WIDENING HORIZON THE CEILING? --horizon-cap 10 (fixed narrow) vs 1000 (today's default).
#
# THIS RE-RUNS A QUESTION ALREADY ASKED ONCE, AND SAYS WHY. `horizon_ab.sh` ran earlier today and
# settled nothing, for two reasons that are now known defects rather than bad luck:
#
#   1. IT USED THE WRONG METRIC. Its verdict was MEAN GATE RATE -- candidate against its own
#      champion -- and that number has since been shown to be near-blind here: two similar nets at
#      depth 2 draw almost everything, so the gate returns 0.500 +/- 0.007 while a fixed anchor
#      resolves the same pair easily. Non-transitivity was also demonstrated directly (a net can
#      beat its parent and be weaker against a third opponent). Beating the net you were trained
#      from is not strength.
#   2. THE ARMS WERE UNBALANCED. n=12 against n=4 -- the second arm was terminated early by its
#      timeout -- and both landed BELOW 0.5 (0.4852 vs 0.4950). Nothing was resolved in either
#      direction, and the run was reported as though the comparison had happened.
#
# So it is asked again with the metric that has actually resolved things today: the built-in final
# control against the FROZEN ORIGIN, which is what refuted capacity (w64 loses 0.179 +/- 0.021 at
# equal time) and the draw filter (exclude 0.828 vs include 0.774, difference +0.054 +/- 0.034).
#
# THE MECHANISM UNDER TEST. `horizon = min(10 + (g-1)*5, cap)`, so the filter admits positions ever
# further from the end of the game -- 755 plies by generation 150. datagen.rs:17-20 warns exactly
# against this: "in self-play by a near-random engine the OUTCOME is nearly independent of a
# position 40 plies earlier -- the players are noise". main.rs:514 records the supporting
# measurement: training on ALL decided positions moved sign accuracy 0.452 -> 0.441, while <=10
# plies moved it to 0.543. If far-from-terminal labels are anti-signal, a schedule that keeps
# admitting more of them should cap strength.
#
# PRE-REGISTERED READING:
#   * CONFIRMED if the capped-at-10 arm ends measurably ABOVE the widening arm against the origin.
#     Then the schedule is the brake, and it should track measured label quality rather than
#     counting generations.
#   * REFUTED if it ends at or below. Then far-from-terminal labels are not the constraint at this
#     strength, and the LAST of the four ceiling candidates is closed -- which would mean the
#     ceiling is not in any single component tested so far and the question changes shape.
#   * UNRESOLVED is likely and is NOT a null: the run-to-run band is ~0.07, wider than a single
#     run's ci95. It means "more seeds".
#
# A LOWER ACCEPT COUNT AT CAP 10 IS EXPECTED, not a failure -- the pool is much smaller. Nothing is
# gated in either arm anyway (--gate-every 100 never fires at --gens 20), so acceptance is not the
# metric and cannot be confused for one.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

# CLEAR STALE OUTPUTS FIRST. v2 writes the same hz_<cap>.log filenames v1 used, and v1's
# hz_1000.log sat here for five hours after that run was abandoned -- long enough that the verdict
# block would have compared a FRESH cap-10 arm against a STALE cap-1000 arm from a different
# experiment with a different metric, and printed it as a result. Removing them makes a missing arm
# read as MISSING rather than as an old number.
#
# APPLIED AFTER the run finished, not during it. Editing this file mid-run is what killed the
# verdict block with a syntax error at line 69 -- bash reads by byte offset, and reverting within a
# minute did NOT undo it.
rm -f hz_10.log hz_1000.log

echo "=== horizon A/B v2: cap 10 (fixed) vs cap 1000 (widening), $GENS generations, nothing gated ==="
echo "=== verdict = built-in final control vs the FROZEN ORIGIN, not the gate rate ==="
for CAP in 10 1000; do
  echo "--- arm: horizon-cap $CAP ---"
  timeout 4200 taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --horizon-cap "$CAP" --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "hz_${CAP}.net" --ledger "hz_${CAP}.jsonl" > "hz_${CAP}.log" 2>&1
  echo "  generations $(grep -cE '^gen ' "hz_${CAP}.log"), final pool $(grep -oE 'pool +[0-9]+' "hz_${CAP}.log" | tail -1)"
done

echo
echo "=== VERDICT: each arm vs its FROZEN ORIGIN (448 pairs, built-in control) ==="
for CAP in 10 1000; do
  printf "  cap %-5s %s\n" "$CAP" "$(grep -A 1 'CONTROL  final champion' "hz_${CAP}.log" 2>/dev/null | head -1 | sed 's/^.*net: //')"
done
echo
echo "  cap10 ABOVE cap1000 => the widening schedule is the brake; make it track label quality."
echo "  cap10 AT OR BELOW   => far-from-terminal labels are not the constraint. That closes the"
echo "                         LAST of the four candidates and the ceiling question changes shape."
