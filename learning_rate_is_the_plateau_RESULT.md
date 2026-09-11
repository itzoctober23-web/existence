# The plateau was the learning rate — 0.692 against 0.499, on a knob never once varied

**2026-09-11.** The first movement on this project's plateau. The control reproduces the plateau
exactly; the treatment leaves it.

## The measurement

Both arms resumed from the **same** start (a snapshot of the production run at generation 10,874),
matched on **generations** rather than wall clock, same seed (20260911), same `--games 8`,
`--depth 3`, `--epochs 3`, `--gate-every 1000000`. The only difference is `--lr`.

| arm | lr | generations | vs the shared start | interval |
|---|---|---|---|---|
| **A** (control) | 0.01 (shipped) | 2,000 | **0.499 ± 0.030** | [0.469, 0.529] |
| **B** | **0.002** | 2,000 | **0.692 ± 0.028** | **[0.664, 0.720]** |

**The intervals are completely disjoint**, difference +0.193 in score. `netmatch`'s between-seed sd
is 0.047 and this effect is ~4× that — unlike several marginal results tonight, this is not a place
where one seed is a lottery.

### The control is the load-bearing part

Arm A read **0.499 ± 0.030**. The plateau was independently measured hours earlier, on a different
run and a different pair of snapshots, at **0.499 ± 0.030** over 1,985 generations
(`nontransitive_walk_RESULT.md`). Three decimal places, same interval.

That is what makes the treatment reading trustworthy. The pre-registration named this as the row to
check *first*:

> **control itself clears 0.5** → prod2's 0.499 did not replicate and the plateau claim needs
> re-examining before anything is concluded about lr.

It did not clear 0.5. It landed on the plateau, exactly.

## Why this knob and not another

`lr` was a hardcoded literal `0.01` inside `main.rs`, not reachable from the command line — the one
generator knob this project has never varied. Everything adjacent is measured:

| knob | status |
|---|---|
| label depth | spent at 3 (`datagen_depth`, `depth5_vs_depth3`) |
| blend | flat across 0.75–1.00 (`blend_RESULT`) |
| epochs | under-fitting refuted (`epochs_ab_RESULT`) |
| horizon | cap obsolete past bootstrap (`horizon_RESULT`) |
| games/generation | 4× changes nothing (`games_per_gen_RESULT`, measured tonight) |
| **learning rate** | **never tested** |

The prediction was mechanical, not a hunch. `trainer.rs` is plain SGD — `Trainer { lr, blend }`, no
momentum, no decay, no schedule. A **constant** step size keeps displacing the weights by the same
amount however close to a basin they are, so the net changes every generation and arrives nowhere.
That is precisely the measured plateau signature: +28.6 Elo to generation 4,818, **−0.7 Elo over the
next 1,985**, and 5-generation steps at 0.4869 [0.4364, 0.5374] — indistinguishable from a coin flip.

## Independent corroboration from a different instrument

`auto_promote` measures live arms against the **champion** — a different reference from the A/B's
shared start — on its own 30-minute schedule:

```text
  lrA.net gen  863: REGRESSION  0.430 +/- 0.032    (lr 0.01)
  lrB.net gen 1392: PASSES      0.593 +/- 0.029    (lr 0.002)
```

Same direction, different reference, different pair counts, collected without knowing about this
experiment. (Those two readings are at *different* generation counts, so they do not constitute a
matched comparison — they corroborate the direction, and the matched verdict above is the result.)

`auto_promote` correctly declined to promote, because two arms were training and the candidate would
have been ambiguous — the fix made to it earlier tonight, working on its first real opportunity.

## What is claimed, and what is not

**Claimed:** at a fixed 2,000-generation budget from this start, `lr 0.002` scores **0.692 ± 0.028**
against that start while the shipped `lr 0.01` scores 0.499 ± 0.030. That is a measurement, not a
ship.

**Not claimed:** an Elo number for the engine. The score converts to ~+140 Elo, but nothing has
passed a formal gate, so that figure is a conversion of this match and not a property of a shipped
champion.

**Not claimed:** that 0.002 is optimal. Two points do not locate a minimum. The obvious follow-ups
are a sweep (0.005 / 0.002 / 0.001 / 0.0005) and a **decay schedule**, which is what the mechanism
above actually argues for — a large step early and a small one late, rather than a small step
throughout. A fixed lower rate may simply be trading early progress for late convergence.

**~~Not replicated.~~ REPLICATED 2026-09-11 02:15** (`lr_sweep_RESULT.md`): a fresh seed (20260912) and a *different start* — the new champion, already trained at 0.002 — gave lr 0.01 → 0.358 ± 0.026 against lr 0.002 → 0.544 ± 0.025. That also kills the rival reading that the effect was only recovery from a net plateaued AT 0.01. Corroborated by a second *instrument* too (`static_deep_residual --ref`), which predicted the full ordering before the matches reported. Originally:  One seed. The effect is ~4× the between-seed sd, which is far stronger than
anything else measured tonight, but a second seed is cheap and should be run before this is treated
as settled.

## Method note

The flag was added with the default left at 0.01 so every prior measurement stays byte-identical,
and the run header now **prints** `lr` — `lr_ab.sh` aborts unless the header matches the requested
value. A flag that does not appear in the log is one nobody can verify took, and this project
retracted a published finding today for exactly that: an environment variable set on a process that
never read it.


## The mechanism, measured in weight space — displacement without progress

The explanation above ("a constant step keeps displacing the weights however close to a basin they
are") was a story about what SGD does. It is now a measurement, and it needs no games: both arms ran
2,000 generations from the same start, so their L2 distance from that start is directly comparable.

| arm | L2 displacement from the shared start | strength vs that start |
|---|---|---|
| lr 0.01 (control) | **8.595** | 0.499 |
| lr 0.002 | **4.973** | **0.692** |

**The control travelled 1.73× farther through weight space and arrived nowhere.** That is the
random-walk signature stated quantitatively: a larger step is not doing more learning, it is doing
more wandering.

### The ratio is NOT 5×, and that is worth stating

The learning rates differ by 5×. For a pure random walk (displacement ∝ step × √n) and for directed
motion (∝ step × n) alike, equal generation counts should give ~5× more displacement for the larger
step. Measured: **1.73×**. The low-lr arm moved far more than proportionally.

~~The likely cause is in `train_from()`: it early-stops on held-out loss with patience 3...~~

**⚠ THAT EXPLANATION IS WRONG, corrected within the hour.** `train_from()` does early-stop — but
`main.rs:1040` states plainly that **it is called ONLY by the ARCH arm**, which `--arch-every 0`
disables in both of these runs. The NET arm trains with a fixed count:

```rust
for e in 0..epochs { loss = tr.epoch(&mut cand, subset, ...); }   // main.rs:814
```

So both arms took **exactly the same number of weight updates** — 3 epochs × 2,000 generations — and
the sub-proportional displacement is **unexplained**.

It is consistent with the large-step arm having SATURATED inside a basin (its spread bounded by the
basin width) while the small-step arm is still moving directedly. That is a hypothesis and it is
recorded as one; substituting a second story for a refuted first one is how this file would stop
being evidence.

**The correction is the same mistake the code comment beside it warns about**, three lines up:

> Second time today I asserted which code path produced a value without following it; the name stays
> literal now.

I made it a third time, in the same file, about the same function — and this time in something
published. The check that catches it costs one grep: *before naming a function as the cause, confirm
the code path being measured actually calls it.*

What survives regardless of the explanation: **the arm that moved farther gained nothing, and the
arm that moved less gained.**
