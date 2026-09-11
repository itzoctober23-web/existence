# Quadrupling the data per generation changes nothing — 0.4642 against 0.4684

**2026-09-11.** `--games 8` has been the default for every measurement this project has taken, and no
file had ever tested it. It is not the constraint.

## The measurement

Two arms, identical in every respect except `--games`: same champion, same seed (20260910), same
`--gate-every 5`, `--depth 3`, `--epochs 3`, `--gate-pairs 224`, same machine. Each batch decision is
a **direct champion-vs-base match** — unsaturated, paired, one measurement rather than a difference
of two.

| arm | n | mean batch score | 95% CI | batches below 0.5 | rate |
|---|---|---|---|---|---|
| `--games 8` (control) | 20 | **0.4684** | [0.4525, 0.4843] | 16/20 | ~6.7 gen/min |
| `--games 32` | 20 | **0.4642** | [0.4468, 0.4817] | **16/20** | **4.7 gen/min** |

The intervals overlap almost completely and both arms put exactly 16 of 20 batches below parity.
**Four times the self-play data per generation produces no measurable change in what a generation is
worth**, and costs ~30% of the generation rate to produce.

That is the third pre-registered reading in `games_per_gen.sh`, written before the arm ran:

> *mean is unchanged → volume is not the lever; the negative expectation is intrinsic to the learner
> or the target, and the next suspects are the optimiser and the replay window.*

## Scope — both arms are POST-RESUME, and that is not a detail

Neither arm's batch gate ever kept a batch, so in both the base stayed at the start net. Every
sample in both columns is therefore *"five generations from a freshly resumed champion"*. This
result says **more data does not shrink the resume transient**. It does **not** say more data fails
to help a generation in the steady state — no arm has measured that, and
`nontransitive_walk_RESULT.md` shows the two regimes give different answers (post-resume 0.4684
resolved below parity; steady state straddling it).

Recorded here rather than discovered later because the limitation was written down *before* the arm
finished, which is the fix for reading a partial or mis-scoped result as a general one.

## What it removes from the board

The natural story for a net-negative generation is *"8 games is a tiny, high-variance fresh sample,
fitted three times per generation — of course it overfits."* The arithmetic supports it: at
`--games 8` a generation adds ~800 positions to a pool with a steady state near 2,000 and trains on
~270 of them. It is a good story and it is wrong. At `--games 32` the fresh sample is four times
larger, the pool turns over four times faster, and the batch score does not move.

So the remaining suspects on the generator side are the **target** and the **optimiser**, not the
data volume. `blend` (the target's composition) is already measured and flat across 0.75–1.00;
`epochs` is measured; `horizon` is measured. What has never been touched is the optimiser — learning
rate, momentum, and whether the replay window's *age* distribution matters independently of its size.

## Cost, stated because a quality result is not a throughput result

`--games 32` ran at **4.7 gen/min against ~6.7** for the control on the same loaded box. Since the
batch quality is unchanged, the higher setting is strictly worse per unit wall clock. `--games 8`
stays.
