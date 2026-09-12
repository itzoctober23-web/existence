#!/usr/bin/env bash
# SNAPSHOT THE LIVE TRAINER'S NET, so the current run can be measured within itself.
#
# WHY. On 2026-09-12 the P1 kill's second conjunct needed a within-run checkpoint series and
# `prodk1926` had exactly TWO nets on disk -- its start and its live file. The measurement had to fall
# back to the `r9` window from 2026-09-10, and `p1_kill_conjunct2_RESULT.md` records that as its main
# unclosed limit. Nothing was saving intermediate nets; this does.
#
# THE TRAINER REWRITES ITS --out FILE EVERY GENERATION -- measured: md5 changed inside a 3s window.
# So a reader can catch a torn write. `auto_promote.sh` already solves this by copying first and using
# the COPY; this does the same and then VERIFIES the copy before keeping it.
#
# Naming follows the convention already on disk (48 nets use it): <tag>.gen<N>.net
set -uo pipefail
cd /home/maswabe/existence || exit 1
DIR=ckpt
mkdir -p "$DIR"

# --- find the live trainer by exe + exact --out, never by cmdline pattern (that matches this script).
TAG=""; OUT=""
for p in $(ls /proc | grep -E '^[0-9]+$'); do
  e=$(readlink /proc/$p/exe 2>/dev/null); e=${e% (deleted)}
  case "${e##*/}" in
    learn)
      OUT=$(tr '\0' ' ' < /proc/$p/cmdline 2>/dev/null | grep -oE '[-][-]out [a-zA-Z0-9_]+\.net' | awk '{print $2}')
      TAG="${OUT%.net}"; break;;
  esac
done
[ -n "$OUT" ] || { echo "$(date +%F_%H:%M) no live trainer -- nothing to snapshot" >> "$DIR/snapshot.log"; exit 0; }
[ -s "$OUT" ] || { echo "$(date +%F_%H:%M) $OUT missing/empty" >> "$DIR/snapshot.log"; exit 1; }

# --- generation from the trainer's OWN log, not from a guess.
G=$(grep -oE '^gen +[0-9]+' "$TAG.log" 2>/dev/null | tail -1 | awk '{print $2}')
[ -n "$G" ] || { echo "$(date +%F_%H:%M) no gen line in $TAG.log -- refusing to name a snapshot by guess" >> "$DIR/snapshot.log"; exit 1; }

DEST="$DIR/$TAG.gen$G.net"
[ -f "$DEST" ] && exit 0          # same generation as last time: the trainer has not advanced

cp -f "$OUT" "$DEST.tmp" || { echo "$(date +%F_%H:%M) copy failed" >> "$DIR/snapshot.log"; exit 1; }

# --- VERIFY the copy before keeping it. A torn read is the failure mode this exists to avoid, and a
# half-written net that silently becomes a measurement point is worse than no point at all.
ref_size=$(stat -c %s "$OUT")
got_size=$(stat -c %s "$DEST.tmp")
if [ "$got_size" != "$ref_size" ]; then
  rm -f "$DEST.tmp"
  echo "$(date +%F_%H:%M) REJECTED gen$G: size $got_size != $ref_size (torn write)" >> "$DIR/snapshot.log"
  exit 1
fi
mv -f "$DEST.tmp" "$DEST"
n=$(ls "$DIR"/*.net 2>/dev/null | wc -l)
echo "$(date +%F_%H:%M) kept $DEST ($got_size bytes); series now $n nets" >> "$DIR/snapshot.log"

# --- keep the series bounded: 200 nets x 50KB = 10MB, and the box has a HARD 15GB limit.
ls -t "$DIR"/*.net 2>/dev/null | tail -n +201 | xargs -r rm -f
