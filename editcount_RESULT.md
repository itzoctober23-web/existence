# The loop DOES install both TT halves, at the pure-draw rate. The barrier is semantic placement.

Two instruments, each sized to its own effect, as `editcount_power_PREREG.md` required.

## 1. `stepdiff` sweep — n=40 per arm, WITH fitness

| edits | broken | identical | **cheaper** | different | **guard-ok** | both_halves |
|---|---|---|---|---|---|---|
| 1 | 3 | 25 | **0** | 12 | **0** | 0 |
| 2 | 7 | 13 | **0** | 20 | **0** | 0 |
| 3 | 11 | 5 | **0** | 24 | **0** | 1 |

Control passed: edits=1 reproduced the recorded n=200 shape at the shared checkpoint
(`broken 3, identical 15, cheaper 0, DIFFERENT 7, guard-ok 0`), so the new argument did not disturb
the default path.

The shape moves exactly as more edits should — no-ops fall 25 -> 13 -> 5, breakage rises 3 -> 7 -> 11,
behaviour-changes rise 12 -> 20 -> 24. **`cheaper` and `guard-ok` stay at ZERO throughout.** That is
the pre-registered outcome: *"identical_cheaper == 0 across all three -> the valley holds even when
both halves can be installed in one step."*

## 2. `ttreach` — n=5000 per arm, NO fitness, so the rare event is actually powered

    edits   welltyped  probe-side  store-side   BOTH   both-rate   expected-if-draw-limited
    1           5000         438         465      0     0.0000     0 (constructional)
    2           5000         824         928     91     0.0182     0.0165
    3           4988        1187        1230    220     0.0441     0.0451

**Observed sits on the expected line.** `ALL_OPS` has 11 operators drawn uniformly, so if the only
constraint were the DRAW, edits=2 gives `2*(1/11)^2 = 0.0165` and edits=3 gives `0.0451`. Measured:
**0.0182 and 0.0441.**

**So the operators compose freely.** Nothing about the type checker, the node positions, or operator
applicability suppresses the combination — a candidate carrying `ProbeRead` and `StoreHere` together
appears exactly as often as chance alone predicts.

## What this settles

**The loop installs both halves routinely.** At lambda 8 with 1-3 edits, a 1.8-4.4% pair rate is
roughly 0.15-0.35 both-halves candidates per generation, so a 25-generation run produces several. The
`both_halves = 1` observed in the n=40 fitness sweep at edits=3 (expected 1.80) is the same fact seen
through an underpowered instrument.

**And not one of them is ever cheaper or guard-passing.** Both columns are 0 across every edit count,
across 120 scored candidates.

**Therefore the barrier is neither expressibility nor draw. It is SEMANTIC PLACEMENT.** Having a
`Probe` node and a `Store` node in the same program is not a transposition table. A working TT needs
the probe to run BEFORE the recursion, the store AFTER it, and both keyed on the same position — and
`ProbeRead` places its read at an arbitrary Int-typed leaf while `StoreHere` appends its write at an
arbitrary statement position. The kinds co-occur at chance; the ARRANGEMENT essentially never does.

This is a sharper statement than "the valley is impassable", and it is a different problem. The
valley explains why each half alone is rejected (probe-only 0.991x, store-only 0.997x). Placement
explains why the PAIR — which is installed regularly — is also rejected: it is not the rung, it is
two unrelated memory operations that happen to share a program.

## Honest limits

* `ttreach` counts NODE KINDS, not arrangement. It cannot distinguish a correctly-wired TT from a
  probe and a store that never interact — which is precisely the gap it was built to expose, but it
  means "semantic placement" is an INFERENCE from (kinds co-occur at chance) AND (0 of them work),
  not a direct measurement of arrangement.
* The fitness sweep is n=40 per arm. `cheaper` and `guard-ok` at 0 across 120 candidates bounds those
  rates below roughly 2.5%, not below zero.
* All from the pristine seed. Live arms mutate evolved parents whose node shapes differ.
* A shape-level reachability check — does any operator sequence produce a probe that DOMINATES the
  recursive call it guards — is the follow-up this points at, and `reachability.rs` already flags
  shape-granularity as its own honest limit.
