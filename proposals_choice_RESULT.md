# "94% of generations offer no choice" is BINOMIAL ARITHMETIC, not a pathology — and 32 proposals fixes it

**2026-09-11 10:1x.** `search_has_no_choice_RESULT.md` established the binding constraint on the
search track: across 79 generations, only **5 (6.3%)** ever handed selection more than one distinct
fitness, and selection cannot select from a set of size <= 1. That is why P2 has 1,062 proposals and
0 accepts. It named the lever — "raise the number of guard-passing, distinctly-scoring candidates
per generation" — and noted that none of the four running arms varied it.

## The mechanism is one line of code and one binomial

Candidates were proposed as `(0..pop)`, so the PROPOSAL COUNT WAS THE POPULATION SIZE, and pop
collapses to 2-4. A "choice" requires **two survivors in the SAME generation**. At the measured mate
guard survival rate of **10.5%**:

    proposals   P(0 survive)   P(exactly 1)   P(>=2 = A CHOICE)
        4          0.642          0.301             0.057
       32          0.029          0.108             0.863

**Predicted 5.7% against a measured 6.3%.** The "94% no choice" figure is not a pathology to be
diagnosed — it is what four proposals at a 10.5% guard MUST produce. Every knob previously tuned
(gate bounds, EPS, guard tolerance, surrogate role, population size, lambda) was operating on a
choice that arithmetic had already excluded.

## The paired measurement

`EXISTENCE_PROPOSALS` separates proposal count from population size (default `pop`, so unset is
byte-identical to every prior measurement). Same seed, same pop, same sets, MAIN lineage:

    cell      gens  proposed  mate-ok   >1 distinct   final pop
    control     6      24        3         0/6            4
    prop32      1      32        2         1/1            3

Truncated to the common range — the paired comparison unequal arms still support:

    control   gen 1:  4 cand, mate-ok 0, distinct 0, pop 1
    prop32    gen 1: 32 cand, mate-ok 2, distinct 2, pop 3

The control did not produce a single generation with a choice in six attempts. `prop32` produced one
on its **first**, with exactly the shape the arithmetic predicts.

## What is and is NOT claimed

* **Claimed:** the no-choice constraint is caused by proposal count, and raising it removes the
  constraint. The arithmetic predicts the historical rate to within 0.6 points, and the first
  generation of the treatment arm behaves as predicted.
* **Claimed:** the population collapse reverses. Control ends at pop 1 in gen 1; prop32 ends at pop
  3. Retention keeps distinct survivors, and there were none to keep before.
* **NOT claimed — that this makes the search track stronger.** An ACCEPT is several stages
  downstream of a choice, and this project has been misled by proxies four times (ICC, SEE
  classification accuracy, threat feature quality, time-to-depth). A funnel that offers a choice is
  a precondition for progress, not progress.
* **NOT claimed at full strength — prop32 has ONE generation.** It proposes 8x the candidates and
  hit its timeout early, so the arms are unequal and the reporter flags the pooled comparison as
  confounded. The common-range row is the honest read; the arithmetic is what carries the weight.

## Cost, stated because it is the obvious objection

32 proposals costs ~8x the candidate evaluations per generation. That is the trade: generations
become slower, but a generation that offers no choice is worth nothing regardless of how fast it
completes. The right setting is the one that makes P(>=2 survivors) high without overshooting --
around 20-32 at a 10.5% guard, and it should be re-derived if the guard rate changes.

## Next

* A longer-timeout re-run so prop32 reaches the control's generation count.
* `choice_2x2.sh` tests the other candidate lever, `EXISTENCE_HARD_FITNESS` (the HARD set is 8
  positions the seed fails BY CONSTRUCTION, scoring 0/8, so `f` itself can vary where the shipped
  guard set is saturated at 25/25). The two are not the same fix: more candidates landing on one
  identical rate would still be no choice.

---

## ADDENDUM 10:2x — the model validates on its own input, and the population inverts

Two more prop32 generations landed. The arm now independently reproduces the parameter the
binomial assumed, which is the strongest form this argument can take: the prediction and the thing
predicted were measured on different data.

    prop32 guard survival        7 / 64 proposed  =  10.9%
    documented rate (79 gens)   61 / 580          =  10.5%

Re-running the arithmetic at the arm's OWN observed rate rather than the historical one:

    4 proposals  ->  P(a choice) = 0.062
    32 proposals ->  P(a choice) = 0.879

**The historical rate was 5/79 = 6.3%. The model predicts 6.2%.** A tenth of a point, from a
parameter measured in a different arm on different generations.

### The generation lines, which show the spread arriving

    gen 1 MAIN  32 cand, mate-ok 2, rates 0.994-0.998707x  distinct:2   pop 3
    gen 2 MAIN  32 cand, mate-ok 5, rates 0.497-0.998707x  distinct:5   pop 7

Generation 2 produced **five survivors with five distinct rates** spanning 0.497 to 0.999. That is
not a marginal improvement on "one value" -- it is a population with real structure for selection
to act on, in a track that had produced 1,062 proposals and 0 accepts.

### The population collapse reverses

    control  pop by generation:  1  2  2  2  3  4
    prop32   pop by generation:  3  7

Retention keeps DISTINCT survivors. The control starts at 1 because generation 1 gave it nothing to
keep; prop32 starts at 3 and doubles. This is the death spiral running backwards, and it follows
mechanically -- nothing about retention changed.

### What is still NOT claimed

**No accept.** Both arms are still short (the reporter flags them as unequal; prop32 costs ~8x per
generation and hit its timeout). A choice is a precondition for progress, and every stage after it
-- EPS retention, the acceptance floor, the game gate -- is untested under a working funnel.
`search_long_run.sh` runs 40 generations on a FRESH seed to answer exactly that, with the accept
count as the primary metric and both informative nulls written in advance.

---

## ADDENDUM 10:3x — the guard rate COMPOUNDS, which corrects my own model

The binomial above treated the mate-guard survival rate as a CONSTANT of the problem (10.5%,
measured over 79 historical generations). Three generations of prop32 say it is not a constant —
it is a property of the POPULATION, and it rises as the population fills:

    gen   cand   guard-passers   rate     distinct   pop
      1     32         2         6.2%        2        3
      2     32         5        15.6%        5        7
      3     32        10        31.2%       10        8

**Five-fold in three generations.** A mutation of a guard-passing parent is far likelier to pass
than a mutation of a broken one, so once survivors exist the rate climbs on its own.

### Why the control cannot show this, and why that matters

    control gen-by-gen rate:  0%  25%  0%  0%  25%  25%

At 4 candidates the rate is QUANTISED to 0/4 or 1/4 — it can only read 0%, 25%, 50%... A trend of
the size prop32 shows is unresolvable in the control **by construction**, not by absence. That is
the same class of error as the rest of this file: the historical arms were not measuring a flat
rate, they were measuring a rate they could not see move.

### The death spiral had TWO reinforcing arms, and breaking one breaks both

    STRUCTURAL   few proposals -> few survivors -> pop collapses -> few proposals
    QUALITATIVE  bad population -> low guard rate -> few survivors -> population stays bad

`EXISTENCE_PROPOSALS` attacks only the first. The measurement above says the second unwinds as a
consequence: pop 1 -> 3 -> 7 -> 8 while the guard rate goes 6% -> 16% -> 31%. This is why the fix
is not merely additive.

### Honest limits

* **n = 3 generations, one seed.** A five-fold rise over three points is suggestive, not
  established.
* **An alternative reading that must be excluded:** the rate could rise because the population is
  CONVERGING on one family of near-identical programs, which would raise pass rates while
  destroying diversity and then stall. The `distinct` column argues against it — 10 survivors
  produced 10 distinct rates in generation 3, so they are not clones — but a longer run is what
  settles it.
* `search_long_run.sh` (40 generations, fresh seed) measures exactly this: whether the rate keeps
  climbing, plateaus, or collapses back, and whether any of it reaches an ACCEPT.

---

## CORRECTION 10:2x — "the guard rate COMPOUNDS" was over-stated. It STEPS, it does not compound.

The addendum above read `6.2% -> 15.6% -> 31.2%` as five-fold compounding over three generations.
Generation 4 landed at **15.6%** and refutes that reading. The full sequence, with the binomial
noise each point actually carries on 32 trials:

    gen  cand  ok   rate    +/- (binomial SE)
      1    32    2   6.2%     +/-4.3
      2    32    5  15.6%     +/-6.4
      3    32   10  31.2%     +/-8.2
      4    32    5  15.6%     +/-6.4

**Not monotonic.** 31.2% against 15.6% is ~1.5 sigma — well inside noise. I was reading a sequence
of four noisy points as a trend because the first three happened to ascend.

### What survives the correction

    gen 1        6.2%
    gens 2-4    20.8%
    difference +14.6 +/- 6.0  =  2.4 sigma

**A real STEP, not a compounding curve.** The population goes from nothing-to-mutate-from to a
working population within one generation, and the rate then sits at ~20% with generation-scale
noise. Pooled across all four: 22/128 = **17.2%**, against the historical baseline of 10.5%.

### What this does and does not change

* **Unchanged:** the binomial explanation of the no-choice constraint. That used the HISTORICAL
  10.5% and predicted the historical 6.3% rate to within 0.1 points. Nothing here touches it.
* **Unchanged:** every generation of prop32 offered selection a choice (4/4), against 0/6 for the
  control.
* **Changed:** the claim that the fix is self-amplifying. A step is not a spiral. The mechanism I
  proposed — mutations of guard-passing parents pass more often — predicts a step just as well as
  it predicts compounding, and four points cannot separate them.
* **Still open, and now the 40-generation run is the only thing that answers it:** whether the rate
  keeps rising, plateaus near 20%, or decays as the population converges.

### The lesson, since this is the second time today

An ascending run of three noisy points is not a trend, and I published it as one within minutes of
seeing it. The guard against this is the one already applied to the ruler: **quote the noise with
the number**. Had the first addendum carried the +/-4.3 / +/-6.4 / +/-8.2 columns, "five-fold"
would not have survived writing it down.

---

## FINAL — the matched paired comparison, both arms complete

Both arms finished. prop32 costs ~8x per generation and stopped at 4 generations against the
control's 6, so the comparison is taken on the COMMON RANGE — the first 4 generations of each,
which is a like-for-like paired read (same seed, same pop, same sets, same binary).

    cell      gens  proposed  guard survivors  generations with a CHOICE
    control      4        16                1                    0/4
    prop32       4       128               22                    4/4

    choices:  Fisher exact two-sided  p = 0.029
    guard:    6.2% (1/16) vs 17.2% (22/128) = +10.9 +/- 6.9  (1.6 sigma) -- NOT separable

**The choice column is the primary and it is decisive.** The guard-rate difference is not, and the
control's 1/16 is consistent with both the historical 10.5% and prop32's 17.2% — so nothing here
claims the fix improves candidate QUALITY, only that it stops starving selection.

### Why the arms being unequal did not cost the result

The reporter flagged `{control: 6, prop32: 4}` as a mild imbalance and compared on the common range
rather than discarding the data or dividing by different denominators. Comparing gens 1-4 against
gens 1-6 would have confounded the lever with training amount — the exact failure that invalidated
the first low-lr sweep, whose arms were unmatched at ~650 generations.

### The claim, stated at the size the evidence supports

* **Established:** 32 proposals per generation gives selection a choice in every generation
  measured, where 4 proposals gave none in six. p = 0.029, and the binomial predicts it
  (`P(>=2 survivors)` = 0.062 at 4 proposals against 0.879 at 32, using the arm's own measured
  guard rate).
* **Not established:** that the guard rate improves, that the effect compounds (refuted above —
  it STEPS), or that any of it produces an ACCEPT.
* **The open question is unchanged and is the one that matters:** `search_long_run.sh`, 40
  generations on a fresh seed, with the accept count as the primary metric.
