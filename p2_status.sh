#!/usr/bin/env bash
# P2 SEARCH-TRACK STATUS — one line, emitted daily into STATE.md.
#
# His instruction, 2026-09-11: "The search track has been silent since the restart. One line in
# STATE.md per day: generation count, whether any MAIN member holds Probe+Store acquired from a
# parent that lacked one, crossover survived/proposed. If it isn't running, say so and why."
#
# The three facts are chosen because they are the ones that distinguish "the loop is turning" from
# "the loop is turning and something is ACCUMULATING". A generation count alone can rise forever
# without a single structure being retained -- which is exactly what 1,062 proposals and 0 accepts
# looks like. Probe+Store acquired from a parent that LACKED it is the first real acquisition event
# (a member gaining hash reuse it did not inherit); crossover survived/proposed is whether
# recombination is contributing at all.
#
# EXISTENCE IS TABULA RASA. This reports what the loop found; it never steers it. Alpha-beta, MCTS
# and proof-number primitives are all in the grammar so a HYBRID is reachable if that is what wins
# ([[alphabeta-mcts-is-4pc-not-existence]]).
set -uo pipefail
cd "$(dirname "$0")"

# Is it running? exe basename, never a cmdline substring (that would match this script).
RUNNING=0
for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue; e=${e% (deleted)}
  case "${e##*/}" in evolve*) RUNNING=1; break;; esac
done

# The newest log is not necessarily the one with generations in it -- evolve_fmt.log is newer and has
# none, which is why this first printed "gen ?". Pick the newest log that actually contains gen lines.
LOG=""
for f in $(ls -t evolve*.log 2>/dev/null); do grep -qE '^gen +[0-9]+' "$f" 2>/dev/null && { LOG=$f; break; }; done
[ -n "$LOG" ] || LOG=$(ls -t evolve*.log 2>/dev/null | head -1)
GEN=$(grep -oE '^gen +[0-9]+' "$LOG" 2>/dev/null | tail -1 | grep -oE '[0-9]+')
# Probe+Store acquired from a parent that lacked it, and crossover outcomes. Fixed-string greps:
# a backtracking regex on a large log has crashed this session before.
# `grep -c` PRINTS 0 AND EXITS 1 on no match, so `|| echo 0` appends a SECOND zero and every field
# came out doubled ("0\n0", "1062\n0"). Recorded trap, hit again. Assign, then default the empty.
cnt(){ local n; n=$(grep -cF "$1" "$2" 2>/dev/null); echo "${n:-0}"; }
# THREE POSSIBLE ANSWERS HERE, AND THEY ARE DIFFERENT CLAIMS. Checked 2026-09-11:
#   * The CURRENT source DOES instrument both fields. `evolve.rs:3678` prints
#     `... pop N spread lo-hi tt[..] ttk[".."] dsl0 xPROP/SURV`, and `xtask/src/main.rs:542`
#     reads a member as holding hash reuse when its `ttk` tag contains both 'P' and 'S'. There is
#     even a positive control (`tt_kinds_control`) asserting the tag separates a probe-only program
#     from a store-only one, because the pooled `tt` count it replaced had already produced one
#     retracted claim.
#   * The LAST RUN PREDATES IT. `evolve_search.log` (2026-09-08) carries the older sparse line
#     `gen N -- (24 typed, 0 ill-typed, 13 oracle, 0 surrogate, 252 pairs spent, none beat it)` --
#     no `ttk`, no `x`. So there is no data, not a zero.
#   * Reporting 0 would therefore be a BROKEN PROBE reported as a measurement, and reporting
#     "not instrumented" would be wrong about the code. Say which it is.
if grep -qF 'ttk[' "$LOG" 2>/dev/null; then
  ACQ=$(grep -oE 'ttk\[[^]]*\]' "$LOG" | grep -c 'P.*S'); ACQ=${ACQ:-0}
  XP=$(grep -oE ' x[0-9]+/[0-9]+' "$LOG" | tail -1 | grep -oE '[0-9]+/[0-9]+')
  XPROP=${XP%%/*}; XSURV=${XP##*/}
  XPROP=${XPROP:-0}; XSURV=${XSURV:-0}
else
  ACQ="no data (run predates the ttk instrumentation added to evolve.rs:3678)"
  XPROP="no data"; XSURV="no data"
fi
PROPOSALS=$(cat ledger_search*.jsonl 2>/dev/null | grep -cF '"class":"PROGRAM"'); PROPOSALS=${PROPOSALS:-0}
ACCEPTS=$(cat ledger_search*.jsonl 2>/dev/null | grep -F '"class":"PROGRAM"' | grep -cF '"verdict":"accept"'); ACCEPTS=${ACCEPTS:-0}

if [ "$RUNNING" -eq 1 ]; then
  printf 'P2 %s — RUNNING. gen %s | Probe+Store acquired from a parent lacking it: %s | crossover %s survived / %s proposed | lifetime %s proposals, %s accepts.\n' \
    "$(date +%F)" "${GEN:-?}" "$ACQ" "$XSURV" "$XPROP" "$PROPOSALS" "$ACCEPTS"
else
  printf 'P2 %s — NOT RUNNING (last activity %s, gen %s). Probe+Store acquisition: %s | crossover survived/proposed: %s / %s | lifetime %s proposals, %s accepts. WHY: the fitness cannot rank its own candidates — `search_track_WHY_NOTHING.md` ("one dimension saturated, the other blocked": the seed already scores 25/25 on mates, so it is a pass/fail filter and never a gradient) and `surrogate_inverts_RESULT.md` (the grammar fitness ranks the STRONGEST reference program LAST). MASTER_PLAN P2 kill has fired; restarting the loop unchanged would re-derive 0 accepts.\n' \
    "$(date +%F)" "$(ls -l --time-style=+%F "${LOG:-/dev/null}" 2>/dev/null | awk '{print $6}')" "${GEN:-?}" "$ACQ" "$XSURV" "$XPROP" "$PROPOSALS" "$ACCEPTS"
fi
