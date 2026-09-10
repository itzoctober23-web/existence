#!/bin/bash
# GUARD TOLERANCE 0 — the direct test of the repair the evidence now points at.
#
# WHY THIS, NOW. At 20:50 the `mates {f}` field refuted the saturation premise on its first line:
#
#   gen 3 MAIN gate REJECT llr -3.18 (36 games W-D-L 2-28-6)  mates 19  hard 0-0
#
# 19, not the seed's 23. MAIN's guard floor is 19 (`23/23 mates (floor 19)`), so the numerator was
# never pinned. The winner SOLD 4 of 23 forced mates for a 29.7% cost cut and the surrogate paid it
# +17.4% for the trade; VERIFY read 0.422, resolved worse. mates/Mcost is not a saturated ratio, it
# is an EXCHANGE RATE, and `guard_tolerance` sets the price.
#
# The wider-licence arm is the same curve, not a separate condition: tolerance 7 / floor 16 reached
# +85.5% surrogate and VERIFY 0.258.
#
# WHAT TOLERANCE 0 DOES. `guard_floor = f.saturating_sub(guard_tolerance)`, so 0 gives floor 23 — no
# mate may be dropped at all. That is what evolve.rs:45 says the rule already is: "keep every mate,
# get cheaper". A floor of 19 does not keep every mate. With the market closed, the only remaining
# way to raise the surrogate is a genuine speedup at unchanged play.
#
# WHAT IT REPLACES, and why that is the right trade. Core 12 held HARD_FITNESS weight 1. My own
# arithmetic predicted that arm inert (hard-set ceiling +8.7% against cost cuts of +13..+31%), and it
# addresses a mechanism that has now been refuted — weighting the numerator raises what mates are
# WORTH, but if mates can be SOLD that raises the price without closing the market. Weight 4 stays on
# core 14 as the dose-response, so the HARD_FITNESS question is not abandoned, only de-duplicated.
#
# PRE-REGISTERED READING, before any gate:
#   * Candidates still resolved WORSE with mates 23  -> tolerance was NOT the mechanism; a
#     same-mates cost cut is itself harmful, and the surrogate's cost term is wrong at the root.
#   * Candidates no longer resolved worse            -> consistent with the exchange-rate account.
#   * FEWER gates reached                            -> EXPECTED and not a failure. Floor 23 is a
#     stricter filter, so fewer candidates qualify. Judge by VERIFY on the ones that do, never by
#     gate count.
#   * `mates` on every gate line MUST read 23. If it reads less, the floor is not doing what this
#     script assumes and the run should be stopped rather than interpreted.
#
# ⚠ SINGLE-ARM SCRIPT, and it does NOT stop whatever currently holds that core. Check `ps` first.
# It also runs xt_r, which predates the SPEC_FILTER header field — harmless, because a log with
# no such field is read as SPEC_FILTER off, which is correct for this arm.
set -u
BIN=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt_r/release/examples/evolve
cd /home/maswabe/existence || exit 1
[ -x "$BIN" ] || { echo "missing binary: $BIN" >&2; exit 1; }
[ -f gate_gtol0_s1.log ] && mv -f gate_gtol0_s1.log gate_gtol0_s1.log.prev

EXISTENCE_GUARD_TOL=0 \
EXISTENCE_EVOLVE_SEED=1 \
EXISTENCE_GATE_SPRT=1 \
EXISTENCE_GATE_ELO0=0 \
EXISTENCE_GATE_ELO1=30 \
EXISTENCE_GATE_MAXPAIRS=400 \
EXISTENCE_GATE_VERIFY=96 \
  taskset -c 12 nice -n 19 "$BIN" 25 8 12 6 3 > gate_gtol0_s1.log 2>&1 &
echo "  core 12 -> gate_gtol0_s1.log  (guard tolerance 0, floor 23)"
