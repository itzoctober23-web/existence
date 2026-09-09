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
