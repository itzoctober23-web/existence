# gate resolution A/B — RESULT (2026-09-08)

Both arms resumed from the same champion (`champion_long.net`), equal wall-clock, then scored
against the same frozen origin at 600 pairs.

| arm | generations | accepts | median gate ci95 | scored vs origin |
|---|---|---|---|---|
| gate-pairs 40 (the default) | 41 | 7 | 0.072 | **0.826 +/- 0.014** |
| gate-pairs 224 | 35 | 1 | 0.031 | **0.859 +/- 0.013** |
| BASELINE — the net both started from | — | — | — | 0.864 +/- 0.013 |

Two-sample resolution at these intervals is ~0.019.

## Against the pre-registration, which was written before any of these numbers existed

gate_ab.sh said: *"The diagnosis predicts BOTH: (a) more accepts -- small real gains stop being
discarded, AND (b) a HIGHER origin control. (a) alone REFUTES it."*

* **(a) is REFUTED.** The sharp gate accepted FEWER, not more: 1 in 35 generations against 7 in
  41. The predicted mechanism was that a blunt gate throws away real small gains. That is not
  what was happening.
* **(b) is CONFIRMED.** 0.859 vs 0.826, a gap of 0.033 against a ~0.019 floor. RESOLVED.

So the gate was the problem and the stated reason for it was wrong. The blunt gate was not
DISCARDING good candidates, it was ACCEPTING BAD ONES: at ci95 0.072 it cannot distinguish a real
change from a coin flip, so roughly half its 7 accepts were noise, and noise accumulates
downward. Sharpening it to ci95 0.031 stopped that.

## The part that matters more, and it is not good news

**Neither arm improved on its starting point.** 0.859 against a 0.864 baseline is a gap of 0.005,
five times below the noise floor. The sharp gate did not learn; it stopped LOSING.

Put together with the replay sweep (window 1 -> 0.843, window 8 -> 0.862, window 999 -> 0.863,
all at or below the same 0.864 start), the picture is consistent across seven independently
scored nets today: **the loop has produced no measurable gain, and its blunt gate was actively
eroding the champion.** Fixing the gate converts a slow loss into a flat line.

## What this redirects

The bottleneck is no longer selection. With a gate that can actually resolve 0.031, the loop
accepted ONE candidate in 35 generations -- which means the candidates being generated are almost
never better. That moves the question from "why is the loop keeping bad nets" (answered) to "why
are the trained candidates not better than their parent", which is about the TRAINING step, not
the gate.

Do NOT read this as "224 pairs is the fix and the loop is healthy". It is the fix for the
degradation, and the degradation was masking the real problem.
