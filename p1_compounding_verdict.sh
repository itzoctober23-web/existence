#!/usr/bin/env bash
# READ THE P1 COMPOUNDING A/B BY THE RULE THAT WAS REGISTERED BEFORE IT RAN.
#
# Written while the arms are still running, deliberately, so the verdict rule cannot be chosen after
# seeing the numbers. Everything here is a transcription of p1_compounding_PREREG.md.
#
# IT REFUSES TO REPORT AN UNFINISHED ARM. The standing reporting rule is: a running experiment writes
# to STATE only, and a _RESULT.md requires the planned N complete. Four retractions in one day came
# from reading partial runs, so that rule is enforced HERE, in code, rather than remembered. An arm
# short of 2,000 generations exits non-zero and prints nothing that looks like a verdict.
#
# THE TWO PRE-REGISTERED TESTS
#   RULER   pooled rating of each arm's final net, CIs must be DISJOINT for a win.
#   GATE    netmatch COMPOUND vs CONTROL, the normal promotion rule: rate - ci95 >= 0.5.
#
# THREE SEEDS PER ARM, NOT ONE. One ruler sample is a lottery; between-seed sd on this box is 0.047
# for the paired instrument, and a single reading inside that is not a result.
#
# MATCHED ON GENERATIONS, NOT WALL CLOCK. Both arms are held to the same generation count; their
# wall clocks differ because the compound arm pays for 20 gate calls and because search_long_run
# shares cores 6-11. That is expected and is why the comparison is not a speed comparison.
set -uo pipefail
cd /home/maswabe/existence || exit 1

WANT=${WANT:-2000}
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
NM=$SCR/xt_cap/release/examples/netmatch
PAIRS=${PAIRS:-224}
CORES=${CORES:-6-11}

fail(){ echo "REFUSING: $*"; exit 1; }

for t in p1c_control p1c_compound; do
  [ -s "$t.net" ] || fail "$t.net missing -- the arm has not produced a net"
  g=$(grep -cE '^gen ' "$t.log" 2>/dev/null || true)
  g=${g:-0}
  echo "$t: $g generations"
  [ "$g" -ge "$WANT" ] || fail "$t reached $g of $WANT generations. Planned N is NOT complete, so this
  is STATE, not a result. Report progress in STATE.md and do not write a _RESULT.md."
done

echo "=============================================================================="
echo " P1 COMPOUNDING -- reading by the PRE-REGISTERED rule (both arms at $WANT gens)"
echo "=============================================================================="

# ---- TEST 1: the ruler, three seeds per arm -------------------------------------------------
echo ""
echo "RULER (sf_ruler.py, depth 4 vs SF-1320, 3 seeds per arm)"
for t in p1c_control p1c_compound; do
  for s in 20260911 20260912 20260913; do
    # Capture to a FILE, then read it. sf_ruler's real lines are
    #   "  W-D-L w-d-l   score 0.xxxx"  and  "  Elo vs this opponent: +NN +/- NN"
    # The first version of this grep looked for lowercase "elo" and would have matched NOTHING while
    # still printing a plausible-looking line. Patterns are taken from the source, not guessed.
    # THE PRODUCTION RUNG, not sf_ruler's defaults. live_ruler.sh:28 uses
    #   --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120
    # and the pooled ruler these arms are compared against is built from THAT. sf_ruler's default is
    # --sf-nodes 1000, a much weaker opponent: MEASURED on the champion it returns 8-0-0, score
    # 1.0000, "a clean sweep -- this is a BOUND, not a rating". Reading the arms at the default would
    # have swept both and produced no comparison at all.
    o=/tmp/p1cv_${t}_${s}.txt
    nice -n 19 taskset -c "$CORES" python3 sf_ruler.py --net "$t.net" \
        --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 --seed "$s" > "$o" 2>/dev/null
    # `score 1.0000` exists, so the leading digit is [01] -- a `0\.` pattern silently matches nothing
    # on exactly the sweep case the BOUND warning is about.
    sc=$(grep -oE 'score [01]\.[0-9]+' "$o" | tail -1)
    el=$(grep -oE 'Elo vs this opponent: [-+][0-9]+ \+/- [0-9]+' "$o" | tail -1)
    wdl=$(grep -oE 'W-D-L [0-9]+-[0-9]+-[0-9]+' "$o" | tail -1)
    # A degenerate score is a BOUND, not a rating -- sf_ruler says so itself. Reporting it as a
    # rating is how a saturated ruler gets mistaken for a flat engine.
    if grep -qF 'BOUND, not a rating' "$o"; then
      echo "  $t seed $s: $wdl $sc  *** BOUND, NOT A RATING -- ruler out of range, re-run weaker/stronger ***"
    else
      echo "  $t seed $s: ${wdl:-?} ${sc:-?}  ${el:-<no Elo line>}"
    fi
  done
done

# ---- TEST 2: the gate, the normal promotion rule ---------------------------------------------
echo ""
echo "GATE (netmatch COMPOUND vs CONTROL, $PAIRS pairs -- promotion rule rate - ci95 >= 0.5)"
[ -x "$NM" ] || fail "no netmatch binary at $NM"
out=$(nice -n 19 taskset -c "$CORES" "$NM" p1c_compound.net p1c_control.net "$PAIRS" 2>&1)
echo "$out" | tail -4
line=$(echo "$out" | grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' | head -1)
[ -n "$line" ] || fail "netmatch produced no 'scores' line -- a missing rate is indistinguishable
  from a match still running, so nothing is concluded"
rate=$(echo "$line" | awk '{print $2}')
ci=$(echo "$line" | awk '{print $4}')
echo ""
echo "  rate $rate  ci95 $ci"
awk -v r="$rate" -v c="$ci" 'BEGIN{
  lo = r - c;
  printf("  rate - ci95 = %.4f -> %s\n", lo, (lo >= 0.5 ? "COMPOUND PASSES the promotion rule" : "NOT a pass"));
  if (lo < 0.5 && r > 0.5) printf("  (point estimate favours COMPOUND but does not clear the bar; between-seed sd here is 0.047)\n");
}'
echo ""
echo " Disjoint ruler CIs are required for the ruler half. A netmatch pass alone is a gate result,"
echo " not a ruler result, and the PREREG requires both to be reported."
