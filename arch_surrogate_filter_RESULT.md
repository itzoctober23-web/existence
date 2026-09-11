# A third of all ARCH proposals were killed on held-out loss, without ever playing a game

**2026-09-11 02:49.** A measured audit of the architecture arm's veto, prompted by tonight's
repeated finding that training loss and strength move in opposite directions in this pipeline.

## The filter

`main.rs:1333`, inside the ARCH arm:

```rust
// FITNESS 5 filter: "must not be worse than the champion by more than 0.5%".
// Better loss does NOT accept; it only buys the right to spend gate time.
if acand_loss > champ_loss * 1.005 {
    ... Reason::SurrogateFilter ...   // rejected, never reached a gate
}
```

It is a **veto**: a candidate whose held-out loss is more than 0.5% worse than the champion's is
rejected without a single game being played.

## The audit

Every ARCH entry across every ledger in the repo, counted by its recorded `reason`:

| reason | count | share |
|---|---|---|
| **surrogate_filter** (vetoed on loss, never gated) | **33** | **34%** |
| lost_on_games | 24 | 25% |
| lost_on_clock | 21 | 22% |
| **accepted** | **14** | 14% |
| no_evidence | 3 | 3% |
| lost_on_cost | 2 | 2% |
| **total proposals** | **97** | |

**One proposal in three was killed by the surrogate before it could be measured by games.**

## Why that matters now

> **⚠ CORRECTED 2026-09-11 03:5x — "anti-correlated" OVERCLAIMS, and the correct word is
> UNINFORMATIVE.** `proxies_RESULT.md` had already settled this at a scale none of tonight's
> readings approach: the held-out surrogate at **r = −0.095 over 239 paired gate results**, training
> loss at **r = +0.379, CI [−0.249, +0.783], n=12** — and crucially it contains examples pointing
> **both** ways (w64 has the lower loss and LOSES; the draw filter has the lower loss and WINS).
> A signal that fails in both directions is noise with respect to strength, not a reversed predictor.
>
> The specific defect in tonight's version: the arms it compares differ in **LEARNING RATE**, which
> changes how much the net fits per generation *independently* of how good the net is. lr moves both
> columns, so the monotone ordering is expected with no causal link between loss and strength.
> n=3 against n=239 besides. This was a closed question, already indexed in `RESULTS_INDEX.md`.
>
> **What survives:** loss must not be used to select or veto candidates here. That conclusion is
> unchanged — it just rests on `proxies_RESULT.md`, not on these three points.

Three independent instruments tonight put training loss **anti-correlated** with strength:

* `epochs_ab_RESULT.md` — more epochs drove train_loss 0.0417 → 0.0253 while mcnemar_z went
  +0.359 → −0.445.
* `depth5_vs_depth3_RESULT.md` — *better* (deeper) labels scored 0.397 against their own start.
* Tonight's decay arms — the **lowest** learning rate carries the **highest** training loss
  (0.0361 against 0.0231) and the low rate is the one that won the sweep (0.589 against 0.544).

If loss is a reversed proxy for strength here, then a veto on *worse loss* is a veto pointed the
wrong way, and some of those 33 proposals were plausibly the good ones. That is a hypothesis about
the filter, **not** a demonstration that any specific proposal was wrongly killed — ARCH candidate
nets are not retained, so none of the 33 can be re-gated after the fact.

**No change made.** ARCH is off in production (`--arch-every 0`, for reasons `width_clock_RESULT.md`
establishes on wall-clock grounds), so this is latent. Recording it so that anyone re-enabling ARCH
knows its cheapest filter is built on the one quantity this project has repeatedly measured as
misleading.

## Method note — I published the wrong number to myself first

The first count said **"54 proposals, 100% surrogate-vetoed, zero ever accepted"** and I had already
begun writing it up as "ARCH is inert by construction". It was a broken grep: I counted ARCH lines
*containing the word* `surrogate`, and the ledger records a `"surrogate":{...}` field on **every**
ARCH entry regardless of outcome. So the pattern matched all of them and the veto rate came out at
100% by construction.

Counting on `"reason":"surrogate_filter"` instead gives 33 of 97, and **14 accepted** — ARCH is not
inert, it accepts about one proposal in seven.

The tell was available and I nearly walked past it: *a rate of exactly 100% with a round-looking
denominator is a pattern matching itself, not a finding.* Same family as the empty table and the
1.0× cost ratio caught earlier tonight — **a number that is too clean is a broken probe until
proven otherwise.**
