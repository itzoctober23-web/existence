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

## ⚠ 2026-09-10 — CORRECTION: the pairs DO hit. "Semantic placement" was wrong, and the tool said so.

The section above inferred that an installed Probe+Store pair is inert — *"the kinds co-occur at
chance; the ARRANGEMENT essentially never does"* — and flagged it explicitly as an inference from
two facts rather than a measurement. `evolve tthits` measured it. The inference is **false**.

    CONTROL ab_hash (hand-built, working TT): 745,219 probes,  7,318 HITS  ( 1.0%)
    seed bare_alpha_beta (no TT at all)     :       0 probes,      0 hits  (0/0, as required)

    mutants carrying BOTH halves     : 10 of 300
    ...that actually EXECUTE a probe : 10
    ...that ever get a HIT           :  8
    total across them                : 74,677,154 probes, 46,391,167 hits

**8 of 10 hit.** The pairs are not decorative and the placement is not refusing them.

### But the HIT RATE is the finding, not the hit count

|  | probes per position | hit rate |
|---|---|---|
| `ab_hash`, a working TT | 124,203 | **1.0%** |
| both-halves mutants | 1,244,619 | **62.1%** |

**10x the probes at 63x the hit rate.** A real transposition table mostly MISSES — every new node is
a position not seen before, so ~1% is what genuine tree reuse looks like at this depth. **A 62% hit
rate means the probe keeps returning the SAME slot**: one key, written once and read back
repeatedly. That is a scratch variable, not a transposition table.

And it is an expensive one. `Node::Probe` costs 12 units, so 1.24M probes per position is **14.9M
cost units of probing alone**, against the seed's entire ~397M per-position search. The pair does not
fail by doing nothing; it fails by doing a great deal of useless work.

### What this replaces

Not *"probe and store never end up correctly arranged"* but **"probe and store readily end up
arranged as a memo cell, which is cheap to reach and costs more than it saves"**. The operators
compose freely (measured), the pair executes (measured), the table is used (measured) — and what
gets built is a degenerate single-slot cache rather than a search-tree table.

That is a more specific and more useful failure than placement-in-general, and it points somewhere
different: the gap is not arrangement but **KEYING**. `ProbeRead` emits `Key(Var("p"))` on whatever
`p` is in scope at an arbitrary Int leaf, so the same key recurs; a transposition table needs the key
to vary with the node being searched.

### And the tool's own closing text was wrong

`tt_hits` printed *"A pair that never hits is not a transposition table"* — my expected conclusion,
baked into the output, contradicted by the very first run. Corrected in place. **Asserting the
expected finding in a tool's output is how a tool stops being able to surprise you**, and this one
surprised me only because the numbers were printed beside it.

## ⚠ 2026-09-10 — SECOND correction, same instrument: the memo-cell reading is refuted too

I replaced "the pairs never hit" with "the pairs form a memo cell — one key stored, read back
repeatedly". Adding a store counter refuted that as well.

    ab_hash (working TT) :   124,203 probes/pos,     8,901 stores/pos  ->  14.0 probes/store
    both-halves mutants  : 1,244,619 probes/pos, 1,269,477 stores/pos  ->   1.0 probes/store

A memo cell stores rarely and probes constantly. **These store MORE than they probe** — 76.2M stores
against 74.7M probes. That is the opposite signature.

### What is MEASURED, and it is now a decent picture

| | value |
|---|---|
| mutants (of 300, 3 edits) carrying both halves | 10 |
| ...that execute a probe | 10 |
| ...that ever hit | 8 |
| hit rate | 62.1% (ab_hash: 1.0%) |
| probes per store | 1.0 (ab_hash: 14.0) |
| table ops per position | 2,514,096 |
| table traffic as cost | ~30.2M units/position, **8% of the seed's entire 397.2M search** |

**Controls held on both runs**: `ab_hash` registers hits and stores; the seed reads 0/0/0.

### What is NOT established: the mechanism

**Three mechanisms proposed, two refuted by this same instrument:**

1. *"an arbitrarily placed pair never hits"* — **REFUTED**, 8 of 10 hit.
2. *"it is a memo cell, few stores read back often"* — **REFUTED**, 1.0 probes/store.
3. **None offered.** I am not proposing a third without a test for it in the same breath.

That is the honest state. The pattern in my own reasoning is the finding worth recording: each time
I explained the previous measurement, the explanation was a story that fit the numbers I had and died
against the next number. `probes/store` was not a hypothesis I held — it was a discriminator I built
because the memo-cell story predicted something specific, and it came back the other way.

**What can be said without a mechanism:** these programs perform ~2.5M table operations per position,
about **8% of a full seed search in table traffic alone**, and get nothing for it — the pair is
installed, the table is used heavily, and the result is still rejected on both cost and correctness.
Whatever the arrangement is, it is not one that converts table traffic into saved search.

**Next discriminator, if one is wanted:** a real TT's value is reading a slot written at a DIFFERENT,
earlier node. Counting hits whose stored key was written at a different tree depth would separate
genuine reuse from self-inflicted hits. That is one more counter — and unlike the two stories above,
it is a measurement before it is a claim.

## ⚠ 2026-09-10 — THIRD mechanism refuted. Stopping the guesses and recording the measurements.

    ZERO-KEY (the `_ => 0` fallback) : 0 probes (0.0%), 0 stores (0.0%)

The `Node::Probe`/`Node::Store` fallback that silently turns a non-Key expression into slot zero
**never fires**. The mutants use real, varying keys.

**Three mechanisms proposed, three refuted, all by the same instrument:**

| # | proposed | refuted by |
|---|---|---|
| 1 | an arbitrarily placed pair never hits | 8 of 10 hit |
| 2 | a memo cell: few stores, read back often | 1.0 probes/store — they store MORE than they probe |
| 3 | the key collapses to 0 via the `_ => 0` fallback | 0.0% zero-key, on both probes and stores |

The brief's rule — *two wrong hypotheses in a row means the harness is wrong* — was applied after the
second and the harness checked out: exactly one probe site and one store site, each in the correct
node handler, verified by grepping every call of `Tt::probe` and `Tt::entry`. The controls hold on
every run: `ab_hash` registers hits, stores, and 0.0% zero-key; the seed reads 0/0/0.

**So the harness is sound and my explanations were not. That is the finding.**

### What is measured, and it is enough to act on

    both-halves mutants (of 300, 3 edits) : 10;  10 probe;  8 hit
    hit rate                              : 62.1%   (ab_hash: 1.0%)
    probes per store                      : 1.0     (ab_hash: 14.0)
    zero-key                              : 0.0%    (ab_hash: 0.0%)
    table ops per position                : 2,514,096
    table traffic as cost                 : ~30.2M units/position = 8% of the seed's 397.2M search

The pairs are installed, keyed properly, and used heavily — and every one is still rejected on cost
and correctness. **Whatever the arrangement is, it converts table traffic into no saved search.**

### The one discriminator that would settle it, named and NOT built tonight

A real TT's value is reading a slot written at a **different, earlier node**. Tagging each stored
slot with the tree depth that wrote it, and counting hits where writer-depth ≠ reader-depth, would
separate genuine reuse from self-inflicted hits — a probe reading back what the same node just wrote.

That is one more counter. I am not building it now: three explanations have already died here, the
actionable conclusion (0 of 120 candidates work) has not moved through any of them, and the 4PC side
has a corpus running. Recorded so the next person spends a counter rather than a story.
