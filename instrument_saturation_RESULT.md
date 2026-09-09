# The frozen-origin metric saturates, and it has now reversed two signs

2026-09-08. The instrument I trusted all session — the one that "resolved" capacity, draws and
horizon while the champion gate was condemned as blind — fails at the top of its own range.

## What happened

`examples/netmatch.rs` plays two saved nets directly. Running it on arms already measured against
the frozen origin:

| comparison | vs frozen origin | head-to-head (224–448 pairs) | agree? |
|---|---|---|---|
| blend 0.75 vs 0.25 | +0.112 ± 0.033 | **0.768 ± 0.022** | ✓ |
| horizon cap10 vs cap1000 | widening +0.064 ± 0.034 | **0.222 ± 0.018** (widening) | ✓ direction |
| champion_long vs bn_075 | 0.861 vs 0.847 → +0.014 | **0.579 ± 0.028** | ✓ but 5× larger |
| **blend 0.75 vs 1.00** | **0.75 better, +0.015 ± 0.033** | **0.459 ± 0.030 → 1.00 better** | ✗ **REVERSED** |
| **capacity w16 vs w64** (equal gens) | **w64 far better, 0.967 vs 0.838** | **0.522 ± 0.022 → w16 better** | ✗ **REVERSED** |

Both reversals have intervals clear of 0.5. These are not ties being read two ways.

The direction convention is verified rather than assumed: the 0.75-vs-0.25 match returns 0.768 for
0.75, matching the sign the origin metric gives for a gap large enough that both instruments agree.

## The mechanism is saturation, not size

The tempting reading — "the origin metric fails on small differences" — does not fit. w64's origin
gap was **0.129**, the largest in the table, and it still reversed.

What both reversals share is a net near the **top of the scale**. w64 scores **0.967** against a
random opponent; there is almost no room left to express a difference, so two nets that both
crush the origin get compressed into whatever noise remains. The same applies to
`champion_long` (0.861) versus `bn_075` (0.847): a 0.014 origin gap corresponds to a **+0.079**
head-to-head edge, roughly 5× compression.

**Beating a random opponent is a saturating measurement.** It discriminates well in the middle of
its range and stops discriminating exactly where the strong arms live.

## What this costs

Every arm in the ceiling investigation was scored against the frozen origin, and the two levers
still standing sit in or near the danger zone:

* **datagen depth, +0.025** — the only surviving ceiling candidate, an effect smaller than the
  +0.015 that just reversed. Its replication is running now and **must be read head-to-head**.
* **blend high side** — I reported last tick that the 0.75/1.00 plateau "survives on the
  non-compressing instrument". That is now **withdrawn**: the instrument was compressing too, and
  the direct match says **1.00 beats 0.75**. The shipped default may be on the wrong side.

The draw filter (+0.086) and horizon (+0.064) were measured with both arms in mid-range and the
horizon one is directly confirmed head-to-head, so those stand.

## What this does NOT establish

* **Head-to-head is not ground truth either.** Non-transitivity has already been demonstrated in
  this tree — a net beat `champion_long` while scoring worse against the origin. Two nets trained
  toward different targets can have a stylistic matchup edge that is not general strength. The
  honest position is that **neither instrument is an oracle**: the origin match saturates at the
  top, the champion match compresses in the middle, and a direct match carries matchup effects.
* **One match per pair, one seed.** The reversals need a second seed before the blend default moves.
* This does not retroactively rescue anything the champion gate called a null. Two instruments being
  flawed in different regimes is not permission to pick whichever agrees.

## What changes

1. Arm-vs-arm decisions get measured **head-to-head**, with the origin match kept only as a sanity
   anchor for "did this learn at all".
2. The depth replication's verdict block gains a direct match between its d2 and d3 arms.
3. Any claim in the tree resting on an origin gap **below ~0.05, or on a net scoring above ~0.95**,
   is provisional until re-measured directly.
