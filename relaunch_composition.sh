#!/bin/bash
# SET COMPOSITION — the one surviving recommendation after tonight's corrections.
#
# WHY THIS AND NOT THE MATE LADDER. `fitness_set_composition_RESULT.md` measured that the shipped
# set's mate-in-1 majority lets a NULL SEARCH (0.0366% of the seed's cost) score 2934.933x while
# keeping every mate. The obvious fix — add MATE-2/3 strata — is REFUTED, and by this repo's own
# source: `forced_mate_set` (evolve.rs:200) already builds mate-in-2 positions, and
# `disagreement_set`'s comment records that at fitness depth 3 that set "has no teeth", being solved
# 40/40 AT DEPTH 2 because the forcing move is also the eval-best move. A mate-in-N does not imply N
# plies of search. Disagreement does, BY CONSTRUCTION: it selects positions where the SEED answers
# differently at D-1 and D, so it follows the fitness depth automatically.
#
# So the surviving, non-refuted change is the RATIO, not a new stratum:
#
#     shipped   n1=12 mate-in-1, n2=6 disagreement, n3=5 window  = 23 positions, 52% mate-in-1
#     this arm  n1= 4 mate-in-1, n2=10 disagreement, n3=10 window = 24 positions, 17% mate-in-1
#
# SIZE IS HELD CONSTANT (23 -> 24) so this isolates COMPOSITION. That matters because every prior
# investigation of this set — `fitness_spec_gap_FINDING.md`, `fitness_saturation_RESULT.md`, and my
# own refuted `relaunch_bigset.sh` — varied SIZE. Checked before launching: no arm in this repo has
# ever varied the composition ratio.
#
# NO CODE CHANGE. n1/n2/n3 are positional args 3/4/5 (`evolve <gens> <lambda> <n1> <n2> <n3> <depth>`).
#
# WHAT IT REPLACES. Core 14 held lambda 32, testing whether candidate SUPPLY was the constraint.
# Measured tonight that it is not: `ttgraft` collected 40 children carrying BOTH halves of the TT rung
# in 499 crossover attempts, so the supply of the one known rung is roughly 5.6% of one candidate in
# four. That arm was 48 minutes into generation 1 with 0 accepts, testing a refuted hypothesis.
#
# BINARY: xt_comp, staged in scratchpad so a later `cargo build` cannot unlink it mid-run (a rebuild
# under a running probe tonight left its /proc/PID/exe reading "(deleted)", which silently broke a
# kill pattern anchored on the plain path). Behaviourally identical to the other arms on the default
# path — verified: the seed reproduces 24 mates / cost 11534432615, and `evolve ttk` passes — plus it
# prints the new `ttk` per-kind TT composition field, which the older arms cannot.
#
# PRE-REGISTERED READING:
#   * FEWER accepts than the control is EXPECTED and is NOT failure. A set that is harder to satisfy
#     admits less; the question is whether what it admits is better, which only VERIFY answers.
#   * The seed's own mate count will NOT be 23 — a different set means a different denominator.
#     Compare FRACTIONS across arms, never raw counts.
#   * If a candidate is accepted here whose `ttk` shows both P and S, read it against the
#     pre-registration in `ladder_valley_RESULT.md`: ~1.15x, 2-3 mates sold, VERIFY below 0.5.
#   * If this arm also reaches 0 accepts by gen 6, then composition is not the constraint either, and
#     the remaining suspect is the CORRECTNESS ORACLE under graft (0 of 40 kept all 25 mates).
set -u
BIN=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt_comp/release/examples/evolve
cd /home/maswabe/existence || exit 1
[ -x "$BIN" ] || { echo "missing binary: $BIN" >&2; exit 1; }
[ -f gate_composition_s1.log ] && mv -f gate_composition_s1.log gate_composition_s1.log.prev

EXISTENCE_EVOLVE_SEED=1 \
EXISTENCE_GATE_SPRT=1 \
EXISTENCE_GATE_ELO0=0 \
EXISTENCE_GATE_ELO1=30 \
EXISTENCE_GATE_MAXPAIRS=400 \
EXISTENCE_GATE_VERIFY=96 \
  taskset -c 14 nice -n 19 "$BIN" 25 8 4 10 10 3 > gate_composition_s1.log 2>&1 &
echo "  core 14 -> gate_composition_s1.log  (n1=4 n2=10 n3=10: 17% mate-in-1 vs the shipped 52%)"
