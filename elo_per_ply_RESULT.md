# One ply of search is worth ~93 Elo. Twelve hundred generations of training bought ~0.

**2026-09-10.** Follows directly from `absolute_ruler_RESULT.md`. Once an external scale existed,
the obvious question was where the missing ~800 Elo lives: in the search or in the eval. This
separates them with one variable.

## The measurement

Same net (`r9.gen1400`), same opponent (SF-1320 @10k nodes), 60 games per point, **only `--depth`
changing**.

| depth | W-D-L | score | Elo vs SF-1320 | gain over previous ply |
|---|---|---|---|---|
| 2 | 1-16-43 | 0.1500 | **−301 ± 86** | — |
| 3 | 6-17-37 | 0.2417 | **−199 ± 81** | **+102** |
| 4 | 14-13-33 | 0.3417 | **−114 ± 81** | **+85** |

**≈93 Elo per ply**, consistent across both steps.

For comparison, from the ruler, at fixed depth 4 across the whole run:

| rung | Elo |
|---|---|
| gen200 | −114 |
| gen600 | −140 |
| gen1000 | −172 |
| gen1400 | −127 |
| **live champion** | **−104 ± 52** |

Pooled: no trend, ~−131. **The entire training history is worth less than one ply.**

## What this settles

**The binding constraint is SEARCH VOLUME, not eval quality.** An extra ply buys ~93 Elo; 1200
generations of learning buys nothing measurable. So effort spent making the eval *smarter* is
competing against a lever worth 93 Elo a step, and losing.

**It also converts the speed question into a strength question.** Depth costs time. If eval were
~2× cheaper the engine would search roughly one ply deeper in the same budget — **+93 Elo, more than
this loop has produced in its entire existence.** That is the case for the f32 → int8/int16
quantisation work, and it is now quantified rather than asserted.

**And it explains the search-track ladder independently.** `surrogate_inverts_RESULT.md` found the
three strongest reference programs are proof-number search and both iterative-deepening variants —
all programs that SEARCH MORE — while the `mates/Mcost` fitness ranks them 9th, 5th and 6th of 10 and
puts the three cheapest on top. The fitness punishes the single property worth 93 Elo a ply. Two
tracks, two instruments, one answer.

## What it does NOT say

**Not that the eval is worthless.** It is the thing being searched with; a random net at depth 8
would not be 1600 Elo. The claim is narrower and about MARGINS: at the current operating point,
one more ply beats anything the learning loop has managed.

**Not that ~93 Elo/ply continues.** Ply value falls as depth rises — this is measured at depths 2-4,
the steep part of the curve. Depth 5 is running. Extrapolating to "9 more plies = 2000 Elo" would be
exactly the linearity assumption that broke a pre-registered prediction earlier today.

## The consequence for what to work on

Stop tuning the accept/reject rule. Its whole effect size — the +0.02 the audits were resolving with
1,344-game matches — is **under one fiftieth of a single ply**. The productive work is anything that
buys depth: cheaper eval (quantisation), a faster interpreter, or a search that spends its nodes
better.
