# The plateau is the TRAINING PROCEDURE'S CEILING, not a gating failure

44 control-vs-origin readings across 17 runs, mined from logs already on disk. Zero compute.

## What the readings say

`champion_long` re-measured today: **0.861 +/- 0.010** (1448W-546D-6L, 2000 games). The inherited
0.864 was right.

**Within-run trajectories are the valid evidence.** Pooled per-generation means mix runs with
different starting nets, widths, epochs and caps, so they are not a controlled comparison. The two
long runs each hold their settings fixed across the whole trajectory:

| run | gens 25 -> 150 | shape |
|---|---|---|
| long_run2 | 0.831, 0.808, 0.825, 0.816, 0.791, 0.808 | FLAT |
| long_run4 | 0.838, 0.853, 0.831, 0.823, 0.856 | FLAT |

Neither improves and neither collapses. They sit in a band of roughly **0.79-0.86** for 125
generations. `long_run4` at generation 150 reads 0.856 +/- 0.035, statistically indistinguishable
from champion_long's 0.861.

Runs that START from a random net climb INTO that band and stop there: mean 0.617 at gen 5, 0.689
at gen 10, 0.787 at gen 20, 0.807 at gen 50, 0.832 at gen 150. So the procedure genuinely learns --
from 0.5 to ~0.83 -- and then asymptotes.

## What this does to the previous conclusion, which was mine and was wrong

I reported, off ONE reading, that "the training step degrades the net and the gate was hiding it":
0.861 -> 0.831 after a generation, resolved at 0.030 +/- 0.018. The direction was real. The
INTERPRETATION was not.

`champion_long` at 0.861 sits at the TOP of the procedure's own band. Training it further pulls it
toward the fixed point near 0.83. That is regression to the process's mean, not damage. The same
arithmetic that made "degradation" look resolved is equally consistent with "champion_long is an
upper-tail draw and further training returns it to centre" -- and the 44 readings distinguish
those two, where one reading could not.

It also re-explains the surrogate-fallback runs (0.864 -> 0.826). I called that damage from a bad
decision rule, then called it damage from training. It is most likely neither: it is what happens
when you stop selecting for upper-tail draws and let the champion fall back to the asymptote.

**How champion_long got to 0.861 at all is then the honest open question.** A gate that rejects
almost everything and keeps the occasional favourable draw is a selector for upper-tail noise. That
would produce exactly one net slightly above the band, which is what we have.

## What it points at

The ceiling is a property of the TRAINING PROCEDURE, so the levers are the procedure's capacity and
data quality, not the gate:

* **WIDTH 16.** Every one of these runs is at ARCH menu rung 0 of `[16, 32, 64, 128, 256, 512]`,
  and `--arch-every 0` disables the step in all of them. A 782 -> 16 -> 1 net is 12,528 weights.
  The strongest prior evidence available for this exact failure shape is from the sibling GPU-RL
  project: a policy sat flat across EIGHT anchors at 302k parameters and broke through within
  minutes at 3.12M. Flat-across-many-anchors is the signature of a capacity bound.
* **DEPTH 2 DATA.** Labels come from a depth-2 search, which is a 3-ply tree here (`choose` applies
  the root move then recurses with the full D). The target cannot be better than its source.
* **THE HORIZON SCHEDULE**, `10 + (g-1)*5`, reaches 755 by generation 150 -- effectively the whole
  game. datagen.rs:17-20 warns that in near-random self-play the outcome is nearly independent of a
  position 40 plies earlier, so late-schedule labels are largely noise.

**Ranked by expected gain: width first.** It is the one with direct evidence from an observed
breakthrough, it is a single flag, and it is the only one of the three that changes what the net
can represent rather than what it is shown.

## Honest limits

* Observational, not controlled: these runs were not designed as a width or horizon sweep.
* `arch_run.log` reads exactly 0.500 +/- 0.062 at generations 10, 20, 30 and 40 -- a net that never
  left random. That is a BROKEN RUN, not a data point about the ceiling, and it drags the pooled
  means down. It is also worth its own look, since the arch arm is the lever named above.
* The band's width (0.79-0.86) is wider than most individual ci95 values, so run-to-run variation
  exceeds within-run measurement error. Any width experiment needs multiple seeds to beat that.


---

## DESIGN RULE, learned the hard way: equal wall clock silently varies the horizon

`horizon = min(10 + (g-1)*5, cap)` widens with GENERATION COUNT. So any experiment that equalises
WALL CLOCK rather than generations also varies the horizon, by a factor of however much the arms'
throughputs differ — and it does so invisibly, since nothing in the output names the horizon as a
variable under test.

**Audited every A/B in this campaign:**

| experiment | arms matched on | generations | final horizon | confounded? |
|---|---|---|---|---|
| width (w16 vs w64) | generations (`--gens 20`) | 20 / 20 | h105 / h105 | **no** |
| draws (exclude vs include) | generations (`--gens 20`) | 20 / 20 | h105 / h105 | **no** |
| depth (d2 vs d3) | **wall clock** (2400s) | 92 / 8 | **h465 / h45** | **YES** |

The two refutations survive — width (0.179 ± 0.021) and draws (+0.086 ± 0.015) are clean
comparisons. Only the depth result is affected, and only because equal-compute was the right design
for THAT question and carries this side effect.

**The rule:** if arms are matched on time rather than generations, pin `--horizon-cap` explicitly.
Equal compute and equal horizon are both defensible; getting one by accident while believing you
have the other is not.

This is the third instance today of the same failure shape — two variables moving, the uncontrolled
one flattering the result. Equal-DEPTH vs equal-TIME in the width gate. Equal-BUDGET vs equal-COST
in the game gate. Equal-WALL-CLOCK vs equal-HORIZON here.


---

## Two harness hazards found while the campaign ran, 2026-09-08

**1. STALE OUTPUT FILES READ AS CURRENT RESULTS.** `horizon_ab2.sh` writes `hz_<cap>.log` — the same
filenames the abandoned v1 run used. Between v2's first and second arm, its verdict block would
have compared a FRESH cap-10 arm against v1's `hz_1000.log`, five hours old, produced by a
different experiment with a different metric (mean gate rate, since shown near-blind), and printed
it as a comparison. Caught because the generation counts made no sense for a sequential script: 15
for one arm and 4 for the other, when the second cannot start until the first ends.

The fix — `rm -f` the outputs at script start, so a missing arm reads as MISSING rather than as an
old number — is parked until the run finishes, for the reason below.

**2. I EDITED A RUNNING SCRIPT.** Having found hazard 1, I fixed it by inserting lines near the top
of `horizon_ab2.sh` **while it was executing**. Bash reads scripts by BYTE OFFSET: inserting six
lines shifts everything after them, and when the interpreter finishes the current compound command
and seeks to the next, it reads from a now-wrong position. That is the failure that destroyed 2811
gate pairs in this project's history.

The `for` loop was already parsed in memory so the arms were safe, but the verdict block AFTER the
loop had not been read yet and would have come from a corrupted offset. Reverted to the committed
bytes immediately (`git checkout`), verified the run still progressing (19 generations and
climbing), and parked the fix to apply once it finishes.

**Both hazards are about the same thing: state that outlives the run that produced it.** A stale
log outlives its experiment; a mid-run edit makes the script outlive its own parsed image. Neither
produces an error — both produce a plausible wrong answer, which is worse.
