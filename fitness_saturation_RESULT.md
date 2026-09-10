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

    core 12  HARD_FITNESS weight 1, SPRT [0,30]   <- treatment, low dose   gate_hardfit_s1.log
    core 14  HARD_FITNESS weight 4, SPRT [0,30]   <- treatment, high dose  gate_hardw4_s1.log
    core 15  no hard fitness, SPRT [0,30]         <- control               gate_sprt30_s1.log
    core 13  EPS 0.10, SPRT [0,30]                <- his directive #3      gate_sprt30_eps_s1.log

**All four on run seed 1**, all on the same binary, all nice 19. Restarted together at 20:04 on the
no-ratchet build (see `search_track_WHY_NOTHING.md`), because that was a BEHAVIOUR change and a split
fleet would confound control against treatment worse than the defect did.

**What each arm is for, in one line each:**

- **15 vs 12** — does folding the hard set into the surrogate stop MAIN candidates being resolved
  worse? The original question.
- **12 vs 14** — is the fix merely UNDER-DOSED? The arithmetic predicts weight 1 cannot outbid
  cost-cutting and weight 4 can. If 12 is inert and 14 is not, the diagnosis was right and only the
  dose was wrong — a different repair from "saturation was the wrong mechanism".
- **13** — his directive #3, already shown live (pop 4 vs pop 2, retaining a `tt`-carrying member).

**Verified at launch, not assumed:** the no-ratchet change is inert until a REJECT (control matches
its pre-change log line-for-line, ignoring the header), the EPS arm is currently identical to the
control, and the two HARD_FITNESS arms differ only where their surrogate scale differs.

**A note on churn.** The fleet was restarted four times on 2026-09-09 — three print-only
instrumentation changes (`hard`, the self-describing header, `mates`) and one behaviour change (the
ratchet). Each restart cost ~10 min per arm and every one was taken while the arms had NO gated
generation, which is the expensive part. The instrumentation is now sufficient to read the result;
further changes should wait for a gate.

Core 14 previously held a fixed 6-pair gate as an old-gate baseline. That question is answered and no
longer earns a core: the old gate is 0/203 historically, produced 3 more REJECTs today, and
`sprt_smoke` settled the new gate directly. The control side of the comparison is the live seed-1 arm
plus the 3 historical readings already tabulated above, which span seeds 1 and 2.

The prediction to falsify: if saturation is the cause, the treatment arm's MAIN candidates should
stop being resolved-worse by VERIFY.

**CORRECTION to that falsifier, written 2026-09-09 19:29, BEFORE the treatment's first gate.** As
first stated it read "if they are still resolved worse, saturation is NOT the mechanism". That is
wrong, and the treatment arms' own logs show why: every generation so far reads **`hard 0-0`**, i.e.
no retained member scores anything on the hard set. With `hf = 0` the treatment's surrogate is
`(23+0)/(cost + hard_cost)` — improvable ONLY by cutting cost, which is exactly the control's
`23/cost` with a larger denominator. **The incentive is then IDENTICAL and the fix has not engaged
at all.** A "still resolved worse" reading in that state would say nothing whatever about saturation.

So the three readings are:

- **Treatment MAIN stops being resolved worse** → consistent with saturation, subject to the
  mechanism caveat below.
- **Treatment MAIN still resolved worse AND `hf > 0` reached the gate** → saturation is refuted as
  the mechanism, and this document is wrong.
- **Treatment MAIN still resolved worse AND `hf` stayed 0** → **UNINTERPRETABLE.** The fix never
  engaged. Not evidence either way.

**The third case is now distinguishable from the second**, as of the instrumentation above: the gate
line carries `hard {hlo}-{hhi}`, and `hhi == 0` marks the run where the fix never engaged. Before
that change a negative treatment reading would have been unreadable — which is why it was worth
doing immediately rather than after the result arrived.

One reason to expect engagement anyway: with `HARD_FITNESS=1` a candidate scoring `hf > 0` gets a
numerator boost, so it is MORE likely to win its generation and reach the gate. Historically 93 of
287 gen-lines carry a non-zero hard score, so the dimension does vary — it simply has not yet in
these two arms.

## WHEN the fix can engage — measured, so `hard 0-0` early is not alarming

`hard > 0` is not uniform across generations. Counted over every arm log on disk:

    gen   gens seen   with hard>0
      1          31             0      0%
      2          38             1    2.6%
      3          34             9     26%
      4          20            10     50%
      5          22            11     50%
      6          27            12     44%
      7          17             4     24%

**A hard-set solve essentially never appears before gen 3, and peaks around 50% at gens 4-6.** The
treatment arms currently read `hard 0-0` at gens 1-2, which is exactly what every other arm does and
therefore carries no information at all. The experiment cannot begin to engage until gen 3, and the
informative window is gens 4-6. **Do not read an early `hard 0-0` as "the fix does not work".**

### PRE-REGISTERED PREDICTION: the fix may be TOO WEAK to outbid cost-cutting

Written before any treatment gate exists. Recovering the cost split from the two arms' seed
surrogates (same program, same sets, so the only difference is the added hard-set cost):

    control   23e6 / C_m         = 0.002490  ->  C_m = 9,236,947,791
    treatment 23e6 / (C_m + C_h) = 0.002084  ->  C_h = 1,799,520,539  (19.5% of C_m)

The hard set's max achieved score is **2 of 8** (see the ladder below: 0 at gen 1, 1 at gens 2-5, 2
from gen 6). So the largest numerator boost the fix can offer is bounded:

    hf=1  ->  (23+1)/23  =  +4.3%
    hf=2  ->  (23+2)/23  =  +8.7%      <- the observed ceiling

Against that, cutting cost moves the DENOMINATOR, and the denominator now includes both sets:

    cut mate-set cost 10%  ->  +9.1%     <- already beats the best possible hard-set gain
    cut mate-set cost 20%  -> +20.1%
    cut mate-set cost 30%  -> +33.5%

And the cost-driven gains actually observed under the standard guard were **+13.1%, +17.4%, +22.3%**
— all of them larger than +8.7%.

**Prediction: cost-cutting still wins, and the treatment still selects cost-cutters.** If that holds,
the remedy is not to abandon the hard set but to WEIGHT it — count a hard solve as worth more than
one mate, or stop charging the hard set's cost to the same denominator — because the diagnosis
(a saturated numerator) would be right while the correction was simply under-powered. That is a
different repair from "saturation was the wrong mechanism", and the two must not be conflated.

### How to read `hard {hlo}-{hhi}` — the range does not always identify the winner

The gate line prints the range over guard-passing candidates, not the winner's own `hf`. So:

- `hhi == 0` → the fix definitely did NOT engage; the winner had `hf = 0`. **Unambiguous.**
- `hlo > 0` → every retained member scored, so the winner did too. **Unambiguous engagement.**
- `hlo == 0, hhi > 0` → **AMBIGUOUS.** Some candidate scored, but the winner may still be a
  cost-cutter with `hf = 0` — which is exactly what the prediction above expects.

Historically the unambiguous-engagement cases (`1-1`, `1-2`, `2-2`) number 39, against 54 ambiguous
(`0-1`, `0-2`). So roughly 40% of engaged generations will read cleanly and 60% will not.

### At `hf = 0` the treatment is inert for selection — but it is NOT a pure rescale

Measured on seed 1, gens 1-2, treatment vs control:

    gen 1 MAIN   control rates 0.999-0.998708x    treatment rates 0.999-0.998706x
    gen 2 MAIN   control rates 0.497-0.496943x    treatment rates 0.497-0.496913x

Within a lineage the relative ranking is preserved to ~5 significant figures, which is what the
arithmetic predicts: with `hf = 0` the treatment rate is `23e6/(cost + hard_cost)` and the control is
`23e6/cost`, so if `hard_cost` tracked `cost` the two would be monotone transforms of each other and
select identically.

It does not track exactly. The incumbent-surrogate ratio differs BY LINEAGE:

    MAIN  0.002084 / 0.002490 = 0.8369
    MCTS  0.000950 / 0.001406 = 0.6757

MCTS pays proportionally more for the hard set. That particular difference is harmless — the two
lineages hold separate champions and populations and never compete — but it proves `hard_cost` is not
a fixed multiple of `cost`, so a small per-candidate residual remains inside each lineage too (the
~3e-5 above).

**CONFOUND, recorded before it can be misread.** Neutral twins scoring exactly `1.000x` are the
common case, so a 3e-5 perturbation is enough to break a tie the other way. **A divergence between
treatment and control that appears BEFORE `hard > 0` is tie-breaking noise, not the treatment
effect.** The `hard {hlo}-{hhi}` field on the gate line is what separates the two: until `hhi > 0`,
any difference is bookkeeping.

**Selection caveat, in the conservative direction.** This distribution is built from lines that PRINT
the `hard` field, and before today's instrumentation only NON-GATED generations did. Gated
generations — those where a candidate actually won — are therefore missing from the counts above.
Those are precisely the generations where a candidate scored highly, which under `HARD_FITNESS` is
correlated with `hf > 0`. So these figures **understate** the engagement rate in the generations that
matter. The new `hard {hlo}-{hhi}` on the gate line closes that hole going forward.

**LIMIT OF THIS RUN, stated before the result arrives.** It can answer the OUTCOME and not the
MECHANISM. A gated generation prints only the VERIFY and gate lines; the `hard h-h` field is printed
only on non-gated `..none` lines. So for exactly the candidates that reach a gate, nothing records
whether their surrogate rose via the hard set or merely via cost -- and it cannot be recovered
arithmetically, because the guard pins `f` at 23 and `rate = (23+hf)/(cost+hard_cost)` leaves two
unknowns in one equation.

**RESOLVED 2026-09-09 19:35 — and my pricing of it was wrong.** I recorded this as "structural, not
a missing `println!`": `hf` is built at `evolve.rs:1656` and dropped at the population boundary,
since `popn: Vec<(Program, u32, f64)>` is a three-tuple, so carrying it to the gate supposedly needed
that tuple widened and every destructuring site updated.

Checking the SCOPE rather than inferring it from the tuple types shows otherwise. `let (hlo, hhi)`
at `evolve.rs:1674` is a plain binding in the generation-loop body, and the `};` at 1700 closes the
inner `let hist = {...}` block, not the enclosing scope — so `hlo`/`hhi` were in scope at the gate
`println!` the whole time. **One line, not a refactor.** The winner's own `hf` genuinely is dropped,
but the RANGE over guard-passing candidates is what the reading needs: `hhi == 0` means no candidate
scored, so the winner had `hf = 0` too, which identifies the uninterpretable case outright.

Deployed: the gate line now carries `hard {hlo}-{hhi}`. Cost of acting was near zero because both
treatment arms had produced ZERO gated generations and gens 1-2 cost ~3.5 min each. The controls on
cores 13/15 were NOT restarted and still run `xt_sprt2`. The change is print-only — no RNG consumed,
no state touched — and that was **verified rather than assumed**: the restarted seed-1 arm reproduces
its pre-instrumentation log byte-for-byte (kept as `gate_hardfit_s1.log.preinstr`).

**If the treatment arm reads clean, the honest claim is "the fix worked", not "saturation was why".**
The distinction is recorded here so it cannot be quietly dropped later.

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

## CONFIRMED FROM EXISTING RECORDS — cost outbids the numerator even where the numerator is FREE

The under-power prediction above was arithmetic. `STATE.md:2668` already contains the measured case,
and it settles the MCTS ambiguity I added `mates {f}` to the gate line to resolve:

```
veto:  gen 4 MCTS  ACCEPT  15 mates  0.001848 (133 nodes, was 0.001406)  gate 0.500
```

**`15 mates` — unchanged from the seed's 15/23.** So that +31.4% surrogate gain was a **pure cost
cut**. Its VERIFY reads **0.490 +/-0.018**: below parity, not resolved.

This is the sharpest version of the diagnosis available, because **MCTS is NOT saturated**. Its
numerator had room — 15 of 23 — so a mate gain was genuinely available and the search did not take
it:

    a mate gain to f=16 would pay  +6.7%
    the cost cut actually paid    +31.4%     -> cost outbid the numerator 4.7x

So the problem is not only that MAIN's numerator is *stuck* at 23/23. It is that **the cost term pays
several times better than the numerator term wherever both are available**, which is exactly the
imbalance `EXISTENCE_HARD_WEIGHT` exists to correct. The weight-1 arm offers at most +8.7%, against a
cost lever that has been observed paying +31.4% in this very lineage. Weight 4 offers +34.8% at
`hf=2`, which is the first setting that can compete with what the search demonstrably prefers.

**And it strengthens rather than weakens the account of what the veto rule buys.** STATE.md is careful
that its 8 of 17 promotions "are all ties" and that "promoting ties is not the same as improving".
This adds the mechanism: the ties are ties *because* they are cost cuts that preserve mates and
change nothing about play except how much search paid for it — and VERIFY at 0.490 says the change is
mildly negative rather than neutral.

**MASTER_PLAN's own kill criterion prescribes this repair.** `MASTER_PLAN.md:277-282` sets P2's kill
as *"no program improves on the seed by eval ~1800 -> grammar or fitness is wrong; fix those"*, and
`STATE.md:2690` records that the condition has FIRED — 0 promotions in 17 game-gate calls. Rebalancing
the fitness is the prescribed response, not an improvisation.

## IF BOTH WEIGHTS FAIL — the next lever, named now with its evidence

Recorded before the A/B reports, so the follow-up is not invented to fit whatever number arrives.

The weight knob attacks the imbalance from one side: it raises what the NUMERATOR pays. There is a
second, already-implemented lever that attacks it from the other side — **stop letting the surrogate
RANK at all.**

`EXISTENCE_SPEC_FILTER` (`evolve.rs:1801`) picks the first candidate with
`r >= 0.9 * best_rate` rather than the single highest-rate one. That converts the surrogate from a
ranking function into a **floor**: anything not much worse than the champion is eligible, and the
GAME GATE decides. Which is the correct division of labour, because the surrogate is a SPEED
measure — `mates/Mcost` rewards searching less without bound — while strength usually requires
searching more. A metric that pays for cheapness cannot rank for strength; it can only screen out
the broken.

**The evidence that it changes behaviour is already on disk.** PATH 1 — the same-play speedup path —
has fired **8 times in this project's history, all 8 under SPEC_FILTER** (4 in `spec_filter_arm.log`,
4 in `gate_spec_veto_arm.log`) and **zero times in any standard arm**. Under the strict rule the
champion has never moved at all.

**Why it is NOT being run now, stated so it does not look like an oversight.** It interacts with the
two things already changed today. The tolerance pick is exactly the case the struct comment at
`evolve.rs:1399` warns about — under a 0.9x filter a bar-raise ratchets the reference DOWNWARD — and
that is why `gated` exists. Both branches now use `gated`, so that hazard is handled, but adding a
third simultaneous change to a running A/B would confound it. **Order matters more than speed here:
finish the weight question, then take this one.**

**The pre-registered reading, so it is honest either way.** If both weights are inert AND
SPEC_FILTER moves the champion, the conclusion is not "saturation was wrong" — it is that the
surrogate cannot be repaired as a ranking function and should only ever screen. That is a larger
claim than the one this document currently makes, and it needs its own arm rather than being read
off the side of this one.

## VERIFIED: VERIFY really is an independent instrument

This document leans on "the gate and VERIFY agree" and on VERIFY being the falsifier's instrument,
so the independence is worth checking rather than assuming. Traced through `evolve.rs`:

    GATE    seed 0xC0FFEE   ^ g ^ (li<<8)      openings walked from that same seed (evolve.rs:1934-44)
    VERIFY  seed 0x5EEDBEEF ^ g ^ (li<<8)      openings walked internally, 4 plies (evolve.rs:1997-2001)

Different base constants, so different game streams AND different opening sets — the two matches do
not replay the same positions. That is what makes "gate REJECT llr -3.18 and VERIFY 0.422 +/-0.027
both say worse" two readings rather than one restated.

**The honest limit.** They share `g` and `li`, so within a generation the two seeds are
deterministically related by a fixed XOR. For this PRNG that yields uncorrelated streams, but it is
not independence in a cryptographic sense, and a systematic defect in `Rng` would hit both. What
protects the falsifier is more basic: VERIFY is a FIXED 96-pair match whose pair count does not
depend on the gate's bounds, its stopping rule, or its cap. So the §7.2 bounds question — live and
unresolved — cannot move the falsifier's reading.
