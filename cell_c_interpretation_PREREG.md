# How to read cell C — written with ONE of two seeds in, before the second lands

**2026-09-12 03:57.** First reading: `candC_labels vs candA_fixed = 0.395 ± 0.028`, interval
[0.367, 0.423], wholly below 0.5. Second seed still running. Writing the interpretation rule now so
the reading is not chosen after seeing both.

## The number, in context

```
B vs A   0.4383  (672 pairs, 3 seeds)   budget drives BOTH moves and labels
C vs A   0.395   (224 pairs, 1 seed)    budget drives LABELS ONLY; moves from fixed depth 3
```

Cell C is **worse than arm B**, which is the opposite of the naive expectation that changing one
thing should hurt less than changing two.

## The reading I will NOT take, and why

The tempting conclusion is "the label channel is where the damage is, and it is bigger than the
combined effect". That does not follow, because **cell C introduces a defect neither A nor B has.**

* In arm **A**, the recorded label and the move played come from the SAME depth-3 search.
* In arm **B**, they come from the SAME budget search.
* In cell **C**, the move comes from the depth-3 search and the label from a DIFFERENT budget
  search. The training target therefore describes a search that did not choose the move that was
  actually played.

So cell C varies two things against A: the labelling policy **and** the consistency between label
and move. A and B are both self-consistent; C is not. A result worse than both arms is exactly what a
label/move mismatch would produce, and it cannot be separated from label quality by this design.

This is the second interpretation correction on cell C. `candidate_a_channel_FINDING.md` already
records the first: it was built to hold positions fixed and cannot, because in a self-play loop the
generator is the net being trained.

## The rule, fixed now

| second seed | reading |
|---|---|
| also wholly below 0.5, near 0.395 | **"decoupling the label from the move that was played is harmful"** — reported as that, NOT as "labels are the damaging channel". |
| contains 0.5, or disagrees in sign | one seed was noise at 224 pairs; report as UNRESOLVED and do not build on it. |

Either way this is **not** a ship, **not** a decomposition of Candidate A's loss, and **not**
evidence about label quality on its own. The design that could isolate label quality is
`budget_label_ab.rs` — a fixed corpus, no self-play, one column swapped — which is built,
smoke-tested, and queued to run last.

## Status

One seed of two. Nothing concluded, nothing shipped.
