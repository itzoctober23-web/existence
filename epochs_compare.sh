#!/usr/bin/env bash
# THE COMPARISON epochs_sweep.sh COULD NOT RUN — epochs 2 vs the shipped 3, at full length.
#
# WHY THIS EXISTS AS A SEPARATE SCRIPT.
# `epochs_sweep.sh` is a half-converted copy of the blend sweep. Its ARMS were correctly switched to
# `--epochs` and they ran clean: the log records "OK: header reports epochs=2" and "default epochs is
# 3, so the control arm IS the shipped setting", and both arms completed 2000/2000 generations from
# the shared start `abbbd0d0c5e0` on seed 20260919. The TRAINING is a valid, matched experiment.
#
# Everything downstream of the arms is still the blend sweep. The verdict loop matches
# `blend_02.net` against `blend_03.net`; the arms wrote `epochs_02.net` and `epochs_03.net`. Neither
# blend file exists, so all three pairs hit `skip: missing net` and the sweep ends with NO VERDICT --
# 2 x 2000 generations of compute reporting nothing. It also signs off "the shipped default stays
# 0.75 unless 0.85 clears" and prints BLENDSWEEPDONE, which describes a different experiment.
#
# The script was RUNNING when this was found, and bash reads a script by BYTE OFFSET while executing
# it, so editing it in place is the failure that destroyed 2811 pairs on this project. A rename-swap
# would not help either: the running shell holds the old inode. The training output is already on
# disk and complete, so the correct move is to leave the running instance alone and do the
# comparison here.
#
# WAITS ON THE CONDITION, NOT A PROXY. The nets were written at 11:42 while both learn processes were
# still at 88% CPU, so a match started then could read a net that is about to be rewritten. This
# waits for NO learn process to hold `--out epochs_*.net`, then requires the file mtimes to sit
# still across two checks before matching. A unit name or a sleep would not have caught either case.
#
# PRE-REGISTERED, and note this is a DIFFERENT question from the blend sweep's:
#   * `02 vs 03` clears `rate - ci95 >= 0.5`  -> epochs 2 beats the shipped 3 on this seed. That is
#     ONE seed. The between-seed sd on this project is 0.047, so a margin inside that is NOT a
#     default change -- it buys a second seed, exactly as blend 0.85 had to.
#   * `03 vs 02` clears instead -> the shipped 3 is confirmed, and epochs is closed as a lever.
#   * neither clears -> epochs is FLAT between 2 and 3 at full length. Record the bound and stop.
#
# THE ARM-vs-START MATCHES ARE A REAL CONTROL, not decoration. TWICE today an incumbent setting
# scored BELOW its own starting net at full length (lr 0.0005 at 0.412, blend 0.75 at 0.438 on seed
# 2). If both arms here land under 0.5 against the shared start, the finding is about the training
# regime and the 02-vs-03 contrast is a comparison between two regressions.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
NM=$SCR/xt_lrd/netmatch
PAIRS=${PAIRS:-224}
LOG=epochs_compare.log
say(){ echo "$(date +%F_%H:%M) [epochs-cmp] $*" | tee -a "$LOG"; }

[ -x "$NM" ] || { say "ABORT: no netmatch at $NM"; exit 1; }

# ---- WAIT FOR THE ARMS TO ACTUALLY EXIT ------------------------------------------------------
arms_alive(){
  local n=0
  for p in /proc/[0-9]*; do
    local e
    e=$(readlink "$p/exe" 2>/dev/null) || continue
    e=${e% (deleted)}
    [ "${e##*/}" = learn ] || continue
    local o
    o=$(tr '\0' '\n' < "$p/cmdline" 2>/dev/null | grep -A1 -x -- '--out' | tail -1) || continue
    case "$o" in *epochs_0*.net) n=$((n+1));; esac
  done
  echo "$n"
}
say "waiting for both epochs arms to exit (they were still at 88% CPU after writing their nets)"
while [ "$(arms_alive)" -gt 0 ]; do sleep 30; done
say "arms have exited"

# mtime must sit still: a net still being written is a torn read, and netmatch would not complain.
prev=""
for _ in 1 2 3; do
  cur=$(stat -c '%Y' epochs_02.net epochs_03.net 2>/dev/null | tr '\n' ' ')
  [ -n "$cur" ] && [ "$cur" = "$prev" ] && break
  prev=$cur; sleep 20
done
say "net mtimes stable: $prev"

# ---- ARMS MUST BE MATCHED --------------------------------------------------------------------
g02=$(grep -cE '^[[:space:]]*gen ' epochs_02.log 2>/dev/null)
g03=$(grep -cE '^[[:space:]]*gen ' epochs_03.log 2>/dev/null)
say "generations: epochs 2 -> ${g02:-0}, epochs 3 -> ${g03:-0}"
if [ "${g02:-0}" -ne "${g03:-0}" ]; then
  say "UNEQUAL ARMS -- the contrast is CONFOUNDED with training amount. Reporting, not hiding."
fi
say "header check: 02 arm -> $(grep -oE 'epochs=[0-9]+' epochs_02.log | head -1), 03 arm -> $(grep -oE 'epochs=[0-9]+' epochs_03.log | head -1)"

for f in epochs_02.net epochs_03.net epochs_start.net; do
  [ -s "$f" ] || { say "ABORT: $f missing or empty"; exit 1; }
done

# ---- THE THREE PAIRED MATCHES ----------------------------------------------------------------
for pair in "02:03" "02:start" "03:start"; do
  A=${pair%%:*}
  Bp=${pair##*:}
  NA="epochs_$A.net"
  if [ "$Bp" = start ]; then NB=epochs_start.net; else NB="epochs_$Bp.net"; fi
  out="epochs_${A}_vs_${Bp}.log"
  say "matching epochs $A vs $Bp at $PAIRS pairs"
  nice -n 19 taskset -c 6-11 "$NM" "$NA" "$NB" "$PAIRS" > "$out" 2>&1
  line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' "$out" | head -1)
  say "  epochs $A vs $Bp: ${line:-NO VERDICT -- check $out, an empty result here is usually a broken run}"
done

say "VERDICT, against the pre-registration in this file's header:"
python3 - <<'PY' | tee -a "$LOG"
import re, os
def rd(p):
    if not os.path.exists(p): return None
    m = re.search(r'scores (0\.\d+) \+/- (0\.\d+)', open(p).read())
    return (float(m.group(1)), float(m.group(2))) if m else None
h  = rd('epochs_02_vs_03.log')
a  = rd('epochs_02_vs_start.log')
b  = rd('epochs_03_vs_start.log')
if h is None:
    print("  no head-to-head verdict parsed -- claiming nothing.")
else:
    r, c = h
    print(f"  epochs 2 vs epochs 3 (shipped):  {r:.3f} +/- {c:.3f}   lower bound {r-c:.3f}")
    if r - c >= 0.5:
        print("  epochs 2 CLEARS the promotion rule on this seed.")
        print(f"  Margin over 0.5 is {r-0.5:.3f}; the between-seed sd is 0.047.")
        print("  NOT a default change yet" if (r-0.5) < 0.047 else "  Margin exceeds the between-seed sd")
        print("  -- next step is a SECOND SEED, the same bar blend 0.85 had to clear.")
    elif r + c <= 0.5:
        print("  the shipped epochs 3 WINS. epochs is closed as a lever; record it and stop.")
    else:
        print(f"  FLAT: the interval spans 0.5. Bound is +/-{c:.3f} at 224 pairs. Record and stop --")
        print("  do not buy a third arm for an effect this size.")
for lbl, v in (("epochs 2", a), ("epochs 3", b)):
    if v: print(f"  CONTROL {lbl} vs the shared start: {v[0]:.3f} +/- {v[1]:.3f}"
                + ("   <-- BELOW its own start" if v[0] + v[1] < 0.5 else ""))
if a and b and a[0] + a[1] < 0.5 and b[0] + b[1] < 0.5:
    print("  BOTH ARMS ARE BELOW THE SHARED START. The head-to-head is then a comparison between two")
    print("  regressions, and the finding is about the training regime, not about epochs.")
PY
say "EPOCHSCOMPAREDONE"
