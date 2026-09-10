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
