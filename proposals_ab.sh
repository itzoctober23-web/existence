#!/usr/bin/env bash
# DOES RAISING PROPOSALS-PER-GENERATION GIVE SELECTION A CHOICE?
#
# `search_has_no_choice_RESULT.md`: across 79 generations and 580 scored candidates, 61 pass the
# mate guard (10.5%) and only 5 of 79 generations ever see MORE THAN ONE distinct fitness.
# Selection cannot select from a set of size <= 1, which is why P2 has 1,062 proposals and 0
# accepts. Its closing line names the lever -- "raise the number of guard-passing,
# distinctly-scoring candidates per generation ... the first suspect that none of the four running
# arms varies" -- and nothing has varied it since.
#
# The mechanism was one line: candidates were proposed as `(0..pop)`, so proposal count WAS
# population size, and pop collapses to 2. `EXISTENCE_PROPOSALS` now separates them.
#
# THE MEASUREMENT, which is deliberately NOT "did it accept something":
#   primary   distinct fitness values per generation, and generations with >1
#   secondary mate-ok count per generation
# An accept is downstream of several more stages; what is being tested here is whether selection
# is offered a choice at all. Claiming more than that would repeat the mistake this project has
# made with proxies four times.
#
# PAIRED: same EXISTENCE_EVOLVE_SEED, same everything but EXISTENCE_PROPOSALS, so the control arm
# is byte-identical to the shipped behaviour (unset => n_prop = pop).
#
# BUILDS A SNAPSHOT FIRST. The live trainers run from their own scratchpad snapshots, so building
# here cannot disturb them -- but the binary this runs must not change under it either, which is
# why it is copied out of the target dir before use.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
SNAP=$SCR/prop_ab
GENS=${GENS:-6}
# SET SIZE IS THE COST KNOB. evolve.rs:2576 records that pop 24 with 80 MATE-1 + 40 MATE-2 took
# 330 SECONDS PER GENERATION -- cost is pop x (n1+n2) evaluations, and EXISTENCE_PROPOSALS
# multiplies the pop side directly, so a 32-proposal arm at default sizes would be ~16x the
# control. Small sets keep both arms bounded; the question here is the SHAPE of the funnel
# (distinct rates per generation), which does not need a large set to show itself.
# ARG ORDER IS gens, POP, n1, n2 -- arg 2 is the POPULATION, not the first set size. Passing
# three args would have set pop=10 and n1=4 and left n2 at its default of 20, i.e. a different
# experiment than the one described. (Third positional/index slip caught by checking today.)
# POP is pinned for BOTH arms so the only difference is the proposal count.
POP=${POP:-4}
N1=${N1:-10}
N2=${N2:-4}
SEED=${SEED:-1}
LOG=proposals_ab.log
say(){ echo "$(date +%F_%H:%M) [propab] $*" | tee -a "$LOG"; }

say "building evolve (nice 19, does not touch the running snapshots)"
mkdir -p "$SNAP"
nice -n 19 ionice -c 3 taskset -c 6-11 cargo build --release --example evolve --quiet 2>&1 | tail -3 | tee -a "$LOG"
BIN=target/release/examples/evolve
[ -x "$BIN" ] || { say "ABORT: build produced no binary"; exit 1; }
cp -f "$BIN" "$SNAP/evolve"
say "snapshot $(md5sum "$SNAP/evolve" | cut -c1-12)"

run_arm(){ # $1=label $2=proposals(empty=default)
  # SPLIT, and not for style: in `local a=$1 b="x${a}"` bash expands EVERY right-hand side
  # BEFORE performing any of the assignments, so ${a} is still unbound at expansion time and
  # `set -u` aborts. That is what killed the first run at line 57.
  local lab=$1 prop=$2
  local out="prop_${lab}.log"
  say "arm $lab: EXISTENCE_PROPOSALS=${prop:-<unset, = pop>}  $GENS gens, seed $SEED"
  if [ -n "$prop" ]; then
    EXISTENCE_EVOLVE_SEED=$SEED EXISTENCE_PROPOSALS="$prop" \
      nice -n 19 taskset -c 6-11 timeout 1800 "$SNAP/evolve" "$GENS" "$POP" "$N1" "$N2" > "$out" 2>&1 || true
  else
    EXISTENCE_EVOLVE_SEED=$SEED \
      nice -n 19 taskset -c 6-11 timeout 1800 "$SNAP/evolve" "$GENS" "$POP" "$N1" "$N2" > "$out" 2>&1 || true
  fi
  say "  $(grep -cE '^gen ' "$out" 2>/dev/null) gen lines written"
}

run_arm control ""
run_arm prop32 32

say "RESULT — does selection get a choice?"
python3 - <<'PY' | tee -a "$LOG"
import re, glob
def summarise(f, lab):
    try: txt = open(f).read()
    except OSError:
        print(f"  {lab:<10} no log"); return
    gens = [l for l in txt.split('\n') if l.startswith('gen ')]
    if not gens:
        print(f"  {lab:<10} no gen lines -- the arm did not run; do not read anything into this")
        return
    multi = 0; tot = 0; oks = []
    for l in gens:
        tot += 1
        m = re.search(r'mate-ok\s+(\d+)', l)
        if m: oks.append(int(m.group(1)))
        rates = re.findall(r'(\d+\.\d{4,})x', l)
        if len(set(rates)) > 1: multi += 1
    mo = sum(oks)/len(oks) if oks else 0.0
    print(f"  {lab:<10} gens {tot:<4} mean mate-ok/gen {mo:6.2f}   gens with >1 distinct rate: {multi}/{tot}")
for f, lab in (("prop_control.log", "control"), ("prop_prop32.log", "prop32")):
    summarise(f, lab)
print()
print("  Baseline from search_has_no_choice_RESULT.md: 5 of 79 generations (6%) had >1 distinct rate.")
print("  PRIMARY question: does prop32 raise mate-ok per generation and the >1-distinct-rate share?")
print("  NOT claimed either way: that this produces an ACCEPT. An accept is several stages")
print("  downstream; this measures only whether selection is offered a choice at all.")
PY
say "PROPABDONE"
