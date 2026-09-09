# The gate was never the bottleneck — the surrogate is

**Headline.** With the sequential gate now VERIFIED (it accepts a real improvement in 26 pairs), the
0/203 acceptance record resolves into a different cause: **the surrogate proposes candidates that are
genuinely worse.** 4 of 4 MAIN candidates that improved `mates/Mcost` were resolved WORSE by a 96-pair
independent-seed VERIFY. The gate has been doing its job correctly the whole time.

## The gate is verified (directive #1)

`sprt_smoke.log`, two pre-registered checks, both passed on the real binary:

    A/A  seed vs ITSELF (must NOT accept)   Inconclusive  llr +0.00  after 30 pairs  W-D-L 11-38-11
    A/B  seed vs depth_one (must decide)    Accept        llr +3.08  after 26 pairs  W-D-L 34-18-0

The A/A stopped at exactly 30 pairs -- the zero-variance give-up firing as designed -- and did not
accept. The A/B accepted an obvious improvement in 26 pairs rather than running to the cap. This is
the first ACCEPT this gate has ever produced.

## What VERIFY says about the surrogate

VERIFY is 96 pairs on an INDEPENDENT seed, so it is the strength reading, not the gate's 6.

    lineage  surrogate   gain vs seed   VERIFY          verdict
    MAIN     0.002817        +13.1%     0.430 +/-0.030  RESOLVED WORSE
    MAIN     0.002924        +17.4%     0.422 +/-0.027  RESOLVED WORSE
    MAIN     0.003045        +22.3%     0.430 +/-0.030  RESOLVED WORSE
    MAIN     0.004618        +85.5%     0.258 +/-0.041  RESOLVED WORSE
    MCTS     0.001506         +7.1%     0.505 +/-0.019  not resolved
    MCTS     0.001811        +28.8%     0.492 +/-0.018  not resolved
    MCTS     0.001848        +31.4%     0.490 +/-0.018  not resolved

Every MAIN candidate is resolved worse, and the LARGEST surrogate gain is the worst player by a wide
margin. Spearman(gain, VERIFY) is negative (pooled -0.357, MCTS -1.000) but **n=7 -- treat the
correlation as suggestive; the 4/4 resolved-worse count is the load-bearing fact.**

## Mechanism, and the natural experiment that supports it

The selection rule is "keep every mate, get cheaper". Whether that degenerates depends entirely on
whether the mate numerator has headroom:

- **MAIN seed = 23/23 mates. SATURATED.** The numerator cannot rise, so the only way to improve
  `mates/Mcost` is to cut cost -- i.e. search less. Every MAIN candidate is resolved WORSE.
- **MCTS seed = 15/23 mates. NOT saturated.** The numerator can rise, so selection is not forced onto
  the cost axis. Every MCTS candidate is neutral (0.490-0.505), not worse.

Saturation predicts which lineage degrades, and the data matches. The gen lines show the guard is
working as intended -- `8 cand, 0 ill, mate-ok 1` means 7 of 8 candidates are rejected for losing
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

It defaults OFF and **no arm had ever enabled it.** Now running as a one-variable A/B:

    core 12  EXISTENCE_HARD_FITNESS=1, SPRT [0,30]   <- treatment   gate_hardfit_s1.log
    core 15  SPRT [0,30], identical otherwise        <- control     gate_sprt30_s1.log
    core 13  SPRT [0,30] + EPS 0.10                                 gate_sprt30_eps_s1.log
    core 14  fixed 6-pair gate                       <- old-gate baseline

The prediction to falsify: if saturation is the cause, the treatment arm's candidates should stop
being resolved-worse by VERIFY. If they are still resolved worse, saturation is NOT the mechanism and
this document is wrong.
