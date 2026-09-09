# A guard tolerance of 4 separates genuine changes from known exploits — on the TARGETED guard only

2026-09-08. The first proposed fix this session that survives its own control.

## The problem it solves

`search_track_WHY_NOTHING.md`: **0 of 33 behaviour-changing single edits pass the all-or-nothing
guard.** Everything that changes play fails `f >= best_found`; everything that passes plays
identically. The guard is, in practice, "do not change behaviour" — so the loop rejects every
candidate that could be an improvement, using a rule that cannot tell "worse" from "different".

## Why the first attempt at this failed

On the ORIGINAL guard (mate + disagreement + **window**-sensitive), measured:

| | loss |
|---|---|
| genuine behaviour-changing candidates | minimum **2** |
| ALPHA exploit (`neg(INF)` → `8`) | **2** |
| DEPTH exploit (`d==0` → `d==1`) | 9 |

The alpha exploit lost exactly as much as the best genuine candidate. **No tolerance separates
them**, and the fix was dead — which is why the control was run before implementing it.

## What changed: the guard now targets the exploit it was built for

The window-sensitive set varies **INF** (a symmetric window, −8..+8) while the exploit raises the
**initial alpha** to +8 (asymmetric). It caught the exploit only incidentally.
`alpha_sensitive_set` builds the guard from the transformation itself — run the seed against the
alpha-raised variant, keep the disagreements, record the seed's answer as correct — so the exploit
scores zero on all of them by construction. The alpha exploit's loss moved **2 → 5**.

## The result, both numbers from the SAME guard

```
genuine:   lost 3: 2    lost 4: 4    lost 5: 6    lost 7: 10   lost 8: 1   lost 22: 2   lost 25: 8
exploits:  ALPHA loses 5      DEPTH loses 8

tolerance 2: admits  0/33 genuine   exploits admitted: NONE
tolerance 3: admits  2/33 genuine   exploits admitted: NONE
tolerance 4: admits  6/33 genuine   exploits admitted: NONE
tolerance 5: admits 12/33 genuine   exploits admitted: ALPHA
```

**A tolerance of 4 admits 6 of 33 behaviour-changing candidates (18%) and rejects both known
exploits.** Where the current all-or-nothing guard admits zero.

## What this does NOT establish

* **The margin is ONE.** Genuine candidates admitted at 4; the nearest exploit at 5. A third
  exploit that loses 4 slips straight through, and this session has already found two exploits
  nobody anticipated. The separation rests on a sample of exactly two adversaries.
* **Admitted ≠ improvement.** These 6 are behaviour-changing and *nearly* correctness-preserving.
  Whether any plays BETTER is a separate question that only games answer — the same wall every
  route in this investigation reaches.
* Both numbers come from one seed and one 100-candidate sample.

## Why it is still worth having

It converts the search from "provably cannot take a step" to "can take 18% of behaviour-changing
steps". That is a precondition for improvement, not improvement itself — but the loop currently has
no precondition at all, and this is the first change measured to supply one without re-admitting a
known exploit.
