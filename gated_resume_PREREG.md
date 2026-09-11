# PRE-REGISTRATION: does the resume dip survive a live strength gate?

**2026-09-10 23:0x, written before any net is measured.**

## What is already established

`resume_dip_RESULT.md` measured, with the gate OFF (`--gate-every 1000000`):

| gen | vs the champion it resumed from |
|---|---|
| 5 | 0.492 ± 0.036 |
| 25 | 0.411 ± 0.034 |
| 100 | **0.366 ± 0.035** (−95.4 Elo) |
| 4,327 | 0.557 ± 0.032 |

and killed its own pre-registered mechanism: the replay pool is 72% full at gen 5 where there is no
damage and 100% full at gen 100 where damage is maximal, so the pool cannot be the cause.

The one candidate the data has not killed is that **training is unguarded**. `main.rs:856` makes
this exact: `--gate-every K > 1` sets `batch_mode`, and then *"No match is played ... the Accept
short-circuits `better`"*. Every candidate is adopted. The production log confirms it: `ACCEPT` on
**5,974 of 5,974** generations.

## The discriminator

Identical to `resume_dip.sh` in every respect except `--gate-every 1`, which turns the real
per-generation match back on. Same champion, same seed, same snapshots (gen 5 / 25 / 100), same
`netmatch` at 160 pairs against the champion, same champion-vs-itself control.

## Pre-registered readings

| gen 5 | gen 25 | gen 100 | reading |
|---|---|---|---|
| ≈0.50 | ≈0.50 | **≈0.50** | **the dip is caused by unguarded acceptance.** The gate is the fix, and the cost of using it is known: it accepts ~8.87% of candidates, so progress is slow but monotone |
| ≈0.50 | falling | **≈0.37** | **unguarded acceptance is REFUTED too.** The dip survives a live gate, so it is not about what gets adopted — it would have to be the datagen distribution shifting under a resumed net, and this file's whole line of attack is wrong |
| dip but shallower (≈0.43–0.47) | | | the gate *reduces* the damage without preventing it — it accepts some genuinely worse candidates, which `accept_rate_vs_noise_RESULT.md`'s 2.50% false-positive floor says must happen sometimes |

**What would falsify "unguarded acceptance is the cause":** a gen-100 reading materially below 0.5.
If the champion still loses ~95 Elo to itself while every change had to win a match first, then what
is adopted is not what is doing the damage.

## Why this is NOT a re-derivation

Three files already measure this gate and I read them before writing this. `acceptance_floor_RESULT`
measures the SIZE of edge it demands (2.7× what a generation produces); `accept_audit_RESULT`
measures the QUALITY of what it keeps (just above 0.5); `accept_rate_vs_noise_RESULT` measures the
RATE (8.87% against a 2.50% floor). **None measures whether the champion REGRESSES without it**,
which is the only question here.

## The tension this will expose either way

`accept_rate_vs_noise_RESULT.md` records the gated loop showing *no* measurable gain over
400-generation windows (0.474 ± 0.031, three readings at or below 0.5). The ungated loop dips to
0.366 by gen 100 and then reaches **0.557** by gen 4,327. If both hold, ungated training ends up
**stronger** than gated training despite the dip — because the gate's floor blocks nearly everything.
That comparison is not clean (different baselines, different window definitions) and this run does
not settle it; it is flagged now so the result is not over-read in either direction.

## Cost and resources

The gate plays a sequential match per generation, so generations are far slower than the 170/min of
the ungated run. The script measures its own rate and reports it rather than assuming, and caps its
wall clock. Runs on Existence's cores (6-11) at nice 19 beside the production run; the 4PC anchor on
0-5 is a timed benchmark and is untouched.
