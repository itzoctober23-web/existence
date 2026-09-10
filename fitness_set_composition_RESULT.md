# The fitness set's discriminating power lives ENTIRELY in its non-mate-in-1 positions

`evolve ttgraft 40 <n1> <n2> <n3> 3` — three runs, the SAME 40 crossover children each time
(crossover is seeded, so the children are identical across runs and this is a paired comparison).
Only the position set changed.

## The measurement

| set | composition | kept all 25 mates | fitter AND correct | degenerate child #30 |
|---|---|---|---|---|
| pure mate-in-1 | 25 / 0 / 0 | 22/40 | **22/40** | 25/25 mates, **2934.933x** |
| mixed (the shipped default) | 15 / 5 / 5 | 0/40 | 0/40 | 19/25 mates, 2274.444x |
| depth-heavy | 5 / 10 / 10 | 1/40 | **1/40** | 11/25 mates, 1306.606x |

`n1` = `mate_set` (mate-in-ONE only), `n2` = `disagreement_set` (depth-requiring), `n3` =
`window_sensitive_set`. All three runs score against the same seed baseline of 25 mates at cost
9,930,290,911.

## What child #30 is, and why it matters

    cost 3,634,500 against the seed's 9,930,290,911  =  0.0366% of the work, a 2,732x reduction
    on pure mate-in-1 : 25/25 mates -> 2934.933x  "a spectacular improvement"
    on mixed          : 19/25 mates -> 2274.444x
    on depth-heavy    : 11/25 mates -> 1306.606x

**It does essentially no search.** A mate-in-one is a one-ply check, so a program that abandons the
search entirely still finds every one of them. This is FITNESS §10's declared degenerate solution —
*"Prune everything / return eval"* — realized exactly, and **the mate-in-1 stratum cannot see it.**
On a mate-1-only set it is the single best program measured in this project by three orders of
magnitude.

## The finding

**The 10 non-mate-in-1 positions in the shipped 25-position set are doing ALL of the discrimination.**
Drop them and 22 of 40 broken programs become "correct and fitter". Add more and the degenerate
child's mate count falls monotonically, 25 -> 19 -> 11.

Read the last column of the table as the real measure of a fitness set: **not how many candidates it
admits, but the best score an obviously-broken program can achieve on it.**

    pure mate-in-1   a null search scores 2934.933x
    mixed            a null search scores 2274.444x
    depth-heavy      a null search scores 1306.606x, and the best CORRECT child scores 1.283x

On the depth-heavy set the only child that passes the correctness guard scores **1.283x** — a
plausible, non-degenerate number in the same range as the hand-built `ab_hash` rung's 1.024x. On the
mate-1 set, 22 children pass with ratios up to 2934x and every one of them is wreckage.

## Why this is a FITNESS result and not a guard result

`ladder_valley_RESULT.md` records that `guard_tolerance` "converts an unreachable rung into a
reachable MATE SALE", and concludes no threshold repairs the ordering because on this fitness the
sale outscores the rung. This says where that bad ordering comes from: **60% of the shipped set is
satisfiable without searching at all.** The surrogate is not mis-weighted; it is being asked to rank
programs using positions that cannot tell a search from a no-op.

FITNESS §3 already specifies the fix and it is not implemented. The spec calls for **500 positions at
each of MATE-1, MATE-2, MATE-3 and MATE-4** — 2,000 stratified, reported per N. `mate_set` builds
**mate-in-ONE only** (it plays random moves and keeps a position where some legal move mates
immediately). So the spec's entire depth ladder is absent, and the only reason the shipped set
discriminates at all is the 10 positions from `disagreement_set` and `window_sensitive_set`, which
are doing the job MATE-2/3/4 was specified to do.

## What this refutes, including my own proposal from earlier tonight

`relaunch_bigset.sh` argued for a bigger set on the grounds that a 23-position set makes
`guard_tolerance` a coarse 17.4% licence, and proposed `n1=48 n2=24 n3=5` — 77 positions, **62%
mate-in-1**. That keeps the dilution almost exactly as it is. The measurement above says set SIZE was
never the axis: **composition is.** A 77-position set that is 62% mate-in-1 buys a finer tolerance
granularity on a surrogate that still cannot see a null search.

The direction is `n1` DOWN and `n2`/`n3` UP, or better, `mate_set` extended to real MATE-2/3/4 as §3
specifies.

## Honest limits

* One depth (3), one net (`Net::random(32, 20260907)`), 40 children drawn from one crossover
  distribution (`ab <- uct`). The MONOTONE trend across three sets is the claim; the specific counts
  are not portable.
* "Depth-heavy" here means more `disagreement_set` and `window_sensitive_set` positions, which is a
  PROXY for §3's MATE-2/3/4 ladder and not the same thing. Building the real ladder is the work this
  result argues for, and it would also let the per-N thresholds (0.9x on MATE-{1,2}, 0.8x on
  MATE-{3,4}) be applied as specified.
* 1 of 40 passing on the depth-heavy set is a small number to reason from; it says the set is
  strict, not that it is correctly calibrated.
