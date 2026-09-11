# The 6-pair game gate cannot accept a candidate that draws, and 85.3% of its games are draws — this is arithmetic about the 6-pair RULE and is NOT an argument for raising `gate_pairs`, which `gate_power_RESULT.md` measured as buying more draws

2026-09-11. Exact enumeration plus 29 completed matches. No running experiment was read for this.
See the provenance note at the end: the sample grew from 21 to 29 while this was written, and the
counts below are recomputed on all 29.

## The question

`search_track_WHY_NOTHING.md` says the MAIN lineage "structurally cannot" improve, on the grounds
that **cost never improves**: "93 generations with a surviving candidate, 0 with a survivor cheaper
than the champion." The funnel fix (`EXISTENCE_PROPOSALS`) has since produced candidates that DO beat
the champion's rate — the logs print `ABOVE:1` through `ABOVE:13` — and they reach the game gate.
So the old account needed re-checking against what the gate actually does with them.

## What the gate can accept, enumerated exactly

Acceptance is `pent_rate - ci95 > 0.5` over 6 pairs. Both quantities come from `gate.rs`: the
pentanomial mean halved, sample variance over `n-1`, `ci95 = 1.96*sqrt(var/n)/2`, and the
rule-of-three `1.5/n` when the observed variance is zero.

**The formula was validated against real output before being used to conclude anything.** Pentanomial
`[0,1,5,0,0]` — five drawn pairs and one half-lost — computes to `0.4583 +/- 0.0817`, and the logs
print `0.458+/-0.082`. Enumerating all 210 possible 6-pair outcomes:

```
outcomes that can ever ACCEPT                        29 of 210   (14%)
ceiling for a candidate drawing >=4 of 6 pairs       0.4800      CANNOT PASS
ceiling for a candidate drawing >=5 of 6 pairs       0.4600      CANNOT PASS
```

A candidate that draws four or more of six pairs **cannot pass at any decisive result whatsoever**.
Not unlikely — arithmetically impossible.

## What the candidates actually do

29 completed gate matches on disk, from the funnel-fixed era:

```
pooled       W=4  D=297  L=47   of 348 games      DRAW RATE 85.3%
modal match  0 wins, 11 draws, 1 loss  ->  0.458 +/- 0.082,  needed > 0.582
```

Every one of the 29 sits inside the unpassable region. The gate has rejected 29 of 29, and for the
observed class of candidate it could not have done anything else.

## The rule punishes the candidate that earns it

Because ci95 is computed from the match's own variance, winning a pair OUTRIGHT inflates the
variance, which widens the interval, which raises the bar:

```
5 drawn + 1 pair WON        rate 0.5833   ci 0.1633   ->  0.4200   REJECT
3 half-wins + 3 draws       rate 0.6250   ci 0.1096   ->  0.5154   ACCEPT
```

The lower-scoring candidate passes. The rule does not reward strength, it rewards CONSISTENCY, and
six pairs is too few for consistency to show.

## The alternative rule is not the fix either

`EXISTENCE_GATE_VETO=1` already exists and tests non-inferiority (`rate + ci95 >= 0.5`). Re-scoring
all 29 completed matches under both rules — free, since both quantities are printed:

```
shipped  (rate - ci95 >  0.5)      0/29 promote
veto     (rate + ci95 >= 0.5)     28/29 promote     including ten at 0.417 and one at 0.375
```

A rule that promotes a candidate which lost 10 of 12 games -- and, at 29 matches, one that lost 9
of 12 at a rate of 0.375 -- is not a veto, it is a rubber stamp. This
also **corrects the claim in `evolve.rs`'s own comment** that the veto "DISCRIMINATES rather than
waving everything through" — that was measured on the mcts_ab arms, where MAIN calls sat at
0.292-0.375. On funnel-era data the MAIN calls sit at 0.417-0.458 with tighter intervals and flip to
PROMOTE.

**So the binding constraint is the SAMPLE SIZE, not the rule.** At 6 pairs ci95 is 0.08-0.25; one
rule can resolve nothing upward and the other nothing downward, because both are thresholding noise.

## What this does and does not overturn

**Overturned:** "cost never improves" as the account of why nothing is promoted. Candidates above the
champion's rate are being produced and are reaching the gate.

**Not overturned:** the conclusion that the track has promoted nothing. That stands, with a different
mechanism.

**Not claimed:** that the candidates are good. Their true strength is unmeasured — the 6-pair gate
cannot measure it, which is the entire point. A candidate could be a genuine improvement or worthless
and produce the same `0.458 +/- 0.082`.

## What was done

`gate_pairs` is argument 7 of the binary, so 6 pairs is a CONFIGURATION, not a law. But raising it
costs time on every candidate, so the honest order is measure-then-spend.

`EXISTENCE_GATE_VERIFY=96` re-matches the same candidate against the same champion at 96 pairs
(ci95 ~0.047) with an independent seed, and is explicitly an observer — it decides nothing, so the
trajectory stays byte-identical to the shipped rule's. The 40-generation "does a choice become an
accept" arm was stopped at generation 1 and replaced with a 15-generation arm carrying this observer
(`search-verify96.service`). Its headline was already determined by the arithmetic above; this one
can distinguish "the gate is discarding winners" from "the candidates are genuinely not better",
which is the question the search track has actually been stuck on.

Readings are pre-registered in the script header.

## What raising `gate_pairs` would actually buy

The enumeration above says acceptance is impossible *conditional on* drawing >=4 of 6 pairs. The
matching question is how often a genuinely better candidate escapes that condition — i.e. the gate's
POWER. Simulated at the observed 85.7% draw rate, using the same `gate.rs` formula, 6000 trials per
cell:

```
pairs |  true 0.500   true 0.530   true 0.550
    6 |      2.1%         6.4%        10.6%
   12 |      2.1%         8.8%        19.3%
   24 |      3.3%        22.8%        53.0%
   48 |      2.9%        35.8%        79.2%
   96 |      1.9%        61.1%        98.1%
  192 |      2.6%        89.2%       100.0%
```

The first column is the FALSE-ACCEPT rate and stays near 2-3% at every size: the rule is properly
conservative and is not what needs fixing. The others are power. **At the shipped 6 pairs, a
candidate that is truly 0.550 — roughly +35 Elo — is accepted 10.6% of the time.** Nine out of ten
real improvements would be thrown away, which is fully consistent with 0 promotions in 21 calls.

A ceiling worth stating separately: at an 85.7% draw rate a candidate winning EVERY decisive game
still only scores `0.5*0.857 + 0.143 = 0.5714`. So ci95 must fall below 0.0714 before ANY candidate
can pass, and at 6 pairs ci95 runs 0.08-0.25.

**If the verify96 arm shows the gate discarding winners, 24 pairs is the recommended setting** — 53%
power at 4x the cost, against 98% at 16x. It is `gate_pairs`, argument 7 of the binary.

CAVEAT, because this is a model and not a measurement: it assumes the draw rate stays at 0.857 and
that a candidate has one fixed true rate. Both are calibrated to the 21 matches on disk and neither
is guaranteed for candidates the search has not produced yet. The power column is a design aid for
choosing a pair count, not a result about any specific candidate.

## CORRECTION — "raise gate_pairs" is not the fix. The spec already specifies the fix, and it is built.

The recommendation above (gate_pairs=24) is superseded. `docs/FITNESS.md` §7.2 states:

> **How much evidence to gather — ENGINE-DECIDED, already.** SPRT is exactly that decision: a
> candidate near a bound gets thousands of pairs, an obvious dud a few hundred; **nobody picks the
> count, the evidence does.**

The loop picked 6. Raising it to 24 would pick a different fixed number, which is the same mistake
with a better constant — and it cannot spend more games on precisely the close cases that need them,
which is the whole point of a sequential test.

**The sequential gate is already implemented** and selected by `EXISTENCE_GATE_SPRT`; unset is
byte-identical to the fixed-pair gate every prior measurement used. Its bounds come from the spec:
alpha = beta = 0.05 (LLR bound 2.944), `EXISTENCE_GATE_MAXPAIRS` default 400.

`evolve.rs`'s own comment at that site reports the same phenomenon this file measured, at far larger
n than the 21 matches used here:

```
evolve.rs comment claims:   0 accepts in 203 decisions,  46.8% zero observed variance
RE-MEASURED 2026-09-11:     0 accepts in 489 decisions,  28.6% zero observed variance (140/489)
```

The comment is a HYPOTHESIS until re-measured, so it was. **The zero-accept finding is confirmed and
STRONGER** — 0 in 489 gate decisions across every evolve log on disk, against the comment's 203 and
this file's 21. The variance share is **stale**: 28.6% now, not 46.8%, because more runs have
accumulated since the comment was written and the mix changed. Use 489/0 and 28.6%.

### The ledger looked like it contradicted this. It does not — and it supplies the control.

`ledger.jsonl` parses to 41 entries with **24 "accepts"**. Checked one by one rather than assumed:

```
22 of 24   pent=[0,0,0,0,0]  rate=0.0  ci95=0.0  resolved=True   <- ZERO pairs played
 2 of 24   real 224-pair matches                                  <- genuine
```

The 22 are exactly the artifact `gate.rs:72-83` documents: a match that played no games had
`rate() = 0/1` and a binomial `ci95 = 0`, so `resolved` was written true for a measurement that never
happened. That comment records the count as 22, and 22 is what is there.

**But the ledger holds only NET (40) and ARCH (1) entries — no MAIN/MCTS rows at all.** It is a
NET-track record, so it never contradicted the search-track count; that was two different populations
being compared. Corrected.

**The two real accepts are the control this whole file needed**, because they ran the SAME
acceptance rule at a different pair count:

```
NET gen 1   pent=[0,27,134,58,5]  224 pairs   rate 0.5458 +/- 0.0216   rate-ci95 = 0.5241  ACCEPT
NET gen 2   pent=[2,22,150,45,5]  224 pairs   rate 0.5324 +/- 0.0209   rate-ci95 = 0.5114  ACCEPT
```

Same rule `rate - ci95 > 0.5`. At **224 pairs it accepts** candidates whose true edge is ~0.53-0.55.
At **6 pairs it has never accepted, in 489 decisions.** That is a measured natural experiment sitting
in the project's own records, and it is stronger evidence than the simulated power curve above: the
rule is not broken, it is starved. Note also that both accepted pents are spread across four buckets
— real variance, not the degenerate all-drawn shape the 6-pair search gate produces 28.6% of the time.

Zero observed variance means every pair landed in one bucket, so `ci95 = 1.5/n = 0.25`, and
acceptance would need `rate > 0.75` from a match whose rate is 0.5 by construction. Those 46.8% were
unacceptable before a single game was played. That is an independent corroboration of the enumeration above at 23x the sample, and it
supersedes the 21-match count as the headline number.

### The bound to use is NOT the default, and the spec says why

`EXISTENCE_GATE_ELO1` defaults to 5 with `elo0 = elo1 − 2`, so the default SPRT tests H0 elo ≤ 3
against H1 elo ≥ 5. §7.2 is explicit that this is a SUPERIORITY test:

> PASSING means H1 accepted, i.e. the candidate is shown to gain at least ~e1 (not merely "not
> negative"). A true NON-REGRESSION test therefore has e1 = 0 and e0 < 0, e.g. [-5, 0].

A cost-reducing mutation at fixed depth returns the SAME move more cheaply. Its true Elo is ~0 by
construction, so it fails a superiority test at ANY pair count — 400 pairs of a true 0.500 resolves
to "not ≥5 Elo", correctly. **The pair count was never the binding constraint for this class of
candidate; the HYPOTHESIS was.**

This is also why PATH 1 exists (identical play on 33 positions → accept with no game). PATH 1 is a
non-regression test done by proof instead of by sampling. Candidates that are NEARLY identical —
differing on one of 33 — fall through to a gate that demands a rout. The gap between the two paths is
where this track's candidates live.

**So the experiment to run is `EXISTENCE_GATE_SPRT=1` with `ELO1=0 ELO0=-5`** — the spec's own
non-regression bounds — not a larger fixed pair count. Not launched yet: the box is running the
verify96 observer and the 4PC confirm gate, and verify96 answers the prior question (are the
rejected candidates actually neutral?) which determines whether a non-regression gate would admit
anything worth having.

CAVEAT, and it is the spec's: §7.2 fixes the acceptance criterion as HUMAN, not engine-chosen —
"an instrument calibrated by its subject measures nothing". Changing e1 from 5 to 0 changes what
counts as enough, so this is to be RUN AS AN EXPERIMENT against the existing rule, never silently
shipped as a default.

## What budget the gate games are actually played at — and why that is NOT the whole explanation

Verified from `evolve.rs:608,669,776`:

```rust
let bud = if name.contains("MCTS") { 256 } else { 16 };   // 512 / 1024 at other sites
```

**MAIN plays its gate games at 16 nodes per move.** That is the same budget the fitness optimises
against — `mates per cost` at budget 16 — so testing there is principled rather than an oversight:
the program is judged at the budget it was selected for. But it does mean the games are played by two
very weak searchers, and a 16-node search is barely a lookahead.

That is an appealing explanation for the 85.7% draw rate, and **it does not survive the data.** The
instrumented `gen 1 MCTS` gate ran at **budget 1024** — sixty-four times MAIN's — and still returned
`W-D-L 0-11-1` with `identity:12/27`. Two programs differing on 15 of 27 positions, searching 1024
nodes each, drew eleven of twelve games.

So the draw rate is NOT explained by the budget alone. Recording the budget as verified context
rather than as a cause, because the obvious causal story is refuted by the one instrumented reading
available. Candidates left open by that reading, none of them yet tested:

* the random-ply openings are balanced, and weak programs from balanced starts draw regardless;
* the guard and hard sets are mate-heavy and tactical, so positional differences they detect need not
  appear in game outcomes;
* twelve games is simply too few to see the decisive fraction.

The `identity:N/M` field now on every gate line is what separates these. It did not exist this
morning, and one reading has already killed the first hypothesis I formed from it.

## The sibling project already does this correctly, and it is a usable precedent

`~/maswabe_corpus/gate_dgnodes.sh` faces the identical problem — a change that buys CHEAPNESS and is
not expected to buy strength — and its stage-2 decision rule is written as:

> the CHEAPER setting ships only if it does not LOSE — **a tie is a win for it, because it buys
> corpus rate at no measured cost.**

That is a NON-INFERIORITY test, arrived at independently in the 4PC tree for a cost-reducing change.
It is the same shape `docs/FITNESS.md` §7.2 names (`e1 = 0, e0 < 0`) and the same shape this file
argues Existence's search gate needs.

**Existence's search gate does the opposite for the same class of change.** It demands
`rate - ci95 > 0.5` — superiority — from cost-reducing mutations whose true Elo is ~0 by
construction. So the two projects, sharing an author and a methodology, have arrived at opposite
rules for the same situation, and the 4PC one is right.

Worth stating plainly because it changes the character of the proposed change: switching Existence's
cost-change acceptance to non-inferiority is not a novel idea to be justified from scratch. It is
applying a rule this tree already uses, where FITNESS already specifies it, to the one track that
does not.

It does NOT license flipping the default silently — §7.2 fixes the acceptance criterion as HUMAN,
and the comparison must still be run as an experiment. But the "is this even reasonable" question is
answered by an existing, working instance.


## Provenance: which numbers were recomputed when the sample grew, and which were not

This file was written on 21 completed gate matches. The arms produced 29 while it was being written,
so the COUNTS above are recomputed on all 29 and the file carries one vintage of each.

RECOMPUTED: the pooled W-D-L and draw rate (0.857 -> 0.853), the rejection count (21/21 -> 29/29),
and the veto re-scoring (0/21 and 20/21 -> 0/29 and 28/29). All are direct counts from printed
values, so redoing them costs nothing and leaving two vintages in one document costs a reader.

NOT RECOMPUTED, deliberately:
* **The 210-outcome enumeration and the 0.48 ceiling.** Pure arithmetic on `gate.rs`'s formula. It
  does not depend on the sample at all.
* **The power simulation.** It was run at the then-measured 0.857 draw rate; the rate is now 0.853.
  Re-running it would move the power figures by far less than the 6000-trial Monte Carlo noise
  already in them, and re-running an analysis to change a number in its fourth significant figure
  invites reading precision that is not there.

The distinction matters more than the numbers: a COUNT should track the data, an ANALYSIS should be
reported at the sample it was performed on and re-run only if the answer could change.
