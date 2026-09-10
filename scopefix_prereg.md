# PRE-REGISTRATION — P2 restarted with the scope fix, written before the log was read

**2026-09-10 17:44.** `gate_scopefix_s1.log`, launched immediately after `typecheck::scope_check`
landed. Config is byte-identical to the recorded baseline `gate_sprt30_s1` — same seed, same bounds,
same args — so the ONE difference is that candidates reading unbound variables are now discarded at
generation time instead of reaching the gate.

    EXISTENCE_EVOLVE_SEED=1 EXISTENCE_GATE_SPRT=1 EXISTENCE_GATE_ELO0=3 EXISTENCE_GATE_ELO1=5
    EXISTENCE_GATE_MAXPAIRS=4000 EXISTENCE_GATE_VERIFY=96   evolve 25 8 12 6 3

## The baseline this is measured against

`gate_power_RESULT.md`, run to completion:

| | value |
|---|---|
| decisions | 16, **zero INCONCLUSIVE** |
| games spent | 592 (mean 37/decision, range 20–82) |
| ACCEPTs | 0 |
| pooled | W-D-L 19-498-75, **0.4527, 95% CI [0.4371, 0.4683]** — resolved WORSE, ~−33 Elo |

## What I expect, and why it is deliberately modest

`tests/scope_escape.rs` measured the escape rate at **26 of 823 applied mutations (3.2%)**. Candidates
take 1–3 edits, so roughly 3–9% of candidates carried at least one hole. If every holed candidate was
a guaranteed loss and they are now gone, the pooled rate should move by **at most ~0.01–0.02** —
against a baseline interval of ±0.016 on 592 games.

**So this run is underpowered for the strength question, and that is stated in advance rather than
discovered afterwards.** Predicting "pooled rate rises" and then reading a +0.01 as confirmation
would be reading noise. The honest predictions are:

1. **Accepts: still 0.** Removing broken candidates does not create good ones. `gate_power_RESULT`
   already resolved that the surviving candidates are genuinely worse, and nothing here changes what
   the generator PRODUCES — only what it stops producing.
2. **Pooled rate: unchanged within noise**, i.e. the new interval overlaps [0.4371, 0.4683].
3. **The real, already-measured deliverable is BUDGET, not strength**: 3.2% of applied mutations no
   longer cost games. That number is exact and does not need this run to confirm it.

## What would surprise me, and what each would mean

* **Pooled rate rises ABOVE 0.4683** — holed candidates were a bigger drag than their frequency
  suggests, which would mean a single hole is worse than an average bad mutation. Plausible: a hole
  can disable a statement outright.
* **An ACCEPT appears** — would be the first in 219 decisions across this repo. It would NOT be
  attributable to the scope fix on this evidence alone and would need its own replication before
  being called anything.
* **Pooled rate FALLS** — the fix removed candidates that were accidentally useful, which would be a
  genuine surprise and worth a specimen rather than a shrug.

## The trap this exists to avoid

The fix is correct on its own terms: a program reading `Value::Unit` where a move belongs is broken
whether or not removing it moves a number. The temptation after a correctness fix is to go looking for
a strength story to justify it. The justification is the 26 measured escapes and GRAMMAR 3's stated
economics, and it does not depend on what this log says.
