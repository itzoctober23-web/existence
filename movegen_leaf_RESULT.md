# A leaf never reads the move list — not building one is a 1.72× speedup

**2026-09-10.** The largest throughput win this project has measured, and it was sitting in a file
that had already concluded there was none.

## The waste

`ab()` began every node with `let list = pos.legal_moves();`. At `depth == 0` that list is used for
exactly one thing — `is_empty()`, to tell mate and stalemate from an ordinary position — and then
dropped. `node_profile` prices the discarded work:

| primitive | ns | share of a leaf |
|---|---|---|
| **`legal_moves()`** | **393.4** | **59.8%** |
| eval | 223.7 | 34.0% |
| make + unmake | 40.8 | 6.2% |
| leaf total | 657.9 | |

and 89% of movegen is the pin-and-emit residual a leaf never looks at. Leaves are **~66%** of a
depth-4 frontier (`throughput_RESULT.md`), so this was the single largest piece of wasted work in the
engine.

## The change

`Position::has_legal_move()` answers the only question asked, from work the generator does first
anyway:

```rust
let checkers = self.attackers_to(ksq, them, self.all);
let danger   = self.attacks_by(them, self.all ^ bb::bit(ksq));
if attacks::king(ksq) & !self.occ[us.idx()] & !danger != 0 { return true; }   // king can move
if checkers.count_ones() >= 2                              { return false; }  // double check, stuck
!self.legal_moves().is_empty()                                                // otherwise: ask it
```

**Correctness is by construction, not by a parallel reimplementation.** Rewriting the pawn, castling
and en-passant rules here would create two generators that can silently disagree — and a
disagreement is not a slow search, it is a search that scores an ordinary position as **mate**. So
the function settles only the two cases that follow directly from what `legal_moves()` itself does,
and delegates everything else to the real generator.

The fallback pays the king probe *on top of* a full generation, so this is a bet that the king can
usually move. Measured over a 2,000-position random-play corpus: **82.5%**, and
`board/tests/has_legal_move.rs` asserts that floor so the bet cannot silently stop paying.

## The measurement

**Equivalence first.** A speed ratio between arms doing different work is meaningless, and this
engine has already produced an nps "win" that was a wall-clock **loss** for exactly that reason
(`throughput_RESULT.md`). Node counts, before against after:

| | depth 4 | depth 5 |
|---|---|---|
| width 16 | 121,562 = 121,562 | 2,082,750 = 2,082,750 |
| width 32 | 144,321 = 144,321 | 1,701,918 = 1,701,918 |

Identical. The engine's own `go` is byte-identical too — same move, same score, same node count on
all five probe positions.

**Then wall clock**, best-of-7:

| arm | before | after | speedup |
|---|---|---|---|
| pipeline search, w16 d5 | 0.777 s | 0.452 s | **1.719×** |
| pipeline search, w32 d5 | 0.724 s | 0.468 s | 1.547× |
| engine, depth 6 midgame | 1.242 s | 0.718 s | **1.729×** |

At the project's own measured **30 Elo per nps doubling**, 1.72× is **~23 Elo** — against ~6 for
int16 quantization and ~19 for a *completely free* eval. It is not a ply (that needs 9.17×).

## What this corrects

`throughput_RESULT.md` closes with:

> Combined with the movegen decomposition finding no hot spot, and eval sitting at third, **there is
> no cheap throughput win available in this engine.**

That conclusion followed a careful decomposition of movegen — `attackers_to` 3%, `attacks_by` 8%,
residual 89% — hunting for a hot spot *inside* it. There is none; the cost is spread across emission
exactly as that file says. **The question it did not ask was whether the call needs to happen at
all.** Making movegen faster was hopeless; declining to call it was 1.72×.

The same file's ranking is what pointed here, and it was right: it put `legal_moves()` first and eval
third, and explicitly said the loop brief's "eval is the biggest single win" was wrong. The brief
still says it.

## Consequence: `NPS_PER_MS` is stale again, by its own rule

The engine converts a movetime into a node budget through `NPS_PER_MS`, set to 2000 earlier today
from a **worst contended** observation of 2,239,873. Re-measured on the loaded box with the same
method — 4 positions × 4 repeats, take the worst — the engine now floors at **2,862,630 nps**. The
constant is 30% low, so a movetime again buys a shallower depth than the clock affords, which is the
defect that made a previously shipped +6.3% worth zero. Re-calibrating it is a separate change and
must use the worst contended figure, never this section's uncontended best-of-7.

## Scope

The pipeline search is what **datagen and netmatch** run, so this is a 1.7× on the training loop as
well as on play — and the training loop is where nearly all of this box's compute goes.

Not claimed: any Elo. Nothing has been gated. The 30 Elo/doubling conversion is a model, and
`node_profile`'s own self-check still reports the primitive attribution as INCOMPLETE at 1.72×
over-prediction, so treat ~23 Elo as an estimate and not a result.
