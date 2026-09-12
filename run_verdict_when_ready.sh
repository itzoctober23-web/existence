#!/usr/bin/env bash
# Fire the PRE-REGISTERED verdict the moment the compound arm lands, and not before.
# The harness itself refuses an arm short of 2000 generations, so this only removes the waiting.
set -uo pipefail
cd /home/maswabe/existence
for i in $(seq 1 240); do
  grep -qF 'ARM p1c_compound END' p1_compounding.log && break
  sleep 15
done
echo "=== arm state ==="; tail -3 p1_compounding.log
./p1_compounding_verdict.sh
