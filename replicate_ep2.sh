#!/usr/bin/env bash
# REPLICATE the one suggestive positive: epochs 2 vs epochs 3, DIFFERENT SEED.
#
# The five-budget sweep put epochs 2 at mean gate rate 0.5087, interval [0.5016, 0.5157] --
# excluding 0.5, i.e. candidates measurably beating their champion, the only configuration all day
# to do so. It is NOT a result yet and this is why:
#   * one of FIVE budgets tested, and at 95% intervals roughly one in four such sweeps throws a
#     crossing by chance alone;
#   * the margin is 0.0016;
#   * generations inside a run are a PATH, not independent samples -- the champion moves on each
#     accept;
#   * neighbours do not corroborate: epochs 1 at 0.5036 and epochs 3 at 0.5011 both straddle 0.5.
#
# DIFFERENT SEED is the point. The original ran at 20260907; this runs at 20260908, so the games,
# the self-play data and the mutation stream all differ. Same seed would re-measure the same path.
#
# PRE-REGISTERED, before the numbers exist:
#   * CONFIRMED if epochs 2's interval again excludes 0.5 AND it beats the epochs-3 control arm.
#     Two independent seeds crossing is no longer a 1-in-4 story and it becomes worth acting on.
#   * REFUTED if the interval straddles 0.5. Then the original was the multiple-comparison fluke
#     it looks like, and it gets struck from the record rather than left to be quoted later.
#   * epochs 3 is the CONTROL and is expected to straddle 0.5 (it measured 0.5011 before). If the
#     control instead comes out clearly above 0.5 on this seed, the seed itself is favourable and
#     NEITHER arm means anything -- that is the outcome that would invalidate the comparison, and
#     it is the reason a control arm is here at all.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-900}
SEED=20260908
INIT=${INIT:-champion_long.net}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
echo "=== epochs 2 replication at seed $SEED (original used 20260907) ==="
for E in 2 3; do
  echo "--- arm: epochs $E ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --init "$INIT" --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs "$E" \
    --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "rep_${E}.net" --ledger "rep_${E}.jsonl" > "rep_${E}.log" 2>&1
  echo "  generation $(grep -cE '^gen ' "rep_${E}.log"), $(grep -cE 'ACCEPT' "rep_${E}.log") accepted"
done

echo
echo "=== MEAN GATE RATE (games; 0.5 = equal to the champion) ==="
python3 - <<'PY'
import json, statistics, os
for e in ('2','3'):
    f=f'rep_{e}.jsonl'
    if not os.path.exists(f): print(f'  epochs {e} no ledger'); continue
    rates=[]
    for l in open(f):
        if not l.strip(): continue
        r=json.loads(l)
        if r.get('class')!='NET': continue
        g=(r.get('gates') or [{}])[0]
        if g.get('rate') is not None: rates.append(g['rate'])
    if not rates: print(f'  epochs {e} no gated generations'); continue
    m=statistics.mean(rates); se=statistics.pstdev(rates)/len(rates)**0.5
    lo,hi=m-1.96*se, m+1.96*se
    mark='EXCLUDES 0.5' if lo>0.5 else ('below 0.5' if hi<0.5 else 'straddles 0.5')
    print(f'  epochs {e}  n {len(rates):>3}  mean {m:.4f}  [{lo:.4f}, {hi:.4f}]  {mark}')
print()
print('  Seed 20260907 gave: epochs 2 -> 0.5087 [0.5016, 0.5157]; epochs 3 -> 0.5011 [0.4952, 0.5070].')
print('  CONFIRMED only if epochs 2 again excludes 0.5 AND beats the epochs-3 control.')
print('  If the CONTROL also clears 0.5, this seed is favourable and neither arm means anything.')
PY
echo "=== done ==="
