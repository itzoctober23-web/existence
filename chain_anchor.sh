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
if ! ./target/release/learn --help 2>&1 | grep -q -- '--anchor-pairs' \
   && ! strings ./target/release/learn | grep -q -- '--anchor-pairs'; then
  echo "ABORT: the built binary does not contain --anchor-pairs. Both arms would be identical."
  exit 1
fi
echo "verified: --anchor-pairs is in the binary"
exec ./anchor_ab.sh
