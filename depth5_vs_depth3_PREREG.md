# PRE-REGISTRATION — depth 5 vs depth 3, matched start, equal wall clock

**2026-09-10 21:4x.** Written while both arms run and before either is measured.

## Why the control exists

My first version of this test ran the depth-5 arm alone and compared it to its own starting net. That
cannot attribute a result. At 8 games/generation depth 5 completes ~3 generations where depth 3
completes ~26, so a loss would be equally explained by *"deeper labels do not help"* and by
*"~100 deep games is too little data"*. A test that cannot distinguish its two explanations is not
worth the cores.

Both arms now start from **the same banked champion** via `--init`, run for the **same wall clock**
(2400s), on **equal cores** (3 each), differing in `--depth` alone.

```
  d5  : --depth 5, cores 6-8,  from d5_start.net  (= p1_champion.net)
  d3c : --depth 3, cores 9-11, from d3_start.net  (= p1_champion.net)
```

Each is judged by `netmatch` against **its own start** — the paired instrument, never the ruler.

## The prediction, unchanged from the earlier note

Measured node costs per move in this engine: depth 1 = 40, depth 3 = 2,352, depth 5 = 81,421. Depth
1 → 3 cost 59× and won decisively because depth-1 labels are nearly information-free. Depth-3 labels
are already informative, so **depth 5 must beat 35× fewer training steps on labels only somewhat
better. I expect it to lose or be unresolved.**

## What each outcome means, given the control

| d5 vs start | d3c vs start | reading |
|---|---|---|
| loses | **wins** | depth 5 is the wrong trade at this budget — the prediction holds, and the control proves the budget was survivable |
| loses | **also loses** | **the champion is at a ceiling for this configuration**, and neither depth helps. That is the more interesting outcome and it reframes the next lever as capacity or search, not labels |
| wins | either | deeper labels still pay and the datagen-depth lever has more in it |

The second row is the one I would have mis-read without the control: I would have blamed depth 5.

## Context that makes the second row plausible

`auto_promote` caught the depth-3 production run **regressing** at gen 2905 — 0.444 ± 0.030 against
the champion, interval entirely below 0.5. So depth-3 training from around this champion was already
costing strength before this test started. If `d3c` also loses, that is consistent and the ceiling
reading is the right one.
