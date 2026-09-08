# replay_ab.sh — read the arms correctly (noted 2026-09-08, DURING the run)

At 22s/generation, a 420s arm reaches ~19 generations. So the three windows bind very
differently, and the labels are misleading if taken at face value:

| arm | binds from | effective meaning at this budget |
|---|---|---|
| `--replay-gens 2` | generation 3 | a real, tight window (~17 gens of binding) |
| `--replay-gens 8` | generation 9 | a real window (~11 gens of binding) |
| `--replay-gens 32` | generation 33 | **NEVER BINDS — this is the "keep everything" arm** |

That is not a wasted arm. "Keep everything" is exactly what his argument predicts should win:
if the champion is not improving (measured flat at 0.838 / 0.853 / 0.831 across generations
30/60/90), then old positions came from an equally-strong player, the staleness premise fails,
and discarding is pure loss. The unlimited arm tests that directly.

But it must be REPORTED as "unlimited", not as "a 32-generation window". Testing 8 vs 32 as
windows would need arms of ~1500s+ so that 32 actually binds for long enough to matter, which
is a different and more expensive experiment.

WHAT THIS RUN CAN AND CANNOT ANSWER
  CAN: does discarding hurt at all? (2 and 8 vs unlimited)
  CANNOT: what the best finite window is. 8 vs 32 is not tested here, because 32 never engages.

Also unchanged: the loop's 0.151 run-to-run variance means a small difference will not resolve.
A null is NOT RESOLVED at this budget, never no-effect.

## WHICH REGIME THIS TESTS (noted during the run, 2026-09-08)

The arms start from SCRATCH (no --init), so they cover generations 1-19, where the champion is
improving rapidly. His argument is about the PLATEAU regime -- gens 30-90 measured flat at
0.838 / 0.853 / 0.831 -- where old positions came from an equally-strong player and the
staleness premise fails.

These are different regimes, and the distinction cuts in a useful direction:

  EARLY (what this run tests): the champion IS improving fast, so old positions really ARE from
  a weaker player. The conventional staleness argument is at its STRONGEST here. This is the
  HARDEST case for "keep everything".

  PLATEAU (what his argument is about): the champion is not improving, so the staleness premise
  does not apply and discarding should be pure loss.

So a win for the unlimited arm HERE would be strong evidence -- it would mean keeping old data
helps even in the regime where discarding has its best case. A win for the tight window here
proves much less about the plateau, because the two regimes genuinely differ.

FOLLOW-UP OWED either way: rerun with `--init champion_long.net` so every arm starts from the
plateaued champion. That is the regime the argument is actually about. Not done in this run
because the script was already executing and editing a running bash script is how 2811 gate
pairs were destroyed -- bash reads it by byte offset.

## RETRACTED — the confound below does not apply, and the data says the opposite

**I was wrong.** Measured at generation 21/42:

    window 1   6 accepts in 42 generations (14%)   accepted mcnemar z: 2.6 -0.6 2.1 -2.1 2.7 4.1
    window 8   6 accepts in 22 generations (27%)   accepted mcnemar z: 4.8  4.3 0.5  3.1 2.1 2.0

The larger window accepts at TWICE the rate and its accepted candidates score HIGHER on the
surrogate, not lower.

Two mistakes in the reasoning below. First, I argued from ABSOLUTE held-out loss (genuinely
higher for the big-pool arm) but the NET arm's acceptance uses `mcnemar` -- a PAIRED sign test on
the same judge positions. The champion is scored on that identical slice, so a higher absolute
loss does not make the paired comparison worse. Second, the absolute-loss filter I was worried
about (`acand_loss > champ_loss * 1.005`) belongs to the ARCH arm, which this sweep disables with
`--arch-every 0`. The confound cannot occur here at all.

Kept below rather than deleted: the reasoning was published, acted on, and wrong, and the way it
was wrong -- reasoning about a statistic the code does not use on a path the run does not take --
is worth more than a clean file.

---

## (RETRACTED) CONFOUND found mid-run (2026-09-08): the surrogate systematically penalises the larger windows

Observed at generation 12:

    arm 1 (window 1)  train 9747-49151   loss ~0.04
    arm 8 (window 8)  train 44697        loss 0.0718   pool 296393

The larger-window arm trains on 6.6x more data and reaches HIGHER held-out loss. That is not a
failure -- a net fitted to eight generations of varied positions will fit THIS generation's
held-out slice worse than one fitted to that slice alone. It is the ordinary bias/variance
trade, showing up exactly where you would expect.

The problem is what judges it. The sweep runs `--gate-pairs 32`, so ci95 ~0.079 and the
`resolves` test (ci95 < 0.05) is almost never true -- which means the FITNESS 5 surrogate, whose
statistic is held-out loss on this generation's slice, decides most accepts. So the larger-window
arms are penalised by the very quantity that a larger window is expected to raise.

MEASURED FROM THE LEDGER, not inferred from the interval width -- the gate resolved in **0 of 57
generations**:

    rp_1 (window 1)  0/42 could resolve   accepted 6, regression 14, no_evidence 22
    rp_8 (window 8)  0/15 could resolve   accepted 2, regression  4, no_evidence  9

Not "rarely". Never. Every acceptance decision in this sweep came from the surrogate or the
non-regression guard, and `no_evidence` dominates (31 of 57) -- the loop honestly reporting that
its gate saw nothing either way. The strict branch of the acceptance rule did not execute once.

WHAT THIS DOES AND DOES NOT INVALIDATE
  - The FINAL verdict is sound. Each arm's champion is scored against the same frozen origin by
    GAMES (examples/control.rs, uncapped depth 2), which the surrogate cannot touch.
  - The PATH to that champion is biased. If arm 8's genuinely stronger candidates were rejected
    on loss, arm 8 ends with a weaker champion, and the sweep would attribute that to the WINDOW
    when the cause is the surrogate.

So a win for the small window must NOT be read as "history hurts". It could equally be "history
raises single-generation held-out loss, and the current acceptance rule punishes that". A win for
the LARGE window is the cleaner result: it would have happened despite this bias, not because of
it.

THE FIX is already identified and committed for other reasons: gate-pairs 40 -> 224 puts the
expected interval at 0.030, under the 0.05 `resolves` threshold, so the GAMES decide and the
surrogate returns to proposing. Re-running this sweep at 224 pairs would remove the confound
entirely. Not done here because changing the binary mid-sweep gives the arms different code,
which is the confound that already invalidated one experiment today.

## MEASURED: a larger window costs NOTHING in wall-clock

    window 1   42 gens   median 27s/gen   final pool  54,621
    window 8   17 gens   median 27s/gen   final pool 372,736

A 6.8x larger training pool runs at the SAME cost per generation, because `--steps-per-gen
50000` fixes the number of gradient steps regardless of pool size. The pool determines what those
50,000 steps sample FROM, not how many there are.

(The generation-count difference is not a slowdown -- arm 8 was still mid-run when this was
measured. Checked before concluding, because "fewer generations" and "slower generations" look
identical in a progress line and mean opposite things.)

This changes the economics of the question. Keeping history is not a trade against speed: if it
helps at all, it is free. The only cost is memory for the buffer, which at ~370k samples is
negligible on this box.

It also means the DEFAULT is the expensive choice in the way that matters. The loop currently
generates ~250k positions per generation and trains on ~50k of them, discarding the rest -- and
the measurement above shows retaining them would not have cost a single second.

## POWER: what this sweep can and cannot detect (computed BEFORE the result, 2026-09-08)

Final scoring is `control.rs --pairs 64`, whose measured ci95 runs 0.038-0.060. Two arms differ
significantly only if their gap exceeds roughly `1.41 x ci95`:

    ci95 0.038  ->  gap must exceed 0.054
    ci95 0.060  ->  gap must exceed 0.085   (~107 Elo at the 0.83 level)

**So this sweep can only detect an effect large enough to be obvious without measuring.** A real
+20 or +40 Elo benefit from keeping history will come back as "no difference", and reporting that
as evidence against the idea would be wrong.

I sized the arms by DURATION (1200s each) and never asked what the scoring step could resolve.
That is the same error as the gate-pairs finding, made one level up: an instrument with a
detection floor far above the effect being sought.

Also, the accept-rate gap that currently looks striking is NOT significant:

    window 1  6/42 = 0.143
    window 8  8/28 = 0.286
    difference +0.143 +/- 0.198, z = 1.41

**HOW TO READ THE RESULT, fixed in advance:**
  - gap > 0.085          -> real, report it
  - gap 0.054 to 0.085   -> suggestive only if both intervals are at the tight end; say so
  - gap < 0.054          -> NOT RESOLVED. Not "no effect". The instrument cannot see it.

To actually resolve a ~+20 Elo difference the scoring step needs ~(0.06/0.015)^2 = 16x the pairs,
i.e. ~1000 pairs per arm. That is a cheap fix -- scoring is a one-off match, not per-generation --
and it is the obvious follow-up regardless of how this run reads.

## ARMS 1 AND 8 COMPLETE — equal generations, so the larger pool is provably free

    window 1   42 generations,  6 accepted (14.3%)   final pool ~55,000
    window 8   42 generations, 11 accepted (26.2%)   final pool ~370,000

**Both arms reached EXACTLY 42 generations in the same wall-clock.** That is stronger than the
earlier median-time measurement: 6.8x the training data cost zero extra seconds, end to end, not
merely a similar per-generation median. `--steps-per-gen 50000` fixes the gradient-step count, so
the pool decides what those steps sample FROM, never how many there are.

The accept-rate gap is NOT significant: +0.119 +/- 0.170, z = 1.37, p ~ 0.17. Suggestive only.
With 42 generations per arm this design cannot resolve an accept-rate difference smaller than
about 17 percentage points, which is a large effect.

So two of the three things this sweep can say are already settled, and neither depends on the
scoring step that is underpowered at 64 pairs:
  - a larger window costs NOTHING (definitive, equal generations in equal time)
  - it accepts more often (suggestive, z = 1.37, not significant)
  - whether that makes it STRONGER (pending, and the 64-pair scoring resolves only ~106 Elo)

## VALIDATION: the arms share a deterministic stream, and which pair to read hardest

Arms 8 and 999 are BYTE-IDENTICAL for their first 4 generations (checked by md5 of the gen
lines):

    gen  pos     train  pool        gen  pos     train  pool
    1    252040   9747   9747       1    252040   9747   9747
    2    253028  14066  23813       2    253028  14066  23813
    3    255253  17549  41362       3    255253  17549  41362
    4    256277  21761  63123       4    256277  21761  63123

That is exactly right and confirms two things: the seed produces the same datagen stream in every
arm, so the arms are properly controlled; and `--replay-gens` genuinely changes nothing until the
window binds.

It also determines which comparison carries the most information:

    window 1   binds from generation 2  ->  differs from 999 for 41 of 42 generations
    window 8   binds from generation 9  ->  differs from 999 for 33 of 42

**So window 1 vs window 999 is the cleanest contrast, and window 8 vs 999 is the weakest** --
those two are literally the same run for the first fifth of it. When the scores land, read 1 vs
999 first; a small 8-vs-999 gap is expected from the shared prefix alone and should not be read
as "8 and unlimited are equivalent".

## RESULT — 2026-09-08, re-scored at 600 pairs against the point all arms STARTED from

The sweep's own scoring used 64 pairs (ci95 0.038-0.060, resolves only a gap >0.054), so it
could not have detected its own hypothesis. Re-scored at 600 pairs (ci95 0.013, two-sample
resolution ~0.018), and with the baseline the sweep never measured:

| net | rate | vs baseline |
|---|---|---|
| BASELINE champion_long (the common start) | 0.864 +/- 0.013 | — |
| replay-gens 1 | 0.843 +/- 0.013 | **-0.021, RESOLVED worse** |
| replay-gens 8 (default) | 0.862 +/- 0.013 | -0.002, unchanged |
| replay-gens 999 (keep everything) | 0.863 +/- 0.013 | -0.001, unchanged |

**NO ARM IMPROVED ON ITS STARTING POINT.** 42 generations each, 6-11 accepted candidates each,
and the best outcome is indistinguishable from where it began. That is the headline and it is not
about the replay window at all.

On the window question, which is what the sweep was for:
  * 1 vs 999 = 0.020 against an 0.018 floor. It JUST clears, so: discarding aggressively is
    measurably harmful. Treat it as suggestive rather than settled — it clears by 0.002.
  * 8 vs 999 = 0.001. NOT RESOLVED. The current default is as good as keeping everything, so
    there is no gain available from widening the window, and no harm either.
  * The direction matters: the conventional staleness argument predicts 1 > 8 > 999 and the
    measurement is the opposite ordering. Keeping data is not costing anything here.

WHY THE BASELINE COULD NOT BE INHERITED. replay_ab2.sh's header says the resumed champion
"measured ~0.83-0.85", and reading the arms against that would have made 0.862/0.863 look like
gains. Those readings predate the control fix, where examples/control had a hardcoded
depth-6/4000-node budget that searched 0.4% of the intended tree; repairing it moved a reading
from 0.504 +/- 0.008 to 0.781 +/- 0.060. Re-measured with the same binary, seed and pair count,
the true starting point is 0.864 — ABOVE all three arms. The inherited number would have
inverted the conclusion.

NEXT: this is consistent with the gate's measured detection floor (median ci95 0.072 at 40 pairs
=> only resolves ~+50 Elo, confirmed again by gate_ab arm 40: 41 generations, 7 accepted, median
ci95 0.072). A gate that cannot see real gains accepts noise instead, and accepted noise walks
downhill. gate_ab is testing exactly that.
