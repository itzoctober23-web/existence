# Budget instability is PERIODIC in realised depth — period ≈ 1 ply, and the second peak is worse

**2026-09-12 08:45.** `budget_instability_is_boundary_straddling_RESULT.md` explained the instability
curve as boundary straddling and flagged one limitation: "the sweep has six points and the right-hand
limb is not bracketed — instability may keep falling past 18000, or rise again at the 3/4 boundary."
It rises again, and the structure is periodic.

## The curve

Same instrument, budget extended from 18000 to 70000, seed 20260912:

```
budget   mean realised depth   straddle%   instability
  2000          2.27              4.0         -0.1
  5269          2.63              6.9         -2.1     <- the operational budget
  8000          2.85              9.4         -4.0     <- PEAK 1   straddling 2/3
 18000          3.34              5.8         -1.9     <- TROUGH   pinned near 3
 25000          3.45              5.7         -1.7
 35000          3.56              7.2         -2.9
 50000          3.71              8.9         -4.3
 70000          3.89              9.2         -5.8     <- PEAK 2   straddling 3/4
```

Second seed (777001) on the rising limb: 25000 → 6.6%, 35000 → 7.5%, 50000 → 9.6%. Same rise.

**Peak-to-peak is 2.85 → 3.89 ≈ 1.04 plies.** The straddle fraction — and the instability that tracks
it at r = +0.98 — is periodic in realised depth with a period of one ply.

## The turnover, observed — the period closes

The second seed reached a deeper mean and shows the fall:

```
seed 777001   25000  depth 3.50  straddle 6.6%   instability -2.0
              35000  depth 3.64  straddle 7.5%   instability -1.9
              50000  depth 3.83  straddle 9.6%   instability -3.7    <- peak
              70000  depth 3.99  straddle 8.9%   instability -2.9    <- FALLING, pinned near 4
```

At mean depth **3.99** — essentially the integer — straddling drops and instability with it. That is
the full cycle: rise into a boundary, peak while straddling it, fall once the distribution pins on
the far side. Seed 20260912 only reached 3.89 and is still at its peak there (9.2%), which is
consistent: it had not yet pinned.

**Where the seeds disagree, and it is not hidden:** at 70000, seed 20260912's instability is −5.8, its
largest, while its straddle is roughly flat (8.9 → 9.2). Seed 777001's instability falls to −2.9 with
straddle. So the straddle-instability correspondence, which is r = +0.98 over the earlier range, is
looser at the extreme right. One point per seed is not enough to say whether that is noise or a real
decoupling, and it is recorded rather than smoothed.

## Why that is the boundary model's own prediction

A budget produces a *distribution* of realised depths. When that distribution sits near an integer,
almost every position completes the same number of plies on any two runs and straddling is low. When
it sits between integers, a large share of positions can go either way and straddling is high. The
period is therefore one ply by construction — and it is the first prediction of this model that was
made in advance and then observed.

## The part that was NOT predicted, and matters more

**The second peak is worse than the first: −5.8 against −4.0.** Instability is not merely periodic, it
is growing with depth.

A plausible reading is that a ply is worth more the deeper you are: flipping between 3 and 4 plies
changes the chosen move more often than flipping between 2 and 3, because the extra ply resolves more
tactics. That is an explanation, not a measurement — the discriminator would be to measure move
change conditional on a depth flip at each boundary, which is one more column in the same harness and
is not run here.

## Consequence: the operational budget sits on a rising limb, and the trough is narrow

```
budget  5269 (in use)   depth 2.63   straddle 6.9%   instability -2.1   -> rising toward PEAK 1
budget 18000-25000      depth 3.3-3.5 straddle 5.7-5.8%  instability -1.7 to -1.9  -> THE TROUGH
budget 50000+           depth 3.7+   straddle 8.9%+  instability -4.3 to -5.8  -> PEAK 2
```

This **refines** the recommendation in the previous file rather than repeating it. That file said "a
larger budget is both deeper and more stable", which is true at 18000 and **false at 50000**. The
correct statement is that stability has a narrow optimum: the trough spans roughly **18000–25000**,
and going further makes things worse than the operational budget already is.

`candB18` — the arm training now under `budget_18000_PREREG.md` — happens to sit in that trough. That
was chosen from a six-point sweep before the second peak was known, so it was luck rather than
judgement, and it is worth recording as such.

## What this does NOT establish

* **Not that instability causes the harm.** That is still the open causal step; this is a search
  diagnostic and no games were played. `budget_18000_PREREG.md` is the experiment that tests it, and
  its result is confounded by compute in one direction, as recorded there.
* **Mean depth is a summary of a distribution.** Peak and trough locations are read off means, and the
  real driver is the shape. The trough sitting at mean 3.34–3.45 rather than exactly 3.0 is
  consistent with a right-skewed distribution but is not evidence for one.
* **Nothing ships**, and no figure is quoted as Elo.

## A parsing error worth recording

Assembling this table, I first re-parsed the ORIGINAL sweep files with the field offsets of the
CURRENT binary. The straddle column was added after those runs, so `$(NF-1)` picked the wrong field
and produced depths of −0.12 and −2.11 — impossible values that were obviously wrong only because
depth cannot be negative. Had the corrupted field been plausible, it would have gone straight into
this table. **Output-format changes silently invalidate every parser written against the old format**,
and the guard is to re-measure rather than re-parse when the instrument has changed.
