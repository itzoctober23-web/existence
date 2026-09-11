# A resume costs ~95 Elo before it pays back — and it confounds every short experiment

**2026-09-10.** `resume_dip_PREREG.md` proposed a mechanism and named what would kill it. The
mechanism is dead. What the discriminator found instead is larger than what it was built to test.

## The curve

Resumed from the shipped champion, snapshots netmatched against that champion (160 pairs, depth 4,
the paired instrument):

| snapshot | replay pool | score vs champion | interval | Elo |
|---|---|---|---|---|
| **control** — champion vs itself | — | **0.500 ± 0.009** | [0.491, 0.509] | 0 |
| gen 5 | 1,411 (72% full) | **0.492 ± 0.036** | [0.456, 0.528] | −5.6 |
| gen 25 | 1,939 (99%) | **0.411 ± 0.034** | [0.377, 0.445] | −62.5 |
| gen 100 | 1,967 (100%) | **0.366 ± 0.035** | [0.330, 0.401] | **−95.4** |
| *d3c, same setup* | full | *0.557 ± 0.032* | *[0.525, 0.589]* | *+39.8* |

The control returned **exactly 0.500** with a ±0.009 interval, so the harness is not reading low.

**Independently replicated:** `auto_promote` measured the unrelated production run at gen 108 as
**0.362 ± 0.031** — a different run, a different pair count, a different script — against this run's
**0.366 ± 0.035** at gen 100. Two harnesses, agreeing to 0.004.

## The pre-registered mechanism is REFUTED

The proposal was that `--init` restores the net but not the data (`main.rs:676` creates the replay
pool unconditionally empty), so a champion trained on ~2,000 samples is damaged by fine-tuning on
~100–300. The falsifier was written down before any net was measured:

> **What would falsify the mechanism:** a flat ≈0.50 at gen 5 followed by a decline. If the damage
> is done before the pool has refilled, the pool cannot be what did it.

That is precisely what happened. The pool is **72% full at gen 5, where there is no damage**, and
**100% full at gen 100, where the damage is maximal**. A pool that is already full cannot be causing
a decline that arrives later. The story was plausible, tidy, and wrong.

## What is left standing

Training here is **completely unguarded**. `--gate-every 1000000` means the strength gate never
runs: the production log shows `gate 0W-0D-0L` and `ACCEPT` on **5,274 of 5,274 generations —
100%**. Nothing ever tests a candidate for strength, so the loop follows its surrogate (self-play
loss) wherever it goes, and for the first ~100 generations after a resume that direction is away
from the inherited champion. This is not proven to be the cause; it is the candidate the data has
not killed. The next discriminator is a run with the gate ENABLED — if the dip survives a live
strength gate, unguarded acceptance is not it either.

## The part that matters most: this CONFOUNDS `depth5_vs_depth3_RESULT.md`

That result, published earlier today, compared two arms at equal wall clock:

* **d5** — 45 generations, **0.397** against its start → "LOSES"
* **d3c** — 4,327 generations, **0.557** → "WINS"

and concluded *"the datagen-depth lever is not monotone … the knee is at or below 3."* But the two
arms sat at wildly different points on the curve above, and the curve has nothing to do with label
depth. A **depth-3** run from the same champion reads 0.411 at gen 25 and 0.366 at gen 100, so at
d5's 45 generations it interpolates to **~0.39–0.40**.

**d5 measured 0.397.** That is indistinguishable from where a depth-3 arm sits at the same generation
count. The A/B cannot separate *"depth-5 labels are worse"* from *"depth 5 completed too few
generations to escape the resume transient"*.

**What survives, and it is still useful:** the practitioner's question was *given 2,400 seconds from
this champion, which depth do I pick?* — and the answer is unchanged, because at the END of that
budget depth 3 is at +39.8 Elo and depth 5 at −79.8. **Do not use depth 5 at this budget.** What does
NOT survive is the inference about label QUALITY, and with it "the knee is at or below 3". Depth-5
labels have not been shown to be worse; they have been shown to be too slow to reach the part of the
curve where they could show anything. `depth5_vs_depth3_RESULT.md` is corrected in place.

## The methodological consequence, which is bigger than either result

**Any experiment that resumes from a champion and measures before ~1,000 generations is reading the
resume transient, not its treatment.** The transient is ~95 Elo deep at gen 100 — larger than most
effects this project chases, and it points the same way (down) regardless of the treatment. Two arms
that complete different numbers of generations are at different depths in that hole, and the arm
that ran fewer will lose for that reason alone.

Three rules follow, and they cost nothing:

1. **Judge arms at matched GENERATION count**, not only at matched wall clock. Matched wall clock is
   the right question for "which setting do I ship"; it is the wrong one for "which labels are
   better", and the two were conflated.
2. **Resume-and-measure-early is invalid.** Either run past the transient (~1,000+ generations) or
   measure against a *same-generation* control resumed from the same champion — which is exactly
   what this run now provides for depth 3.
3. **A control arm is not optional in a resumed experiment.** The d5/d3c A/B had one and it still
   was not enough, because the control ran a different number of generations. The control must match
   on the axis that carries the confound.

## Method note

The snapshot poller had a bug worth recording: `grep -c` prints `0` **and exits 1** when it matches
nothing, so `|| echo 0` emitted two zeros and `[ "0\n0" -ge N ]` died with "integer expected".
Verified it did not corrupt the result — all four nets are distinct, so each snapshot came from its
intended generation.

This file also corrects its own PREREG: that file described the champion as fine-tuned on "~100–300"
samples, true of generation 1 and misleading about the rest, since the pool is 72% full by gen 5. A
sharper test of the (now dead) pool mechanism would have snapshotted at generation 1–2.
