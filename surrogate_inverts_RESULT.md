# The grammar fitness ranks the STRONGEST reference program LAST — a direct inversion

**2026-09-10.** MASTER_PLAN P2's kill condition is *"no program improves on the seed → grammar or
fitness is wrong; fix those"*. It fired today: two arms, 13h and 7h, **0 accepts across 19 gate
decisions**. This file establishes which of the two is wrong, and by what mechanism.

## The measurement

Five reference programs — deliberately different search paradigms, not one-token mutants — scored
two ways. Same net, same budget on both sides of every game, 24 pairs per match, **0 forfeits in
480 games**.

| program | surrogate (mates/Mcost) | game score |
|---|---|---|
| alpha-beta + hash + **ID** | **0.304226 — WORST of five** | **0.612 — BEST of five** |
| capture extension (rung 6) | 8.694256 | 0.492 |
| bare alpha-beta (seed) | 8.694256 | 0.484 |
| alpha-beta + hash reuse | 8.609310 | 0.484 |
| depth-one (purity seed) | 0.778889 | 0.427 — worst |

Spearman rank correlation: **−0.300**.

## The inversion is not a tie being over-read

Iterative deepening beats every other program head to head, each match its own 24-pair pentanomial:

```
bare alpha-beta  vs  +hash+ID   0.365   (ID wins)
+hash reuse      vs  +hash+ID   0.375   (ID wins)
+hash+ID         vs  capture    0.594   (ID wins)
```

Meanwhile the three programs the surrogate rates HIGHEST — bare, +hash, capture — are
indistinguishable from each other in games: 0.500, 0.500, 0.500 against one another. So the games
separate exactly two things, and the surrogate gets **both of them wrong**: the one program clearly
best is ranked last, and the one clearly worst (depth-one, 0.427) is ranked above it.

## The mechanism, and it is the same one as this morning

`mates/Mcost` is a RATIO. Every program except depth-one solves **60 of 60**, so the numerator is
constant and the ranking is decided entirely by cost:

```
bare alpha-beta    6,901,108
+hash reuse        6,969,200
capture extension  6,901,108
+hash + ID       197,221,900     <- 28.6x the seed
```

Iterative deepening re-searches from depth 1 upward. It costs 28.6× more and finds the same mates,
so the ratio buries it — while in real games that extra work is exactly what makes it strongest.
This is the same defect recorded in `surrogate_rewards_giving_up_RESULT.md`, where a candidate
gained 26% on the surrogate by solving THREE FEWER mates for a 31% cost saving. There it rewarded
giving up accuracy; here it punishes spending for strength. One ratio, both failures.

## What this settles that the correlation study could not

`surrogate_validity.py` measured 134 real gated candidates and found r = +0.0617, CI
[−0.109, +0.229] — no detectable relationship, but ambiguous: mutants may simply be near-identical
to their parents, so a null could mean "nothing to rank" rather than "cannot rank". It also named
its limit — 12-game gates attenuate any true correlation, and more candidates would not fix it.

These five are **not** near-identical, and they were played with enough pairs to separate them. The
surrogate still inverts. **So P2's "fitness is wrong" is established, and no gate width rescues it:
the metric misorders programs whose strength difference is large, structural, and cleanly measured.**

## What it does NOT say

**Not that mates-per-cost is worthless.** It correctly ranks depth-one — which solves 4/60 — below
the programs that solve 60/60. It fails specifically when the mate count SATURATES, and then it is
ranking cost alone. On this set, 4 of 5 programs sit at 60/60.

**Not that the mutation operators are broken**, and not that GRAMMAR 9's fitter-path premise is
refuted. Both remain untested — a search cannot demonstrate a fitter path while being steered by a
metric that inverts.

## The fix this points at — and why it is NOT lexicographic ranking

Ranking mates-first-then-cost was tried on paper this morning and withdrawn: capture extension
solves 18/25 on the hard set, fewer than the seed's 25, so mates-first makes the one program that
scores there permanently unreachable.

What the data actually says is narrower: **the ratio is only meaningful while the numerator can
still move.** At saturation it silently becomes a cost race, and cost-minimisation is not the
objective. A harder position set — where no program scores 60/60 — restores the numerator's
variance and would let the same metric discriminate. That is a change to the SET, not the formula,
and it leaves capture extension reachable.

`mate2_treat.log` and `gate_set_mateheavy.log` already exist in this repo and point the same way.
Testing that is the next step; it is not tested here, and this file claims nothing about it.

## ⚠ TESTED, SAME DAY — the set is NOT the fix. My prediction was refuted.

Pre-registered in the commit that added `--hard`: *"on the hard set the numerator varies, so the
ranking stops being decided by cost alone and Spearman rises well above −0.300 — ideally positive.
REFUTED IF Spearman stays at or below 0."*

**It fell to −0.900** — from −0.300 to almost perfect inversion. On the MATE-2 set:

| program | solved | cost | per Mcost | game score |
|---|---|---|---|---|
| depth-one | 2/40 | 3,463,440 | **0.577 — highest** | **0.427 — worst** |
| bare alpha-beta | 2/40 | 4,630,848 | 0.432 | 0.484 |
| +hash reuse | 2/40 | 4,676,784 | 0.428 | 0.484 |
| capture extension | 2/40 | 4,630,848 | 0.432 | 0.492 |
| **+hash+ID** | **11/40** | 132,222,004 | **0.083 — lowest** | **0.612 — best** |

**The numerator did exactly what I predicted and it did not help.** Variance was restored — ID
solves **11 of 40 while every other program solves 2**, a 5.5× spread where the mate-1 set had none.
The ranking got *worse* anyway.

**The real mechanism, which "saturation" was only a special case of:** the denominator's dynamic
range dwarfs the numerator's. Costs span **3.46M to 132M — 38×**. Accuracy spans **5.5×**. A ratio
of the two is therefore a cost measurement with a rounding error attached, on *any* position set.
Making the set harder raised the accuracy range from 1× to 5.5× and raised the cost range from
28.6× to 38× at the same time, because the programs that solve more do so precisely by searching
more.

**So three proposed repairs are now dead**: lexicographic ranking (bans capture extension),
seed-anchored floors (already in place, not the issue), and a harder set (tested, made it worse).
They failed for one reason — each tried to fix a *ratio* whose denominator carries most of the
variance.

**What the evidence points at now, untested and stated as a lead:** stop dividing. Measure accuracy
**at equal cost** rather than accuracy **per cost** — cap every program to the same cost budget per
position and count what it solves. That is exactly what the GAME gate already does (same net, same
budget both sides), and the game gate ranks these five correctly. It also leaves capture extension
reachable, since it would be compared at equal spend rather than penalised for spending.

## Harness note — the first run of this experiment was worthless and looked perfect

I first passed `cost_per_move = 0`, believing 0 meant "no ceiling". `Interp::cost_cap` defaults to
2e9, and 0 makes `cost >= cost_cap` true on the FIRST node, so every program returned `MOVE_NONE`
on every move. A forfeit is scored `Some(!is_a)` — decisive, not a draw — so playing both colours
gave one win and one loss per pair, and **all ten matches returned exactly 0.500 with zero
variance**. Depth-one "drew" with alpha-beta and the ranking was pure artefact.

`gate.rs:355` had already anticipated this: *"the caller has to be able to see how many of these
happened before believing the score"*, and maintains a `FORFEITS` counter. I had not read it. The
tool now reads it, prints the rate, and **refuses to report any ranking above 20% forfeits** —
verified by re-running the broken configuration, which reports 80 of 80 games forfeited and aborts.
