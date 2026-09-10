# The depth-1 gain is resolved; at depth 4 nothing is, and two instruments disagree in sign

**2026-09-10.** A matched depth comparison on r9's own rungs — `gen400` → `gen800`, exactly 400
generations inside one run, so the labels mean what they say. Both arms are `netmatch`, 448 pairs,
seed 20260907, the same two files, with **only the depth argument differing.**

| depth | `gen400` scores | ⇒ `gen800` | interval on gen800 | verdict |
|---|---|---|---|---|
| **1** — what the gate selects on | 0.467 ± 0.019 | **0.533** | [0.514, 0.553] | **RESOLVED better** |
| **4** — what strength is judged at | 0.516 ± 0.021 | **0.484** | [0.463, 0.505] | unresolved, point estimate below 0.5 |

The depth-1 gain of **+0.033 is resolved**. At depth 4 the same 400 generations produce **−0.016,
unresolved** — and, importantly, the depth-4 interval **excludes the depth-1 value**: gen800 cannot
be at 0.533 in the judged game.

## The window correction does not rescue it

`depth_transfer_window_note.md`, committed before this ran, established that this morning's +0.139
came from a ~1,470-generation window against this one's 400, and that linear scaling predicts
**+0.038** here. Observed: **−0.016**. So the shortfall is *not* explained by window size — the
scaled expectation and the measurement are on opposite sides of zero.

## But a second depth-4 instrument disagrees, and I am not choosing between them

The ancestor control measured the *same pair* at depth 4 and got the opposite sign:

```
netmatch,  FIXED depth 4,  448 pairs   gen800 = 0.484   [0.463, 0.505]
ancestor,  depth CAP 4, EQUAL-TIME,  400 pairs   gen800 = 0.521   [0.487, 0.555]
overlap [0.487, 0.505] — compatible, but straddling 0.5 in opposite directions
```

**Protocol does not explain it.** Equal-time derived caps of 6953 vs 6915 nodes — within 0.5% — so
the two arms gave the nets essentially equal work anyway. Different seeds and pair counts plausibly
account for a 0.037 gap between two unresolved readings.

**So the honest position is narrower than "gains do not transfer":**

* At the **gate's** depth the loop is measurably improving: +0.033, resolved, twice now.
* At the **judged** depth, over this window, **nothing is resolved in either direction**, and the two
  available instruments differ in sign.
* What IS excluded at depth 4 is that gen800 carries the depth-1 sized gain — [0.463, 0.505] does not
  reach 0.533.

That last point is the one with teeth: **the improvement the gate sees is larger than anything
present at depth 4.** Whether the true depth-4 effect is a small positive, zero, or slightly negative
is not settled by 448 pairs.

## What would settle it

Not more depth-4 pairs on this single window — 448 already gives ±0.021 and the effect being argued
about is smaller than that. The efficient test is the one already running: the **accept audit at
n≈30**, which measures 30 independent accept decisions at depth 4 rather than one 400-generation
window, and therefore has 30 chances to see a per-accept effect instead of one shot at its sum.

## Method note

Both depth arms come from one script invoked twice with one changed argument, because the failure
this repo has recorded three times today is comparing numbers produced under different conditions.
The ancestor control's reading is quoted here as a *disagreement to be explained*, not averaged in —
pooling two instruments would manufacture a false precision from exactly the mismatch that makes
them interesting.
