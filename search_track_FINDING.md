# The search track's surrogate rewarded searching LESS (2026-09-08)

Recorded here because the code change was swept into the gate-A/B commit by a `git add -A` and
landed under a message that does not describe it. The logs that hold the evidence are gitignored,
so the numbers live here.

## What happened

P2 (search learning) is described in MASTER_PLAN:276 as "open-ended, runs from day one". It was
not running -- `evolve_search.log` shows a run that reached generation 13 and died, with nothing
restarting it. Restarted at 12:30, it immediately produced:

```
  seed: bare alpha-beta  80 mates  3049174420 cost  0.03 mates/Mcost  (71 nodes)
  gen   1  ACCEPT  80 mates  8.73 mates/Mcost  (71 nodes, was 0.03)
  gen   2  ACCEPT  80 mates  8.74 mates/Mcost  (70 nodes, was 8.73)
```

333x cheaper from ONE type-preserving edit, at an unchanged node count. A 290x gain from a single
mutation is not a discovery.

## The mechanism, tested rather than assumed

`evolve.rs` accepts on `f >= best_found && rate > best_rate` -- keep every mate, get cheaper --
with no game gate in the loop (its header says the gate "is what would confirm a winner on the
clock", i.e. deliberately outside). The set came from `mate_set`, which collects positions holding
a mate in ONE. On such a set shallowness cannot lose a mate, so the guard could never bite.

Measured with the SEED program, no mutation involved (`examples/mate_surrogate_probe.rs`):

| set | depth 1 | depth 2 | depth 3 |
|---|---|---|---|
| mate-in-1 (what it optimised on) | 80/80 mates, 246M cost | 80/80, 3049M | 80/80, 34551M |
| mate-in-2 (the repair) | **17/40** forcing, 124M | 40/40, 1356M | 40/40, 16540M |

On mate-in-1, searching less keeps every mate and costs 12x less -- the guard is inert. On
mate-in-2, searching less loses 23 mates -- the guard bites. **The repair is the SET, not the
rule.** The rule was right all along and had nothing to enforce.

## My probe was wrong first, and the control caught it

The mate-in-2 arm initially read 0 mates at depths 1, 2 AND 3. A set built to be solvable scoring
nothing at every depth is not believable, and the fault was mine: the scorer credited a move only
if it mated IMMEDIATELY, and a mate-in-two's first move never does. Fixed by recording the forcing
move alongside each position. Without that check, "depth 3 solves nothing" would have been written
up as a property of the search.

## State

evolve.rs now runs on 80 mate-in-1 + 40 forced-mate-in-2 (positions with no mate-in-1 available),
and REFUSES TO RUN if the depth-requiring half is empty -- otherwise the loop optimises toward a
depth-1 mate detector while printing ACCEPT. Seed on the mixed set: 120/120 mates, 0.03 mates/Mcost.


---

# THE ANSWER (2026-09-08): no ladder rung is REACHABLE, because mutations cannot introduce a primitive

~690 candidates across two runs, zero accepts. Earlier today I blamed, in order: the surrogate
(fixed, real), the fitness depth (moved to D=3, defensible but not the cause), and the DISTANCE to
the fitter rung (+104 nodes, true but not the whole story). The actual constraint is harder than
any of those.

## What the operator set can construct

Every `Node::` variant any operator in `crates/grammar/src/mutate.rs` ever builds:

    Const, Max, Min, Arith, Cmp, Store, Set, Field, Nop, Loop, If, Budget, Avg

And what it can NEVER build — verified by grep, with the pattern proven against reference.rs so
this is an absence and not a broken predicate:

    Pred, Probe, Key, TRead, Eval, Moves, Apply, Terminal, ScoreOf, Argmax, Foreach, Call, ...

`Pred` occurs in mutate.rs exactly twice: line 57 inside `children()` (traversal) and line 97
inside the rebuild. Neither constructs one. `Store` and `Field` appear only as match PATTERNS —
`Op::WrapIf` wraps an existing `Store`/`Set`/`Nop`, and the `Field` case swaps which FieldId an
already-present `Field` reads. `Probe`, `Key` and `TRead` are constructed zero times.

`Op::WrapIf` is worth stating outright because GRAMMAR 9 names it as the operator for rung 6. It
always emits the SAME condition — `Cmp(Budget, Const(r % 5), Gt)` — a budget comparison. There is
no path by which it produces a capture predicate.

## Therefore every declared rung is unreachable, not merely distant

| rung | needs | constructible? |
|---|---|---|
| capture extension (rung 6, +9 nodes) | `Pred(m, p, IsCapture)` | **NO** |
| table reduction (rung 7, +15 nodes) | `TRead(3, [d, i])` | **NO** |
| alpha-beta + hash reuse (+104, the only FITTER rung) | `Probe(Key(p))` | **NO** |

Not "a hundred edits away". **Impossible at any edit count, at any depth, with any budget.** The
mutation operators can tune constants, rearrange existing structure, and wrap existing statements
in an If or a Loop. They cannot add a primitive the program does not already contain.

That is a complete explanation for zero accepts in ~690 candidates, and it supersedes my three
earlier explanations. It also means the search track cannot be fixed by any parameter: not depth,
not edit count, not population, not the fitness set.

## What this says about GRAMMAR 4 and GRAMMAR 9 together

GRAMMAR 9 asserts a path of single mutations from the seed where every step is fitter. GRAMMAR 4
supplies the operators that would have to walk it. Measured, the operators cannot reach a single
declared rung — so the ladder and the operator set have never been checked AGAINST EACH OTHER.
Each is individually reasonable; jointly they do not compose.

The honest framing: the ladder is a statement about the GRAMMAR (these programs are expressible),
and the operator set is a statement about the SEARCH (these edits are available). Expressible is
not reachable, and nothing in the repo connected the two until now.

## What would fix it — not doing any of this on a hunch

1. An INSERT-PRIMITIVE operator that can introduce a well-typed `Pred`, `Probe`/`Key`, or `TRead`
   at a type-correct position. This is the smallest change that makes any rung reachable at all.
2. Or seed the search from a program that ALREADY contains the primitives, so the remaining edits
   are constant tweaks and rearrangement — which is what the current operators are good at.
3. Either way, add a REACHABILITY TEST to the ladder: for each rung, assert that some finite
   sequence of operators can produce it from the seed. That test would have failed on day one and
   is the check whose absence let ~690 candidates run against an impossible target.


---

# THE ACCEPTS ARE THE SHALLOWNESS MODE AGAIN — my guard does not bite at D=3 (2026-09-08)

I said these accepts were "not the degenerate mode" because 20/20 mates held including the
forced-mate-in-2 positions. Wrong. Program saving landed, the program can now be read, and it is
the shallowness failure wearing a different depth.

Exact numbers, six decimals:

    seed      20 mates  0.002436 mates/Mcost  71 nodes
    gen 13    20 mates  0.026726 mates/Mcost  73 nodes   11.0x
    gen 23    20 mates  0.026794 mates/Mcost  73 nodes

`diff seed.prog evolved_gen13.prog` is TWO changes:

1. **`Const(0)` -> `Const(1)`** inside `If(Cmp(Var("d"), Const(0), Eq), Ret(Eval(Var("p"))))` --
   the HORIZON GUARD. The search now returns eval at `d == 1` instead of `d == 0`, i.e. it stops
   **one ply earlier**. At fitness depth 3 it effectively searches depth 2. That is the whole 11x.
2. `Set("best", Max(best, vv))` wrapped in `Loop(Const(2), ...)` -- semantically a no-op, since
   Max is idempotent. It contributes the +2 nodes and a little cost. gen 23's further "gain" is
   that same Loop constant going 2 -> 1, i.e. doing the redundant work once instead of twice.

No `Pred` appears, so the new WrapIfPred operator had nothing to do with it.

## Why the guard failed

I added forced-mate-in-2 positions this morning specifically so a shallow program would lose
mates. That guard was calibrated for a fitness depth of 2, where dropping to depth 1 costs 23 of 40
forcing moves. The fitness now runs at depth 3, and I measured earlier that the mate-in-2 set is
solved **40/40 at depth 2** -- the forcing move is also the eval-best move, so it is found without
needing the extra ply. Cutting depth 3 -> 2 therefore costs nothing on this set.

**A depth guard has to require the FULL fitness depth, and mine required less than it.** When I
moved the fitness from D=2 to D=3 I did not re-check whether the guard still had teeth. It did not.

## The fix this points to

Positions where the answer is genuinely unavailable one ply shallower -- constructed by DISAGREEMENT
rather than by mate distance: run the seed at depth D-1 and depth D and keep positions where they
return different moves and the deeper one is correct. That is self-calibrating: it requires exactly
the depth the fitness runs at, whatever that is set to, instead of assuming a mate-in-N implies
N plies of search.

## Standing correction

Everything said today about the search track "finding two genuine improvements at D=3" is
withdrawn. It found "search one ply less", twice, and my window-narrowing test refuted the wrong
hypothesis -- the mechanism was depth all along, which is the same mode I had already caught and
believed I had fenced off.
