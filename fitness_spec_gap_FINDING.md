# The surrogate is not the one FITNESS §3 specifies — and that explains the saturation

**Headline.** Everything measured tonight (saturation at 23/23, cost outbidding the numerator 4.7×,
0 accepts in 99 gate calls) is downstream of a surrogate that differs from `docs/FITNESS.md` §3 in
three ways. The most important one is that **§3 defines the surrogate as a FILTER and the code uses
it as a RANKING function.**

## What §3 specifies, verbatim

> - Set: MATE-N, N in {1,2,3,4}, **500 each**, mined by retrograde walk from actual game endings in
>   own self-play [...]
> - Metric: found / (cost units consumed over the set), **reported per N**.
> - **Role: (a) a filter — a PROGRAM candidate must score >= 0.9x the champion on MATE-{1,2} and
>   >= 0.8x on MATE-{3,4} to reach the ladder**; (b) an early gradient before the eval is useful;
>   (c) the structural counterweight to reckless pruning.
> - **Not a substitute for Elo. A program can be a mate specialist and lose games; the ladder
>   catches that.**

## Three deviations, in increasing order of consequence

**1. Set size: 23 against a specified 2,000.** §3 asks for 500 positions at each of MATE-1..4. The
run builds `forced_mate_set` and reports `23/23 mates (floor 19)` plus an 8-position hard set. That
is ~1% of the specified set.

**2. Not stratified by N.** §3 says "reported per N", with different thresholds for MATE-{1,2} and
MATE-{3,4} — because a mate-in-1 and a mate-in-4 measure different things, and §3's own repair note
(`evolve.rs:43-56`) records that a mate-in-1-only set cannot punish shallowness. The implementation
collapses everything to one scalar.

**3. THE ROLE IS INVERTED, and this is the one that matters.** §3 makes the surrogate a FILTER: score
≥0.9× the champion and you *reach the ladder*, where games decide. The shipped code instead RANKS by
it and sends only the single highest-rate candidate to the gate (`popn[0]`, strict `rate > best_rate`).

A filter cannot be gamed by cheapness, because being cheaper than the champion buys nothing once you
are over the bar. A **ranking function** rewards cheapness without limit — which is exactly the
failure measured: `mates/Mcost` is a SPEED metric, and the one candidate ever accepted improved it
31.4% while holding the seed's own mate count.

**§3 anticipates this in its last line:** *"Not a substitute for Elo."* Using it to rank is using it
as a substitute for Elo.

## The spec-compliant mode already exists, and is default-OFF

`EXISTENCE_SPEC_FILTER` picks the first candidate with `r >= 0.9 * best_rate` (`evolve.rs:1801`).
That is §3's filter rule, threshold and all — the flag is even named for it. So the search track has
been running in a **non-spec mode by default**, with the spec-compliant mode behind a flag that no
standard arm has ever set.

Corroborating: PATH 1 (the same-play speedup promotion) has fired **8 times in this project's
history, all 8 under SPEC_FILTER**, and zero times in any standard arm. The champion has never moved
under the strict rule.

## What this does to tonight's work

- The saturation diagnosis stands, but its CAUSE is now upstream: 23/23 is saturated *because the
  set is 1% of the specified size*. A 2,000-position set stratified over MATE-1..4 would not sit at
  the ceiling.
- `EXISTENCE_HARD_WEIGHT` is a workaround for a set that is too small and unstratified. It may still
  help, and the running A/B still answers its question — but it is not the repair §3 asks for.
- The next lever I named earlier (`SPEC_FILTER`) is not an alternative idea. **It is the spec.**

## Not changed yet, deliberately

Four arms are mid-generation on a dose-response that is minutes from its first gate. Changing the
surrogate's role now would confound it. Order: let the A/B resolve, then run the spec-compliant
configuration — filter role, and a mate set built to §3's size and stratification — as its own arm.

**Do not read this as "the code is wrong and the spec is right" without the caveat §3 itself gives:**
the 500-per-N set must be "mined by retrograde walk from actual game endings in own self-play", and
at iteration zero there is very little self-play to mine. The small set may be a deliberate
bootstrap. What is NOT defensible on that ground is the ROLE inversion, which costs nothing to fix
and is what turns a speed metric into the thing selecting champions.

## §10 names this exact degenerate solution, and names the filter as its catcher

`FITNESS.md` §10 is a table of degenerate solutions against the check that catches each. Its first
row:

| Degenerate solution | Caught by |
|---|---|
| **Prune everything / return eval** | **mates-per-cost filter (3)**; ladder (7) |

"Prune everything" is searching less, which is exactly what a cost cut is. So the spec's FIRST line
of defence against cheapness is the mates-per-cost **filter** — and the implementation inverted that
filter into the ranking function that *rewards* cheapness. The designated catcher became the driver.

**The second line of defence was intact, but only by accident.** The ladder is the other named
catcher, and under the strict rule it did hold: 0 accepts in 99 gate calls means no cost-cutter was
ever promoted. But it held because the fixed 6-pair gate could not accept ANYTHING — it needed ~70%
of pairs and returned intervals spanning 0.5. That is not a filter working; it is a stuck system that
happened to be stuck in the safe direction. Where the bar was relaxed, the protection vanished
immediately: the veto rule promoted a cost-cutter (`STATE.md:2668`, "ACCEPT 15 mates", VERIFY 0.490),
and PATH 1 promoted 8 times under `SPEC_FILTER`.

**So as of tonight the ladder is a real catcher for the first time** — it resolves, it accepted an
obvious improvement at llr +3.08 in 26 pairs, and it rejected a real candidate at llr -3.18 in 18.
That makes fixing the filter's role newly worthwhile rather than newly urgent: with a working ladder
behind it, a filter that admits more candidates is no longer dangerous, because the games decide.
Those two changes are complements, and the ladder had to come first.

## What §3's specified set actually COSTS — measured, not guessed

The set-size deviation (23 against a specified 2,000) is easy to call a defect and harder to price.
Two arms now give two measured points on the same program at the same depth and budget, so the
scaling is measured rather than assumed:

    |set| = 23   seed cost   9,235,450,584     (control arm header)
    |set| = 77   seed cost  31,941,536,946     (bigset arm header)

    per position = 420,483,081 cost units
    intercept    = -4.4e8, i.e. ~0 against a 9.2e9 base -> cost is LINEAR in |set|

Extrapolating to the specified size:

    |set|   seed cost              vs the 23-position cost
       23     9,671,110,858          1.0x
       77    32,377,197,220          3.5x
      500   210,241,540,389         22.8x
     2000   840,966,161,556         91.1x

An ungated generation costs ~3.5 min at 23 positions, so **2,000 positions is ~5.1 hours per ungated
generation and ~5.3 days for a 25-generation run.** That is why the implementation uses 23, and it
is a real constraint rather than an oversight.

**~~CAVEAT that cuts the estimate~~ — MEASURED, and it was WRONG.** I wrote that §3's stratification
would reduce the cost, since a MATE-1 position is solved at depth 1 while the current set is
mate-in-2, so the 91x was "an upper bound, not a forecast". `crates/interp/examples/mate_surrogate_probe.rs`
measures it directly, and mate distance barely matters:

    depth   MATE-1/position   MATE-2/position   ratio
      1        3,077,185         3,093,122      1.01x
      2       38,114,680        33,907,025      0.89x
      3      431,889,314       413,506,638      0.96x

**DEPTH dominates, at ~11-12x per extra ply; mate distance is worth 0.89-1.01x.** An alpha-beta
search to a fixed depth explores that depth whether or not the mate is shallow. Both figures also
cross-check the arms' own 420,483,081/position at depth 3 (1.03x and 0.98x), which is an independent
confirmation from a different program on a different set. So stratifying buys nothing and the 91x
stands as a forecast rather than a bound.

**But the same table hands over a different lever, and it changes the answer.** Cost is ~12x per ply,
so §3's 2,000-position set costs:

    depth 3:  827,013,276,000  =  89.5x the current fitness cost   <- unaffordable
    depth 2:   67,814,050,000  =   7.3x                            <- affordable

And depth 2 does not give up the guard. From the same probe: the mate-in-2 set scores **17/40 at
depth 1 and 40/40 at depth 2**, so shallowness still LOSES mates (the guard bites, which is the whole
point of the forced-mate repair) while the set remains fully solvable. **§3's specified set is
affordable at fitness depth 2 — 7.3x, not 90x.**

**What that costs in exchange, stated rather than buried:** fitness depth is a real property of what
is being optimised, not a free knob. Dropping from 3 to 2 changes the programs the surrogate
prefers, and `netmatch.rs:23-29` records depth 4 as this project's strength standard with a measured
case where a depth-2 result did NOT hold at depth 4. So this is a trade to be tested, not a free
win — but it moves §3's set from "unaffordable" to "one arm".

**What this changes about the recommendation.** "Implement §3's set" is not a small fix, and saying
so is more useful than repeating that the code deviates. The tractable version is the middle ground
already running: 77 positions at 3.5x, which is affordable and already tightens the guard's licence
from 17.4% to 5.2%. If per-N costs turn out heavily skewed toward MATE-1 being cheap, a
stratified 500x4 may cost far less than 91x and become reachable — which is a measurement worth
making before the set size is decided either way.
