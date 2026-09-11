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
