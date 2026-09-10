# Why the MAIN lineage has never produced an improvement, and structurally cannot

2026-09-08. Not a bug report — an account of the fitness, backed by 93 generations of its own logs.

## The fitness has two dimensions. One is saturated and the other is blocked.

`fitness = (mates, mates-per-cost)` with acceptance `f >= best_found && rate > best_rate`.

**MATES IS SATURATED.** The set has 25 positions and the seed scores **25/25**, the maximum. The
guard is `f >= best_found`, so every surviving candidate also scores exactly 25. Mates therefore
cannot discriminate between survivors — it is a pass/fail filter, never a gradient.

**COST NEVER IMPROVES.** Measured across every search-track run on disk:

```
generations with >=1 surviving candidate:            93
of those, any survivor cheaper than the champion:     0
```

Ninety-three generations, zero. The population holds members *below* the champion — that is the
plateau tolerance working as designed, and the kill criterion has been met twice — but nothing has
ever crossed above it.

## Why cost never improves: the guards block exactly the mutations that would reduce it

For a correct alpha-beta at fixed depth, there are two ways to get cheaper:

1. **Search less** — a shallower horizon, a raised alpha, a narrower window. This is by far the
   most reachable, and the track found it twice within hours (`Const(0)`→`Const(1)` in the horizon
   guard; `neg(INF)`→`8` at the root). Both were caught, and the disagreement and window-sensitive
   sets were built precisely so such candidates score **zero** rather than "fewer".
2. **Do the same search more efficiently** — hash reuse, better ordering. This preserves every
   answer, so it passes the guards.

The guards are correct and necessary. But their consequence is that the fitness reduces to *"be
cheaper without searching less"*, and for an already-correct alpha-beta at fixed depth the set of
such programs is nearly empty. **The only member ever found is hash reuse at 1.024x** — and
`ladder_valley_RESULT.md` shows it sits behind a conjunctive valley (halves at 0.991x and 0.997x)
that a strict climb cannot cross.

So the reachable search space is: one known improvement, behind a valley, and everything else is an
exploit the guards correctly reject.

## And even if cost DID improve, it would not mean strength

`docs/FITNESS.md`, measured this session: **nine of eleven reference programs never read
`Budget`** — the entire alpha-beta family, including `ab_id`. A depth-limited program that costs
half as much returns the **identical move** sooner; it does not search twice as far. Cost converts
to playing strength only for a budget-aware program, and in this reference set only `uct_mcts` and
`proof_number` are.

The MAIN lineage is therefore optimising a quantity that (a) empirically never improves, and (b)
would not produce a stronger engine if it did.

## What this is and is not

**It is not a bug.** Every component is behaving as specified: the guards block the exploits they
were built for, the plateau tolerance carries intermediates (verified by TT-primitive counts, not
inferred), the game gate refuses to promote on the surrogate alone. The *composition* is what has
no gradient.

**It is not a claim that the thesis is dead.** It is a claim about this fitness on this seed at this
depth. Three things would change it, in rough order of how directly they attack the cause:

* **A budget-aware seed.** Then cost is convertible and the cost term means what it says. This
  collides with MASTER_PLAN:53, which requires ID to be discovered rather than seeded — a real
  tension, and one for the Given column rather than a patch.
* **An unsaturated correctness dimension.** A position set the seed does *not* score 25/25 on gives
  mates a gradient instead of a pass/fail. The MCTS lineage already has this (11/25), which is why
  its surrogate moves at all.
* **Strength itself as the fitness**, via games against a fixed anchor. That is the only signal
  measured to resolve anything today — but `proxies_RESULT.md` puts the cost at ~1,650 pairs to
  resolve one generation's real edge, which is why it is not the per-generation metric.

## Provenance

93-generation figure derives from every `search_track_*.log` on disk plus the live run, counting
generations whose `mate-ok` was non-zero and whose best surviving rate exceeded 1.000x of the
champion. The saturation claim is structural (25 positions, guard at the maximum), not statistical.


---

## THE ROOT CAUSE, found 2026-09-08: alpha-beta is EXACT, so correctness cannot discriminate

The section above says mates is saturated and treats that as a property of the position set. It is
not. It is a property of **alpha-beta**, and no set can fix it.

**Alpha-beta with a full window returns the same value as minimax.** So every correct variant at
the same depth returns the **same move**. `bare_alpha_beta`, `ab_hash`, `ab_id` and `ab_hash_id`
are not merely similar — they are behaviourally IDENTICAL. They differ only in what they cost to
compute an answer they all agree on.

That predicts both saturations exactly, and both are measured:

| set | seed | every exact variant | why |
|---|---|---|---|
| guard (25 positions) | 25/25 | 25/25 | all return the same moves |
| hard (8 positions, depth+1 answers) | 0/8 | **0/8** | all return the same depth-3 moves |

I built the hard set specifically to create a correctness gradient, and ran the control before
restructuring acceptance around it. **Every reference program scores 0/8 — including
`capture_extension`, which was the specific hope**, since it searches deeper on tactical lines. The
gradient does not exist for anything reachable.

**Had I skipped that control and rebuilt the acceptance rule first, selection would have been
driven by a dimension on which every candidate scores zero** — no change at all, dressed as a fix.

### What this means

The MAIN lineage's fitness cannot have a correctness gradient at fixed depth. Not "does not
currently" — cannot, as a consequence of alpha-beta's exactness. The only quantity that varies
among correct programs is COST, and `docs/FITNESS.md` shows cost cannot convert to strength for a
depth-limited program.

So the search space decomposes into exactly two kinds of candidate:

* **Exact variants** — identical play, differing only in cost. Cost is unconvertible, so these
  cannot be stronger. Only hash reuse is even cheaper (1.024x), and it is behind a valley.
* **Inexact variants** — extensions and reductions, which change the effective depth and therefore
  CAN play differently. `capture_extension` and `table_reduction` are the two in the reference set,
  measured at 0.985x and 0.993x: both LOSSES on mates-per-cost, because changing the search costs
  more without winning anything the guard set can see.

**The fitness rewards the class that cannot improve and penalises the only class that can.**

### What would actually change it

A fitness that can see the value of an extension. That means positions where a SMALL change of
effective depth flips the answer — not a full extra ply, which is what the depth+1 hard set demands
and what nothing reachable can deliver. Building such a set from `capture_extension`'s own
disagreements would work mechanically and is REFUSED: it bakes the intended answer into the
measurement, which is the same defect as an operator that inserts a gadget.

The honest alternatives remain the three already recorded: a budget-aware seed, strength itself as
the fitness (~1,650 pairs per generation), or accepting that this lineage is a null result and
saying so.


---

## MEASURED, not inferred: the WHOLE alpha-beta family plays identically

The section above argued exactness from theory and from a coincidence of scores. `evolve moveagree`
tests it directly — run each reference program on the same 40 random positions at depth 3 and
compare the MOVE returned against the seed's:

| program | agrees with seed | class |
|---|---|---|
| depth-one | 10/40 (25.0%) | different paradigm |
| bare alpha-beta | 40/40 (100%) | exact |
| alpha-beta + hash reuse | **40/40 (100%)** | exact |
| alpha-beta + iterative deepening | **40/40 (100%)** | exact |
| alpha-beta + hash + ID | **40/40 (100%)** | exact |
| **capture extension (rung 6)** | **40/40 (100%)** | predicted "may differ" |
| **table reduction (rung 7)** | **40/40 (100%)** | predicted "may differ" |
| UCT-style MCTS | 4/40 (10.0%) | different paradigm |
| proof-number search | 0/40 (0.0%) | different paradigm |

**The exactness claim is confirmed. And my two-class decomposition was wrong in its second half.**

I wrote that extensions and reductions "change the effective depth and therefore CAN play
differently", and offered that as the one class capable of improving. Measured: `capture_extension`
and `table_reduction` return the IDENTICAL move on all 40 positions. The escape hatch I proposed
does not exist at this depth.

So **every one of the seven alpha-beta-family programs plays exactly the same chess.** The only
programs that diverge are other paradigms. GRAMMAR 9's ladder is not a strength ladder at depth 3 —
it is purely a COST ladder, and its rungs are indistinguishable as players.

That makes two rungs strictly worse than the seed rather than merely unfitter: capture extension
costs 1.5% more (0.985x) and table reduction 0.7% more (0.993x), both for identical play. **Pure
overhead**, not a trade.

### Limit of this control

40 random positions at depth 3. The honest claim is "zero disagreements observed here", not "never
differs" — a capture extension must eventually change a move on some position, and deeper searches
give extensions more room to matter. What is established is that at the depth this fitness actually
runs, the family is behaviourally uniform, which is what the argument needed.


---

## Depth 4, with a valid cost cap: uniformity does NOT break

The "raise the fitness depth so extensions have room to matter" escape was the last cheap way out.
It is closed, at least at depth 4. Re-run with the per-run cost cap raised to 5e10 so nothing is
truncated (`noMove` is 0 for every program — the earlier depth-4 run had 17-19 of 20 searches cut
off by the default 2e9 ceiling and was worthless):

| program | agrees with seed @ d4 | noMove |
|---|---|---|
| bare alpha-beta | 12/12 (100%) | 0 |
| alpha-beta + hash reuse | **12/12 (100%)** | 0 |
| alpha-beta + iterative deepening | 12/12 (100%) | 0 |
| alpha-beta + hash + ID | 12/12 (100%) | 0 |
| **capture extension (rung 6)** | **12/12 (100%)** | 0 |
| **table reduction (rung 7)** | **12/12 (100%)** | 0 |
| depth-one | 3/12 (25%) | 0 |
| UCT MCTS | 0/12 (0%) | 0 |
| proof-number | 0/12 (0%) | 0 |

**All seven alpha-beta-family programs still play identically.** Extensions and reductions do not
start diverging with an extra ply of room. Only other paradigms differ.

**And hash reuse agrees 12/12**, which closes the transposition-table soundness alarm completely:
the depth-4 "18/20" that raised it was the cost cap truncating searches, with `MOVE_NONE ==
MOVE_NONE` scored as agreement and the two "disagreements" being positions where ab_hash COMPLETED
and the seed did not — because ab_hash is cheaper. The TT is sound and the 1.024x rung stands.

### Where that leaves it

Uniformity holds at both depths tested (40/40 at depth 3, 12/12 at depth 4). The MAIN lineage's
fitness cannot discriminate on correctness at either, cost is the only signal, and cost is
unconvertible for a depth-limited program. Two rungs of the declared ladder are pure overhead at
both depths: capture extension 0.985x and table reduction 0.993x, for identical play.

Limit: 12 positions at depth 4 and 40 at depth 3, and only those two depths. This is not "identical
at every depth forever" — a capture extension must eventually change a move somewhere. It is that
at the depths this fitness can afford to run, the family is behaviourally uniform, which is what
the argument needs and all it claims.


---

## 2026-09-08, LATER: the uniformity had a cause, and fixing it makes rung 6 play differently

The sections above conclude that the entire alpha-beta family plays identically and that the only
class which could improve — the inexact variants — is empty. **The measurements were right and the
explanation was wrong.** Two of the seven were no-ops:

* `pred` (GRAMMAR primitive #5) was a stub returning false, so capture extension's condition never
  fired. It was the seed plus a dead branch.
* `capture_extension` also applied the extension at EVERY depth rather than "at horizon" as
  GRAMMAR 9 rung 6 specifies — invisible while the predicate was dead.

With the predicate implemented and the extension moved to the horizon (`d == 1`), measured on 10
positions at depth 3:

| program | evals | cost | agrees with seed |
|---|---|---|---|
| bare alpha-beta | 1,413,909 | 1.000x | 10/10 |
| alpha-beta + hash reuse | 1,368,508 | 0.980x | 10/10 |
| **capture extension (rung 6)** | **1,971,912** | **1.679x** | **8/10 — DIFFERS** |
| table reduction (rung 7) | 1,413,909 | 1.007x | 10/10 (still a no-op) |

**Cost 73x -> 1.679x, and it plays differently on 2 of 10 positions.** This is the first program in
the alpha-beta family measured to play different chess from the seed. The class is not empty.

### What this does and does not establish

**Does:** a correctness gradient is now POSSIBLE. A candidate can differ from its parent in what it
plays, which is the precondition for any fitness that scores play rather than cost. The
`search_track_WHY_NOTHING` argument — mates saturated, cost unconvertible, therefore no gradient —
loses its second leg.

**Does not:** it says nothing about whether the different moves are BETTER. Playing differently is
necessary, not sufficient. That needs the value oracle (`evolve ttvalue` does exactly this for a
pair of programs) or games against a fixed anchor, and until one of those is run, "rung 6 differs"
is all that is claimed.

`table_reduction` remains a no-op for an unrelated reason the predicate fix does not touch: `tread`
discards its index arguments, so `TRead(3, [d, i])` cannot vary by depth or move index, and the
harness passes only three tables so index 3 is out of range anyway.


---

## The diagnosis completed, 2026-09-08

Four measurements, each of which corrected a guess I had made earlier, now compose into one account.

**1. The operators are NOT the blocker.** 150 single-edit mutants of the seed, 8 positions at
depth 3: 18 broken (12%), 79 identical (53%), **53 play differently (35%)**, produced by InsertMax
20, Delete 10, Dup 9, ReplaceConst 7, Tweak 5, WrapIfPred 2. I built this expecting the useful
bucket might be empty, which would have moved the blocker to the operator set. It is not empty.

**2. Correct programs are behaviourally identical, so correctness cannot rank them.** All seven
alpha-beta-family reference programs agree with the seed 40/40 at depth 3 and 12/12 at depth 4.
That is alpha-beta's exactness, not an accident of the set.

**3. The hard set does not discriminate either — for ANYTHING.** It was built specifically to
create a correctness gradient, with the seed scoring 0/8 by construction. Every reference program
also scores 0/8. And across **92 generations of live search, roughly 1,100 candidates, not one has
ever scored above 0**. Unsaturated, and unreachable by anything the search actually produces.

**4. Cost is the only remaining signal and it cannot convert to strength.** Nine of eleven
reference programs never read `Budget`, so a depth-limited program that costs half as much returns
the identical move sooner (`docs/FITNESS.md`).

### The account

The search HAS behaviour-changing candidates — 35% of single edits — and the fitness CANNOT RANK
THEM BY PLAY. Mates saturates at 25/25 among everything that passes the guard; the hard set reads 0
for everything ever generated; and what remains is cost, which for these programs is not strength.
So the population wanders among genuinely different programs guided by a signal that is not
measuring the thing anyone wants.

This is not "the fitness needs tuning". Every component behaves as specified, and the composition
has no gradient toward playing strength.

### What is left

Only games against a fixed anchor have resolved anything all session — they refuted capacity
(0.179 ± 0.021) and the draw filter (+0.086 ± 0.015). As a per-generation fitness they cost roughly
1,650 pairs to resolve one generation's real edge (`proxies_RESULT.md`), which is why they are not
already the metric. That is the honest trade: the only valid signal is the one that is
unaffordable at this cadence, and everything cheaper has now been measured and found uninformative.


---

## THE CRUX, measured: no single edit is both correct and behaviour-changing

`evolve stepdiff`, 100 single-edit mutants of the seed, scored against the loop's REAL guard (the
25-position mate/disagreement/window set, `f >= best_found`):

```
BROKEN (returned no move)                     12
IDENTICAL (passes guard, plays the same)      55
DIFFERENT (plays differently)                 33
...AND passes the guard (THE USEFUL KIND)      0
```

**Zero of thirty-three.** The space at one edit is strictly partitioned: everything that changes
play FAILS the guard, everything that passes the guard plays IDENTICALLY. Rule of three puts the
95% upper bound on the useful rate at ~9%; the point estimate is 0.

**My first version of this instrument reported 35%.** It sorted on "did it return a move" rather
than on the guard the loop actually enforces, and its own doc comment claimed the buckets meant
"keeps all answers". Same experiment, correct criterion, and the answer moves 35 points to zero.

### What this actually says

The guard set is 25 positions chosen to be MAXIMALLY SENSITIVE — mate-in-1, depth-disagreement,
window-sensitive. Any behavioural change is overwhelmingly likely to lose at least one of them. So
`f >= best_found` over that set is, in practice, **"do not change behaviour"**.

That puts the correctness guard and the improvement goal in direct conflict:

* The guards are RIGHT about exploits. They were built after two real ones (searching a ply
  shallower; raising initial alpha to +8) and they catch that class exactly.
* But a genuine strength improvement ALSO changes behaviour, and on a 25-position all-or-nothing
  bar it will lose something. Capture extension is the worked example: it plays differently on 2 of
  10 positions and would be rejected by any all-or-nothing guard that those 2 positions touch.

So the loop is not failing to find improvements. It is **rejecting every candidate that could be
one**, using a rule that cannot distinguish "worse" from "different".

### Where that leaves the search track

The three components are each individually correct and jointly unworkable: operators that produce
behavioural novelty, a guard that rejects all of it, and a cost term that cannot convert to
strength. Fixing this means the guard must tolerate REGRESSION on some positions in exchange for
gains on others — which requires being able to weigh them, which requires a strength signal, which
is games. The same conclusion the ceiling work reached from the other direction, and at the same
price: ~1,650 pairs per generation.


---

## A pre-registered prediction of mine, FALSIFIED

Before implementing `pred` I wrote: it had been a stub returning false, so `Op::WrapIfPred` was a
disguised DELETE (wrapping a statement in `if false` removes it), and fixing it should RAISE MAIN's
mate-ok rate above its measured baseline of 2.20/12.

| lineage | pred stubbed | pred working | delta |
|---|---|---|---|
| MAIN | 2.20/12 (n=45) | **2.12/12 (n=16)** | −0.08 |
| MCTS (control) | 4.47/12 (n=32) | 4.10/12 (n=10) | −0.37 |

**It did not rise.** And the CONTROL moved further than the treatment — MCTS contains no `Pred` and
cannot be affected by the fix, so its −0.37 sets the noise scale, against which MAIN's −0.08 is
nothing. The comparison itself is sound: both runs are MU=8, λ=12, EPS=0.020, identical seed cost
and identical mutation seeding, so the only difference is the predicate's semantics.

**Why the prediction was wrong**, and `stepdiff` had already said so: WrapIfPred produced only **2
of 53** behaviour-changing single edits. It is one operator of eleven and a weak one, so changing
its semantics entirely — from "delete the statement" to "make it conditional" — moves the aggregate
survival rate hardly at all. Both semantics are destructive for the same reason: wrapping
`set best` in a condition breaks the accumulator whether the condition is `false` or `is_capture`.

The `pred` fix remains correct and necessary — it is what made rung 6 fire at all, and rung 6 is the
first family member measured to play differently. It simply did not do the thing I predicted it
would do to the population, and the prediction is recorded as failed rather than quietly dropped.


---

## The obvious fix is dead: measured, before implementing it

The crux ("0 of 33 behaviour-changing edits pass an all-or-nothing guard") suggests one fix: allow
a candidate to LOSE a couple of guard positions. The loss distribution looked encouraging —

```
guard positions lost by the 33 behaviour-changing candidates (of 25):
  lost  2:  3      lost  6:  7      lost 22:  2
  lost  3:  3      lost  7:  6      lost 25:  8
  lost  5:  3      lost  9:  1
```

— minimum 2, with a clean gap before the catastrophic cluster. A tolerance of 2 would admit 3 of 33
where the current guard admits zero.

**Then I measured what the known exploits lose, instead of asserting it.**

| program | guard score | LOSES | mates/Mcost |
|---|---|---|---|
| seed | 25/25 | — | 1.00x |
| DEPTH exploit (`d==0` → `d==1`) | 16/25 | **9** | **7.37x** |
| ALPHA exploit (`neg(INF)` → `8`) | 23/25 | **2** | **1.29x** |

**The alpha exploit loses exactly 2 — the same as the minimum a genuine change loses.** A tolerance
of 2 re-admits it, and at 1.29x the seed's rate it is accepted instantly and the search degenerates.
A tolerance of 1 admits nothing, since no genuine change loses fewer than 2. **No tolerance value
separates the classes.**

I had asserted both exploits lose 5, "by construction", because the disagreement and window subsets
are 5 positions each. Both numbers were wrong — 9 and 2. That is the fourth time this session that
reasoning from construction disagreed with measurement, and the first three were caught the same
way.

### Why this matters beyond the dead fix

Loss-count cannot separate an exploit from an improvement, and neither can rate: the alpha exploit
posts 1.29x, which any rate-based rule would reward. The two classes are distinguishable only by
whether the different moves are BETTER — which is playing strength, which is games. Every route out
of this loop now terminates at the same place, and at the same measured price of ~1,650 pairs per
generation.

---

## UPDATE 2026-09-09 — the count, deduplicated, and the premise that has since changed

**0 accepts in 99 INDEPENDENT gate calls.** Deduplicated by
`(run_seed, guard_tolerance, hard_fitness, lineage, generation)` over every arm log on disk:

    unique lineage-generations : 234
    of those, produced a pick  :  99   (42.3%)
    ACCEPTED                   :   0

**The raw counts were 510 and 215 — a 2.17x inflation from duplicate trajectories.** Arms with
identical configuration replay the identical run, so the same lineage-generation appears in several
logs. That is the same error I made earlier today reporting "4 of 4 MAIN candidates"; it is recorded
here because the raw number is the one a casual `grep -c` produces. Reassuringly the *rate* is robust
to it (42.2% raw vs 42.3% deduped) — only the denominator moves.

**The search is NOT failing to find surrogate improvements.** It finds one in 42.3% of
lineage-generations. What fails is the conversion: `fitness_saturation_RESULT.md` measures 3 of 3
MAIN candidates that improved the surrogate as RESOLVED WORSE by a 96-pair independent-seed VERIFY,
while MCTS — whose seed is 15/23 and therefore not saturated — is 0 of 3. So this document's account
stands, with the emphasis moved: the problem is not a barren search, it is a surrogate whose gains do
not transfer.

**A measurement trap, recorded because I walked into it first.** I initially counted "max candidate
rate > 1.000x" across gen lines and got **0 of 270**, which looks like a dramatic finding and is
tautological: a GATED generation prints VERIFY and gate lines and no `rates` field at all, so every
line carrying a `rates` span is by construction one where nothing was picked. Any statistic drawn
from the `..none` lines alone measures the selection, not the search.

**PREMISE CHANGE, so the numbers above do not match the top of this file.** This document opens with
"the set has 25 positions and the seed scores 25/25". That was the old guard set. The current runs
use the forced-mate set and report **`23/23 mates (floor 19)`** — the repair described in
`evolve.rs:43-56`, which swapped a mate-in-1 set (where no amount of shallowness can lose a mate) for
one where shallowness does lose mates. **The conclusion is unchanged because 23/23 is still
saturated**: the numerator is at its maximum, so `mates/Mcost` can still only be improved by cutting
cost. The repair moved the number, not the defect. That is what `EXISTENCE_HARD_FITNESS=1` is now
being A/B'd against.

---

## 2026-09-09 — THE BAR RATCHETS ON REJECTION, and the codebase already has the fix

**A gate REJECTION raises `best_rate` to the rejected candidate's rate.** `evolve.rs:2157`, in the
reject path:

```rust
if spec_filter { lineages[li].gated.insert(format!("{c:?}")); }
else           { lineages[li].best_rate = rate; }
```

So the champion never moves — 0 accepts and 0 PATH-1 promotions in any standard arm, and the MAIN
population spread tops out at exactly the seed's `0.002490` in every one of them — while **the bar
the next generation must clear keeps rising, set by programs the gate just measured as WORSE.**

### It cost a gate call that would otherwise have happened

Measured on the EPS arm, seed 1:

    seed / champion rate ............. 0.002490    champion never changed
    gen-3 candidate, gate REJECTED ... 0.002924    VERIFY 0.422 +/-0.027 = RESOLVED WORSE
    best_rate after that rejection ... 0.002924    +17.4% over the seed

    gen-4 candidates vs the RAISED bar:   0.002770 = 0.947x  ->  below, NO gate
                                          0.002810 = 0.961x  ->  below, NO gate
    the SAME candidates vs the CHAMPION:  0.002770 = 1.112x  ->  above, would have gated
                                          0.002810 = 1.129x  ->  above, would have gated

Generation 4 produced no gate call at all, and the reason is not that its candidates were weak
against the champion — they were 11-13% above it. They were below a bar inherited from a program that
had already been rejected as worse.

### Why this compounds with the saturation diagnosis rather than replacing it

The rejected candidates are cost-cutters (MAIN's numerator is saturated at 23/23, so a surrogate gain
can only come from the denominator). So the ratchet raises the bar **specifically along the cost
axis** — the one axis VERIFY says is anti-correlated with strength. Each rejection makes the next
step harder in the direction already measured as wrong.

### The repair is already in this file, used on the other branch

The `spec_filter` branch does the right thing: it records the program in a `gated` HashSet and
**leaves `best_rate` alone**. The struct comment at `evolve.rs:1399` explains the asymmetry — under a
TOLERANCE filter, raising the bar would ratchet it DOWNWARD, "because every rejection lowers the
reference the next 0.9x is measured against", so that path needs the set instead.

That reasoning is about protecting the tolerance filter. It does not argue that raising the bar is
CORRECT under the strict rule; the bar-raise is simply the older anti-re-proposal mechanism that the
`gated` set was invented to replace. Applying `gated` to both branches would prevent re-proposal
without inheriting a bar from a rejected program.

**NOT changed yet, and deliberately so.** Four arms are mid-run, the A/B on `HARD_FITNESS` weight is
the question currently being answered, and changing the acceptance dynamics underneath it would
confound both. Recorded here as the next repair, with its evidence attached.

### PRE-REGISTERED consequence of removing the ratchet

Written before the fixed arms have produced a single gate.

With `best_rate` no longer rising on rejection, it stays at the CHAMPION's rate — and the champion
has never moved in any standard arm. The population, meanwhile, persists across generations and holds
members above that rate. So the pick should now find a non-gated candidate above the bar in most
generations, where previously an inherited bar suppressed it.

**Prediction: the fraction of lineage-generations that reach a gate should RISE materially from the
measured baseline of 99/234 = 42.3%.** If it does not move, the ratchet was not what was suppressing
gate calls and the account above is wrong.

Two things this is NOT:

- **Not a claim that more gates is progress.** More gates means more DATA, not more strength. The
  accept count is still 0 and only an accept changes that.
- **Not unbounded.** `gated` accumulates, so a lineage whose population is exhausted of untried
  members above the bar will stop gating on its own. That is the intended stopping behaviour, not a
  failure.

**Cost consequence, stated so it is not a surprise.** A gated generation is dominated by VERIFY (96
pairs = 192 games) plus the gate itself, and measured at roughly 40 minutes. If the gating fraction
goes from ~42% toward ~100%, generations get correspondingly slower in wall-clock. A 25-generation
run becomes an overnight job rather than an afternoon one. That is the right trade — the previous
speed came from skipping measurements, not from being efficient.

### 2026-09-09 21:08 — two refuted hypotheses in a row, so I checked the HARNESS

The standing rule is that two wrong hypotheses in a row means the harness is wrong, not the subject.
Tonight had exactly two:

1. **Saturation** — MAIN's numerator pinned at 23/23 so only cost can improve. REFUTED by the `mates`
   field: the winner read **19**, having sold 4 mates.
2. **`GUARD_TOL=0` closes the market** — REFUTED by running it: `mate-ok 0` of 8 at gen 3, the search
   freezes rather than redirects.

**So I checked the harness before proposing a third.** Both refutations arrived FROM instrumentation
behaving correctly — the `mates` field printed what it was built to print, and the tolerance-0 arm
measured what it was built to measure. That is the harness working, not failing. But "it produced an
answer" is not the same as "it can produce the RIGHT answer", so the question worth asking is
whether this pipeline could recognise a genuine improvement if one appeared.

**It can.** `GRAMMAR.md:617,676` records hash reuse as the only rung ever measured fitter than the
seed: **0.98x the seed's cost at D=3, identical play**. Scored against today's live configuration:

    seed     f=23  cost 9.235e9  rate 0.002490
    ab_hash  f=23  cost 9.051e9  rate 0.002541  = 1.0204x best_rate
    guard f >= 19 ?  PASS        rate > best_rate ?  PASS

A same-play speedup keeps every mate and clears the bar by 2%. **The selection rule is not blind to
a real improvement** — so the failure is not that the pipeline cannot see one, and my two wrong
hypotheses were wrong about the MECHANISM rather than about the instrument.

**What that leaves.** The operators have to actually PRODUCE such a candidate. GRAMMAR 9 records that
hash reuse is two edits that only pay off together — a store nothing reads is pure overhead and a
probe of an empty table can never hit — so a single mutation reaches neither half. That is a
reachability question about the mutation operators, not a measurement question about the fitness,
and it is the one thing tonight's work has NOT touched.

### Reachability checked — NO spec gap, and the real constraint measured

Having refuted two hypotheses I checked the third before acting on it, and it does not hold either.

**The operators are spec-compliant.** `mutate_program` does `let edits = 1 + rng.below(3)`, which is
GRAMMAR §4's *"1-3 operators per candidate, chosen uniformly"* exactly. And `evolve.rs:1579` runs
**crossover on 1 candidate in 4** — which is precisely the designed mechanism for assembling a
two-part improvement like hash reuse, where a store nothing reads and a probe of an empty table are
each useless alone. So "a single mutation cannot reach a two-edit improvement" was wrong: the search
makes multi-edit moves and recombines across parents by design.

**The measured constraint, over all of today's arms:**

    22 generations, 160 candidates
      ill-typed ...................  16   10.0%
      kept every mate .............  13    8.1%
      WELL-TYPED BUT LOSE MATES ... 131   81.9%   <- the binding constraint

Type-safety is not the problem — 90% of candidates are well-typed, which is `mutate.rs`'s own
verified claim holding in production. **Four candidates in five are well-formed programs that
compute something different and lose mates for it.** `mutate.rs:437` predicted exactly this: *"Three
random edits to a program that already computes the exact minimax value will almost always break
it"*, with 90 of 106 rejected by the oracle on the first real run.

**What this rules in and out.** It rules OUT a spec deviation in the operators, and it rules OUT
"the pipeline cannot see a good candidate" (the `ab_hash` arithmetic above shows a same-play speedup
clears the bar by 2%). What it leaves is that the seed is a program computing an EXACT minimax value,
so nearly every perturbation of it is a strictly worse program — and 8 candidates per generation
against a ~92% failure rate yields well under one viable candidate per generation. That is a
population-size and operator-bias question, not a fitness question, and it is the first thing tonight
that none of the four running arms addresses.

## 2026-09-10 — the account completed: it is the ACCEPTANCE RULE, and the one rule that would fire was never run

This file's 2026-09-08 account is that mates are saturated and cost is blocked, so the fitness cannot
discriminate. A day of measurement closes it. Six findings, in the order they force each other.

### 1. The bottom line, as a COUNT: 0 of 78 gate calls ever resolved BETTER

Classifying every gate call discretely — RESOLVED WORSE (`rate+ci95 < 0.5`), TIE, RESOLVED BETTER
(`rate-ci95 > 0.5`) — across every arm running today:

    arm                        cell               worse   tie   better
    gate_diversity_PAIRED_off  div OFF filt OFF      2      20      0
    gate_diversity_s1          div ON  filt OFF      2      22      0
    gate_filter_only           div OFF filt ON       0       4      0
    gate_div_x_filter          div ON  filt ON       0       5      0
    gate_specfilter_s1         12+6+5  filt ON       2       5      0
    gate_sprt30_s1             12+6+5  filt OFF      2      14      0
                                                    ------------------
                                                     8      70      0

Six arms, both fitness compositions, both selection rules, both gate types, three binaries, three
seeds. **88-100% ties. Zero better.**

### 2. Why the ratio makes selling mates mandatory, measured

Every gated candidate sits at the guard floor — MAIN 14 of 14 at 19-20 (seed 23, floor 19), MCTS 16 of
24 at exactly 6 (seed 10, floor 6). Not a spread with some sellers; a pile-up at the boundary. The
exchange rate says why:

    mates 19: cost -29.7%, mates -17.4%  ->  ratio 1.71
    mates 20: cost -28.9%, mates -13.0%  ->  ratio 2.22
    mates 20: cost -31.1%, mates -13.0%  ->  ratio 2.38      mean 2.10

**Selling 1% of mates buys ~2% of cost.** Under `found/cost` that is strictly profitable every time
until the guard forbids the next sale — and structurally so, because the marginal mate is the one the
seed works hardest for, so dropping it saves disproportionate cost. **No tolerance value changes this;
the dial sets where selling stops, never whether it pays.**

### 3. There is NO neutral-step path in the code

    if same_play && rate > best_rate { ... champ = c }   // PATH 1: strictly cheaper only
    if same_play                     { ... continue }    // no-op VETO: DISCARDED
    resolved_up = pent_rate - ci95 > 0.5                 // PATH 2: RESOLVED BETTER only

A candidate that plays identically at equal cost — the definition of a neutral step — is thrown away.
(I first read the VETO line as admitting drift; it does not. The comment above PATH 1 records that
promoting on equal rate was a BUG, fixed.) `ladder_valley_RESULT.md` measures the nearest known rung
at **~59 nodes of neutral-or-worse territory**. A search that discards every neutral step cannot cross
that, whatever the selection rule.

### 4. Exactly one rule accepts a neutral step, and no arm was running it

    None if veto_only => gsc.pent_rate() + gsc.ci95() >= 0.5,   // accepts TIES
    None              => gsc.pent_rate() - gsc.ci95() >  0.5,   // shipped: needs BETTER

`EXISTENCE_GATE_VETO` accepts everything not resolved worse. **It accepts 70 of the 78.** Checked from
`/proc/PID/environ`: it was set in none of the nine arms running. `gate_gateveto` is now running it
against `gate_diversity_PAIRED_off` — same binary, args, seed and gate, flag the only difference.

### 5. `HARD_FITNESS` was retired on a set-dependent premise

Retired here with *"`hard 0-0`, so HARD_FITNESS has not engaged"*. Measured by composition: 2% nonzero
on 4+10+10, but **45-62% on 12+6+5**. The premise holds where it was measured and fails on the other
set. `harder_set` is the only construction that rewards searching BETTER (seed scores 0/8 by
construction), candidates already solve 1-2 of those positions, and the surrogate discards it.

### 6. Two measurement corrections that invalidated three of my own readings today

* **Seed pairs are error bars.** `composition_s1` vs `s2` — identical config, different seed — differ
  by **2.41 se** on `distinct` and **3.72 se** on gate rate. Trajectories are entirely seed-dependent;
  outcomes are not. Any single-arm rate comparison must be checked against this floor first.
* **Gate rates are autocorrelated.** `acf1 = +0.50` for MCTS: 24 gate calls carry the information of 8,
  so nominal intervals are too narrow by **x1.7**. Correcting both a candidate effect (+3.89 -> +2.26
  se) and the noise floor (+3.72 -> +2.80 se) showed the effect was *below* the floor.

**Counts survived both corrections; rates did not.** Every conclusion above rests on counts.

### What this changes about this file's thesis

The 2026-09-08 account said the fitness cannot discriminate, and that stands. What is added is that
**even a fitness that did discriminate would not help while the acceptance rule requires RESOLVED
BETTER and the gate resolves 0 of 78.** The selection rule chooses which non-improvement to spend games
on. `GATE_VETO` is the first mechanism tested that changes what can be accepted at all rather than what
gets ranked first.
