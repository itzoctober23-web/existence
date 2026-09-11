# The lr result replicates on a fresh seed and a different start — and the optimum is lower still

**2026-09-11 02:15.** Three arms, 2,000 generations each, same start (`p1_champion`), same seed
(20260912), differing only in `--lr`. Verdict by `netmatch` against the shared start.

| arm | vs shared start | interval | first run (seed 20260911, plateaued start) |
|---|---|---|---|
| lr 0.01 | **0.358 ± 0.026** | [0.332, 0.384] | 0.499 ± 0.030 |
| lr 0.002 | **0.544 ± 0.025** | [0.518, 0.569] | 0.692 ± 0.028 |
| **lr 0.0005** | **0.589 ± 0.026** | [0.563, 0.616] | — |

## What is settled

**The shipped change is sound, and this is a real replication.** Different seed, and — more
importantly — a different *start*: the first A/B ran from a net plateaued at lr 0.01, this one from a
net already trained at 0.002. The pre-registered row fires:

> **0.002 beats 0.01 again** → replicated on a second seed and a different start. The shipped change
> is sound.

The rival explanation is dead. The pre-registration named it explicitly: *"the effect was specific
to a net plateaued AT 0.01, i.e. a recovery from over-large steps rather than a better rate."* If
that were true, lr 0.002 would have shown nothing from a start that had already had it. It scored
**0.544**, interval clear of 0.5.

**lr 0.01 is not merely neutral, it is destructive.** From this start it scored 0.358 — it made the
champion substantially *worse*. Independently corroborated: `auto_promote`, on its own schedule and
without knowledge of this run, measured the same arm against the champion at **0.356**. Two separate
224-pair matches of the same comparison, agreeing to 0.002.

## What is NOT settled — and the project's own rule says so

**0.0005 versus 0.002 is inside the noise floor.** The gap is 0.589 − 0.544 = **0.045**, and
`netmatch`'s between-seed sd is **0.047**. The difference is smaller than the sd of the instrument
across seeds. The intervals themselves overlap on [0.563, 0.569].

By this project's own standard — the one it applied to its own champion promotion — **one seed
cannot settle this**. The honest statement is that 0.0005 is *ahead on this seed* and that the
pre-registered row is:

> **0.0005 beats 0.002** → the optimum is lower still; sweep again before settling.

Production has **not** been switched to 0.0005 on this reading. The decay A/B now running includes
`lr 0.0005 constant` as an arm, which tests it again on a third seed at no extra cost.

## The pre-registration, scored

`lr_sweep_PREREGISTRATION.md` was committed at 02:02, before any match reported. It used the
game-free `static_deep_residual --ref` instrument built an hour earlier.

| predicted | outcome |
|---|---|
| lr 0.01 is not the best rate | ✅ 0.358, worst by a distance |
| lr 0.0005 is top or joint-top | ✅ 0.589, top |
| *weak:* 0.002 ahead of 0.01 | ✅ 0.544 vs 0.358 |
| predicted row: "0.0005 beats 0.002, sweep again" | ✅ the row that fired |

**The instrument predicted the complete ordering.** Its `corr` column read 0.769 > 0.722 > 0.637 for
0.0005 > 0.002 > 0.01; netmatch returned 0.589 > 0.544 > 0.358. Perfect rank agreement, from an
instrument that plays no games, recorded in git beforehand.

### Where it was wrong, which matters more

The pre-registration also said, of the same table:

> No arm clearly beat its own start.

**That is refuted.** Two arms beat the start decisively (0.544 and 0.589, both intervals clear of
0.5). The residual instrument had placed 0.002 and 0.0005 at 0.722 and 0.769 against a start of
0.746 — i.e. at or slightly *below* it.

**So the instrument ranks but does not calibrate.** It orders arms correctly and cannot tell you
whether an arm beats the reference. That is a genuine limitation and it is the kind that would
mislead if the tool were used alone: it would have said "nothing here gained" about a run that
produced two clear gains.

The disagreement between its two columns was also resolved, in favour of `corr`: **sign agreement
ranked 0.002 last and it finished second.** The pre-registration flagged that conflict rather than
quoting the column that read better, and the flag was worth having.

## The mechanism, and what runs next

The pre-registration's mechanism argument — each drop in rate buys a burst that then saturates — is
**partially supported and now sharper**:

* lr 0.002 from the champion **does** gain at 2,000 generations (0.544).
* But `prod3`, same start and rate, read **0.478** at generation 5,778, and another run read 0.458 at
  generation 1,248.
* Lowering again to 0.0005 gains more (0.589).

Gain early, fade later, and lowering again recovers it. `trainer.rs` is plain SGD with no schedule,
and a constant step keeps displacing the weights however close to a basin they are — already
measured in weight space, where the 0.01 arm travelled 1.73× farther than the 0.002 arm and gained
nothing.

`lr_decay_ab.sh` is running now and tests exactly this: **A** lr 0.002 constant, **B** lr 0.002
decaying to 0.000493 over 2,000 generations, **C** lr 0.0005 constant. B and C *end* at the same
rate deliberately — if B beats C the advantage came from the large early steps, so the schedule is
doing real work rather than being a slow route to 0.0005.

## Method note

Every arm's `--lr` was verified from the program's own header before any verdict was read
(`lr=0.01`, `lr=0.002`, `lr=0.0005`, each with 2000 generations). A flag that does not appear in the
log is one nobody can verify took, and this project retracted a published finding for exactly that.
