# The staged capacity experiment would have run at width 16 and reported width 64

**2026-09-10.** `w64_from_champion.sh` was staged and ready to launch as the next experiment after
the datagen-depth lever was spent. It is mis-specified, and its failure mode is the dangerous one:
it produces a plausible number that answers a different question.

## What it would have done

The script copies the shipped champion and resumes into rung 2:

```bash
cp -f p1_champion.net w64_start.net
timeout "$SECS" ... "$LEARN" --init w64_start.net --rung 2 ...
```

The champion is **width 16** — its header reads `45584e54 0100 1000`, i.e. magic `EXNT`, version 1,
`n_hidden = 0x0010 = 16`. And `--init` **adopts the saved net's rung, overriding `--rung`**, by
design and for a good reason (`main.rs`):

> ADOPT the saved net's rung rather than demanding it match `--rung`. The first version aborted
> unless the width equalled `WIDTH_MENU[--rung]`. That was fine only while ARCH could never widen:
> the moment a widening step is accepted the champion is width 32, and every later resume of a
> rung-0 run would abort on its own successful progress.

So the two flags are in direct conflict, and `--init` wins. Run for 15 s with exactly the script's
arguments, the engine says so itself:

```text
ARCH menu [16, 32, 64, 128, 256, 512]  start rung 2 (width 64)  arch-every 0
RESUMED at rung 0 (width 16), overriding --rung 2
...
ARCH: started at width 64 (rung 2), finished at width 16 (rung 0); 0 of 0 proposed steps passed both gates
```

**The whole run would have been width 16.** Its `netmatch` against `w64_start.net` would have been a
width-16 net against a width-16 net — an A/A — reported as a capacity result.

## The reporting line makes it worse, not better

The script's own summary greps for the width:

```bash
echo "w64: ... ($(grep -oE 'start rung [0-9]+ \(width [0-9]+\)' w64c.log | head -1))"
```

That matches `start rung 2 (width 64)` — the line printed **immediately before** the override. The
script would have printed **"width 64"** while running width 16. A check that reads the line above
the correction is worse than no check: it manufactures confirmation.

## Why `--arch-every 0` seals it

The script also passes `--arch-every 0`, which disables the ARCH arm entirely — the one mechanism
that *can* move the champion up the width menu. So the run could not have widened even in principle.
The experiment disabled the only route to the thing it was trying to measure.

## What the achievable experiment is

"Does more capacity help **from the champion**?" cannot be asked by resuming a w16 net into a w64
slot; that is not what resume means. Two designs are actually available, and they answer different
questions:

1. **Let ARCH propose the widening and gate it** — `--init p1_champion.net` with `--arch-every N > 0`.
   This is the designed path, and it is decision-relevant: if the widening step never passes its
   gates, *that is the answer* to whether capacity helps here.
2. **Matched random starts** — w16 and w64 both from random init at equal wall clock, judged on the
   absolute ruler. Clean capacity comparison, but it says nothing about the current champion.

Design 1 is the one worth running, because it asks about the net actually in play.

## The guard added

The script now **aborts** unless the width it achieved is the width it intended, read from the
`RESUMED`/override line rather than the line before it. `w64_prereg.md`'s framing — *"both arms
resume from the SAME shipped champion via `--init`, so the only difference is the accumulator
width"* — is unachievable as written and is marked so, rather than left to be discovered by a future
reader as a plausible-looking result.

## Method note

This was caught by reading `--init`'s implementation before launching, not by running it and
wondering about the number. The probe that settled it cost 15 seconds and printed the contradiction
in the engine's own words — the same discipline that separated "the env var is set" from "the engine
loaded the net" earlier today, after a published finding had to be retracted for exactly that gap.
