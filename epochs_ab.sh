#!/usr/bin/env bash
# IS THE TRAINING STEP UNDER-FITTING? epochs 3 (current) vs 10 vs 30, everything else identical.
#
# WHY THIS IS THE NEXT EXPERIMENT. Today's gate A/B settled that the loop's SELECTION step was
# broken and fixed it (40 pairs -> 224; the blunt gate scored 0.826, the sharp one 0.859, from a
# 0.864 start). But the sharp arm accepted ONE candidate in 35 generations and did not improve on
# its starting point, so selection is no longer the binding constraint. The ledgers say why:
#
#     mcnemar_z, the loop's own paired candidate-vs-champion surrogate on held-out positions
#         gate-pairs 40   median +0.102, positive in 22/41 generations
#         gate-pairs 224  median -0.128, positive in 17/35 generations
#
# A median of ~0 and a ~50% sign rate is a COIN FLIP. The training step is not producing better
# candidates -- not "small gains the gate cannot resolve", but indistinguishable from the parent
# on the loop's own metric, which needs no games and so cannot be blamed on gate resolution.
#
# THE HYPOTHESIS. `train_from` warm-starts from the CHAMPION and runs max_epochs=3 with a patience
# of 3, so the patience can never fire and every candidate is three epochs away from a net that is
# already trained. Three epochs from a warm start may simply not move the net far enough to be
# better OR worse -- which is exactly what a coin-flip z looks like. Training loss also ROSE
# across generations in both arms (first-5 mean 0.0352 -> last-5 0.0444, +26%), which is not what
# a converging optimiser does.
#
# PRE-REGISTERED READING, written before any of these numbers exist:
#   * If the mcnemar_z median moves clearly positive as epochs rise, the training step was
#     UNDER-FITTING and epochs is a real lever. That is the actionable outcome.
#   * If the median stays at ~0 at 30 epochs, the optimisation is NOT the constraint and the
#     problem is upstream -- the labels (self-play at depth 2) or the surrogate itself. Report
#     that as a refutation of the under-fitting hypothesis, not as "inconclusive".
#   * A HIGHER median with a LOWER origin score would mean the candidates are fitting the labels
#     better while playing worse, which indicts the LABELS specifically. That outcome is the most
#     informative of the three and must not be reported as a failure.
#
# The origin score is secondary here: 3 arms at ~15 minutes cannot resolve a small strength
# difference (the loop carries 0.151 run-to-run variance across restarts). The z-distribution is
# the measurement; the score is a sanity check that a bigger z is not bought with a worse net.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-900}
SEED=20260907
INIT=${INIT:-champion_long.net}
ARMS=${ARMS:-"3 10 30"}
PAIRS_SCORE=${PAIRS_SCORE:-600}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
echo "=== epochs A/B: ${SECS}s per arm, all resuming from $INIT, gate-pairs 224 ==="
for E in $ARMS; do
  echo "--- arm: epochs $E ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --init "$INIT" --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs "$E" \
    --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "ep_${E}.net" --ledger "ep_${E}.jsonl" > "ep_${E}.log" 2>&1
  gens=$(grep -cE '^gen ' "ep_${E}.log")
  acc=$(grep -cE 'ACCEPT' "ep_${E}.log")
  echo "  generation $gens, $acc accepted"
done

echo
echo "=== the measurement: paired surrogate distribution per arm ==="
python3 - "$ARMS" <<'PY'
import json, statistics, sys
for e in sys.argv[1].split():
    try: rows=[json.loads(l) for l in open(f'ep_{e}.jsonl') if l.strip()]
    except OSError: print(f'  epochs {e:<3} no ledger'); continue
    net=[r for r in rows if r.get('class')=='NET' and r.get('surrogate')]
    z=[r['surrogate'].get('mcnemar_z') for r in net]
    z=[x for x in z if x is not None]
    tl=[r['surrogate'].get('train_loss') for r in net if r['surrogate'].get('train_loss') is not None]
    if not z: print(f'  epochs {e:<3} no NET rows'); continue
    pos=sum(1 for x in z if x>0)
    print(f'  epochs {e:<3} n {len(z):>3}  mcnemar_z median {statistics.median(z):+.3f}  '
          f'mean {statistics.mean(z):+.3f}  >0 {pos}/{len(z)}  train_loss median {statistics.median(tl):.6f}')
print()
print('  Baseline to beat, from today\'s gate A/B at epochs 3:')
print('    gate-pairs 224  median -0.128, positive 17/35')
print('    gate-pairs 40   median +0.102, positive 22/41')
PY

echo
echo "=== scored against the frozen origin (sanity check, NOT the measurement) ==="
for E in $ARMS; do
  [ -f "ep_${E}.net" ] || { echo "  epochs $E: NO CHAMPION (never accepted one)"; continue; }
  printf "  epochs %-3s " "$E"
  taskset -c 15 nice -n 19 ionice -c 3 ./target/release/examples/control \
    --champion "ep_${E}.net" --pairs "$PAIRS_SCORE" --seed 987654 2>/dev/null \
    | grep "fixed depth 2" | sed 's/^ *//'
done
echo "  (the net all three started from measures 0.864 +/- 0.013 on this same match)"
echo "=== done ==="
