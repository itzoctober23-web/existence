# Unguarded acceptance causes the resume dip — and the gate prevents it by preventing progress

**2026-09-10.** `gated_resume_PREREG.md` asked whether the ~95 Elo resume dip survives a live
strength gate, and named the falsifier before any net was measured. It did not fire.

## The measurement

Identical to the ungated run in every respect except `--gate-every 1`, which turns the real
per-generation match back on. Same champion, same seed, same instrument.

| run | generations | vs its own start | interval |
|---|---|---|---|
| **GATED** | 37 | **0.520 ± 0.038** | [0.482, 0.558] |
| ungated (`resume_dip_RESULT.md`) | 25 | **0.411 ± 0.034** | [0.377, 0.445] |
| ungated | 100 | 0.366 ± 0.035 | [0.330, 0.401] |

**The intervals are disjoint.** Difference +0.109, **4.2 σ** — resolved. With the gate on, the
champion does not fall below parity; without it, it loses ~95 Elo by generation 100.

The pre-registered falsifier — *"a gen-100 reading materially below 0.5 with the gate ON refutes
unguarded acceptance as the cause"* — did not fire. **Unguarded acceptance is the mechanism.**

## But the honest reading is narrower than "the gate is the fix"

The gate did not *steer* the run. It **refused almost everything**: 33 rejects against 2 accepts in
35 decisions. Its first two acceptances were genuine — 0.538 ± 0.033 and 0.540 ± 0.033 in real
400-game matches — but the champion moved twice in 37 generations. At generations 5 and 25 it was
still **byte-identical to its start**, which is why those snapshots could not be taken at all.

So "the gate prevents the dip" is substantially "the gate prevents change". That is exactly the
dilemma `acceptance_floor_RESULT.md` already records — *the gate demands an edge 2.7× larger than a
generation produces* — seen from the other side. Watched live, the floor is stark: a candidate
scoring **0.518 ± 0.034** was **rejected**, because 0.518 − 0.034 = 0.484 < 0.5. It outscored the
champion and was refused.

## The trade, stated plainly

| | adopts | early behaviour | where it ends up |
|---|---|---|---|
| **ungated** | everything (mean candidate **0.4931**) | −95 Elo by gen 100 | **0.557 ± 0.032** by gen 4,327 |
| **gated** | 5.7% (2 of 35) | stays at parity | unknown — 4,327 generations would take ~55 h at 1.3 gen/min |

**The dip is the price of a loop that moves.** The ungated run digs a 95 Elo hole and then climbs
out of it to +40; the gated run never digs the hole and, on this evidence, never goes anywhere
either. The two endpoints are NOT compared here and cannot be on this budget — the gated run is
~130× slower per generation, so matching 4,327 generations is two days of box time. Anyone quoting
this file as "gating is better" or "gating is worse" is quoting something it does not say.

## What it settles, and what it leaves open

**Settled:** the resume dip is caused by adopting candidates that are, on average, slightly worse
than the champion. `reject_holdout_RESULT` (0.4907), `accept_audit_RESULT` (0.5179) and
`accept_rate_vs_noise_RESULT` (8.87%) put the mean adoption at **0.4931**, below parity, and turning
adoption off removes the dip at 4.2 σ.

**Still open:** the magnitude. Naively compounding −4.8 Elo/generation predicts −479 Elo at gen 100
against a measured −95 — a 5× over-prediction, because each candidate is scored against the
*current* champion, so it is a biased random walk rather than a ladder. This run does not close that
gap.

**Not tested:** whether a gate with a *lower* floor — one that rejects the clear losers without
refusing 0.518 — gets the climb without the hole. That is the experiment both this file and
`acceptance_floor_RESULT.md` point at, and neither has run it.

## Method note

Two bugs in the harness, both worth recording because both were silent:

* The snapshot line was `[ -s "$OUT" ] && cp ... && echo`, which prints **nothing** when the file is
  absent. With the gate on the trainer writes `--out` only on an ACCEPT, and the first came at gen
  26 — so the gen-5 and gen-25 snapshots were skipped and the log said nothing. An absent
  measurement that announces nothing is indistinguishable from one never requested. It now reports
  `NO SNAPSHOT` and why.
* I then pushed a syntactically broken version of the fix. `bash -n` caught it; I missed the catch,
  because the check and the commit were separate commands in one invocation — `bash -n x && echo OK`
  failed quietly while `git commit && git push` ran regardless. A verification only gates if the
  action is *conditional* on it.
