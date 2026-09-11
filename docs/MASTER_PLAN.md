# Existence — Master Plan

Project title: **ExistenceIsPain**. Engine name on rating lists: **Existence** (short form EIP).
The name is the thesis: the engine starts knowing nothing except that losing is bad —
its only initial signal is the outcome. Everything it knows was learned from that.

## Principle
Given: the rules of chess, machinery that contains no chess opinion, and one editable
seed — bare alpha-beta recursion, nothing on it. Learned: everything else — evaluation,
every method layered on the search (and the search itself if a better one exists), the
balance between them, and the concepts it uses. A parallel lineage starts from depth-one
so the seed row can be retired from Given if it is rediscovered. Explained: from its own computation only, in a vocabulary drawn
from the rules and from what it learned. No human chess knowledge enters at any
point. Humans may annotate what it discovered, in a separate layer, after the fact.

Test of the Given column: point the identical toolkit at Shogi with a Shogi movegen
and nothing changes but the movegen and the rules-defined state it exposes (including
pieces in hand). If any other row would have to change, that row contains a chess
opinion. The search grammar passes this test only in the weak sense: it contains no
chess, but it does carry a declared prior over which search algorithms are short.

## Given (declared, defended line by line)
Two kinds of rows, deliberately kept apart:
- **Knowledge rows** — what the engine knows about the game. Only the rules. Every
  other piece of chess knowledge is learned.
- **Objective rows** — what success means and how it is checked: outcome as signal,
  the STC->LTC gate, SPRT, the correctness oracle. Fixed by the experimenter and NOT
  learnable by design. A learner that can choose its own objective chooses the one it
  is already maximizing (Goodhart); the gate is the only thing keeping every other
  learned component honest. Tabula rasa concerns knowledge, not who defines success —
  AlphaZero did not get to decide that winning was good either. The STC/LTC choice is
  a declared judgment about what to MEASURE, never about how to play.

| Component | Why it is not a chess opinion |
|---|---|
| Rules: perft-verified bitboard movegen, terminal conditions, zero-sum outcome | It is the game |
| Search primitive grammar: expand child, evaluate leaf, store/probe hash, max/min/average/mix, iterate to budget, compare to bound, select-by-quantity, read learned table | Generic building blocks. NOTE: this row is the real Given column. Expressiveness is not neutrality — program length is a prior, and a grammar in which alpha-beta is a short program is biased toward alpha-beta (`compare to bound` partly pre-loads "bounding"). GRAMMAR.md must state the primitive count and the program length of alpha-beta, MCTS, and PN-search in it; those numbers ARE the prior and are declared, not hidden. The purity lineage's "rediscovered bounding" is discounted accordingly |
| Seed search program: BARE alpha-beta — recursion with a window, nothing else. No ordering, no hash reuse, no iterative deepening, no quiescence, no extensions, no reductions, no pruning | A 1958 idea, given as an EDITABLE, REPLACEABLE seed, not a fixed skeleton. Everything layered on it since (where nearly all its strength lives) must be discovered. A parallel purity lineage starts from depth-one instead; if it reaches alpha-beta on its own, this row moves to Learned |
| Evolutionary search over programs in that grammar | Generic optimizer for discrete programs |
| Input: the game state as the rules define it — one binary plane per piece type per side (derived from the movegen's piece set), side to move, and any non-board state the rules carry (castling rights, en passant square; pieces in hand for games that have them) | Is the position; nothing more. No flip, no mirror, no king-relativity, no derived features. Stated as a rule, not a count, so the Shogi test survives |
| Gradient descent; SPSA (tables, continuous params); CMA-ES (small discrete structure choices only) | Generic optimizers. SPSA scales to thousands of parameters and matches the table shape; CMA-ES degrades past a few hundred |
| Objectives: game outcome; agreement with own deeper search | Self-referential; no external judgment. **MEASURED 2026-09-08: the second half was switched off and it is the one that works.** The loop trained on the outcome ALONE (blend = 0), justified by a comment that was correct at iteration zero — mixing the net's own root score is self-referential *when the net is random* — and never revisited once it stopped being random. Against a trained champion, same data, 10 replicates: outcome-only scores 0.4805 [0.4722, 0.4887], i.e. training makes the champion WORSE; blending in its own search score at 0.75 scores 0.5258 [0.5123, 0.5393]. The plateau runs from 0.5 to 1.0, so once the search score is present the outcome adds nothing measurable. Training toward a SEARCH score is not self-reference — it is distillation of the search into the net, which is the mechanism this whole design rests on. |
| Gate: SPRT ladder at fixed TIME — STC filter, LTC confirmation | Three parts, three answers (FITNESS.md 7.2). FIXED: the acceptance semantics, alpha = beta = 0.05, and the TC within a phase — the first because an instrument calibrated by its subject measures nothing, the last for comparability (one Elo scale across the ledger). ENGINE-DECIDED: how much evidence to gather (SPRT stops on evidence; the bandit allocates). DATA-DERIVED: the bounds schedule, from the median of closed, unbiased post-acceptance gains, with a declared floor. The ladder itself prevents optimizing for one clock. Fixed-cost-budget SPRT as a cheap third check for eval-only changes |
| Bandit allocating compute across learning tracks | Methodology |
| Menus (activations, optimizers, net-size steps) the searches choose from | Declared option sets, not choices |
| Template vocabulary of rules words (piece names, squares, captures, checks) | Rendering only; carries no judgment |
| Test methodology, seeds, hardware | Declared |

## Iteration zero
- Random-init net. No piece values exist anywhere in the system.
- Search program = bare alpha-beta: negamax recursion with an alpha-beta window to a
  fixed depth, children in emission order (shuffled). No hash reuse, no iterative
  deepening, no quiescence, no ordering, no extensions, no reductions, no pruning. All
  of these must be DISCOVERED as program edits that beat the current program on the
  clock. The bounding rule and backup rule are themselves mutable.
- Parallel purity lineage: identical, but seeded with "evaluate each legal move's
  resulting position; play the max" (depth one). Low bandit floor. Its ledger records
  the timestamps at which lookahead, minimax, and bounding appear. If it reaches
  alpha-beta, the seed row leaves the Given column.
- No search tables exist yet; a table exists only once an evolved program reads one.
- Move ordering is irrelevant at depth one; once ordering matters, it starts shuffled.
- No online statistics exist.
- Draw value, mate-distance penalty, eval->WDL scale: unset, to be fit.
- Behavior: finds mates inside its horizon (terminal conditions are rules); wanders otherwise.
- Known cost: bare alpha-beta with a random eval and shuffled ordering is bad chess
  for a while, and every eval is measured mid-capture. Early iterations are noisy.
  Expected and accepted. Expected discovery order for the search program: hash reuse
  -> iterative deepening -> hash-move-first ordering -> capture extension at the
  horizon (qsearch) -> table-driven ordering statistics -> reductions -> pruning.

## Learned
**Eval** — the net, from outcomes and its own search scores. Feature set grown from
piece x square via a grammar of rules-derived predicates (attacked-by, defends,
same-line-as, adjacent, ...). Width, depth, activation, output bucketing,
king-relativity, symmetry: discovered or not.

**Search program** — the search algorithm itself, as a program in the primitive
grammar, evolved with fixed-time SPRT as fitness. Compiled, so no per-step cost
beyond the eval and the primitives themselves. Within it, every decision that reads a table (reductions, pruning,
extensions, null-move, static cutoffs, probcut, multicut, IIR, aspiration, hash
replacement, depth step, time allocation) is an integer lookup whose contents SPSA
tunes and whose index features are learned. Evolution shapes the program; SPSA fills
its tables.

**Online statistics** — none at start. A grammar (index keys x update events x decay)
proposes counters. History, killers, countermove, continuation history are points in
it; so are counters nobody has tried.

**Constants** — draw score, mate-distance penalty, WDL scale, every datagen and
training hyperparameter, resign/adjudication thresholds, SPSA step/perturbation schedule.

**The dial** — how much to trust the eval vs. how deep to search. Never set. Found by
the gate: net-growth candidates and search-less candidates compete on the clock, and
the engine drifts to whichever is buying Elo. It moves along that curve as it strengthens.

## Piece values and all other concepts: emergent, never stated
The net never contains "queen = 9". It learns that positions with more of a piece
shape end in wins. Effective values are READ OUT post hoc via ablation (mean eval delta
from removing a piece type over many positions). Watching that number go from noise to
~9 in the first iterations is the first concept-discovery result. Same for every
concept after it.

## The loop — one loop, no schedule
```
datagen (current net + tables)  -> outcomes + root search scores
  -> train net                  -> candidate
  -> evolve search programs     -> candidates
  -> SPSA over tables inside the current program -> candidates
  -> proposers: features, stats, architecture, hyperparams -> candidates
  -> correctness oracle (search-program candidates only): reference-search agreement
  -> bandit picks which surviving candidates get gate time
  -> STC SPRT vs. champion (filter)
  -> LTC SPRT vs. champion (confirm; non-regression is the bar)
  -> accept -> new champion, ledger entry
```
Every track runs from iteration zero. Nothing has a human start date. A track
contributes the moment its candidates start passing. While the eval is noise, only the
eval passes — that is the system deciding the order. The gate decides; the surrogate
only proposes (blocks "prune everything" degenerate solutions).

## Making the search track robust (methodology, declared)
0. **Correctness oracle before any candidate may gate.** Fitness alone (fixed-time SPRT
   + mates-per-cost) rewards bugs that happen to win: a program that mishandles a hash
   collision or a fail-soft bound can return wrong scores and still gain Elo, and
   evolution WILL find and exploit such bugs because they are free Elo (AutoML-Zero hit
   exactly this). Every candidate program must first agree with a reference full-width
   search to fixed depth on a fixed position set — same best move and same score within
   tolerance where the candidate claims exactness. Programs are allowed to search LESS
   (pruning) but not to return scores inconsistent with what they claim to have searched.
   Cheap; without it the search track's headline result is unfalsifiable.
1. **Eval-independent fitness from day one — and the structural counterweight to
   recklessness.** Checkmate is a rule. Mine own games (retrograde from game endings)
   for mate-in-N positions; add "mates found per node" to search-program fitness
   alongside fixed-time SPRT. Deeper/tighter programs pay off on mate-finding
   immediately, before the eval knows anything. More importantly: the characteristic
   failure of UNSOUND pruning is missing a forced mate, so this objective punishes the
   "prune everything" degenerate solution directly. It is a rules-derived counterweight
   to fixed-time fitness rewarding recklessness, not a hand-designed penalty. This is
   the strongest answer to the objection that clock-based fitness selects for
   reckless search.
2. **Offline ladder check before compute is spent.** Verify alpha-beta, hash reuse, ID,
   and qsearch are expressible in the grammar and that a path of single mutations from
   the seed exists where every step is fitter. Use the author's existing net as the test
   eval for this rig only (test methodology, never training input). Fix grammar/fitness
   if a step is not a gain.
3. **Population, not a single champion.** Several lineages, different mutation rates,
   keep the best few alive. Prevents dead-basin lock-in.
4. **Compute floor via the bandit** so evolution cannot be starved by the faster-looking
   eval track.
5. **Declared fallback.** If, after grammar/fitness fixes, no program improves on the seed
   by eval ~1800, freeze the search at bare alpha-beta plus hand-tuned hash/ID and
   record in the ledger that those rows moved to Given. The claim shrinks honestly.

## Openings and draw death — engine-derived only
Self-play from the start position drifts toward draws as strength rises (>70% around
2800-3000 at slow TC; mostly draws by 3300+). Draws label every position ~0 and starve
the SPRT gate of information. No human-curated book is allowed. Three legal sources:

1. **Random opening plies** — count is a learned hyperparameter. Effective early;
   weakens as the engine strengthens (random moves yield trivial positions).
2. **Self-generated unbalanced book** — mine own games for positions where own search
   eval sits in a band (e.g. +0.6 to +1.5); start games there, each played from both
   sides. Band edges are learned hyperparameters. Becomes the main source as the measured
   draw rate rises (the bandit allocates the mix; no rating threshold). This is
   what TCEC's book does, with the judgment coming from the engine.
3. **Chess960 start positions** — a rules-defined family; carries no opinion. Adds
   diversity, prevents opening memorization, improves eval generalization, and yields
   the net wanted for 960 prep.

Gating in a draw-heavy regime uses pentanomial (game-pair) statistics on the
unbalanced book: the question becomes "did you win more pairs", as on fishtest.
The mix across the three sources is itself allocated by the bandit.

4PC note: draws are rare — measured ~1 in 200 (0.34-0.79% across four gates) — so
this section largely does not apply to the 14x14 target.

## Explanation layer — engine-only evidence
Emitted per move from the search and the net. No external input.
1. **Plan** — PV rendered as piece trajectories; critical moments flagged where the
   eval spread between candidates is largest.
2. **Contrast** — for each top alternative: its refutation line and eval drop.
3. **Attribution** — ablation deltas per piece/square; which learned features and
   statistics fired and how much each contributed.
4. **Mode** — decided by depth (tactic) or by eval (judgment), read off the search.
5. **Confidence** — engine-derived, two signals: (a) PV stability across depths — a
   plan that changes every ply is not a plan; (b) static-eval vs. deep-search residual
   for this position class. Early in training both read "no idea" and the renderer
   says so. Later they separate "sure judgment" / "calculated tactic" / "unclear".
6. **Render** — deterministic template over the rules vocabulary plus learned-feature
   identifiers (engine's own ids, e.g. F137). Every sentence maps to a number in 1-5.
   An optional prose renderer may paraphrase ONLY this evidence and may not add claims.

Explanations are honest at every stage: at iteration zero they faithfully report
meaningless plans and random attributions, flagged low-confidence. They become
trustworthy on exactly the schedule the play does, and say where they are on it.

Never: a commentary head trained on human annotations. Fluent, disconnected, misleading.

## Self-documentation — the ledger is written by the engine
Every accepted change is entered in the ledger BY THE SYSTEM at acceptance time, from
evidence it already holds. Humans annotate; they do not author.
Each entry contains:
1. **What changed** — program diff, new/changed table, added feature or statistic,
   architecture step, hyperparameter change; in the engine's own identifiers.
2. **What it earned** — e1 at acceptance (the bar it provably cleared; FITNESS.md 9),
   with the gate EVIDENCE: SPRT LLR, pairs, mates-per-cost delta, adversary win rate
   before/after. Not an SPRT point estimate (biased) and not a per-acceptance match
   (unmeasurable at useful precision).
3. **Where it mattered** — old and new champion run on a fixed position set; the
   positions where they DISAGREE most, with both PVs and both evals. A contrast
   explanation for a version, using the same machinery as the per-move contrast.
4. **What it means, in its own terms** — for a feature/statistic: the predicates it is
   built from and its top-firing exemplars; for a search edit: the mates it now finds,
   the nodes it now skips, the positions where its move changed.
5. **Confidence** — pairs, error bars, and the explanation-layer confidence signals on
   the disagreement positions.
Rendered through the same template as per-move explanations. Result: a development
log from "iteration 0: random eval, bare alpha-beta" onward, with evidence at every
step and no human narration.

**Versioning — the engine names its own versions.** Every gate acceptance is a new
champion; a "version" is an event, not a schedule (many per day early; roughly weekly by
P4). The scheme is fixed here so that champion 1 is never named by hand:
- `Existence <N>.<h>` — N is the champion counter (its age); h is a short hash of the full
  ledger up to and including this acceptance (a fingerprint of everything it has learned).
- A pronounceable name derived deterministically from h (proquint-style encoding of 32
  bits into two syllable-words, e.g. `kidop-sinub`). No vocabulary, no human choice.
- A subtitle that is the engine's own identifier for the accepted change and its gate
  result, e.g. `program P0031 (cleared e1=2.0 at LTC)`, `feature F137 (cleared e1=1.4)`, `network 384->512 (cleared e1=2.0)`. Never a per-acceptance Elo: that quantity is unmeasurable at useful precision (FITNESS.md 7.2/9) and the identity string is the most public field in the system.
Full form: `Existence 412 kidop-sinub — program P0031 (cleared e1=2.0 at LTC)`. Every part is
derived from the ledger; none is chosen by a person. Human-meaningful names for
discovered concepts belong to the annotation layer only.
- **Releases** for external play are tagged milestone champions (first hash reuse,
  first reduction, each +100 Elo, etc.), the way Stockfish tags a release while master
  moves daily. A release carries its champion identity unchanged. The ledger IS the write-up; every claim in any paper
points at an entry, every entry points at a gate result and a position set.

## Concept layer — post hoc, never touches training
1. **Ledger mining** — every accepted feature/statistic with its Elo and discovery
   time. The record of what it learned, in order.
2. **Probes** — linear probes on hidden activations for known concepts (material,
   mobility, king safety, ...) to see what is in the net beyond the explicit features.
3. **Excavation** — sparse dictionary / NMF on activations; keep directions predictive
   of deep value that known probes do not explain.
4. **Exemplars** — top-firing positions per candidate concept. Look at them.
5. **Teachability** — show a strong human the exemplars and the engine's move; test on
   held-out positions. If accuracy rises, the concept is real and transferable.
Humans may NAME concepts here. Names are annotations on engine-discovered structure,
marked as such, never inputs.

## Adversarial robustness — standing track and honesty check
KataGo was beaten by a far weaker adversary exploiting cyclic blind spots that
self-play never visits. A system whose selling point is explaining itself is especially
exposed: it will confidently explain a blind spot. Therefore:
1. **Adversary track** — a cheap agent whose reward is finding positions where the
   engine at a low cost budget disagrees with itself at a high one, or where a weaker opponent
   scores against it. Its positions feed the replay buffer with extra weight.
2. **Explanation honesty check** — every accepted net is evaluated on the adversary's
   positions: explanation confidence (PV stability, residual) MUST be low where the
   engine is wrong. A confident explanation on an adversarial position is a
   regression and blocks acceptance.
3. Ledger records adversary win rate per champion; it should fall over time.

## Phases
- **P0** (wk 1-2) Rust skeleton. Board parameterized by size and player count.
  Movegen validated against the C++ movegens TWO ways: (a) perft on the standard
  fixture suite, AND (b) random full games walked to terminal, comparing legal-move
  sets ply by ply. Perft alone is insufficient — measured: 40/40 perft agreement while
  94 genuine rules divergences existed, because fixture positions never reach those
  states (repetition, 50-move, odd promotions, Teams-specific terminal cases). Both
  checks run in CI. Identity engine plays. Gate harness runs.
- **P1** (wk 2-8) Eval discovers chess. Milestone ~2000 vs. SF-limited.
  Kill: no iteration-over-iteration gain across iterations 4-8 AND the static-vs-deep
  residual is not shrinking -> pipeline bug; stop and find it. (Early iterations are
  noisy by design; do not fire the kill on iterations 0-3.)

  > **Correction, 2026-09-11 — THIS KILL COULD NEVER HAVE FIRED, because its second conjunct is not
  > a measurable quantity as written.** The residual was implemented for the first time on that date
  > (`crates/pipeline/examples/static_deep_residual.rs`, `static_deep_residual_RESULT.md`) and the
  > measurement refuted the metric, not the engine. Two mechanical defects:
  >
  > 1. **The two sides are the same function.** The search evaluates leaves with the net under test
  >    (`datagen.rs:187`), so static and deep are one function at two depths. An **untrained random
  >    net scores corr 0.900** — the agreement is mechanical and carries no information.
  > 2. **The residual scales with the net's output range**, and learning to distinguish positions
  >    necessarily widens that range. The direction that indicates learning is the direction that
  >    makes the number grow, and the untrained net wins the trainer's own tanh metric outright
  >    (0.0181 against 0.167–0.260).
  >
  > A conjunctive kill with one unmeasurable conjunct never fires — which is exactly what happened:
  > the first conjunct was measured repeatedly through the 2026-09-10/11 plateau while the
  > conjunction stayed un-evaluable. **The plateau was ultimately explained without it** — the
  > learning rate, a hardcoded literal that had never been varied (`lr_sweep_RESULT.md`,
  > `lr_decay_RESULT.md`) — so no pipeline bug was hiding behind the dead conjunct.
  >
  > **Replacement, implemented and validated rather than proposed:** score the deep side with ONE
  > FROZEN REFERENCE net for every checkpoint (`static_deep_residual --ref`). The target then stops
  > moving with the net under test and both defects vanish — the untrained control falls from corr
  > 0.900 to **−0.019**, sign agreement 47.8%, chance. It reports a correlation rather than a
  > residual because the two sides are in different nets' units and subtracting them would be a unit
  > mismatch. Validated by predicting the complete ordering of a three-arm learning-rate sweep, in a
  > pre-registration committed before the matches reported.
  >
  > **Known limitation, recorded because it would otherwise mislead:** the replacement RANKS but
  > does not CALIBRATE. It ordered those arms correctly while also reporting "no arm beat its own
  > start" about a run in which two arms beat it decisively. Use it to compare candidates, never to
  > decide whether one has improved.
- **P2** (open-ended, runs from day one) Search learning from the bare alpha-beta
  seed. Milestones, each a timestamped ledger entry: hash reuse; iterative deepening;
  hash-move-first ordering; capture extension (qsearch); first ordering statistic;
  first reduction; first pruning rule; any edit to the bounding or backup rule that
  wins on the clock. Purity lineage milestones recorded separately: lookahead,
  minimax, bounding. Kill: no program improves on the seed by eval ~1800 -> grammar or
  fitness is wrong; fix those; invoke the declared fallback only after that.
- **P3** (mo 3-8) Structure learning: stat grammar, feature grammar, architecture,
  index features. Milestone ~3000. Explanation layer live. Concept layer starts —
  watch material and king safety appear. Kill: a track with no passing candidate in
  4 weeks gets its bandit weight floored, not deleted.
- **P4** (mo 6-18) Scale: bigger net, more data, OpenBench, longer LTC.
  Milestone 3300+; prep-useful and differently-opinionated. Concept excavation in
  earnest; teachability trials.
- **P5** (later, GPU) Learned neural search controller (MCTSnets-style) as a candidate
  program family, only if the evolved-program track shows the search space has
  structure worth a per-step net call. Honest write-up either way.
- **P6** 4PC target switched on once P2 is positive. An existing external C++ 4PC engine
  serves as the strength ruler and as the perft/legal-move oracle (referenced by path,
  never vendored; see CRATE.md). Concept layer on 4PC = writing the theory of a game
  that has none.

## Specification documents
All written and audited (see each file's self-audit and correction logs):
1. **GRAMMAR.md** — the real Given column for search; primitives, seeds, declared prior,
   mutation operators, cost model, ladder check.
2. **FITNESS.md** — the acceptance pipeline: oracle, exactness taint, mates-per-cost,
   surrogates, SPRT ladder with derived bounds, global anchor, bandit, degenerate-solution map.
3. **CRATE.md** — workspace and crate layout; Given crates vs pipeline crates; where each
   measurement (interpreter benchmark, preflight) lives.
4. **SCHEMAS.md** — position, program, table, net, feature, statistic, champion bundle,
   ledger entry, explanation record, activation dump.
Open measurements (not decisions): parser node counts (GRAMMAR 6), interpreter NPS ratio
(GRAMMAR 8 / CRATE 4), candle GPU throughput (CRATE 7). All are P0/P1 work with kill
conditions written.

## Architecture
**Language: Rust, for the engine AND the pipeline.** Chosen for long-term stability:
the datagen workers, evolution driver, SPSA driver, gate runner, and ledger writer run
unattended for months, and memory-safety and data-race bugs are the class of failure
that costs weeks silently. Rust removes most of that class. Performance parity with
C++ is established (Reckless, Viridithas). SIMD for NNUE inference via std::arch.
No C or C++ in the repo; the C++ movegens are external oracles used only for perft
diffing.

One crate, two board instantiations (8x8 two-player; 14x14 four-player Teams).
Engine small and hot; pipeline large and boring. Ledger from day one. Concept and
explanation ANALYSIS (probes, excavation, notebooks) may use Python against dumped
activations and search traces — analysis only, nothing on the training or gating path.

## Safeguards (methodology, declared)
**Throughput invariant, asserted every run:** GPU actually in use (nvidia-smi
utilization above a floor), positions/sec above a per-config floor, worker heartbeat,
interpreter/dependency check (e.g. cffi, CUDA libs) before the first game. Any
failure aborts the run loudly. Silent CPU fallback has already cost a 17x loss for 25
minutes with zero errors in the log; a multi-month fixed-time-gated plan cannot assume
the infrastructure — it must check it.

**External games never enter training or acceptance.** Games against outside opponents
(Lichess, rating lists, matches against other engines) are a PUBLIC READOUT only. The
outcome label is rules-derived and would be clean, but the POSITIONS are shaped by the
opponent's choices, so training on them imports human opening theory and strategy through
the state distribution without a single human-labelled move — and "learned everything from
the rules" stops being true. This is the contamination route that gets taken out of
convenience later ("we already have these games"), not by decision, which is why it is
declared now, before any such games exist. Note this is also what Leela Chess Zero does:
it trains on distributed SELF-PLAY and does not train on its Lichess games.
Exactly one use is permitted, and it is not a fitness signal: positions where a WEAKER
opponent scored against the champion may feed the ADVERSARIAL set (FITNESS 8) for the
explanation-honesty check. That set never influences acceptance — it tests whether the
engine is CONFIDENTLY wrong where it is wrong, and it reaches blind spots self-play
cannot, since self-play never plays the pathological lines a random outside bot will.

Fixed-time gating as an STC->LTC ladder from iteration zero (scaled to hardware; the
ratio matters more than the absolute values; grow both with compute). A candidate must
pass STC and then not regress at LTC. Search-program candidates always run the full
ladder. SPRT decides, surrogate proposes. One change per candidate; net
frozen while tables test and vice versa. Bandit floors. Declared STC/LTC values and ratio. Every
explanation sentence traceable to a number. Concept names marked as annotation.

## Expected rediscovery order
Search (main lineage, from bare alpha-beta): hash reuse -> iterative deepening ->
hash-move-first -> capture extension (qsearch) -> history/killers -> LMR-shaped
reductions -> null move -> futility-style margins.
Search (purity lineage, from depth-one): lookahead -> minimax -> alpha-beta bounding
-> then as above.

**MEASURED CONSTRAINT ON THE EARLY REGIME (2026-09-07). The order above is not wrong; it
is unreachable at shallow depth.** Cost of each program relative to the bare alpha-beta seed,
on the GRAMMAR 9 mate-in-1 set (`examples/ladder.rs`):

| program | D=2 | D=3 | D=4 |
|---|---|---|---|
| hash reuse | 1.250x | 1.218x | **1.113x** |
| iterative deepening | 1.084x | 1.098x | 1.086x |
| hash reuse + ID | 1.357x | 1.341x | 1.218x |

Hash reuse is a LOSS at every depth tested, but the loss SHRINKS with depth and the shrinking
accelerates (-0.032 from D2 to D3, then -0.105 from D3 to D4). A transposition table needs
transpositions, and a shallow search barely has any. Two independent estimates put break-even
near **depth 5-6**: extrapolating this trend, and separately, `tt_pressure.rs` measures an 11.3%
eval saving at depth 4 against a ~22% cost overhead, so the table must eliminate >18% of nodes
to pay.

**The consequence for the bootstrap, which this plan did not previously state:** evolution
running at depth 2 cannot discover hash reuse, because at depth 2 hash reuse is a 25% loss and
the fitness function will correctly reject it. The first predicted milestone is only reachable
once the engine already searches deep enough for it to pay. That is a chicken-and-egg in the
search track and it should be handled deliberately -- by running the PROGRAM arm's fitness at a
deeper fixed budget than datagen uses, or by accepting that the early ladder rungs arrive out
of the predicted order.

(Correction log: an earlier revision of this line SWAPPED hash reuse and iterative deepening,
reasoning that ID creates the repeated searches a table needs. Measured and REFUTED: ID's cost
is flat at ~1.086x across D2-D4, and hash+ID is worse than hash alone at every depth. The swap
has been reverted. What is true is the depth constraint above, not a reordering.)

Eval: material -> king safety -> mobility, structure -> king-relative / threat-like
features. Then its own point on the eval/search dial. Anything outside this list is
the headline.

## What would be new
Anything in the learned tables or grammars with no counterpart in Stockfish. Any
statistic keyed on something humans did not try. A dial position that is not SF's.
Concepts without names. And the method: a search learned jointly, which SF's
development model structurally cannot do.

## Honest odds
- Eval half: near certain.
- Search half, split three ways:
  - beating the identity-table baseline: near certain;
  - rediscovering SF-shaped tables from zero (the purity result): likely;
  - BEATING mature hand-tuned tables: hard. Mature 2-player engines report
    first-move-cutoff ~90%+ and low LMR re-search rates; the author's C++ 4PC engine
    independently measures 89.5% / 2.3% in a different game. Hand-tuned tables sit
    close to the ordering/pruning ceiling. "Rediscovered" and "beat" are different
    claims; do not conflate them in the write-up. (The 4PC figure is corroboration,
    not a 2-player measurement.)
- Search program: discovering hash reuse, ID, qsearch, ordering, reductions, pruning
  from the bare seed: likely (each is a short edit with a clear clock gain; mate-finding
  gives a gradient from day one). Purity lineage reaching alpha-beta from depth-one:
  likely, eventually. Escaping the alpha-beta basin to something better on the clock:
  low — alpha-beta with a hash is close to optimal for minimax with a strong eval, which
  is why every engine converged on it. A clean negative here is itself a result.
  Precedents for the method: AutoML-Zero (evolved gradient descent from primitives),
  AlphaDev (found faster sorting than libc). Neither is game search; no direct proof exists.
- Novel structure (a program, table, statistic, or feature with no SF counterpart): possible.
- Explanation layer: certain (engineering).
- Concept discovery finding something unnamed: plausible (AlphaZero did).
- Superhuman (~2800+): 6-12 months is the LEAST reliable estimate here. It is
  defensible for a from-zero alpha-beta engine that starts with hand heuristics; with
  none at iteration zero, on one consumer GPU, it is untested. Treat as conditional.
- Stockfish-distance: multi-year tail.
- Publishable regardless of strength.

## For the author
Prep: a strong engine that disagrees with SF for reasons it can show, validated
against SF. FM: the teachability step with the author as subject. 4PC: the first
written theory of the game, from the engine's activations, tested on the rank-1 human.

## Experiment log — P1 bootstrap

**2026-09-07, first learning loop. NEGATIVE. Training degrades generalisation.**

Built: self-play datagen, hand-written backprop trainer, net-vs-net gate, and the loop.

Three measurements, in order:

1. *The game-gate has no resolution at iteration zero.* Two randomly-initialised nets draw
   **86-100%** of their games at every random-opening depth tried (4/8/12/16/20 plies). A
   60-game match therefore carries a +/-0.13 interval and cannot resolve any realistic
   improvement. This is not a gate bug: MASTER_PLAN "Iteration zero" predicts wandering play,
   and two wanderers draw. A game-based acceptance test only becomes usable once play is
   decisive, so the bootstrap phase needs the held-out surrogate (FITNESS 5) instead.

2. *Self-play labels are sparse.* Only **13-17%** of recorded positions come from a decided
   game (30% of games decisive at depth 2, 37% at depth 4). The rest are labelled 0.

3. *Training makes held-out correlation WORSE.* On 72,199 training positions (2,522 decided)
   and 23,769 held-out (619 decided): correlation between eval and outcome went
   **+0.354 -> +0.237**, delta-z **-0.128 +/- 0.112**. Train loss fell 0.035 -> 0.006
   monotonically while held-out correlation collapsed and thrashed. That shape is
   memorisation: 782x128 = ~100k parameters against ~2.5k informative labels is ~40
   parameters per label.

**Two traps recorded, both hit:**
- `blend` mixing the net's own root score into its target is SELF-REFERENTIAL at iteration
  zero: it trains the net toward what it already says. Default is now 0; the blend has to
  earn its place with a measurement once the search score beats the raw outcome.
- The first verdict compared correlation against an ABSOLUTE threshold (`> 0.15`) and printed
  "LEARNING" on a null result. The random net's own baseline is **+0.354** -- a fixed function
  correlates with outcomes because losing sides tend to have fewer pieces. Any claim must beat
  the baseline, not zero, and carry an interval.

**THE TRAINER WAS BROKEN, and the three nulls above were measuring a broken optimiser.**
Found by the control that should have been run first: can it fit a target that is LINEAR IN
ITS OWN INPUTS (material, computed from the same planes the net sees)? It could not --
train loss fell 30x while held-out correlation went 0.118 -> 0.065.

Cause: a POV frame mismatch. `Net::eval` computes a WHITE-POV value and applies the mover
flip only at the very end, so the network's raw output is white-POV. The trainer was training
that raw output toward a MOVER-relative target, i.e. demanding the same output be +m and -m
for the same material. Contradictory, and the cheapest solution is to predict the mean --
which is exactly the collapse that was measured and misread three times as "capacity",
"label sparsity" and "self-referential blend".

After the fix the control passes: correlation **0.118 -> 0.905** on material.

*Retracted:* the capacity sweep, the decided-only comparison and the first blend result. All
three were run against the broken optimiser and carry no information.

*Still not demonstrated:* learning from self-play OUTCOMES. With the fixed trainer,
+0.282 -> +0.322, delta-z +0.044 +/- 0.118 on 552 decided held-out positions -- inside the
noise. The optimiser is no longer a suspect; the remaining candidates are the size of the
held-out set and the ~85% zero labels.

**Method note, the expensive lesson:** run the trivially-learnable-target control BEFORE
attributing a null to the data or the architecture. Three hypotheses and roughly an hour were
spent explaining a result produced by a sign error. The same control settled the equivalent
question on the sibling 4PC project in one run.

**2026-09-07, P1 finding: the outcome label is only informative NEAR the terminal.**

With the trainer fixed, a paired before/after test on held-out decided positions, sweeping how
far from the end a training position may be (distance to terminal is rules-derived, so this
filter is legal):

| training set | n | sign accuracy before -> after |
|---|---|---|
| all decided | 2087 | 0.452 -> **0.441** (worse) |
| <= 60 plies from end | 1673 | 0.452 -> 0.536 |
| <= 30 plies from end | 1067 | 0.452 -> 0.521 |
| <= 10 plies from end | 414 | 0.452 -> **0.543** |

Training on ALL decided positions degrades the eval. Restricting to near-terminal positions
improves it, and the effect is strongest with 5x LESS data. Interpretation: when both players
are near-random, the game result is nearly independent of a position 40 plies earlier, so
distant labels are not weak signal -- they are anti-signal, and they dominate by count.

This is the bootstrap problem in its concrete form. The label only becomes informative further
back as play improves, which is why the horizon should widen with strength rather than being
fixed. That schedule is a hyperparameter, i.e. LEARNED, not declared.

**Metric note:** paired squared error is NEGATIVE in every arm even where sign accuracy
clearly improves, because an untrained net predicts ~0 -- safe under MSE (error ~1) and
useless in play -- while a trained net predicts with magnitude and is sometimes wrong. MSE
punishes confidence. Sign accuracy plus McNemar is the meaningful pair here; report both.

**2026-09-07, P1 status: loop closed end-to-end, learning NOT yet demonstrated.**

The loop runs: self-play -> near-terminal filter -> train -> acceptance -> control. Two bugs
were found by the control, both of the same family (measuring the thing you trained on):

- The first "hold-out" was the tail of the training list. It measured training-set fit,
  accepted 7 of 10 candidates, and the resulting champion scored **0.500** against the original
  random net. Fixed by splitting BEFORE training; acceptances fell to 4 of 10 and the control
  is unchanged at **0.497 +/- 0.077**. Keep the control-vs-origin match permanently: it is the
  only statistic in the loop that cannot be gamed by a bad hold-out.
- Acceptance at iteration zero cannot use the game-gate: two wandering nets draw 86-100%, so an
  80-game match carries +/-0.11 and rejects everything on merit-independent grounds. During
  bootstrap the held-out surrogate decides and the gate acts as a NON-REGRESSION guard; the
  gate takes over automatically once the draw rate falls below 60%.

**The binding constraint is data volume, not a bug.** 150 self-play games give ~21,000
positions, of which 6-19 games are decisive, and the near-terminal filter leaves **13-295
labels per generation**. A 25k-parameter net cannot learn from ~150 labels, and McNemar on ~40
held-out positions is noise -- which is why acceptances look random.

Ranked next steps: (a) orders of magnitude more self-play, which needs the search fast enough
that datagen is not the bottleneck; (b) raise the decisive fraction -- adjudication by a
rules-derived terminal condition, not by a material heuristic, which would be a Given row;
(c) only then revisit acceptance thresholds.

**2026-09-07, P1 MILESTONE: the engine learns from its own self-play. Replicated.**

Control = final champion vs the ORIGINAL random net it started from, 160 games per seed,
colours alternating, openings played from both sides.

| seed | result | rate |
|---|---|---|
| 20260907 | 46W-96D-18L | 0.588 +/- 0.076 |
| 11111 | 45W-104D-11L | 0.606 +/- 0.076 |
| 22222 | 55W-100D-5L | 0.656 +/- 0.074 |
| 33333 | 29W-126D-5L | 0.575 +/- 0.077 (lower bound 0.498) |
| **POOLED** | **175W-426D-39L / 640** | **0.6062 +/- 0.0379 -> [0.568, 0.644]** |

All four seeds positive; three individually significant; pooled clearly above 0.5. Draw rate
66.6%, so the effect is real but modest in game terms -- as expected when both sides still
wander.

**What unlocked it was data volume, and the lever was counter-intuitive: SEARCH SHALLOWER.**
Measured usable labels per second (decided AND within 30 plies of the terminal):

| datagen depth | games/s | decisive | usable labels/s |
|---|---|---|---|
| 1 | 702 | 25% | **4623** |
| 2 | 92 | 7% | 98 |
| 3 | 8.5 | 40% | 92 |

Depth 1 is 8x faster per game AND more decisive than depth 2, because at iteration zero a
deeper search is only maximising a random eval -- the extra plies buy nothing and cost
everything. Labels per generation went 13-295 -> 7,364-48,678, and the gate's draw rate fell
from ~95% to 61-82%, so the game-gate is starting to resolve on its own and will take over
from the surrogate as play sharpens.

No Elo figure is quoted: this is a score rate against a fixed opponent, not a shipped gate.

**2026-09-07, P1: the plateau is an ACCEPTANCE stall, and "search deeper" does not fix it.**

Learning is fast then flat: control vs the frozen origin reads 0.635 / 0.630 / 0.600 / 0.605
at generations 8/16/24/32. The acceptance trace explains it exactly:

    . A A A A A . . . . . . . . . . . . . . . . . . . . . . . .

Accepted at generations 2-6, then NOTHING for 24 generations. The champion freezes. (Found
because two arms with different horizons produced BYTE-IDENTICAL gate results -- impossible
unless the champion object was the same, i.e. nothing had been accepted in either.)

*Rejected hypothesis:* the widening horizon reintroducing far-from-terminal anti-signal.
Capping it at 30 changed nothing (0.680 +/- 0.065 vs 0.698 +/- 0.064).

*Structural diagnosis:* AlphaZero's engine of improvement is that SEARCH(net) > net, so the
data is always better than the thing that produced it. At datagen depth 1 the search is barely
stronger than the raw eval, so once the net fits its own play the data stops being better and
the ladder has no next rung.

*But the obvious fix FAILS.* Deepening after bootstrap (depth 1 for 6 generations, then depth
2) does produce more acceptances -- `.AA.AAAA....AAAA..A.A` vs `.AAAAAA.......A.A.AA.` -- yet
at EQUAL WALL TIME it is weaker:

| schedule | control vs origin |
|---|---|
| depth 1 throughout | **0.598 +/- 0.062** |
| depth 1 then depth 2 | 0.527 +/- 0.063 (not significant) |

The label-volume loss (~47x fewer usable labels/second at depth 2) outweighs the improvement
operator at this speed. **Acceptance rate is not a proxy for strength** -- arm B accepted more
often and was weaker. Any future schedule change must be judged on the control at equal wall
time, never on how often it accepts.

The real unlock is making deep search cheap enough that both hold at once: incremental
accumulator, then the bytecode. Until then depth 1 is the correct datagen setting.
