#!/usr/bin/env bash
# SECOND TRAINING SEED for the blend decision, judged at DEPTH 4. The nets already exist.
#
# WHAT IS ACTUALLY OPEN. STATE.md:257-283 records a direct head-to-head between blend 0.75 and 1.00
# resolving at **+0.050 ± 0.016 in favour of 1.00** — but at **depth 2**. The block immediately
# below it (STATE.md:277) states the threat plainly:
#
#     "bh_100 is stronger at depth 2 and not at depth 4. That is a real depth-dependent difference
#      in the nets, not an instrument failing -- and it directly threatens the blend candidate,
#      whose whole case was built at depth 2."
#
# Depth 4 is this project's strength standard (`netmatch.rs:23-29`). So the blend candidate's entire
# case rests on a measurement taken at the wrong depth, and the deciding number — a DIRECT match at
# depth 4 — has never been taken. `blend_h2h.sh` is taking it on training seed 20260907. This takes
# it on the second training seed, which `blend_seed2.sh` calls "the only thing between here and
# shipping".
#
# WHY IT COSTS NOTHING. `s2_075.net` and `s2_100.net` are completed 20-generation arms at seed
# 424242, produced by `blend_seed2.sh` and sitting on disk unused. Only the match is missing.
#
# WHY A DIRECT MATCH AND NOT THE ORIGIN CONTROL, for this pair especially. STATE.md:285-299 records
# that the "frozen" origin is `Net::random(WIDTH_MENU[rung], seed)` — it depends on the RUN SEED, so
# arms at different seeds face DIFFERENT opponents and their origin rates are not comparable. That
# is the stated explanation for `s2_100` reading 0.945 where `bh_100` read 0.832. A direct match
# involves no origin at all, so it is immune to that entirely, and it is the only way to compare
# across the two training seeds.
#
# PRE-REGISTERED, matching blend_seed2.sh's own wording:
#   * 1.00 wins at depth 4 on BOTH seeds => two training seeds agree at the strength standard, and
#     the default should move. Report as "passed the gate", never as "+N Elo" — no Elo is measured.
#   * 0.75 wins at depth 4 => the +0.050 was a depth-2 artifact, exactly as STATE.md:277 warns, and
#     the shipped default stands on a measurement rather than on the risk argument alone.
#   * SEEDS DISAGREE => the effect is seed-specific and a shipped default must not move on it.
set -uo pipefail
cd "$(dirname "$0")"
CORE=${CORE:-14}
PAIRS=${PAIRS:-448}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}
[ -x "$NM" ] || { echo "no netmatch at $NM"; exit 1; }
for n in s2_075 s2_100; do [ -f "$n.net" ] || { echo "ABORT: $n.net missing"; exit 1; }; done

echo "=== seed 424242: blend 0.75 vs 1.00, DIRECT, depth 4 (strength standard) ==="
timeout 5400 taskset -c "$CORE" nice -n 19 ionice -c 3 "$NM" s2_075.net s2_100.net "$PAIRS" 4 777 \
  2>&1 | grep -E 'scores|=>|arms:' | sed 's/^/  /'
echo
echo "=== the same pair at DEPTH 2, where the +0.050 was measured ==="
echo "  Included deliberately: if 1.00 wins at depth 2 and not at depth 4 on THIS seed too, the"
echo "  depth-dependence is reproduced and the blend case is a depth-2 artifact rather than noise."
timeout 5400 taskset -c "$CORE" nice -n 19 ionice -c 3 "$NM" s2_075.net s2_100.net "$PAIRS" 2 777 \
  2>&1 | grep -E 'scores|=>|arms:' | sed 's/^/  /'
echo
echo "  s2_075's score is shown; below 0.5 means blend 1.00 is stronger."
