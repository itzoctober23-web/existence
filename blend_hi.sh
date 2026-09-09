#!/usr/bin/env bash
# IS THE SHIPPED BLEND OF 0.75 THE OPTIMUM? 1.00 vs 0.75 vs 0.85 on the metric that RESOLVES.
#
# WHY THIS IS ASKED AT ALL. main.rs:307-345 records a blend sweep whose conclusion is stated
# plainly: "once the search score is in the target, the game outcome contributes nothing
# measurable. blend = 1.0 (no outcome at all) matches the best mix." The evidence was a flat
# plateau:
#     blend 0.75  0.5258 [0.5123, 0.5393]
#     blend 0.85  0.5234 [0.5071, 0.5398]
#     blend 0.95  0.5293 [0.5137, 0.5449]
#     blend 1.00  0.5281 [0.5112, 0.5450]
#
# EVERY ONE OF THOSE NUMBERS IS A CANDIDATE-VS-TRAINED-CHAMPION GATE RATE -- the metric since
# measured near-blind here: it returns 0.500 +/- 0.007 on pairs a frozen anchor separates easily,
# and non-transitivity was shown directly. The plateau is therefore a NULL ON A COMPRESSING
# INSTRUMENT, which is the single place a compressing instrument is most misleading: it cannot
# distinguish "these four arms are equal" from "this ruler cannot see the difference".
#
# THE COMPRESSION IS NOW MEASURED, NOT ASSUMED. Today's 20-generation arms scored against the
# FROZEN ORIGIN give blend 0.75 = 0.847 +/- 0.022 and blend 0.25 = 0.735 +/- 0.025, a difference of
# +0.112 +/- 0.033. The old blind metric put that same pair at 0.5258 vs 0.4898 = +0.036 +/- 0.016.
# Same direction, roughly 3x the magnitude. So the blind metric COMPRESSES but does not INVERT --
# which rehabilitates the DIRECTION of old conclusions drawn on it while destroying every NULL,
# because a null is exactly what compression manufactures. "The outcome contributes nothing" is a
# null. It is unsupported.
#
# PRE-REGISTERED READING:
#   * 1.00 BELOW 0.75 => the shipped default is right, and right for a reason the experiment could
#     not see at the time. main.rs chose 0.75 over 1.00 on a stated RISK argument (a pure bootstrap
#     off the current net has no anchor to reality and can drift) while the measurement said they
#     were equal. That risk argument would then have real support instead of being a hunch that
#     happened to be prudent.
#   * 1.00 ABOVE 0.75 => a shippable improvement on the highest-leverage knob measured so far, and
#     the anchor-to-reality worry is refuted at this horizon.
#   * INDISTINGUISHABLE => the plateau is real and survives a non-compressing instrument. That is a
#     genuine result about self-distillation, not a non-result, BUT it needs a second seed before
#     anyone believes a null on 20 generations.
#
# 0.75 IS RE-RUN AS AN IN-SCRIPT CONTROL rather than reusing bn_075.log's 0.847. Comparing a fresh
# arm against a number from an earlier run assumes run-to-run reproducibility that has never been
# checked. It is also a free determinism test: same binary, same seed, --threads 1. If the re-run
# reproduces 0.847 exactly, the ~0.07 between-run band is purely SEED variance; if it does not, the
# band is run variance and several of today's comparisons need re-reading. Either answer is worth
# the 40 minutes, and neither is available from the old log.
#
# ARM ORDER IS DELIBERATE: 1.00 and 0.75 are the essential pair, so they run FIRST and a timeout
# kill costs only the expendable 0.85 curve-shape arm.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
ARMS=${ARMS:-"1.00 0.75c 0.85"}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

# Scoped to THIS script's outputs. bn_075.log/bn_025.log/bn_000.log are the completed low-side
# sweep and deleting them would destroy the comparison this run exists to extend.
rm -f bh_*.log bh_*.jsonl bh_*.net

echo "=== blend high A/B: $ARMS, $GENS generations, seed $SEED, nothing gated ==="
echo "=== verdict = built-in final control vs the FROZEN ORIGIN, never the champion gate rate ==="
for B in $ARMS; do
  tag=$(echo "$B" | tr -d '.')          # 1.00 -> 100, 0.75c -> 075c
  val=${B%c}                            # strip the control marker for the actual flag
  echo "--- arm: blend $val ${B##*[0-9]} ---"
  timeout 4200 taskset -c "${CORE:-15}" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --blend "$val" --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "bh_${tag}.net" --ledger "bh_${tag}.jsonl" > "bh_${tag}.log" 2>&1
  echo "  generations $(grep -cE '^gen ' "bh_${tag}.log"), final loss $(grep -oE 'loss [0-9.]+' "bh_${tag}.log" | tail -1), decisive $(grep -oE 'dec +[0-9]+/[0-9]+' "bh_${tag}.log" | tail -1)"
done

echo
echo "=== VERDICT: each arm vs its FROZEN ORIGIN (448 pairs, built-in control) ==="
for B in $ARMS; do
  tag=$(echo "$B" | tr -d '.')
  printf "  blend %-6s %s\n" "$B" "$(grep -A 1 'CONTROL  final champion' "bh_${tag}.log" 2>/dev/null | head -1 | sed 's/^.*net: //')"
done
echo "  reference, completed low-side sweep (same binary/seed/settings):"
for pair in 075:0.75 025:0.25 000:0.00; do
  t=${pair%%:*}; lbl=${pair##*:}
  printf "    blend %-6s %s\n" "$lbl" "$(grep -A 1 'CONTROL  final champion' "bn_${t}.log" 2>/dev/null | head -1 | sed 's/^.*net: //')"
done
echo
echo "  FIRST read the 0.75c control against bn_075's 0.847 +/- 0.022. If they differ materially,"
echo "  the run-to-run band is real and NO cross-run comparison in this investigation is safe --"
echo "  read that before reading the 1.00 answer, because it decides whether the answer means"
echo "  anything at all."
