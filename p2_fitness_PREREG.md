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
