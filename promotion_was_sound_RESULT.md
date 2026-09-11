# The champion's promotion was sound — my suspicion that it was a lucky seed is refuted

**2026-09-11.** A one-seed promotion, re-tested two ways, holds. This file exists because I proposed
the opposite and was wrong, and the proposal was reasonable enough to be worth recording alongside
its refutation.

## The suspicion

`d3c` was promoted to champion on a single reading: **0.557 ± 0.032**, interval [0.525, 0.589],
clearing the project's bar (`rate − ci95 ≥ 0.5`) by **0.025**. `netmatch` printed its own warning at
the time:

> effect 0.057 against a between-seed sd of 0.047 → ~5 seeds for ~80% power. **ONE SEED CANNOT
> SETTLE THIS.**

The between-seed sd (0.047) is *larger* than the within-run interval (0.032), so a different seed
could plausibly have given a different answer. That matters far beyond one promotion, because
everything measured since has used that net as its reference:

* `prod2` read **0.365 ± 0.027** against it after 3,127 generations — a large regression.
* The plateau (`nontransitive_walk_RESULT.md`) was dated relative to runs resumed from it.

If the promotion had been a lucky draw, the champion would be *weaker* than its predecessor, the
loop would have been chasing an inflated target all night, and the plateau would have a mundane
explanation. That is a specific, checkable story, so I checked it.

## The measurement

Two re-tests, chosen to fail in different ways:

| reading | pairs | seed | score | interval | Elo |
|---|---|---|---|---|---|
| original (the promotion) | 224 | 20260907 | 0.557 ± 0.032 | [0.525, 0.589] | +39.8 |
| **extension** | **448** | 20260907 | **0.559 ± 0.021** | **[0.538, 0.579]** | **+41.2** |
| **independent** | 224 | **911911** | **0.538 ± 0.030** | [0.508, 0.568] | +26.5 |

**All three intervals are clear of 0.5.** The extension is the tightest and `netmatch` calls it *"A
is stronger, interval clear of 0.5"*; the fresh seed is more marginal but still ahead.

**The suspicion is refuted.** The promotion was correct and the champion is genuinely stronger than
the net it replaced, by something like +26 to +41 Elo.

## Why two re-tests and not one

The first attempt re-ran at 448 pairs with the **default seed** — which is the seed the original
used, so it replays the same openings and adds more of them. That tightens the estimate but shares
whatever opening luck the original had; it is an *extension*, not a replication. `netmatch.rs`
itself records why that distinction matters:

> bh_100 vs champion_long read 0.529 ± 0.030 at 224 pairs and was reported as a tie; at 896 pairs on
> **a fresh seed** it is 0.458 ± 0.016, champion_long stronger with the interval clear of 0.5.

So the fresh-seed run is the one that actually tests the seed hypothesis, and it is the one that
matters here. It came back at 0.538 — lower than 0.557, consistent with some seed luck in the
original, but still clear of parity.

## What it leaves standing, and what it removes

**Removed:** the convenient explanation for tonight's plateau. The champion is not inflated, so
`prod2` failing to beat it is a real difficulty rather than an artefact of a bad reference.

**Standing, and now sharper:** unguarded training from *this* champion regresses and then plateaus
(`resume_dip_RESULT.md`, `nontransitive_walk_RESULT.md`). The champion being genuinely strong makes
that harder, not easier, to explain away — which is the useful outcome. A refuted suspicion that
would have explained the plateau means the plateau still needs explaining.

## Method note

The fresh-seed run initially **crashed** and I nearly read the crash as "still running": the
argument order is `netA netB pairs DEPTH seed`, I put the seed in the depth slot, and it tried to
search to depth 911,911 and overflowed its stack. The log contained only the header, so a grep for
`scores` returned nothing — indistinguishable from a match in progress. `netmatch` now rejects a
depth outside 1..=12 with exit 2 and names the likely cause.
