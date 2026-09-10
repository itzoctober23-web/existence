# The one verified rung is behind a VALLEY, and the valley is structural

`crates/interp/examples/evolve.rs valley 15 5 5 3` — deterministic, re-run reproduces the integers.

## The measurement

25 positions (15 mate-in-1, 5 depth-requiring, 5 window-sensitive) at depth 3, scored with the
search track's OWN `fitness`, on the same set that loop builds. Not a second harness.

| program | nodes | mates | cost | mates/Mcost | vs seed |
|---|---|---|---|---|---|
| bare alpha-beta (seed) | +0 | 25 | 9,930,290,911 | 0.002518 | 1.000x |
| probe only (never stores) | +69 | 25 | 10,023,369,775 | 0.002494 | **0.991x** |
| store only (never probes) | +39 | 25 | 9,962,055,071 | 0.002510 | **0.997x** |
| hash reuse (both halves) | +104 | 25 | 9,698,559,036 | 0.002578 | **1.024x** |

Mates are 25/25 for all four, so `f >= best_found` never discriminates; the entire decision is
cost, which is what makes the ratios exact rather than noisy.

**The control passed.** The full rung reproduces its gain on this set (+2.4%), independently of
the 120-position ladder that reported 0.98x cost. Had it not, the 0.98x would have been the thing
to re-examine and nothing else in the table would have meant anything.

## What it means

`evolve` accepts on `f >= best_found && rate > best_rate` — **strictly** greater. So:

* Neither half can ever be taken. Both are below 1.000x.
* Therefore the pair can never be assembled one edit at a time.
* And the pair is **+104 nodes**, which is not "1-3 mutations" under any reading.

**GRAMMAR 9 requires "a path of single mutations from the seed exists where every step is fitter".
For the ONLY rung ever measured as fitter than the seed, that premise is now measured FALSE.**

## Why this generalises past this set

The valley is not a property of these 25 positions, it is a property of what a transposition table
IS. `Interp::run` clears the table per position, so:

* probe-only searches a table **nothing ever wrote** — every probe is a guaranteed miss, so it is
  pure overhead by construction, not by sampling.
* store-only writes entries **nothing ever reads** — pure overhead by construction.

A TT is two edits whose payoff is *conjunctive*. Neither half pays alone, in this or any set. So a
larger or different position set moves the ratios and cannot move the sign.

## What this KILLS, before the time was spent

The obvious next move was to add mutation operators that can build `Probe`/`Key`/`Field`/`Store`,
since `tests/reachability.rs` proves the current operators cannot. **That work would not have
helped.** Reachability is necessary and not sufficient: the primitives being constructible says
nothing about a monotone path existing, and here it demonstrably does not. The operator work was
the natural thing to do next and it is now refuted for the cost of one 90-second probe.

It also means the reachability test's own note — "this failing means the single known improvement
is now inside the search space, re-run the search track" — was too optimistic, and is corrected.

## What it points AT

The bottleneck is the **search**, not the grammar. The valley is shallow — 0.9% and 0.3% — and
strict hill climbing is the only reason it is impassable. Options, in the order I would try them:

1. **Accept non-improving steps.** Plateau/drift acceptance (`rate >= best_rate * (1 - eps)`) with
   eps ~2% would let both halves be taken. Cheapest change; risks random walk, so it needs the
   `f >= best_found` mate guard to stay strict, which it does.
2. **A population with regression tolerance** rather than a single champion — the standard answer
   to a conjunctive-payoff valley.
3. **NOT** an operator that inserts probe+store as one edit. That is hand-coding the answer;
   MASTER_PLAN line 53 requires these to be DISCOVERED, and a gadget-inserting operator would make
   the discovery vacuous.

Option 3 is the tempting one and it is exactly the thing this project has agreed not to do.

## Honest limits

* One net (`Net::random(32, 20260907)`), one depth (3), one set. The SIGN is structural, per above;
  the MAGNITUDES (0.991 / 0.997 / 1.024) are specific to this configuration.
* This says nothing about whether hash reuse is worth having in the final engine — it is, at
  depths where a TT sees repeat traffic. It says the SEARCH TRACK cannot get there from here.

## 2026-09-09 21:47 — the valley IS crossable, and EPS is what crosses it

This document concludes the pair "can never be assembled one edit at a time", which is exactly right
about the ACCEPTANCE rule. But acceptance is not the only way a program survives a generation, and
the numbers above say the population can hold both halves at once.

**Retention is `x.2 >= top * (1 - EPS)` (`evolve.rs:1730`), not `> best_rate`.** Against the seed's
1.000x top:

    EPS 0.02  ->  keeps anything >= 0.980x     probe only 0.991x  RETAINED
                                               store only 0.997x  RETAINED
    EPS 0.10  ->  keeps anything >= 0.900x     both retained with far more margin

**Neither half can be ACCEPTED, and both are RETAINED.** They sit in the population as parents while
the champion stays the seed — which is precisely what a plateau tolerance is for.

**And crossover is already wired to combine them.** `evolve.rs:1586` runs `mutate::crossover(parent,
donor)` on one candidate in four, drawing the donor from the retained population. So the assembly
path is: EPS keeps probe-only and store-only alive on the valley floor, crossover unites them, and
the united program is +2.4% — above `best_rate` and therefore ACCEPTABLE.

**What that reframes.** The reachability objection is not "the operators cannot make a two-part
improvement"; they can, by construction, and GRAMMAR §4's "1-3 operators" is implemented faithfully.
It is that BOTH halves must be present in the population at the SAME time, and each is only there
because EPS tolerates a small loss. **EPS is not a diversity nicety here — it is the mechanism that
makes the one known rung reachable at all**, and the EPS 0.10 arm running tonight widens exactly that
window (0.900x against 0.980x).

**Not claimed.** That this has ever happened, or that it is likely. Both halves must be generated,
both retained simultaneously, and crossover must pick that pair — and `+104 nodes` is a large target.
The point is narrower and it corrects this document: the path is not closed by the acceptance rule,
because retention and acceptance are different gates. Whether the path is TAKEN is a question about
population size, EPS width and crossover rate — the three things the `lam32` and EPS arms vary.

## The valley floor IS populated — measured, and it happens at GENERATION 6

The `tt[...]` field on each gen line counts TT primitives PER POPULATION MEMBER, so it is a direct
observable for whether both halves of the rung are alive at once. Across every arm log:

    33 MAIN gen-lines, 2 contain a member with tt > 0  (6.1%)

and both of those hold MULTIPLE TT-carrying members at the same time:

    gen 6  MAIN  mate-ok 5, rates 0.997-0.999999x [>=.98:5, distinct:5]  tt[0,0,3,3,0,0]
    gen 7  MAIN  mate-ok 4, rates 0.878-1.000000x [>=.98:3]              tt[0,0,0,3,3,0,0,4]

**At gen 6, five candidates sat at 0.997-1.000x** — exactly the band where probe-only (0.991x) and
store-only (0.997x) live — and the population carried **two members with 3 TT primitives each**. That
is the crossover precondition, observed rather than argued: both halves retained simultaneously,
neither acceptable alone.

### ⚠ CORRECTION, 22:07, same evening — that last sentence OVERREADS THE METRIC

Written an hour earlier and wrong in the one way that mattered. `tt_prims` (`evolve.rs:1184`) is:

```rust
let here = matches!(n, Probe(_) | Store(..) | Key(_) | Field(..)) as usize;
```

**It pools FOUR node kinds into ONE integer.** So `tt 3` can be `Key+Field+Probe`, `Key+Field+Store`,
`Probe+Store+Key`, or three `Key`s. **A count of 3 in two members is equally consistent with both
members carrying the SAME half.** "Both halves retained simultaneously" is an inference about
COMPOSITION read off a metric that cannot express composition — the exact failure mode already
recorded for the `mates` field, where a pooled number hid that candidates were SELLING mates.

**What the measurement does support, and this part stands:**
- TT primitives are REACHED by mutation and RETAINED in a population — 2 of 33 MAIN gen-lines.
- It happens at gens 6-7, later than anything now running.
- Retention, not acceptance, is what keeps them (rates 0.997-1.000x are all below `> best_rate`).

**What it does NOT support:** that the two members are complementary halves, and therefore that the
crossover precondition was observed. That remains **NOT CLAIMED**, exactly as the section above it
already said — I contradicted my own "Not claimed" paragraph one section later.

**The instrument fix, since the count cannot be made to answer this.** `ttk[...]` now prints the KIND
COMPOSITION per member (`P`robe / `S`tore / `K`ey / `F`ield) beside the count, so a probe-half and a
store-half are distinguishable on sight. Print-only, no behaviour change. It cannot be applied to the
runs already in flight — those keep printing counts — so **the composition question is open until an
arm launched with the new binary reaches gen 6.**

**Why this correction matters more than the finding did.** A structural claim about search
reachability was about to rest on a four-way-pooled counter. Had `ttk` shown both members carrying
`KFS`, the conclusion "EPS makes the rung reachable" would have been backwards, and nothing in the
logs would have contradicted it.

### The instrument's positive control, and what it already settles

`evolve ttk` tags every reference rung and asserts the halves are separable. It passes:

| program | tt | ttk |
|---|---|---|
| bare alpha-beta (seed) | 0 | `-` |
| probe only (never stores) | 30 | `P10K10F10` |
| store only (never probes) | 10 | `S5K5` |
| hash reuse (both halves) | 40 | `P10S5K15F10` |
| **UCT MCTS** | **40** | **`P10S5K15F10`** |

**Two things fall out of this table that were not visible before.**

**1. The observed counts are ambiguous in BOTH directions, which vindicates the retraction.** The
cheapest reference half is store-only at `tt 10`, so `tt 3` looks at first like a mere fragment —
3.3x too small to be a half. But the reference programs apply the pattern at 5-10 CALL SITES, while
a mutation reaches ONE. A minimal probe is `P1+K1+F1 = 3`. A minimal *united* member is
`P1+S1+K1+F1 = 4`. So the gen-6 pair at `tt 3` is equally "two fragments" and "two minimal probe
halves", and **the gen-7 member at `tt 4` is equally a fragment and a minimal member carrying BOTH
HALVES AT ONCE** — which, if true, is not the crossover precondition at all but the rung itself, in
miniature, inside one program. The count cannot distinguish these. `ttk` can, and only on runs
launched with the new binary.

**2. UCT MCTS carries a FULL TT complement — `P10S5K15F10`, byte-identical to `ab_hash`.** The
second lineage is not merely a source of `Avg`; it is a donor pool that already contains both halves
of the rung, at every call site. And `donors` (`evolve.rs:1561`) flat-maps over **every lineage's**
population, so a MAIN recipient can draw an MCTS donor. That is a materially different reachability
story from the one this document has been telling: the halves may not need to be independently
*discovered* in MAIN at all — they can be *carried in* from the MCTS seed by the crossover that runs
on one candidate in four. `tests/reachability.rs:185` already proves crossover moves kinds across
lineages; it asserts only that the moved set is non-empty, and never checks for these four kinds
specifically.

**Still not claimed:** that this happens, or that a grafted TT subtree lands anywhere useful. Both
are now cheap to measure instead of argue — the first by `ttk` on a fresh run, the second by
extending the reachability test to name `Probe`/`Store` explicitly.

**And it happens at GENERATION 6-7, which nothing running tonight has reached.** Every live arm is
at generation 1-5. The measurement above comes from `gate_veto_arm_noverify.log`, a longer run. So:

- **The `hard` gradient switches on at gen 3 and peaks at gens 4-6** (measured earlier: 0% at gen 1,
  26% at gen 3, ~50% at gens 4-6).
- **The TT valley floor populates at gen 6-7.**

**Do not judge any of tonight's arms before generation 6.** At roughly 40 minutes per gated
generation that is two to three hours out. Every reading taken before then is from a regime where
the interesting structure has not yet appeared — which is worth stating plainly, because four arms
producing "REJECT, resolved worse" at gens 3-4 looks like a settled answer and is not one.
