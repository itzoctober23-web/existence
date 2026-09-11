# The fix for the plateau decides on the instrument that was condemned the same day

**2026-09-10.** `acceptance_floor_RESULT.md` proposed batch gating as the answer to the P1 gate's
floor. Batch gating decides using the frozen-origin metric. `instrument_saturation_RESULT.md`, dated
the **same day**, showed that metric saturates and reverses signs — and the batch gate operates
entirely inside the band where it does.

## The proposed fix

`acceptance_floor_RESULT.md` established the arithmetic and it is not in dispute:

| | value |
|---|---|
| acceptance rule | `pent_rate − ci95 > 0.5` |
| gate ci95 at 224 pairs | **0.0309** |
| a real per-generation edge | **0.0114** |

> **The shipped gate demands an edge 2.7× larger than a generation actually produces.**

Its answer was `--gate-every K`: auto-accept K−1 generations, then test the accumulated batch, so
the edge under test is ~K× larger while the gate's ci95 is unchanged. Sound reasoning. The
experiment was designed (`batch_ab2.sh`, 09-08) and **never concluded** — there is no batch RESULT
file.

## What the batch gate actually measures

Run today at K=5 from the current champion, matched on generations to the ungated arm:

```text
batch gate g5  champ-vs-origin 0.963+/-0.012  base 0.960+/-0.014  increment +0.003+/-0.018 -> ROLL BACK
batch gate g10 champ-vs-origin 0.970+/-0.011  base 0.960+/-0.014  increment +0.010+/-0.017 -> ROLL BACK
batch gate g20 champ-vs-origin 0.972+/-0.011  base 0.960+/-0.014  increment +0.012+/-0.018 -> ROLL BACK
batch gate g25 champ-vs-origin 0.960+/-0.013  base 0.960+/-0.014  increment +0.000+/-0.019 -> ROLL BACK
batch gate g30 champ-vs-origin 0.961+/-0.014  base 0.960+/-0.014  increment +0.001+/-0.020 -> ROLL BACK
batch gate g35 champ-vs-origin 0.948+/-0.016  base 0.960+/-0.014  increment -0.012+/-0.021 -> ROLL BACK
```

**7 decisions, 0 KEEPs.** The rollback is real, not cosmetic — `main.rs:1207` sets `champion = base`
and saves it, so every batch of five generations is discarded. K=5 does not soften the filter; it
freezes the loop and wastes five generations per decision doing it.

## Why it cannot work as built

The decision is `champ-vs-origin − base-vs-origin`: a **difference of two scores against a third
party**, where that third party is the frozen random origin. Measured operating points across all 7
decisions: **0.948 – 0.972, mean 0.961**.

`instrument_saturation_RESULT.md` — dated 2026-09-08, the same day as the file proposing this fix:

> # The frozen-origin metric saturates, and it has now reversed two signs
> | capacity w16 vs w64 (equal gens) | w64 far better, **0.967** vs 0.838 | head-to-head **0.522 ± 0.022 → w16 better** | ✗ **REVERSED** |
>
> What both reversals share is a net near the **top of the scale** … **Beating a random opponent is a
> saturating measurement.**

Its two recorded reversals are at **0.861** and **0.967**. **All 7 batch decisions land at or above
0.861 (100%), and one at 0.972 sits above the 0.967 reversal point.** The batch gate is not near the
saturated region; it lives there.

Two compounding costs, both visible in the numbers above:

1. **No headroom.** Five generations moved champ-vs-origin 0.960 → 0.970. On a scale that tops out
   at 1.0, a real improvement has almost nowhere to go.
2. **Twice the noise, for no reason.** Subtracting two independent measurements gives a combined
   ci95 of **0.0184** on an increment of +0.003 to +0.012. A *direct* champion-vs-base match would
   be one measurement, paired, and centred at 0.5 — full resolution instead of the compressed tail.

## The repair this names

The batch gate should decide with a **direct paired match between the champion and the batch base**,
exactly as `netmatch` does when it reads 0.520 ± 0.038 for the K=1 arm. That is unsaturated, one
measurement rather than a difference of two, and paired. It does not fix the floor — 0.031 at 224
pairs is still wider than a 5-generation edge — but it stops the decision being made on a scale
where the project has already documented the sign flipping.

## Where this leaves the three arms

| filter | adopts | vs its own start |
|---|---|---|
| **K = ∞** (never gate) | everything; mean candidate 0.4931 | 0.366 ± 0.035 at gen 100, recovering to **0.557** by gen 4,327 |
| **K = 5** (batch gate) | **0 of 7 batches** | frozen — the champion is repeatedly reset to base |
| **K = 1** (gate always) | 3 of 43; a 0.518 candidate rejected | 0.520 ± 0.038 at gen 37 |

Both gated settings freeze the loop, by different routes and on different instruments. The only
configuration that has been shown to *go* anywhere is the one that adopts everything and digs a 95
Elo hole first.

## What is NOT claimed

* Not that gating is wrong in principle — that a filter demanding more than a generation produces
  will reject nearly everything is arithmetic, and both `acceptance_floor_RESULT.md` and this file
  agree on it.
* Not that the direct-paired repair works. It is named, not measured. Its floor problem is
  untouched, and a change that only fixes the *scale* may still reject everything.
* K=5 is one setting, one seed, 7 decisions. What is solid is the operating point (0.948–0.972,
  measured) against a saturation band recorded two days earlier — that part needs no more data.


## ⚠ CORRECTION — I published "0 KEEPs" from a RUNNING measurement, for the third time tonight

The section above was written at 7 decisions and reported **"7 decisions, 0 KEEPs"**. The run
continued. At 10 decisions it reads:

```text
batch gate g45  champ-vs-origin 0.981+/-0.010  base 0.960+/-0.014  increment +0.021+/-0.017 -> KEEP
batch gate g50  champ-vs-origin 0.969+/-0.012  base 0.973+/-0.012  increment -0.004+/-0.017 -> ROLL BACK
```

**1 KEEP in 10.** So "K=5 freezes the loop" is too strong: it advances, roughly once per fifty
generations, against K=1's 3 accepts in 45. The batch gate is not inert. It is *very* slow.

Everything about the saturation stands and is now measured over more decisions, not fewer:
**operating points 0.948–0.981, mean 0.963, and 10 of 10 at or above the 0.861 reversal point** —
including the KEEP itself at **0.981**, the most saturated reading in the set, which is precisely
where `instrument_saturation_RESULT.md` says the metric is least trustworthy.

And the KEEP is worth reading closely: increment **+0.021 ± 0.017**, so it cleared its interval by
0.004. On a scale whose top is 1.0 and whose sign has reversed at 0.967, a decision resting on
0.004 of margin at an operating point of 0.981 is not a measurement anyone should bank.

**The error is mine and it is a repeat.** Tonight I also called the 4PC anchor "saturated" from
31-0-0 and retracted it three games later, and read a ruler trend as a 136 Elo collapse that
`netmatch` refuted. `fourpc-truncated-llr-is-noise` exists for exactly this. **A partial result is
not a result** — and the rule applies to a KEEP/ROLL-BACK tally as squarely as to an LLR.
