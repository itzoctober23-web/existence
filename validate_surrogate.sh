#!/usr/bin/env bash
# DID EXCLUDING DRAWS FIX THE SURROGATE? Re-run the epochs contrast that exposed it.
#
# THE DEFECT. `z` is the game result from White's POV: +1, -1, or 0 for a DRAW. paired_sign tested
# `(white > 0.0) == (z > 0.0)`, and for a draw that demands an evaluation of <= 0 -- "Black is
# better". There is no draw class. Measured over 276,000 self-play games today: 50.3% DRAWN.
#
# WHY IT INVERTS WITH TRAINING. A better net says "about equal" on a drawn position, i.e. near
# zero, and the sign of a near-zero number is noise. So the more the net improves, the more of the
# held-out set lands in the noisy half and the WORSE this statistic reads. Pre-fix, measured:
#
#     epochs   heldout_loss   mcnemar_z median   candidate better
#        3       0.041705         +0.359            14/26  (54%)
#       10       0.031720         +0.141            14/27  (52%)
#       30       0.025284         -0.445             9/27  (33%)
#
# Loss falls monotonically on the SAME held-out data while the paired statistic collapses. Two
# metrics on identical data moving in opposite directions is not overfitting -- one is broken.
#
# PRE-REGISTERED READING, written before this runs:
#   * CONFIRMED if the pathological ORDERING disappears: epochs 30 should no longer sit far below
#     epochs 3. The mechanism being removed is specifically the one that penalised a better net.
#   * REFUTED if epochs 30 is still dramatically below epochs 3 (a gap near the pre-fix 0.80).
#     Then draws were not the explanation and I report that, having already committed the claim.
#   * A z that is near zero for BOTH arms is NOT a failure of this fix. It would mean the
#     candidates are genuinely no better than their parent -- which is the honest state of the loop
#     and is what the gate A/B already indicated (1 accept in 35 generations at a resolving gate).
#     The fix is about the metric being MEANINGFUL, not about it being positive.
#
# Sample size roughly halves, because draws are dropped. That is the honest cost and it makes each
# arm's z noisier; compare medians and sign rates, not single generations.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-900}
SEED=20260907
INIT=${INIT:-champion_long.net}
ARMS=${ARMS:-"3 30"}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
echo "=== surrogate validation: draws EXCLUDED, ${SECS}s per arm, gate-pairs 224 ==="
for E in $ARMS; do
  echo "--- arm: epochs $E ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --init "$INIT" --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs "$E" \
    --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "sv_${E}.net" --ledger "sv_${E}.jsonl" > "sv_${E}.log" 2>&1
  echo "  generation $(grep -cE '^gen ' "sv_${E}.log"), $(grep -cE 'ACCEPT' "sv_${E}.log") accepted"
done

echo
echo "=== paired surrogate, draws excluded (pre-fix numbers alongside) ==="
python3 - "$ARMS" <<'PY'
import json, statistics, sys, os
pre = {"3": ("+0.359", "14/26", "0.041705"), "10": ("+0.141", "14/27", "0.031720"),
       "30": ("-0.445", "9/27", "0.025284")}
for e in sys.argv[1].split():
    f = f'sv_{e}.jsonl'
    if not os.path.exists(f):
        print(f'  epochs {e:<3} no ledger'); continue
    rows = [json.loads(l) for l in open(f) if l.strip()]
    net = [r for r in rows if r.get('class') == 'NET' and r.get('surrogate')]
    z = [r['surrogate'].get('mcnemar_z') for r in net]
    z = [x for x in z if x is not None]
    hl = [r['surrogate'].get('heldout_loss') for r in net if r['surrogate'].get('heldout_loss') is not None]
    if not z:
        print(f'  epochs {e:<3} no NET rows'); continue
    pos = sum(1 for x in z if x > 0)
    p = pre.get(e, ("?", "?", "?"))
    print(f'  epochs {e:<3} n {len(z):>3}  z median {statistics.median(z):+.3f}  >0 {pos}/{len(z)}  '
          f'heldout_loss med {statistics.median(hl):.6f}')
    print(f'            pre-fix: z median {p[0]}  >0 {p[1]}  loss {p[2]}')
print()
print('  Pre-fix the 3->30 gap was 0.804 and in the WRONG direction (more training, worse z).')
print('  CONFIRMED if that ordering is gone. REFUTED if the gap is still near 0.80.')
print('  Both arms near zero is NOT a failure of the fix -- it means the candidates really are')
print('  no better than their parent, which is the honest state the gate A/B already showed.')
PY
echo "=== done ==="
