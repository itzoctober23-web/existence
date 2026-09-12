#!/usr/bin/env bash
# REGENERATE RESULTS_INDEX.md — one line per result file, newest first.
#
# WHY THIS EXISTS. Twice on 2026-09-10 I spent real compute re-deriving something already on disk:
#
#   * launched the ARCH width search at `--arch-every 5` — the exact value `width_clock_RESULT.md`
#     records moving away from — and re-derived its 11-attempt conclusion at 95.4% of wall clock.
#     Every verdict landed inside the ranges that file already published.
#   * started collecting the candidate-quality distribution by hand (n=3, mean 0.480) before noticing
#     `reject_holdout_RESULT.md` and `accept_audit_RESULT.md` had it at far tighter intervals.
#
# Both were preventable by reading one headline. The reason I did not is that there are 40+ result
# files and no way to scan them: `ls` gives names, and a name like `gate_power_RESULT.md` does not
# tell you it concluded "the generator, not the gate, is what has no gradient". The HEADLINE does,
# and every file in this project already has one written to be read alone.
#
# So this is not documentation for its own sake. It is the index that makes the standing rule —
# check the results directory BEFORE designing an experiment — cost seconds instead of minutes, which
# is the difference between a rule that gets followed and one that does not.
#
# Ordered NEWEST FIRST deliberately: a correction is usually newer than the thing it corrects, and
# several files tonight were narrowed or retracted within the hour.
set -uo pipefail
cd "$(dirname "$0")"
OUT=RESULTS_INDEX.md

# THE FINDING SET IS WIDER THAN *_RESULT.md, and that gap cost hours twice.
# `surrogate_inverts_RESULT.md` records re-deriving a mechanism that
# `search_track_WHY_NOTHING.md` had already established -- and that file is NOT a *_RESULT.md, so
# it was invisible to this index. `fitness_spec_gap_FINDING.md`, `ceiling_ANALYSIS.md` and
# `surrogate_validation.md` are in the same position. An index that covers most of the evidence
# answers "nothing on that" with false confidence.
# `*_PREREG.md` added 2026-09-11. A pre-registration states the decision rule for a question BEFORE
# it is answered, which makes it the single most relevant file type for "read this before designing
# an experiment" -- and seven of them were invisible here. `structural_next_PREREG.md` fixes the
# order of the next three structural experiments; leaving it unindexed would repeat, on the file that
# chooses the next week's work, exactly the defect this script exists to prevent.
FINDINGS=$(ls -t *_RESULT.md *_FINDING.md *_ANALYSIS.md *_WHY_NOTHING.md *_CAVEAT.md *_PREREG.md \
                surrogate_validation.md EXPERIMENTS.md NET_TRACK_STATE.md 2>/dev/null | awk '!seen[$0]++')

{
  echo "# Results index — every \`*_RESULT.md\` headline, newest first"
  echo
  echo "Regenerate with \`./index_results.sh\`. **Read this before designing an experiment.**"
  echo "Grep it for the lever you are about to test; the headlines are written to be read alone."
  echo
  echo "> Two experiments on 2026-09-10 were launched against questions already answered here —"
  echo "> one burned 95% of its wall clock re-deriving \`width_clock_RESULT.md\` at the exact"
  echo "> setting that file abandoned. The cost of checking is one grep."
  echo
  # THE P2 STEER LIVES HERE, NOT IN RESULTS_INDEX.md. This script regenerates that file with `>`,
  # so a banner hand-written into the index is erased by the next run -- which is what happened to
  # the first version of this note on 2026-09-11. A steer that a routine command deletes is a
  # decoration. The same applies to the per-file headlines: they come from each file's H1, so a
  # caveat belongs in the H1, not in the table.
  echo "> **P2 / SEARCH TRACK — the binding constraint is the GENERATOR, not the gate.** Several of"
  echo "> the newest entries below are gate measurements, and read top-down they invite more gate"
  echo "> work. They are downstream. \`gate_power_RESULT.md\` settles the gate: the SPRT gate is built,"
  echo "> wired, and resolves in 37 games; more pairs buy more draws; a trained net would defeat the"
  echo "> track's purpose. That file records TWO occasions of cores spent re-testing SELECTION after"
  echo "> the measurement had moved the problem to GENERATION. On 2026-09-11 a third was begun and"
  echo "> stopped at the audit. Next P2 work is GRAMMAR 4 — the type checker and mutation operators."
  echo
  echo "| result | headline |"
  echo "|---|---|"
  for f in $(ls -t *_RESULT.md 2>/dev/null); do
    # The H1, stripped of its marker, with pipes escaped so a headline containing one cannot break
    # the table. Falls back to the filename rather than emitting a blank row.
    h=$(head -1 "$f" | sed 's/^#\+ *//' | sed 's/|/\\|/g')
    [ -n "$h" ] || h="(no headline — open the file)"
    printf '| [%s](%s) | %s |\n' "$f" "$f" "$h"
  done
  echo
  echo "## Findings that are not \`*_RESULT.md\`"
  echo
  echo "Same standing as the table above. \`search_track_WHY_NOTHING.md\` is the file"
  echo "\`surrogate_inverts_RESULT.md\` records re-deriving from scratch at a cost of hours --"
  echo "it was never indexed because of its name."
  echo
  echo "| file | headline |"
  echo "|---|---|"
  # THE GLOB IS THE INDEX. Anything not matched here is invisible to regeneration, so an entry
  # added to RESULTS_INDEX.md BY HAND is silently deleted the next time this script runs. That
  # happened on 2026-09-12: one run dropped three files that exist on disk and were indexed --
  # WEEK1_RETRO.md (the DAY-7 FINAL VERDICT, which this index itself describes as gating what
  # starts next), p1_kill_criterion_STATUS.md, and grammar4_addfn_unpark_blocker.md. None matched
  # a pattern below. The loss is silent and looks like a successful regeneration: the script
  # reports "N result files indexed" either way, and the dropped rows are in a DIFFERENT table
  # from the count.
  # So: never hand-edit RESULTS_INDEX.md to add a file. Add the PATTERN here instead.
  for f in $(ls -t *_FINDING.md *_ANALYSIS.md *_WHY_NOTHING.md *_CAVEAT.md *_PREREG.md \
                   *_RETRO.md *_STATUS.md *_blocker.md \
                   surrogate_validation.md EXPERIMENTS.md NET_TRACK_STATE.md 2>/dev/null | awk '!seen[$0]++'); do
    h=$(head -1 "$f" | sed 's/^#\+ *//' | sed 's/|/\\|/g')
    [ -n "$h" ] || h="(no headline — open the file)"
    printf '| [%s](%s) | %s |\n' "$f" "$f" "$h"
  done
  echo
  echo "## Topic index — which lever has already been studied"
  echo
  echo "A headline can only carry so much. On 2026-09-11 I re-derived \`proxies_RESULT.md\` (held-out"
  echo "surrogate r=-0.095 over 239 gate results) because I grepped this index for \"blend\" when"
  echo "designing that experiment, and never for \"loss\". The headline index answers *is there a file"
  echo "about X*; this one answers *has anyone measured X*, which is the question that was actually"
  echo "being asked."
  echo
  echo "| topic | files that measure it |"
  echo "|---|---|"
  for t in "learning rate:lr[ -]|learning.rate" "training loss:train_loss|training loss|held-out loss" \
           "surrogate/proxy:surrogate|proxy|proxies" "net width:width|w64|w32|arch" \
           "search depth:depth [0-9]|datagen.depth|deeper" "blend/target:blend|target" \
           "epochs:epoch" "seeds & noise:between-seed|seed lottery|seed variance" \
           "gate & thresholds:pent_rate|ci95|gate power|threshold" \
           "calibration:calibrat|confidence|residual" "speed/nps:nps|speedup|node_profile|throughput" \
           "plateau:plateau|flat|stuck"; do
    lbl=${t%%:*}; pat=${t#*:}
    # RANK BY MATCH COUNT, never alphabetically. The first version took `head -6` off a
    # `grep -l` list, which is alphabetical -- so "training loss" listed six files and omitted
    # `proxies_RESULT.md`, the one that settles it at n=239. An index that truncates away the
    # definitive file is worse than no index, because it answers "yes, covered" and hides where.
    hits=$(grep -cEi "$pat" $FINDINGS 2>/dev/null | awk -F: '$2>0{print $2"\t"$1}' \
           | sort -rn | head -5 | cut -f2 | sed 's/_RESULT\.md//;s/\.md$//' | paste -sd', ' -)
    [ -n "$hits" ] && printf '| %s | %s |\n' "$lbl" "$hits"
  done
  echo
  echo "## Non-\`_RESULT\` files worth knowing"
  echo
  echo "| file | what it is |"
  echo "|---|---|"
  for f in MASTER_PLAN.md STATE.md; do
    [ -f "docs/$f" ] && printf '| [docs/%s](docs/%s) | %s |\n' "$f" "$f" "$(head -1 "docs/$f" | sed 's/^#\+ *//' | sed 's/|/\\|/g' | cut -c1-90)"
    [ -f "$f" ] && printf '| [%s](%s) | %s |\n' "$f" "$f" "$(head -1 "$f" | sed 's/^#\+ *//' | sed 's/|/\\|/g' | cut -c1-90)"
  done
} > "$OUT"

n=$(ls *_RESULT.md 2>/dev/null | wc -l)
echo "  wrote $OUT: $n result files indexed"
# A headline that is just the filename means the file has no H1 and the index is lying about it.
bad=$(grep -c '(no headline' "$OUT" || true)
[ "${bad:-0}" -eq 0 ] || echo "  WARNING: $bad file(s) have no H1 and are indexed as unreadable"
