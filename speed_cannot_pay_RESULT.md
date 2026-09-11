# The engine cannot spend a speedup at all — and quantization is worth ~5 Elo, not a ply

**2026-09-10.** Two measurements about the "inference speed → search depth" lever. The direction of
the lever is right and is confirmed here; the specific mechanism proposed for it is not, and there is
a blocker upstream of both that makes every speed number so far worth exactly zero.

## 1. THE BLOCKER: nothing converts speed into depth

`crates/engine/src/main.rs` hardcodes `let mut depth: u32 = 4` and, on `go`, calls
`search.best_move(&mut pos, depth)`. It never reads `movetime`, `wtime`, or `btime`. **The engine
plays a fixed number of plies regardless of the clock.**

So a 2× faster engine searches *the identical tree* in half the time and scores *identically*. Every
speed result in this repo has been measured against an engine that cannot spend the speed:

* `simd_refuted_RESULT.md` — `target-cpu=native` "buys NOTHING" (0.3%/0.6%). It could not have.
* `width_clock_RESULT.md` — 11 widening attempts, sign never turned. A wider net at fixed depth just
  takes longer on the same tree.
* the shuffle-division fix shipped today — **+6.3% nps, and therefore +0 Elo.**

**MASTER_PLAN line 43 specifies the gate at fixed TIME** ("SPRT ladder at fixed TIME — STC filter,
LTC confirmation"). The engine plays fixed DEPTH. That gap is why the speed track has looked dead:
not because speed does not matter, but because the instrument cannot express it.

## 2. Quantization cannot buy a ply, and the gap is a factor of seven

**What a ply costs**, from `search_bench` node counts at width 16 — deterministic, so contention
cannot distort them:

| step | nodes | ratio |
|---|---|---|
| depth 3 | 16,218 | — |
| depth 4 | 121,562 | 7.50× |
| depth 5 | 2,082,750 | 17.13× |
| depth 6 | 12,494,302 | 6.00× |

Geometric mean **B = 9.17×**. One extra ply at fixed time requires a **~9× nps speedup.**

**What eval costs**, from `node_profile` — measured, not recalled:

```
  make + unmake (pair)     44.9 ns
  eval                    247.2 ns
  shuffle + buffer copy   238.2 ns
  interior node           725.1 ns
  leaf node               972.2 ns      eval's share of a leaf: 25.4%
```

So the ceiling on any eval optimisation is `972.2 / (972.2 − 247.2)` = **1.34×**, achieved only by
making eval *entirely free*. int16 quantization plausibly halves it, giving **1.15×**.

Converting to Elo with the standard model — Elo ∝ log(nodes), 89 Elo/ply
(`elo_per_ply_RESULT.md`), one ply = 9.17× — gives **27.8 Elo per doubling of nps**:

| change | speedup | doublings | Elo |
|---|---|---|---|
| int16 eval 2× faster | 1.146× | 0.196 | **~5** |
| eval entirely FREE | 1.341× | 0.423 | **~12** |
| **one ply** | **9.17×** | 3.20 | **89** |
| for comparison — all of training | — | — | **~235** (untrained 954 → gen200 1206) |

**Quantization is worth roughly 5 Elo, and buys 0.20 of a ply rather than one.** The claim that it
"buys one ply" is off by a factor of ~8 in speedup terms. It is not more than everything the training
loop has produced; it is about 2% of it.

This is a bound on the *specific mechanism*, not on the lever. 89 Elo/ply is real and the curve has
**not flattened by depth 5**, so depth remains the largest single lever measured in this project.

## 3. What would actually buy plies is spec-locked, deliberately

B = 9.17 is enormous. Well-ordered alpha-beta approaches the square root of the move count; this
search is near the unordered worst case. The reason is in the Given column, verbatim
(MASTER_PLAN line 38):

> **Seed search program: BARE alpha-beta — recursion with a window, nothing else. No ordering, no
> hash reuse, no iterative deepening, no quiescence, no extensions, no reductions, no pruning.**
> *Everything layered on it since (where nearly all its strength lives) must be discovered.*

`search.rs` goes further and **actively shuffles** children to deny even the accidental prior in
movegen emission order. `node_profile` prices that denial: **shuffle + buffer copy = 238.2 ns/node,
24.5% of a leaf — almost exactly what eval costs.** The project spends about a quarter of every node
refusing to be given a move ordering.

That is a design choice, not a defect, and this file does not propose reversing it. But it settles
where the plies are: **branching-factor reduction is worth multiples of anything available in the
eval, and it is reachable only through P2** — the search track that must *discover* ordering, hash
reuse and pruning. That is the same conclusion `gate_power_RESULT.md` reached from the other end
("the generator, not the gate, is what has no gradient"), arrived at independently.

## What this makes next, in order

1. **Make the engine spend time** — read `movetime`/`wtime` and choose depth from a budget. Until
   this exists, no eval optimisation can be measured as Elo by any instrument here, and the gate is
   not the fixed-TIME gate the spec calls for. This is the unlock, and it is worth more than the
   quantization it would enable, because it also retroactively makes the +6.3% already shipped
   worth something.
2. **Then** quantize, and expect ~5 Elo — worth having, cheap, and correctly sized in advance rather
   than discovered to be small afterwards.
3. **Datagen depth** is measured separately and in flight (`depth_ruler.sh`).

## Method note

Node counts were used rather than wall-clock ratios for the branching factor, because three arms and
eight 4PC datagen lanes were running and a contended timing would have been a speed ratio between
arms doing different work — the failure this repo has recorded repeatedly. Node counts are
deterministic and contention-free. The eval share is from `node_profile`, re-run today rather than
quoted from its header.

## 4. CONFIRMED on a second, independent measurement — and the blocker is now FIXED

The branching factor above came from `search_bench`, which sums over a position set. Re-measured by
the engine itself on ONE position, which is what a `go` actually pays for:

```text
  depth        2      3       4         5          6          7
  startpos    176   2,352   12,469     81,421     410,798   3,875,126
  midgame     352  10,309   72,977  1,234,802  12,696,968
```

Per-ply ratios: startpos 13.4 / 5.3 / 6.5 / 5.0 / 9.4, midgame 29.3 / 7.1 / 16.9 / 10.3. Geometric
mean **13.8 in the midgame** — *worse* than the 9.17 used above, so the quantization estimate of
~5 Elo is if anything generous. The conclusion is unchanged on two independent instruments.

**The blocker in §1 is now fixed** (`crates/engine`). `go movetime` / `wtime` / `btime` / `nodes`
are read and converted to a NODE budget; the depth is chosen from that budget by a measured table;
the cap acts only as a safety net. Verified:

```text
  go                    depth 4   12,469 nodes    <- bare `go` is byte-identical to the old
  go movetime 50        depth 3   10,309 nodes       fixed-depth behaviour, so every prior
  go movetime 200       depth 4   72,977 nodes       measurement still reproduces
  go movetime 5000      depth 5  1,234,802 nodes
  go movetime 20000     depth 6 12,696,968 nodes
```

Time is converted to NODES rather than checked against a clock inside the search, so a game stays
deterministic and reproducible from its seed (FITNESS 10) — a millisecond check would make the same
seed play differently under load, which on this box is not hypothetical.

**Two bugs found and fixed while building it, both of which would have silently corrupted results:**

1. **Raising depth and relying on the cap does not work.** Without iterative deepening, an abort
   inside the FIRST root child means no root move ever completed, so the score stays −INF and the
   move returned is whatever the shuffle put first. Measured: `go movetime 1000` reported
   `score cp -32000` and a random move. Depth must be chosen so the search FINISHES.
2. **The first calibration table was ~10× too large**, because it used `search_bench` figures
   (d4 = 121,562, summed over a position set) for what a single `go` costs (d4 = 12,469). That makes
   the engine pick a shallower depth than the clock affords — silently throwing time away, the exact
   opposite of what this change exists to fix. It is now calibrated on the MIDGAME position, since
   startpos and midgame differ by ~31× at depth 6 and the cheap case would overspend everywhere.

What this unlocks: eval optimisations can now be measured as Elo, the gate can be the fixed-TIME
gate MASTER_PLAN line 43 specifies, and the +6.3% nps shipped earlier today stops being worth zero.
It does not change the sizing in §2 — quantization is still ~5 Elo, and the plies are still in the
branching factor, still reachable only through P2.

## 5. THE MODEL CROSS-CHECKS, on numbers that were already measured

§2's "27.8 Elo per doubling of nps" is a MODEL — it assumes Elo is linear in log(nodes) and divides
89 Elo/ply by log2(9.17). That is an extrapolation, and this repo has had a measured 12% become a
wall-clock LOSS. So it is worth testing against data that already exists, which costs no games.

`elo_per_ply_RESULT.md` measured Elo at each depth directly. This file measured the node cost of each
depth directly. Dividing one by the other gives Elo-per-doubling with no model in between:

| step | Elo gain (measured) | node ratio (measured) | doublings | Elo per doubling |
|---|---|---|---|---|
| depth 2→3 | +102 | 13.36× | 3.74 | 27.3 |
| depth 3→4 | +85 | 5.30× | 2.41 | 35.3 |
| depth 4→5 | +79 | 6.53× | 2.71 | 29.2 |
| **pooled 2→5** | **+266** | — | **8.85** | **30.0** |

**Measured 30.0 against a modelled 27.8 — 8% apart**, from two independent sets of inputs. The model
holds, and the sizing tightens rather than moves:

| change | Elo (model, 27.8/doubling) | Elo (measured, 30.0/doubling) |
|---|---|---|
| int16 eval 2× faster | ~5 | **~6** |
| eval entirely FREE | ~12 | **~13** |

So the conclusion is not resting on an extrapolation. Quantization is worth about **6 Elo**, against
~235 for all of training and 89 for one ply. It is cheap and worth doing; it is not the lever.

The two derivations share `elo_per_ply_RESULT.md` as an input, so this is a consistency check on the
*node-cost* half rather than a fully independent replication. What would be fully independent is
`elo_vs_time.sh`, which measures Elo against the CLOCK directly — now possible for the first time,
because until today the engine ignored `go` parameters and every point on that curve would have
returned the same number.


## 6. CORRECTION 2026-09-10 23:0x — the leaf formula billed a shuffle a leaf never performs

§2 sizes every eval conclusion on `node_profile`'s `eval's share of a leaf: 25.4%`, from
`leaf = movegen + make/unmake + eval + shuffle = 972.2 ns`. **A leaf does not shuffle.**
`pipeline/src/search.rs::ab()` runs in this order: `nodes += 1`, `legal_moves()`, then the
`depth == 0` early return with the eval — and only *after* that return does it take the per-depth
buffer and shuffle. So a leaf pays movegen (which runs before the check, to detect mate/stalemate)
and eval, and never reaches the shuffle. The formula added it anyway.

| | leaf | eval share | ceiling on eval work |
|---|---|---|---|
| as recorded | 972.2 ns | 25.4% | 1.34× |
| corrected (no shuffle at a leaf) | 651.6 ns | **35.4%** | **1.54×** |

At the measured 30 Elo/doubling that moves a **completely free eval** from ~13 Elo to **~19**. The
direction of §2 is unchanged — eval is still not a ply, and the plies are still in the branching
factor — but the eval track was under-valued by about 50%.

### And the self-check that should have caught this could not fail

The `CROSS-CHECK` block was two `println!` lines asserting *"width 16 = 796145 nps = 1256
ns/node"* — a **hardcoded string**. That figure was true when written and is now 3.3× wrong (live
`search_bench` at width 16 reads ~2.6M nps), and being a string it could never notice. It printed
its own falsification condition — *"if the primitives do not roughly bracket that, the attribution
is INCOMPLETE"* — while supplying a constant that made the test pass forever.

It now measures. Running the real search on the same positions and net:

```text
  depth 4, same net: 423733 nodes in 0.159s = 2,663,824 nps = 375 ns/node
  primitives imply a LEAF of 645 ns -> 1.72x the measured node cost
  ** ATTRIBUTION INCOMPLETE ** primitives over-predict by more than 1.6x.
```

**It still fires after the fix, and that is the honest state.** The shuffle bug explained part of the
gap (2.41× → 1.72×); the rest is that isolated timings are an UPPER BOUND — each primitive carries
its own loop and timing overhead, and a node inside a real search pipelines and hits warm caches. So
the corrected 35.4% is better than the 25.4% it replaces and is **still not safe to quote an Elo
figure from**. Naming the gap is the requirement the tool's own comment sets, and it is now named
rather than papered over with a stale constant.


## 7. 2026-09-10 23:3x — the instrument passes its own self-check for the first time, and eval is now the lever

Two more faults in `node_profile`, both found by asking what the engine actually executes:

**(a) It timed a shuffle the engine does not use.** `sh` was measured with `rng % (i + 1)`, but
`search.rs::below()` has used Lemire multiply-shift since **e0d8774** ("remove the division from the
child shuffle — +6.3% nps, measured"). So the profile reported the modulo cost as the live one and
advertised the difference — **"91.6 ns/node available"** — as headroom. That saving was *already
banked*. It also fed the wrong shuffle into the interior-node cost, inflating it by the same amount.

**(b) The leaf model was stale again within the hour.** After `movegen_leaf_RESULT.md` a leaf calls
`has_legal_move()` and evals; it never builds a list and never shuffles. Billing it for a full
`legal_moves()` overstated the leaf by ~350 ns.

Corrected, the self-check finally passes:

```text
  interior (movegen + make/unmake + Lemire shuffle)   497.4 ns
  leaf     (has_legal_move + make/unmake + eval)      358.4 ns
  eval's share of a LEAF                              59.6%

  depth 4, same net: 423733 nodes in 0.097s = 4,355,743 nps = 230 ns/node
  primitives imply a LEAF of 358 ns -> 1.56x the measured node cost
  within 1.6x -- the three primitives account for the node, shares are usable.
```

It read **2.41×** this morning, **1.72×** after the shuffle-billing fix, and **1.56×** now. For the
first time the attribution is consistent with the engine it describes, so the shares below are
usable for sizing rather than merely for ranking.

### This inverts §2, and the movegen work is why

| | leaf | eval share | ceiling on eval work |
|---|---|---|---|
| as recorded | 972.2 ns | 25.4% | 1.34× |
| after the shuffle-billing fix | 651.6 ns | 35.0% | 1.54× |
| **after `has_legal_move`** | **358.4 ns** | **59.6%** | **2.48×** |

| change | Elo, as first sized | Elo now |
|---|---|---|
| int16 quantization | ~5–6 | **~15** |
| eval entirely free | ~12–13 | **~39** |

§2 concluded *"quantization is worth roughly 5 Elo … it is not the lever"*, and that was correct for
the engine of this morning, where eval sat behind 393 ns of movegen. **Removing the movegen made
eval the lever.** It was third in `throughput_RESULT.md`'s ranking; it is now 59.6% of a leaf, for
the plain reason that the work in front of it is gone.

The plies are still in the branching factor — 2.48× is 1.3 doublings against the 3.2 a ply needs —
so §3 stands unchanged. What has changed is that the eval track is now worth roughly **3× what it
was**, and it is the largest remaining item the profile can see.
