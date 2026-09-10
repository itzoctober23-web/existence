# EPS 0.10 is INERT BY CONSTRUCTION: the band it buys is empty

Measured 2026-09-09 23:5x from the arms' own histogram field, which was already on disk. No new run.

## The measurement

Every `gate_*.log`, every generation, every lineage — the `[>=.98:N .90-.98:N .50-.90:N <.50:N]`
histogram summed:

    over 78 gen-lines, 60 scored candidates:
      >=.98       40   66.7%  ########################################
      .90-.98      0    0.0%
      .50-.90      6   10.0%  ######
      <.50        14   23.3%  ##############

Retention is `x.2 >= top * (1 - EPS)` (`evolve.rs:1730`):

* **EPS 0.020** keeps the `>=.98` band.
* **EPS 0.100** keeps `>=.98` **and** `.90-.98`.

**So EPS 0.10 buys exactly one band, and that band holds 0 of 60 candidates.** The arm on core 13
has been running since 20:04 testing a parameter that cannot change the population it selects.
Verified it really is configured: its own header reads `EPS=0.100` against the control's `EPS=0.020`,
so this is inertness, not a misconfiguration.

## The distribution is BIMODAL, and that is the real finding

Mutations to this seed either **preserve behaviour almost exactly** (≥0.98x, 66.7%) or **break it
badly** (<0.90x, 33.3%). Nothing lands in between. That is a property of the program space, not of
the retention rule, and it has three consequences that were previously attributed elsewhere:

1. **Widening EPS cannot work at any setting below ~0.10.** The next non-empty band starts at 0.90,
   so the first EPS that changes anything is 0.10-and-a-bit — and what it then admits is the
   `.50-.90` band, i.e. programs that have lost half their mates. That is not diversity, it is the
   mate-selling market this project has measured six times.
2. **The population collapse to 2 is not an EPS failure.** Measured across control, EPS 0.10 and
   composition arms: MAIN holds `pop 2` in every one. Only the ≥0.98 members survive, and few of
   them are DISTINCT (`distinct:1` on most lines). Widening the window does not help because the
   candidates simply are not there.
3. **The `ladder_valley` premise needs no help from EPS.** The valley halves measure 0.991x
   (probe-only) and 0.997x (store-only) — both ≥0.98, so both are ALREADY retained at the default
   EPS 0.020. The EPS 0.10 arm was launched on the reasoning that a wider window was needed to keep
   them alive. It was not: they were never at risk.

## Honest limits

* **n = 60 scored candidates** across 78 gen-lines is not large, and the arms are young (gens 1-4).
  A 0/60 in a band whose neighbours hold 40 and 6 is strong evidence of a gap rather than of
  sampling, but it is not proof that the band is empty in principle.
* The histogram counts CANDIDATES, not distinct programs. A generation with `distinct:1` contributes
  several identical rates, so the effective sample is smaller than 60.
* This says nothing about EPS in a run whose champion has MOVED. Every arm here still sits on the
  seed, so `top` is the seed's rate throughout, and the bands are measured relative to a fixed point.

## What follows

Core 13 is running a measured-inert configuration. The band structure — not the retention width — is
what a diversity intervention has to change, and nothing in the current knob set does that.
