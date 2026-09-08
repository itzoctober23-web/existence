#!/usr/bin/env bash
# IS THE WIDENING HORIZON THE BRAKE? --horizon-cap 10 (fixed) vs 1000 (today's default).
#
# THE DISCREPANCY THIS EXPLAINS. Two measurements of the SAME blend disagree:
#     hyper_ab.rs --blends, blend 0.75, horizon 10, 10 replicates:  0.5258 [0.5123, 0.5393]
#                                                                   EXCLUDES 0.5 -- candidates BEAT
#                                                                   their champion
#     this loop, blend 0.75, widening horizon, 28 generations:      0.5008 +/- 0.0058
#                                                                   dead equal
# Same target, same trainer, opposite conclusions. One documented difference: hyper_ab fixed the
# horizon at 10 plies, while the loop computes `horizon = min(10 + (g-1)*5, cap)` -- about 145 by
# generation 28.
#
# And main.rs:465 already measured what that costs: "training on ALL decided positions makes the
# eval WORSE (sign acc 0.452 -> 0.441) while <=10 plies makes it BETTER (-> 0.543) on 5x less
# data. Far-from-terminal labels are anti-signal while both players are near-random." The schedule
# widens on the assumption that play improves; measured across eight runs, it does not.
#
# I TESTED A VERSION OF THIS BEFORE AND IT CAME OUT NULL -- correlation of generation against gate
# rate within a run was +0.109, six of seven runs positive, which argues the widening does NOT hurt.
# That test is weak: it conflates the horizon with everything else that changes over a run, and the
# champion moves on accepts. This is the direct version -- hold the horizon fixed and compare.
#
# PRE-REGISTERED:
#   * CONFIRMED if horizon-cap 10 gives a mean gate rate clearly above the uncapped arm, and
#     ideally above 0.5. That would reconcile the two measurements and make the schedule the brake.
#   * REFUTED if the arms are indistinguishable. Then the horizon is not the difference and the
#     hyper_ab/loop gap has another cause -- most likely that a single controlled training step is
#     simply not the same thing as a step inside a moving lineage, which would be worth knowing.
#   * A cap of 10 means the pool is much SMALLER (5x less data per the note above), so a lower
#     accept count is expected and is not by itself a failure. Read the gate rate, not the accepts.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-900}
SEED=20260910
INIT=${INIT:-champion_long.net}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
echo "=== horizon A/B: ${SECS}s per arm, blend 0.75, gate-pairs 224, epochs 3 ==="
for H in 10 1000; do
  echo "--- arm: horizon-cap $H ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --init "$INIT" --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs 3 \
    --horizon-cap "$H" --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "hz_${H}.net" --ledger "hz_${H}.jsonl" > "hz_${H}.log" 2>&1
  echo "  generation $(grep -cE '^gen ' "hz_${H}.log"), $(grep -cE 'ACCEPT' "hz_${H}.log") accepted"
done

echo
echo "=== MEAN GATE RATE per arm (games; 0.5 = equal to the champion) ==="
python3 - <<'PY'
import json, statistics, os
for h in ('10','1000'):
    f=f'hz_{h}.jsonl'
    if not os.path.exists(f): print(f'  horizon-cap {h:<5} no ledger'); continue
    rates=[]; pool=[]
    for l in open(f):
        if not l.strip(): continue
        r=json.loads(l)
        if r.get('class')!='NET': continue
        g=(r.get('gates') or [{}])[0]
        if g.get('rate') is not None: rates.append(g['rate'])
        s=r.get('surrogate') or {}
        if isinstance(s, dict) and s.get('train_n') is not None: pool.append(s['train_n'])
    if not rates: print(f'  horizon-cap {h:<5} no gated generations'); continue
    m=statistics.mean(rates); se=statistics.pstdev(rates)/len(rates)**0.5
    lo,hi=m-1.96*se,m+1.96*se
    mark='EXCLUDES 0.5' if lo>0.5 else ('below 0.5' if hi<0.5 else 'straddles 0.5')
    extra=f'  median train_n {statistics.median(pool):.0f}' if pool else ''
    print(f'  horizon-cap {h:<5} n {len(rates):>3}  mean {m:.4f}  [{lo:.4f}, {hi:.4f}]  {mark}{extra}')
print()
print('  hyper_ab at horizon 10 measured 0.5258 [0.5123, 0.5393]; this loop uncapped gave 0.5008.')
print('  CONFIRMED if capped clearly beats uncapped. A lower accept count at cap 10 is EXPECTED')
print('  (much smaller pool) and is not itself a failure -- read the gate rate.')
PY
echo "=== done ==="
