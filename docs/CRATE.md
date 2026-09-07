# CRATE.md — Repository and Crate Layout

Rust throughout (MASTER_PLAN "Architecture"). Engine small and hot; pipeline large and
boring. One workspace, several crates, one board abstraction instantiated for 8x8
two-player and 14x14 four-player Teams.

## 0. Principles that decide the layout
1. **Everything the engine LEARNS is data, not code.** Nets, tables, programs, hyper-
   parameters, `score_of`, opening books, position sets: all files, all content-hashed,
   all referenced from the ledger. The binary never changes when the engine gets smarter.
2. **Everything the engine is GIVEN is code, and small.** Rules, grammar, interpreter,
   cost model, gate, bandit. Every crate in this list maps to a Given row; a crate with no
   Given row is pipeline plumbing.
3. **Two boards, one code path.** The board is a generic parameter, not a runtime flag, so
   the 8x8 build pays nothing for 14x14 and vice versa.
4. **The hot loop is one crate** (`engine`) with no dependency on the pipeline crates. It
   can be built, benchmarked, and shipped alone.
5. **Oracles are external and read-only.** The C++ movegens and Stockfish live outside the
   repo, referenced by path in config, used only by tests and the gate runner.

## 1. Workspace
```
existence/
  Cargo.toml                 # workspace
  README.md  LICENSE  docs/{MASTER_PLAN,GRAMMAR,FITNESS,CRATE,SCHEMAS}.md
  crates/
    board/                   # Given: rules
    grammar/                 # Given: primitive grammar, program AST, types, mutation
    interp/                  # Given: bytecode compiler + interpreter + cost accounting
    nnue/                    # Given: network inference skeleton (architecture is data)
    engine/                  # hot loop: board + interp + nnue + UCI; no pipeline deps
    oracle/                  # correctness oracle: reference full-width search, taint
    gate/                    # SPRT ladder, anchor matches, fastchess driver
    datagen/                 # self-play workers, books, position-set mining
    evolve/                  # program evolution: proposers, population, ladder check
    spsa/                    # table tuning
    trainer/                 # NNUE training (Rust; GPU via candle or tch bindings)
    proposers/               # feature grammar, stat grammar, architecture menu, hyper
    bandit/                  # allocation
    ledger/                  # records, hashing, versioning/naming, rendering
    explain/                 # explanation layer (P3): plan, contrast, attribution, confidence
    adversary/               # adversarial position mining (P3)
    xtask/                   # dev tasks: perft, random-game diff, benchmarks, CI
  data/                      # NOT in git; content-addressed store (see SCHEMAS.md)
  configs/                   # declared constants (TC, alpha/beta, cost table, hardware)
  analysis/                  # Python notebooks, concept layer (read-only consumers)
```

## 2. `board` — the rules
- `trait Rules` with associated consts `WIDTH`, `HEIGHT`, `PLAYERS`, `TEAMS` and
  associated types `Bitboard` (u64 for 8x8; a 256-bit type for 14x14 with corners masked),
  `Piece`, `State`.
- Implementations: `Chess` (8x8, 2 players, 2 teams), `Teams4` (14x14, 4 players, 2 teams,
  fixed R-B-Y-G rotation, Teams terminal rules).
- API used by everything else: `legal_moves`, `apply`, `terminal -> Outcome`, `key`,
  `planes()` (rules-defined input planes + side to move + rules-carried non-board state),
  `pred(move, PredId)`.
- **No evaluation, no ordering, no heuristics of any kind.** If a function in this crate
  would be different for a game with different rules only, it belongs here; if it would be
  different for a game with different *strategy*, it does not exist here.
- Tests: perft fixture suites for both boards; random games walked to terminal with
  ply-by-ply legal-move-set comparison against the external C++ oracles (both required,
  MASTER_PLAN P0). Both run in CI on every commit.

## 3. `grammar` — programs as data
- Typed AST per GRAMMAR.md Sections 1-3; `Program` = 1-4 typed functions; serializable
  (see SCHEMAS.md) and content-hashed.
- Type checker; size counter (this is what replaces GRAMMAR.md's hand-estimated node
  counts — the first thing built, and the prior in GRAMMAR 6 is re-derived from it).
- Mutation operators (GRAMMAR 4), all type-preserving, seeded RNG.
- Static exactness taint (FITNESS 2.3), computed per `ret` site, with the declared
  window-cutoff exemption as a pattern match.
- No execution here; `grammar` knows nothing about speed.

## 4. `interp` — the only place a program runs
- Compiles a `Program` to a register bytecode; interprets it with a running cost counter
  (per-primitive cost table from `configs/cost.toml`) that IS the budget (GRAMMAR 8).
- Generic over `Rules`. Calls `nnue` for `eval`. Owns the hash table and `score_of` table
  storage (both content-addressed data).
- Smoke ceiling: rejects programs that cannot complete `choose` on a fixed position within
  budget (GRAMMAR 8).
- **The benchmark lives here:** `xtask bench-interp` runs the compiled main seed against a
  hand-written Rust bare alpha-beta with the same net and reports NPS ratio. Acceptance
  >= 0.50 (GRAMMAR 8). This measurement is taken in P0 before anything else is built on
  the interpreter; failing it revisits the bytecode design, not the grammar.

## 5. `nnue` — inference skeleton
- Accumulator + layers, generic over input feature count and hidden sizes given at load
  time from the net file header. Architecture is data (ARCH candidates change the header,
  not the code). SIMD via `std::arch` with a scalar fallback used in tests.
- Feature computation for the learned feature set: `board.planes()` plus the accepted
  conjunctions from the feature grammar, described in the net file, computed
  incrementally where the conjunction's inputs change locally and rebuilt otherwise.
- Verification mode: incremental vs from-scratch parity on every eval (test builds only).

## 6. `engine` — the shipped binary
- `board` + `interp` + `nnue` + UCI (2-player) and the 4PC protocol (Teams). Loads a
  champion bundle (SCHEMAS.md) and plays. No pipeline dependency; `cargo build -p engine`
  produces the thing that goes on a rating list.
- Reports its own identity string from the bundle (ledger-derived name; MASTER_PLAN
  "Versioning"). Never a hand-set version string.

## 7. Pipeline crates
- `oracle`: frozen reference search (unit-tested against a brute-force enumerator),
  position-set management, pass/fail per FITNESS 2.
- `gate`: drives fastchess; STC/LTC/fixed-cost SPRT with derived bounds; global anchor
  matches; release matches; staleness rule; writes evidence to `ledger`.
- `datagen`: self-play workers on the champion; random-ply / book / 960 openings;
  mate-set retrograde mining; STANDARD/TACTICAL set refresh; held-out split. Emits
  positions in the SCHEMAS.md format.
- `evolve`: population of programs per lineage (MAIN, PURITY); proposers using `grammar`
  mutations; runs oracle + mates-per-cost filter; queues survivors to `gate`. Also hosts
  the offline ladder check (GRAMMAR 9) as an `xtask`.
  **Lineage semantics, stated once:** both lineages share the CHAMPION's net, tables,
  feature set, statistics, and `score_of`; they differ ONLY in the search program. The
  PURITY lineage's candidates are gated against the PURITY lineage's own current program
  (with the champion's data), not against the MAIN champion — its ledger is the record of
  what a depth-one seed reaches on its own. The day a PURITY program beats the MAIN
  champion's program at LTC (same data), it may be proposed to MAIN as an ordinary
  PROGRAM candidate; that event is what retires the seed row from Given.
- `spsa`: perturbs tables of the current program; agreement surrogate; queues to `gate`.
- `trainer`: reads positions, trains nets; GPU via candle (pure Rust) — tch/PyTorch
  bindings are the declared fallback if candle throughput is insufficient (measured in
  P1; declared either way).
- `proposers`: feature grammar (conjunctions of `PredId`s), stat grammar (key x event x
  decay), architecture menu, hyperparameter perturbation. Pure proposal; no judgement.
- `bandit`: Thompson sampling over arms with floors; reads acceptances and compute-hours
  from `ledger`; writes allocation decisions to `ledger` daily.
- `ledger`: append-only records (SCHEMAS.md), content hashing, champion counter, proquint
  naming, template renderer for entries and per-move explanations.
- `explain` (P3): plan/contrast/attribution/confidence from `engine` traces.
- `adversary` (P3): low-vs-high-budget disagreement mining; feeds `datagen` weights and
  FITNESS stage 6.

## 8. Throughput invariant (MASTER_PLAN Safeguards) — where it lives
`xtask preflight` runs before every pipeline start and every N hours: GPU visible and
above utilization floor once warm; positions/sec above the per-config floor; worker
heartbeat; dependency check (CUDA libs, candle backend); disk headroom. Any failure
aborts loudly and writes a ledger event. No pipeline crate may start without it.

## 9. Configuration — the declared constants, in one place
`configs/*.toml`, versioned in git, hashed into every ledger entry that used them:
`gate.toml` (STC, LTC, alpha, beta, width, floor, bootstrap e1, anchor window), `cost.toml`
(per-primitive costs), `hardware.toml` (threads, pinning, GPU), `phases.toml` (what
changes at each declared phase boundary). Changing a config is itself a ledger event.

## 10. Build, test, CI
- `cargo test --workspace` runs unit tests + perft suites + random-game diffs (needs the
  oracle paths; skipped with a loud warning if absent).
- `xtask bench-interp`, `xtask ladder-check`, `xtask preflight`.
- CI matrix: 2-player and 4PC boards; debug + release; `-D warnings`; `cargo clippy`.
- Release profile: `lto = "fat"`, `codegen-units = 1`, `target-cpu=native` for local
  builds; a portable build for rating lists.

## 11. Self-audit
- Does any crate in the Given set contain a chess opinion? `board`: rules only (2).
  `grammar`: GRAMMAR.md. `interp`: cost table is engineering, declared. `nnue`: skeleton;
  architecture is data. PASS.
- Can the pipeline change engine behaviour without a ledger event? Only via data files,
  all content-hashed and referenced by the champion bundle. PASS.
- Is the 14x14 board a first-class citizen or a retrofit? Generic parameter from day one;
  CI builds both. PASS.
- Known risks: (a) 256-bit bitboards for 14x14 have no native SIMD popcount path on all
  targets — measure; (b) candle GPU throughput unproven for this workload — fallback
  declared; (c) the interpreter benchmark is the single biggest schedule risk and is
  therefore first in P0.
