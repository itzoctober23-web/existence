# EXPERIMENTS — what was tried, and why it failed

The do-not-regress list. Every entry states the design, the result, and — where the result did
not hold up — what was wrong with the EXPERIMENT rather than the idea.

---

## CURRENT STATE (2026-09-08) — the defaults and what each rests on

| constant | value | evidence |
|---|---|---|
| `blend` | **0.75** | +0.0453 +/- 0.0158 over blend 0 against a trained champion, ~6.6 SE. Plateau 0.5-1.0. Costs nothing at iteration zero (0.5617 vs 0.5574). 0.75 not 1.0 is a JUDGEMENT about drift, not a measurement. |
| `horizon-cap` | **1000** (rail) | Optimum moves outward with champion strength: ~40 random, uncapped trained. The SCHEDULE `10+5*(g-1)` already does that; the cap only has to stop strangling it. h160 and h1000 select the same 58,734 positions. |
| `steps-per-gen` | **0** (off) | Refuted on a controlled A/B: 0.5352 vs epochs' 0.5444, difference not significant and the sign REVERSED from the loop. |
| `gate-depth-cap` | **4** | Full-tree cost from startpos: d3 1,921 / d4 3,145 / d5 140,009 / d6 328,495 nodes. At the old default of 6 a 4,000-node budget bought 1.29% of the tree and both sides played at random. |
| `cost-nodes` | **derived** | Tree size is NET-DEPENDENT; a fixed 4,000 covered 57% of one seed's tree and 33% of another's, aborting 4 of 6 experiment arms. |
| interpreter accumulator | **width >= 64** | eval+apply per node: 399->441ns at w16 (WORSE), 665->582 at w64, 2290->1456 at w256. DORMANT while the champion is width 16. |
| search accumulator | **width >= 64** | Independent measurement, same crossover: 32 is 0.91x (LOSS), 128 is 1.14x, 512 is 1.51x. This is why width 32 loses the CLOCK gate — a wider net's cost with none of the saving. |
| `gate-pairs` | **224** (was 40) | Median ci95 over 230 generations was 0.079 = a **+56 Elo detection floor**: real gains below that were rejected as noise, noise above it accepted as gain. Also restores the games as decider — `resolves` needs ci95 < 0.05, which 40 pairs (0.071) structurally never reached. Gate is 2.7% of a generation, so resolution is nearly free. |
| `arch-pairs` | **224** (was 160) | 160 was derived to hit ci95 < 0.05 EXACTLY. Four gates in two runs measured 0.050/0.051/0.052/0.053 — `resolves` false every time by 0.001-0.003, so its strict branch never executed. 224 targets 0.0435. |
| `arch` stride | **grows: +1,-1,+2,-2,...** | At stride 1 the only widening from rung 0 is width 32, a rung the clock gate must reject (measured 0.372 +/- 0.053), so the arm re-proposed a known cost cliff forever and width 128 was unreachable. Reach, not answer: the gates still decide. |
| `games` per generation | **2400** (of those tried) | Equal wall-clock, origin-scored: 150 -> 0.555, 600 -> 0.773, 2400 -> 0.828. Monotone, and the INVERSE of generation count (140 / 45 / 12 generations). |
| acceptance | sign, then width, then surrogate | Gate resolves the SIGN -> it decides. Narrow interval straddling 0.5 -> reject (precisely measured null). Only a WIDE straddle reaches the surrogate. |

**QUALIFICATION ON "THE PLATEAU" (2026-09-08, and it applies to everything below).** The origin
control resolves a gap of ~0.054, which is about **70 Elo**. Seven readings across two runs:

    gen  25  0.780 +/- 0.039      gen  30  0.838 +/- 0.037
    gen  50  0.805 +/- 0.040      gen  60  0.853 +/- 0.037
                                  gen  90  0.831 +/- 0.040
                                  gen 120  0.823 +/- 0.035
                                  gen 150  0.856 +/- 0.035

EVERY gap between readings is below 0.054. So "flat" means **"no change larger than ~70 Elo was
detected"**, NOT "no improvement occurred". The champion could have gained 50 Elo across those
125 generations and this metric could not have shown it. There is even a hint of upward drift --
0.780 -> 0.805 within run 3, 0.838 -> 0.856 within run 4 -- that was dismissed as noise; it is
still not significant, but "flat" was the wrong word.

This does NOT overturn the gate-resolution finding, which rests on separate and direct evidence:
the acceptance rule's games-based branch fired 0 times in 57 generations, read from the ledger
rather than inferred from control readings. But every "the plateau persists" statement below
should be read as "not resolved at ~70 Elo".

**WHAT IS CLOSED, as of 2026-09-08.** Two explanations for the plateau are now measured out:

| explanation | status |
|---|---|
| not enough CAPACITY | **CLOSED at every rung.** 16->32 and 16->64 reached the game gates and lost (64 loses at EQUAL NODES, 0.498); 16->128 and 16->256 never passed the surrogate filter. Two different failure modes, one conclusion. |
| not enough SEARCH DEPTH | untested here; the gates run at depth 2-4 by design |
| the TRAINER NEVER SEES HISTORY | **OPEN, and the current lead.** ~250k positions generated per generation, ~50k trained on, everything older discarded. The replay buffer is written every generation and, with `--steps-per-gen 0`, never read. |
| CONVERSION blind spot | OPEN. Every gate and control starts from a RANDOMISED opening, so the true starting position is never scored or trained on. 88% of won positions convert from random openings (n=200). |
| the champion loses 44/320 to a RANDOM net | OPEN and unexplained. Distinguishable by running the control at several depths; not yet run. |

## DETECTION FLOORS — what each instrument in this project can actually see

Added 2026-09-08 after four separate instruments turned out unable to detect what they were built
for. Every gate here now has a stated floor; a result below its floor is NOT RESOLVED, which is
not the same as no effect.

| instrument | pairs | ci95 | resolves | ~Elo | note |
|---|---|---|---|---|---|
| NET gate (was) | 40 | 0.071 | 0.071 | **49** | resolved 0 of 57 generations — measured |
| NET gate (now) | 224 | 0.030 | 0.030 | 21 | also returns the decision to the games |
| ARCH gate (was) | 160 | 0.051 | 0.051 | 36 | derived to hit 0.05 exactly, missed by 0.001-0.003 |
| ARCH gate (now) | 224 | 0.044 | 0.044 | 30 | |
| origin control (was) | 224 | 0.038 | 0.054 | **70** | why "the plateau" was overstated |
| origin control (now) | 1000 | 0.018 | 0.025 | 33 | one-off per reading: 3.3% overhead |
| replay sweep scoring | 64 | 0.060 | 0.085 | **106** | running; result must be read against this |
| gate A/B scoring | 600 | 0.020 | 0.028 | 34 | raised from 64 before launch |

**The pattern: the CHEAP components were the broken ones.** The gate is 2.7% of a generation, the
control a one-off match, the scoring step a single match at the end. Nobody examines a 3% cost
while hunting a plateau, so the constants that decided everything went unquestioned while
capacity, data volume and horizon — all expensive, all visible — were investigated exhaustively
and cleared.

**Second pattern: every fix was nearly free.** Raising resolution costs 3-17% because these are
one-off or small-fraction costs, not per-node work. There was never a trade-off to weigh; the
under-powered settings bought nothing.

**Third, and the one to carry forward: I built the same defect into my own experiments twice** —
the replay sweep varies a flag on a path that never reads it, and `gate_ab.sh` was going to test
a ~20 Elo hypothesis with a ~106 Elo instrument. Before running anything, ask what it can detect,
not just how long it takes.

**The one methodological finding that produced most of the others:** these constants are
COUPLED, and a one-dimensional sweep through a two-dimensional interaction returns a confident,
reproducible, wrong answer. The horizon was swept four times and shipped three times before the
blend was found; every reading was real and every one was taken with the other constant set
wrong. Nothing inside those measurements could have revealed it.

**Second finding, about instruments:** the loop has 0.151 run-to-run variance on the control
metric — larger than most effects worth testing — because it is path-dependent. A fixed-dataset
A/B has se ~0.005 on the same question. Use the loop to ask whether something COMPOUNDS; use
the A/B to ask whether it works at all.

---

## 2026-09-08 — THE TRAINER NEVER SEES HISTORY, and no experiment has ever tested whether it should

He asked whether throwing positions away is worth it, arguing that a champion which is not
improving cannot be generating "stale" data. Chasing that turned up something I had wrong.

**The replay buffer is written every generation and, in the default configuration, never read.**
It is used in exactly two places: `main.rs:460`, gated behind `steps_per_gen > 0` (defaulted OFF,
and off for a measured reason), and `main.rs:619`, the ARCH arm. With `--steps-per-gen 0` and
ARCH idle, training runs `tr.epoch(&mut cand, subset, ...)` where `subset` is THIS GENERATION's
slice. So per generation:

    ~250,000 positions generated
    ~50,000 trained on (this generation's decisive slice)
    everything from every previous generation discarded, permanently

Not "a rolling 250k window", which is what I told him and what the dashboard implied. The
trainer sees ONE generation. 22.6M positions generated across the run; ~50k in front of the
trainer at any moment.

**I nearly measured this with an instrument that could not detect it.** `replay_ab.sh` swept
`--replay-gens` 2/8/32 with `--arch-every 0 --steps-per-gen 0` -- so all three arms were
byte-identical, and would have produced three scores within noise that I would have written up
as "not resolved at this budget". A null that reads like a measurement. Killed mid-run after
reading the two call sites.

**And the prior result does not close the question.** EXPERIMENTS records `steps-per-gen` as
REFUTED, and I repeated that to him. Reading it properly: that A/B used "ONE shared dataset
(4000 games, 17,266 training samples)" for BOTH arms. It compared HOW MANY GRADIENT STEPS to
take over one generation's data -- epochs 3 (~51,798 updates) against a fixed 20,000 -- and
found no significant difference. Both arms saw identical positions. It never tested accumulated
history, and its own closing line says so: "WHAT THIS DOES NOT SHOW: whether a step budget
matters over MANY generations."

So: training on history is UNTESTED, not refuted. The correct experiment turns `--steps-per-gen`
on so line 460's `pool` is the replay buffer, and varies `--replay-gens` so the pool spans
different amounts of history. Both flags are needed -- varying the window alone does nothing,
which is the trap that killed the first attempt.

---

## 2026-09-08 — CAPACITY CLOSED AT EVERY RUNG, and two different failure modes

The stride fix let the arm sweep the whole menu from width 16. It did. Every rung is rejected,
and the reason CHANGES with width:

| step | surrogate | fixed-cost | clock | outcome |
|---|---|---|---|---|
| 16 -> 32 | 0.0730 vs 0.0777 (z 2.43) PASS | 0.544 | **0.334** | reached gates, lost on clock |
| 16 -> 64 | 0.0701 vs 0.0775 (z 4.12) PASS | **0.498** | **0.281** | reached gates, lost on BOTH |
| 16 -> 128 | 0.0757 vs 0.0712 FAIL | — | — | never reached a gate |
| 16 -> 128 | 0.0801 vs 0.0769 FAIL | — | — | never reached a gate |
| 16 -> 256 | 0.0685 vs 0.0628 FAIL | — | — | never reached a gate |

**Two distinct failures, not one.** At 32 and 64 the widened net trains fine and then loses the
GAMES. At 128 and 256 it never even gets that far: 30 epochs on the replay buffer is not enough
to train a net with 8-16x the parameters back to the champion's held-out loss, so the FITNESS 5
filter stops it. Function-preserving widening guarantees parity at BIRTH; it guarantees nothing
after training, and the bigger the net the more the training moves it.

So "capacity is the plateau's cause" is now refuted across the entire declared menu, by two
independent mechanisms. What is NOT shown: that a wider net trained PROPERLY (many more epochs,
or its own datagen) would fail. The 30-epoch budget is a constant that was tuned for width 16
and never re-derived, which makes the 128/256 rejections a statement about the training budget
as much as about capacity.

## 2026-09-08 — CORRECTION: the origin control did NOT break its plateau

I reported "+306 Elo, the first movement above the flat band" from the dashboard. That was the
GEN 60 reading. The full sequence:

    gen 30: 260W-16D-44L  0.838 +/- 0.037   (~+277)
    gen 60: 264W-18D-38L  0.853 +/- 0.037   (~+305)
    gen 90: 257W-18D-45L  0.831 +/- 0.040   (~+276)

Three overlapping intervals oscillating around ~0.84. I read the peak of the oscillation as a
trend and said so out loud, including to him. The plateau has NOT broken; this is the same flat
band as long_run3 (0.831 / 0.808 / 0.825 / 0.808), shifted by nothing.

This is the third time today a RATE has been read as more than it was -- 0.838 hid 44 losses to
a random net, "49% decisive" hid a game where the engine shuffled for thirteen moves on a won
position, and now a three-point oscillation was read as movement. The dashboard showing only the
LATEST control makes this easy; it should show the series.

---

## 2026-09-08 — THE GATE CANNOT SEE IMPROVEMENT: median ci95 0.079 = a +56 Elo detection floor

Measured over 230 generations of ledger (long_run3 + long_run4), the per-generation NET gate at
32 pairs has a **median ci95 of 0.079**. So it can only resolve a candidate better than 0.579:

    0.579 -> +56 Elo. Anything smaller is invisible to it.

Self-play improvement does not arrive in +56 Elo steps. So the loop has been REJECTING genuine
small gains as noise while ACCEPTING noise that happened to look large -- every accept observed
today sits at 0.52-0.66, which is precisely the size of a random fluctuation at ci95 0.079.

**This is a better candidate for the plateau than anything else on the list.** Capacity is closed
by measurement (32/64 lost the game gates, 128/256 never cleared the surrogate). Data volume is
under test. But neither matters if the acceptance test cannot tell a better net from a lucky one:
strength cannot accumulate through a filter that discards every increment below +56 Elo.

**And it is cheap, which is why it went unnoticed.** The gate is 2.7% of a generation's cost --
64 games against 2400 spent on datagen. Buying resolution is nearly free:

| pairs | resolves | gate games | overhead vs datagen |
|---|---|---|---|
| 32 (current) | +56 Elo | 64 | 2.7% |
| 81 | +35 Elo | 162 | 7% |
| 224 | +21 Elo | 410 | 17% |

The ARCH arm already learned this lesson in miniature -- its pair count went 32 -> 160 -> 224 for
exactly this reason -- and the per-generation gate was never revisited.

**IT ALSO REPAIRS THE SURROGATE OVERRIDE, which I had been treating as a separate defect.**
The acceptance rule hands the decision to the GAMES when `ci95 < 0.05` and falls through to the
held-out-loss surrogate when it does not. Expected interval by pair count:

    32 pairs -> 0.079   surrogate decides
    40 pairs -> 0.071   surrogate decides      <- the old default
    81 pairs -> 0.050   gate decides
   224 pairs -> 0.030   gate decides           <- the new default

So `resolves` was structurally almost never true, which is exactly what was measured this
morning: it fired ONCE in ten generations. The surrogate was the de-facto decider.

That matters because the surrogate demonstrably disagrees with the games. The width-64 ARCH
candidate produced the strongest surrogate reading of any candidate all day -- held-out loss
0.0701 vs 0.0775, paired z 4.12 -- and then failed to beat width 16 AT EQUAL NODES. Lower loss,
no more games won. A loop where that surrogate decides is a loop optimising held-out loss rather
than strength.

One under-powered constant therefore produced three symptoms I had been chasing separately:
real gains below +56 Elo rejected as noise; noise above it accepted as improvement; and the
surrogate permanently overriding the games.

**PRE-REGISTERED PREDICTION, so this can be wrong.** Raising the pair count should produce BOTH
more accepts AND a rising origin control. If accepts rise and the origin control stays flat at
~0.84, the diagnosis is refuted: the candidates were never better and the gate was right to
reject them. That outcome is just as informative and must be reported as a refutation.

---

## 2026-09-08 — THE OPENING BLIND SPOT IS SYSTEMIC: nothing in this project ever sees the real start

Audited every call path after `control.rs` turned out to have drifted. `open_plies = 6` is
hardcoded in datagen, in both gate functions, and in every example that generates games:

    datagen (the loop)        play_game(net, depth, rng, 6, 160, ...)
    gate::match_nets          startpos + open_plies random moves
    gate::match_nets_capped   same
    capacity.rs, paired.rs, trainer_control.rs, datagen_cost.rs   all `6`

So the true starting position and the first six plies are NEVER generated, never trained on, and
never scored -- by any harness in the project. This is consistent by design rather than a bug:
without opening randomisation every self-play game would be identical, which is exactly what
`showgame.rs` demonstrated (60 "games" from startpos were byte-identical because the search takes
a strict argmax, so a per-game seed only shuffles tie-breaks).

But the consequence is real. An engine that will be played FROM the starting position is trained
and evaluated exclusively on positions at least six random plies away from it. Measured:

  - from startpos: wins +18 material (queen, both rooks) and then shuffles one piece between d8
    and f6 for thirteen moves. Deterministic, so n=1 -- one line, not a rate.
  - from randomised openings: converts 88% of won positions (147 of 168, n=200).

The 88% says conversion is broadly fine. The startpos line says there is at least one
deterministic path the engine plays badly and that no gate in the project can ever observe,
because every gate starts past it.

**IT EXTENDS TO 4PC AND TO THE EXTERNAL ANCHOR TOO** (checked 2026-09-08). The 4pchess benchmark
passes no `SPRT_OPENINGS`, so `tools/sprt.py:305` falls through to `gen_openings(..., plies=6)` --
the same 6-ply randomisation. So the ONE number in either project that is not self-referential
also never sees the real starting position. The blind spot is complete across both engines and
every harness.

One redeeming detail: `gen_openings` is EVAL-SCREENED (sprt.py:90-91, "reject positions already
outside" a bound), so it discards openings where one side is already winning. The randomisation
is not blind -- which is why those games are decided by play rather than by the opening.

NOT ACTED ON. Removing the randomisation would collapse self-play diversity to a single game.
The fix, if this matters, is to ADD startpos-rooted evaluation alongside the randomised gates --
not to replace them. Recorded because it is the kind of gap that is invisible to every metric
currently collected, which is precisely why it needs writing down rather than remembering.

---

## 2026-09-08 — RESOLVED: the "44 losses to a random net" was two different measurements

Ran the control at both depths, which is the test I said would distinguish the two explanations:

```
fixed depth 2, uncapped    99W-28D-1L    0.883 +/- 0.038
fixed depth 3, uncapped   110W-18D-0L    0.930 +/- 0.028
depth cap 6, 4152 nodes     8W-113D-7L   0.504 +/- 0.008   no difference detected
```

**The benign explanation wins.** Losses fall 1 -> 0 and the rate climbs 0.883 -> 0.930 as depth
rises, so the random-EVAL opponent was being carried by SEARCH: alpha-beta at depth 2 still sees
captures two plies ahead even with a noise eval, and that is what stole games. Not a blind spot
in the champion.

And the premise was wrong anyway. `260W-16D-44L` came from the IN-LOOP control, which runs under
`equal_time_caps`; this uncapped match loses ONE game in 128 at depth 2 and none at depth 3. I
was treating two different regimes as one number.

**The real finding is the third line.** The budgeted gate buys 4152 nodes against the 992,296 a
full depth-6 search costs from startpos -- **0.4% of the tree**. Neither side finishes its first
root move, so both play near-randomly, 113 of 128 games draw, and the gate returns 0.504 +/-
0.008. A TIGHT interval around no-difference, which reads like a confident null and is no
evidence whatsoever.

This repo already documents exactly this failure ("at the old default of 6 a 4,000-node budget
bought 1.29% of the tree and both sides played at random") and fixed it for the NET gate by
capping depth at 4. `examples/control.rs` still hardcodes 6, so the diagnostic that exists to
check the loop has the very defect the loop was fixed for.

---

## 2026-09-08 — OPEN: the champion loses 44 of 320 games to a RANDOM net, and nothing explains it

The origin control has been read all session as a success -- 0.838 +/- 0.037, "+258 Elo vs
iteration zero". Look at the raw line instead of the rate:

    control vs origin @gen 30: 260W-16D-44L

**44 losses to a randomly-initialised network**, after 150+ generations of training. A net whose
eval is noise should be losing essentially every decisive game, not winning 44.

Two things this could be, and they have different consequences:
  - The opponent is not as weak as "random". The origin is a random EVAL, but it is searched with
    the same alpha-beta at the same depth, so it still sees captures two plies ahead and avoids
    immediate blunders. Then 0.838 is roughly the honest value of the learned eval over noise at
    this depth, and the ceiling is a property of the search, not the net.
  - Or the champion's eval is actively wrong in some class of positions, and those are the 44.

These are distinguishable: play the same control at several depths. If the loss count falls as
depth rises, the opponent was being carried by search; if it holds, the champion has a blind
spot the aggregate hides. NOT YET RUN.

Flagged because "0.838 vs origin" has been quoted repeatedly today, including to him, as
evidence the loop learns. It IS evidence of that. It is also evidence of something unexplained,
and the rate hides the 44 while the raw W-D-L does not -- the same way "49% decisive" hid a game
where the engine won +18 material and then shuffled for thirteen moves.

---

## 2026-09-08 — CAPACITY IS NOT THE CONSTRAINT: width 64 loses at EQUAL NODES, not just on the clock

The stride fix let the arm reach an untried rung. It reached it, and the answer refutes my own
framing of the problem:

| step | surrogate | fixed-cost (equal NODES) | clock (equal TIME) | nodes cand vs champ |
|---|---|---|---|---|
| 16 -> 32 | 0.0730 vs 0.0777, z 2.43 | 0.544 +/- 0.050 | 0.334 +/- 0.052 | 6119 vs 7252 |
| 16 -> 64 | 0.0701 vs 0.0775, z **4.12** | **0.498 +/- 0.053** | **0.281 +/- 0.043** | 5511 vs 7584 |

**I had the shape of this wrong.** The cost-cliff story was: a wider net knows more per node and
merely cannot pay its clock cost, so reaching a rung where the accumulator pays (128 at 1.14x)
might flip it. Width 64 kills that. It does not beat width 16 at EQUAL NODES -- 0.498 straddles
0.5 -- so the extra capacity is buying no extra knowledge at all, and it is simultaneously much
slower (5511 nodes against 7584). Worse on both axes, and worse than width 32 on both axes.

Note the surrogate said the opposite, loudly: held-out loss 0.0701 vs 0.0775 at paired z 4.12,
the strongest surrogate reading any ARCH candidate has produced. Lower held-out loss, no more
games won. That is the FITNESS 5 surrogate doing exactly what the docs warn it does -- proposing,
not deciding -- and it is the clearest example yet of why the gate is the authority.

**What this closes and what it does not.** Widening is measured harmful at 32 and neutral-to-
harmful at 64, from a width-16 champion on this corpus. It does NOT close 128+: the accumulator
crossover is real and unmeasured above 64, and `capacity.rs` exists because "wider is better"
already failed once here. But the prior should now be that capacity is NOT the plateau's cause,
and the plateau needs a different explanation -- the conversion blind spot (every gate starts
from a randomised opening; the true starting position is never scored) is the current candidate.

 against a moving baseline (n=4, observation)

Every ARCH attempt is now in the ledger with a named reason. Four on record:

| candidate loss | champion loss | outcome |
|---|---|---|
| 0.0746 | 0.0847 | reached the gate -> `lost_on_clock` |
| 0.0762 | 0.0719 | `surrogate_filter` |
| 0.0857 | 0.0706 | `surrogate_filter` |
| 0.0730 | 0.0777 | reached the gate -> `lost_on_clock` |

The champion's OWN held-out loss moves 0.0706 -> 0.0847 across generations, a ~20% spread,
because the judge set is that generation's held-out slice and the champion changes underneath
it. The candidate's post-training loss moves about as much. So `acand_loss > champ_loss * 1.005`
is one noisy number against another noisy number, and 2 attempts in 4 reached a gate.

**Function-preserving widening guarantees equality AT BIRTH, not after training.** The candidate
starts as an exact copy of the champion's function and is then trained up to 30 epochs; that
training is what moves it, and it moves either way. So the filter is not measuring "is width 32
better" -- it is measuring "did this particular 30-epoch run land above or below a baseline that
also wandered".

NOT ACTED ON, deliberately. The 0.5% ratio test is FITNESS 5's declared rule, and `paired_loss_z`
already exists in the same function (it read 4.21 on the attempt that passed) but is used only
later, for `surrogate_ok`. Replacing an unpaired ratio with the paired statistic would very
likely be more sensitive on identical data -- the pairing is free, both nets see the same judge
positions -- but that is a change to a SPEC'D rule and n=3 is not evidence. Recorded so the
next person sees the filter's noise floor before reading any single ARCH verdict as a fact.

---

## 2026-09-08 — WIDTH 32 IS A COST CLIFF, and stride-1 stepping could never get past it

First widening ever to reach a game gate (after function-preserving widening removed the
birth handicap):

```
ARCH w 16 -> w 32 (3 ep, loss 0.0746 vs 0.0847, paired z 4.21)
  fixed-cost 0.525 +/- 0.051 ok
  clock      0.372 +/- 0.053 [5962 vs 6985 nodes]   => hold
```

Read it in three parts.

**The widening fix worked.** Held-out loss 0.0746 against the champion's 0.0847 at paired
z 4.21. Every previous attempt was WORSE at birth (0.0744 vs 0.0687) and died at the surrogate
filter without playing a game. This one passed the filter and reached the gates.

**The gate rejected it, correctly.** 0.525 at equal NODES, 0.372 at equal TIME. Width 32 knows
more per node and searches 5962 nodes where width 16 searches 6985. That is FITNESS 10's named
degenerate case -- "bigger net that wins fixed-cost-budget, loses on clock" -- and the clock
gate exists precisely to catch it. Verified against the code rather than inferred: `resolves`
needs both ci95 < 0.05 (they were 0.051 and 0.053), so the non-regression branch applied and
`clock_win = 0.372 + 0.053 > 0.5` is false. It fails the strict branch too.

**My follow-up hypothesis was REFUTED BY DATA ALREADY IN THE TREE.** I proposed making the
incremental accumulator pay at width 32 to close the node-rate gap. `search.rs:68-74` had
already measured it -- same tree, identical node counts on both paths, so the ratios are real:

| width | refresh | incremental | |
|---|---|---|---|
| 32 | 1718969 | 1560497 | **0.91x LOSS** |
| 128 | 889406 | 1014240 | 1.14x win |
| 512 | 240701 | 363458 | 1.51x win |

Width 32 is the worst of both worlds: a wider net's cost with none of the accumulator's saving.
Checking beat running the experiment.

**THE STRUCTURAL FINDING.** The accumulator does not pay until ~128, and `arch::propose` only
ever stepped +/-1 rung. So from rung 0 the ONLY widening available was width 32 -- a rung the
clock gate must reject -- and the arm would re-propose that same cliff forever. Width 128, where
the same measurements say the cost flips, was unreachable BY CONSTRUCTION. The capacity ladder
had a hole at its first rung and no way over it.

**REPLICATED 2026-09-08, second independent run.** The clock verdict is not one noisy reading:

| run | fixed-cost (equal nodes) | clock (equal time) | nodes cand vs champ |
|---|---|---|---|
| gen 20, long_run3 | 0.525 +/- 0.051 | **0.372 +/- 0.053** | 5962 vs 6985 |
| gen 30, long_run4 | 0.544 +/- 0.050 | **0.334 +/- 0.052** | 6119 vs 7252 |

Both say the same thing: width 32 knows MORE per node (~0.53-0.54) and is much worse per SECOND
(~0.33-0.37). The second attempt also cleared the surrogate independently (loss 0.0730 vs
0.0777, paired z 2.43), so the widening fix reproduces too -- the pre-fix arm never once got
this far.

**CONFIRMED LIVE, not just argued from the code.** The very next ARCH attempt in the same run
(generation 40, still on the stride-1 binary) proposed the IDENTICAL rung:

```
ARCH w 16 -> w 32 ... clock 0.372 +/- 0.053  => hold        (gen 20, reached the gate)
ARCH w 16 -> w 32: held-out 0.0762 vs champ 0.0719 -- surrogate filter, no gate   (gen 40)
```

Same target, re-proposed, because `[1,-1]` and `[-1,1]` both collapse to `+1` at rung 0 where
narrowing does not exist. Every future attempt would have done the same thing forever.

Fix: strides grow with the attempt (+1, -1, +2, -2, +3, ...), so from width 16 the arm reaches
128 by attempt 4. This deliberately does NOT hardcode "128 is good" -- that would hand the
search its answer. It widens the arm's REACH; the fixed-cost and clock gates still decide every
step on games. Tests assert the arm still tries the cheap adjacent rung FIRST, still reaches
past the cliff, can still NARROW (a capacity search that only grows is not a search), and never
steps off the menu.

NOT SHOWN: that width 128 passes. It may lose on the clock too -- 8x the parameters against a
1.14x accumulator saving is not obviously a win, and three capacity levers have already measured
flat elsewhere in this file. What changed is that the question can now be ASKED.

---

## 2026-09-08 — CAPACITY COULD NEVER INCREASE: the ARCH arm judged a newborn against a veteran

The origin control across one 2400-games run:

| generation | control vs origin |
|---|---|
| 25 | 254W-24D-42L, 0.831 +/- 0.037 |
| 50 | 243W-31D-46L, 0.808 +/- 0.039 |
| 75 | 255W-18D-47L, 0.825 +/- 0.042 |

**Flat.** Fifty generations of accepted candidates bought nothing measurable. In the same log:

```
ARCH w 16 -> w 32: held-out 0.0744 vs champ 0.0687 -- surrogate filter, no gate
ARCH w 16 -> w 32: held-out 0.0728 vs champ 0.0649 -- surrogate filter, no gate
ARCH w 16 -> w 32: held-out 0.0745 vs champ 0.0609 -- surrogate filter, no gate
```

Three widening proposals, three rejections, **none of which played a game**.

**The mechanism.** `train_fresh(p.width(), ...)` built the candidate by RANDOM INITIALISATION at
the new width, trained it for at most 30 epochs on the replay buffer, and then compared its
held-out loss against a champion carrying 75 generations of accumulated training. That contest
cannot be won at birth. The FITNESS 5 filter ("must not be worse than the champion by more than
0.5%") then rejected it before it could reach the game gate, every time, by construction.

So the champion was pinned at width 16 permanently, and two things followed that look unrelated
until you see this:
  - the origin control went flat, because the only axis left was weights at fixed capacity;
  - `INCREMENTAL_MIN_WIDTH = 64` meant the incremental accumulator -- "the biggest single win" on
    the task list -- stayed DORMANT forever, since the loop could never reach width 64.

**The fix is function-preserving widening (Net2WiderNet, Chen et al. 2015), not a weaker filter.**
Loosening the surrogate would have let genuinely worse candidates through; the problem was never
the threshold, it was that the candidate was born crippled. Each new unit copies a source unit
and every source unit's outgoing weight is divided by its replica count, so the wider net
computes an IDENTICAL function at birth, starts at exactly the champion's loss, and is judged on
what the extra capacity ADDS.

Three tests, each catching a different way this goes silently wrong:
  - widening to the SAME width reproduces the net exactly (asserted on every weight, no tolerance)
  - 16 -> 64 changes the eval by <= 2cp across 40 walked positions
  - no copied unit is bit-identical to its source -- exact duplicates get identical gradients
    forever, so the wider net would have more parameters and no more capacity. That is the
    failure mode the paper warns about, and it would have passed both other tests.

NOT YET SHOWN: that widening now actually passes a game gate, or that width 32 beats width 16 on
strength. This removes a structural impossibility; it does not prove capacity is the binding
constraint. The next run's ARCH lines are the evidence, and they may still say 16 is right.

---

## 2026-09-08 — GAMES PER GENERATION: strength is MONOTONE in it, and generation count is an anti-metric

Three arms, the SAME 300s of wall-clock each, single core, same seed, then every arm's champion
scored against the SAME frozen origin (`Net::random(hidden, 20260907)`) with an identical match.
Equal time is the whole design: equal generations would have compared different amounts of work.

| games/gen | generations reached | decisive at the end | vs origin (64 pairs, depth 2) |
|---|---|---|---|
| 150 | **140** | 19/150 (13%) | 14W-114D-0L, **0.555 +/- 0.028** |
| 600 | 45 | 170/600 (28%) | 71W-56D-1L, **0.773 +/- 0.045** |
| 2400 | 12 | 814/2400 (34%) | 84W-44D-0L, **0.828 +/- 0.038** |

**Monotone, and the ordering is the exact inverse of generation count.** The 150-games arm ran
11.7x more generations than the 2400 arm and finished 0.27 weaker. It also ended on the decisive
rate it STARTED with -- 19/150 at generation 1, 19/150 at generation 140 -- with 15 accepts in
140 tries. It mostly draws (114 of 128 games); the 2400 arm wins outright (84 of 128).

**Why this matters more than the constant it settles.** I had concluded the opposite one hour
earlier, from a correct measurement: datagen is 3.3ms/game and therefore essentially the entire
cost of a generation, so 150 games/gen makes generations 27x cheaper and "hundreds of thousands
of generations" reachable. Every part of that is true and the conclusion was still wrong, because
cheap generations are not the goal. A generation's VALUE is its training signal, which scales with
the decisive games in it, and 150-game generations yield ~800 training samples against ~46,000.
Twelve times more generations, fifty times less signal each.

Generation count is not a neutral proxy here -- it is ANTI-correlated with strength across the
whole range tested. Any future tuning that optimises generations, steps, or throughput without an
origin-scored control is liable to select the arm that learned nothing, by a factor of twelve.

NOT SHOWN: whether the trend continues past 2400, and whether it survives multi-hour runs where
the replay buffer saturates. Both arms of that are open.

---

## 2026-09-08 — DEFECT: the gate promoted a LOSING candidate, and the guard was named `no_regression`

Found by reading `ledger_newdefaults.jsonl` after the loop plateaued from generation 7:

| gen | gate rate | ci95 | resolved | mcnemar z | decision |
|---|---|---|---|---|---|
| 6 | 0.656 | 0.079 | false | 9.15 | accepted |
| 7 | 0.539 | 0.086 | false | 0.39 | no_evidence |
| **8** | **0.484** | 0.093 | false | 3.55 | **accepted** |
| 9 | 0.508 | 0.084 | false | -1.89 | no_evidence |

Generation 8 went **15W-32D-17L** — a losing record — and was promoted over the champion.

**Two defects, compounding.**

1. `no_regression` read `pent_rate() + ci95() > 0.5`, which passes anything above `0.5 - ci95`.
   At generation 8 that bar was **0.407**. Adding the interval to the candidate's own score
   converts uncertainty into permission; the check could not refuse. It now reads
   `pent_rate() >= 0.5`. The guard is not asked "is the candidate PROVEN worse?" — with a wide
   interval nothing is ever proven and that question always answers no — it is asked "does the
   gate CONTRADICT the surrogate?", and a point estimate below 0.5 does.

2. The handover to the gate never happens. `gate_can_resolve` requires `ci95 < 0.05`; the actual
   ci95 at 24-64 pairs is 0.048-0.095, so it was true **once in ten generations** (gen 1). The
   surrogate was documented as deciding only during bootstrap, "until play is decisive enough to
   resolve". In practice it decides permanently, so once mcnemar stops tracking real strength the
   champion random-walks on the surrogate's noise. That is exactly the shape of the plateau.

**Verified after the fix:** generation 28 scored 0.479 and was REJECTED, where the old rule
would have computed `0.479 + 0.028 = 0.507 > 0.5` and let the surrogate promote it.

Defect 2 is fixed only in the sense that defect 1 now backstops it. The threshold itself is
still unreachable at the pair counts the loop runs, and that remains open.

---

## 2026-09-08 — `--control-every 0` aborted the whole run with SIGABRT

`if g % ctrl_every == 0` with no positive guard: "attempt to calculate the remainder with a
divisor of zero". `arch_every` was already guarded as `arch_every > 0 && ...` at the ARCH step,
so the two flags disagreed about what `0` meant, and the natural reading of "never run the
control" killed the process. Cost two timing probes before it was spotted.

---

## 2026-09-08 — the "biggest single win" is DORMANT in every run the loop performs

`INCREMENTAL_MIN_WIDTH = 64` (crates/interp/src/lib.rs:71), and the loop starts at
`WIDTH_MENU` rung 0 = **width 16** (crates/pipeline/src/arch.rs:31). The incremental accumulator
is therefore switched OFF for the entire loop unless an ARCH step widens the champion to >= 64.

The threshold is correct on its own evidence — the accumulator is a measured LOSS at width 16
(399->441ns per node) and only pays from 64 up. Both facts are right and the conclusion is still
that the headline optimisation buys the running loop nothing. This is not a code defect; it is a
threshold interacting with a starting rung, which no measurement of either one alone would show.

ANSWERED 2026-09-08 for the adjacent rung: width 32 loses the CLOCK gate at 0.372 +/- 0.053
while winning on equal nodes at 0.525, because the accumulator is a 0.91x LOSS at that width.
So starting one rung wider would be strictly worse. Whether a rung where the accumulator PAYS
(128 at 1.14x, 512 at 1.51x) is worth its parameters is now reachable and still unmeasured. Do
not assume wider is better — `capacity.rs` exists because that assumption failed before.

---

## 2026-09-08 — METHODOLOGICAL: my arms have been n=1, and it shows

**The problem, stated against my own data.** I ran single-seed arms all day and drew
causal conclusions from them. Two runs with IDENTICAL `--epochs 3` disagree:

| run | flags that differ | outcome |
|---|---|---|
| `cap40` | threads 4, arch-every 5 | NO collapse; control 0.694 +/- 0.044 at gen 20 |
| `long`  | threads 3, arch-every 6 | collapse from gen 13; rate 0.398, McNemar z -13.31 |

`--threads` repartitions the RNG stream across workers, so the two saw different games
from the same seed. That is enough to make them different draws, not a controlled
contrast. The `--epochs 1` arm then degraded LESS than `long` and MORE than `cap40`,
which is consistent with epochs mattering and equally consistent with it mattering not
at all.

**What this invalidates.** The commit "Decouple the trainer's step budget from datagen
volume" argues from the `long` run's collapse that raising volume 125x broke the
trainer's step budget. That mechanism is plausible and the arithmetic is real (~200 ->
~110,000 samples per generation at fixed epochs). But the EVIDENCE offered for it is one
run, and another run at the same setting did not collapse. `--steps-per-gen` remains
worth testing; it is not yet supported. Default stays 0.

**The rule, which was already written down and which I did not follow:** >= 3 seeds per
arm and >= 2 checkpoints before believing a difference. Eval power is n = 2/e^2 per arm,
so resolving a 0.05 effect needs ~800 pairs, not 320.

**Design for the re-run, once the box is free** (a 58.6M-row 4PC training holds it):
- 3 seeds x {epochs 3, steps-per-gen 20000}, all other flags IDENTICAL including
  `--threads`, since threads changes the data.
- Report the POOLED control across seeds with its interval, not the best arm.
- Pre-register the prediction before looking: if step budget is the mechanism, the
  fixed-budget arm should hold its McNemar z above zero where the epochs arm goes
  negative. If both go negative, the mechanism is wrong and the cause is elsewhere.

## 2026-09-08 — no blend RAMP is needed: 0.75 is free at iteration zero and wins later

Blend sweep against a RANDOM champion (iteration zero), 10 replicates, horizon 10:

| blend | mean | 95% CI |
|---|---|---|
| 0.00 | 0.5574 | [0.5455, 0.5693] |
| 0.25 | 0.5637 | [0.5550, 0.5724] |
| 0.75 | 0.5617 | [0.5512, 0.5722] |
| 1.00 | 0.5461 | [0.5327, 0.5594] |

All above 0.5 — training a random net helps regardless — and 0 through 0.75 are
statistically indistinguishable. So the blend costs NOTHING at iteration zero.

Set against the trained-champion sweep (blend 0 -> 0.4805, blend 0.75 -> 0.5258), a FIXED 0.75
is correct throughout and no ramp is required. That is worth knowing because the obvious design
— ramp the blend as the search becomes trustworthy — would have added a schedule, and a
schedule is another constant to get wrong. The measurement says a constant suffices.

TWO SIDE POINTS:
- blend 1.0 is the WORST arm here, which is the only place all day that pure self-reference has
  shown a cost. It supports the anchor argument used to pick 0.75 over 1.0 — an argument I
  explicitly recorded as a judgement the data did not support at the time. It does now, at the
  end of the curve where the search is least trustworthy.
- the original random-champion horizon sweep that produced "peak at 20" ran at blend 0. Whether
  removing the cap is safe at iteration zero is the last open piece; running it now.

## 2026-09-08 — search track PAUSED: it cannot discriminate at the only depth it can afford

294 candidates across ~12 generations, on a correct and fair mutation operator set:

    failed_oracle  154    rejected for being WRONG (disagreed with full-width reference)
    no_evidence    130    reached the games and the gate could not tell them apart
    lost_on_games   10    genuinely refuted
    accepted         0

Of 140 candidates that reached the game gate, **121 drew every pair** — the gate produced no
information on 86% of the candidates it was given.

That is not a defect in the search; it is the depth-1 limit already recorded in GRAMMAR 8,
observed directly. At depth 1 there is almost nothing for a search PROGRAM to do differently:
cutoffs, ordering and transpositions all pay at depth >= 2. And depth 2 is unaffordable — the
seed's own full search costs 37.3M cost units per position, about 2.7s per move, so one 160-ply
pair is ~14 minutes.

So the track is burning a core to confirm a limit it has already established. PAUSED rather
than deleted; every piece of it is verified and will be needed:
  - the oracle rejects 52% of well-typed candidates for being incorrect, which is the stage
    that stops "cheaper because it searches less" from reading as an improvement;
  - the operator draw is uniform across GRAMMAR 4's declared set;
  - the sequential gate stops after 16 all-drawn pairs instead of spending the full cap.

It resumes when a candidate can be judged at a depth where search technique exists. On today's
measurements that needs a cheaper SEARCH, not a cheaper interpreter — the interpreter is
already at 0.971x of hand-written Rust.

## 2026-09-08 — the horizon filter should be REMOVED, not tuned (at blend 0.75)

Full sweep against the trained champion at blend 0.75, 10 replicates per arm:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 10 | 8,854 | 0.5352 | [0.5265, 0.5438] |
| 20 | 16,830 | 0.5539 | [0.5445, 0.5633] |
| 40 | 32,229 | 0.5988 | [0.5846, 0.6130] |
| 80 | 53,775 | 0.6074 | [0.5930, 0.6219] |
| 160 | 58,734 | 0.6215 | [0.6052, 0.6378] |
| 1000 | **58,734** | 0.6242 | [0.6057, 0.6428] |

h160 and h1000 have the SAME sample count, so no decided position lies beyond 160 plies and
those two arms are the same dataset — the filter is inert past 160 and the curve has saturated,
not peaked.

**So the horizon filter is not a knob to tune, it is a restriction to remove.** Monotone
improvement all the way to "use every decided position", 0.5352 -> 0.6242.

The arc of this constant today, which is worth keeping as a caution about one-dimensional
sweeps:
1. shipped cap 40 (n=1, uncapped looked catastrophic)
2. swept at blend 0, found a peak at 20, shipped 20
3. swept at blend 0 against a TRAINED champion, found the whole axis below 0.5 and declared
   the schedule "backwards" and the axis "exhausted"
4. shipped blend 0.75 for unrelated reasons
5. re-swept: the optimum inverted, and the correct setting is no cap at all

Every step was measured. Steps 2 and 3 were measured under a blend that made the answer
meaningless, and nothing in the measurement itself could reveal that — only changing the OTHER
constant did.

## 2026-09-08 — horizon at blend 0.75: monotone up, plateau from ~40

Completed sweep against the trained champion, blend 0.75, 10 replicates per arm:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 10 | 8,854 | 0.5352 | [0.5265, 0.5438] |
| 20 | 16,830 | 0.5539 | [0.5445, 0.5633] |
| 40 | 32,229 | 0.5988 | [0.5846, 0.6130] |
| 80 | 53,775 | 0.6074 | [0.5930, 0.6219] |

h80 - h40 is 0.0086 +/- 0.0204: not significant. The curve rises steeply to 40 and then flattens.

Put beside the blend-0 sweep, the full picture is that these two constants define a plane and
the loop was sitting in its worst corner:

| | blend 0 | blend 0.75 |
|---|---|---|
| narrow (h10) | 0.4867 | 0.5352 |
| wide (h80) | 0.3977 | **0.6074** |

The shipped configuration was blend 0, horizon 20 -> 0.4648. The measured best corner is
blend 0.75, horizon 40-80 -> ~0.60. Every arm at blend 0 is below 0.5 (training makes the
champion worse); every arm at blend 0.75 is above it.

That also explains why tuning the horizon alone looked hopeless this morning: at blend 0 the
whole axis tops out at "no change", so the knob genuinely had no good setting. It had no good
setting because the OTHER knob was wrong.

h160 and h1000 running to decide whether a cap should exist at all -- if the plateau holds,
the schedule needs no cap and `--horizon-cap` becomes a safety rail rather than a tuning knob.

## 2026-09-08 — RETRACTION: the horizon schedule is NOT backwards. It was disabled by blend=0.

The horizon optimum REVERSES with the blend. Same trained champion, same data, 10 replicates:

| horizon | blend 0.00 | blend 0.75 |
|---|---|---|
| 10 | 0.4867 | 0.5352 |
| 20 | 0.4648 | **0.5539** |
| 40 | 0.4203 | **0.5988** |
| 80 | 0.3977 | (pending) |

At blend 0 every arm is below 0.5 and NARROWER is better. At blend 0.75 every arm is above 0.5
and WIDER is better, monotonically, in the opposite direction.

**This retracts "the horizon schedule is backwards", recorded a few hours ago.** That entry
argued the schedule's widening was "an active harm that grows with generation" and that the cap
was "treating a symptom". The measurement behind it was real; the conclusion drawn from it was
scoped to blend = 0 and I did not say so, because I did not yet know the blend mattered.

The mechanism is now clear and the loop's original design was right:
- with an OUTCOME label, a position 40 plies from the end is labelled by a result that had
  little to do with it. Noise grows with distance, so narrow wins.
- with a SEARCH-SCORE label, distance from the terminal is nearly irrelevant — a depth-2 search
  is about as informative at ply 40 as at ply 10. The extra positions are extra signal.

So `horizon = 10 + 5*(g-1)` widening with generation is CORRECT, and the comment justifying it
("the label becomes informative further back as play improves") was right for a reason it did
not state: it becomes informative once the label is a search score.

CONSEQUENCE: the shipped horizon default of 20 is measured WRONG under the shipped blend of
0.75 — h40 beats it by 0.045, roughly 6 SE. Waiting on h80 before changing it, since the curve
is still rising and I have already shipped two horizon defaults today on incomplete sweeps.

Also invalidates today's datagen-depth runs: both were at horizon 10, now known to be well
below the optimum, and at 1500 games where sample count is itself limiting.

## 2026-09-08 — operator fairness: CONFIRMED live, after three layers of the same bug

The search track's operator draw is now uniform. 249 proposals on a freshly built binary:

| operator | count | share |
|---|---|---|
| ReplaceConst | 41 | 16.5% |
| Dup | 41 | 16.5% |
| WrapIf | 33 | 13.3% |
| Delete | 32 | 12.9% |
| SwapSiblings | 31 | 12.4% |
| Tweak | 27 | 10.8% |
| InsertMax | 24 | 9.6% |
| WrapLoop | 20 | 8.0% |

Expected 12.5% each; at n=249 that is 31 +/- 5.5 per bucket at 1 sigma, so an 8.0-16.5% range
is chance. Before: **InsertMax 38%, WrapIf 3%** — a 12x spread, now 2x.

It took three fixes, and each one revealed the next:
1. **Selection was a race.** A fresh random operator was drawn on every retry and whichever
   applied first was kept, so usage was proportional to how many node types an operator
   accepts. Tweak appeared 0 times in 67 proposals.
2. **The draw itself was skewed.** With the operator chosen before placement, the distribution
   was still Delete 40 / InsertMax 39 / SwapSiblings 2. `Rng::new` was `Rng(seed | 1)` with no
   warmup, and the search seeds a fresh generator per candidate from a small structured value;
   a bare xorshift64's first output correlates with its seed. splitmix64 finalizer fixed it.
3. **The running process had a stale binary.** The ledger still showed InsertMax 38% because
   the isolated build tree's last build was the RNG negative control. Restoring source is not
   deploying it. (run.sh now builds and asserts freshness in the tree it executes.)

TWO SIDE EFFECTS, both good and neither predicted:
- `0 ill-typed` per generation, down from 2. Try-every-position no longer abandons an operator
  that is merely hard to place.
- Oracle rejection fell from ~70% to 9 of 24. The operators that used to dominate were the most
  destructive ones, so a fair draw sends more candidates to the gate — the search got cheaper
  per useful candidate as a consequence of being fair.

## 2026-09-08 — datagen depth is a NULL at blend 0, and the reason is structural

| datagen depth | decisive | samples | mean | 95% CI |
|---|---|---|---|---|
| 2 | 415/1500 (28%) | 3,345 | 0.4703 | [0.4622, 0.4784] |
| 3 | 948/1500 (63%) | 7,733 | 0.4703 | [0.4521, 0.4885] |

Identical means, despite depth 3 producing **more than twice the decisive rate** — much better
play, same training result.

The reason is structural rather than empirical, and I should have seen it before running the
sweep: **at blend = 0 the stored root score is never read.** The training target is the game
outcome alone, so search depth can only change WHICH GAMES ARE PLAYED, never what the label
says about them. `--deepen-at` defaulting to 1,000,000 meant the deepening the code calls
"AlphaZero's engine of improvement" had never run — but running it changes nothing while the
label ignores the search.

So the two knobs are COUPLED and I tested them independently: deeper search is worth more
precisely when the target includes the search score. Re-running depth 2/3/4 at blend 0.75.

This is the same shape as the blend finding itself. The loop had two halves of one mechanism —
a search score worth trusting, and a target that reads it — and both were switched off, each
for a reason that made sense at iteration zero.

## 2026-09-08 — BLEND: training flips from harmful to beneficial. The stall was the LABEL.

Same trained champion, same data, horizon 10, 10 replicates per arm. Only the training TARGET
differs: `(1-blend) * game_outcome + blend * own_root_score`.

| blend | mean | 95% CI | |
|---|---|---|---|
| 0.00 | 0.4805 | [0.4722, 0.4887] | significantly WORSE than the champion |
| 0.25 | 0.4898 | [0.4799, 0.4997] | worse |
| 0.50 | **0.5188** | [0.5114, 0.5261] | **BETTER, excludes 0.5** |
| 0.75 | **0.5258** | [0.5123, 0.5393] | **BETTER, excludes 0.5** |

0.75 vs 0.00 is **+0.0453 +/- 0.0158**, monotone across the sweep.

**This is the acceptance stall, and it was never a horizon problem.** Every horizon arm was
below 0.5 because the LABEL was wrong, not because the wrong positions were selected. I swept
the horizon from 3 to 1000 and the whole axis topped out at "no change"; changing one term in
the target moves it to a measured gain.

The loop hardcodes `blend = 0.0`, and the comment says why: mixing the net's own root score
into its target is self-referential WHEN THE NET IS RANDOM, so it "teaches nothing". True at
iteration zero, and it stopped being true the moment the net was trained — but the constant
never moved, and nothing re-tested it. MASTER_PLAN lists "Objectives: game outcome; agreement
with own deeper search" in the GIVEN column: half of the declared objective was switched off.

The mechanism is AlphaZero's: the search score is a lower-variance target than a single game
outcome, because it summarises a subtree rather than one playout. It only becomes a BETTER
target once the search is worth trusting — which is the same condition the code comment
describes for deepening, and which nothing had checked had arrived.

Peak not yet located: 0.75 is the highest arm tested and the curve is still rising. blend = 1.0
is pure self-reference (train toward what the net already says) and must be degenerate, so
there is a peak between. Refining before changing the default.

## 2026-09-08 — horizon tuning is EXHAUSTED: the best case is neutral, never a gain

Completing the sweep against the trained champion with narrower arms:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 3 | 3,235 | 0.4977 | [0.4851, 0.5102] |
| 5 | 4,845 | 0.4797 | [0.4648, 0.4946] |
| 10 | 8,854 | 0.4867 | [0.4688, 0.5047] |
| 15 | 12,849 | 0.4664 | [0.4531, 0.4797] |
| 20 | 16,830 | 0.4648 | [0.4482, 0.4815] |
| 40 | 32,229 | 0.4203 | [0.4052, 0.4355] |
| 80 | 53,775 | 0.3977 | [0.3904, 0.4049] |

**Across the entire swept range, 3 to 1000, no setting produces a gain.** The best arms (h3,
h10) have intervals touching 0.5 — indistinguishable from not training at all. Everything wider
is significantly worse.

So the horizon is a DAMAGE knob, not a strength knob: it controls how much training on this
data hurts, and its optimum is "hurt least". That closes it as a lever and moves the question
somewhere else entirely — the champion has extracted what this data distribution contains, and
no filter over the same positions recovers more.

The remaining candidates are the ones that change WHAT THE LABEL IS or WHAT THE DATA IS, not
which subset of it is used:
  - the label. blend = 0 hardcodes "train on the game outcome only". The stated reason is that
    mixing the net's own root score is self-referential WHEN THE NET IS RANDOM — a premise that
    expired the moment the net was trained. MASTER_PLAN lists "agreement with own deeper
    search" as a Given objective. Testing now.
  - the data. Self-play by a converged champion revisits what it already knows; the openings
    are 4 random plies.
  - capacity. Width 16, and the ARCH arm walked DOWN to it under a clock gate.

## 2026-09-08 — the horizon SCHEDULE is backwards, and the champion has converged

Same sweep, but against a TRAINED champion (champion_ep3_s20260907, width 16) instead of a
random one. 10 replicates per arm, one shared dataset, 1082/4000 decisive:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 10 | 8,854 | 0.4867 | [0.4688, 0.5047] |
| 20 | 16,830 | 0.4648 | [0.4482, 0.4815] |
| 40 | 32,229 | 0.4203 | [0.4052, 0.4355] |
| 80 | 53,775 | 0.3977 | [0.3904, 0.4049] |

**TWO findings, and both matter more than the horizon constant.**

1. **EVERY arm is below 0.5.** Training the trained champion on fresh self-play data makes it
   WORSE at every horizon tested. Only h10 has an interval touching 0.5; the rest are
   significantly worse. This is the acceptance stall, quantified: the champion has converged
   with respect to this data distribution, and more of the same data degrades it.

2. **THE SCHEDULE IS BACKWARDS.** `horizon = 10 + 5*(g-1)` widens with generation, on the
   stated theory that "the label becomes informative further back as play improves". Measured
   against exactly the condition that theory describes — an improved player — wider is
   MONOTONICALLY WORSE, and the gradient is steep (0.4867 -> 0.3977 from h10 to h80).

   The random-champion sweep peaked at 20; the trained-champion sweep peaks at the narrowest
   arm tested. The optimum moved the OPPOSITE direction from the one the schedule assumes.

Consequence: the widening schedule is not a refinement, it is an active harm that grows with
generation — consistent with the eight-generation collapse seen in the very first uncapped run,
which I attributed to the cap being absent rather than to the schedule being wrong.

Narrower arms (3, 5, 10, 15) are running to locate the optimum, and the schedule direction
should be re-derived from that rather than patched.

## 2026-09-08 — horizon SWEPT: 20 is the optimum, and 40 (my pick) was measurably worse

10 replicates per arm, one shared dataset, filter applied per arm:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 10 | 5,052 | 0.5527 | [0.5470, 0.5585] |
| **20** | **9,418** | **0.5660** | **[0.5599, 0.5721]** |
| 40 | 17,266 | 0.5371 | [0.5289, 0.5454] |
| 80 | 27,303 | 0.5320 | [0.5208, 0.5433] |
| 1000 | 30,151 | 0.5188 | [0.5084, 0.5291] |

Unimodal, peak at 20. **20 vs 40 = 0.0289 +/- 0.0102, excluding zero.** The default I shipped
this morning was measurably worse than an untested neighbour I had named in the comment as
untested and then not tested.

h20 beats h40 on 45% FEWER samples, so again quality over quantity — the same shape as the
capped-vs-uncapped result, now with the peak located rather than just bounded.

DECLARED LIMIT: measured against a RANDOM champion, i.e. early in a run. The schedule widens
the horizon with generation *because* the label becomes informative further back as play
improves, so this optimum should MOVE. It sets the cap early generations run into; it is not a
claim about a strong champion, and re-measuring against a trained champion is the obvious
follow-up.

## 2026-09-08 — horizon cap 40: CONFIRMED on a controlled A/B (and it was shipped on n=1)

Same champion, ONE raw generation (4000 games, 40,202 decided positions), the horizon filter
applied per arm so both see the same games, 10 replicates each:

| arm | samples | mean | sd | 95% CI |
|---|---|---|---|---|
| horizon 40 | 17,266 | **0.5371** | 0.0133 | [0.5289, 0.5454] |
| horizon 1000 (uncapped) | 30,151 | 0.5188 | 0.0167 | [0.5084, 0.5291] |

Difference **0.0183 +/- 0.0133** at 95% -> [0.0050, 0.0316], excludes zero. Significant.

**The capped arm wins on 43% FEWER samples.** So this is not data quantity, it is data QUALITY:
labels far from the terminal are anti-signal while play is weak. The code comment beside the
constant asserted exactly that and had never demonstrated it.

This was shipped as a default this morning on n=1 per arm, before the run-to-run variance
(0.151) was known -- i.e. on evidence I spent the afternoon refusing from --steps-per-gen. It
now has evidence that survives the standard. The n=1 caveat in main.rs is superseded and
`run_horizon_experiment.sh` (3 loop seeds) is no longer needed: the loop is the wrong instrument
for this question, at 25x worse resolution than the A/B.

Worth noting against the entry above: the SAME harness refuted the step budget and confirmed
the horizon. It is discriminating, not merely returning nulls.

## 2026-09-08 — mate-in-2 surrogate: NULL, the set is too rare to build

The mate-in-1 surrogate is saturated: the seed's terminal guard fires before its depth guard,
so every program finds all of them without searching and the count filter compares 0 < 0
forever. Mate-in-2 needs real lookahead, so it should discriminate — a program that prunes
unsoundly misses it, which is the failure FITNESS 3 exists to catch before games are spent.

Built it (forward search: a move such that for every reply, some follow-up mates; sparse
positions so depth 3 stays cheap). MEASURED: **400,000 random walks produced 2 positions.**
Mate-in-1 needs ONE winning move to exist; mate-in-2 needs EVERY reply to lose, which is orders
of magnitude rarer on positions reached by random play.

A 2-position surrogate carries no signal, so the default reverts to mate-in-1 plus the
mates-per-COST rate check, which does discriminate — on waste rather than on correctness. The
builder is kept behind `--mate2` because it works; what failed is finding enough instances
cheaply, not the idea. A retrograde construction (start from a mated position, unwind three
plies) would produce them by the thousand and is the obvious next attempt if this surrogate
ever needs to bite.

## 2026-09-08 — step budget: REFUTED on a controlled A/B. The loop signal was trajectory noise.

Same champion, ONE shared dataset (4000 games, 17,266 training samples), 8 replicates per arm
differing only in the hyperparameter and the training seed, each gated against that champion:

| arm | updates | mean | sd | 95% CI |
|---|---|---|---|---|
| epochs 3 | ~51,798 | **0.5444** | 0.0168 | [0.5328, 0.5561] |
| steps-per-gen 20000 | 20,000 | 0.5352 | 0.0167 | [0.5236, 0.5467] |

Difference 0.0092 +/- 0.0163 at 95%. **Not significant, and the sign is REVERSED** from the
loop, where steps led on 2 of 2 seeds.

**This refutes the mechanism, not just the effect.** The morning diagnosis was that raising
self-play volume 125x pushed gradient steps per generation from ~200 to ~110,000 and was
overwriting the champion each cycle. If that were right, the arm doing 2.6x MORE updates should
be worse. It is nominally better.

**And it shows why the loop could not answer this.** The standard error here is 0.0059 against
the loop's 0.151 run-to-run spread -- a 25x improvement in resolution, from removing path
dependence rather than from more compute. The loop's apparent effect was the trajectory, which
is exactly what the variance measurement predicted.

`--steps-per-gen` stays defaulted OFF and is now off for a measured reason. Kept in the code
because the harness that tests it is worth more than the flag.

WHAT THIS DOES NOT SHOW: whether a step budget matters over MANY generations. A single-step A/B
cannot see a compounding effect. But the burden has moved -- there is no longer a measured
single-step benefit to compound.

## 2026-09-08 — mate-in-2 surrogate: NULL, the set is too rare to build

The mate-in-1 surrogate is saturated: the seed's terminal guard fires before its depth guard,
so every program finds all of them without searching and the count filter compares 0 < 0
forever. Mate-in-2 needs real lookahead, so it should discriminate — a program that prunes
unsoundly misses it, which is the failure FITNESS 3 exists to catch before games are spent.

Built it (forward search: a move such that for every reply, some follow-up mates; sparse
positions so depth 3 stays cheap). MEASURED: **400,000 random walks produced 2 positions.**
Mate-in-1 needs ONE winning move to exist; mate-in-2 needs EVERY reply to lose, which is orders
of magnitude rarer on positions reached by random play.

A 2-position surrogate carries no signal, so the default reverts to mate-in-1 plus the
mates-per-COST rate check, which does discriminate — on waste rather than on correctness. The
builder is kept behind `--mate2` because it works; what failed is finding enough instances
cheaply, not the idea. A retrograde construction (start from a mated position, unwind three
plies) would produce them by the thousand and is the obvious next attempt if this surrogate
ever needs to bite.

## 2026-09-08 — step budget: INTERIM, 2 of 3 seeds, and the SEED VARIANCE dominates

Control vs the frozen origin at generation 10:

| seed | epochs 3 | steps-per-gen 20000 | diff |
|---|---|---|---|
| 20260907 | 0.641 +/- 0.045 | 0.756 +/- 0.043 | +0.115 (intervals separated) |
| 424242 | 0.792 +/- 0.032 | 0.822 +/- 0.032 | +0.030 (intervals overlap) |

At generation 20 both arms land in the low 0.8s on both seeds and nothing separates.

**Direction is consistent — steps ahead 2 of 2 — but the effect is not the interesting number.
THIS is:** epochs-3 scored **0.641 on one seed and 0.792 on the other, at identical settings**.
A 0.151 spread between runs that differ only in seed, against a treatment effect of 0.030 to
0.115.

The between-run variance is LARGER than the thing being measured. That has a direct
consequence for the design: three seeds is not enough. To resolve a 0.03 effect against a 0.15
run-to-run spread needs roughly (0.15/0.03)^2 = 25 runs per arm, not 3. The n>=3 rule was
written to stop me believing single runs; it does not by itself make an effect of this size
measurable.

So the honest statement when seed 3 lands will be about DIRECTION (2 or 3 of 3 favouring the
fixed step budget) and not about magnitude — and even the direction is weak evidence from three
paired samples. `--steps-per-gen` stays defaulted OFF until either the effect is bigger or the
sample is.

This also retroactively explains the two identical-setting runs that disagreed earlier today
and started this whole experiment. They were not evidence of a step-budget effect at all; they
were two draws from a distribution this wide.

## 2026-09-08 — mate-in-2 surrogate: NULL, the set is too rare to build

The mate-in-1 surrogate is saturated: the seed's terminal guard fires before its depth guard,
so every program finds all of them without searching and the count filter compares 0 < 0
forever. Mate-in-2 needs real lookahead, so it should discriminate — a program that prunes
unsoundly misses it, which is the failure FITNESS 3 exists to catch before games are spent.

Built it (forward search: a move such that for every reply, some follow-up mates; sparse
positions so depth 3 stays cheap). MEASURED: **400,000 random walks produced 2 positions.**
Mate-in-1 needs ONE winning move to exist; mate-in-2 needs EVERY reply to lose, which is orders
of magnitude rarer on positions reached by random play.

A 2-position surrogate carries no signal, so the default reverts to mate-in-1 plus the
mates-per-COST rate check, which does discriminate — on waste rather than on correctness. The
builder is kept behind `--mate2` because it works; what failed is finding enough instances
cheaply, not the idea. A retrograde construction (start from a mated position, unwind three
plies) would produce them by the thousand and is the obvious next attempt if this surrogate
ever needs to bite.

## 2026-09-08 — step budget: INTERIM, seed 1 of 3

First arms to complete with the derived per-net gate budget (the earlier attempts aborted on
the coverage guard). Control vs the frozen origin, seed 20260907:

| gen | epochs 3 | steps-per-gen 20000 |
|---|---|---|
| 10 | 0.641 +/- 0.045 | **0.756 +/- 0.043** |
| 20 | 0.809 +/- 0.040 | **0.834 +/- 0.035** |

At generation 10 the intervals do not overlap ([0.596, 0.686] vs [0.713, 0.799]), which is a
real separation favouring the fixed step budget. By generation 20 they overlap and the
difference is not significant.

**NOT A RESULT YET, and the reason is written above in this file.** This is n=1. Two runs at
IDENTICAL settings disagreed earlier today — that is what started this experiment — so a single
seed showing a clean separation is exactly the evidence that has already misled me once. Seeds
424242 and 987654 are running. The claim waits for the pooled three.

What IS established independently of the arms: both now compound strongly (0.809 and 0.834
against the origin at gen 20, against 0.694 in the earlier run), because the derived budget
gives the gate 100% coverage instead of the 33-57% a fixed 4000 nodes happened to produce.

## 2026-09-08 — FIRST SEARCH-TRACK RESULT: 128 mutations, 0 accepted

The search track ran end to end for the first time. 8 generations x 16 candidates against the
bare alpha-beta seed, gated on games at equal cost budget:

    106 well-typed, 22 ill-typed (type checker rejected them before any compute)
     90 failed the correctness oracle
     16 reached the game gate
      0 beat the champion

**This is the expected outcome and it is a measurement, not a failure.** A single random
mutation of a 71-node program that already computes the exact minimax value has almost no way
to improve it; GRAMMAR 6 puts the nearest real milestone (hash reuse) at +104 nodes, which is
not one mutation away. What the run establishes is that the PIPELINE works: candidates are
generated, ill-typed ones are rejected for free, incorrect ones are caught before spending
games, and the survivors are judged by play.

The oracle is doing the heavy lifting -- 90 of 106 well-typed candidates were REJECTED FOR
BEING WRONG, i.e. they returned a move the full-width reference disagreed with. Without that
stage every one of them would have gone to the gate, and the cheap-but-worse ones would have
been indistinguishable from genuine improvements on a cost-based metric.

TWO BUGS THE RUN EXPOSED, both in my harness rather than in the idea:

1. THE INTERPRETER NEVER ENFORCED ITS BUDGET (see above). One mutant looped for four hours.
2. THE SURROGATE WAS INERT. The seed scored 0/40 on the mate-in-1 set, so the filter compared
   0 < 0 and passed everything. Cause: the surrogate ran at the GAMES depth (3), where a
   single position costs ~411M cost units (ladder), overshooting the 2e9 safety cap and
   forfeiting. Mate-in-1 needs one ply. Given its own --surrogate-depth (default 2) it now
   scores 40/40 with 0 forfeits, and an assertion aborts the run if the seed ever fails its
   own surrogate again -- an inert filter that silently passes everything is worse than no
   filter, because it looks like a stage.

## 2026-09-08 — the cost model was never built, and it inverted a published result

GRAMMAR 8 and CRATE 4 both specify a per-primitive cost table (`configs/cost.toml`).
Neither existed; the interpreter charged a flat `self.cost += 1` per node, so a full NNUE
forward pass cost exactly what `const 3` cost. That is not neutral — it is a thumb on the
scale against any program that spends cheap work to avoid expensive work, which is exactly
a transposition table's trade. MEASURED (examples/cost_calibrate.rs, width 32, min-of-5):

| primitive | cost | |
|---|---|---|
| arith / cmp / const / var | 1 | |
| key (zobrist) | 97 | |
| terminal | 703 | |
| apply | 788 | |
| eval | **1365** | 293 ns |
| moves (legal_moves) | 2232 | |

Re-derived ladder, hash reuse relative to the seed: flat 1.250x (D2) / 1.218x (D3) becomes
**1.04x / 1.01x**. The 25% penalty was the instrument. RETRACTS the magnitude of the
"break-even near depth 5-6" claim and the chicken-and-egg constraint written into
MASTER_PLAN from it; the direction (overhead falls with depth) survives.

## 2026-09-08 — a fixed gate budget is wrong when tree size is NET-DEPENDENT

The 3-seed experiment reported ALL ARMS COMPLETE with 4 of 6 arms at zero generations. The
gate-coverage guard had aborted them: a fixed 4,000-node budget covered 57% of one seed's
depth-4 tree and 33% of another's, because different random nets produce different
alpha-beta cutoffs and therefore different tree sizes. The guard was RIGHT — it refused to
run gates that would return confident-looking 0.500s. `--cost-nodes` now derives from the
measured full tree at the cap depth.

The second half is worse and is a repeat: the wrapper never checked exit codes, so four
aborts printed as success. The 4PC queue runner already carries this exact lesson — "a
failed EXPERIMENT is a result; a failed SCRIPT is a bug."

## 2026-09-08 — the search track exists, and its oracle caught its own bug first

`evolve_search.rs`: mutate -> CORRECTNESS ORACLE (full-width negamax agreement) -> mates-per-cost
surrogate -> GAME GATE (candidate program vs champion program, same net, equal COST budget,
pentanomial). The previous `evolve.rs` had only the surrogate, scored against a random net.

First run printed `seed: bare alpha-beta, 71 nodes; oracle 7/10`. A seed failing its own
oracle is impossible — alpha-beta returns the full-width minimax value by construction. The
reference was a ply shallow: the seed's `choose` expands the ROOT itself then searches D more
plies, and I called the reference with depth-1. That is the SAME off-by-one GRAMMAR 8 records
behind the bogus "0.14x" interpreter reading. Fixed; seed now 10/10.

## 2026-09-07 — the ones that held

- **Self-play VOLUME.** 80 -> 10,000 games/generation. Training samples 33-267 ->
  12,212-106,485; the per-generation gate went from all-drawn 0.500 to 20W-44D-0L
  (0.656 +/- 0.057). This is the change that made the gate able to resolve at all.
- **Horizon cap 40.** Uncapped, the gate collapsed once the horizon passed 60 plies
  (eight straight generations below 0.5, four flagged `regression`). Capped, the champion
  compounded 0.552 -> 0.567 -> 0.570 -> 0.694. CAVEAT: also n=1 per arm; the capped arm's
  own trajectory is solid, the cross-arm claim is suggestive.
- **Gate depth cap 6 -> 4.** A 4,000-node budget bought 1.29% of a depth-6 tree, so the
  search never finished its first root move and both sides played near-randomly. Every
  budgeted gate was returning a confident-looking 0.500 that meant nothing.

## 2026-09-07 — the ones that failed, with the reason

- **`ab_hash` was not a transposition table.** Never called `eval`; returned constant 0 at
  every leaf. Its ladder "confirmation" of a 3.2x gain was the bug. Repaired: 175 nodes,
  and hash reuse measures a LOSS at D2-D4 that shrinks with depth (1.250x -> 1.113x),
  break-even near depth 5-6.
- **"Swap ladder steps 4 and 5."** Reasoned that iterative deepening creates the traffic a
  table needs. REFUTED: ID's cost is flat at ~1.086x across D2-D4 and hash+ID is worse
  than hash alone everywhere. Reverted in GRAMMAR 9 and MASTER_PLAN.
- **"Emission order gives alpha-beta a free ordering heuristic."** REFUTED in the
  informative direction: it is an ANTI-ordering costing +16/+31/+54% nodes at depth 3/4/5.
- **"Deeper random openings will make games decisive."** Null: 8.3% decisive at 4 plies,
  15.0% at 32, inside the noise at n=60. The lever was volume, not opening depth.

---

## 2026-09-09 — the gate was fixed, and it cleared the real suspect

- **Sequential gate: VERIFIED, and it ACCEPTS.** FITNESS 7 specifies SPRT; the shipped gate was a
  fixed 6-pair match that had never accepted anything in 203 decisions. `sprt_smoke` ran both
  pre-registered checks on the real binary: A/A (seed vs itself, must NOT accept) returned
  `Inconclusive llr +0.00 after 30 pairs, W-D-L 11-38-11` -- the zero-variance give-up firing as
  designed -- and A/B (seed vs `depth_one`, must decide) returned `Accept llr +3.08 after 26 pairs,
  W-D-L 34-18-0`. First accept this gate has ever produced.

- **Gate bounds `[0,10]` -> `[0,30]`. The WIDTH was the cost driver, not the stopping rule.** The
  LLR scales with `(p1-p0)`, so a 10-Elo width gives each pair 0.0144 of evidence and the test
  crawls. Simulated with the exact formula from `gate.rs` at the measured 0.806 draw rate, 400 runs
  per cell: `[0,10]` burns the full 400-pair cap **63.8%** of the time against a null candidate,
  median 274 pairs. `[0,30]` burns it **0.0%**, median 55, false-accepts 4.8% (alpha=0.05), power
  81.8% at +25 Elo and 98.5% at +50. `[0,50]`/`[0,100]` are cheaper still but their power collapses
  at +25 (39.8%, 13.5%) -- they would discard real gains. **Deviates from FITNESS 7.2's 2-Elo width,
  flagged as open.**

- **FAILED: decisive openings via a material gap.** FITNESS 7.3 asks for an unbalanced book. Probe at
  24 pairs: control (balanced) **8-32-8, 66.7% draws**; treatment (material gap >= 2) **9-30-9,
  62.5%**. Two games on 48 -- noise. Pentanomial was `[0,0,24,0,0]` in BOTH arms. **What was wrong
  with the experiment: nothing -- the idea does not transfer.** Both sides evaluate with
  `Net::random`, so neither can convert an edge it cannot see; `unbalanced_open.rs:4` had already
  said so. MASTER_PLAN:154's three legal sources all fail at iteration zero: random plies is what we
  run, the self-generated book needs a working eval to mine, and Chess960 is symmetric. That section
  targets draw-death **from strength**; ours is a random eval shuffling to a repetition. Same
  symptom, different cause. **Do not re-run the material-gap book.**

- **The surrogate proposes candidates that are WORSE — and that, not the gate, is why nothing was
  ever accepted.** With the gate cleared, VERIFY (96 pairs, independent seed) says: under the
  standard guard, MAIN **3/3 resolved WORSE** (0.422+/-0.027, 0.430+/-0.030, 0.430+/-0.031) while
  MCTS is **0/3** (0.490, 0.492, 0.505). Mechanism, with a natural experiment: MAIN's seed is
  **23/23 mates -- saturated** -- so "keep every mate, get cheaper" can only be satisfied by
  searching less, and cheapness is the axis that costs strength; MCTS's seed is **15/23**, not
  saturated, and does not degrade. `evolve.rs:1639` already stated the saturation; the known repair
  (swap mate-in-1 for a forced-mate set) is INCOMPLETE because 23/23 is still saturated. Acting on
  it via `EXISTENCE_HARD_FITNESS=1`, which was implemented, pre-justified, defaulted OFF, and had
  never once been enabled by any arm. A/B running on two seeds; falsifier pre-registered.

- **MY OWN ERROR, recorded because the do-not-regress list is also for method.** I first reported
  "4 of 4 MAIN resolved worse" plus a Spearman over n=7. Both wrong. Arms with identical
  configuration replay the SAME trajectory, so seed-1 gen-3 appeared in three log files and seed-2
  gen-1 in two, and I counted duplicates as independent observations; I also pooled a relaxed-guard
  arm (tolerance 7 / floor 16, where a candidate may SHED mates to buy cost) with the tolerance-4
  arms. Corrected to 3/3 and 0/3, Spearman withdrawn as uninterpretable at n=3. `ab_report.py` now
  does the dedup and stratification, reads the guard and the HARD_FITNESS flag from each log's own
  HEADER rather than its filename, and was validated against the corrected hand count before use.
