# The accumulator's crossover was a property of its DIFF, not of the width

`crates/pipeline/examples/search_bench.rs`, depth 4, 4-position suite. Node counts identical across
every arm at every width, so the ratios are work-for-work (that check is in the bench's own header:
"a speed number for two different trees is meaningless").

## What was already there, and what I nearly re-derived

The task list says "Incremental NNUE accumulator -- biggest single win". It is **already implemented
and already wired**, in `pipeline::search::Searcher` -- the search that datagen, the gate, `arch`,
`main` and every example actually run. `engine/src/search.rs` is the separate SEED reference binary.

`Searcher::new` also carried the measurement that gated it:

    hidden  32   refresh 1718969   incr 1560497   0.91x  LOSS
    hidden 128   refresh  889406   incr 1014240   1.14x  win
    hidden 512   refresh  240701   incr  363458   1.51x  win
    -> `incremental = ... && net.n_hidden >= 64`

**I reproduced those ratios before changing anything** (0.90x at 32, 1.10x at 128 on a contended
box), so the harness is the same one and the original finding stands.

## The diagnosis was right; the conclusion drawn from it was not

That comment explains the loss correctly: *"the saving scales with width (~38 rows rebuilt -> ~4
touched); the bookkeeping does not."* True. But the bookkeeping was not a constant of nature -- it
was **O(features ACTIVE)**: two `Net::active` scans, four passes over a 13-word bitset, and two
~38-element `Vec` builds, per node, at every width.

`FeatSnap::delta` (new, in `nnue`) diffs the same fields `active_with` reads as **per-plane bitboard
XORs**, so it is **O(features CHANGED)** -- typically 2-6. An untouched plane costs one compare.

## Result: the crossover is gone

    width    refresh     old delta        XOR delta
       8       967k          --        1188k   1.23x
      16       906k          --        1125k   1.24x
      32       747k        674k  0.90x  956k   1.28x      <- was a LOSS, now a win
     128       403k        442k  1.10x  561k   1.39x
     512       122k          --         185k   1.52x

**The XOR delta wins at every width measured, so the `>= 64` gate is removed and the DEFAULT width
(32) now benefits.** Measured again with defaults live: 1.32x at 32, 1.37x at 128.

Both arms are retained behind env switches (`EXISTENCE_FULL_REFRESH`, `EXISTENCE_OLD_DELTA`) so the
ratio stays re-measurable instead of becoming a claim in a comment.

## Correctness

Behaviour-preserving is asserted, not assumed:

* `nnue/tests/incremental.rs::feat_snap_delta_matches_full_refresh` -- **7041 move-deltas**, 60 of
  them changing >6 features. Random walks alone are not enough coverage (they almost never promote),
  so the suite forces promotion, castling and both en-passant colours by FEN.
* `engine/src/search.rs::tests::accumulator_search_scores_match_from_scratch_eval` -- an
  **independent second alpha-beta** written in the test, calling `Net::eval` at every leaf, compared
  against the accumulator search over 15 (position, depth) probes. It compares SCORES, not moves:
  the search shuffles, so which of several equal-valued moves returns is RNG-dependent, but the root
  value is order-independent by alpha-beta's soundness theorem.
* Node counts identical to refresh at every width in the bench above.
* Full workspace `cargo test --release` green, including perft and xcheck.

## Also fixed: the bench label lied below width 64

`search_bench` printed `incr` whenever `EXISTENCE_FULL_REFRESH` was unset, but `best_move` set
`incremental = ... && n_hidden >= 64`. **At hidden 32 the "refresh vs incr" comparison was an A/A**
-- both arms ran refresh -- and it read 1.00x. Anyone measuring small widths through this harness
would have concluded the accumulator was free. Now that the gate is gone the label is honest again.

## Honest limits

* Contended box (4 evolve arms + 6 datagen lanes). Ratios are back-to-back and reproduce across
  reps; the ABSOLUTE nps are depressed and are not comparable to the original comment's numbers.
* Depth 4, one 4-position suite, `Net::random`. The direction is structural (strictly less work per
  node, same rows touched); the magnitudes are specific to this configuration.
* This is a SPEEDUP of the seed's eval, not a strength change. It buys datagen volume and gate
  throughput. It is not an Elo claim and no gate has been run on it.
