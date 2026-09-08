#!/usr/bin/env bash
# DOES THE ANCHOR GATE HELP? --anchor-pairs 0 (today's behaviour) vs 224.
#
# THE DEFECT IT TARGETS, measured rather than argued. At depth 2 -- the depth every run gates at:
#     ep_1 vs champion_long        0.545 +/- 0.018   ep_1 WINS, resolved
#     champion_long vs origin      0.864 +/- 0.013
#     ep_1          vs origin      0.834 +/- 0.013   0.030 gap, resolved
# ep_1 beats its champion and is beaten by that champion through a third opponent. The accept rule
# is "beats the current champion", and that quantity moves opposite to strength against a fixed
# opponent. Across eight runs the mean gate rate was >= 0.5 in ALL EIGHT while not one final net
# rose above the 0.864 start and three fell below it.
#
# The anchor gate adds a second match against the frozen origin and VETOES a promotion that is
# resolved worse against it. It can only veto, never promote.
#
# THE MEASUREMENT IS THE ORIGIN SCORE, not the gate rate and not the accept count. Both arms end
# scored against the same frozen origin at 600 pairs, and the question is whether the anchored arm
# lands ABOVE the 0.864 they start from -- or at least stops falling below it.
#
# PRE-REGISTERED, before any number exists:
#   * CONFIRMED if the anchored arm's origin score is >= the unanchored arm's AND >= 0.864. That
#     would be the first configuration all day to hold its starting point on purpose.
#   * PARTIAL if the anchored arm merely stops the DECLINE (both near 0.864, unanchored lower).
#     Worth having -- three of eight runs today lost ground -- but it is damage control, not
#     learning, and must be reported as such.
#   * REFUTED if the anchored arm is no better, or accepts nothing at all. Vetoing every promotion
#     is a way to score 0.864 by doing nothing, which is NOT a win; the accept count is reported
#     alongside precisely so that outcome cannot masquerade as success.
#   * The 0.151 run-to-run variance across full restarts does not apply here (both arms resume
#     from the same champion), but two arms is still two samples -- a gap under ~0.018 is not
#     resolved at 600 pairs.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-900}
SEED=20260909
INIT=${INIT:-champion_long.net}
PAIRS_SCORE=${PAIRS_SCORE:-600}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
echo "=== anchor gate A/B: ${SECS}s per arm, gate-pairs 224, epochs 3 ==="
for A in 0 224; do
  echo "--- arm: anchor-pairs $A ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --init "$INIT" --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-pairs 224 --anchor-pairs "$A" --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "an_${A}.net" --ledger "an_${A}.jsonl" > "an_${A}.log" 2>&1
  echo "  generation $(grep -cE '^gen ' "an_${A}.log"), $(grep -cE 'ACCEPT' "an_${A}.log") accepted"
done

echo
echo "=== reasons per arm (anchor_regression is the intransitive veto firing) ==="
python3 - <<'PY'
import json, os
from collections import Counter
for a in ('0','224'):
    f=f'an_{a}.jsonl'
    if not os.path.exists(f): print(f'  anchor {a:<4} no ledger'); continue
    c=Counter()
    for l in open(f):
        if not l.strip(): continue
        r=json.loads(l)
        if r.get('class')=='NET': c[r.get('reason')]+=1
    print(f"  anchor {a:<4} " + '  '.join(f'{k}={v}' for k,v in c.most_common()))
PY

echo
echo "=== scored against the frozen origin — THIS is the measurement ==="
for A in 0 224; do
  [ -f "an_${A}.net" ] || { echo "  anchor $A: NO CHAMPION (never accepted one)"; continue; }
  printf "  anchor-pairs %-4s " "$A"
  taskset -c 15 nice -n 19 ionice -c 3 ./target/release/examples/control \
    --champion "an_${A}.net" --pairs "$PAIRS_SCORE" --seed 987654 2>/dev/null \
    | grep "fixed depth 2" | sed 's/^ *//'
done
echo "  (both arms started from a net measuring 0.864 +/- 0.013 on this same match)"
echo "  Read with the accept counts above: 0.864 reached by accepting NOTHING is not a win."
echo "=== done ==="
