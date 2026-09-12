#!/usr/bin/env bash
# Gate Candidate A: node-budget datagen (arm B) against fixed depth 3 (arm A).
#
# Both arms were launched 2026-09-12 02:00 from the SAME frozen champion (cand_start.net, md5
# verified against p1_champion.net at launch), same seed, same binary, --gens 2000 --games 8,
# differing ONLY in `--datagen-budget 5269`. Matched on GENERATIONS, per the PREREG -- wall-clock
# matching would confound, because a budget changes per-generation cost.
#
# WHY THIS CAN RUN ON A BUSY BOX. netmatch plays at FIXED DEPTH 4, so its node counts are
# deterministic and its result is load-immune. That is the opposite of a movetime match against an
# external opponent, which needs a quiet box. No deferral on load is therefore needed or wanted --
# waiting for quiet would just delay the answer.
#
# WHAT IT REFUSES TO DO. It will not report a verdict from arms that did not finish, did not move
# off their start net, or produced an unparsable match. Each of those is an ABORT, not a number.
set -uo pipefail

D=/home/maswabe/existence
cd "$D" || exit 1
NM=$D/target/release/examples/netmatch
PAIRS=${PAIRS:-224}
DEPTH=4
LOG=$D/gate_candidate_a.log
say(){ echo "$(date +%F_%H:%M) [candA] $*" | tee -a "$LOG"; }

[ -x "$NM" ] || { say "ABORT: no netmatch at $NM"; exit 1; }

# ---- guard: both arms must be COMPLETE -----------------------------------------------------
for u in cand-a-fixed cand-b-budget; do
  st=$(systemctl --user is-active "$u.service" 2>/dev/null)
  if [ "$st" = "active" ]; then
    say "DEFER: $u is still running"
    exit 75
  fi
done
for f in candA_fixed candB_budget; do
  g=$(grep -c '^gen ' "$D/$f.log" 2>/dev/null || echo 0)
  if [ "$g" -lt 2000 ]; then
    say "ABORT: $f reached only $g/2000 generations -- arms are NOT matched, and an unmatched"
    say "  comparison is exactly what invalidated the truncated low-lr sweep."
    exit 1
  fi
done

# ---- the arms must actually differ from their start, and from each other -------------------
s_md5=$(md5sum cand_start.net | cut -d' ' -f1)
for f in candA_fixed.net candB_budget.net; do
  [ -s "$f" ] || { say "ABORT: $f missing or empty"; exit 1; }
  m=$(md5sum "$f" | cut -d' ' -f1)
  [ "$m" = "$s_md5" ] && { say "ABORT: $f is IDENTICAL to cand_start.net -- the arm trained nothing"; exit 1; }
done
if [ "$(md5sum candA_fixed.net | cut -d' ' -f1)" = "$(md5sum candB_budget.net | cut -d' ' -f1)" ]; then
  say "ABORT: the two arms produced byte-identical nets -- the budget flag was inert"
  exit 1
fi
say "both arms complete at 2000 generations, both moved off the start net, and they differ"

run(){ # run <netA> <netB> <seed> <label>
  local out="$D/gca_$4_$3.log"
  nice -n 19 taskset -c 6-11 "$NM" "$1" "$2" "$PAIRS" "$DEPTH" "$3" > "$out" 2>&1
  local line rate ci
  line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' "$out" | head -1)
  rate=$(echo "$line" | grep -oE '0\.[0-9]+' | head -1)
  ci=$(echo "$line" | grep -oE '0\.[0-9]+' | tail -1)
  if [ -z "$rate" ]; then
    say "  $4 seed $3: NO RATE PARSED -- match did not run (see $out)"
    return 1
  fi
  say "  $4 seed $3: $rate +/- $ci"
  echo "$rate $ci"
}

# ---- primary comparison: B vs A, three seeds -----------------------------------------------
# THREE SEEDS BECAUSE ONE IS A LOTTERY. Between-seed sd on this instrument is 0.047, which is
# larger than most effects this project has chased, so a single reading cannot resolve anything.
say "PRIMARY: candB_budget vs candA_fixed, $PAIRS pairs, depth $DEPTH"
RATES=""
for sd in 20260907 911911 424242; do
  r=$(run candB_budget.net candA_fixed.net "$sd" "BvA") || true
  [ -n "$r" ] && RATES="$RATES $(echo "$r" | awk '{print $1}')"
done

# ---- context: each arm against the shared start ---------------------------------------------
say "CONTEXT: each arm against the shared start net"
run candA_fixed.net cand_start.net 20260907 "Avstart" >/dev/null || true
run candB_budget.net cand_start.net 20260907 "Bvstart" >/dev/null || true

# ---- verdict --------------------------------------------------------------------------------
say "VERDICT"
python3 - "$RATES" <<'PY' | tee -a "$LOG"
import sys, statistics as st
v=[float(x) for x in sys.argv[1].split()]
if not v:
    print("  no parsable readings -- NO VERDICT"); raise SystemExit
m=st.mean(v)
sd=st.stdev(v) if len(v)>1 else float('nan')
print(f"  B vs A over {len(v)} seeds: {v}")
print(f"  mean {m:.4f}" + (f"   observed between-seed sd {sd:.4f}" if len(v)>1 else ""))
# The pre-registered bar, and the honest floor underneath it.
BETWEEN=0.047
if abs(m-0.5) < BETWEEN:
    print(f"  |mean - 0.5| = {abs(m-0.5):.4f} < between-seed sd {BETWEEN}")
    print("  => UNRESOLVED. Reported as a BOUND, not a refutation: over 2000 generations the node")
    print("     budget did not beat fixed depth by more than the seed noise of this instrument.")
elif m > 0.5:
    print(f"  => budget arm AHEAD by {m-0.5:.4f}, which exceeds the between-seed sd {BETWEEN}")
    print("     NOT a ship. structural_next requires rate - ci95 >= 0.5 and the channel is")
    print("     confounded until cell C runs (candidate_a_channel_FINDING.md).")
else:
    print(f"  => budget arm BEHIND by {0.5-m:.4f}, exceeding the between-seed sd {BETWEEN}")
PY
say "done"
exit 0
