# The nearest decision-theoretic shape is the one nothing can build, and the farthest is reachable

2026-09-11. Track A step 4. Measured by `crates/grammar/tests/reachability.rs`, which derives the
constructible set EMPIRICALLY — every operator applied at every position of every reference program,
recording which node kinds appear that were not in the input. Nothing here is read off `mutate.rs`.

## The measurement

| yardstick | nodes | vs seed | constructible? | blocked by |
|---|---|---|---|---|
| (b) mix-backup | 78 | **+7** | **NO** | no operator emits `Avg` or `Mix` |
| (a) extend-by-uncertainty | 85 | +14 | yes | — |
| (c) bound-gap stopping | 127 | **+56** | yes | — |

For scale, the whole of UCT-style MCTS is +60 from the same seed.

## What it establishes

**Edit distance and reachability are independent.** The shape closest to the seed cannot be
assembled at any edit count, while the one nearly as far away as a different paradigm can be. A plan
that ranks candidate techniques by "how small a change is it" is ranking on an axis that does not
predict whether the search can get there.

`(b)` is blocked on two primitives, not on its idea. A blend backup needs `Mix` — a three-argument
form — and `Avg`, and the mutation operators build hand-written shapes, none of which is a
three-argument blend. That is a fact about the operator set.

## What it does NOT establish, and the file this sits next to says it loudest

**Constructible is NECESSARY, not SUFFICIENT.** `reachability.rs` carries the counter-example in its
own header: hash reuse has been constructible since the memory operators landed and the loop has
still never built it, because the payoff is CONJUNCTIVE — `ladder_valley_RESULT.md` scores
probe-only at 0.991x, store-only at 0.997x, and the pair at 1.024x. Every single step is downhill,
so an acceptance rule of `rate > best_rate` can never take the first one.

So "(a) and (c) are reachable" means the search SPACE contains them. Whether a monotone path through
the FITNESS reaches them is a different question, and this instrument cannot answer it. Reading
these two rows as "the loop can find them" would repeat the error the header was rewritten to
correct.

## Recorded as pins, not prose

Two assertions now protect this, and they are deliberately different:

* **Rungs** must stay constructible — every declared rung has been since the memory operators
  landed, so one going unreachable is a REGRESSION in the search space.
* **Yardsticks** have their measured reachability PINNED. They were added to be measured, not
  guaranteed. If `(b)` becomes constructible, an operator gained the ability to emit `Avg`/`Mix`,
  and that should be a decision someone wrote down rather than a side effect.

Conflating the two would have turned this measurement into a false alarm: the first run failed with
"a rung became UNREACHABLE again — this is a regression in the search space itself", about a
yardstick that was never reachable and is not a rung.

## What follows

Making `(b)` constructible is a live option and a deliberate one. It would mean giving an existing
operator a source that can emit a three-argument blend — the same judgement call as extending
`ProbeRead` to a sixth source for `unc`, where the line was drawn at emitting the COMBINATION rather
than the primitive. It is not done here, because it belongs in a change that argues for it.

The cheaper reading is that `(b)`'s +7 was never the opportunity it looked like. Distance measured
the wrong thing.
