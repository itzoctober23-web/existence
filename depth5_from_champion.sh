#!/usr/bin/env bash
# DOES DEPTH 5 STILL PAY? The one lever from today's chain that never got a fair test.
#
# WHY IT MATTERS NOW. Datagen depth 1 -> 3 was +128 Elo and produced two promotions, but the margins
# are shrinking: 0.622 +/- 0.022 against the original champion, then 0.586 +/- 0.020 against that.
# Both passed; the second is smaller. That is what deceleration looks like, and the obvious question
# is whether the same lever has more left in it.
#
# THE EARLIER ATTEMPT WAS NOT A TEST. The depth-5 arm reached 34 generations before the cores were
# consolidated, and its reading (-374 +/- 74) is a barely-trained net, not a verdict on depth 5.
#
# MATCHED START, which is what makes it a comparison: both arms resume from the SAME shipped champion
# via --init, so the only difference is the search that produces the label. Judged by netmatch
# head-to-head against that shared starting point -- the paired instrument, never the ruler, which
# carries +/-50 Elo and produced a four-reading false decline tonight.
# PRE-REGISTERED PREDICTION, written before the run. Measured node costs per move from this engine:
#   depth 1      40 nodes
#   depth 3   2,352 nodes    -- 59x depth 1
#   depth 5  81,421 nodes    -- 35x depth 3
#
# Depth 1 -> 3 cost 59x and WON decisively, because depth-1 labels are nearly information-free: a
# one-ply root score barely depends on the position, so almost any increase in label quality pays.
# Depth 3 labels are already informative, so the same dramatic win is NOT expected -- depth 5 must
# beat 35x FEWER training steps on labels that are only somewhat better.
#
# So the honest prior is that this LOSES or is unresolved at equal wall clock, and the informative
# outcome is the size of the gap rather than the sign. If depth 5 wins anyway, label quality matters
# far more than training volume even in the informative regime, which would be a bigger result than
# the original depth finding.
#
# Recording this now because the temptation after a +128 result is to expect the next step of the
# same lever to pay, and that expectation should be on the record before the number arrives.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_cap/release/learn
NM=$SCR/xt_cap/release/examples/netmatch
SECS=${SECS:-2400}
CORES5=${CORES5:-12-15}
[ -x "$LEARN" ] || { echo "no learn"; exit 1; }

cp -f p1_champion.net d5_start.net
timeout "$SECS" taskset -c "$CORES5" nice -n 19 ionice -c 3 "$LEARN" \
  --init d5_start.net --rung 0 --gens 1000000 --games 8 --threads 4 --depth 5 --epochs 3 \
  --gate-every 1000000 --arch-every 0 --control-every 0 \
  --seed 20260910 --out d5.net --ledger ledger_d5.jsonl > d5.log 2>&1

echo "depth5: $(grep -cE '^gen ' d5.log) generations in ${SECS}s"
nice -n 19 taskset -c "$CORES5" "$NM" d5.net d5_start.net 224 > d5_vs_start.log 2>&1
echo "  depth5 vs its own start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' d5_vs_start.log | head -1)"
echo "  (compare against the depth-3 run's margin from the same start)"
