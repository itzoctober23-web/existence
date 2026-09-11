# lr 0.0002 SHIPPED — the shipped rate lost to its own start, on exactly matched arms

**2026-09-11 07:57.** Three arms from champion `34a5ace752d8`, seed 20260916, **2000 generations
each — exactly matched**, no pause. Verdict by `netmatch` against the shared start.

| arm | vs shared start | interval | rule (≥ 0.500) |
|---|---|---|---|
| **0.0005** (was shipped) | **0.412 ± 0.026** | [0.386, 0.438] | fails — **loses to its own start** |
| **0.0002** | **0.535 ± 0.028** | [0.506, 0.563] | **0.507 — CLEARS** |
| 0.0001 | 0.546 ± 0.031 | [0.515, 0.577] | clears, but see below |

**Shipped:** `low_00002.net` promoted to `p1_champion` (`34a5ace752d8` → `044e754f57ba`, previous
kept as `p1_champion_prev_pre_lr00002.net`). Production relaunched at `lr=0.0002` from the new
champion, verified from its own header, and `keepalive.sh`'s default changed so a crash-restart uses
the shipped rate. **Passed the gate.** No Elo figure is quoted for it.

## The control is what makes this trustworthy

**lr 0.0005 scored 0.412 with its interval entirely below 0.5.** The rate that was shipped an hour
earlier *loses to the net it started from*. That is not a treatment reading — it is the control, and
it replicates the truncated run's 0.422 on a **fresh seed** and a **different start**.

It is also the same shape lr 0.002 showed once the champion had been trained at 0.002 (0.484, and
prod3 at 0.471 after 12,283 generations). A rate that has done its work stops paying from a net that
already has it. That pattern has now appeared at three successive rates.

## What the truncated run got right, and what it did not

The earlier run was cut short at ~650 generations with **unmatched arms** (666 / 645 / 652) when
`timeout` killed them while frozen. Its numbers were 0.422 / 0.551 / 0.516. This run is 2000 /
2000 / 2000 — **exactly matched** — and reproduces the ordering and very nearly the values.

**But its "turning point" is NOT confirmed.** The truncated run had 0.0001 *below* 0.0002 (0.516 vs
0.551) and that was read as the first sign of an optimum. At full length **0.0001 is nominally
ABOVE 0.0002** — 0.546 against 0.535, a gap of **0.011** against a between-seed sd of **0.047**.
They are indistinguishable. The pre-registration (`low_sweep2_PREREG.md`, committed before any
verdict existed) said plainly:

> No promotion on a single seed if the margin is under `netmatch`'s between-seed sd of 0.047.

So **0.0001 was not shipped and is not claimed to be better.** The optimum is still not bracketed
from below, and the sequence 0.01 → 0.002 → 0.0005 → 0.0002 has still not turned.

## The baseline guard, which was not theoretical

`auto_promote` was measuring `prodk0633` (16,633 generations) against the champion *while this sweep
ran*, on its own 30-minute cycle. Had it banked, "vs shared start" would have stopped meaning "vs
champion", and promoting on 0.535 would have been promoting against a net that was no longer the
incumbent. Checked immediately before the promotion: `low_start.net` and `p1_champion.net` were
still byte-identical. `ship_sweep_winner.sh` carries the re-measure path for the next time they are
not.

## Method notes

* **The verdict lines were never going to appear where I first looked.** The sweep runs under
  `systemd-run`, so its stdout goes to the **journal**, not `low_sweep2.out`. The generation counts
  I was reading came from the arm logs and were correct; the verdicts were in
  `journalctl --user -u low-sweep`.
* **`low_00001_vs_start.log` was a stale file from the truncated run** (mtime 06:04, value 0.516)
  and would have been read as this run's arm-3 verdict. Only files written after 07:40 belong here.
  Timestamps, not filenames, decide which run a file came from.
