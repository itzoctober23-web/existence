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

## 2026-09-09 22:20 — THE RUNG IS ONE GRAFT FROM THE SEED. Reachability was never the constraint.

`crossover_can_carry_the_tt_rung_from_mcts` (`crates/grammar/tests/reachability.rs`), 4000 crossover
attempts with `bare_alpha_beta` as recipient and `uct_mcts` as donor:

    4000 well-typed children of 4000 attempts        (100% type-check rate)
      carrying >=1 TT kind ................ 1959     (49.0%)
      carrying Probe AND Store, ONE graft ..  224     ( 5.6%)
      TT kinds that ever arrived: {Field, Key, Probe, Store}

**This refutes the reachability half of this document.** The header of `reachability.rs` records
that no MUTATION operator can introduce `Probe`/`Key`/`Field`/`Store`, and concluded hash reuse "is
not reachable at any edit count". That is true of mutation and irrelevant to crossover, which
`evolve.rs:1586` runs on one candidate in four with donors drawn from **every** lineage
(`evolve.rs:1561`). `uct_mcts` tags `P10S5K15F10` — byte-identical to `ab_hash`. **The second
lineage seed has been carrying a complete transposition table this whole time.**

**The valley argument survives; the "can never be assembled" conclusion does not.** The valley is
still conjunctive and each half is still below 1.000x, so neither half can be ACCEPTED. What changes
is that the pair does not have to be assembled one edit at a time at all — 5.6% of crossover children
arrive with both halves already present.

**Order-of-magnitude, stated as arithmetic and not as a claim.** At lambda 8, `i % 4 == 3` gives 2
crossover candidates per lineage-generation; donors are half MCTS by count, and 5.6% of those carry
both halves. That is roughly 5-6% per generation, so over a 25-generation run a Probe+Store child
appearing at least once is likelier than not. **This is a back-of-envelope on the PRISTINE seed, not
a measurement of the live runs.**

**What is NOT established, explicitly:**
- **That such a child keeps its mates.** Well-typed is not correct. `mutate.rs:437` records 90 of 106
  well-typed candidates rejected by the correctness oracle, and grafting a UCT subtree into
  alpha-beta is a far more violent edit than the 1-3 mutations that produced those rejects.
- **That it scores above `best_rate`.** `ab_hash` is 1.024x, but that is the HAND-BUILT rung with the
  pattern applied at 10 call sites. A single graft delivers `P1S1...`-scale coverage, and the valley
  table gives no reason to think one call site pays what ten do.
- **That the 5.6% holds for EVOLVED recipients.** The test grafts into the pristine seed. Live
  recipients are mutated programs whose subtree shapes differ.

**What this reframes.** The search track's failure has been read all evening as a SELECTION problem
(gate bounds, guard tolerance, set size, surrogate role) and then as a SUPPLY problem (lambda 32).
This says the supply of the one known rung is fine — **it is 5.6% of one candidate in four** — and
moves the suspicion to the correctness oracle and the cost model, which is where the next measurement
belongs. The cheap next step is to score those 224 children on the valley set and report how many
keep 25/25 mates and what they cost. That is a direct measurement, not another arm.

## 2026-09-09 22:30 — the ORACLE is what stops the rung. 0 of 40. Type-safety is not semantic safety.

`evolve ttgraft 40 15 5 5 3`, scoring 40 real `ab <- uct` children that carry BOTH halves, on the
same 25-position set and the same `fitness` as the table at the top of this file (control: the seed
reproduces 25 mates / cost 9,930,290,911 / rate 0.002518 exactly).

    attempts to collect 40 both-halves children : 499
    kept all 25 mates                          : 0 / 40
    kept mates AND rate > seed                 : 0 / 40

    mates kept:   0 -> 15 children      cost:  <1e6 (barely searches) -> 11
                 14-20 -> 21                   < seed                -> 27
                 22-23 ->  4                   > seed (blowup)       ->  2
                    25 ->  0

**This is the first of the three pre-registered outcomes: the CORRECTNESS ORACLE stops the rung, not
the surrogate and not the gate.** Reachability is solved — 5.6% of grafts carry both halves, and 499
attempts produced 40 of them in under a minute. Not one was a working program.

**The deeper result: 100% of these children TYPE-CHECK and 0% preserve semantics.** `crossover`
returns only after `typecheck::check_program(&out).is_ok()`, and every one of the 4000 attempts in
the reachability test passed. Grafting a UCT subtree into alpha-beta keeps the types — both compute
a value from a position — while destroying the minimax recursion. **The type system is exactly as
strong as it was designed to be and that is not strong enough to protect meaning.** GRAMMAR 4's
type checker is doing its job; the job is smaller than the search needs.

**And the surrogate alone is catastrophically gameable, which this measures rather than argues:**

    child #2:  15 mates, cost 3,310,801,733, ratio 1.800x   <- 1.8x FITTER while losing 10 mates
    best seen: ratio 2274.444x                              <- by barely searching at all

`mates/Mcost` has no floor on cost, so a program that abandons the search and still stumbles onto
some mates dominates the seed by three orders of magnitude. **The ONLY thing standing between this
search and a 2274x degenerate solution is the `f >= best_found` mate guard** — and here it rejected
all 40. FITNESS §10 lists "prune everything / return eval" against exactly this, and the guard is
the mechanism. Tonight's repeated temptation to widen `guard_tolerance` to let candidates through is
now measured as the single most dangerous knob in the loop.

**Where this leaves the search track**, with the eliminations now all measured rather than argued:

| suspect | verdict |
|---|---|
| selection (gate bounds, surrogate role, set size) | not the binding constraint |
| supply (lambda) | not the binding constraint — 40 both-halves children in 499 tries |
| reachability | **SOLVED** — crossover carries the rung from the MCTS seed |
| **correctness under graft** | **THE CONSTRAINT — 0 of 40** |

**What is still NOT claimed.** That no graft anywhere can work — 40 children of a pristine seed at
one depth on one set is a small sample, and the best kept 23 of 25 mates, which is close. The claim
is narrower and firmer: **a single unconstrained subtree graft is not a viable route to this rung**,
and the next question is whether a graft restricted to semantically compatible sites does better.
That is a real design question about crossover, not another arm of the same experiment.

### ⚠ CORRECTION, 22:38 — "0 of 40" used the WRONG GUARD. The live loop would ACCEPT 4 of them.

The section above scored "kept mates" as **strictly 25**. The live loop does not do that. It uses
`f >= guard_floor` where `guard_floor = f.saturating_sub(guard_tolerance)`, and MAIN runs
tolerance 4 — so the real bar is **21**, not 25. Re-scoring the same 40 children against the rule
`evolve` actually applies:

    tolerance 0 (floor 25):  0 pass guard,  0 also fitter
    tolerance 2 (floor 23):  1 pass guard,  1 also fitter
    tolerance 4 (floor 21):  4 pass guard,  4 also fitter   <- THE LIVE SETTING
    tolerance 6 (floor 19):  9 pass guard,  9 also fitter

**Four of forty — 10% — would be ACCEPTED by the running loop**, and all four carry the identical
minimal united tag `P1S1K2F1`:

    #3   22 mates  cost 7,598,468,488  1.150x  P1S1K2F1
    #4   22 mates  cost 7,598,470,314  1.150x  P1S1K2F1
    #9   22 mates  cost 7,599,115,805  1.150x  P1S1K2F1
    #31  23 mates  cost 7,688,872,004  1.188x  P1S1K2F1

So "the correctness oracle is what stops the rung" is **wrong as stated**. The STRICT oracle stops
it; the guard the loop actually runs lets 10% through, at ratios of 1.15-1.19x — **higher than the
hand-built `ab_hash` rung's 1.024x.**

**But look at WHAT gets through, because this is the whole point:**

    ab_hash    25 mates, cost 9,698,559,036   -2.4% cost, ZERO mates lost  -> 1.024x
    child #3   22 mates, cost 7,598,468,488  -23.5% cost, 3 mates lost     -> 1.150x

**A working transposition table buys 2.4% and loses nothing. These buy 23% and lose 2-3 forced
mates.** That is not a TT working — it is a search that stops earlier, and `mates/Mcost` pays more
for the truncation than it charges for the missed mates. The graft did not deliver the rung; it
delivered a cheaper broken search wearing the rung's node kinds.

**The real finding, and it is sharper than the one it replaces.** `guard_tolerance` does not make
the valley crossable. **It converts an unreachable rung into a reachable MATE SALE** — and the thing
it admits scores BETTER on the surrogate than the genuine improvement does. This is the same pattern
measured six times tonight (5x `mates 19`, 1x `mates 20`, every one resolving worse under VERIFY),
now caught at its source with the mechanism visible: the sellers are not near-misses at finding a
TT, they are a different and easier thing that the surrogate ranks above it.

**Which makes the dose-response above the most important table on this page.** Tolerance 0 admits
the truth (nothing) and freezes the search — measured directly tonight, `mate-ok 0` at gen 3.
Tolerance 4 admits 10% mate-sellers. Tolerance 6 admits 22.5%. There is no setting that admits the
rung and not the sale, **because on this fitness the sale scores higher than the rung.** The knob was
never the problem. `mates/Mcost` cannot rank a 2.4%-for-free improvement above a 23%-for-3-mates
truncation, and no guard threshold repairs that ordering.

**What this predicts, pre-registered:** if any live arm ever accepts a candidate carrying `ttk` with
both `P` and `S`, it will read ~1.15x on the surrogate, will have SOLD 2-3 mates, and will resolve
BELOW 0.5 under VERIFY — like every other seller. An accept is not a discovery here, and the `ttk`
field now makes the distinction visible on the gate line itself.

## 2026-09-10 — the crossover precondition IS met, and the `ttk` field is what settled it

At 21:47 I claimed two population members held "both halves of the rung ... neither acceptable
alone". At 22:07 I RETRACTED that: `tt` pools `Probe|Store|Key|Field` into one integer, so `tt 3` in
two members is equally consistent with both holding the SAME half. The retraction was correct — the
count could not support the claim. I then built `ttk`, which prints the per-kind composition, exactly
so the question could be settled by measurement rather than re-argued.

**It is settled, and the original claim was right.** `gate_composition_s2`, MAIN lineage:

    gen 4  pop 4  ttk["-", "-", "P1K1F1", "S1K1"]
    gen 5  pop 7  ttk["-", "-", "P1K1F1", "P1K1F1", "P1K1F1", "S1K1", "S1K1"]
    gen 6  pop 8  ttk["-", "-", "P1K1F1", "P1K1F1", "P1K1F1", "S1K1", "S1K1", "-"]

`P1K1F1` is a **probe half** — Probe + Key + Field, no Store. `S1K1` is a **store half** — Store +
Key, no Probe. These are DIFFERENT MEMBERS of one population, coexisting for three consecutive
generations, growing from one of each to three probe-halves and two store-halves.

**That is the crossover precondition, observed rather than argued.** Retention keeps both halves
alive because `x.2 >= top * (1 - EPS)` does not require improvement; acceptance would reject either
alone because each is below `best_rate` (probe-only 0.991x, store-only 0.997x). Exactly the state
this document predicted and could not previously demonstrate.

**Three things follow, and only the first is a claim about strength:**

1. **The two halves are simultaneously present and both retained.** Measured, three generations
   running, in the arm whose population reaches `MU`.
2. **This is the depth-heavy composition arm, not the control.** The control collapses to `pop 2`
   and its MAIN population is unobservable from gen 3 because it gates every generation. So this is
   NOT a demonstration that composition causes it — only that composition is where it is visible.
3. **Crossover has not united them.** `evolve.rs:1586` runs crossover on one candidate in four with
   donors drawn from every lineage, and `crossover_can_carry_the_tt_rung_from_mcts` measures 5.6% of
   `ab<-uct` grafts carrying both. Yet zero accepts, and `editcount_RESULT.md` measures that when
   both halves ARE installed the result is 1.0 probes/store and 62% hits — table traffic, not a
   transposition table.

**So the precondition is met and the conclusion is unchanged.** Having both halves in the population
is necessary and demonstrably not sufficient. What the arm is waiting on is not a rarer event; it is
a DIFFERENT one — a union that behaves like a table rather than like two memory operations sharing a
program.
