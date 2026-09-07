# Existence

**Project: ExistenceIsPain.** A chess engine given the rules and nothing else, that
learns its evaluation, its search, and the concepts it uses from its own games — and
then explains, in its own terms, what it learned and why it plays what it plays.

The name is the thesis. At iteration zero the engine knows exactly one thing about
chess: losing is bad. Its only initial signal is the outcome of games it plays with a
random evaluation and a bare search. Everything it knows was learned from that.

## What is given
- The rules of chess (perft-verified bitboard movegen, terminal conditions, zero-sum outcome).
- A grammar of search primitives with no algorithm spelled out — though its primitives
  set a prior over which algorithms are short, and that prior is declared — plus one
  editable seed:
  bare alpha-beta recursion — no ordering, no hash reuse, no iterative deepening, no
  quiescence, no extensions, no reductions, no pruning.
- The game state as the rules define it: one binary plane per piece type per side,
  derived from the movegen's piece set, plus side to move and any non-board state the
  rules carry (castling rights, en passant square; pieces in hand for games that have
  them). No flipping, no mirroring, no derived features.
- Generic optimizers (gradient descent, SPSA, evolutionary search over programs) and
  self-referential objectives (game outcome; agreement with its own deeper search).
- A gate (SPRT at fixed time), a compute allocator, and test methodology.

Test of the list: point the same toolkit at Shogi with a Shogi movegen and nothing
changes but the movegen and the rules-defined state it exposes — including pieces in
hand. If any other row would have to change, that row contains a chess opinion.

## What is learned
Everything else. The evaluation network and the features it sees. Every method layered
on the search — and the search itself if a better one exists. Every table, statistic,
constant, and schedule. The value of a draw. How much to trust the eval versus how
deep to search. Nothing has a human start date; every learning track runs from
iteration zero and contributes when it starts passing the gate.

A parallel lineage starts from depth-one lookahead instead of alpha-beta, so the one
seed can be retired from the given list if the engine rediscovers it.

## What it explains
Per move: its plan (PV as piece trajectories), why the natural alternatives fail
(refutation lines and eval drops), which pieces and learned features drove the eval,
whether the decision was calculation or judgment, and how confident it is — all from
its own search and network, never from human commentary.

Per version: the ledger is written by the engine at acceptance time — what changed,
what it earned, the positions where old and new champion disagree, and what the change
means in the engine's own identifiers. The ledger is the development history and the
write-up. Humans annotate; they do not author.

Versions name themselves. Every gate acceptance is a new champion, identified as
`Existence <N>.<hash>` (age and a fingerprint of everything learned so far), with a
pronounceable name derived deterministically from the hash and a subtitle that is the
engine's own identifier for the change and its gate result — e.g.
`Existence 412 kidop-sinub — program P0031 (cleared e1=2.0 at LTC)`. No version is ever named
by a person. Releases for external play are tagged milestone champions.

Post hoc: concept discovery over the learned features and hidden activations — what it
learned, in what order, and whether any of it has no human name yet.

## Language
Rust, engine and pipeline. Python for analysis notebooks only.

## Status
Specification complete and audited: [`MASTER_PLAN.md`](docs/MASTER_PLAN.md) (phases, kill
criteria, safeguards, odds), [`GRAMMAR.md`](docs/GRAMMAR.md) (the search grammar — the real
Given column, with its prior stated as a number), [`FITNESS.md`](docs/FITNESS.md) (what the
engine is rewarded for), [`CRATE.md`](docs/CRATE.md) (layout), [`SCHEMAS.md`](docs/SCHEMAS.md)
(data formats). No code yet. Next: P0 — Rust workspace, parameterized board, movegen
validated by perft AND random games to terminal, the interpreter benchmark (the first
measurement that can kill the plan), one gate end to end.

## License and release
**GPLv3** (see [`LICENSE`](LICENSE)). Copyright (C) 2026 the Existence authors. Open source from the
first commit, because the claim requires it: "learned everything from the rules" is only
checkable if the Given column can be read in the code, and the ledger is only a write-up
if the code that produced it is inspectable. Open source is also what OpenBench requires
for shared testing compute.

**Originality.** All code in this repository is written from scratch for this project.
No code from any chess engine is vendored, copied, or adapted; the C++ movegens and
Stockfish used as oracles live outside the repository and are referenced by path only.
The tabula rasa design requires this — borrowing another engine's search or evaluation
would break the thesis — so there is no engine code to attribute. Rust library
dependencies carry their own permissive licenses; their notices are collected in
`THIRD_PARTY_LICENSES` (generated by `cargo about`) as those licenses require.

**What is released and when.** Code and the ledger: with every commit. Champion bundles
(nets, tables, evolved programs): on a delay of one milestone release, so the strongest
version has a window of being only the author's. The ledger and the position sets its
entries reference are released with any paper, since they are what make its claims
checkable.
