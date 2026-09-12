# PRE-REGISTRATION — does a LARGER, more stable budget avoid the harm?

**2026-09-12 08:05, written while `candB18` trains and ~37 minutes before any result exists.**

## The prediction being tested

`budget_instability_is_boundary_straddling_RESULT.md` measured that a node budget's move instability
is governed by **boundary straddling** — the fraction of positions that reach a different realised
depth on two runs — with Pearson **r = +0.981** over eight points and identical rank order on both
seeds independently. Because straddling is non-monotone in budget size, it produces a configuration
that is normally impossible:

```
budget  5269 (in use)   straddle ~8.0%   instability -2.6   mean realised depth 2.63
budget 18000            straddle ~5.7%   instability -1.6   mean realised depth 3.34
```

**A budget of 18000 is both DEEPER and MORE STABLE than the one in use.** Depth and stability
normally trade off; here they do not, because stability is set by straddling rather than by depth.

Variance amplification is the last of the three mechanisms named in
`budget_harm_is_emergent_RESULT.md` still standing — drift was contradicted and decisiveness feedback
weakened by `trajectory_drift_RESULT.md`. If it is the operative one, **the 18000 arm should be
measurably less harmed than the 5269 arm.**

## The arm

`candB18` is byte-identical to `candB2_budget` in every flag except the budget:

```
--init cand_start.net --gens 2000 --games 8 --threads 1 --depth 3 --epochs 3
--lr 0.0002 --lr-decay 1.0 --blend 0.85 --seed 777777 --datagen-budget 18000
```

Same **training seed 777777**, same frozen start (`cand_start.net`, md5 `9545a35289e9`, verified at
launch), so `candA2_fixed` and `candB2_budget` are both valid comparators without re-running them.

## Primary comparison and the standard

**PRIMARY: `candB18` vs `candB2_budget`**, 224 pairs, depth 4, three match seeds. This is the clean
one-variable comparison — same training seed, same start, same flags, only the budget differs. It
needs no reference to the start net and no assumption about what the control did.

**The standard is the gate's own, not a pooled interval.** `candidate_a_replication_RESULT.md`
records that my pooled-CI bar was mis-specified: three match seeds RE-PLAY the matches, so their
interval measures match noise and cannot speak to training-seed variation. `gate_candidate_a.sh`
applies `|mean − 0.5| > 0.047`, the project's measured between-seed sd, and that is the bar here.

| outcome | reading |
|---|---|
| mean > 0.5 and \|mean − 0.5\| > 0.047 | **Prediction confirmed.** A larger, more stable budget is measurably better than the operational one at matched generations. Variance amplification survives as the mechanism and the straddle curve becomes a design tool: pick budgets off its minima. |
| \|mean − 0.5\| ≤ 0.047 | **UNRESOLVED by the gate's standard**, exactly as the replication was. Report it as a bound, not a direction. Note the sign for the record but do NOT claim it. |
| mean < 0.5 and \|mean − 0.5\| > 0.047 | **Prediction FAILS, and informatively.** A budget that is both deeper and more stable is *worse*. That would refute variance amplification as the operative mechanism, leaving all three named mechanisms dead and forcing a new hypothesis rather than a bigger experiment. |

**Predicted, so it can be wrong:** `candB18` beats `candB2_budget`, landing somewhere near 0.53–0.56,
but with a real chance of falling inside the 0.047 band and resolving to UNRESOLVED — the same fate
as the replication, and for the same structural reason.

## Secondary, for context only — not decision-bearing

* `candB18` vs `cand_start.net` — the measurement that put the two existing budget arms **below their
  own start** (0.445 and 0.422). If B18 lands at or above 0.5 here, the harm is not merely reduced but
  absent.
* `candB18` vs `candA2_fixed` — against the fixed-depth control that finished **above** its start
  (0.552).

These are single-seed context matches in the existing gate and carry the caution
`candidate_a_budget_loses_RESULT.md` already records: 0.535 against a 0.047 between-seed sd is
0.74× the noise, which is why that file declined to claim arm A had improved.

## What cannot be concluded either way

* **Nothing about production.** `budget_harm_is_emergent_RESULT.md` established the harm needs 2000
  generations to appear and every single-pass channel is null. This is a 2000-generation arm, so it
  can speak to the harm — but it is one training seed, and one seed is a lottery.
* **Not that 18000 is optimal.** The straddle sweep has six points and the right-hand limb is not
  bracketed; instability may keep falling past 18000 or rise again at the 3/4 boundary.
* **Nothing ships.** `candB18.net` is diagnostic. No figure will be quoted as Elo gained.
