#!/usr/bin/env bash
# CAN THE REDESIGNED LOOP ACTUALLY RATCHET? --gate-every 5 with the anchor-increment gate.
#
# This is the payoff test for the whole day's diagnosis. Everything else measured what was BROKEN;
# this asks whether the replacement works.
#
# WHAT THE DIAGNOSIS ESTABLISHED, all measured rather than argued:
#   * All three cheap per-generation signals are uninformative. The held-out surrogate scores
#     r = -0.095 against 239 gate results; training loss scores r = +0.379, CI [-0.249, +0.783],
#     n = 12 -- indistinguishable from zero and the WRONG SIGN; and the candidate-vs-champion gate
#     returns 0.500 +/- 0.007 on nets a fixed anchor separates easily, with non-transitivity shown
#     directly.
#   * Only games against a FIXED opponent have resolved anything. They refuted capacity
#     (w64 loses 0.179 +/- 0.021 at equal time) and the draw filter (+0.054 +/- 0.034).
#   * Those games are unaffordable per generation: pair sd 0.2362, so 224 pairs resolve +/-0.031
#     while the best per-generation edge ever measured here is +0.0114 -- 2.7x too coarse.
#
# THE REPLACEMENT, and why it should work if anything does. Train K generations unconditionally,
# then measure BOTH the accumulated champion and the batch base against the FROZEN ORIGIN and
# compare the INCREMENTS with a two-sample test. Signal grows by K while the floor stays put, and
# the comparison is against a fixed anchor rather than a moving parent. That is the same principle
# the 4PC ICC protocol reached independently: judge a change by its increment over a fixed
# baseline.
#
# PRE-REGISTERED READING, and the null is the likely one:
#   * RATCHETS if the control-vs-origin trace CLIMBS across the run and ends measurably above where
#     it started. Then the feedback path was the defect, the redesign fixes it, and the ceiling
#     work resumes on a loop that can actually hold a gain.
#   * DOES NOT RATCHET if the trace is flat within noise, or every batch rolls back. Then the
#     feedback path was NOT the whole story -- the per-generation change may simply be too small to
#     accumulate, or absent. That would be the honest end of this line of attack and it should be
#     written up as one, not retried with a bigger K until something passes.
#   * ROLLS BACK EVERY TIME while the control climbs would mean the gate is too strict and is
#     discarding real gains; that is a different fix (lower the bar) and must not be confused with
#     the first case.
#
# The trace is the evidence, not the endpoint: --control-every 5 records champion-vs-origin at every
# batch boundary, so the SHAPE is visible and a single lucky endpoint cannot be mistaken for a
# trend.
#
# Pinned to the xt3 binary, which is the one carrying the anchor-increment gate. The queued A/B
# campaign runs on xt2 and must keep the binary its earlier arms used.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-40}
K=${K:-5}
SEED=${SEED:-20260907}
INIT=${INIT:-champion_long.net}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt3/release/learn}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }

echo "=== ratchet test: --gate-every $K, anchor-increment gate, $GENS generations ==="
echo "=== starting from champion_long, re-measured today at 0.861 +/- 0.010 vs the origin ==="
timeout 7200 taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
  --init "$INIT" --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
  --gate-every "$K" --gate-pairs 224 --arch-every 0 --control-every "$K" \
  --seed "$SEED" --out "rt_k${K}.net" --ledger "rt_k${K}.jsonl" > "rt_k${K}.log" 2>&1

echo
echo "=== THE TRACE: champion vs the frozen origin at every batch boundary ==="
grep -E "control vs origin" "rt_k${K}.log" | sed 's/^ */  /'
echo
echo "=== BATCH GATE DECISIONS (increment over the fixed anchor) ==="
grep -E "batch gate" "rt_k${K}.log" | sed 's/^ */  /'
echo
echo "  Trace CLIMBING and ending above its start => the loop ratchets; the feedback path was it."
echo "  Trace FLAT within noise                   => the redesign is not sufficient. Write it up."
echo "  Every batch ROLLING BACK while the trace climbs => gate too strict, a different fix."
