# The node budget makes self-play 14.5 points more DECISIVE and yields 27% more usable rows — a third channel the pre-registration never named

**2026-09-12.** Measured while checking whether the two Candidate A arms were data-matched before
reading their gate. They are not, and the reason is a mechanism nothing on file anticipated.

Both arms: same frozen champion (`cand_start.net`, md5 `9545a35289e9`), same seed 20260912, same
binary, `--gens 2000 --games 8 --depth 3`, 16,000 games each. The ONLY difference is
`--datagen-budget 5269`.

| arm | games | decisive | decisive % | plies/game | trained rows | rows per decisive |
|---|---|---|---|---|---|---|
| A fixed depth 3 | 16,000 | 8,866 | **55.4%** | 109.9 | 500,327 | 56.4 |
| B budget 5,269 | 16,000 | 11,189 | **69.9%** | 99.4 | 633,865 | 56.7 |

```
decisive rate   55.4%  ->  69.9%     +14.5 points
game length     109.9  ->  99.4 plies   -9.6%
usable rows     500,327 -> 633,865     +26.7%
```

## Why this is one mechanism and not three numbers

**Rows per decisive game is unchanged** — 56.4 against 56.7, a 0.5% difference. The loop trains on
decided games near the terminal, so the entire **+26.7%** row gain is accounted for by the
**+26.2%** gain in decisive games. Nothing about the filter, the horizon or the pool changed; the
games themselves simply resolve more often.

And they resolve *sooner*: B generated **fewer** total positions (1,590,154 against 1,759,158, a
ratio of 0.904) while producing more decided ones. Shorter games, more of them decided, same
harvest per decided game.

The plausible reading, consistent with `budget_realised_depth_RESULT.md`: a budget gives more depth
to NARROW positions, which are disproportionately forcing — checks, recaptures, single-reply lines.
Seeing further exactly there converts drifting games into finished ones instead of letting them run
to the 160-ply cap.

## Why it matters to Candidate A

`structural_next_PREREG.md` frames the budget as a LABEL-QUALITY intervention.
`candidate_a_channel_FINDING.md` already showed it also moves the POSITION DISTRIBUTION. This is a
**third** channel, and it is the crudest of the three: the budget arm simply gets **27% more
training data** out of the same 16,000 games.

That is not a confound in the setup — it is a real consequence of the treatment, and any arm running
a node budget in this loop inherits it. But it means a win by arm B could be bought with more data
rather than better data, and the pre-registered decomposition (cell C: budget labels on the
control's positions) does **not** separate it, because cell C walks the control's trajectory and
therefore inherits the control's decisive rate.

**A fourth cell would be needed to isolate it**: the control arm run to the same number of TRAINED
ROWS rather than the same number of generations. Recorded, not run.

### Is that fourth cell already answered? Checked, and NO — but the prior is discouraging

`games_per_gen_RESULT.md` reads "quadrupling the data per generation changes nothing" (0.4642
against 0.4684), which looks like it closes the question outright: if 4x data does nothing, +27%
certainly does nothing.

It does not close it, because that file scopes itself explicitly:

> Neither arm's batch gate ever kept a batch ... Every sample in both columns is therefore *"five
> generations from a freshly resumed champion"*. This result says **more data does not shrink the
> resume transient**. It does **not** say more data fails to help a generation in the steady
> state — no arm has measured that.

Its arms ran `--gate-every 5` over 20 batches. These arms ran **2,000 generations**, far past the
resume transient `resume_dip_RESULT.md` puts at ~1,000 generations. Different regime, and
`nontransitive_walk_RESULT.md` is on record that the two regimes give different answers.

So the data-volume channel stays open for the steady state. But a 4x increase producing nothing
post-resume is a weak negative prior on +27% producing something here, which is why the fourth cell
ranks behind getting the A/B/C verdict rather than ahead of it.

## Status

Measured and reproducible from the arm logs. **The strength verdict is NOT in.** The first netmatch
reading was 0.439 ± 0.028 for B against A, but its provenance became unclear when two gate instances
overlapped, so all seed files were discarded and the three seeds are being re-measured from scratch.
Nothing here depends on that verdict; if anything it sharpens the question, because more decisive
games and 27% more rows is a large advantage for an arm that is not obviously winning.
