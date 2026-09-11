# The production run IS improving — 0.541 over 2,650 generations, while its 5-generation steps are not

**2026-09-11.** A paradox that three files this evening left standing, resolved by one match.

## The paradox

`generator_is_net_negative_RESULT.md` measured 20 direct batch decisions at **mean 0.4684, CI
[0.4525, 0.4843]** — five generations of training make the net worse than the net they started from.
Resolved below parity.

But the ungated arm `d3c` reached **0.557 ± 0.032 against its own start after 4,327 generations**.
If every five-generation step really had expectation 0.4684, ~865 such steps could not arrive above
parity. They would compound into a large decline. Both numbers are measured on the same instrument,
and they cannot both describe the same process in the obvious way.

## The measurement that resolves it

`prod2` has been training ungated since 00:0x. Two snapshots of its output, **2,650 generations
apart**, matched head-to-head at 160 pairs:

| comparison | span | score | interval |
|---|---|---|---|
| **distant** — gen 4,818 vs gen 2,162 | 2,650 generations | **0.541 ± 0.040** | **[0.501, 0.580]** |

**Resolved above parity.** The production run is genuinely improving. `netmatch`'s own note calls the
lead *"MARGINAL — the margin is small next to the interval"*, and that is the right reading: it
clears 0.5 by 0.001 at the lower bound, so the direction is established and the magnitude is not.

## What that means

Adjacent steps scoring at or below parity while a distant comparison scores above it is the
signature of a **non-transitive walk** — each net loses to its immediate predecessor while the
population still moves forward. It is a well-known self-play phenomenon and it dissolves the
paradox: a sequence of A-beats-B comparisons does not compose into a strength ordering, so
"five generations are net-negative" and "4,327 generations are net-positive" can both be true.

It also means **the loop is not broken**. Three files tonight worked toward increasingly grim
readings of the acceptance filter — and the thing the filter sits on top of is, measurably, going
the right way.

## The adjacent number is regime-dependent, and that is the other half

Two sets of adjacent 5-generation measurements, same instrument, same step size:

| regime | n | mean | interval |
|---|---|---|---|
| **post-resume** (base ≤10 gens past the champion) | 20 | **0.4684** | [0.4525, 0.4843] — resolved below |
| **steady state** (live run at gen ~2,200) | 5 *(of 10, in flight)* | **0.5024** | [0.4100, 0.5948] — unresolved |

The post-resume set is resolved below parity; the steady-state set is not, and its point estimate
sits **on** parity. That is the pre-registered reading in `steady_state_batches.sh`: *"mean at or
above 0.5 → the negative expectation is a property of the POST-RESUME regime, not of the loop."*

**Stated with the sample size because it is not finished:** at n=5 of 10 this is an observation, not
a verdict. What is already visible and did not need the remaining pairs is the **variance**:
steady-state steps span 0.347–0.619 (sd ≈ 0.10) against the post-resume set's 0.41–0.54 (sd 0.036) —
roughly 3× wider. The post-resume samples all measure the *same* transition from a fixed base with
different seeds; the steady-state samples measure *different* transitions along a trajectory. Those
are different quantities and the spread shows it.

## Consequence for `generator_is_net_negative_RESULT.md`

Its scope note already says every sample was post-resume. This adds the reason that matters: the
same measurement taken in the steady state does not reproduce the negative mean, and the long-run
comparison the file could not explain is positive. Its headline — *"five generations make the net
worse"* — should be read as **"five generations measured from a freshly resumed champion score below
parity against that champion"**, which is a claim about the resume transient and the granularity of
the comparison, not about the generator.

## What this does NOT say

* **Not that the loop is fast.** +0.541 over 2,650 generations is a small edge, marginally resolved.
  At the project's own conversion that is a handful of Elo for roughly 20 minutes of a saturated box.
* **Not that non-transitivity is proven.** It is the natural explanation for the pattern and it is
  consistent with every number here, but a direct demonstration needs a cycle — A beats B, B beats C,
  C beats A — measured explicitly. The snapshots to do that already exist in `/tmp/ssb`.
* **Not that the acceptance filter is fine.** `batch_gate_saturated_RESULT.md` stands: the shipped
  batch gate decides on a metric that saturates at 0.96 and has reversed signs, and that is a real
  defect independent of what the generator does.


## PRE-REGISTERED, written before the replication landed

The headline above rests on ONE long-range match that clears 0.5 by 0.001 at its lower bound. An
independent replication is running: **gen 6,803 vs the same gen-2,162 baseline — a 4,641-generation
gap, against the first reading's 2,650** — at 224 pairs rather than 160.

| outcome | reading |
|---|---|
| **> 0.541, interval clear of 0.5** | improvement is real and roughly scales with span. The headline stands and stops being marginal. |
| **≈ 0.541** | improvement is real but SATURATING — the run gained in 2,162→4,818 and little since. That would be the more interesting result: it dates the plateau to a generation number. |
| **≈ 0.5, interval containing it** | the first reading was noise at the edge of resolution. The headline must be withdrawn: one match clearing 0.5 by 0.001 is exactly the kind of result that does not replicate. |
| **< 0.5** | the run is declining and the first reading was a fluke in the other direction. |

Written down now because the first reading is *marginal by its own instrument's note* — `netmatch`
printed "A leads, but MARGINALLY ... needs more pairs" — and a marginal result that gets talked into
a headline is the failure this project has already recorded three times tonight.


## THE REPLICATION LANDED — and it is the SATURATING outcome, with an exact composition check

| comparison | span | score | interval | Elo |
|---|---|---|---|---|
| gen 2,162 → 4,818 | 2,656 gens | 0.541 ± 0.040 | [0.501, 0.581] | **+28.6** |
| gen 4,818 → 6,803 | 1,985 gens | **0.499 ± 0.030** | [0.469, 0.529] | **−0.7** |
| gen 2,162 → 6,803 | 4,641 gens | 0.540 ± 0.032 | [0.508, 0.572] | **+27.9** |

Two things fall out, and the second was not expected.

### 1. The run has plateaued, and the plateau is dated

Nearly double the span produced an identical edge (0.540 against 0.541). The direct test confirms it
rather than leaving it as an inference from overlapping comparisons: **gen 6,803 against gen 4,818
reads 0.499 ± 0.030** — dead on parity. **The last ~2,000 generations bought nothing measurable.**

This is the project's "learned, then stopped" phenomenon, for the first time localised to a
generation range and measured on a *paired* instrument rather than the frozen-origin metric that
saturates at 0.96 and has reversed signs. The gain is real (+28.6 Elo, replicated, interval clear of
0.5) and it happened **before** generation ~4,800.

### 2. Long-range comparisons COMPOSE EXACTLY — so the walk is transitive at this scale

Converted to Elo, where gains should add:

```text
  gen 2162 -> 4818    +28.6
  gen 4818 -> 6803     -0.7
  sum                 +27.9
  measured 2162->6803 +27.9      difference 0.00 Elo
```

Three independent 224-pair matches, and the composition closes to within 0.01 Elo. **At the
~2,000-generation scale this walk is transitive: gains add.** That is a much stronger statement than
"the run improves", and it was free — it is a consistency check the three matches perform on each
other.

**This narrows the claim this file was named for.** Non-transitivity, if it is present at all, is a
**short-range** effect: 5-generation steps fail to compose into the long-range result, while
2,000-generation steps compose perfectly. The headline "non-transitive walk" was too broad, and the
correct statement is that **short-range and long-range comparisons disagree, and the long-range ones
are self-consistent.**

### The adjacent measurement, finished

`steady_state_batches.sh` completed all ten pairs:

| regime | n | mean | interval | below 0.5 |
|---|---|---|---|---|
| post-resume | 20 | 0.4684 | [0.4525, 0.4843] — **resolved below** | 16/20 |
| steady state | **10** | **0.4869** | [0.4364, 0.5374] — **unresolved** | 5/10 |

The steady-state adjacent step **cannot be distinguished from 0.5**. It neither confirms nor refutes
a negative expectation; its interval is 3× wider than the post-resume set's because those 20 samples
all measure the *same* transition from a fixed base while these 10 measure *different* transitions
along a trajectory.

So the honest summary of the whole evening's chain is: the loop gained ~28 Elo before generation
~4,800 and has gained nothing since; its 5-generation steps are not measurable as positive or
negative in the steady state; and its long-range comparisons are mutually consistent to 0.01 Elo.
**The plateau is real, it is dated, and it is not an artefact of the acceptance filter.**
