# The search track's surrogate rewarded searching LESS (2026-09-08)

Recorded here because the code change was swept into the gate-A/B commit by a `git add -A` and
landed under a message that does not describe it. The logs that hold the evidence are gitignored,
so the numbers live here.

## What happened

P2 (search learning) is described in MASTER_PLAN:276 as "open-ended, runs from day one". It was
not running -- `evolve_search.log` shows a run that reached generation 13 and died, with nothing
restarting it. Restarted at 12:30, it immediately produced:

```
  seed: bare alpha-beta  80 mates  3049174420 cost  0.03 mates/Mcost  (71 nodes)
  gen   1  ACCEPT  80 mates  8.73 mates/Mcost  (71 nodes, was 0.03)
  gen   2  ACCEPT  80 mates  8.74 mates/Mcost  (70 nodes, was 8.73)
```

333x cheaper from ONE type-preserving edit, at an unchanged node count. A 290x gain from a single
mutation is not a discovery.

## The mechanism, tested rather than assumed

`evolve.rs` accepts on `f >= best_found && rate > best_rate` -- keep every mate, get cheaper --
with no game gate in the loop (its header says the gate "is what would confirm a winner on the
clock", i.e. deliberately outside). The set came from `mate_set`, which collects positions holding
a mate in ONE. On such a set shallowness cannot lose a mate, so the guard could never bite.

Measured with the SEED program, no mutation involved (`examples/mate_surrogate_probe.rs`):

| set | depth 1 | depth 2 | depth 3 |
|---|---|---|---|
| mate-in-1 (what it optimised on) | 80/80 mates, 246M cost | 80/80, 3049M | 80/80, 34551M |
| mate-in-2 (the repair) | **17/40** forcing, 124M | 40/40, 1356M | 40/40, 16540M |

On mate-in-1, searching less keeps every mate and costs 12x less -- the guard is inert. On
mate-in-2, searching less loses 23 mates -- the guard bites. **The repair is the SET, not the
rule.** The rule was right all along and had nothing to enforce.

## My probe was wrong first, and the control caught it

The mate-in-2 arm initially read 0 mates at depths 1, 2 AND 3. A set built to be solvable scoring
nothing at every depth is not believable, and the fault was mine: the scorer credited a move only
if it mated IMMEDIATELY, and a mate-in-two's first move never does. Fixed by recording the forcing
move alongside each position. Without that check, "depth 3 solves nothing" would have been written
up as a property of the search.

## State

evolve.rs now runs on 80 mate-in-1 + 40 forced-mate-in-2 (positions with no mate-in-1 available),
and REFUSES TO RUN if the depth-requiring half is empty -- otherwise the loop optimises toward a
depth-1 mate detector while printing ACCEPT. Seed on the mixed set: 120/120 mates, 0.03 mates/Mcost.
