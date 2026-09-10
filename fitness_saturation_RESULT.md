# The gate was never the bottleneck — the surrogate is

**Headline.** With the sequential gate now VERIFIED (it accepts a real improvement in 26 pairs), the
0/203 acceptance record resolves into a different cause: **the surrogate proposes candidates that are
genuinely worse.** Under the standard guard, **3 of 3 MAIN candidates that improved `mates/Mcost`
were resolved WORSE** by a 96-pair independent-seed VERIFY, while **0 of 3 MCTS candidates were.**
The gate has been doing its job correctly the whole time.

**Strength of evidence: 3 candidates per lineage.** That is enough to motivate the fix below and not
enough to settle it. The running A/B is the test that settles it.

## The gate is verified (directive #1)

`sprt_smoke.log`, two pre-registered checks, both passed on the real binary:

    A/A  seed vs ITSELF (must NOT accept)   Inconclusive  llr +0.00  after 30 pairs  W-D-L 11-38-11
    A/B  seed vs depth_one (must decide)    Accept        llr +3.08  after 26 pairs  W-D-L 34-18-0

The A/A stopped at exactly 30 pairs -- the zero-variance give-up firing as designed -- and did not
accept. The A/B accepted an obvious improvement in 26 pairs rather than running to the cap. This is
the first ACCEPT this gate has ever produced.

## What VERIFY says about the surrogate

VERIFY is 96 pairs on an INDEPENDENT seed, so it is the strength reading, not the gate's 6.

**STANDARD GUARD (tolerance 4, MAIN floor 19) — n=6 unique**

    lineage  seed gen  surrogate   gain      VERIFY          verdict
    MAIN      2   2    0.002817   +13.1%     0.430 +/-0.030  RESOLVED WORSE
    MAIN      1   3    0.002924   +17.4%     0.422 +/-0.027  RESOLVED WORSE
    MAIN      1   4    0.003045   +22.3%     0.430 +/-0.030  RESOLVED WORSE
    MCTS      2   1    0.001506    +7.1%     0.505 +/-0.019  not resolved
    MCTS      2   2    0.001811   +28.8%     0.492 +/-0.018  not resolved
    MCTS      1   4    0.001848   +31.4%     0.490 +/-0.018  not resolved

**RELAXED GUARD (tolerance 7, MAIN floor 16) — n=1, a DIFFERENT CONDITION**

    MAIN      2   2    0.004618   +85.5%     0.258 +/-0.041  RESOLVED WORSE

That last row is kept separate on purpose: with tolerance 7 the candidate may SHED up to 7 mates to
buy cost, which is not the same mechanism as "keep every mate, get cheaper". It is consistent with
the story -- loosening the guard makes play collapse further -- but it is not evidence for it.

**CORRECTION LOG.** The first version of this document reported "4 of 4 MAIN" and a pooled Spearman
over n=7. Both were wrong. Arms with identical configuration replay the SAME trajectory, so seed-1
gen-3 appeared in three separate logs and seed-2 gen-1 in two; those duplicates inflated the apparent
sample. It also pooled the relaxed-guard arm with the standard ones. After deduplication the honest
counts are 3/3 and 0/3, and **the Spearman is withdrawn as uninterpretable at n=3 per lineage.**

## Mechanism, and the contrast that supports it

The selection rule is "keep every mate, get cheaper". Whether that degenerates depends entirely on
whether the mate numerator has headroom:

- **MAIN seed = 23/23 mates. SATURATED.** The numerator cannot rise, so the only way to improve
  `mates/Mcost` is to cut cost -- i.e. search less. 3 of 3 MAIN candidates resolved WORSE.
- **MCTS seed = 15/23 mates. NOT saturated.** The numerator can rise, so selection is not forced onto
  the cost axis. 0 of 3 MCTS candidates resolved worse; all sit at 0.490-0.505.

Saturation predicts which lineage degrades, and the data matches. The gen lines show the guard itself
is working as intended -- `8 cand, 0 ill, mate-ok 1` means 7 of 8 candidates are rejected for losing
mates -- and then **cost alone picks the survivor**, along the one axis that costs real strength.

`evolve.rs:1639` already states this ("mates are SATURATED at 25/25, so the surrogate can only
improve via cost"), and `evolve.rs:48` records an earlier, more violent form of it (0.03 -> 8.73
mates/Mcost in one edit). The known repair was to swap the mate-in-1 set for a forced-mate set so
shallowness loses mates. **That repair is incomplete: 23/23 is still saturated.**

## Action

`EXISTENCE_HARD_FITNESS=1` (evolve.rs:1653) folds the hard set into the surrogate as
`(f + hf) / (cost + hard_cost)`. It charges the summed cost of BOTH sets, so hard-set solves cannot
be bought with bloat. Its precondition was already measured as met (20/39 lineage-generations contain
a member above zero; seed is 0/8 by construction), and today's independent count agrees: 93 of 287
gen-lines carry a non-zero hard score.

It defaults OFF and **no arm had ever enabled it.** Now running as a one-variable A/B on TWO seeds,
because a single run seed is a lottery and the falsifier is a small-n count:

    core 12  HARD_FITNESS=1, seed 1, SPRT [0,30]   <- treatment      gate_hardfit_s1.log
    core 14  HARD_FITNESS=1, seed 2, SPRT [0,30]   <- treatment      gate_hardfit_s2.log
    core 15  SPRT [0,30], seed 1, no hard fitness  <- live control   gate_sprt30_s1.log
    core 13  SPRT [0,30] + EPS 0.10, seed 1        <- directive #3   gate_sprt30_eps_s1.log

Core 14 previously held a fixed 6-pair gate as an old-gate baseline. That question is answered and no
longer earns a core: the old gate is 0/203 historically, produced 3 more REJECTs today, and
`sprt_smoke` settled the new gate directly. The control side of the comparison is the live seed-1 arm
plus the 3 historical readings already tabulated above, which span seeds 1 and 2.

The prediction to falsify: if saturation is the cause, the treatment arm's MAIN candidates should
stop being resolved-worse by VERIFY. If they are still resolved worse, saturation is NOT the
mechanism and this document is wrong.

## Cost note — one figure MEASURED, one still an ESTIMATE

**Measured.** Generations that produce no gate call cost ~3.5 min each (the two new arms both cleared
gens 1-2 in 6-7 min). The fixed-gate control averaged ~47 min/gen over its first four generations,
two of which were gated. Solving those together puts a GATED generation near ~90 min, and the gate
itself is only 12 games there -- so the cost is VERIFY (96 pairs = 192 games) plus the fitness
evaluation of 8 candidates, not the gate.

**Estimate, not measured.** An earlier version of this note claimed the sequential gate adds "~9-16
min, roughly +25%". That rests on a per-game cost of 5-9 s that was never pinned down: the smoke
binary is the only clean sample and it had logged >=9 min CPU for <=112 games when last observed
before exit, which is a LOWER bound of ~4.8 s/game with no upper bound. The +25% is therefore an
estimate and is labelled as one.

**The exact number arrives on its own.** Each arm's gen-3 VERIFY is a known 192-game unit of work
bounded by two log-line timestamps; that gives s/game directly, with no new load and no new harness.
Until then the affordability claim stands only as: the gate is a minority of a gated generation, and
a gated generation is ~90 min.

VERIFY runs unconditionally after every gate call, including rejects. That is NOT waste to be
optimised away -- it is the instrument that produced the 3/3-vs-0/3 reading above, and it is the
falsifier for the HARD_FITNESS arm. It stays.
