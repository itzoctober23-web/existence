#!/usr/bin/env bash
# THIRD training seed for blend 0.85 — the seed the power calculation asks for.
#
# WHY A THIRD. Two seeds give mean effect 0.0685 for 0.85 over 0.75 with 95% CI [+0.011, +0.126].
# That excludes zero by 0.011, and the power calculation against the pooled between-seed sd (0.0418)
# says ~2.9 seeds. We have two. This is AT the boundary, not past it, and this project has withdrawn
# two candidates today that looked exactly this good with one fewer seed.
#
# WHY A NEW SCRIPT RATHER THAN `SEED=987654 ./blend_085_seed2.sh`. That was tried and is a SILENT
# NO-OP: `blend_085_seed2.sh` hardcodes the output names `s2_075.net` / `s2_085.net` and skips
# training when they exist, so a different SEED re-matches the SAME seed-424242 nets and reports the
# result as a third seed. Caught on the first line of its output ("arms: s2_075 20 generations")
# before any number was recorded. Filenames here carry the seed.
#
# BOTH ARMS MUST BE TRAINED. There is no 0.75 arm at seed 987654 either -- `bn_075` is seed 20260907
# and `s2_075` is 424242 -- so this trains the pair, which is the only way the comparison is
# within-seed and therefore free of the changing-origin problem.
#
# PRE-REGISTERED: 0.85 winning a third time makes this the best-evidenced open candidate in the tree
# and justifies a real multi-seed campaign. A loss or a tie drops the cross-seed mean below the
# resolution boundary and the candidate goes back to unproven -- which is the outcome the boundary
# exists to allow.
set -uo pipefail
cd "$(dirname "$0")"
CORE=${CORE:-13}
SEED=${SEED:-987654}
GENS=${GENS:-20}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt_nm/release/examples/netmatch}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ] || { echo "no netmatch at $NM"; exit 1; }

for B in 0.75 0.85; do
  tag="s3_${B/./}"
  if [ -f "$tag.net" ]; then echo "=== $tag exists, skipping ==="; continue; fi
  echo "=== training blend $B at seed $SEED, $GENS generations -> $tag.net ==="
  timeout 4200 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 --blend "$B" \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "$tag.net" --ledger "$tag.jsonl" > "$tag.log" 2>&1
  echo "  $(grep -cE '^gen +[0-9]+ ' "$tag.log") generations"
done

a=$(grep -cE '^gen +[0-9]+ ' s3_075.log 2>/dev/null); b=$(grep -cE '^gen +[0-9]+ ' s3_085.log 2>/dev/null)
echo "  arms: s3_075 $a generations, s3_085 $b generations"
[ "$a" = "$b" ] || echo "  *** UNEQUAL ARMS -- the match below is confounded with training amount ***"

echo "=== seed $SEED: blend 0.75 vs 0.85, DIRECT, depth 4 ==="
timeout 5400 taskset -c "$CORE" nice -n 19 ionice -c 3 "$NM" s3_075.net s3_085.net 448 4 777 \
  2>&1 | grep -E 'scores|=>|arms:|effect|ONE SEED|defensible' | sed 's/^/  /'
echo "  s3_075's score is shown; below 0.5 means blend 0.85 is stronger, a THIRD time."
