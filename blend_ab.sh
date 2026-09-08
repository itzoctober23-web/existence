#!/usr/bin/env bash
# IS THE TRAINING TARGET TOO SELF-REFERENTIAL? blend 0.75 (shipped) vs 0.25 vs 0.00.
#
# `Trainer` builds its target as  target = (1 - blend) * z + blend * root  where `z` is the game
# result and `root` is the ROOT SEARCH SCORE recorded during datagen -- produced by the CHAMPION.
# At the shipped blend of 0.75, three-quarters of what the candidate is trained to reproduce is
# the champion's own evaluation. main.rs:272 says this outright: "Mixing the net's OWN root score
# into its target is self-referential and teaches nothing. The blend only earns its place once the
# search score is better than [the net]."
#
# WHY IT IS WORTH RE-ASKING. The blend was gated (hyper_ab.rs --blends: 0.4805 / 0.4898 / 0.5188 /
# 0.5258 at blends 0.00 / 0.25 / 0.50 / 0.75) and 0.75 won, so this is NOT a claim that it is
# wrong. But that A/B scored nets against the FROZEN ORIGIN -- "is this net good" -- and the loop's
# actual problem is a different question: "is the CANDIDATE better than the CHAMPION it came from".
# Measured over 239 generations today, the mean gate rate is 0.5024, i.e. candidates are
# indistinguishable from their parent. A target that is three-quarters "reproduce the parent" is a
# mechanism that would produce exactly that, and it has never been measured against that question.
#
# THE METRIC IS THE GATE, NOT THE SURROGATE. mcnemar_z has been measured against 239 paired gate
# results at corr -0.095, 95% CI [-0.220, +0.032] -- no usable strength signal -- so this reads
# MEAN GATE RATE, which is games: 224 pairs per generation against the champion, 0.5 = identical.
#
# PRE-REGISTERED READING, before the numbers exist:
#   * CONFIRMED if mean gate rate rises as blend falls. Then the target being self-referential is
#     a real brake and the blend schedule should track measured champion quality, not sit fixed.
#   * REFUTED if 0.75 holds its own or wins. Then the bootstrap value of the root score outweighs
#     its self-reference even while the champion is weak, and the 0.5024 has another cause. Say so.
#   * All three near 0.5024 means blend is NOT the brake, which is also worth knowing and closes a
#     lever rather than opening one.
# A rate BELOW 0.5 is not automatically bad here: it means the candidate lost to its parent, which
# at blend 0 could simply mean the raw game result is too noisy a target at this strength.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-900}
SEED=20260907
INIT=${INIT:-champion_long.net}
ARMS=${ARMS:-"0.75 0.25 0.00"}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
echo "=== blend A/B: ${SECS}s per arm, gate-pairs 224, surrogate override OFF ==="
for B in $ARMS; do
  tag=${B/./}
  echo "--- arm: blend $B ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --init "$INIT" --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs 3 \
    --blend "$B" --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "bl_${tag}.net" --ledger "bl_${tag}.jsonl" > "bl_${tag}.log" 2>&1
  echo "  generation $(grep -cE '^gen ' "bl_${tag}.log"), $(grep -cE 'ACCEPT' "bl_${tag}.log") accepted"
done

echo
echo "=== MEAN GATE RATE per arm (games, 224 pairs/generation; 0.5 = equal to champion) ==="
python3 - "$ARMS" <<'PY'
import json, statistics, sys, os
for b in sys.argv[1].split():
    f = f"bl_{b.replace('.','')}.jsonl"
    if not os.path.exists(f):
        print(f"  blend {b:<5} no ledger"); continue
    rates=[]
    for l in open(f):
        if not l.strip(): continue
        r=json.loads(l)
        if r.get('class')!='NET': continue
        g=(r.get('gates') or [{}])[0]
        if g.get('rate') is not None: rates.append(g['rate'])
    if not rates:
        print(f"  blend {b:<5} no gated generations"); continue
    m=statistics.mean(rates)
    se=statistics.pstdev(rates)/max(1,len(rates))**0.5
    print(f"  blend {b:<5} n {len(rates):>3}  mean gate rate {m:.4f} +/- {1.96*se:.4f}")
print()
print("  Reference from today, epochs 3 at the shipped blend 0.75: mean gate rate 0.5011 (n=26).")
print("  Pooled over 239 generations from seven runs: 0.5024.")
print("  CONFIRMED if the rate rises as blend falls; REFUTED if 0.75 holds or wins.")
PY
echo "=== done ==="
