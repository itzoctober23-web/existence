#!/usr/bin/env bash
# BLEND 0.85 vs THE SHIPPED 0.75 — full length, matched arms, shared start.
#
# WHY THIS, AND WHY NOW.
# `ruler_trend_RESULT.md` (2026-09-11) established that NOTHING is gained by running longer:
# all seven production runs are FLAT on the absolute ruler (|z| < 2 every one, chi2/dof 0.32-0.75,
# and the ruler is NOT saturated -- 0.72 observed against a ~0.95 ceiling). What DOES move is the
# configuration: prod1 1315 -> prodk0759 1481 is +166 +/- 22 across runs, against +28 +/- 38 from
# 16,680 generations WITHIN a run. We are 119 Elo short of his 1600 target, so the only thing that
# closes it is another configuration change of roughly the size the lr change gave.
#
# `blend_RESULT.md` names the candidate: "blend 0.85 is now the best-evidenced open candidate in
# the tree", replicated on FIVE independent training seeds, with a non-monotonic response --
# 0.75 -> 0.85 gains, 0.85 -> 1.00 gives it back, so 1.00 is dead and 0.85 is not merely "more".
#
# WHAT IS MISSING, AND IT IS EXACTLY WHAT THE LR SWEEP WAS MISSING. Every one of those arms is a
# TWENTY-generation run. The lr result looked settled at ~650 generations too, and the full-length
# re-run moved the magnitude (0.551 -> 0.535 for the winner, and the control fell to 0.412). The
# pre-registration in blend_RESULT.md is explicit and binding: 0.85 is "NOT a default change" and
# "the shipped default stays 0.75 until something authorised to move it does so." A full-length
# matched pair, gated by the normal rule, is what authorises it.
#
# THE BLIND-METRIC TRAP THIS DELIBERATELY AVOIDS. main.rs:490-509 records a blend sweep where
# 0.75/0.85/0.95/1.00 all overlap -- an apparent null. blend_RESULT.md proves that instrument
# COMPRESSES this contrast ~3x (+0.036 vs +0.112 on the same arms) and concludes: "a NULL measured
# on the blind metric is worthless. Compression manufactures nulls." So the verdict here is the
# PAIRED arm-vs-arm netmatch, never the candidate-vs-champion gate rate.
#
# LOAD. Two arms, --threads 1 each, pinned to 6-11 (Existence's cores; his 12-15 are never
# touched, 4PC owns 0-5) at nice 19 / ionice idle. Production keeps running beside them, which is
# deliberate: a PAIRED comparison cancels contention, and both arms feel the same box.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_lrd/learn          # immutable snapshot, never the build tree
NM=$SCR/xt_lrd/netmatch
GENS=${GENS:-2000}
PAIRS=${PAIRS:-224}
CAP=${CAP:-18000}
SEED=${SEED:-20260917}           # fresh seed: not one the 20-gen campaign or the lr sweep used
LOG=blend_sweep.log
say(){ echo "$(date +%F_%H:%M) [blend] $*" | tee -a "$LOG"; }

[ -x "$LEARN" ] || { say "ABORT: no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { say "ABORT: no netmatch at $NM"; exit 1; }

# ---- PROVE THE FLAG BINDS ------------------------------------------------------------------
# A sweep whose knob is silently ignored runs every arm on the default and produces a confident
# null. That has happened on this box (an env var set for the wrong side of a pipeline ran every
# arm on the default net and forced a retraction), which is why lr_sweep_low.sh probes --lr before
# it starts. main.rs:569 echoes `blend={blend}` in the run header, so the same check works here.
# NOTE: this binary has NO --help; asking for one starts a run. Probe with --gens 0 only.
say "probing that the deployed learn honours --blend"
timeout 120 "$LEARN" --gens 0 --games 2 --blend 0.85 --out "$SCR/blend_probe.net" > /tmp/blend_probe.txt 2>&1 || true
if grep -qE 'blend=0\.85' /tmp/blend_probe.txt; then
  say "  OK: header reports $(grep -oE 'blend=[0-9.]+' /tmp/blend_probe.txt | head -1)"
else
  say "ABORT: deployed learn does not honour --blend (header: $(head -c 120 /tmp/blend_probe.txt | tr '\n' ' '))"
  exit 1
fi
# And prove the DEFAULT is what we think, so the control arm is the shipped configuration.
timeout 120 "$LEARN" --gens 0 --games 2 --out "$SCR/blend_probe0.net" > /tmp/blend_probe0.txt 2>&1 || true
grep -qE 'blend=0\.75' /tmp/blend_probe0.txt \
  && say "  OK: default blend is 0.75, so the control arm IS the shipped setting" \
  || say "  WARN: default blend is $(grep -oE 'blend=[0-9.]+' /tmp/blend_probe0.txt | head -1) -- control is explicit anyway"

# ---- DO NOT STACK TRAINERS -----------------------------------------------------------------
# Box limit is hard: stacked relaunchers have frozen this machine. Production is one `learn`; two
# arms make three, which fits 6 cores. More than one EXTRA is refused.
n=0
for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue; e=${e% (deleted)}
  case "${e##*/}" in learn) n=$((n+1));; esac
done
say "learn processes already live: $n"
[ "$n" -gt 1 ] && { say "ABORT: $n learn processes already live -- not yet runnable"; exit 75; }

[ -s p1_champion.net ] || { say "ABORT: no champion net"; exit 1; }
cp -f p1_champion.net blend_start.net
say "shared start $(md5sum blend_start.net | cut -c1-12), seed $SEED, $GENS gens per arm, cap ${CAP}s"

for B in 075 085; do
  V=$(echo "$B" | sed 's/^0*//;s/^\(.\)\(.*\)/0.\1\2/')
  say "arm blend $V -> blend_$B.net"
  ( timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
      --init blend_start.net --gens "$GENS" --games 8 --threads 1 --depth 3 --epochs 3 \
      --lr 0.0002 --blend "$V" --gate-every 1000000 --arch-every 0 --control-every 0 \
      --seed "$SEED" --out "blend_$B.net" --ledger "blend_$B.jsonl" > "blend_$B.log" 2>&1 ) &
done
wait
say "both arms finished"

# ---- ARMS MUST BE MATCHED ------------------------------------------------------------------
# An unequal pair confounds the blend with the amount of training. The lr sweep's whole first
# attempt was invalidated this way (arms unmatched at ~650 generations).
g075=$(grep -cE '^gen ' blend_075.log 2>/dev/null); g085=$(grep -cE '^gen ' blend_085.log 2>/dev/null)
say "generations completed: 0.75 -> ${g075:-0}, 0.85 -> ${g085:-0}"
if [ "${g075:-0}" -ne "${g085:-0}" ]; then
  say "UNEQUAL ARMS (${g075:-0} vs ${g085:-0}) -- the comparison is CONFOUNDED with training amount."
  say "  Reporting it rather than hiding it. Re-run with a larger CAP before drawing a verdict."
fi
# Confirm each arm actually ran at its own blend, from its own log header.
say "header check: 0.75 arm -> $(grep -oE 'blend=[0-9.]+' blend_075.log | head -1), 0.85 arm -> $(grep -oE 'blend=[0-9.]+' blend_085.log | head -1)"

# ---- VERDICT: PAIRED, arm vs arm, and each vs the shared start ------------------------------
for pair in "085:075" "085:start" "075:start"; do
  A=${pair%%:*}; Bp=${pair##*:}
  NA="blend_$A.net"; NB=$([ "$Bp" = start ] && echo blend_start.net || echo "blend_$Bp.net")
  [ -s "$NA" ] && [ -s "$NB" ] || { say "skip $A vs $Bp: missing net"; continue; }
  out="blend_${A}_vs_${Bp}.log"
  nice -n 19 taskset -c 6-11 "$NM" "$NA" "$NB" "$PAIRS" > "$out" 2>&1
  line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' "$out" | head -1)
  say "  blend 0.$A vs $Bp: ${line:-NO VERDICT}"
done

say "REMINDER: the shipped default stays 0.75 unless 0.85 clears rate - ci95 >= 0.5 head to head."
say "BLENDSWEEPDONE"
