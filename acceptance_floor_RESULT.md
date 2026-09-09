# The gate demands an edge 2.7× larger than a generation produces — and the experiment that cleared it used a broken ruler

2026-09-08. Found by reading the acceptance rule and then reading the run that was supposed to have
tested it. No new compute; every number below is already in the tree or falls out of numbers that are.

## The rule, and the arithmetic

```rust
// main.rs:675
let resolved_up = sc.pent_rate() - sc.ci95() > 0.5;
```

**Acceptance requires an edge strictly larger than the gate's own ci95.** With the measured
pentanomial pair sd of 0.2362:

| | value | cross-check |
|---|---|---|
| gate 224 pairs → ci95 | **0.0309** | main.rs:733 records median **0.031** ✓ |
| a real per-generation edge | **0.0114** | needs 1,649 pairs; STATE.md records **~1,650** ✓ |

Both derivations reproduce numbers recorded independently elsewhere in the tree, so the model is
not being fitted to its own conclusion.

> **The shipped gate demands an edge 2.7× larger than a generation actually produces.**

## The premise is already written down — with only half its consequence

main.rs:757, justifying why the *anchor* gate should merely veto rather than demand improvement:

> "at 224 pairs its ci95 is ~0.031, and real steps are far smaller than that"

That sentence is equally true of the **champion** gate, where it is fatal rather than cautionary.
That consequence is drawn nowhere. The observation was used to soften a secondary check while the
primary check kept the same floor.

**And the anchor cannot rescue it.** main.rs:752 is explicit: the anchor gate is "applied AFTER the
champion match, and only to candidates that already passed it, so it can only ever veto. It cannot
promote anything the champion gate rejected." No `--anchor-pairs` setting reaches a candidate the
0.031 floor already killed.

## What the floor rejects

Compression makes it worse — the champion gate reports **+0.036 ± 0.016** for a pair the frozen
origin puts at **+0.112 ± 0.033**, ~3×. But the conclusion that matters needs no compression
estimate at all:

| lever (frozen-origin scale) | vs 0.0309 face-value bar | vs 0.096 compressed bar |
|---|---|---|
| blend 0.25→0.75 | +0.112 passes | passes |
| draw filter | +0.086 passes | **rejected** |
| horizon | +0.064 passes | **rejected** |
| **datagen depth** | **+0.025 REJECTED** | **rejected** |

**Datagen depth is the only surviving ceiling candidate, and the loop cannot accept it even taking
the gate wholly at face value.**

## The experiment that already tested this returned a false negative

`batch_ab.sh` asked exactly this question and answered "learning failure — every batch ROLLED
BACK". **That answer is withdrawn.** Three independent defects, found by reading the run:

**1. The instrument was the broken one.** `bg_5.log` printed

```
batch gate g5 (last 5 gens): 0.500+/-0.007 LLR +0.00 -> ROLL BACK
```

a single head-to-head rate against the net the batch started from. The current source prints
`champ-vs-origin ... base ... increment` (main.rs:873) because the batch gate was rewritten to
measure the increment over a fixed anchor in `bae8c7b` at **19:18**. The binary that produced that
log was built at **17:13** and ran at **17:22** — two hours earlier. Verified by *content*, not
mtime: `strings` finds no `champ-vs-origin` in that binary and one occurrence in the current one.

And `0.500 ± 0.007` is not a null — it is the blind gate's signature, the same tight-interval 0.500
two random movers produce.

**2. The arms were unequal.** Time-boxed at 1800s each, it ran **11 generations at K=5 and 5 at
K=1**, because the K=1 arm stops for a 224-pair match every generation. The uncontrolled variable
favoured the batch arm.

**3. The negative was pre-registered as expected.** `batch_ab.sh:29`: "Given 0.5024 I expect this
outcome." An expectation confirmed by a later-condemned instrument is the cheapest kind of wrong
answer to get, and it closed the gate as a suspect for the rest of the day.

## Queued

`batch_ab2.sh` — K=5 vs 1, **K deliberately unchanged** so the instrument is the only difference:
the fixed anchor-increment gate, arms matched on generations, and a verdict block that **refuses to
compare unequal arms**. It hard-refuses to run on a binary lacking `champ-vs-origin`, which is
verified by behaviour: pointed at the old build, it exits with a message instead of reproducing the
withdrawn result.

## What this does NOT claim

* **Not that the loop learns.** It claims the loop's filter cannot see a real step, which is a
  different and weaker statement. The batch re-run is what distinguishes them.
* The 3× compression rests on **one** pair of arms. Every conclusion above that depends on it is
  marked; the depth conclusion deliberately does not.
* `--gate-every 5` auto-accepts four generations between checks. If a bad generation lands mid-batch
  the rollback discards the good ones with it — a real cost, not a free win.
