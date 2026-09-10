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

### "Crossover has not united them" is NOT yet evidence that it cannot — the power says 51% no-attempt

Across every arm and every generation, the complete set of population members carrying any TT
primitive:

    P10S5K15F10  15x   the MCTS SEED -- a full transposition table by construction
    P1K1F1       12x   a probe half, mutation-created, MAIN lineage
    P10S6K16F10  12x   an MCTS mutant -- one EXTRA Store and Key than the seed
    S1K1          5x   a store half, mutation-created, MAIN lineage

**No MAIN-lineage member has ever carried both halves.** The only both-halves members are the MCTS
seed and its mutants, which start that way.

**Before reading that as a failure of crossover, count the attempts.** At gen 6 of
`gate_composition_s2` the MAIN population is
`["-","-","P1K1F1","P1K1F1","P1K1F1","S1K1","S1K1","-"]`, and crossover fires on `i % 4 == 3` for
`i` in `0..8` — so `i = 3` and `i = 7`:

* `i=3` -> recipient is `popsnap[3]` = `P1K1F1`, a probe half. It needs a store-half DONOR, drawn
  uniformly from all lineages: **2 of 10 = 0.200**.
* `i=7` -> recipient is `popsnap[7]` = `"-"`, carries no half, cannot unite: **0**.

    P(a union is even ATTEMPTED in one generation)  ~ 0.20
      over  3 generations of coexistence:  0.49
      over 10 generations:                 0.89
      over 19 generations:                 0.99

**The halves have coexisted for three generations (4, 5, 6). P(no attempt yet) = 0.51.** Observing no
union is exactly what a coin flip looks like. It carries no information about whether crossover CAN
unite them.

**Which is the same trap as `both_halves == 0` at n=40 earlier tonight** — a zero read as a result
when the expected count was below one. The pre-registration there caught it; this needed catching
too, because the tempting conclusion ("crossover is attempted and fails") is both plausible and
unsupported.

**Nineteen generations remain in each composition arm, and P(at least one attempt) reaches 0.99 over
that span.** So this becomes answerable by waiting, and the thing to watch for on a gen line is a
single member tagged with BOTH a `P` and an `S` — `P1S1K2F1` or similar. That tag has never appeared
outside the MCTS lineage.

## 2026-09-10 — CROSSOVER CAN UNITE THE HALVES. 11 of 200 are PATH-1 acceptable.

`evolve ttunion 200 3` crosses `ab_probe_only` with `ab_store_only` — the two halves, hand-built —
and asks the question the live arms are sampling at 0.20/generation.

**All three controls hold**, which is what makes the rest readable:

| program | probes | hits | stores | p/s | same-play | cheaper |
|---|---|---|---|---|---|---|
| `ab_probe_only` | 759,795 | 0 | **0** | inf | true | false |
| `ab_store_only` | **0** | 0 | 55,114 | 0.0 | true | false |
| `ab_hash` (the union) | 745,219 | 7,318 | 53,405 | 14.0 | true | **true** |

The probe half probes and never stores; the store half stores and never probes; neither is cheaper;
only the union is. All three play identically to the seed, confirming they are behaviour-preserving
halves of ONE improvement rather than three different programs.

**The result:**

    well-typed children      : 200 of 200
    carrying BOTH halves     :  26   (13.0%)
    ...that play IDENTICALLY :  17   (65% of those)
    ...AND cheaper (PATH-1)  :  11   (5.5% of ALL attempts)

**Eleven children are behaviour-preserving speedups — exactly what PATH 1 accepts, with no games
played.** The route this document has spent all night showing to be open, correct, aimed at a
qualifying target, and empty, is now shown to be REACHABLE: given both halves in hand, one crossing
in eighteen produces something PATH 1 would take.

### What it projects onto the live arms

> **⚠️ SUPERSEDED 2026-09-10.** The 0.055 below is measured on halves that are biased optimistic BY
> CONSTRUCTION -- `ab_probe_only()` is `ab_hash_parts(true,false)`, i.e. one known-correct table split
> down the middle. Rejoining it is a friendlier question than the one the arms face. Read "The minimal
> halves, measured" at the foot of this file before quoting anything in this section.

    P(union even attempted per generation)   ~ 0.20    measured from composition_s2's gen-6 population
    P(a union is PATH-1 acceptable)          ~ 0.055   measured here
    -> P(acceptable union per generation)    ~ 0.011
       over the 19 generations remaining:      19%
       over 50 generations:                    42%

**So the arms are not chasing something impossible — they are chasing a ~1% per-generation event, and
19% is a real chance of catching it in the run that is already going.** That is a very different
statement from "no gradient in either currency", which is where the single-edit measurements left it.

### The limit that matters most

**These are the HAND-BUILT halves, `P10K10F10` and `S5K5`, applied at 10 and 5 call sites. The live
population's halves are MINIMAL — `P1K1F1` and `S1K1`, one site each.** A union of two minimal halves
is a probe and a store at one site apiece; whether that reproduces the 5.5% is NOT measured, and the
`ab_hash` control shows coverage matters (its whole 2.4% gain comes from 10 sites at a 1% hit rate).

Also: the united children here run at **30.4% hits and 3.8 probes/store**, against `ab_hash`'s 1.0%
and 14.0. They are cheaper and behaviour-preserving, so PATH 1 takes them — but they are not
behaving like `ab_hash`. Some of the 11 may be cheap for a reason unrelated to transposition reuse.
That is worth knowing before any of this is called "the search found hash reuse".

---

## The minimal halves, measured (2026-09-10)

The section above named its own limit: its halves sit at 10 and 5 call sites, the live population's at
one each. That limit turned out to be the more important half of the result.

**The halves here are built the way the population builds them** -- single mutations off the seed, kept
when they carry exactly one side -- and the mode ABORTS if it cannot draw both in 20,000 attempts rather
than falling back to the hand-built pair, which would have reproduced the old number under a new label.
The tags it built, `P1K1F1` and `S1K1`, are byte-identical to what `gate_composition_s2` carries at
gens 4-6. Same controls as before: probe half 0 stores, store half 0 probes, only `ab_hash` cheaper.

                              hand-built (10/5 sites)   MINIMAL (1/1 site)
    well-typed children              200 of 200            200 of 200
    carrying BOTH halves              26  (13.0%)           18  ( 9.0%)
    ...that play IDENTICALLY          17  (65.4%)            4  (22.2%)
    ...AND cheaper (PATH-1)           11  ( 5.5%)            3  ( 1.5%)
    hit rate / probes-per-store     30.4% / 3.8           4.9% / 4.2

### What is NOT resolved: the headline rate

**3/200 vs 11/200 is Fisher p = 0.0531. That is not a resolved difference and must not be reported as
one.** Wilson 95%: minimal [0.51%, 4.32%], hand-built [3.10%, 9.58%] -- they overlap. The point estimate
falls 3.7x, but this run cannot distinguish that from noise. `ttunion 1000 3 1` is running to settle it.

### What IS resolved: the bottleneck is SOUNDNESS, not speed

Decomposing the funnel separates a decisive difference from a null one:

    behaviour preservation among union-carriers   65.4% vs 22.2%   Fisher p = 0.0065   DECISIVE
    cheaper GIVEN it preserves behaviour          64.7% vs 75.0%   Fisher p = 0.91     NULL

**A minimal union is not failing because it is slow. It is failing because it returns the wrong answer
~78% of the time.** Once it preserves behaviour it is, if anything, slightly likelier than the
hand-built one to be cheaper. Every generation of effort aimed at making the united child *faster* is
aimed at the step that is already working.

### Why the hand-built number was optimistic BY CONSTRUCTION

`ab_probe_only() = ab_hash_parts(true, false)` and `ab_store_only() = ab_hash_parts(false, true)`. They
are not two independent programs that happen to carry one primitive each -- they are ONE known-correct
table with one side deleted, at matched sites. Crossing them re-joins a table that was sound before it
was split. That measures *"can crossover rejoin a correct table"*, which is close to a tautology, not
*"do two independently-placed random edits form a table"*, which is what the arms are attempting.

So the 5.5% was never the live rate, and the 19% projection built on it was never the live projection.
Using the minimal point estimate instead: **0.20 x 0.015 = 0.30%/generation -> 5.5% over the 19
generations remaining** (range across the CI: ~2% to ~15%), and 13.9% over 50.

### The union event has still never happened outside the MCTS lineage

Across both composition arms, every member tagged with both P and S is `P10S5K15F10` or
`P10S6K16F10` -- that is `uct_mcts`, which was BORN with a table, and one store-edit off it. The
evolved minimal members are `P1K1F1` or `S1K1`, never fused. **Zero genuine unions in 21 generations of
live population.** A near-miss to watch for is a single member tagged like `P1S1K2F1`.

### Next discriminator, named and NOT built

The soundness gap predicts something countable: if a one-site probe reads entries a one-site store wrote
in a different context, then hits should be dominated by writer-context != reader-context. Tag each
stored slot with its writer call-site id and count cross-context hits. Not built -- the actionable
conclusion (soundness is the barrier, cheapness is not) does not move on the answer, and the n=1000
power run is the thing that changes a published number.

### RESOLVED at n=1000 (2026-09-10)

The n=200 run could not separate 3/200 from 11/200 (Fisher p=0.0531) and was reported as unresolved.
`ttunion 1000 3 1` settles it.

    minimal n=1000     10/1000 = 1.00%   CI [0.54%, 1.83%]
    minimal POOLED     13/1200 = 1.08%   CI [0.63%, 1.84%]
    vs hand-built      11/ 200 = 5.50%   CI [3.10%, 9.58%]
    Fisher exact                          p = 0.00018   RESOLVED

**The minimal rate is ~5x lower than the hand-built rate, and that difference is real.** The 5.5%
was measuring a friendlier question, as the construction predicted.

### The funnel, pooled -- one decisive step and one that is exactly null

    behaviour preservation among carriers   65.4% (17/26)  vs  22.8% (21/92)   p = 0.00009
    cheaper GIVEN it preserves behaviour    64.7% (11/17)  vs  61.9% (13/21)   p = 1.00000

**p = 1.0000 on the second row.** Once a minimal union preserves the answer it is cheaper at
indistinguishably the same rate as the hand-built one. The ENTIRE gap is soundness: a probe and a
store dropped at one arbitrary site each return the wrong answer 77% of the time. Nothing about
making these children faster is the problem.

### Corrected projection

    P(union attempted/gen) 0.20  x  P(acceptable) 0.0108  =  0.217%/generation
      over the 19 generations remaining:   4.0%   (CI range 2.4% - 6.8%)
      over 50 generations:                10.3%

**The published 19% becomes 4.0%.** The arms are chasing a real event, roughly five times rarer than
this document claimed an hour ago.

---

## The rung is a THREE-part conjunction, not two (2026-09-10)

`evolve ttunion 600 3 1`, splitting the table stats by whether the child preserves behaviour. The
split was built because the pooled numbers could not distinguish two opposite explanations of the 23%
that ARE sound: a table that genuinely reuses, versus a pair that never fires. Those predict opposite
hit rates, so the measurement settles what an argument cannot.

    sound   (plays identically): 812,454 hits / 1,664,245 probes = 48.8%,  5 of 8  NEVER HIT AT ALL
    unsound (changes the answer): 24,435,203 / 226,550,469 = 10.8%,       24 of 36 NEVER HIT AT ALL

**The second line is the result.** 24 of 36 children that change the answer never record a single
hit. A table that never hits should be pure overhead and behaviourally inert -- it cannot change what
the search returns. These do.

### Why: a MISS is not a no-op, it injects the constant 0

Traced through the interpreter rather than inferred:

    Tt::probe()  -> on miss returns `Slot::default()`            (interp/src/lib.rs:352)
    Slot         -> `#[derive(Default)]`, every field 0, incl. `score` AND `flag`   (:242)
    Node::Field(Slot, Score) -> `Value::Num(sl.score)`           (:846)

So `Field(Probe(Key(p)), Score)` on a miss evaluates to **0**, and any program that uses that value
in place of a real score has silently substituted a constant. That is why a never-hitting union is
unsound: it is not inert, it is a zero-injector.

**This is the SAME bug the repo already documented in `ab_hash`** (`reference.rs:255`): *"the program
returned `score` from an empty slot, i.e. the constant 0, without ever calling `eval`"*, measured
then at ZERO evaluations at every depth. The fix recorded there is the missing ingredient here:
*"1. A VALIDITY marker. `flag != 0` distinguishes a stored entry from an empty slot; the old code had
no way to, which is the whole bug."*

### What this reframes

**Hash reuse is not `Probe + Store`. It is `Probe + Store + a validity test`, and the `Slot` type
already carries the field for it (`flag`).** The hand-built halves are both derived from `ab_hash`,
which CONTAINS the flag check -- so crossing them re-supplies the third ingredient for free. The
minimal halves do not carry it at all.

That single fact explains every number in this document that was previously only described:

* why the minimal rate (1.08%) is ~5x below the hand-built rate (5.5%, p = 0.00018) -- the hand-built
  crossing is solving a 2-part problem, the live population a 3-part one;
* why SOUNDNESS is decisive (65.4% vs 22.8%, p = 0.00009) while CHEAPNESS is exactly null
  (64.7% vs 61.9%, p = 1.00000) -- the missing piece is a correctness guard, and guards do not
  make things faster;
* why 77% of unions change the answer.

**The valley is deeper than this document has been assuming all night.** GRAMMAR 9 asks for a path of
single fitter mutations; the measurements here have been treating the target as two edits. It is at
least three, and the third one (`flag != 0`) pays NOTHING on its own and nothing in pairs -- it only
pays in the presence of both others.

### Pre-registered next test, NOT yet run

If the validity test is the missing ingredient, then a union that also carries a `flag`-guard should
be sound at a much higher rate than 22.8%. Build a third minimal half that tests `flag != 0` and
measure the three-way crossing. **Prediction: soundness rises toward the hand-built 65%; the PATH-1
rate rises with it.** If soundness does NOT rise, the validity marker is not the missing ingredient
and this section is wrong -- which is the point of writing the prediction down first.

### Correction to my own framing: "three-part conjunction" UNDERSTATES it, and +104 was already here

The section above concluded the rung is `Probe + Store + a validity test` and called it a three-part
conjunction. Reading `ab_hash_parts` (`reference.rs:315`) rather than reasoning about it, the probe
half ALONE is:

    If(flag != 0,
       If(depth >= d,
          Seq[ If(flag == 1, Ret(score)),
               If(flag == 2, If(score >= b, Ret(score))),
               If(flag == 3, If(score <= a, Ret(score))) ]))

One validity test, one depth test, three bound-type dispatches and two bound comparisons -- and the
file explains why it is nested rather than conjoined: *"the grammar has no `and`, and adding one for
this would change the primitive count that GRAMMAR 6's prior is measured in."*

**And this document already carried the number, near the top:** *"the pair is +104 nodes, which is not
'1-3 mutations' under any reading."* So the DEPTH of the conjunction was never in dispute; I restated
a known fact in weaker terms. Recording that rather than leaving two inconsistent framings in one
file, which is the exact trap this project keeps paying for.

**What tonight actually added, stated narrowly:**

1. The minimal-halves PATH-1 rate, 13/1200 = 1.08% against the hand-built 5.50%, Fisher p = 0.00018 --
   resolved, where the n=200 run could not resolve it.
2. Which STEP of the funnel is binding: soundness 65.4% vs 22.8% (p = 0.00009) while
   cheaper-given-sound is 64.7% vs 61.9% (p = 1.00000, exactly null).
3. The MECHANISM for the unsoundness, traced through the interpreter rather than argued: `Tt::probe`
   returns `Slot::default()` on a miss, `Slot` derives `Default`, and `Field(Slot, Score)` therefore
   yields the CONSTANT 0. A miss is not inert -- it is a zero-injector. That is why 24 of 36 unsound
   children never record a hit.
4. That the guard `ab_hash` uses to prevent exactly this (`flag != 0`) is the ingredient the minimal
   halves lack, and that a bare flag READ is single-edit reachable while a flag TEST is what actually
   matters -- the distinction mode 3 exists to measure.

None of that changes the +104-node conclusion. It explains WHY the 104 nodes cannot be approached
piecewise: the guards pay nothing individually, and without them the probe actively corrupts the
answer rather than merely costing time.

### Mode 2 (bare flag READ) — the CONTROL arm, and it is a null

`evolve ttunion 600 3 2`. A third half carrying a `Field(_, Flag)` READ is crossed into the
probe-store child. This exists to answer a question the treatment arm cannot answer alone: does merely
having the flag PRESENT change anything, or does it have to be TESTED?

    third half built by single mutation: P1K1F1 with 1 flag read

                              2-way minimal (n=1000)   + bare flag READ (n=600)
    carrying BOTH halves              74                      63
    ...play IDENTICALLY               17  (23.0%)             10  (15.9%)
    ...AND cheaper (PATH-1)           10                      10
    sound children that NEVER HIT      -                       6 of 10
    unsound children that NEVER HIT    -                      46 of 53

**Soundness: 23.0% -> 15.9%, Fisher p = 0.3895. NOT resolved.** The point estimate falls but this run
cannot distinguish that from noise, so the honest reading is that **a bare flag read neither helps nor
hurts measurably**. It does not raise soundness, which is what the validity-marker hypothesis
predicts — a read used as a VALUE just substitutes one number for another, it gates nothing — but a
null is weaker evidence than a fall, and it is reported as a null.

**The zero-injection signature is reproduced independently here:** 46 of 53 unsound children never
record a single hit, and still change the answer. That is the third dataset showing it.

**Why this control was necessary.** Without it, a positive result in mode 3 would be ambiguous: adding
ANY third program to the crossing changes program size, node counts and mutation surface, so an
improvement could come from the extra material rather than from the guard. Mode 2 supplies the extra
material WITHOUT the guard. If mode 3 rises above both, the guard is doing the work.

Mode 3 (`ttunion 600 3 3`, third half required to TEST the flag — an operand of a `Cmp`/`Pred` or an
`If` condition) is running. **Pre-registered:** soundness rises materially above BOTH 23.0% and 15.9%,
or the validity-marker hypothesis is wrong and this section says so.

## The population is 3:1 skewed toward PROBE-carriers, and 43% of generations cannot union at all

Measured across every arm log (`gate_*.log`), counting per generation how many population members
carry the probe half (P and no S) versus the store half (S and no P):

    arm               pop  probe  store   P(random pair is P x S)
    composition_s1     5     1      0        0.000
    composition_s1     6     1      0        0.000
    composition_s1     6     1      0        0.000
    composition_s1     8     2      1        0.062
    composition_s1     8     2      1        0.062
    composition_s1     8     2      1        0.062
    composition_s2     3     1      0        0.000
    composition_s2     3     1      0        0.000
    composition_s2     4     1      1        0.125
    composition_s2     7     3      2        0.245
    composition_s2     8     3      2        0.188
    composition_s2     8     5      1        0.156
    composition_s2     8     5      1        0.156
    composition_s2     8     5      1        0.156

    pooled: 33 probe-carriers vs 11 store-carriers over 14 generations  ->  3.0 : 1
    mean P(random pair is P x S) at the observed composition : 0.090
    the same populations BALANCED (3 probe / 3 store of 8)   : 0.281   -> 3.1x

**Two things fall out, and the second is the sharper one.**

1. **The skew costs a factor of ~3.1 in the union rate.** The projection in this document uses
   `P(union attempted per generation) ~ 0.20`, measured from one gen-6 population. The composition
   term inside that is 0.090 where a balanced population would give 0.281.
2. **Six of the fourteen generations hold NO store-carrier at all.** In 43% of generations the union
   event is not improbable, it is IMPOSSIBLE -- there is nothing to cross the probe half with. A
   projection that treats every generation as an independent trial at a uniform rate is therefore
   wrong in a way that OVERSTATES the chance, and the true figure is lower than the 4.0% recorded
   above.

### Why this is not obviously a selection problem, which is what makes it worth measuring

The valley table at the top of this document measures **store-only at 0.997x and probe-only at
0.991x**. Retention is `x.2 >= top * (1 - EPS)`, so the CHEAPER half -- the store -- is the one
retention should favour. **The observed skew runs the opposite way, 3:1 toward probes.** So the cause
is more likely SUPPLY (single mutations emit probe-carriers more often than store-carriers) than
SELECTION, and those have different fixes: operator weights versus the retention rule.

`evolve ttsupply <draws> <edits>` counts it directly -- what fraction of single mutations off the seed
carry the probe half only, the store half only, both, or neither. **Pre-registered:** a supply ratio
near 3:1 means the skew is supply and the fix is operator weights; near 1:1 means it is selection and
the fix is retention. Zero store-only children would mean supply is the binding constraint outright,
and no retention rule can keep what is never generated.

### RESOLVED: the 3:1 skew is SELECTION, not supply — and it runs against the valley's own prediction

`evolve ttsupply 20000 1`, pre-registered reading committed before the run:

    well-typed children : 20000 of 20000
    probe-carrier only  :  1812  (9.060%)
    store-carrier only  :  1768  (8.840%)
    BOTH halves at once :     0  (0.000%)
    SUPPLY RATIO probe:store = 1.02 : 1

**Supply is balanced to within 2%.** Against the live populations' 3.0:1:

    supply rate  p(probe | carrier)   = 0.5061
    live population                   = 33/44 = 0.7500
    exact binomial, two-sided           p = 0.001306      RESOLVED

    expected store-carriers at the supply rate : 21.7
    observed                                   : 11
    -> selection removes roughly HALF the store-carriers that supply provides

**So the fix is the retention rule, not operator weights** — which is what the pre-registered reading
said a ~1:1 supply ratio would mean, written down before the number existed.

### And it contradicts what the valley table predicts

The table at the top of this document measures **store-only 0.997x, probe-only 0.991x**. Retention is
`x.2 >= top * (1 - EPS)`, so it keeps whatever is CHEAPER — which is the STORE half. Selection is
instead culling stores at twice the rate of probes. **Something in the selection path is not behaving
the way the recorded costs say it should.**

The most likely mundane explanation is that those costs do not transfer: the valley table measures the
HAND-BUILT halves at 10 and 5 call sites, while the population carries MINIMAL ones (`P1K1F1`, `S1K1`)
at one site each — the same distinction that already cost this document a 5x error in the PATH-1 rate
(5.50% hand-built vs 1.08% minimal, p = 0.00018). If minimal store-carriers happen to be the more
expensive of the two, retention explains the skew mechanically and nothing is wrong.

**That is a hypothesis with a countable prediction, so it gets measured rather than argued.** Extending
`ttsupply` to report the COST DISTRIBUTION of minimal probe-carriers against minimal store-carriers.
**Pre-registered:** minimal stores measurably more expensive than minimal probes explains the skew via
retention and closes it; costs equal or stores cheaper means retention cannot explain it and the
selection path itself needs auditing.

### One more hard number, worth stating on its own

**0 of 20,000 single edits produce BOTH halves** (95% upper bound 0.015%). This document has assumed
throughout that the union requires crossover; that assumption is now measured rather than argued.

## The STORE half is the binding constraint, and the whole chain is now measured

The `ttk[...]` field is printed from `popn`, which `evolve.rs` builds as:

    pool.sort_by(|a, b| b.2.partial_cmp(&a.2))   // by RATE, DESCENDING
    pool.retain(|x| x.2 >= top * (1.0 - EPS));   // the EPS band
    pool.retain(dedup);
    pool.truncate(MU);                            // elitist: keep the top MU

**So the tag array is in RATE ORDER, and every arm log has been recording each carrier's rank all
along.** No new run was needed; the quantity was already on disk.

    rank position within the population (0 = best rate, 1 = worst)
      probe-carriers   n=33   mean 0.630   median 0.667
      store-carriers   n=11   mean 0.933   median 1.000

      store-carriers in the BOTTOM HALF : 100%
      probe-carriers in the BOTTOM HALF :  70%
      store-carriers ranked LAST        :  64%

      Mann-Whitney U   z = +3.94   p = 0.00008     RESOLVED

**Every store-carrier that has ever appeared in these populations sat in the bottom half, and
two-thirds of them were dead last.** Selection sorts by rate and truncates to MU, so they are the
first thing removed.

### The chain, end to end, with the evidence for each link

    1. SUPPLY of the two halves is balanced        1812 vs 1768 of 20,000    ratio 1.02 : 1
    2. minimal STORE-carriers rank systematically worst   100% bottom half   p = 0.00008
    3. selection is elitist                        sort by rate desc, truncate(MU)
    4. => stores are culled                        33 probes vs 11 carriers   p = 0.001306
    5. => 6 of 14 generations hold NO store-carrier  the union is IMPOSSIBLE, not improbable, in 43%
    6. => the union rate loses ~3.1x               P(pair) 0.090 observed vs 0.281 balanced

**The store half -- not the probe half, not crossover, not the validity guard -- is the binding
constraint on the hash-reuse rung.** A store that nothing reads is pure overhead by construction
(this document's own words, near the top), so it is always the worst member present, so elitist
selection removes it before a probe-carrier can ever be crossed with it.

### And it explains the contradiction rather than leaving it

The valley table measures **store-only 0.997x against probe-only 0.991x** -- the store half is the
CHEAPER of the two, which predicts the opposite skew. That table measures the HAND-BUILT halves at 10
and 5 call sites. **The minimal halves invert the order**, and the rank data is what shows it. This is
the third time in this document that a hand-built/minimal difference has overturned a conclusion drawn
from the hand-built numbers (the PATH-1 rate 5.50% -> 1.08%, the soundness funnel, and now this).

**Standing correction:** the valley table's ratios describe `ab_probe_only`/`ab_store_only`. They do
NOT describe what the population carries, and no argument about population dynamics should be built
on them again.

### What this makes actionable, stated without recommending a change

The barrier is not expressiveness, not reachability, not crossover, and not the guard. It is that
**elitist truncation removes the store half faster than crossover can use it.** Anything that keeps a
store-carrier alive for one more generation attacks the actual constraint: a non-elitist slot, a
diversity-preserving retention rule, or explicitly protecting minority TT kinds. Which of those is
legitimate under MASTER_PLAN is a separate question from which one would work, and this section only
establishes the second.
