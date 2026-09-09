#!/usr/bin/env bash
# Read the 2x2 P2 factorial. Written BEFORE any cell produced a gate decision, so the analysis is
# fixed in advance rather than chosen after seeing which cell looks good.
#
# THE FOUR CELLS, all seed 1, all EXISTENCE_GATE_VERIFY=96:
#   control     strict filter  x strict gate
#   spec        SPEC filter    x strict gate
#   veto        strict filter  x veto gate
#   spec_veto   SPEC filter    x veto gate
#
# WHY PROMOTIONS ARE NOT THE ENDPOINT. A promotion is what the cell's own rule decided; it is not
# evidence the promoted program is better. The independent 96-pair VERIFY observer is the only
# instrument here that measures true strength, so every promotion is read against it. A promotion the
# observer scores below 0.500 is a FALSE POSITIVE and counts against its cell, not for it.
#
# WHY W-D-L IS REPORTED. 95 of 203 historical decisions carried ci95 == 1.5/6 exactly -- gate.rs's
# zero-variance fallback -- with pent_rate exactly 0.500. That has two causes needing OPPOSITE fixes
# and the rate cannot separate them: MIRRORED (draws == 0) means the candidate plays like the champion
# and NO pair count can ever help; ALL-DRAWN (wins == losses == 0) means the match is not decisive and
# sample size was never the issue. Three of the four cells log it.
set -uo pipefail
cd "$(dirname "$0")"
printf "%-11s %-5s %-6s %-6s %-11s %-9s %-8s %s\n" CELL GENS PROMO NO-OP GATE-CALLS NO-SIGNAL VERIFY-N VERIFY-MEAN
for cell in s1-control:gate_control_arm.log s1-veto:gate_veto_arm.log s2-control:gate_control_s2.log s2-veto:gate_veto_s2.log; do
  name=${cell%%:*}; f=${cell#*:}
  [ -f "$f" ] || { printf "%-12s (no log yet)\n" "$name"; continue; }
  # DISTINCT generations, not log lines. Each generation emits ONE line PER LINEAGE (MAIN + MCTS),
  # so counting lines reports double -- I read "gens 4" for an arm whose last line was gen 2.
  gens=$(grep -oE '^ *gen +[0-9]+' "$f" | awk '{print $2}' | sort -un | wc -l)
  # PROMO SPLIT. PATH 1 accepts on `same_play` ALONE. Its own comment defines the path as identical
  # play "AND COSTS LESS", but the cost half was guaranteed by the caller: the strict filter picks
  # only when rate > best_rate. SPEC_FILTER picks on r >= 0.9*best_rate, so under SPEC a candidate
  # that is WORSE on the surrogate can reach PATH 1 and be recorded as a "speedup".
  # MEASURED, not inferred: the control arm's gen-1 candidate is `rates 0.999x` -- below the
  # incumbent -- and the SPEC cells promoted at gen 1 on `0.002490 was 0.002490`.
  # So a real promotion must show a rate STRICTLY BETTER than the "was" value. Counted here rather
  # than patched into evolve.rs, because four arms are mid-run and the analysis can separate them.
  promo=$(grep -cE 'ACCEPT' "$f")
  noop=$(awk '/ACCEPT speedup/{ if (match($0,/([0-9.]+) was ([0-9.]+)/,m) && m[1]+0 <= m[2]+0) n++ } END{print n+0}' "$f")
  real=$(( promo - noop ))
  calls=$(grep -cE 'gate (REJECT|ACCEPT) ' "$f")
  # no-signal: ci95 exactly 1.5/6 = 0.250 at the 6-pair default
  nosig=$(grep -oE 'gate (REJECT|ACCEPT) [0-9.]+\+/-0\.250' "$f" | wc -l)
  vn=$(grep -cE 'VERIFY' "$f")
  vmean=$(grep -oE 'VERIFY [0-9.]+' "$f" | awk '{s+=$2;n++} END{if(n)printf "%.3f",s/n; else printf "-"}')
  printf "%-11s %-5s %-6s %-6s %-11s %-9s %-8s %s\n" "$name" "$gens" "$real" "$noop" "$calls" "$nosig" "$vn" "$vmean"
done

echo
echo "  DESIGN: rule (strict vs veto) x SEED (1, 2). The filter arm was dropped -- SPEC_FILTER cannot"
echo "  produce gate data until PATH 1 re-checks cost, measured over 4 generations of pure no-ops."
echo "  Two seeds because one cannot settle a champion-vs-champion question. The blend campaign needed"
echo "  FIVE: at two seeds its interval was [+0.011, +0.126], clearing zero by 0.011 -- at the boundary,"
echo "  not past it. All five seeds did agree in SIGN (0.460 0.403 0.403 0.394 0.440, every one below"
echo "  0.500); it was the INTERVAL that needed the seeds, not a disagreement between them."
echo
echo "  MIRRORED vs ALL-DRAWN among no-signal decisions (all cells below log W-D-L):"
for cell in s1-veto:gate_veto_arm.log s2-control:gate_control_s2.log s2-veto:gate_veto_s2.log; do
  name=${cell%%:*}; f=${cell#*:}
  [ -f "$f" ] || continue
  mir=$(grep -oE 'W-D-L ([0-9]+)-0-([0-9]+)' "$f" | wc -l)
  drw=$(grep -oE 'W-D-L 0-([0-9]+)-0' "$f" | wc -l)
  printf "    %-12s mirrored(draws=0) %-4s all-drawn(w=l=0) %s\n" "$name" "$mir" "$drw"
done
echo
echo "  Any promotion whose VERIFY is below 0.500 is a FALSE POSITIVE. Check them individually:"
grep -HnE 'ACCEPT|VERIFY' gate_control_arm.log spec_filter_arm.log gate_veto_arm.log gate_spec_veto_arm.log 2>/dev/null \
  | grep -B1 -A1 'ACCEPT' | tail -12 | sed 's/^/    /' || true
