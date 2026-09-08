#!/usr/bin/env bash
# Runs after the epochs-2 replication, and REBUILDS FIRST.
#
# The anchor gate is source-only right now: ./target/release/learn predates --anchor-pairs, and an
# unknown flag is silently ignored by the arg() helper. Running the A/B against that binary would
# execute TWO IDENTICAL ARMS and report a clean null -- the most expensive kind of wrong answer,
# because it looks like a result. So: wait for the replication, confirm nothing is mid-run, build,
# and verify the flag is actually present before handing over.
#
# Waits on chain_replicate.log because chain_replicate.sh EXECS replicate_ep2.sh, so that is where
# its output lands. The previous chain watched a file nobody wrote and would have silently skipped
# its experiment; same shape, checked this time.
set -uo pipefail
cd "$(dirname "$0")"
for _ in $(seq 1 2000); do
  grep -q "=== done ===" chain_replicate.log 2>/dev/null && break
  sleep 5
done
grep -q "=== done ===" chain_replicate.log 2>/dev/null || { echo "replication never finished; not chaining"; exit 1; }
echo "replication finished — checking the box is clear before rebuilding"

# Never rebuild under a running job: that has already cost this project a gate at 323 pairs.
for _ in $(seq 1 120); do
  busy=0
  for p in /proc/[0-9]*; do
    e=$(readlink "$p/exe" 2>/dev/null) || continue
    case "$(basename "${e% (deleted)}")" in learn|control) busy=1;; esac
  done
  [ "$busy" -eq 0 ] && break
  sleep 10
done
echo "box clear — building learn with the anchor gate"
cargo build --release -p pipeline --bin learn 2>&1 | tail -2
# VERIFY BY BEHAVIOUR, NOT BY BYTES. The first version grepped the binary for the string and
# aborted this experiment on a FALSE NEGATIVE: rustc does not store arg()-only literals as
# contiguous greppable text, so --horizon-cap also greps 0 while demonstrably working, and
# --gate-pairs greps 1 only because it appears in a println format string. It also called
# `learn --help`, which learn does not implement -- so that "check" silently STARTED A TRAINING
# RUN with default settings.
#
# learn now prints anchor-pairs in its settings line, so the flag can be confirmed from the
# program's own output on a 1-generation run.
probe=$(timeout 120 ./target/release/learn --gens 1 --games 4 --anchor-pairs 224 \
        --arch-every 0 --control-every 0 2>&1 | head -1)
case "$probe" in
  *anchor-pairs=224*) echo "verified from its own output: $probe" ;;
  *) echo "ABORT: binary did not report anchor-pairs=224. Both arms would be identical."
     echo "  got: $probe"; exit 1 ;;
esac
exec ./anchor_ab.sh
