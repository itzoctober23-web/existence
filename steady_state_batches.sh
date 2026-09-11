#!/usr/bin/env bash
# IS A 5-GENERATION STEP NET-NEGATIVE IN THE STEADY STATE, OR ONLY JUST AFTER A RESUME?
#
# `generator_is_net_negative_RESULT.md` measured 20 direct batch decisions at mean 0.4684, CI
# [0.4525, 0.4843], and concluded five generations make the net worse. That measurement has a scope
# limit I did not state when publishing it:
#
#   The batch gate rolls back on reject, so the BASE only moves on a KEEP -- and there was exactly
#   ONE KEEP, at g10. So decisions 1-2 measured from the start and decisions 3-20 measured from the
#   g10 champion. **All 20 samples are "5 generations from a net at most 10 generations past the
#   champion"** -- the early post-resume regime.
#
# That regime is precisely the one `resume_dip_RESULT.md` shows is anomalous: a resumed run loses
# ~95 Elo by generation 100 before recovering to +39.8 by 4,327. So 0.4684 may simply BE the resume
# transient, measured per batch, rather than a property of the loop in general. The two findings
# would then be one finding, and "the generator is net-negative" would be overstated.
#
# THIS COSTS NO TRAINING. `prod2` is already running ungated -- it adopts every candidate -- so two
# snapshots of its `--out` taken 5 generations apart ARE "base" and "base + 5 generations" in the
# steady state. Sample pairs from the live run and match them head-to-head; no second trainer, no
# extra datagen, and it measures the production configuration rather than a proxy for it.
#
# PRE-REGISTERED READING (statistic: mean of the pair scores, same as the control):
#   * mean ~0.4684 -> the negative expectation is REAL and general; the published finding stands and
#     the scope note is unnecessary.
#   * mean at or above 0.5 -> the negative expectation is a property of the POST-RESUME regime, not
#     of the loop. `generator_is_net_negative_RESULT.md` must be narrowed to that regime, and it and
#     `resume_dip_RESULT.md` are describing the same phenomenon from two directions.
#   * mean between -> the effect decays with distance from the resume; report the two numbers side by
#     side and do not pick one.
#
# The snapshots are COPIED, never read in place: the trainer rewrites that file every generation and
# a net being written while it is played is a net that changed partway through its own match.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
NM=$SCR/xt_cap/release/examples/netmatch
SRC=${SRC:-prod2}
PAIRS=${PAIRS:-160}
N=${N:-10}
STEP=${STEP:-5}
[ -x "$NM" ] || { echo "no netmatch at $NM"; exit 1; }
[ -s "${SRC}.net" ] || { echo "no ${SRC}.net"; exit 1; }

gens() { local c; c=$(grep -cE '^gen ' "${SRC}.log" 2>/dev/null); echo "${c:-0}"; }

echo "$(date '+%H:%M') steady-state batches from $SRC (currently $(gens) generations), $N pairs $STEP apart"
mkdir -p /tmp/ssb && rm -f /tmp/ssb/*.net
for i in $(seq 1 "$N"); do
  g0=$(gens); cp -f "${SRC}.net" "/tmp/ssb/a$i.net" || continue
  # wait for STEP more generations
  while :; do
    g=$(gens); [ "$g" -ge $((g0 + STEP)) ] && break
    # the trainer may have stopped; do not spin forever
    pgrep -x learn >/dev/null 2>&1 || { grep -q . /dev/null; }
    sleep 1
  done
  cp -f "${SRC}.net" "/tmp/ssb/b$i.net" || continue
  # A pair is only usable if the net actually CHANGED. Identical md5 means the snapshots caught the
  # same net and the match would be a degenerate A/A returning 0.500 by construction.
  if cmp -s "/tmp/ssb/a$i.net" "/tmp/ssb/b$i.net"; then
    echo "  pair $i: snapshots identical at gen $g0 -> $(gens), discarding"
    rm -f "/tmp/ssb/a$i.net" "/tmp/ssb/b$i.net"
  else
    echo "  pair $i: gen $g0 -> $(gens) captured"
  fi
done

echo "  matching $(ls /tmp/ssb/a*.net 2>/dev/null | wc -l) usable pairs at $PAIRS pairs each"
: > /tmp/ssb/scores.txt
for f in /tmp/ssb/a*.net; do
  i=$(basename "$f" .net); i=${i#a}
  [ -s "/tmp/ssb/b$i.net" ] || continue
  nice -n 19 taskset -c 6-11 "$NM" "/tmp/ssb/b$i.net" "/tmp/ssb/a$i.net" "$PAIRS" > "/tmp/ssb/m$i.log" 2>&1
  s=$(grep -oE 'scores 0\.[0-9]+' "/tmp/ssb/m$i.log" | head -1 | awk '{print $2}')
  [ -n "$s" ] && { echo "$s" >> /tmp/ssb/scores.txt; echo "    pair $i: $s"; }
done

python3 - <<'PY'
import statistics as st
v=[float(x) for x in open('/tmp/ssb/scores.txt')]
if len(v) < 3:
    print(f"  only {len(v)} usable pairs -- not enough to summarise"); raise SystemExit
se=st.stdev(v)/len(v)**0.5
lo,hi=st.mean(v)-1.96*se, st.mean(v)+1.96*se
print(f"  STEADY STATE  n={len(v)}  mean {st.mean(v):.4f}  95% CI [{lo:.4f}, {hi:.4f}]")
print(f"    below 0.5: {sum(1 for x in v if x<0.5)}/{len(v)}")
print(f"    POST-RESUME control: mean 0.4684  CI [0.4525, 0.4843]  16/20 below 0.5")
print("    READING: " + ("negative expectation is REAL AND GENERAL" if hi < 0.5
      else "negative expectation is a POST-RESUME artefact -- narrow the published claim" if lo >= 0.5
      else "unresolved -- interval contains 0.5, report both numbers"))
PY
echo "STEADYSTATEDONE"
