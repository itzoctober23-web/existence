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
