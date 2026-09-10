# Accepts sit just above 0.5 at depth 4 — and the call is 0.0006 from resolving. Not a null.

**2026-09-10.** The complement to `reject_holdout_RESULT.md`. That measured what the gate THROWS
AWAY; this measures what it KEEPS. Together they decide whether the loop's accept/reject decision
carries any depth-4 signal, which is the standing explanation for the flat ancestor readings.

## The measurement

21 accepted candidates, each replayed against **the champion it beat at the gate**, at depth 4,
32 pairs each, 1344 games.

| | accepts | rejects (for comparison) |
|---|---|---|
| n | 21 | 34 |
| candidate-level mean | **0.5179** | 0.4982 |
| sd across candidates | 0.0483 | 0.0401 |
| pooled rate | **0.5179 ± 0.0185** | 0.4982 ± 0.0135 |
| pooled interval | **[0.4994, 0.5364]** | [0.4847, 0.5117] |

## The verdict the tool printed was too strong, and I am not banking it

`reject_audit` fired its "indistinguishable from 0.5 at a TIGHT interval → the champion is on a
RANDOM WALK → that would explain the plateau completely" branch. That branch triggers whenever the
lower bound falls below 0.5 **by any margin**, and here the margin is:

```
lower bound 0.4994   →   0.0006 below 0.5   →   3% of one interval width
```

**Three per cent of an interval is not a null.** At this effect size roughly **28 candidates would
RESOLVE the pool above 0.5**, and there are 21. The data is seven candidates short of establishing
the *opposite* conclusion, and the printed paragraph would have been quoted as a finding.

The tool now discloses this: it prints the margin, what fraction of an interval it is, and how many
candidates would resolve the call — the same boundary disclosure added to `surrogate_validity.py`
earlier today after its verdict cleared a self-chosen threshold by 0.012.

## ⚠ The boundary disclosure is NOT yet verified, and I am saying so

I added the margin-disclosure to the tight branch and then ran a control — but the control used
4 pairs per candidate, giving ci95 = 0.0525, which takes the **UNRESOLVED** branch instead. That
branch behaved correctly (it printed "Ignorance, not a null. Need roughly 716 pairs total"), so the
run validated something, just not the thing I changed.

**The tight-branch disclosure has therefore never executed.** It will be exercised by the n≈30 re-run
at 32 pairs, which is where this question resolves anyway. Until then the code is written and
unproven, which is a different state from working, and this project has been bitten by exactly that
twice today — a committed rc=75 branch that was never in the running process, and a recipe option the
engine ignored.

Also worth noting from that control: at 4 pairs per candidate the same 26 accepts read **0.4639 ±
0.0525** against 0.5179 ± 0.0185 at 32 pairs. Not a contradiction — sd across candidates rises from
0.0483 to 0.1277 when each is measured with 8× fewer games — but a useful reminder that the accept
estimate is unstable until the per-candidate match is wide enough.

## The sharper test: do accepts differ from rejects?

"The gate discriminates" means the two groups separate. Both were measured with the same
instrument, same depth, same pairs per candidate:

```
difference  +0.0197     se 0.0126     t = 1.57
95% CI      [-0.0050, +0.0444]   →  contains zero
```

**Not resolved.** The gate's decision does not demonstrably separate the two populations at depth 4
— but the point estimate is in the right direction, and the interval is wide enough that a real
effect of ~+0.02 is entirely compatible with it.

## What can honestly be said today

* Accepts are **not** demonstrably stronger than what they replaced at depth 4 — and they are **not**
  demonstrably equal either. The interval will not yet separate 0.5179 from 0.5.
* Rejects **are** tight around 0.5 (n=34, ±0.0135), so the gate discards nothing.
* The strong random-walk story — accepts carry no depth-4 signal — is **not established**. It is one
  of two readings still alive, and the other is a small real gain of order +0.02 per accept.
* At ~7% accepts over 400 generations that is ~28 accepts, and +0.02 each does not compound linearly,
  so neither reading contradicts the ancestor control's flat 400-generation windows.

## The next step, which is cheap

P1 is still banking accept samples at ~20/hour. **Re-run at n≈30**, which the arithmetic above says
is where this resolves either way. No new code, no new configuration — the trainer is already
writing the pairs.
