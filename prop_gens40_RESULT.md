# P2 `search_long_run` at its planned 40 generations: 0 accepts — and the MCTS gate could not have produced any other number

**2026-09-11 22:35. Planned N COMPLETE** — 40 distinct generations, `evolve` exited of its own
accord, both lineages printed their terminal lines. This is the deliverable the directive asked for:
*"run `search-long-run` to its planned 40 generations and report accepts."*

```
MAIN   0 accepted    final 19 mates   0.002385 mates/Mcost  (71 nodes)
MCTS   0 accepted    final 11 mates   0.001211 mates/Mcost  (131 nodes)
                     79 decisions, 0 ACCEPT, 79 REJECT
```

## The games

```
948 games:  W 5   D 778   L 165        DRAW RATE 0.821
when a game WAS decisive, the candidate won 5 of 170 = 2.9%
```

## Why the zero carries less information than it appears to

Holding each decision's OWN observed draw count fixed and granting the candidate a win in every
decisive game it actually played:

```
MCTS   40 / 40 decisions ARITHMETICALLY UNPASSABLE
MAIN    8 / 39 decisions ARITHMETICALLY UNPASSABLE
zero-variance readings (all 12 games drawn -> rule-of-three ci95 = 1.5/6 = 0.250):  17/79 = 21.5%
```

**The MCTS lineage's gate could not emit ACCEPT under any game outcome it was capable of producing.**
Its 40 rejects are therefore not 40 pieces of evidence about the generator.

**This is a REPLICATION, not a discovery.** `gate_arithmetic_RESULT.md` established it earlier the
same day by exact enumeration — of 210 possible 6-pair outcomes only 29 (14%) can ever accept, and a
candidate drawing >=4 of 6 pairs cannot pass at any decisive result. That result pooled 29 matches /
348 games at an 85.3% draw rate. **This run is the larger sample: 79 decisions / 948 games at 82.1%,
and it reproduces the earlier W=4 as W=5.** `refmatch_discrimination_PREREG.md` predicted the
zero-variance degenerate reading *before* it was observed; it occurred in 21.5% of decisions here.

## What is NOT concluded

**Not "the generator is fine and the gate is broken."** `gate_candidates_are_game_neutral_RESULT.md`
measured the candidates themselves at 14.7% decisive against 31–42% for reference programs, and the
2.9% decisive-win rate above is consistent with genuinely weak candidates. Both are true and a 6-pair
gate cannot separate them.

**Not "raise `gate_pairs`."** `gate_power_RESULT.md` MEASURED that more pairs buy more draws.

**Not "use deeper mate sets."** `forced_mate_set` already exists as a mate-in-2 builder and its own
doc comment records the outcome: at fitness depth 3 that set is solved 40/40 AT DEPTH 2, so the search
track cut one ply (`Const(0)` -> `Const(1)`, 11x cheaper) with all 20 mates intact. A mate-in-N does
not imply N plies of search.

## What follows — pre-registered BEFORE this verdict existed

`p2_fitness_PREREG.md` was committed at generation 37 and amended at generation 38, both while this
run's outcome was still unknown. The trigger it declared — zero accepts at generation 40 — has now
fired, so the next move is fixed and was not chosen after seeing the number:

**Games PRIMARY, mates a FILTER, and the stratum that grows is `disagreement_set` — NOT mate depth.**
A MATE-N stratum catches gross truncation but is blind to a one-ply cut; disagreement requires the
plies by construction. The live set was `10 + 4 + 5`, so the load-bearing stratum was **4 of 19
positions (21%)**. `fitness_set_composition_RESULT.md` measured the direction: depth-heavy `5/10/10`
holds a null search to 1306.606x where pure mate-in-1 lets it reach 2934.933x.

The prereg also records, in advance, the most likely way this fails: if candidates barely differ
behaviourally, changing what we weigh does not change what there is to weigh — and the named successor
is GRAMMAR 4 (type checker + mutation operators), which `gate_power_RESULT.md` already nominated.
