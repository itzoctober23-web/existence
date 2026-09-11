# EXISTENCE_HARD_N was inert on the code path that runs, and an experiment concluded from it

2026-09-11. A void experiment, its cause, and two compounding faults. Nothing here needed new compute.

## What was claimed

`hardn_probe.sh` ran with `EXISTENCE_HARD_N=40` to ask whether the HARD set is a real gradient at
five times the default size, and reported:

> MAIN still scores ZERO at n=40, five times the positions. The set is not a gradient at any size
> reachable this way: the DIFFICULTY is wrong, not the sample. It must be rebuilt around positions
> the population can PARTIALLY solve.

The recommended action is rebuilding the HARD set, which is expensive.

## Fault 1 — the knob does nothing on this path

`evolve.rs` has TWO hard-set construction sites:

```
line  659   let hard = harder_set(n_hard, depth, &net, 3_000 * (n_hard/8).max(1));   // reads the var
line 2627   let hard = harder_set(8, depth, &net, 3_000);                            // HARDCODED
```

`EXISTENCE_HARD_N` is read at line 657 and feeds line 659 — a different entry point. The evolve loop
runs the **line 2627** path. The variable is therefore compiled into the binary, present in its
strings, documented in the script, and **has no effect on the arm that ran**.

**The tell was in the run's own header and was there to be read:**

```
hardn40 arm   HARD set: 8 positions the seed FAILS by construction; seed scores 0/8
control arm   HARD set: 8 positions the seed FAILS by construction; seed scores 0/8
```

Byte-identical. The arm asked for 40 and reported 8, exactly like the control.

Two hypotheses were tested and rejected before finding it, which is worth recording because the
second was confidently wrong: the snapshot binary was NOT stale (it contains both
`EXISTENCE_HARD_N` and `EXISTENCE_GATE_VERIFY`), and the header text mismatch against line 661 was
not evidence of an old build — the source simply has two different `HARD set` printlns, and the log
matches line 2778.

## Fault 2 — the verdict was drawn from 4 generations

Independent of the inert knob, the conclusion could not have been supported anyway. MAIN scores on
the hard set in **2 of 28** generations across the funnel-era logs — a base rate of 0.071.

```
P(zero in  4 generations | rate 0.071) = 0.74     <- what the probe ran
P(zero in 10 generations)              = 0.48
P(zero in 22 generations)              = 0.20     <- 80% power
```

Zero was the single most likely outcome, and would have occurred just as readily if `n=40` had
worked perfectly and changed nothing. The probe could not distinguish "the set is inert" from "the
set behaves exactly as it did at n=8".

This is the same failure retracted earlier the same day — "0 of 10 MAIN generations scored, so the
HARD set is inert" — where generation 11 scored. **A zero is evidence only when the run was long
enough for a non-zero.**

## What was fixed

* **Line 2627 now reads `EXISTENCE_HARD_N`**, with the search budget scaled as the other call site
  scales it. Type-checked (`cargo check --example evolve`, clean). NOT yet rebuilt or re-run — the
  box is loaded and the running arms use a snapshot copy, so this takes effect on the next build.
* **A shortfall is now printed.** `harder_set` returns what it could FIND, which need not be what
  was asked. A silent shortfall is the same bug in a quieter costume — the header would read
  "8 positions" for a request of 40 and nothing would say why. It now prints requested vs built and
  states that any n-scaling claim is bounded by the number built, not the request.
* **`hardn_probe.sh`'s reporter now refuses an underpowered verdict.** It counts MAIN generations,
  computes `P(zero | base rate)`, and if that exceeds 0.20 it prints the generations needed for 80%
  power and says explicitly: do NOT rebuild the hard set on this reading. Verified by re-running the
  corrected reporter against the existing log — it now returns UNDERPOWERED instead of a verdict.

## Status of the question

**Unanswered.** Whether the HARD set is a gradient at larger n has never been measured. The claim
that it is not is withdrawn. A valid test needs the rebuilt binary and at least 22 generations, and
must check the header's requested-vs-built line before reading anything else.

## The detector, and the two that failed their control tests first

Finding this by hand is not a method, so it was turned into a check. Two designs were tried and
discarded, both caught by running them against a known-bad AND a known-good case:

1. **Static call-graph audit** of all 19 `EXISTENCE_*` variables — mapped each read to its enclosing
   function and asked whether that function is reachable from `main`. Returned **all-clear**,
   including for `HARD_N`. Useless for this bug by construction: the read IS reachable, the value
   simply never reaches the use site, and reachability cannot see a hardcoded literal.
2. **Whole-header comparison** between a treatment arm and its control. **Inverted on both
   controls.** It passed the known-bad pair, because `hardn_probe` also set `HARD_FITNESS` and that
   difference showed up while `HARD set: 8 positions` was identical in both; and it failed the
   known-good pair (control vs 32 proposals), because the header never prints the proposal count, so
   two genuinely different arms looked identical. Deleted rather than kept — a tool that fails its
   own control test is a trap.

The shared lesson: **"something differed" is not "the thing under test differed".**

`assert_setting_took.py` maps each variable to the ONE observable it controls and asserts
requested-vs-observed on that observable:

```
prop_hardn40.log   EXISTENCE_HARD_N  requested 40  observed 8   *** DID NOT TAKE   (exit 1)
prop_prop32.log    EXISTENCE_PROPOSALS requested 32 observed 32  OK                (exit 0)
prop_control.log   EXISTENCE_PROPOSALS requested 4  observed 4   OK                (exit 0)
```

It also distinguishes INERT from NOT YET EXERCISED. `EXISTENCE_GATE_VERIFY` prints only inside the
gate block, so an arm with no gate call yet cannot show it; reporting that as a failure is a false
alarm, and a checker that cries wolf is the one ignored the day it is right. Caught on the live
`search-verify96` arm at generation 1 with zero gate calls — it now reports UNDETERMINED.

**Retrospective sweep — 5 of 5 completed arms clean:** `prop_control`, `prop_prop32`, `prop_hard`,
`prop_hardp32`, `prop_long32` all had every requested setting visible in their own output. So the
2x2 and the funnel-fix conclusion stand on arms that genuinely ran what they claimed. `hardn40` was
the only void one.

It is wired as a GATE, not a decoration: `hardn_probe.sh` now runs it BEFORE the reporter and exits
non-zero on mismatch, with the status taken directly rather than through a pipe, so the verdict
cannot be written when the configuration under test was not the one that ran.

## The new instrumentation is validated live, and its first reading already complicates a hypothesis

Built and run 2026-09-11 on the same seed/proposals as the live arm (seed 7, 32 proposals), so the
output is directly comparable to `prop_verify96.log`.

**Non-regression, checked first.** The 6-line header md5 is identical (`8b686983345d`), and the
`gen 1 MAIN` line is BYTE-IDENTICAL to the running snapshot's. On the gate line, everything before
the new fields matches exactly — `0.458+/-0.082`, `W-D-L 0-11-1`, `mates 10`, `surrogate 0.003529`,
`ABOVE:2`, `needed >0.582`, `pop 1 distinct:1`. The changes decide nothing, as claimed.

**The gate line now reads:**

```
gen 1 MCTS gate REJECT 0.458+/-0.082 (12 games W-D-L 0-11-1) ... identity:12/27  rel[>=.98:6 .90-.98:0 .50-.90:2 <.50:1]
```

**`identity:12/27` is not what I expected.** `gate_arithmetic_RESULT.md` reasons that a candidate
identical on 32 of 33 positions plays almost the same games, draws nearly all of them, and so lands
in the region the 6-pair gate cannot accept. That made near-identity the suspected shape of the
rejected candidates. This one agrees on **12 of 27 — 44%** — and is a substantially different
program, **yet it still drew 11 of 12 games.**

So behavioural difference on the position set does NOT translate into game divergence, at least here.
That is a different and more interesting problem than near-identity: the games are not sensitive to
the thing the position set measures.

**One data point, MCTS lineage, and not a finding.** It is recorded because it points somewhere the
existing reasoning did not, and because the field now exists to accumulate more. The MAIN lineage,
which is the one that has never promoted anything, has not produced an instrumented gate line yet.

**Minor correction while here:** the code comment says identity is checked on "33 positions". In this
configuration it is **27** — the guard set is 19 (10+4+5) plus 8 hard. The 33 assumed 25 guard
positions. The count is printed now, so the number in the log is the number that was checked.
