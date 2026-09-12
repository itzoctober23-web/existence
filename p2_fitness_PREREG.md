# PRE-REGISTRATION — the P2 fitness change: games PRIMARY, mates FILTER

**Written 2026-09-11 22:0x, while `prop_gens40` is at generation 37 of 40 and its verdict does not
yet exist.** The directive pre-registered the move conditionally — *"If it's zero, the next move is a
fitness question (games as primary, mates as filter), not another instrument"* — so the rule is fixed
here before the trigger fires, not chosen after seeing it.

## The trigger, stated so it cannot be reinterpreted

At generation 40, count `gate ACCEPT` lines in `prop_gens40.log`. **Zero accepts fires this change.**
Any non-zero count does NOT: it would mean the gate can move, and the correct next step would be to
characterise what got through, not to rebuild fitness.

Current standing (STATE, not a verdict): 0 accepts / 71 rejects at gen 37.

## Why games PRIMARY — measured, not assumed

* `proxies_RESULT.md`: **"Every cheap proxy for strength has failed. Only games measure strength here."**
* `surrogate_inverts_RESULT.md`: the grammar fitness **ranks the strongest reference program LAST** —
  a direct inversion, so the current primary term is worse than uninformative.

## Why mates as FILTER, not as score — measured, not assumed

`fitness_set_composition_RESULT.md` measured the best score an **obviously-broken** program (child
#30, which does essentially no search) can reach, as a function of set composition:

```
set (mate-in-1 / disagreement / window)    null-search score      mates kept
  25 / 0 / 0   pure mate-in-1                2934.933x            25/25
  15 / 5 / 5   mixed (shipped)               2274.444x            19/25
   5 / 10 / 10 depth-heavy                   1306.606x            11/25
```

A mate-in-one is a one-ply check, so a program that abandons search entirely still finds every one.
**The mate-in-1 stratum cannot see the degenerate solution** — FITNESS §10's declared "prune
everything / return eval". The 10 non-mate positions were doing all the discrimination.

The live run is `10 + 4 + 5` — **53% mate-in-1**, i.e. the majority of the set is the stratum measured
to be blind. Mates therefore become a *filter* (a correctness precondition: solve them or be
discarded) and stop contributing to the *score*.

## THE THREAT TO THIS DESIGN, stated before running it

`gate_candidates_are_game_neutral_RESULT.md`: the gate's own candidates are **14.7% decisive against
31–42% for reference programs**. `refmatch_discrimination_PREREG.md`'s arms establish that the games
CAN discriminate a real behavioural difference — so the neutrality belongs to the GENERATOR, not to
the games or the gate rule.

**If candidates barely differ behaviourally, making games primary changes what we weigh, not what
there is to weigh.** This is the most likely way this change returns null, and saying so now means a
null cannot later be explained away as "the fitness was still wrong".

Consistent with this, `gate_power_RESULT.md` already concluded the binding constraint is UPSTREAM,
and `gate_arithmetic_RESULT.md` forecloses the instrument route: more pairs were MEASURED to buy more
draws.

## The verdict rule

**Primary:** decisive-game fraction of the population's candidates, against the 14.7% baseline.
This is the quantity the change is supposed to move, and it is measurable without waiting for an
accept.

**Secondary:** accepts over a matched number of generations.

**PASS** requires the decisive fraction to rise with a CI excluding 14.7%. An accept count alone does
NOT pass — with 6-pair arithmetic an accept can occur in 14% of outcomes by luck.

**Matched on GENERATIONS, not wall clock**, and against a control run at the current composition from
the SAME seed — `resume_transient` cost ~95 Elo before paying back, and only matched arms cancel it.

## What would falsify the whole direction

Decisive fraction unchanged after the composition shift ⇒ the generator, not fitness, is the
constraint, and the next move is GRAMMAR 4 (type checker + mutation operators), which
`gate_power_RESULT.md` already names as the successor. **That is a real possible outcome of this
experiment, not a fallback to retreat to.**

---

## AMENDMENT 2026-09-11 22:15 — written while `prop_gens40` is at gen 38 of 40, trigger NOT yet fired

Amending before the trigger is legitimate; amending after would not be. The reason for the amendment
is that I read further and found the mechanism I was about to reach for is **already measured and
already refuted.**

**What I was about to design, and why it would have failed.** `FITNESS.md` §3 specifies the mate set
as MATE-N for N in {1,2,3,4}, 500 each, and §10 lists the mates-per-cost filter as the check that
catches "prune everything / return eval". The live set is 19 positions, mate-in-ONE only, so the
obvious move is "restore the deeper mate strata". **That is measured to be nearly toothless.**
`forced_mate_set` (`evolve.rs:200`) ALREADY EXISTS as a mate-in-2 builder, and its own doc comment
records the outcome:

> *"The forced-mate-in-2 set was added to stop a candidate from simply searching less, and it worked
> at fitness depth 2. At fitness depth 3 it has no teeth: that set is solved 40/40 AT DEPTH 2
> (measured — the forcing move is also the eval-best move), so cutting 3 -> 2 costs nothing on it.
> The search track promptly found exactly that: `Const(0)` -> `Const(1)` in the horizon guard, one ply
> shallower, 11x cheaper, all 20 mates intact. A depth guard must require the FULL fitness depth, and
> a mate-in-N does not imply N plies of search. **Disagreement does, by construction.**"*

**The reconciliation, which is the actual design input:**

```
candidate                        MATE-2 stratum         disagreement_set
child #30 (null search, 2732x)   CAUGHT (12/24)         caught
Const(0)->Const(1) (ONE ply)     BLIND (40/40 at d2)    caught by construction
```

A MATE-N stratum catches **gross** truncation and is **blind to a one-ply cut**. The disagreement set
is not, because a disagreement position is *defined* as one where shallow and deep search differ — it
requires the plies by construction.

**Amended design.** The load-bearing stratum is `disagreement_set`, NOT mates of any depth:

* mates stay a **filter** for gross truncation only, and stop contributing to the score — unchanged;
* the stratum that must GROW is `disagreement_set` (`n2`), not a deeper mate set. The live run is
  `10 + 4 + 5`, so the load-bearing stratum is **4 of 19 positions, 21%**;
* `fitness_set_composition_RESULT.md` already measured the direction — depth-heavy `5/10/10` holds a
  null search to 1306.606x against 2934.933x on pure mate-in-1.

**A second reason not to lean on mates at all:** `surrogate_inverts_RESULT.md` measured mates/Mcost
on the MATE-2 set and found it inverts against games there too — the `depth-one` program is ranked
**highest per Mcost (0.577) while scoring worst in games (0.427)**. So a deeper mate set does not fix
the inversion; it relocates it.

**Spec gap, recorded rather than silently patched:** `FITNESS.md` §10 credits the mates-per-cost
filter with catching "prune everything / return eval". Measurement says that holds for gross
truncation only and fails for a one-ply cut. The doc is the authority on ORDER, but this row overstates
what the filter delivers, and §3's own MATE-{1,2,3,4} filter inherits the weakness. Flagging, not
editing — the docs are not mine to quietly rewrite.

**The verdict rule is UNCHANGED** (decisive-game fraction against the 14.7% baseline, CI excluding it).
Only the lever changed: grow disagreement, not mate depth.

**Process note, because it is the recurring cost.** `fitness_set_composition_RESULT.md` records the
same lesson about itself — *"I measured before reading. `git grep mate_set` would have shown
`forced_mate_set` in one command, and its comment answers the question the experiment was designed to
ask."* That is now the third time tonight the results dir already held the answer.

---

## AMENDMENT 2 — 2026-09-11 23:26, treatment at gen 34/40, its decisive fraction NOT yet examined

**What I looked at, precisely:** I validated `p2_fitness_verdict.sh` by running it against the
COMPLETE control log (`prop_gens40.log`), whose numbers are already published in
`prop_gens40_RESULT.md`. **I have not computed or looked at the treatment's decisive fraction.** This
amendment is driven entirely by a property of the control and of the baseline, both of which predate
the treatment's outcome.

**The flaw the self-test exposed.** The control's own decisive fraction is:

```
CONTROL (10+4+5)   79 decisions, 948 games, 170 decisive
                   fraction 0.179  CI [0.155, 0.204]
baseline           0.147           <- lower bound 0.155 is ABOVE it
```

**The control already "passes" the pre-registered primary.** So "treatment CI excludes 0.147" cannot
distinguish an improvement from doing nothing — a rule both arms satisfy is not a test. The 0.147
figure comes from `gate_candidates_are_game_neutral_RESULT.md`, measured on a different population and
configuration, and is **not commensurable with this control**. Importing a constant from another
experiment as the bar was the error.

**Amended primary: TREATMENT vs CONTROL, directly.** The two arms are paired by construction —
`fitness_set_composition_RESULT.md` established that crossover is seeded, so both runs see the SAME
children and only the position set differs. That is the comparison the design actually supports.

```
PASS   treatment decisive fraction ABOVE the control's, with non-overlapping 95% CIs
NULL   CIs overlap
FAIL   treatment below the control, CIs non-overlapping
```

**0.147 is DEMOTED to context**, still worth printing because it is where the "candidates are
game-neutral" claim came from, but it is no longer the bar.

**Unchanged:** power is quoted WITH the estimate, never under it — the treatment reaches the gate far
less often than the control, and the decision counts go in the headline. An accept count alone still
does not pass. The null reading is still the pre-registered one: if candidates barely differ
behaviourally, changing what we weigh does not change what there is to weigh, and the successor is
GRAMMAR 4.

**Why this is legitimate now and would not be in ten minutes:** the treatment has not produced its
verdict, and the change was forced by validating the instrument against already-published control data
— not by seeing the treatment's answer. Had I run the script on the treatment first, this amendment
would have been unusable.
