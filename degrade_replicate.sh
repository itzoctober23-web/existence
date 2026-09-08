#!/usr/bin/env bash
# DOES ONE GENERATION OF TRAINING MAKE THE NET WORSE? Three seeds, measured against the FROZEN
# ORIGIN, which is the only opponent that has been able to resolve anything all day.
#
# THE READING THAT PROMPTED THIS. In the batch-gate arm, the champion after ONE generation of
# unconditional training scored 0.831 +/- 0.015 against the origin, where champion_long itself
# measures 0.861 +/- 0.010 (1448W-546D-6L, re-measured today rather than inherited). Difference
# 0.030 +/- 0.018 -- resolved, excludes zero. The loss counts are the louder signal: champion_long
# loses SIX games in 2000 to the origin; the trained net loses 265.
#
# Meanwhile the batch gate comparing those same two nets HEAD TO HEAD returned 0.500 +/- 0.007 -- a
# precisely measured dead heat. Two similar nets at depth 2 draw nearly everything, so the interval
# is tiny and the gate is confidently blind. A fixed, dissimilar anchor resolves the same
# difference easily.
#
# IF THIS REPLICATES it inverts the standing story. The loop would not be "failing to improve" --
# its training step would be actively DEGRADING the net, and the gate rejecting everything (13
# generations, 0 accepts) would have been PROTECTING it rather than being too strict. It also
# explains the one case where the gate was loosened: the surrogate-fallback runs accepted 7-11
# candidates and drove the champion 0.864 -> 0.826.
#
# PRE-REGISTERED, before the numbers exist:
#   * CONFIRMED if all three seeds land near 0.83 and below champion_long's 0.861 - 0.010. Then the
#     training step is the defect and every gate/blend/epochs experiment run today was tuning the
#     filter on a poisoned source.
#   * REFUTED if the seeds straddle 0.861. Then the single 0.831 was an unlucky draw -- entirely
#     possible off ONE reading, which is why this exists -- and the plateau needs another cause.
#   * A SPLIT (one low, two fine) means the degradation is data-dependent, not systematic, and the
#     next question is what those generations sampled.
#
# CONFOUND, STATED: the in-loop control and this script may not use identical opening seeds, so the
# 0.831 and 0.861 are independent samples rather than a paired comparison. The CI arithmetic above
# treats them as such. Each arm here uses the SAME control harness, so the three replicates ARE
# mutually comparable.
set -uo pipefail
cd "$(dirname "$0")"
INIT=${INIT:-champion_long.net}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xtarget/release/learn}
CTRL=${CTRL:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/examples/control}
PAIRS=${PAIRS:-1000}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$CTRL" ] || { echo "no control at $CTRL"; exit 1; }

echo "=== one generation of training, 3 seeds, scored vs the FROZEN ORIGIN ==="
echo "=== baseline: champion_long = 0.861 +/- 0.010 (re-measured, 1448W-546D-6L) ==="
for S in 20260907 20260908 20260909; do
  echo "--- seed $S ---"
  # --gate-every 5 with --gens 1 means generation 1 is accepted UNCONDITIONALLY and no batch gate
  # fires, which is exactly the intervention under test: train once, gate nothing, then measure.
  timeout 600 taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
    --init "$INIT" --gens 1 --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-every 5 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$S" --out "dg_${S}.net" --ledger "dg_${S}.jsonl" > "dg_${S}.log" 2>&1
  if [ -f "dg_${S}.net" ]; then
    timeout 900 taskset -c 15 nice -n 19 ionice -c 3 "$CTRL" \
      --champion "dg_${S}.net" --pairs "$PAIRS" > "dg_${S}_ctrl.log" 2>&1
    echo "  $(grep -E 'rate' "dg_${S}_ctrl.log" | tail -1)"
  else
    echo "  no net produced -- check dg_${S}.log"
  fi
done

echo
echo "=== VERDICT ==="
echo "  champion_long (untrained baseline):  0.861 +/- 0.010"
for S in 20260907 20260908 20260909; do
  printf "  seed %-9s %s\n" "$S" "$(grep -E 'rate' "dg_${S}_ctrl.log" 2>/dev/null | tail -1)"
done
echo
echo "  All three near 0.83 => the TRAINING STEP degrades the net, and every gate experiment today"
echo "  was tuning a filter on a poisoned source. Straddling 0.861 => the single 0.831 was noise."
