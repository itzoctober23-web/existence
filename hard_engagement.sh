#!/usr/bin/env bash
# WHEN can EXISTENCE_HARD_FITNESS engage? Counts hard>0 by generation across every arm log.
#
# WHY THIS EXISTS. The treatment arms read `hard 0-0` at gens 1-2 and I nearly read that as "the fix
# never engages". It is not evidence: NO arm has ever scored on the hard set at gen 1, and only one
# of 38 did at gen 2. The dimension switches on at gen 3 and peaks near 50% at gens 4-6.
#
# CAVEAT, and it runs conservative. Before 2026-09-09 only NON-GATED generations printed the `hard`
# field, so gated generations -- the ones where a candidate actually won -- are missing here. Those
# are exactly the generations where a candidate scored highly, which under HARD_FITNESS correlates
# with hf>0. These counts therefore UNDERSTATE engagement where it matters. The gate line now carries
# `hard {hlo}-{hhi}`, which closes the hole for future runs.
cd "$(dirname "$0")"
for f in *.log; do
  grep -hoE 'gen +[0-9]+ +[A-Z]+ .*hard [0-9]+-[0-9]+' "$f" 2>/dev/null
done | sed -E 's/gen +([0-9]+) +([A-Z]+).*hard ([0-9]+)-([0-9]+)/\1 \2 \4/' \
 | awk '{tot[$1]++; if($3>0) hit[$1]++}
        END{printf "  gen  gens_seen  with_hard>0   rate\n";
            for(g=1;g<=15;g++) if(tot[g]) printf "  %3d  %9d  %11d  %5.1f%%\n", g, tot[g], hit[g]+0, 100*(hit[g]+0)/tot[g]}'
