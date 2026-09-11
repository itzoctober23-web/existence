# PRE-REGISTRATION: does a resume damage the champion, and if so WHEN?

**2026-09-10 22:4x, written before any net is measured.**

## The observation that prompted it

`auto_promote` measured the production run against the champion it resumed from:

```text
22:33 prod1.net gen 108: REGRESSION 0.362 +/- 0.031  (interval entirely below 0.5)
```

prod1 started as a byte-identical copy of `p1_champion.net`. 108 generations later it loses to that
champion at [0.331, 0.393] — far outside noise. Yet `d3c`, run the same way, reached **0.557 ±
0.032** after 4,327 generations. So a resumed run appears to get worse before it gets better.

## The candidate mechanism

`main.rs:676` creates the replay pool unconditionally empty:

```rust
let mut replay: Vec<Sample> = Vec::new();
```

`--init` restores the **net** (`main.rs:555`) and suppresses the horizon ramp (`:728`). It does not
restore the **data**. Measured pool sizes over the first six generations of two resumed runs:

| run | gen 1 | 2 | 3 | 4 | 5 | 6 | … steady state |
|---|---|---|---|---|---|---|---|
| prod1 | 295 | 571 | 823 | 1178 | 1411 | 1510 | ~2,150 |
| d3c | 100 | 340 | 634 | 848 | 999 | 1292 | ~1,826 |

So a champion trained on ~2,000 samples is fine-tuned on ~100–300 for its first generations. With
`--gate-every 1000000` the strength gate never runs, so **every** candidate is accepted — nothing
stops a small, unrepresentative sample from overwriting an inherited champion.

**This is a plausible story, and a plausible story is not a measurement.** The discriminator below
is designed so the story can fail.

## The discriminator

Resume from the champion, snapshot the output net at generations **5, 25 and 100**, and `netmatch`
each against the champion (160 pairs, depth 4, the paired instrument — never the ruler, whose
sequence has been refuted three times).

## Pre-registered readings

| gen 5 | gen 25 | gen 100 | reading |
|---|---|---|---|
| already ≈0.36 | ≈0.36 | ≈0.36 | **damage is immediate** — done while the pool is smallest, consistent with the mechanism |
| ≈0.50 | falling | ≈0.36 | damage is **gradual**, so pool size is NOT the cause and the story is refuted; something about continued training is the cause |
| ≈0.50 | ≈0.50 | ≈0.50 | the gen-108 reading does not reproduce — suspect the measurement, not the subject |

**What would falsify the mechanism:** a flat ≈0.50 at gen 5 followed by a decline. If the damage is
done before the pool has refilled, the pool cannot be what did it.

## Control, and why it is needed

The champion is netmatched **against itself** at the same pair count. It must return ≈0.500. Without
it, a systematically low reading from the harness (seed, colour assignment, a stale net path) would
be indistinguishable from real damage — and this repo has already retracted one finding where a net
silently failed to load and every arm looked identical.

## What it does not decide

Whether the dip is worth fixing. `d3c` recovered to 0.557 over 4,327 generations, so the cost may be
a transient the loop pays back. Sizing that needs the recovery curve, not this.

## Resource note

Runs on Existence's own cores (6-11) at nice 19 alongside the production run, for ~2 minutes of
training. The 4PC anchor on cores 0-5 is a TIMED benchmark and is not touched.
