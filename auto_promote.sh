#!/usr/bin/env bash
# BANK IMPROVEMENT AUTOMATICALLY. Twice tonight the running net had already passed the champion and
# only got promoted because I happened to run a head-to-head by hand. Between those checks the gain
# was real but unbanked, and a crash would have lost it.
#
# THE RULER IS NOT USED HERE, deliberately. It carries +/-50 Elo at 120 games and produced a
# four-reading "decline" while the net was genuinely stronger (champion_deep_RESULT.md). Direction
# needs the PAIRED instrument, so promotion is decided by netmatch and nothing else.
#
# Acceptance is the project's own rule: rate - ci95 >= 0.5. Same bar the manual promotions cleared.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
NM=$SCR/xt_cap/release/examples/netmatch
PAIRS=${PAIRS:-224}
EVERY=${EVERY:-1800}
# 6-11 is Existence's half of the box. It was 6-15, which reaches onto 12-15 -- the four cores
# reserved for HIS desktop. A background measurement is never allowed to take those.
CORES=${CORES:-6-11}
[ -x "$NM" ] || { echo "no netmatch at $NM"; exit 1; }

# DETECTION AND PROMOTION ARE SEPARATED, and that is the whole point of this revision.
#
# The old loop assigned `cand` inside a scan over every running trainer, so the LAST one seen won
# -- an arbitrary arm whenever two arms train at once. Because that arm could be silently
# promoted into the shared p1_champion.net, the only safe response was to STOP the script for the
# duration of an A/B. That threw away the detector along with the promoter, and this script's one
# confirmed catch is a REGRESSION (hold, 0.444 +/- 0.030, interval entirely below 0.5, ~1500
# generations that were costing strength invisibly to both loss and the ruler).
#
# So: MEASURE EVERY live arm, always. PROMOTE only when exactly one arm is training, where
# "the candidate" is unambiguous. A regression is now reported during an A/B instead of going
# unwatched precisely when two experimental arms are running.
while true; do
  sleep "$EVERY"
  # Arms are identified by the running trainer's own --out. `\-\-out` emits a stray-backslash
  # warning from grep; a bracket expression is the portable spelling.
  arms=""
  for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
    e=$(readlink /proc/$p/exe 2>/dev/null) || continue; e=${e% (deleted)}
    case "$e" in */release/learn)
      o=$(tr '\0' ' ' < /proc/$p/cmdline 2>/dev/null | grep -oE '[-][-]out [^ ]+' | awk '{print $2}')
      [ -n "$o" ] && arms="$arms $o";;
    esac
  done
  arms=$(printf '%s\n' $arms | sort -u)
  n=$(printf '%s\n' $arms | grep -c .)
  [ "$n" -gt 0 ] || { echo "$(date '+%H:%M') no trainer running -- skipping"; continue; }

  for cand in $arms; do
    [ -s "$cand" ] || continue
    clog="${cand%.net}.log"
    G=$(grep -cE '^gen ' "$clog" 2>/dev/null)
    # Copy before matching: the trainer rewrites this file every generation, and reading it live
    # would compare a net that changed halfway through its own match.
    cp -f "$cand" "/tmp/ap_$(basename $cand)" || continue
    nice -n 19 taskset -c "$CORES" "$NM" "/tmp/ap_$(basename $cand)" p1_champion.net "$PAIRS" \
      > "ap_$(basename ${cand%.net}).log" 2>&1
    line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' "ap_$(basename ${cand%.net}).log" | head -1)
    rate=$(echo "$line" | grep -oE '0\.[0-9]+' | head -1)
    ci=$(echo "$line"   | grep -oE '0\.[0-9]+' | tail -1)
    rm -f "/tmp/ap_$(basename $cand)"
    # An unparsable result is HOLD, never promote. A missing rate means the match did not run,
    # and treating "no evidence" as "no objection" is how an unmeasured net becomes champion.
    [ -z "$rate" ] && { echo "$(date '+%H:%M') $(basename $cand) gen $G: netmatch produced no rate -- NOT promoting"; continue; }

    verdict=$(python3 -c "
r,c=$rate,$ci
print('PROMOTE' if r-c>=0.5 else ('REGRESSION' if r+c<0.5 else 'hold'))" 2>/dev/null)

    if [ "$verdict" = "PROMOTE" ] && [ "$n" -eq 1 ]; then
      cp -f p1_champion.net "p1_champion_prev_g${G}.net"
      cp -f "$cand" p1_champion.net
      echo "$(date '+%H:%M') $(basename $cand) gen $G: PROMOTED   $rate +/- $ci"
    elif [ "$verdict" = "PROMOTE" ]; then
      # Measured as better, but $n arms are training: promoting one into the champion both arms
      # are judged against would change the A/B's baseline mid-experiment.
      echo "$(date '+%H:%M') $(basename $cand) gen $G: PASSES but $n arms training -- NOT promoting  $rate +/- $ci"
    elif [ "$verdict" = "REGRESSION" ] && [ "${G:-0}" -lt 1000 ]; then
      # THE RESUME TRANSIENT IS NOT A REGRESSION, and calling it one makes this a false alarm on
      # every fresh run. resume_dip_RESULT.md measured a resumed arm against the champion it
      # resumed from: 0.492 at gen 5, 0.411 at gen 25, 0.366 at gen 100 (-95.4 Elo), recovering to
      # 0.557 by gen 4,327. Sub-parity before ~1,000 generations is the EXPECTED shape, reproduced
      # independently by this very script (it read 0.362 +/- 0.031 at gen 108 against that run's
      # 0.366 +/- 0.035 at gen 100 -- two harnesses agreeing to 0.004).
      #
      # A monitor that fires on normal behaviour trains its reader to ignore it, which is worse
      # than silence. Still REPORTED, because a deeper-than-expected dip is worth seeing -- just
      # not labelled as a fault.
      echo "$(date '+%H:%M') $(basename $cand) gen $G: transient $rate +/- $ci  (below 0.5, but gen $G < 1000 -- expected resume dip, see resume_dip_RESULT.md)"
    elif [ "$verdict" = "REGRESSION" ]; then
      echo "$(date '+%H:%M') $(basename $cand) gen $G: REGRESSION $rate +/- $ci  (interval entirely below 0.5, PAST the resume transient)"
    else
      echo "$(date '+%H:%M') $(basename $cand) gen $G: hold       $rate +/- $ci"
    fi
  done
done
