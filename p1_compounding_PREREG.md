# PRE-REGISTRATION — P1 compounding: champion-following datagen at a node budget, into a declared
# rolling window, promoted through the normal gate

Registered 2026-09-11, **before the arm is launched**. His directive: *"That is what every from-zero
pipeline does and what FITNESS specified; it has never been run here... This is the only P1
experiment this week."*

Reporting discipline in force from today: **while this runs it writes to STATE.md only.** A
`_RESULT.md` is written only when the planned N (2,000 generations, both arms) is complete.

---

## What production actually does today — MEASURED, from `p1_production.sh:91` and `main.rs`

```
--init ${TAG}_start.net --gens 1000000 --games 8 --threads 4 --depth 3 --epochs 3
--gate-every 1000000 --arch-every 0 --control-every 0
```

| clause of the directive | production today | verdict |
|---|---|---|
| datagen from the **current champion** | `play_games(&champion, …)` at `main.rs:771`, and `champion = cand` at `:1173` | **ALREADY TRUE in-process.** Not a gap. |
| at a **node/cost budget**, not fixed depth | `--depth 3`; `--datagen-nodes` defaults to **0** (`:185`) — budget OFF | **GAP** |
| into a **rolling window** (last N gens, N declared) | the replay window at `:869` is reached **only if `--steps-per-gen > 0`**, which defaults to **0** (`:311`). Production passes `--epochs 3`, so it takes the `else` at `:871` and trains on **`subset` — this generation only** | **GAP** |
| champion **re-promoted through the normal gate** | `--gate-every 1000000` over a ~12,550-generation run ⇒ the game gate **never fires**. Promotion is decided by the held-out **surrogate** | **GAP** |

**The honest correction to the framing:** the loop *does* compound — datagen follows an improving
champion. What it does **not** do is compound *through the gate*, *at a budget*, or *over a window*.
Three of four clauses are real, and all three are **configuration using flags that already exist and
are already tested**. No new code is required, which is why this can be run now rather than built.

## The arms

Both arms: 2,000 generations, same `--init` (the current champion), same seed, same `--threads`,
same cores, run **sequentially** — the box limit is ONE trainer, and two concurrent trainers is the
failure mode that froze it before.

```
CONTROL  (production, verbatim):
  --depth 3 --epochs 3 --gate-every 1000000

COMPOUND (the structural change):
  --datagen-nodes <B> --steps-per-gen <S> --replay-gens <N> --gate-every <K> --gate-pairs <P>
```

## THE CONSTANTS, MEASURED AND FIXED BEFORE LAUNCH

```
B = --datagen-nodes 10309     S = --steps-per-gen 777
N = --replay-gens 8           K = --gate-every 100      P = --gate-pairs 224
```

**`B = 10,309` — measured, not chosen.** `main.rs:189` carries the project's own measured
single-position midgame costs: `d2 352, d3 10,309, d4 72,977, d5 1,234,802`. Depth 3 costs 10,309
nodes/move, so that budget is compute-matched to the control by construction.

> **What the budget actually does here, stated so it is not overclaimed later.** `main.rs:190` picks
> the largest depth whose measured cost fits the budget and then uses the budget as a **hard cap**.
> It does **not** search until nodes run out, so it will **not** go deeper on cheap positions — depth
> stays 3 until the budget reaches 72,977. The real change is therefore that per-move spend becomes
> **bounded** instead of unbounded-at-depth-3. That is a narrower change than "variable allocation",
> and this clause of the experiment should be read as such.

**`S = 777` — derived from the control, exactly.** `trainer.rs:120` `epoch()` does **one SGD update
per sample** (batch 1), and `steps()` draws `n_steps` samples with replacement then runs one epoch —
so exactly `n_steps` updates. The control does `3 × train_n` updates/generation, and `train_n` over
the last 500 production generations is **mean 259, median 255** ⇒ `3 × 259 ≈ 777`. The compound arm
therefore takes the **same number of gradient steps**, drawn from a **~2,072-sample window instead of
a ~259-sample slice** — an 8× broader draw at identical step count. That is the whole hypothesis.

**`N = 8`** is the existing default, declared as required. Production already *builds* this window
every generation (ledger: `pool` 2072 mean vs `train_n` 259) and then **discards it**, because
`steps_per_gen=0` routes training down the `subset` branch at `main.rs:871`.

**`K = 100, P = 224`.** 20 gate calls over 2,000 generations. `P=224` is the existing measured
default (`main.rs:196`: 40→224 because the gate could not otherwise resolve the improvements it
exists to detect). Gate cost is charged to the compound arm and it is still held to 2,000
generations.

## How each constant was set BEFORE launch (not chosen after)

1. **`B` (datagen node budget).** Set to the **measured median nodes/move of depth-3 datagen on this
   champion**, so the compound arm spends the same search per move on average and the change is
   *allocation*, not *more compute*. `main.rs:192` already prints the budget→depth mapping. Measured,
   not guessed — a budget picked by eye would make any difference unattributable.
2. **`S` (steps per generation).** Set so the compound arm takes the **same number of gradient steps
   per generation** as `--epochs 3` over `subset` does today. This is the whole point: the window
   changes *which* positions the steps draw from, not *how many* steps there are.
3. **`N` (window, generations) = 8**, the existing default, **declared here** as the directive
   requires. Not tuned in this experiment; tuning it is a separate question and changing two things
   at once would make the verdict unattributable.
4. **`K`, `P` (gate cadence and pairs).** The gate costs games. Its cost is **counted against the
   compound arm's compute**, and the arm is still held to 2,000 generations — so the gate is a real
   expense, not a free extra.

## Verdict rule — fixed now

Two outcomes are reported, both pre-specified:

* **Ruler:** pooled rating of each arm's final net on `sf_ruler.py`, with CI, at matched generations.
  A win requires the CIs to be **disjoint**. A slope is a trend only at |z| > 2.
* **Gate:** `netmatch` of COMPOUND's final net vs CONTROL's final net, `rate − ci95 ≥ 0.5`, the
  normal promotion rule. Between-seed sd on this box is **0.047**, so a single pairing inside that
  is not a result.

**Matched on GENERATION count, not wall clock.** A resume costs ~95 Elo before it pays back, and
wall-clock matching has already invalidated one published A/B here.

## What would falsify the directive's premise

If COMPOUND finishes 2,000 generations and its ruler CI **overlaps** CONTROL's *and* the netmatch
sits inside 0.5 ± ci95, then the structural change is not the lever, and `WEEK1_RETRO.md` says so.
That is a real possible outcome and it is registered as such.

## What this experiment does NOT test

Not the window size, not the gate cadence, not the label depth (that lever is spent —
`depth5_vs_depth3_RESULT.md`). One structural change, three settings moved together **because the
directive specifies them together as one pipeline shape**, and the arm is the shape, not any one
knob. If it wins, attributing the win to one of the three is a follow-up, and that is stated here so
it is not claimed later.
