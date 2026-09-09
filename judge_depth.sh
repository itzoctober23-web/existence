#!/usr/bin/env bash
# IS THE VERDICT A PROPERTY OF THE NETS, OR OF THE DEPTH WE JUDGE THEM AT?
#
# THE PROBLEM, straight out of this tree's own files. netmatch.rs:23-29 says the project judges
# strength at depth 4 and that defaulting to depth 2 meant "every comparison I ran -- the blend
# reversal, the b2_5 gain, the capacity null, draws, epochs -- answered 'which net is better at
# depth 2' while I read them as 'which net is stronger'." Yet depth_replicate.sh:102 and
# depth_parity.sh both invoke it with an explicit `448 2`, i.e. the depth that warning is about.
#
# And STATE.md:326-350 measures that crossing search parity moves the distill gap 2.6x, while two
# extra plies WITHIN a parity class move it 2.5%. So depth 2 and depth 4 are the same parity class
# and depth 3 and 5 are the other one. If a verdict is a fact about the nets it should survive the
# crossing. If it flips, the verdict was a fact about the RULER.
#
# This does not re-litigate parity -- STATE.md establishes it on the target distribution via
# distill_gap. It asks the separate, practical question that determines whether past verdicts stand:
# does the MATCH protocol's parity change WHO WINS, not just by how much.
#
# WHY IT IS SAFE TO RUN BESIDE A TIME-BOXED ARM. netmatch plays a fixed number of pairs at a fixed
# depth from seeded openings, so its RESULT is deterministic and contention changes only how long it
# takes. The depth-4 datagen arm on core 12 is time-boxed and IS contention-sensitive, so this is
# pinned to a single separate core and left running for the whole of that arm rather than starting
# and stopping -- a steady neighbour perturbs both of its seeds equally, an intermittent one does not.
#
# PRE-REGISTERED:
#   * SAME WINNER at all four depths => the d2-vs-d3 verdict is about the nets. The parity caveat
#     limits its INTERPRETATION (it is not "deeper search sees more") but not its direction.
#   * WINNER FLIPS ACROSS PARITY (2,4 disagree with 3,5) => every verdict this project reached with
#     `netmatch ... 2` is protocol-dependent, and the ones listed in netmatch.rs:26-28 need re-reading
#     at depth 4. That would be the largest single correction available here.
#   * UNRESOLVED EVERYWHERE => 448 pairs cannot separate these nets and the honest answer is that the
#     original +-0.025 was never measurable at this sample size. netmatch prints the pairs needed.
set -uo pipefail
cd "$(dirname "$0")"
CORE=${CORE:-15}
PAIRS=${PAIRS:-448}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}
[ -x "$NM" ] || { echo "no netmatch at $NM"; exit 1; }

for SEED in 424242 987654; do
  A="dr_${SEED}_d2.net"; B="dr_${SEED}_d3.net"
  if [ ! -f "$A" ] || [ ! -f "$B" ]; then echo "seed $SEED: nets missing, skipped"; continue; fi
  for D in 2 3 4 5; do
    cls=$([ $((D % 2)) -eq 0 ] && echo EVEN || echo odd)
    echo "=== seed $SEED, judged at depth $D ($cls) : $A vs $B ==="
    timeout 5400 taskset -c "$CORE" nice -n 19 ionice -c 3 "$NM" "$A" "$B" "$PAIRS" "$D" 777 2>&1 \
      | grep -E 'scores|=>|arms:|pairs,' | sed 's/^/  /'
    echo
  done
done
echo "  d2 scoring BELOW 0.5 means the depth-3 arm is stronger."
echo "  Compare the four depths: a winner that changes with the judging depth was never a fact"
echo "  about these nets. Depths 2 and 4 share a parity class; 3 and 5 share the other."
