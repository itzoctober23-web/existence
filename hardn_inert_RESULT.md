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
