# FITNESS.md — Acceptance, Oracles, Surrogates, Ladder, Bandit

GRAMMAR.md decides what the engine CAN find. This file decides what it is REWARDED for.
Every rule here is an objective row (see MASTER_PLAN "Given": objective rows are fixed by
the experimenter and not learnable). Every number is declared with a reason and a
revisit trigger. Evolution exploits holes in fitness; this file's job is to have none it
can reach.

## 0. The acceptance decision in one line
A candidate becomes champion iff it passes, in order:
  (1) type-check + cost cap  ->  (2) correctness oracle (programs only)  ->
  (3) cheap surrogate filter  ->  (4) STC SPRT  ->  (5) LTC SPRT (non-regression)  ->
  (6) explanation-honesty check on adversarial positions.
Stages 1-3 cost seconds; 4-5 cost games; 6 costs a fixed evaluation. The bandit decides
which surviving candidates get stage 4-5 time. Nothing skips a stage.

## 1. Candidate classes
| Class | Produced by | Runs oracle? | Ladder | Extra |
|---|---|---|---|---|
| PROGRAM | evolution over the grammar | yes | full (STC, LTC; VLTC once declared) | mates-per-cost in surrogate |
| TABLE | SPSA over an existing program's tables | no (tables cannot break exactness; taint is static) | STC, LTC | agreement surrogate |
| NET | training iteration (same arch) | no | fixed-cost-budget gate, then STC, LTC | held-out loss surrogate |
| ARCH | architecture-menu step | no | fixed-cost-budget gate, then STC, LTC, VLTC once declared | held-out loss surrogate |
| FEATURE / STAT | grammar proposers | STAT: no; FEATURE: no | as TABLE | held-out loss (FEATURE), agreement (STAT) |
| HYPER | perturbation of a training/datagen hyperparameter | no | judged by the NET it produces | — |

**THE HYPER ARM HAS ITS FIRST MOVED HYPERPARAMETER — datagen DEPTH, 2026-09-10.** This row declared
the class and nothing had ever moved in it. `main.rs:152` set `--depth` default **1**, so every P1
measurement in this repo — the ~1216 plateau, the deceleration curve, the width/blend/horizon sweeps,
the 2,200-generation champion — was taken on a loop labelling its own positions with a ONE-PLY
search.

Measured at equal wall clock on the absolute ruler (`datagen_depth_RESULT.md`):

| datagen depth | generations in 1800s | Elo vs SF-1320 | absolute |
|---|---|---|---|
| 1 | 610 | −236 ± 53 | ~1084 |
| **3** | **6** | **−108 ± 48** | **~1212** |
| 6 | 0 — could not complete one generation | — | — |

**+128 ± 72, resolved.** Six deep generations are statistically indistinguishable from a champion
built from ~2,200 shallow ones. Judged exactly as this row specifies: *by the NET it produces*, on
the ruler, not by a gate.

The value is now **3**, and the budget is chosen by WALL CLOCK so generations actually complete —
depth 6 is not merely worse, it produced no net at all at 2,400 games per generation. The production
run (`deep1.log`, 400 generations, rung snapshot every 25) is the first HYPER-class change this
project has shipped, and `--datagen-nodes` expresses the same knob in nodes per move for when the
knee needs finding between 3 and 6.


## 2. Correctness oracle (PROGRAM candidates)
### 2.1 Reference search
Full-width negamax to fixed depth R, no pruning, no hash, no extensions, using the
CURRENT champion's evaluation and `score_of` table. R = 4 for the standard set; R = 6
for the tactical set (Section 2.2). Deterministic. Implemented once in Rust, unit-tested
against a brute-force enumerator, frozen; it is part of the test rig, not the engine.
### 2.2 Position sets
- STANDARD: 2,000 positions sampled uniformly from the last 10 champions' self-play,
  excluding positions within 2 plies of a terminal. Refreshed every 20 champions (so a
  program cannot overfit a static set); the previous set is kept as a regression check.
- TACTICAL: 500 positions where the reference at R=6 finds a forced WIN/LOSS outcome
  that the reference at R=2 does not. Refreshed with STANDARD.
- Both sets are drawn from the ENGINE's games only (rules-derived filtering only).
### 2.3 Exactness taint (fixes GRAMMAR 10.7 item 1)
Computed statically per `ret` site. A returned Score is EXACT iff along every path from
that `ret` back to the function's `moves(p)` iteration:
  a. the iteration is `foreach` over the FULL list (not `sort`-truncated, not `sample`),
  b. no `ret` occurs inside the loop before the list is exhausted — EXCEPT the
     window-cutoff pattern: an early `ret` is permitted iff it is guarded by
     `cmp(x, y, >=)` or `cmp(x, y, >)` where x and y are the function's two Score
     parameters (the window), every recursive call passes the window as
     `(neg(second), neg(first))`, and the root call passes `(neg(INF), INF)`. Under
     exactly these conditions the root value equals the full-width minimax value
     (alpha-beta's soundness theorem), so the seed and its window-preserving
     descendants are EXACT. Any other early exit is INEXACT. This exemption is
     pattern-matched statically and is a DECLARED special case: it encodes the
     soundness of one algorithm's cutoff, not a preference for it — a program that
     never uses it loses nothing but the strong check.
  c. the accumulated value passes only through `max`/`min`/`neg`/`call` of functions that
     are themselves EXACT at the depth passed,
  d. the depth argument at the call site equals the parent's depth minus exactly 1 (no
     reduction, no extension) OR the function has no depth argument and terminates only
     on `terminal`,
  e. no `avg`, `mix`, `tread`, `sample`, or slot-derived value enters the accumulated
     Score.
Mixed paths: if ANY path to a `ret` violates a-e, that `ret` is INEXACT. A function is
EXACT only if all its `ret`s are. `max(exact, avg(...))` is therefore INEXACT — the
conservative direction. There is no partial credit. Tables read for ordering only (not
entering the Score) do not break exactness; tables entering the Score do.
### 2.4 Pass rules
- EXACT `ret` sites: on STANDARD at the program's declared depth D (its `tread` value),
  clamped to <= R, the returned root score and best move must equal the reference's.
  Tolerance 0. Any mismatch = REJECT (it is a bug, not a style).
- INEXACT `ret` sites: on STANDARD and TACTICAL, the returned score must satisfy
  LOSS_bound <= s <= WIN_bound (from `score_of`), the returned move must be in
  `moves(p)`, and the program must not return a WIN-class score where the reference at
  R=6 proves no forced win exists within 6 plies (and symmetric for LOSS). Violations
  = REJECT.
- Termination: every position must return within the budget; timeout = REJECT.
- Determinism: two runs with the same seed must agree; disagreement = REJECT.
### 2.5 What the oracle does NOT check
It does not check strength. A program may prune 99% of the tree and pass. Strength is
the ladder's job; the oracle only forbids lying about what was searched.

## 3. Mates-per-cost (PROGRAM surrogate and fitness component)
Unit note: all fixed budgets in this file are counted in COST UNITS — the interpreter's
running sum of per-primitive modeled cost (GRAMMAR 8), incremented as primitives execute.
Not nodes ("node" is alpha-beta vocabulary), and not evaluations: eval-count is neutral
between eval-bound paradigms (AB, MCTS) but NOT against terminal-driven ones —
proof-number search proves mates from `terminal()` with zero `eval` calls, so
found/evaluations divides by zero and the early gradient would point at search-without-
eval, the opposite of the thesis. Cost units are consumed by every program (every
primitive costs something), are deterministic and reproducible, and cannot be gamed by
avoiding any one primitive. Wall time on the declared hardware is recorded alongside as
the reality check on the cost model (revisit trigger: cost-vs-time correlation < 0.95).
Time-based stages (STC, LTC, anchor) are paradigm-neutral already.
- Set: MATE-N, N in {1,2,3,4}, 500 each, mined by retrograde walk from actual game
  endings in own self-play (rules-derived; refreshed with STANDARD). A position is in
  MATE-N iff the reference at R=2N proves a forced mate in exactly N.
- "Found": the program's returned move is a mating move (rules-derived: the reference
  proves it). NO score condition while `score_of` is still being learned — early on the
  WIN table value is noise, and comparing to it would make the primary bootstrap signal
  meaningless. The returned score is RECORDED for diagnostics. Once `score_of` is stable
  (its WIN entry has moved < 5% over the last 10 champions), a second, stricter metric
  is added alongside: found-with-score = mating move AND the mating move's score >= the
  score of every other root move the program evaluated (rank-based, not absolute). Both
  metrics reported; the filter in (a) uses the plain one until the strict one is live.
- Metric: found / (cost units consumed over the set), reported per N. Programs that
  find more mates per unit of work are searching more soundly, not just more.
- Role: (a) a filter — a PROGRAM candidate must score >= 0.9x the champion on MATE-{1,2}
  and >= 0.8x on MATE-{3,4} to reach the ladder; (b) an early gradient before the eval
  is useful (the purity lineage's main signal until eval ~1500); (c) the structural
  counterweight to reckless pruning — unsound pruning's characteristic failure is a
  missed forced mate.
- Not a substitute for Elo. A program can be a mate specialist and lose games; the ladder
  catches that.

## 4. Agreement-with-deeper-self (TABLE / STAT surrogate)
- Set: STANDARD.
- Reference move: the CHAMPION at 32x the candidate's cost budget.
- Metric: fraction of positions where the candidate's move equals the reference move,
  weighted by |champion static eval - champion 32x search score| so tactical positions
  count more than quiet ones (this is the weighting that stops "agree on quiet
  positions, ignore tactics" from scoring well). Weights are rules/search-derived.
- Filter: candidate must reach >= champion's own agreement at the same budget minus
  0.5 percentage points. Ties go to lower cost.
- Reported alongside Elo in every ledger entry so drift between surrogate and Elo is
  visible; if three consecutive accepted TABLE candidates raised agreement but not LTC
  Elo, the weighting is reviewed (revisit trigger).

## 5. Held-out loss (NET / ARCH / FEATURE surrogate)
- Held-out: 2% of each datagen batch, never trained on, refreshed per iteration.
- Metric: loss on the blended target (outcome/search-score blend with the current
  learned lambda), compared to the champion on the same held-out set.
- Filter: must not be worse than the champion by more than 0.5%. Better loss does NOT
  by itself accept; a NET must still pass fixed-cost-budget and the ladder. (Loss and Elo
  diverge regularly; loss is a cheap "not broken" check, nothing more.)

  > **Note, 2026-09-11 — this filter runs on a signal already MEASURED as uninformative, and it
  > rejects a third of all ARCH proposals before any game is played.**
  >
  > The parenthetical above ("loss and Elo diverge regularly") is not a hedge, it is an established
  > measurement. `proxies_RESULT.md` settles it at a scale no single experiment here matches:
  >
  > | signal | evidence | result |
  > |---|---|---|
  > | held-out surrogate (`mcnemar_z`) | 239 paired gate results | r = **−0.095**, CI [−0.220, +0.032] |
  > | training loss | n=12 control-vs-origin | r = **+0.379**, CI [−0.249, +0.783] |
  > | training loss, width A/B | equal-time h2h | w64 loss **lower** (0.0262 vs 0.0397) and w64 **loses** |
  > | training loss, draws A/B | control vs origin | include loss **lower** (0.0191 vs 0.0726) and include **wins** |
  >
  > **Both directions appear**, which is what "uninformative" means and why it is the right word:
  > the signal is noise with respect to strength, not a reversed predictor.
  >
  > **The consequence for this filter.** A one-sided veto built on noise rejects candidates
  > essentially at random. Counted across every ledger in the repo: **33 of 97 ARCH proposals (34%)
  > were rejected with `reason: surrogate_filter` and never played a game**
  > (`arch_surrogate_filter_RESULT.md`). The 0.5% tolerance was chosen as a cheap "not broken"
  > guard, and against a signal with r ≈ 0 it discards a third of the search at no informational
  > gain. ARCH is off in production on wall-clock grounds (`width_clock_RESULT.md`), so this is
  > latent; recorded for whoever re-enables it, where the fix is to widen or drop the filter.
  >
  > > **Correction to this note, made before it was believed.** It first claimed the relationship is
  > > a monotone INVERSION, on three arms whose median loss (0.02130 / 0.02690 / 0.03550) ordered
  > > exactly opposite to their strength (0.484 / 0.586 / 0.628). That is real but it is **not
  > > evidence of inversion**: those three arms differ in LEARNING RATE, which changes how much the
  > > net fits per generation *independently* of how good it is, so lr drives both columns and the
  > > monotonicity is expected without any causal link. n=3 against n=239 besides. `proxies_RESULT.md`
  > > was already in `RESULTS_INDEX.md` when I wrote it — the index built for exactly this.

## 6. Fixed-cost-budget gate (NET / ARCH / FEATURE only)
- Purpose: separate eval quality from speed for changes whose speed cost is known.
- SPRT at a fixed cost budget per move (declared: the cost of ~20k seed-program
  evaluations on the seed net, recorded as a number in the ledger), book openings, seat/colour-swapped, alpha = beta
  = 0.05, bounds [e1 - 2, e1] with the same global e1 as 7.2 (bootstrap 5; floor 0.5). Sequential, so it spends games only
  where the evidence is ambiguous.
- (Correction log: the previous draft fixed 2,000 games and claimed ~+/-4 Elo CI. The
  author's own gates measure ~+/-15 Elo at 2,000 games in a decisive-heavy regime;
  reaching +/-4 would need ~28,000 games. "CI lower bound > 0" at +/-15 would have
  silently demanded a ~15 Elo point estimate. A fixed game count cannot be right at
  every strength; SPRT with the derived bound is.)
- Then proceed to STC/LTC where speed counts. A NET that passes fixed-cost-budget but fails STC
  has a speed problem; both results are recorded so the dial (Section 9) sees it.
- A NET that passes fixed-cost-budget but fails STC has a speed problem; the ledger records
  both so the dial (Section 9) sees it.

## 7. The ladder (all classes)
### 7.1 Phase-1 values (declared; revisit per phase)
- STC: 5s + 0.05s. LTC: 30s + 0.3s. Ratio 6:1, held as both grow.
- Phase-boundary values (changed at declared phase starts, never at a rating): P3 start
  moves to STC 10+0.1, LTC 60+0.6 (fishtest's, calibrated for CPU alpha-beta engines);
  VLTC declared at P4 for PROGRAM and ARCH only. Phase boundaries are themselves defined
  in MASTER_PLAN by what has been built (explanation layer live, etc.), not by Elo.
- Hardware and thread count fixed per phase and recorded in the ledger with every entry.
### 7.2 SPRT bounds
- SPRT semantics, stated so they are not misread: bounds [e0, e1] test H0: elo <= e0
  against H1: elo >= e1. PASSING means H1 accepted, i.e. the candidate is shown to gain
  at least ~e1 (not merely "not negative"). A true NON-REGRESSION test therefore has
  e1 = 0 and e0 < 0, e.g. [-5, 0]: H0 "worse than -5" vs H1 "not worse than 0".
- **Three things live in "the gate", and they get three different answers:**
  1. **The acceptance criterion (what counts as enough) — FIXED, human.** If the engine
     picked the bar it must clear, "stronger" would be engine-defined and the claim
     collapses: an instrument calibrated by its subject measures nothing, and the
     degenerate solution (bounds that accept everything) is obvious and unstoppable.
     Concretely fixed: the error rates alpha = beta = 0.05, and the TC within a phase.
     The TC is fixed for COMPARABILITY, not purity — if each champion were tested at a
     clock it selected, the ledger's Elo numbers would not be on one scale and
     cross-version claims would mean nothing. Declared per phase; changes between
     phases are planned and recorded.
  2. **How much evidence to gather — ENGINE-DECIDED, already.** SPRT is exactly that
     decision: a candidate near a bound gets thousands of pairs, an obvious dud a few
     hundred; nobody picks the count, the evidence does. The bandit decides which
     candidates and arms get gate time. Both are in Learned. The engine controls how
     much evidence is collected; it does not control what counts as enough.
  3. **The bounds schedule — DATA-DERIVED, not hand-picked.** The first draft hand-set
     STC [0,10] early and [0,2]/[0.5,2.5] "from ~2500" — two arbitrary numbers and a
     magic switch point, which this project refuses everywhere else. Instead:
     - e1 (the gain a candidate must show) = the MEAN gain per acceptance, GLOBAL across
       classes, measured by ONE ANCHOR MATCH: current champion vs. the champion from 20
       acceptances ago (any class), 2,000 games at LTC from the book, both sides; divide
       the measured delta by 20. Recomputed every 20 acceptances; recorded with its CI.
     - Why global and not per class: an anchor measures the CHAMPION's progress, and every
       class moves the champion. "Champion vs champion-20-NET-acceptances-ago" still
       contains every other class's gains in that window; it does not measure NET's gain
       per acceptance any more than a global anchor does. Per-class anchors were a
       differencing scheme that cannot attribute (attribution would need the
       counterfactual champion-without-class-X, which nobody can afford). One global
       series has better precision and one attributable cost, and its only job is to
       calibrate what an acceptance is worth against reality.
     - Why an anchor and not per-acceptance matches: a 500-game match has ~+/-30 Elo CI;
       a median of 20 of them wobbles ~+/-8 Elo, while the bound width is 2 and the floor
       0.5 — the bar would swing across its entire meaningful range on noise, and looser
       bounds admitting smaller gains would feed that noise back into the next median (an
       oscillation the floor does not stop). Measuring one 20x-larger effect once gives
       ~0.4 Elo SE on the per-acceptance mean from 2,000 games — 5x fewer games, ~11x
       better precision — and a single, attributable compute cost. Additivity of small Elo
       increments is assumed and is safe at these magnitudes. One anchor champion is held.
     - Width e1 - e0 fixed at 2 Elo for STC, LTC, and the fixed-cost-budget gate (a declared resolution constant).
     - LTC bounds: [e1-2, e1] when derived e1 >= 1 (must gain); [-2, 0] (non-regression)
       when derived e1 < 1, i.e. when typical accepted gains are already tiny and the
       job of LTC is to protect against depth-dependent regressions rather than to
       demand more.
     - **Floor, to stop the loosening spiral:** e1 >= 0.5 always. Without a floor, looser
       bounds accept smaller gains, the median falls, bounds loosen further. The floor is
       fishtest's LTC lower bound and is declared.
     - Not circular: the schedule is calibrated to the observed effect size in CLOSED
       history (already-accepted, already-measured gains), never to whether the current
       candidate passes. A candidate cannot move its own bar.
     - Bootstrap: until 20 acceptances exist, e1 = 5 (declared start value; recorded;
       retired automatically).
- Summary: the engine decides the evidence, the human declares the threshold semantics,
  the error rate, and the scale; the schedule moves from hand-set to data-derived. One
  fewer arbitrary number in the Given column.
- (Correction log: the first draft wrote LTC [0, 5] and called it non-regression. That
  would have required proving +5 at LTC after already passing STC and rejected most good
  candidates. Fixed to [-5, 0].)
### 7.3 Openings and statistics
- Random-ply openings until the self-generated unbalanced book exists (MASTER_PLAN
  "Openings"); the book from then on. Every opening played from both sides.
- Pentanomial (game-pair) statistics UNCONDITIONALLY, from the first gate. Pentanomial's
  benefit is pairing, not draws: each opening played from both sides cancels opening bias
  and reduces variance at ANY draw rate. The author's 4PC gates already run pentanomial
  at ~0.4% draws (e.g. `pent 392-7-888-8-467`), and that is where the variance figures
  in these documents came from. Fishtest uses pentanomial unconditionally for the same
  reason. (Correction log: an earlier draft switched to pentanomial only above 40% draws,
  which would have kept the early, highly decisive run on the higher-variance statistic
  exactly when resolution matters most.)
### 7.4 Ordering of candidates into the ladder
The bandit (Section 9) picks; within a class, candidates with better surrogate scores go
first. A candidate waits at most 48 hours before its STC starts or it is re-derived
against the newer champion (stale candidates are not tested against a champion they
were not built from).

## 8. Explanation-honesty check (stage 6, all classes; ACTIVE FROM P3)
The explanation layer does not exist before P3. Until it does, stage 6 is recorded in
every ledger entry as N/A (not as PASS). It becomes mandatory the day the layer ships.
- Set: ADVERSARIAL — positions found by the adversary track (MASTER_PLAN "Adversarial
  robustness") where the current champion at a low cost budget disagrees with itself at a
  high one, or where a weaker opponent scored against it. 300 positions, refreshed weekly.
- Check: on ADVERSARIAL, the candidate's explanation-layer confidence (PV stability,
  static-vs-deep residual class) must be LOW on at least 80% of positions where the
  candidate's move differs from its own 32x-cost move. A confident wrong answer is a
  regression in the property the project sells.
- Fail = REJECT even if the ladder passed. Recorded with the entry.

## 9. The bandit
- Arms: PROGRAM, TABLE, NET, ARCH, FEATURE, STAT, HYPER, PURITY.
- Reward per arm = (acceptances_i in the last 7 days x e1) / (gate compute-hours spent
  on arm i in the same window, including rejected candidates).
  - Attribution by COUNT, calibration by ANCHOR. An acceptance belongs to exactly one arm
    — that is the one quantity in the system unambiguously owned by an arm. Each accepted
    candidate provably cleared >= e1 via its own SPRT, so valuing it at e1 is earned, not
    estimated. The global anchor (7.2) exists only to keep e1 honest against reality; it
    never attributes.
  - Why not the SPRT point estimate: biased upward, arm-dependently, by early stopping.
  - Why not per-acceptance matches: +/-30 Elo each; unusable at the needed precision.
  - Why not anchor deltas per arm: the anchor measures the champion, which every arm
    moves. Two failed attempts are recorded in 12.7 and 12.9: attributing a per-class
    anchor to its class read a slow arm as 20x too good; matching windows then made
    reward = G / (compute share), which cancels the arm's productivity entirely and
    converges to uniform allocation regardless of what any arm produces.
  - Sanity example: NET accepts 40 in a week on 100 h, PROGRAM accepts 2 on 100 h -> NET
    reads 20x PROGRAM, which is the truth.
- Ledger "what it earned": e1 at the time of acceptance — the bar the candidate provably
  cleared via its own SPRT — together with the SPRT LLR and pair count as acceptance
  EVIDENCE. The same quantity the bandit uses; the two cannot disagree. Individual
  acceptances are not separately measured (they cannot be, at useful precision, without
  more games than they are worth), and no per-class anchor mean is stamped on entries
  (per-class anchors no longer exist; see 12.10). Milestone RELEASES get a dedicated
  4,000-game match against the previous release so release-to-release claims carry
  their own CI.
- Allocation: Thompson sampling over rewards, with a FLOOR of 5% of gate compute per
  arm (no arm starves; PURITY floor is 3%). Recorded daily in the ledger.
- The dial (eval-trust vs. depth) is not an arm; it emerges from which NET/ARCH and
  PROGRAM candidates pass on the clock. The ledger reports, per champion, mean cost units and mean
  evaluations per move at LTC, and eval network cost, so the dial's position is visible over time.

## 10. Degenerate solutions -> the check that catches each
| Degenerate solution | Caught by |
|---|---|
| Prune everything / return eval | mates-per-cost filter (3); ladder (7) |
| Claim wins that aren't there | oracle inexact rule (2.4) |
| Exploit hash collision / stale slot | oracle exact rule (2.4); determinism check |
| Never be exact so exact-check never fires | impossible: taint is static (2.3) |
| Agree on quiet positions, ignore tactics | residual-weighted agreement (4) |
| Overfit the oracle set | set refresh every 20 champions + old set kept (2.2) |
| Fast at STC, worse at LTC | LTC non-regression / positive lower bound (7.2) |
| Good at the test clock, bad at chess | STC->LTC ratio held; VLTC for structural changes at P4 |
| Bigger net that wins fixed-cost-budget, loses on clock | fixed-cost-budget then STC (6, 7) |
| Confident explanations of blind spots | honesty check (8) |
| Starve the slow-but-important track | bandit floors (9) |
| Test against a champion the candidate wasn't built from | 48h staleness rule (7.4) |
| Infinite loop / budget abuse | cost cap + hard ceilings (GRAMMAR 8); oracle timeout |
| Stochastic program that passes by luck | determinism check (2.4); seeds fixed per game |

## 11. Numbers table (every declared constant, in one place, with revisit triggers)
| Constant | Value | Why | Revisit when |
|---|---|---|---|
| Oracle depth R | 4 std / 6 tactical | cheap; deep enough to prove short mates | oracle takes > 5 min per candidate |
| STANDARD size | 2,000 | CI on agreement ~1pp | agreement filter flips accept/reject on noise |
| Set refresh | every 20 champions | overfit window | oracle pass rate rises while LTC Elo stalls |
| MATE-N sizes | 500 x 4 | stable per-N ratio | any N has < 50 positions available |
| Mates filter | 0.9x / 0.8x champion | tolerate variance, forbid collapse | a known-good program is rejected by it |
| Agreement budget ratio | 32x | strong reference, affordable | reference takes > champion's LTC time |
| Agreement filter | champion - 0.5pp | noise band | drift trigger in Section 4 |
| Held-out fraction | 2% | cheap | held-out CI too wide to separate candidates |
| Fixed-cost-budget gate | SPRT @ fixed cost budget, bounds [e1-2, e1] with the global e1 | fixed counts cannot be right at every strength; measured +/-15 Elo at 2,000 games | derived e1 hits the floor |
| STC / LTC | 5+0.05 / 30+0.3 | phase 1 | P3 phase boundary -> fishtest values |
| SPRT bounds | derived: e1 = global anchor delta / 20, width 2, floor e1 >= 0.5, bootstrap e1 = 5; alpha = beta = 0.05 fixed | calibrates resolution to observed effect size from closed history; error rate and TC fixed for validity and comparability | floor binds for > 50 acceptances (effect sizes have collapsed) |
| Adversarial set | 300 / weekly | enough to estimate 80% | adversary finds < 300 |
| Honesty threshold | 80% low-confidence-when-wrong | strict but not perfect | false rejects of known-good nets |
| Bandit window / floor | 7 days / 5% (3% purity) | responsive, no starvation | an arm shows late payoff patterns |
| Anchor match | GLOBAL: champion vs champion-20-acceptances-ago, 2,000 games LTC; e1 = delta/20 | ~0.4 Elo SE; calibrates e1 only, never attributes | fewer than 20 acceptances in 60 days (anchor stale; use what exists, flag it) |
| Release match | 4,000 games vs previous release | release-to-release claims carry their own CI | — |
| Staleness | 48 h | champion churn | churn slows to weekly |

## 12. Self-audit
### 12.1 Holes checked
- Is anything here a chess opinion? Position sets are drawn from own games with
  rules-only filters. Weights use own search residuals. Mate sets use rules-derived
  terminal outcomes. STC/LTC choice is the declared measurement judgment (MASTER_PLAN).
  PASS.
- Can the program influence its own oracle verdict? Taint is static; sets are external;
  the reference SEARCH is frozen (the reference EVAL is the current champion's, so the
  comparison is self-consistent at one moment). Consequence, stated: exactness verdicts
  are meaningful only within a single champion epoch and are not comparable across
  epochs; the oracle asks "does this program search what it claims, given this eval",
  never "is this eval right". PASS.
- Can a NET game the fixed-cost-budget gate? Only by being better at fixed cost budget; the ladder
  then charges for speed. PASS.
- Can the bandit lock onto one arm? Floors. PASS.
- Can a candidate be accepted without LTC? No path in Section 0 skips stage 5. PASS.
### 12.2 Known weaknesses (declared)
- R=4/6 is shallow; a program could be exact and correct to depth 4 and buggy at depth
  12 (e.g., a hash bug that only bites deep). Mitigation: TACTICAL set at R=6 and the
  ladder itself; a deep-only bug that gains Elo is, by definition, not caught. Revisit
  R if the ledger shows accepted programs later reverted for correctness.
- Mates-per-cost rewards mate-finding specifically; a program could over-invest in it.
  The ladder is the counterweight; the filter thresholds (0.9x/0.8x) are deliberately
  loose so mates gate soundness, not style.
- The explanation-honesty check depends on the adversary track producing positions; if
  it produces none, stage 6 is vacuous. Ledger records the set size; < 100 positions
  disables the check and flags it.
- Thompson sampling with a 7-day window is myopic; a track with a 3-week payoff (ARCH)
  can look bad. The 5% floor is the only protection. Revisit trigger stated.
- Budget-driven programs (iterative deepening, budget termination) have no static
  depth, so rule (d) cannot be established and they are INEXACT by construction. As
  programs mature past the fixed-depth seed, the oracle's protection therefore narrows to
  the INEXACT bounds check + determinism + mates-per-cost + the ladder. This is a real
  weakening and is declared. Partial mitigation: a program with an ID wrapper around an
  EXACT inner function is oracle-checked on the INNER function at each fixed depth it
  completes (the taint is per function); only the root's choice of which iteration to
  return is unchecked. Revisit if an accepted budget-driven program is later found to
  return provably wrong scores.
### 12.3 Open items
- Exact form of the "WIN-class" score threshold when `score_of` is still being learned
  early on (proposal: WIN-class = within 10% of the current WIN table value).
- Whether TABLE candidates should also run the oracle when the table enters the Score
  (currently: taint marks such paths INEXACT, so tables cannot fake exactness; but a
  table could still push an INEXACT score outside bounds — the bounds check catches it).
  Decision: no separate oracle run; bounds check suffices. Revisit if a TABLE acceptance
  is later reverted for an impossible score.
- VLTC values at P4.
### 12.11 Corrections applied in eighth (external) audit
- Pentanomial statistics made unconditional (pairing, not draws, is the benefit).
- Arm enum unified to PURITY across FITNESS, CRATE, SCHEMAS.
### 12.10 Corrections applied in seventh (external) audit
- Bandit attribution, third attempt: anchor deltas cannot be attributed to arms because
  every arm moves the one champion (the window-matched version reduced to reward = G /
  compute-share, i.e. uniform allocation). Replaced with attribution by acceptance COUNT
  x e1, calibration by ONE GLOBAL anchor. Per-class anchors removed for the same reason.
### 12.9 Corrections applied in sixth (external) audit
- "Evaluations" as the budget unit was not neutral against terminal-driven search (PN
  finds mates with zero eval calls -> divide-by-zero and a degenerate attractor toward
  search-without-eval). Re-denominated to COST UNITS (GRAMMAR 8 cost model, accumulated
  at runtime), with wall time as the reality check. Propagated to GRAMMAR.
- Bandit attribution: numerator and denominator now cover the same anchor window per arm.
### 12.8 Correction applied in fifth audit (author) — superseded by 12.9
- Budgets re-denominated from "nodes" to "evaluations". Superseded: see 12.9.
### 12.7 Corrections applied in fourth (external) audit
- Derived e1 from per-acceptance 500-game matches was noise-dominated (+/-8 Elo wobble
  on a 2-Elo-wide bound; oscillation the floor could not stop). Replaced by per-class
  ANCHOR matches (champion vs champion-20-ago, 2,000 games, delta/20): ~11x better
  precision at 5x fewer games, one attributable cost.
- Bandit reward and ledger "what it earned" re-based on the anchor series; per-acceptance
  matches dropped; milestone releases get a dedicated 4,000-game match.
- Fixed-cost-budget gate: fixed 2,000 games with a claimed +/-4 CI (measured +/-15) replaced by
  SPRT at fixed cost budget with the derived bound. Previous "CI > 6" revisit trigger would
  have fired immediately and permanently.
### 12.6 Corrections applied in third (external) audit
- The gate row bundled three things. Separated: acceptance criterion (fixed, with the
  argument why), evidence quantity (engine-decided via SPRT + bandit, already Learned),
  bounds schedule (now data-derived from closed history with a declared floor against
  the loosening spiral). Magic "2500" switch point removed.
### 12.5 Corrections applied in second (external) audit
- SPRT bounds: LTC [0,5] mislabelled as non-regression; fixed to [-5,0]; semantics spelled out.
- Mates-per-cost: score condition removed while `score_of` is unstable; rank-based
  strict metric added for later.
- Bandit reward: SPRT point estimate replaced by a fixed 500-game post-acceptance match,
  which also becomes the ledger's "what it earned".
- Fixed-cost-budget gate: 2,000 games; pass = CI lower bound > 0; contradictory +5 threshold removed.
- "Reference is frozen" qualified: search frozen, eval is the champion's; exactness is
  epoch-local.
### 12.4 Corrections applied in first self-audit
- Exactness rule (b) as first written classified the alpha-beta SEED as INEXACT (its
  cutoff is an early `ret`), which would have disabled the strong oracle check for every
  alpha-beta descendant. Added the declared window-cutoff exemption.
- Stage 6 given an activation point (P3) and an N/A recording rule before it.
- Budget-driven programs' INEXACT-by-construction status declared as a known weakening,
  with the per-function inner-check mitigation.


## MEASURED 2026-09-08 — the cost term rewards something that cannot become strength

FITNESS 3 scores mates-per-COST. The search track's game gate scores strength. **For the programs
this track actually evolves, those cannot be made to agree**, and the reason is structural.

**The observation.** The MCTS lineage drove its surrogate from 0.001145 to 0.005034 — a 4.4x
improvement — while five consecutive game gates returned *exactly* 0.500 +/- 0.250. The surrogate
saw a large gain; the games saw nothing at all.

**The immediate cause.** `play_progs` gave both sides the same `budget` and no cost ceiling, so
being cheaper bought a candidate nothing: it simply returned sooner with the same answer.

**Why the obvious fix does not work.** Equalising COST per move instead of budget looks like the
remedy, and it is inert or harmful:

* Set at 4e8 it never binds — the alpha-beta seed costs 3.972e8 per position and the MCTS seed
  3.843e8. Measured, after the change had been written and nearly reported as working.
* Set low enough to bind, it is *worse*. `interp/src/lib.rs:501` unwinds the entire program on a
  ceiling hit and `run` returns MOVE_NONE, which `play_progs` treats as a FORFEIT. A binding
  ceiling therefore does not grant the cheaper program more search — it makes whichever program
  crosses the line first LOSE OUTRIGHT, converting the gate into a pure cost race. That is the
  surrogate's own failure mode, imported into the instrument built to catch it.

**The real reason.** A DEPTH-LIMITED program cannot spend a saving. `bare_alpha_beta` searches to
the depth in table 0 and ignores `Budget` entirely, so halving its cost means finishing sooner with
the **identical move** — not searching twice as far. Cost-efficiency converts to strength only for
a BUDGET-AWARE program, one that searches until its allowance is exhausted.

**What that implies, and it is worse than the paragraph above first said.** I wrote that
"iterative deepening is the canonical budget-aware search" and pointed at rung 5. That is true of
real iterative deepening and **false of `ab_id` as implemented here** — it loops depth 1..D, which
is still depth-limited and never reads `Budget`. Checked rather than assumed, across every
reference program:

| program | reads `Budget` |
|---|---|
| depth_one, bare_alpha_beta, capture_extension, table_reduction | no |
| ab_hash, ab_probe_only, ab_store_only | no |
| **ab_id, ab_hash_id** | **no** |
| uct_mcts | **YES** |
| proof_number | **YES** |

**Nine of eleven reference programs are budget-blind — the entire alpha-beta family.** Only the
MCTS and proof-number paradigms can spend a saving.

**So FITNESS 3 IS NOT PARADIGM-NEUTRAL, which is the property it was chosen for.** Its cost term
converts to playing strength for MCTS and PN, and cannot convert for alpha-beta, because an
alpha-beta program that costs half as much returns the identical move sooner. The fitness therefore
scores two paradigms on different currencies while presenting one number, and the MAIN lineage —
the one the whole search track is built on — is the half where the currency is counterfeit.

That is a Given-column defect, not a tuning problem. It also predicts exactly the divergence
observed: the surrogate/game disagreement showed up in the MCTS lineage (surrogate 4.4x, games
flat) because that is the lineage where cost is real, while MAIN's surrogate gains are unconvertible
by construction.

This is not a bug to patch in the gate. It is a statement about what mates-per-cost can and cannot
measure, and it belongs here. The gate's cost-ceiling plumbing is left in place (it is correct for
a budget-aware seed) with the value disabled, so nothing pretends to work.
