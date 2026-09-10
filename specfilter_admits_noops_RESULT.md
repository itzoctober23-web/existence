# SPEC_FILTER admits NO-OPS to the game gate, and spends the whole budget on them

`gate_specfilter_s1` runs FITNESS §3's filter role — pick on `r >= 0.9 * best_rate` instead of the
strict `r > best_rate`. Measured after ~3 hours: **2 generations reached, 3 gates played, 0 accepts.**

## What it is gating

    gen 1 MAIN  VERIFY 0.500+/-0.016 (96 pairs)
    gen 1 MAIN  gate INCONCLUSIVE llr +0.00  0.500+/-0.050 (60 games W-D-L 8-44-8)  mates 23  surrogate 0.002490  ABOVE:0
    gen 1 MCTS  gate INCONCLUSIVE llr +0.00  0.500+/-0.050 (60 games W-D-L 2-56-2)  mates 15  surrogate 0.001406  ABOVE:0
    gen 2 MAIN  gate INCONCLUSIVE llr +0.00  0.500+/-0.050 (60 games W-D-L 8-44-8)  mates 23  surrogate 0.002487  ABOVE:0

The seed's own header reads `23/23 mates, 9235450584 cost, 0.002490 mates/Mcost`. So the gated
candidates carry **the seed's exact mate count and the seed's exact surrogate**, and the games come
back **perfectly symmetric** (8-44-8, 2-56-2) at exactly 0.500 with `llr +0.00` and `ABOVE:0`.

**These are behaviourally identical programs.** The gate is not failing to resolve them — there is
nothing to resolve.

## Why the filter admits them, which is not a bug in the filter

The strict rule requires `rate > best_rate`, which excludes equality by construction. §3's filter
requires only `rate >= 0.9 * best_rate`, and a no-op scores **exactly** `best_rate` — comfortably
inside. Generation 2 is sharper still: `surrogate 0.002487` against the seed's `0.002490` is
*slightly worse*, and `0.002487 >= 0.9 * 0.002490` admits it anyway.

`evolve.rs` already records this invariant break for the OTHER acceptance route: *"the 'costs less'
half was silently guaranteed by the caller, because the strict filter picks only when
`popn[0].2 > best_rate`. EXISTENCE_SPEC_FILTER breaks that unstated invariant."* That was fixed for
PATH 1 (`92a542a`, now `same_play && rate > best_rate`). **The same invariant break on PATH 2 was
never addressed** — nothing stops a no-op reaching the game gate.

## The cost, which is the point

Each of these costs **96 VERIFY pairs plus 60 games** to learn that a program identical to the
champion plays identically to the champion. That is why this arm sits at generation 2 while the
control is at 6 and both composition arms are at 5-6: it is spending its entire budget adjudicating
ties.

## What this settles about §3's filter role

`relaunch_spec_filter.sh` pre-registered: *"a champion that moves here is expected and is NOT by
itself evidence of strength — the ladder decides that."* The measured outcome is weaker than that
anticipated: **the champion does not move at all.** The filter admits candidates that cannot differ
from the champion, the gate returns INCONCLUSIVE, and no promotion occurs.

So the §3 role inversion, run as specified, does not produce the mate-selling promotions I expected
it to. It produces **tie adjudication**. That is a different failure and a cheaper one to fix: a
no-op check before the gate (`rate == best_rate` and identical play → skip, no games) would recover
the whole budget. That check is exactly PATH 1's `same_play` test, already written and already
verified against `ab_hash`.

## Honest limits

* One seed, one arm, 3 gates. The pattern (identical mates, identical surrogate, symmetric W-D-L,
  `llr +0.00`) is unambiguous per gate, but 3 gates is not a rate.
* `ABOVE:0` on every line means no candidate exceeded the champion, so this arm has not yet been
  shown the case §3 was actually meant to admit — a candidate that is slightly WORSE on the surrogate
  but better in games. It may still produce one; it has only reached generation 2.
* INCONCLUSIVE is the gate behaving correctly. Nothing here indicts the gate.
